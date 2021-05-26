use crate::LoaderError;
use emf_core_base_rs::CBaseAccess;
use std::path::Path;

/// A type responsible for loading the core module.
pub trait CoreLoader<'a> {
    /// Library type.
    type Library: 'a;
    /// Module type.
    type Module;
    /// Instance type.
    type Instance;
    /// Extensions type.
    type Extensions;
    /// Interface type.
    type Interface: CBaseAccess<'a>;
    /// Error type.
    type Error: Into<LoaderError<Self::Error>>;

    /// Loads a library.
    fn load_library(&mut self, module_path: &Path) -> Result<Self::Library, Self::Error>;

    /// Unloads a library.
    fn unload_library(&mut self, library: Self::Library);

    /// Loads the module from the library.
    fn fetch_module_handle(
        &mut self,
        library: &mut Self::Library,
    ) -> Result<Self::Module, Self::Error>;

    /// Bootstraps the base interface.
    #[allow(clippy::type_complexity)]
    fn initialize_interface(
        &mut self,
        module: &mut Self::Module,
        target_version: &Version,
    ) -> Result<(Self::Interface, Self::Instance, Self::Extensions), Self::Error>;

    /// Terminates the core module.
    fn terminate_interface(
        &mut self,
        module: Self::Module,
        instance: Self::Instance,
    ) -> Result<(), Self::Error>;
}

use emf_core_base_rs::version::Version;
pub use native::{NativeLoader, NativeLoaderError, ValidationError};

mod native {
    use crate::core_loader::CoreLoader;
    use emf_core_base_rs::module::native_module::{NativeModule, NativeModuleInstance};
    use emf_core_base_rs::ownership::Owned;
    use emf_core_base_rs::version::Version;
    use emf_core_base_rs::{CBase, CBaseRef, Error};
    use libloading::Library;
    use std::marker::PhantomData;
    use std::path::{Path, PathBuf};

    pub use manifest::{
        get_manifest_path, parse_manifest, parse_manifest_from_file, ModuleManifest,
        ValidationError,
    };

