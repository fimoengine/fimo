//! Implementation of the module.
use crate::CoreInterface;
use fimo_core_int::rust::module_registry::IModuleRegistry;
use fimo_core_int::rust::settings_registry::SettingsRegistry;
use fimo_core_int::rust::{IFimoCore, IFimoCoreVTable};
use fimo_ffi::object::ObjectWrapper;
use fimo_ffi::vtable::{IBaseInterface, VTable};
use fimo_ffi::{ObjArc, Object, Optional, StrInner};
use fimo_module::{
    impl_vtable, is_object, FimoInterface, IModuleInstance, IModuleInterfaceVTable, ModuleInfo,
};
use fimo_version_core::Version;

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_core";

struct CoreWrapper {
    interface: CoreInterface,
    parent: ObjArc<IModuleInstance>,
}

sa::assert_impl_all!(CoreWrapper: Send, Sync);

is_object! { #![uuid(0x8e68e497, 0x4dd1, 0x481c, 0xafe2, 0xdb7c063ae9f4)] CoreWrapper }

impl_vtable! {
    impl inline IFimoCoreVTable => CoreWrapper {
        |ptr| {
            let this = unsafe { &*(ptr as *const CoreWrapper) };
            IModuleRegistry::from_object(this.interface.as_module_registry().coerce_obj())
        },
        |ptr| {
            let this = unsafe { &*(ptr as *const CoreWrapper) };
            SettingsRegistry::from_object(this.interface.as_settings_registry().coerce_obj())
        },
    }
}

impl_vtable! {
    impl IModuleInterfaceVTable => CoreWrapper {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBaseInterface {
            let i: &IFimoCoreVTable = CoreWrapper::get_vtable();
            i.as_base()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn version(_ptr: *const ()) -> Version {
            IFimoCore::VERSION
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
            let this = &*(ptr as *const CoreWrapper);
            this.parent.clone()
        }
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.into(),
        version: IFimoCore::VERSION.into(),
    }
}
