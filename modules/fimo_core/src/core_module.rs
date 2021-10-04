//! Implementation of the module.
use crate::CoreInterface;
use fimo_core_interface::rust::FimoCoreVTable;
use fimo_ffi_core::ArrayString;
use fimo_module_core::rust::{ModuleInstanceArc, ModuleInterfaceVTable};
use fimo_module_core::ModuleInfo;

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_core";

const VTABLE: FimoCoreVTable = FimoCoreVTable::new(
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

const INTERFACE_VTABLE: ModuleInterfaceVTable = ModuleInterfaceVTable::new(
    |_ptr| {
        fimo_core_interface::fimo_core_interface_impl! {to_ptr, VTABLE}
    },
    |_ptr| {
        fimo_core_interface::fimo_core_interface_impl! {id}
    },
    |_ptr| {
        fimo_core_interface::fimo_core_interface_impl! {version}
    },
    |ptr| {
        let core = unsafe { &*(ptr as *const CoreWrapper) };
        core.parent.clone()
    },
);

struct CoreWrapper {
    interface: CoreInterface,
    parent: ModuleInstanceArc,
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
