//! Loader for Rust modules.
use crate::{
    ffi_loader::LoaderManifest, Module, ModuleInfo, ModuleInstance, ModuleLoader, ModulePtr,
};
use fimo_ffi_core::TypeWrapper;
use libloading::Library;
use parking_lot::Mutex;
use std::any::Any;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
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
        pub static RUST_MODULE_DECLARATION: $crate::rust_loader::ModuleDeclaration =
            $crate::rust_loader::ModuleDeclaration {
                rustc_version: $crate::rust_loader::RUSTC_VERSION,
                core_version: $crate::rust_loader::CORE_VERSION,
                load_fn: $load_fn,
            };
    };
}

/// A Rust module loader.
#[derive(Debug)]
pub struct RustLoader {
    libs: Mutex<Vec<Arc<Library>>>,
}

/// Wrapper of a Rust module.
pub struct RustModule {
    path: PathBuf,
    parent: &'static RustLoader,
    internal_module: Box<dyn RustModuleExt>,
    library: Arc<Library>,
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

    /// Casts itself to a `&(dyn Module + 'static)`.
    fn as_module(&self) -> &(dyn Module + 'static);

    /// Casts itself to a `&mut (dyn Module + 'static)`.
    fn as_module_mut(&mut self) -> &mut (dyn Module + 'static);
}

/// Function pointer to the load function for a Rust module.
pub type ModuleLoadFn =
    TypeWrapper<unsafe extern "C-unwind" fn() -> Result<Box<dyn RustModuleExt>, Box<dyn Error>>>;

impl RustLoader {
    /// Creates a new `RustLoader`.
    pub fn new() -> &'static mut Self {
        Box::leak(Box::new(Self {
            libs: Default::default(),
        }))
    }
}

impl ModuleLoader for RustLoader {
    fn get_raw_ptr(&self) -> ModulePtr {
        ModulePtr::Fat(unsafe { std::mem::transmute(self.as_any()) })
    }

    unsafe fn load_module(&'static self, path: &Path) -> Result<Arc<dyn Module>, Box<dyn Error>> {
        let manifest_path = path.join(MODULE_MANIFEST_PATH);
        let file = File::open(manifest_path)?;
        let manifest: LoaderManifest = serde_json::from_reader(BufReader::new(file))?;

        let library = match manifest {
            LoaderManifest::V0 { library_path } => Arc::new(Library::new(library_path)?),
        };

        self.libs.lock().push(library.clone());
        RustModule::new(library, self, path).map(|module| module as Arc<dyn Module>)
    }

    unsafe fn load_module_library(
        &'static self,
        path: &Path,
    ) -> Result<Arc<dyn Module>, Box<dyn Error>> {
        let library = Arc::new(Library::new(path)?);
        let module_dir = path.parent().unwrap();

        self.libs.lock().push(library.clone());
        RustModule::new(library, self, module_dir).map(|module| module as Arc<dyn Module>)
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
        library: Arc<Library>,
        parent: &'static RustLoader,
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

        let internal_module = (module_declaration.load_fn)()?;

        let mut module = Arc::new(Self {
            path: module_path.as_ref().to_path_buf(),
            library,
            parent,
            internal_module,
        });

        let weak_ref = Arc::downgrade(&module);
        Arc::get_mut_unchecked(&mut module)
            .internal_module
            .set_weak_parent_handle(weak_ref);

        Ok(module)
    }

    /// Extracts a reference to the internal module.
    pub fn as_internal(&self) -> &(dyn Module + 'static) {
        self.internal_module.as_module()
    }

    /// Extracts a mutable reference to the internal module.
    pub fn as_internal_mut(&mut self) -> &mut (dyn Module + 'static) {
        self.internal_module.as_module_mut()
    }
}

impl Drop for RustLoader {
    fn drop(&mut self) {
        let libs = self.libs.get_mut();
        libs.retain(|lib| !(Arc::strong_count(lib) == 1 && Arc::weak_count(lib) == 0));

        if !libs.is_empty() {
            panic!("not all libraries were unloaded!")
        }
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
        self.path.as_path()
    }

    fn get_module_info(&self) -> &ModuleInfo {
        self.internal_module.get_module_info()
    }

    fn get_module_loader(&self) -> &'static (dyn ModuleLoader + 'static) {
        self.parent
    }

    fn create_instance(&self) -> Result<Arc<dyn ModuleInstance>, Box<dyn Error>> {
        self.internal_module.create_instance()
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

impl std::fmt::Debug for RustModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.library.fmt(f)
    }
}
