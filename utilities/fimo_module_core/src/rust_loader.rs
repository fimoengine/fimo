//! Loader for rust modules.
use crate::{
    Error, ErrorKind, IModule, IModuleInstance, IModuleLoader, IModuleLoaderVTable, IModuleVTable,
    ModuleInfo, PathChar, SendSyncMarker,
};
use fimo_ffi::object::ObjectWrapper;
use fimo_ffi::vtable::IBaseInterface;
use fimo_ffi::{fimo_object, fimo_vtable, impl_vtable, is_object, ObjArc, ObjWeak, SpanInner};
use libloading::Library;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Rust version the library was compiled with.
pub const RUSTC_VERSION: &str = env!("RUSTC_VERSION");

/// Loader type of the Rust loader.
pub const MODULE_LOADER_TYPE: &str = "fimo::module::loader::rust";

/// Path from a module root to the manifest.
pub const MODULE_MANIFEST_PATH: &str = "rust_module.json";

/// Name of the module declaration.
pub const MODULE_DECLARATION_NAME: &str = "RUST_MODULE_DECLARATION";

const MODULE_DECLARATION_NAME_WITH_NULL: &[u8] = b"RUST_MODULE_DECLARATION\0";

/// Exports a module to enable its loading with the Rust loader.
#[macro_export]
macro_rules! rust_module {
    ($load:expr) => {
        #[no_mangle]
        #[doc(hidden)]
        pub static RUST_MODULE_DECLARATION: $crate::rust_loader::ModuleDeclaration =
            $crate::rust_loader::ModuleDeclaration {
                rustc_version: $crate::rust_loader::RUSTC_VERSION,
                load_fn: $load,
            };
    };
}

/// Rust module manifest.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema")]
pub enum LoaderManifest {
    /// Version `0` manifest schema.
    #[serde(rename = "0")]
    V0 {
        /// Path to the library.
        library_path: PathBuf,
        /// Version of the compiler, the module was compiled with.
        compiler_version: String,
    },
}

/// Declaration of a module.
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct ModuleDeclaration {
    /// Used Rust version.
    pub rustc_version: &'static str,
    /// Load function.
    pub load_fn: extern "C" fn() -> Result<ObjArc<IRustModuleInner>, Error>,
}

/// A Rust module loader.
#[derive(Debug)]
pub struct RustLoader {
    inner: RawLoader,
}

impl RustLoader {
    /// Creates a new `RustLoader` reference.
    pub fn new() -> &'static Self {
        let loader = Box::new(Self {
            inner: RawLoader::new(),
        });
        Box::leak(loader)
    }

    /// Unloads not needed modules.
    pub fn evict_module_cache(&self) {
        self.inner.evict_module_cache()
    }

    /// Loads a new module from it's root.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behaviour.
    pub unsafe fn load_module(&'static self, path: &Path) -> Result<ObjArc<RustModule>, Error> {
        let manifest_path = path.join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path).map_err(|e| Error::new(ErrorKind::Internal, e))?;
        let buf_reader = BufReader::new(file);
        let manifest: LoaderManifest =
            serde_json::from_reader(buf_reader).map_err(|e| Error::new(ErrorKind::Internal, e))?;

        match manifest {
            LoaderManifest::V0 { library_path, .. } => self.load_module_raw(library_path.as_path()),
        }
    }

    /// Loads a new module from it's library path.
    ///
    /// # Safety
    ///
    /// - The module must be exposed in a way understood by the module loader.
    /// - The module ABI must match the loader ABI.
    ///
    /// Violating these invariants may lead to undefined behaviour.
    pub unsafe fn load_module_raw(&'static self, path: &Path) -> Result<ObjArc<RustModule>, Error> {
        let lib = self.inner.load_module_raw(path)?;
        RustModule::new(lib, self, path)
    }
}

