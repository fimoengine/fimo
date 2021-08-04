//! Loader for Rust modules.
use crate::{
    ffi_loader::LoaderManifest, Module, ModuleInfo, ModuleInstance, ModuleLoader, ModulePtr,
};
use fimo_ffi_core::TypeWrapper;
use libloading::Library;
use std::any::Any;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::sync::{Arc, Weak};

/// Rust version the library was compiled with.
pub const RUSTC_VERSION: &str = env!("RUSTC_VERSION");

/// Core version the library was linked with.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Loader type of the Rust loader.
pub const MODULE_LOADER_TYPE: &str = "fimo_rust_loader";

/// Path from a module root to the manifest.
pub const MODULE_MANIFEST_PATH: &str = "rust_module.json";

/// Name of the module declaration.
pub const MODULE_DECLARATION_NAME: &str = "RUST_MODULE_DECLARATION";

const MODULE_DECLARATION_NAME_WITH_NULL: &[u8] = b"RUST_MODULE_DECLARATION\0";

/// Exports a module to enable its loading with the Rust loader.
#[macro_export]
macro_rules! export_rust_module {
    ($load_fn:expr) => {
        #[no_mangle]
        #[doc(hidden)]
        pub const RUST_MODULE_DECLARATION: $crate::rust_loader::ModuleDeclaration =
            $crate::rust_loader::ModuleDeclaration {
                rustc_version: $crate::rust_loader::RUSTC_VERSION,
                core_version: $crate::rust_loader::CORE_VERSION,
                load_fn: $load_fn,
            };
    };
}

/// A Rust module loader.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct RustLoader([u8; 0]);

/// Wrapper of a Rust module.
pub struct RustModule {
    library: ManuallyDrop<Library>,
    internal_module: ManuallyDrop<Box<dyn RustModuleExt>>,
}

/// Declaration of a module.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct ModuleDeclaration {
    /// Used Rust version.
    pub rustc_version: &'static str,
    /// Used Core version.
    pub core_version: &'static str,
    /// Load function.
    pub load_fn: ModuleLoadFn,
}

/// Extension of the [Module] trait for Rust modules.
pub trait RustModuleExt: Module {
    /// Sets the reference to the wrapping [RustModule].
    ///
    /// The handle must remain stored as a `Weak<T>` handle, as it otherwise
    /// prevents the dropping of the module. This handle must be used when
    /// constructing an instance.
    fn set_weak_parent_handle(&mut self, module: Weak<RustModule>);
}

/// Function pointer to the load function for a Rust module.
pub type ModuleLoadFn = TypeWrapper<
    unsafe extern "C-unwind" fn(
        Arc<dyn ModuleLoader>,
        *const Path,
    ) -> Result<Box<dyn RustModuleExt>, Box<dyn Error>>,
>;

impl RustLoader {
    /// Creates a new `RustLoader`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { 0: [0; 0] })
    }
}

impl ModuleLoader for RustLoader {
    fn get_raw_ptr(&self) -> ModulePtr {
        ModulePtr::Fat(unsafe { std::mem::transmute(self.as_any()) })
    }

    unsafe fn load_module(&self, path: &Path) -> Result<Arc<dyn Module>, Box<dyn Error>> {
        let manifest_path = path.join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path)?;
        let manifest: LoaderManifest = serde_json::from_reader(BufReader::new(file))?;

        let library = match manifest {
            LoaderManifest::V0 { library_path } => Library::new(library_path)?,
        };

        // SAFETY: A `RustLoader` is always in an `Arc`.
        let self_arc = {
            Arc::increment_strong_count(self as *const Self);
            Arc::from_raw(self as *const Self)
        };

        RustModule::new(library, self_arc, path).map(|module| module as Arc<dyn Module>)
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

impl RustModule {
    /// Creates a new module from a library.
    ///
    /// # Safety
    ///
    /// The library must export the module using the [export_rust_module!] macro.
    /// Results in an error if the Rust or Core version mismatches.
    pub unsafe fn new(
        library: Library,
        parent: Arc<RustLoader>,
        module_path: impl AsRef<Path>,
    ) -> Result<Arc<Self>, Box<dyn Error>> {
        let module_declaration = library
            .get::<*const ModuleDeclaration>(MODULE_DECLARATION_NAME_WITH_NULL)?
            .read();

        if module_declaration.rustc_version != RUSTC_VERSION
            || module_declaration.core_version != CORE_VERSION
        {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Version mismatch",
            )));
        }

        let internal_module = (module_declaration.load_fn)(parent, module_path.as_ref())?;

        let mut module = Arc::new(Self {
            library: ManuallyDrop::new(library),
            internal_module: ManuallyDrop::new(internal_module),
        });

        let weak_ref = Arc::downgrade(&module);
        Arc::get_mut(&mut module)
            .unwrap()
            .internal_module
            .set_weak_parent_handle(weak_ref);

        Ok(module)
    }
}

impl Module for RustModule {
    fn get_raw_ptr(&self) -> ModulePtr {
        if cfg!(any(windows, unix)) {
            sa::assert_eq_size!(*const u8, Library);
            ModulePtr::Slim(unsafe { std::mem::transmute_copy(&self.library) })
        } else {
            unimplemented!()
        }
    }

    fn get_module_path(&self) -> &Path {
        self.internal_module.get_module_path()
    }

    fn get_module_info(&self) -> &ModuleInfo {
        self.internal_module.get_module_info()
    }

    fn get_module_loader(&self) -> Arc<dyn ModuleLoader> {
        self.internal_module.get_module_loader()
    }

    fn create_instance(&self) -> Result<Arc<dyn ModuleInstance>, Box<dyn Error>> {
        self.internal_module.create_instance()
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self.internal_module.as_any()
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

impl Drop for RustModule {
    fn drop(&mut self) {
        // The module needs to be dropped first
        // SAFETY: The values are initialized.
        unsafe {
            ManuallyDrop::drop(&mut self.internal_module);
            ManuallyDrop::drop(&mut self.library);
        }
    }
}

impl std::fmt::Debug for RustModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.library.fmt(f)
    }
}
