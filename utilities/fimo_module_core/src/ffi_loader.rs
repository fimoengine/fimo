//! Loader for ffi modules.
use crate::{
    Module, ModuleInfo, ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader,
    ModulePtr,
};
use fimo_ffi_core::{ConstSpan, NonNullConst, TypeWrapper};
use libloading::Library;
use parking_lot::Mutex;
use serde::Deserialize;
use std::any::Any;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Loader type of the FFI loader.
pub const MODULE_LOADER_TYPE: &str = "fimo_ffi_loader";

/// Path from a module root to the manifest.
pub const MODULE_MANIFEST_PATH: &str = "ffi_module.json";

/// Name of the module declaration.
pub const MODULE_DECLARATION_NAME: &str = "MODULE_DECLARATION";

const MODULE_DECLARATION_NAME_WITH_NULL: &[u8] = b"MODULE_DECLARATION\0";

/// Exports a module to enable its loading with the FFI loader.
#[macro_export]
macro_rules! export_ffi_module {
    ($get_module_info:expr, $create_instance:expr, $get_exportable_interfaces:expr,
    $get_interface_dependencies:expr, $get_interface:expr) => {
        #[no_mangle]
        #[doc(hidden)]
        pub static MODULE_DECLARATION: $crate::ffi_loader::FFIModuleVTable =
            $crate::ffi_loader::FFIModuleVTable {
                get_module_info_fn: $get_module_info,
                create_instance_fn: $create_instance,
                get_exportable_interfaces_fn: $get_exportable_interfaces,
                get_interface_dependencies_fn: $get_interface_dependencies,
                get_interface_fn: $get_interface,
            };
    };
}

/// Module manifest.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(tag = "schema")]
pub enum LoaderManifest {
    /// Version `0` manifest schema.
    #[serde(rename = "0")]
    V0 {
        /// Path to the library.
        library_path: PathBuf,
    },
}

/// A FFI module loader.
#[derive(Debug)]
pub struct FFIModuleLoader {
    libs: Mutex<Vec<Arc<Library>>>,
}

/// A FFI module.
#[derive(Debug)]
pub struct FFIModule {
    library: Arc<Library>,
    module_path: PathBuf,
    parent: &'static FFIModuleLoader,
    module_vtable: NonNullConst<FFIModuleVTable>,
}

/// A FFI module instance.
#[derive(Debug)]
pub struct FFIModuleInstance {
    parent: Arc<FFIModule>,
    instance_ptr: fimo_ffi_core::Arc<ModulePtr>,
    module_vtable: NonNullConst<FFIModuleVTable>,
}

/// A FFI module interface.
#[derive(Debug)]
pub struct FFIModuleInterface {
    parent: Arc<FFIModuleInstance>,
    interface_ptr: fimo_ffi_core::Arc<ModulePtr>,
}

/// A FFI module vtable.
#[repr(C)]
#[derive(Debug)]
pub struct FFIModuleVTable {
    /// `get_module_info` function.
    pub get_module_info_fn: ModuleGetModuleInfoFn,
    /// `create_instance` function.
    pub create_instance_fn: ModuleCreateInstanceFn,
    /// `get_exportable_interfaces` function.
    pub get_exportable_interfaces_fn: ModuleInstanceGetExportableInterfacesFn,
    /// `get_interface_dependencies` function.
    pub get_interface_dependencies_fn: ModuleInstanceGetInterfaceDependenciesFn,
    /// `get_interface` function.
    pub get_interface_fn: ModuleInstanceGetInterfaceFn,
}

/// A function pointer to the `get_module_info` function of a `FFIModule`.
pub type ModuleGetModuleInfoFn =
    TypeWrapper<unsafe extern "C-unwind" fn() -> fimo_ffi_core::NonNullConst<ModuleInfo>>;

