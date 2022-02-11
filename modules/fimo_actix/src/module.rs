//! Implementation of the module.
use crate::FimoActixServer;
use fimo_actix_int::{IFimoActix, IFimoActixVTable};
use fimo_core_int::rust::settings_registry::{
    SettingsEventCallbackHandle, SettingsEventCallbackId,
};
use fimo_core_int::rust::IFimoCore;
use fimo_ffi::marker::SendSyncMarker;
use fimo_ffi::vtable::{IBase, VTable};
use fimo_ffi::{impl_vtable, is_object, ObjArc, Object, Optional, StrInner};
use fimo_module::{FimoInterface, IModuleInstance, IModuleInterfaceVTable, ModuleInfo};
use fimo_version_core::Version;

#[cfg(feature = "rust_module")]
mod rust_module;

#[cfg(feature = "rust_module")]
mod core_bindings;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_actix";

struct FimoActixInterface {
    server: FimoActixServer<String>,
    parent: ObjArc<IModuleInstance>,
    core: Option<(ObjArc<IFimoCore>, SettingsEventCallbackId)>,
}

sa::assert_impl_all!(FimoActixInterface: Send, Sync);

is_object! { #![uuid(0xd7eeb555, 0x6cdc, 0x412e, 0x9d2b, 0xb10f3069c298)] FimoActixInterface }

impl_vtable! {
    impl inline IFimoActixVTable => FimoActixInterface {
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
    }
}

impl_vtable! {
    impl IModuleInterfaceVTable => FimoActixInterface {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBase<SendSyncMarker> {
            let i: &IFimoActixVTable = FimoActixInterface::get_vtable();
            i.as_super()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn version(_ptr: *const ()) -> Version {
            IFimoActix::VERSION
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn extension(
            _ptr: *const (),
            _ext: StrInner<false>,
        ) -> Optional<*const Object<IBase<SendSyncMarker>>> {
            Optional::None
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn instance(ptr: *const ()) -> ObjArc<IModuleInstance> {
            let this = &*(ptr as *const FimoActixInterface);
            this.parent.clone()
        }
    }
}

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
        name: MODULE_NAME.into(),
        version: IFimoActix::NAME.into(),
    }
}