    /// A loader for native modules.
    #[derive(Debug)]
    pub struct NativeLoader<'a> {
        phantom: PhantomData<&'a ()>,
    }

    impl Default for NativeLoader<'_> {
        fn default() -> Self {
            Self {
                phantom: PhantomData,
            }
        }
    }

    impl<'a> CoreLoader<'a> for NativeLoader<'a> {
        type Library = Library;
        type Module = NativeModule<'a, Owned>;
        type Instance = NativeModuleInstance<'a, Owned>;
        type Extensions = ();
        type Interface = CBase<'a>;
        type Error = NativeLoaderError;

        fn load_library(&mut self, module_path: &Path) -> Result<Self::Library, Self::Error> {
            let manifest_path = manifest::get_manifest_path(module_path).map_or(
                Err(ValidationError::NoManifest(module_path.to_path_buf())),
                Ok,
            )?;
            let manifest = parse_manifest_from_file(&manifest_path)?;
            match manifest {
                ModuleManifest::V0 { manifest } => Ok(Library::new(&manifest.library_path)?),
            }
        }

        fn unload_library(&mut self, library: Self::Library) {
            drop(library)
        }

        fn fetch_module_handle(
            &mut self,
            library: &mut Self::Library,
        ) -> Result<Self::Module, Self::Error> {
            use emf_core_base_rs::ffi::collections::NonNullConst;
            use emf_core_base_rs::ffi::module::native_module::NativeModuleInterface;

            unsafe {
                let sym = library
                    .get::<*const NativeModuleInterface>(b"emf_cbase_native_module_interface\0")
                    .map_err(|_e| NativeLoaderError::ModuleInterfaceNotFound)?;
                Ok(NativeModule::new(NonNullConst::new_unchecked(*sym)))
            }
        }

        #[allow(clippy::type_complexity)]
        fn initialize_interface(
            &mut self,
            module: &mut Self::Module,
            target_version: &Version,
        ) -> Result<(Self::Interface, Self::Instance, Self::Extensions), Self::Error> {
            use emf_core_base_rs::ffi::collections::{ConstSpan, Optional};
            use emf_core_base_rs::ffi::module::native_module::NativeModuleBinding;
            use emf_core_base_rs::ffi::module::ModuleHandle;
            use emf_core_base_rs::ffi::{
                Bool, CBase as CBaseFFI, CBaseFn, CBaseInterface, FnId, TypeWrapper,
                CBASE_INTERFACE_NAME,
            };
            use emf_core_base_rs::module::{InterfaceDescriptor, InterfaceName};
            use std::ptr::NonNull;

            #[allow(improper_ctypes_definitions)]
            extern "C-unwind" fn has_fn(_b: Option<NonNull<CBaseFFI>>, id: FnId) -> Bool {
                let id: u32 = unsafe { std::mem::transmute(id) };
                if id == 0 {
                    Bool::True
                } else {
                    Bool::False
                }
            }

            #[allow(improper_ctypes_definitions)]
            extern "C-unwind" fn get_fn(
                _b: Option<NonNull<CBaseFFI>>,
                _id: FnId,
            ) -> Optional<CBaseFn> {
                Optional::None
            }

            const BASE_MODULE: ModuleHandle = ModuleHandle { id: 0 };

            unsafe {
                let mut instance = module
                    .into_mut()
                    .as_mut()
                    .load(BASE_MODULE, None, TypeWrapper(has_fn), TypeWrapper(get_fn))
                    .map_or_else(|e| Err(Error::new(e)), |v| Ok(NativeModuleInstance::new(v)))?;

                if let Err(e) = module.initialize(&mut instance) {
                    module.unload(instance)?;
                    return Err(From::from(e));
                };

                let interface_desc = InterfaceDescriptor {
                    name: InterfaceName::from(CBASE_INTERFACE_NAME),
                    version: *target_version,
                    extensions: ConstSpan::new(),
                };

                let interface = match module.get_interface(&instance, &interface_desc, |i| {
                    i.interface.cast::<CBaseInterface>()
                }) {
                    Ok(i) => i,
                    Err(_e) => {
                        module.terminate(&mut instance)?;
                        module.unload(instance)?;
                        return Err(NativeLoaderError::CoreInterfaceNotFound);
                    }
                };

                let interface = CBase::new(CBaseRef::new(*interface.as_ref()));

                Ok((interface, instance, ()))
            }
        }

        fn terminate_interface(
            &mut self,
            mut module: Self::Module,
            mut instance: Self::Instance,
        ) -> Result<(), Self::Error> {
            unsafe {
                module.terminate(&mut instance)?;
                module.unload(instance)?;
            }
            Ok(())
        }
    }

    /// Possible loader errors.
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum NativeLoaderError {
        /// Libloading error
        LibError(libloading::Error),
        /// Serde error.
        SerdeError(serde_json::Error),
        /// Error originating from the api.
        APIError(Error<Owned>),
        /// Error while validating a module manifest.
        ModuleManifestError(ValidationError),
        /// The provided core module is invalid.
        InvalidCoreModule(PathBuf),
        /// The interface of the module is not exposed.
        ModuleInterfaceNotFound,
        /// The library does not expose the required interface.
        CoreInterfaceNotFound,
        /// A path does not belong to a module.
        NotAModule(PathBuf),
    }

    impl From<libloading::Error> for NativeLoaderError {
        fn from(err: libloading::Error) -> Self {
            NativeLoaderError::LibError(err)
        }
    }

    impl From<serde_json::Error> for NativeLoaderError {
        fn from(err: serde_json::Error) -> Self {
            NativeLoaderError::SerdeError(err)
        }
    }

    impl From<Error<Owned>> for NativeLoaderError {
        fn from(err: Error<Owned>) -> Self {
            NativeLoaderError::APIError(err)
        }
    }

    impl From<ValidationError> for NativeLoaderError {
        fn from(err: ValidationError) -> Self {
            NativeLoaderError::ModuleManifestError(err)
        }
    }

    mod manifest {
        use serde::Deserialize;
        use std::fmt;
        use std::fs::File;
        use std::io::{BufReader, Read};
        use std::path::{Path, PathBuf};

        /// Loader manifest.
        #[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
        #[serde(tag = "schema")]
        pub enum ModuleManifest {
            /// Version `0` manifest schema.
            #[serde(rename = "0")]
            V0 {
                /// Manifest contents.
                #[serde(flatten)]
                manifest: v0::LoaderManifest,
            },
        }

        /// Possible validation errors.
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum ValidationError {
            /// An IO error.
            IOError(std::io::Error),
            /// A serde error.
            SerdeError(serde_json::Error),
            /// The path does not contain a module.
            NotAModule(PathBuf),
            /// The path does not point to a manifest.
            NoManifest(PathBuf),
        }

        impl std::fmt::Display for ValidationError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    ValidationError::IOError(err) => {
                        write!(f, "IO error: {}", err)
                    }
                    ValidationError::SerdeError(err) => {
                        write!(f, "Serde error: {}", err)
                    }
                    ValidationError::NotAModule(path) => {
                        write!(f, "The path contains no module: path: {}", path.display())
                    }
                    ValidationError::NoManifest(path) => {
                        write!(
                            f,
                            "The path does not point to a manifest file: path: {}",
                            path.display()
                        )
                    }
                }
            }
        }

        mod v0 {
            use serde::Deserialize;
            use std::path::PathBuf;

            /// Loader manifest.
            #[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
            pub struct LoaderManifest {
                pub library_path: PathBuf,
            }
        }

        /// Parses the manifest using a reader.
        pub fn parse_manifest<R: Read>(
            reader: BufReader<R>,
        ) -> Result<ModuleManifest, ValidationError> {
            serde_json::from_reader(reader).map_err(ValidationError::SerdeError)
        }

        /// Parses the manifest from a module.
        pub fn parse_manifest_from_file(
            manifest_path: &Path,
        ) -> Result<ModuleManifest, ValidationError> {
            if !manifest_path.exists() || !manifest_path.is_file() {
                Err(ValidationError::NotAModule(manifest_path.to_path_buf()))
            } else {
                let file = match File::open(manifest_path) {
                    Ok(file) => file,
                    Err(e) => return Err(ValidationError::IOError(e)),
                };
                let reader = BufReader::new(file);
                parse_manifest(reader)
            }
        }

        /// Returns the path to the loader manifest.
        ///
        /// No value will be returned if `path` is not a module.
        pub fn get_manifest_path(path: &Path) -> Option<PathBuf> {
            if !path.is_dir() {
                None
            } else {
                let manifest_path = path.join("native_module.json");
                if !manifest_path.exists() || !manifest_path.is_file() {
                    None
                } else {
                    Some(manifest_path)
                }
            }
        }
    }
}
