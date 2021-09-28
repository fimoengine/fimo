//! Implementation of the module.
use crate::CoreInterface;
use fimo_core_interface::rust::{FimoCore, FimoCoreCaster, FimoCoreInner, FimoCoreVTable};
use fimo_ffi_core::ArrayString;
use fimo_module_core::{DynArcBase, ModuleInfo, ModuleInstance, ModuleInterface, ModulePtr};
use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_core";

const VTABLE: FimoCoreVTable = FimoCoreVTable::new(
    |ptr| {
        let wrapper = unsafe { &*(ptr as *const CoreWrapper) };
        wrapper.interface.get_interface_version()
    },
    |ptr, extension| {
        let wrapper = unsafe { &*(ptr as *const CoreWrapper) };
        let extension = unsafe { &*extension };
        wrapper
            .interface
            .find_extension(extension)
            .map(|ext| ext as *const _)
    },
    |ptr| {
        let wrapper = unsafe { &*(ptr as *const CoreWrapper) };
        &**wrapper.interface.as_module_registry()
    },
    |ptr| {
        let wrapper = unsafe { &*(ptr as *const CoreWrapper) };
        &**wrapper.interface.as_settings_registry()
    },
);

struct CoreWrapper {
    interface: CoreInterface,
    parent: Arc<dyn ModuleInstance>,
}

impl Deref for CoreWrapper {
    type Target = FimoCore;

    fn deref(&self) -> &Self::Target {
        let self_ptr = self as *const _ as *const ();
        let vtable = &VTABLE;

        unsafe { &*FimoCore::from_raw_parts(self_ptr, vtable) }
    }
}

impl ModuleInterface for CoreWrapper {
    fimo_core_interface::fimo_core_interface_impl! {}

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

impl FimoCoreInner for CoreWrapper {
    fn as_base(&self) -> &dyn DynArcBase {
        self
    }

    fn get_caster(&self) -> FimoCoreCaster {
        let core = &**self;
        let (_, vtable) = core.into_raw_parts();
        FimoCoreCaster::new(vtable)
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(String::from(&crate::INTERFACE_VERSION).as_bytes())
        },
    }
}
