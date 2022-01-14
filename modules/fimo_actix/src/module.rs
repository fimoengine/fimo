//! Implementation of the module.
use crate::FimoActixServer;
use fimo_actix_interface::FimoActixVTable;
use fimo_core_interface::rust::settings_registry::{
    SettingsEventCallbackHandle, SettingsEventCallbackId,
};
use fimo_core_interface::rust::FimoCore;
use fimo_ffi::object::CoerceObject;
use fimo_ffi::vtable::{IBaseInterface, ObjectID, VTable};
use fimo_ffi::{ObjArc, Object, StrInner};
use fimo_ffi_core::{ArrayString, Optional};
use fimo_module_core::{FimoInterface, IModuleInstance, IModuleInterfaceVTable, ModuleInfo};
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
    core: Option<(ObjArc<FimoCore>, SettingsEventCallbackId)>,
}

sa::assert_impl_all!(FimoActixInterface: Send, Sync);

impl ObjectID for FimoActixInterface {
    const OBJECT_ID: &'static str = "fimo::modules::actix::actix";
}

impl CoerceObject<FimoActixVTable> for FimoActixInterface {
    fn get_vtable() -> &'static FimoActixVTable {
        static VTABLE: FimoActixVTable = FimoActixVTable::new::<FimoActixInterface>(
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
        &VTABLE
    }
}

impl CoerceObject<IModuleInterfaceVTable> for FimoActixInterface {
    fn get_vtable() -> &'static IModuleInterfaceVTable {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBaseInterface {
            let i: &FimoActixVTable = FimoActixInterface::get_vtable();
            i.as_base()
        }
        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn version(_ptr: *const ()) -> Version {
            fimo_actix_interface::FimoActix::VERSION
        }
        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn extension(
            _ptr: *const (),
            _ext: StrInner<false>,
        ) -> Optional<*const Object<IBaseInterface>> {
            Optional::None
        }
        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn instance(ptr: *const ()) -> ObjArc<IModuleInstance> {
            let this = &*(ptr as *const FimoActixInterface);
            this.parent.clone()
        }

        static VTABLE: IModuleInterfaceVTable =
            IModuleInterfaceVTable::new::<FimoActixInterface>(inner, version, extension, instance);
        &VTABLE
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
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(
                String::from(fimo_actix_interface::FimoActix::NAME).as_bytes(),
            )
        },
    }
}