is_object! { #![uuid(0x1aef7722, 0xf3a9, 0x43e9, 0xa27b, 0x753510e42700)] RustLoader }

impl_vtable! {
    impl IModuleLoaderVTable => RustLoader {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBaseInterface {
            &*(&__VTABLE as *const _ as *const IBaseInterface)
        }

        unsafe extern "C" fn evict(ptr: *const ()) {
            let this = &*(ptr as *const RustLoader);
            this.evict_module_cache()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn load_module(
            ptr: *const (),
            path: SpanInner<PathChar, false>,
        ) -> crate::Result<ObjArc<IModule>> {
            let this = &*(ptr as *const RustLoader);
            let path: &[PathChar] = path.into();
            let path = to_path_buf(path);
            let m = this.load_module(path.as_path());
            let m = m.map(ObjArc::coerce_object);
            From::from(m)
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn load_module_raw(
            ptr: *const (),
            path: SpanInner<PathChar, false>,
        ) -> crate::Result<ObjArc<IModule>> {
            let this = &*(ptr as *const RustLoader);
            let path: &[PathChar] = path.into();
            let path = to_path_buf(path);
            let m = this.load_module_raw(path.as_path());
            let m = m.map(ObjArc::coerce_object);
            From::from(m)
        }
    }
}

#[cfg(unix)]
fn to_path_buf(path: &[PathChar]) -> PathBuf {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let v = Vec::from(path);
    PathBuf::from(OsString::from_vec(v))
}
#[cfg(windows)]
fn to_path_buf(path: &[PathChar]) -> PathBuf {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    PathBuf::from(OsString::from_wide(path))
}

#[derive(Debug)]
pub(crate) struct RawLoader {
    libs: Mutex<Vec<Arc<Library>>>,
}

impl RawLoader {
    pub fn new() -> Self {
        Self {
            libs: Default::default(),
        }
    }

    pub fn evict_module_cache(&self) {
        self.libs
            .lock()
            .retain(|lib| !(Arc::strong_count(lib) == 1 && Arc::weak_count(lib) == 0));
    }

    unsafe fn load_module_raw(&'static self, path: &Path) -> Result<Arc<Library>, Error> {
        let library = match Library::new(path) {
            Ok(l) => l,
            Err(e) => return Err(Error::new(ErrorKind::Unknown, e)),
        };
        let library = Arc::new(library);
        self.libs.lock().push(library.clone());
        Ok(library)
    }
}

impl Drop for RawLoader {
    fn drop(&mut self) {
        self.evict_module_cache();
        if !self.libs.get_mut().is_empty() {
            panic!("not all libraries were unloaded!")
        }
    }
}

/// Wrapper for a rust module.
#[derive(Debug)]
pub struct RustModule {
    path: Vec<PathChar>,
    parent: &'static RustLoader,
    module: ObjArc<IRustModuleInner>,
    _library: Arc<Library>,
}

impl RustModule {
    #[cfg(unix)]
    fn path_to_vec(p: &Path) -> Vec<PathChar> {
        use std::os::unix::ffi::OsStrExt;
        Vec::from(p.as_os_str().as_bytes())
    }

    #[cfg(windows)]
    fn path_to_vec(p: &Path) -> Vec<PathChar> {
        use std::os::windows::ffi::OsStrExt;
        p.as_os_str().encode_wide().collect()
    }

    unsafe fn new(
        lib: Arc<Library>,
        parent: &'static RustLoader,
        path: &Path,
    ) -> Result<ObjArc<Self>, Error> {
        let module_declaration =
            match lib.get::<*const ModuleDeclaration>(MODULE_DECLARATION_NAME_WITH_NULL) {
                Ok(s) => s,
                Err(e) => return Err(Error::new(ErrorKind::Internal, e)),
            };

        let module_declaration = **module_declaration;

        if module_declaration.rustc_version != RUSTC_VERSION {
            return Err(Error::new(
                ErrorKind::FailedPrecondition,
                "Compiler version mismatch",
            ));
        }

        let inner = (module_declaration.load_fn)()?;
        let mut module = ObjArc::new(Self {
            path: Self::path_to_vec(path),
            _library: lib,
            parent,
            module: inner,
        });

        let weak_module: ObjWeak<IRustModuleParent> =
            ObjWeak::coerce_object(ObjArc::downgrade(&module));
        let m = ObjArc::get_mut_unchecked(&mut module);
        let inner = ObjArc::get_mut_unchecked(&mut m.module);
        inner.set_parent_handle(weak_module);

        Ok(module)
    }

    /// Extracts the path of the module.
    #[inline]
    pub fn module_path(&self) -> &[PathChar] {
        self.path.as_slice()
    }

    /// Constructs a [`PathBuf`] containing the path to the module root.
    #[inline]
    pub fn module_path_buf(&self) -> PathBuf {
        to_path_buf(self.module_path())
    }

    /// Extracts a reference to the [`ModuleInfo`].
    #[inline]
    pub fn module_info(&self) -> &ModuleInfo {
        self.module.module_info()
    }

    /// Extracts a reference to the loader.
    #[inline]
    pub fn module_loader(&self) -> &'static RustLoader {
        self.parent
    }

    /// Instantiates the module.
    #[inline]
    pub fn new_instance(&self) -> Result<ObjArc<IModuleInstance>, Error> {
        self.module.new_instance().into_rust()
    }
}

is_object! { #![uuid(0xb61d9c2c, 0xc739, 0x426f, 0x8ba7, 0xb8e2acb4cbbd)] RustModule }

impl_vtable! {
    impl inline IRustModuleParentVTable => RustModule {
        |_ptr| RustModule::get_vtable(),
        |ptr| {
            let this = unsafe { &*(ptr as *const RustModule) };
            this.module_path().into()
        },
        |ptr| unsafe {
            let this = &*(ptr as *const RustModule);
            IModuleLoader::from_object(this.module_loader().coerce_obj())
        },
    }
}

impl_vtable! {
    impl IModuleVTable => RustModule {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBaseInterface {
            &*(&__VTABLE as *const _ as *const IBaseInterface)
        }

        unsafe extern "C" fn module_path(ptr: *const ()) -> SpanInner<PathChar, false> {
            let this = &*(ptr as *const RustModule);
            this.module_path().into()
        }

        unsafe extern "C" fn module_info(ptr: *const ()) -> *const ModuleInfo {
            let this = &*(ptr as *const RustModule);
            this.module_info()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn module_loader(ptr: *const ()) -> &'static IModuleLoader {
            let this = &*(ptr as *const RustModule);
            IModuleLoader::from_object(this.module_loader().coerce_obj())
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn new_instance(
            ptr: *const (),
        ) -> crate::Result<ObjArc<IModuleInstance>> {
            let this = &*(ptr as *const RustModule);
            this.new_instance().into()
        }
    }
}

fimo_object! {
    /// Parent of a type-erased rust module.
    ///
    /// Implements a part of the [`Module`] interface.
    #![vtable = IRustModuleParentVTable]
    pub struct IRustModuleParent;
}

impl IRustModuleParent {
    /// Coerces the rust module to a [`IModule`].
    #[inline]
    pub fn as_module(&self) -> &IModule {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let vtable = (vtable.as_module)(ptr);
            &*IModule::from_raw_parts(ptr, vtable)
        }
    }

    /// Fetches the path to the module root.
    pub fn module_path(&self) -> SpanInner<PathChar, false> {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.module_path)(ptr) }
    }

    /// Fetches a pointer to the [`IModuleLoader`] which loaded the module.
    pub fn module_loader(&self) -> &'static IModuleLoader {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { (vtable.module_loader)(ptr) }
    }
}