/// A function pointer to the `create_instance` function of a `FFIModule`.
pub type ModuleCreateInstanceFn = TypeWrapper<
    unsafe extern "C-unwind" fn() -> fimo_ffi_core::Result<
        fimo_ffi_core::Arc<ModulePtr>,
        fimo_ffi_core::Error,
    >,
>;

/// A function pointer to the `get_exportable_interfaces` function of a `FFIModuleInstance`.
pub type ModuleInstanceGetExportableInterfacesFn =
    TypeWrapper<unsafe extern "C-unwind" fn(ModulePtr) -> ConstSpan<ModuleInterfaceDescriptor>>;

/// A function pointer to the `get_interface_dependencies` function of a `FFIModuleInstance`.
pub type ModuleInstanceGetInterfaceDependenciesFn = TypeWrapper<
    unsafe extern "C-unwind" fn(
        ModulePtr,
        NonNullConst<ModuleInterfaceDescriptor>,
    ) -> fimo_ffi_core::Result<
        ConstSpan<ModuleInterfaceDescriptor>,
        fimo_ffi_core::Error,
    >,
>;

/// A function pointer to the `get_interface` function of a `FFIModuleInstance`.
pub type ModuleInstanceGetInterfaceFn = TypeWrapper<
    unsafe extern "C-unwind" fn(
        ModulePtr,
        NonNullConst<ModuleInterfaceDescriptor>,
    ) -> fimo_ffi_core::Result<
        fimo_ffi_core::Arc<ModulePtr>,
        fimo_ffi_core::Error,
    >,
>;

impl FFIModuleLoader {
    /// Creates a new `FFIModuleLoader`.
    pub fn new() -> &'static mut Self {
        Box::leak(Box::new(Self {
            libs: Default::default(),
        }))
    }
}

impl Drop for FFIModuleLoader {
    fn drop(&mut self) {
        let libs = self.libs.get_mut();
        libs.retain(|lib| !(Arc::strong_count(lib) == 1 && Arc::weak_count(lib) == 0));

        if !libs.is_empty() {
            panic!("not all libraries were unloaded!")
        }
    }
}

