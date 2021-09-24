//! Implementation of the module.
use crate::FimoActixServer;
use fimo_actix_interface::{FimoActixCaster, FimoActixInner, FimoActixVTable};
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    DynArcBase, ModuleInfo, ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor,
};
use std::any::Any;
use std::sync::Arc;

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

struct FimoActixInterface {
    server: FimoActixServer<String>,
    parent: Arc<dyn ModuleInstance>,
}

impl FimoActixInner for FimoActixInterface {
    fn as_base(&self) -> &dyn DynArcBase {
        self
    }

    fn get_caster(&self) -> FimoActixCaster {
        FimoActixCaster::new(&VTABLE)
    }
}

impl ModuleInterface for FimoActixInterface {
    fimo_actix_interface::fimo_actix_interface_impl! {}

    fn get_instance(&self) -> Arc<dyn ModuleInstance> {
        Arc::clone(&self.parent)
    }

    fn as_any(&self) -> &(dyn Any + Send + Sync + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + Send + Sync + 'static) {
        self
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

#[allow(dead_code)]
fn get_actix_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe {
            ArrayString::from_utf8_unchecked(fimo_actix_interface::INTERFACE_NAME.as_bytes())
        },
        version: fimo_actix_interface::INTERFACE_VERSION,
        extensions: Default::default(),
    }
}
