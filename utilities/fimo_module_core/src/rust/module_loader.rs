//! Loader for rust modules.
use crate::rust::{
    Module, ModuleArc, ModuleCaster, ModuleLoader, ModuleLoaderVTable, ModuleObjCaster,
    ModuleObject, ModuleResult, ModuleVTable,
};
use crate::{DynArc, DynArcBase, DynArcCaster, ModulePtr};
use libloading::Library;
use parking_lot::Mutex;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

/// Rust version the library was compiled with.
pub const RUSTC_VERSION: &str = env!("RUSTC_VERSION");

/// Loader type of the Rust loader.
pub const MODULE_LOADER_TYPE: &str = "fimo_rust_loader";

/// Path from a module root to the manifest.
pub const MODULE_MANIFEST_PATH: &str = "rust_module.json";

/// Name of the module declaration.
pub const MODULE_DECLARATION_NAME: &str = "RUST_MODULE_DECLARATION";

const MODULE_DECLARATION_NAME_WITH_NULL: &[u8] = b"RUST_MODULE_DECLARATION\0";

/// Exports a module to enable its loading with the Rust loader.
#[macro_export]
macro_rules! _export_rust_module {
    ($load_fn:expr) => {
        #[no_mangle]
        #[doc(hidden)]
        pub static RUST_MODULE_DECLARATION: $crate::rust::module_loader::ModuleDeclaration =
            $crate::rust::module_loader::ModuleDeclaration {
                rustc_version: $crate::rust_loader::RUSTC_VERSION,
                load_fn: $load_fn,
            };
    };
}

const LOADER_VTABLE: ModuleLoaderVTable = ModuleLoaderVTable::new(
    |_ptr| {
        let vtable = &LOADER_VTABLE;
        ModulePtr::Slim(vtable as *const _ as *const u8)
    },
    |_ptr| "fimo::module_loader::rust",
    |ptr| {
        let loader = unsafe { &*(ptr as *const RustLoader) };
        loader.inner.evict_module_cache()
    },
    |ptr, path| {
        let loader = unsafe { &*(ptr as *const RustLoader) };
        let path = unsafe { &*(path as *const Path) };

        let manifest_path = path.join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path).map_err(|e| Box::new(e) as Box<dyn Error>)?;
        let manifest: LoaderManifest = serde_json::from_reader(BufReader::new(file))
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;

        unsafe {
            let library = match manifest {
                LoaderManifest::V0 { library_path, .. } => {
                    loader.inner.load_module_raw(library_path.as_path())?
                }
            };

            RustModule::from_library(library, loader, path)
        }
    },
    |ptr, path| {
        let loader = unsafe { &*(ptr as *const RustLoader) };
        let path = unsafe { &*(path as *const Path) };

        unsafe {
            let library = loader.inner.load_module_raw(path)?;
            RustModule::from_library(library, loader, path)
        }
    },
);

const MODULE_VTABLE: ModuleVTable = ModuleVTable::new(
    |ptr| {
        let module = unsafe { &*(ptr as *const RustModule) };
        if cfg!(any(windows, unix)) {
            sa::assert_eq_size!(*const u8, Library);
            ModulePtr::Slim(unsafe { std::mem::transmute_copy(&module.library) })
        } else {
            unimplemented!()
        }
    },
    |_ptr| "fimo::module::rust",
    |ptr| {
        let module = unsafe { &*(ptr as *const RustModule) };
        module.path.as_path()
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const RustModule) };
        module.module.as_module().get_module_info()
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const RustModule) };
        &**module.parent
    },
    |ptr| {
        let module = unsafe { &*(ptr as *const RustModule) };
        module.module.as_module().create_instance()
    },
);

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
}

impl Deref for RustLoader {
    type Target = ModuleLoader;

    fn deref(&self) -> &Self::Target {
        let self_ptr = self as *const _ as *const ();
        let vtable = &LOADER_VTABLE;

        unsafe { &*ModuleLoader::from_raw_parts(self_ptr, vtable) }
    }
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

    unsafe fn load_module_raw(&'static self, path: &Path) -> ModuleResult<Arc<Library>> {
        let library = Arc::new(Library::new(path)?);
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
    path: PathBuf,
    parent: &'static RustLoader,
    module: RustModuleInnerArc,
    library: Arc<Library>,
}

impl RustModule {
    #[inline]
    unsafe fn from_library(
        library: Arc<Library>,
        parent: &'static RustLoader,
        path: &Path,
    ) -> ModuleResult<ModuleArc> {
        let module_declaration = library
            .get::<*const ModuleDeclaration>(MODULE_DECLARATION_NAME_WITH_NULL)?
            .read();

        if module_declaration.rustc_version != RUSTC_VERSION {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Compiler version mismatch",
            )));
        }

        let inner = (module_declaration.load_fn)()?;
        let mut module = Arc::new(Self {
            path: path.to_path_buf(),
            library,
            parent,
            module: inner,
        });

        let weak_module = Arc::downgrade(&module);
        let m = Arc::get_mut_unchecked(&mut module);
        let inner = RustModuleInnerArc::get_mut(&mut m.module).unwrap();
        inner.set_parent_handle(weak_module);

        let caster = ModuleCaster::new(&MODULE_VTABLE);
        Ok(ModuleArc::from_inner((module, caster)))
    }
}

impl RustModuleInner {
    /// Sets the reference to the wrapping [RustModule].
    ///
    /// The handle must remain stored as a `Weak<T>` handle, as it otherwise
    /// prevents the dropping of the module. This handle must be used when
    /// constructing an instance.
    #[inline]
    pub fn set_parent_handle(&mut self, module: Weak<RustModule>) {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.set_parent_handle)(ptr, module)
    }

    /// Coerces the rust module to a [`Module`].
    #[inline]
    pub fn as_module(&self) -> &Module {
        let (ptr, vtable) = self.into_raw_parts();
        unsafe { &*(vtable.as_module)(ptr) }
    }
}

/// A type-erased rust module.
pub type RustModuleInner = ModuleObject<RustModuleInnerVTable>;

/// [`DynArc`] caster for a [`RustModuleInner`].
pub type RustModuleInnerCaster = ModuleObjCaster<RustModuleInnerVTable>;

/// A [`DynArc`] for a [`RustModuleInner`].
pub type RustModuleInnerArc = DynArc<RustModuleInner, RustModuleInnerCaster>;

/// Extension of the [ModuleVTable] for Rust modules.
#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RustModuleInnerVTable {
    set_parent_handle: fn(*mut (), Weak<RustModule>),
    as_module: fn(*const ()) -> *const Module,
}

impl RustModuleInnerVTable {
    /// Constructs a new `RustModuleInnerVTable`.
    pub fn new(
        set_parent_handle: fn(*mut (), Weak<RustModule>),
        as_module: fn(*const ()) -> *const Module,
    ) -> Self {
        Self {
            set_parent_handle,
            as_module,
        }
    }
}

impl DynArcCaster<RustModuleInner> for ModuleObjCaster<RustModuleInnerVTable> {
    unsafe fn as_self_ptr<'a>(&self, base: *const (dyn DynArcBase + 'a)) -> *const RustModuleInner {
        let ptr = base as *const _ as *const ();
        RustModuleInner::from_raw_parts(ptr, self.vtable)
    }
}

/// Rust module manifest.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
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
    pub load_fn: extern "C" fn() -> ModuleResult<RustModuleInnerArc>,
}
