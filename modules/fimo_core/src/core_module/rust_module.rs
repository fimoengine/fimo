use crate::FimoCore;
use fimo_module_core::rust_loader::{RustModule, RustModuleExt};
use fimo_module_core::{Module, ModuleInfo, ModuleInstance, ModuleLoader, ModulePtr};
use std::any::Any;
use std::error::Error;
use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::{Arc, Weak};

fimo_module_core::export_rust_module! {fimo_ffi_core::TypeWrapper(construct_module)}

struct FimoCoreRust {
    module_info: ModuleInfo,
    parent: MaybeUninit<Weak<RustModule>>,
}

impl FimoCoreRust {
    fn new() -> Box<Self> {
        Box::new(Self {
            parent: MaybeUninit::uninit(),
            module_info: super::construct_module_info(),
        })
    }
}

impl Drop for FimoCoreRust {
    fn drop(&mut self) {
        unsafe { self.parent.assume_init_drop() }
    }
}

impl Module for FimoCoreRust {
    fn get_raw_ptr(&self) -> ModulePtr {
        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            self.parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_raw_ptr()
        }
    }

    fn get_module_path(&self) -> &Path {
        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            &*(self
                .parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_module_path() as *const _)
        }
    }

    fn get_module_info(&self) -> &ModuleInfo {
        &self.module_info
    }

    fn get_module_loader(&self) -> &'static (dyn ModuleLoader + 'static) {
        // SAFETY: The value is initialized and lives as long as the instance.
        unsafe {
            self.parent
                .assume_init_ref()
                .upgrade()
                .unwrap()
                .get_module_loader()
        }
    }

    fn create_instance(&self) -> Result<Arc<dyn ModuleInstance>, Box<dyn Error>> {
        let parent = unsafe { self.parent.assume_init_ref().upgrade().unwrap() };
        Ok(FimoCore::new(parent))
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
    }
}

impl RustModuleExt for FimoCoreRust {
    fn set_weak_parent_handle(&mut self, module: Weak<RustModule>) {
        self.parent = MaybeUninit::new(module);
    }

    fn as_module(&self) -> &(dyn Module + 'static) {
        self
    }

    fn as_module_mut(&mut self) -> &mut (dyn Module + 'static) {
        self
    }
}

#[allow(dead_code, improper_ctypes_definitions)]
extern "C-unwind" fn construct_module() -> Result<Box<dyn RustModuleExt>, Box<dyn Error>> {
    Ok(FimoCoreRust::new())
}
