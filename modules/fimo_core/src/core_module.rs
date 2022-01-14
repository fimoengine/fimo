//! Implementation of the module.
use crate::CoreInterface;
use fimo_core_interface::rust::module_registry::IModuleRegistry;
use fimo_core_interface::rust::settings_registry::SettingsRegistry;
use fimo_core_interface::rust::FimoCoreVTable;
use fimo_ffi::object::{CoerceObject, ObjectWrapper};
use fimo_ffi::vtable::{IBaseInterface, ObjectID, VTable};
use fimo_ffi::{ArrayString, ObjArc, Object, Optional, StrInner};
use fimo_module_core::{FimoInterface, IModuleInstance, IModuleInterfaceVTable, ModuleInfo};
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

impl ObjectID for CoreWrapper {
    const OBJECT_ID: &'static str = "fimo::modules::core::core_wrapper";
}

impl CoerceObject<FimoCoreVTable> for CoreWrapper {
    fn get_vtable() -> &'static FimoCoreVTable {
        static VTABLE: FimoCoreVTable = FimoCoreVTable::new::<CoreWrapper>(
            |ptr| {
                let this = unsafe { &*(ptr as *const CoreWrapper) };
                IModuleRegistry::from_object(this.interface.as_module_registry().coerce_obj())
            },
            |ptr| {
                let this = unsafe { &*(ptr as *const CoreWrapper) };
                SettingsRegistry::from_object(this.interface.as_settings_registry().coerce_obj())
            },
        );
        &VTABLE
    }
}

impl CoerceObject<IModuleInterfaceVTable> for CoreWrapper {
    fn get_vtable() -> &'static IModuleInterfaceVTable {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBaseInterface {
            let i: &FimoCoreVTable = CoreWrapper::get_vtable();
            i.as_base()
        }
        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn version(_ptr: *const ()) -> Version {
            fimo_core_interface::rust::FimoCore::VERSION
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

        static VTABLE: IModuleInterfaceVTable =
            IModuleInterfaceVTable::new::<CoreWrapper>(inner, version, extension, instance);
        &VTABLE
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(
                String::from(fimo_core_interface::rust::FimoCore::VERSION).as_bytes(),
            )
        },
    }
}
