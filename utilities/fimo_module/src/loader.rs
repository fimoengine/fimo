//! Loader for rust modules.
use crate::{Error, ErrorKind, IModule, IModuleLoader};
use fimo_ffi::error::wrap_error;
use fimo_ffi::ptr::{coerce_obj, IBase};
use fimo_ffi::{DynObj, ObjArc, ObjectId};
use libloading::Library;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;
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
        pub static RUST_MODULE_DECLARATION: $crate::loader::ModuleDeclaration =
            $crate::loader::ModuleDeclaration {
                rustc_version: $crate::loader::RUSTC_VERSION,
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
#[derive(Copy, Clone)]
pub struct ModuleDeclaration {
    /// Used Rust version.
    pub rustc_version: &'static str,
    /// Load function.
    #[allow(clippy::type_complexity)]
    pub load_fn: unsafe fn(
        &'static DynObj<dyn IModuleLoader>,
        &'_ Path,
    ) -> Result<ObjArc<DynObj<dyn IModule>>, Error>,
}

impl Debug for ModuleDeclaration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleDeclaration")
            .field("rustc_version", &self.rustc_version)
            .finish()
    }
}

/// A Rust module loader.
#[derive(Debug, ObjectId)]
#[fetch_vtable(
    uuid = "1aef7722-f3a9-43e9-a27b-753510e42700",
    interfaces(IModuleLoader)
)]
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

impl IModuleLoader for RustLoader {
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync> {
        coerce_obj(self)
    }

    fn evict_module_cache(&self) {
        self.inner.evict_module_cache()
    }

    unsafe fn load_module(
        &'static self,
        path: &'_ Path,
    ) -> crate::Result<ObjArc<DynObj<dyn IModule>>> {
        let manifest_path = path.join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path)
            .map_err(|e| Error::new(ErrorKind::Internal, wrap_error(e)))?;
        let buf_reader = BufReader::new(file);
        let manifest: LoaderManifest = serde_json::from_reader(buf_reader)
            .map_err(|e| Error::new(ErrorKind::Internal, wrap_error(e)))?;

        match manifest {
            LoaderManifest::V0 { library_path, .. } => self.load_module_raw(library_path.as_path()),
        }
    }

    unsafe fn load_module_raw(
        &'static self,
        path: &'_ Path,
    ) -> crate::Result<ObjArc<DynObj<dyn IModule>>> {
        let lib = self.inner.load_module_raw(path)?;

        let module_declaration =
            match lib.get::<*const ModuleDeclaration>(MODULE_DECLARATION_NAME_WITH_NULL) {
                Ok(s) => s,
                Err(e) => return Err(Error::new(ErrorKind::Internal, wrap_error(e))),
            };

        let module_declaration = **module_declaration;

        if module_declaration.rustc_version != RUSTC_VERSION {
            return Err(Error::new(
                ErrorKind::FailedPrecondition,
                "Compiler version mismatch",
            ));
        }

        (module_declaration.load_fn)(coerce_obj(self), path)
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

    unsafe fn load_module_raw(&'static self, path: &Path) -> Result<Arc<Library>, Error> {
        let library = match Library::new(path) {
            Ok(l) => l,
            Err(e) => return Err(Error::new(ErrorKind::Unknown, wrap_error(e))),
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
