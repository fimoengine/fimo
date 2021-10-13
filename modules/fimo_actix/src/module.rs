//! Implementation of the module.
use crate::FimoActixServer;
use fimo_actix_interface::FimoActixVTable;
use fimo_core_interface::rust::settings_registry::{
    SettingsEventCallbackHandle, SettingsEventCallbackId,
};
use fimo_core_interface::rust::{FimoCore, FimoCoreCaster};
use fimo_ffi_core::ArrayString;
use fimo_module_core::rust::ModuleInterfaceVTable;
use fimo_module_core::{rust::ModuleInstanceArc, DynArc, ModuleInfo};

#[cfg(feature = "rust_module")]
mod rust_module;

#[cfg(feature = "rust_module")]
mod core_bindings;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_actix";

const VTABLE: FimoActixVTable = FimoActixVTable::new(
    |ptr| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.start()
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.stop()
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.pause()
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.resume()
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.restart()
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.get_server_status()
    },
    |ptr, path, builder| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        let path = unsafe { &*path };
        interface.server.register_scope(path, builder)
    },
    |ptr, id| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.unregister_scope(id)
    },
    |ptr, callback| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.register_callback(callback)
    },
    |ptr, id| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.server.unregister_callback(id)
    },
);

const INTERFACE_VTABLE: ModuleInterfaceVTable = ModuleInterfaceVTable::new(
    |_ptr| {
        fimo_actix_interface::fimo_actix_interface_impl! {to_ptr, VTABLE}
    },
    |_ptr| {
        fimo_actix_interface::fimo_actix_interface_impl! {id}
    },
    |_ptr| {
        fimo_actix_interface::fimo_actix_interface_impl! {version}
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const FimoActixInterface) };
        interface.parent.clone()
    },
);

struct FimoActixInterface {
    server: FimoActixServer<String>,
    parent: ModuleInstanceArc,
    core: Option<(DynArc<FimoCore, FimoCoreCaster>, SettingsEventCallbackId)>,
}

sa::assert_impl_all!(FimoActixInterface: Send, Sync);

impl Drop for FimoActixInterface {
    fn drop(&mut self) {
        if let Some((core, id)) = self.core.take() {
            let registry = core.get_settings_registry();
            let handle = unsafe { SettingsEventCallbackHandle::from_raw_parts(id, registry) };
            registry.unregister_callback(handle);
        }
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(
                String::from(&fimo_actix_interface::INTERFACE_VERSION).as_bytes(),
            )
        },
    }
}