impl ModuleLoader for FFIModuleLoader {
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
        FFIModule::new(library, self, path).map(|module| module as Arc<dyn Module>)
    }

    unsafe fn load_module_library(
        &'static self,
        path: &Path,
    ) -> Result<Arc<dyn Module>, Box<dyn Error>> {
        let library = Arc::new(Library::new(path)?);

        self.libs.lock().push(library.clone());
        FFIModule::new(library, self, path).map(|module| module as Arc<dyn Module>)
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

impl FFIModule {
    /// Creates a new module from a library.
    ///
    /// # Safety
    ///
    /// The library must export the module using the [export_ffi_module!] macro.
    pub unsafe fn new(
        library: Arc<Library>,
        parent: &'static FFIModuleLoader,
        module_path: impl AsRef<Path>,
    ) -> Result<Arc<Self>, Box<dyn Error>> {
        let module_vtable = NonNullConst::new_unchecked(
            *library.get::<*const FFIModuleVTable>(MODULE_DECLARATION_NAME_WITH_NULL)?,
        );

        Ok(Arc::new(Self {
            library,
            module_path: module_path.as_ref().to_path_buf(),
            parent,
            module_vtable,
        }))
    }
}

impl Module for FFIModule {
    fn get_raw_ptr(&self) -> ModulePtr {
        if cfg!(any(windows, unix)) {
            sa::assert_eq_size!(*const u8, Library);
            ModulePtr::Slim(unsafe { std::mem::transmute_copy(&self.library) })
        } else {
            unimplemented!()
        }
    }

    fn get_module_path(&self) -> &Path {
        self.module_path.as_path()
    }

    fn get_module_info(&self) -> &ModuleInfo {
        unsafe { &*(self.module_vtable.as_ref().get_module_info_fn)().as_ptr() }
    }

    fn get_module_loader(&self) -> &'static (dyn ModuleLoader + 'static) {
        self.parent
    }

    fn create_instance(&self) -> Result<Arc<dyn ModuleInstance>, Box<dyn Error>> {
        // SAFETY: A `FFIModule` is always in an `Arc`.
        let self_arc = unsafe {
            Arc::increment_strong_count(self as *const Self);
            Arc::from_raw(self as *const Self)
        };

        FFIModuleInstance::new(self_arc).map(|instance| instance as Arc<dyn ModuleInstance>)
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

// SAFETY: The implementation of a module is always `Send + Sync`.
unsafe impl Send for FFIModule {}
unsafe impl Sync for FFIModule {}

impl FFIModuleInstance {
    /// Creates a `FFIModuleInstance` from a `FFIModule`.
    pub fn new(parent: Arc<FFIModule>) -> Result<Arc<Self>, Box<dyn Error>> {
        // SAFETY: The pointer is valid.
        let instance_ptr =
            unsafe { (parent.module_vtable.as_ref().create_instance_fn)().into_rust()? };
        let module_vtable = parent.module_vtable;

        Ok(Arc::new(Self {
            parent,
            instance_ptr,
            module_vtable,
        }))
    }
}

impl ModuleInstance for FFIModuleInstance {
    fn get_raw_ptr(&self) -> ModulePtr {
        *self.instance_ptr
    }

    fn get_module(&self) -> Arc<dyn Module> {
        self.parent.clone()
    }

    fn get_available_interfaces(&self) -> &[ModuleInterfaceDescriptor] {
        // SAFETY: All pointers are valid.
        let descriptors = unsafe {
            (self.module_vtable.as_ref().get_exportable_interfaces_fn)(*self.instance_ptr)
        };

        unsafe {
            fimo_ffi_core::span::slice_from_raw_parts(descriptors.as_ptr(), descriptors.len())
        }
    }

    fn get_interface(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>> {
        // SAFETY: A `FFIModuleInstance` is always in an `Arc`.
        let self_arc = unsafe {
            Arc::increment_strong_count(self as *const Self);
            Arc::from_raw(self as *const Self)
        };

        FFIModuleInterface::new(self_arc, interface)
            .map(|interface| interface as Arc<dyn ModuleInterface>)
    }

    fn get_interface_dependencies(
        &self,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<&[ModuleInterfaceDescriptor], Box<dyn Error>> {
        // All pointers are valid.
        let descriptors = unsafe {
            (self.module_vtable.as_ref().get_interface_dependencies_fn)(
                *self.instance_ptr,
                NonNullConst::from(interface),
            )
            .into_rust()
            .map(|descriptors| {
                fimo_ffi_core::span::slice_from_raw_parts(descriptors.as_ptr(), descriptors.len())
            })?
        };

        Ok(descriptors)
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

// SAFETY: Module instances are always `Send + Sync`.
unsafe impl Send for FFIModuleInstance {}
unsafe impl Sync for FFIModuleInstance {}

impl FFIModuleInterface {
    /// Creates a new `FFIModuleInterface` from a `FFIModuleInstance`.
    pub fn new(
        parent: Arc<FFIModuleInstance>,
        interface: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<Self>, Box<dyn Error>> {
        // SAFETY: All pointers are valid.
        let interface_ptr = unsafe {
            (parent.module_vtable.as_ref().get_interface_fn)(
                *parent.instance_ptr,
                NonNullConst::from(interface),
            )
            .into_rust()?
        };

        Ok(Arc::new(Self {
            parent,
            interface_ptr,
        }))
    }
}

impl ModuleInterface for FFIModuleInterface {
    fn get_raw_ptr(&self) -> ModulePtr {
        *self.interface_ptr
    }

    fn get_instance(&self) -> Arc<dyn ModuleInstance> {
        self.parent.clone()
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

// SAFETY: Module interfaces are always `Send + Sync`.
unsafe impl Send for FFIModuleInterface {}
unsafe impl Sync for FFIModuleInterface {}