fimo_vtable! {
    /// VTable of a [`IRustModuleParent`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0x84adcffb, 0x0aea, 0x4181, 0xa0bb, 0x32b78e14c882)]
    pub struct IRustModuleParentVTable {
        /// Fetches the module interface for the rust module.
        pub as_module: unsafe fn(*const ()) -> &'static IModuleVTable,
        /// Fetches the path to the module root.
        pub module_path: unsafe fn(*const ()) -> SpanInner<PathChar, false>,
        /// Fetches a pointer to the [`ModuleLoader`] which loaded the module.
        pub module_loader: unsafe fn(*const ()) -> &'static IModuleLoader,
    }
}

fimo_object! {
    /// A type-erased rust module.
    #![vtable = IRustModuleInnerVTable]
    pub struct IRustModuleInner;
}

impl IRustModuleInner {
    /// Coerces the rust module to a [`IModule`].
    #[inline]
    pub fn as_module(&self) -> &IModule {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            let vtable = (vtable.as_module)(ptr);
            &*IModule::from_raw_parts(ptr, vtable)
        }
    }

    /// Sets the reference to the wrapping [`IRustModuleParent`].
    ///
    /// The handle must remain stored as a `ObjWeak<T>` handle, as it otherwise
    /// prevents the dropping of the module. This handle must be used when
    /// constructing an instance.
    ///
    /// # Safety
    ///
    /// May only be called once during the initialization.
    pub unsafe fn set_parent_handle(&mut self, module: ObjWeak<IRustModuleParent>) {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.set_parent_handle)(ptr, module)
    }
}

impl Deref for IRustModuleInner {
    type Target = IModule;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_module()
    }
}

fimo_vtable! {
    /// VTable of a [`IRustModuleInner`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSyncMarker]
    #![uuid(0x9068933c, 0xec51, 0x4146, 0x9b03, 0x68ea717494f4)]
    pub struct IRustModuleInnerVTable {
        // The functions don't need the C ABI, as by this point we already ensured
        // that the module was compiled with the same version of the compiler.
        /// Fetches the module interface for the rust module.
        pub as_module: unsafe fn(*const ()) -> &'static IModuleVTable,
        /// Sets the reference to the wrapping [`IRustModuleParent`].
        ///
        /// The handle must remain stored as a `ObjWeak<T>` handle, as it otherwise
        /// prevents the dropping of the module. This handle must be used when
        /// constructing an instance.
        pub set_parent_handle: unsafe fn(*mut (), ObjWeak<IRustModuleParent>)
    }
}
