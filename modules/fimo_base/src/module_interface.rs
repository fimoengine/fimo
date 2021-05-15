//! Module interface
use crate::FimoBase;
use emf_core_base_rs::ffi::collections::{ConstSpan, NonNullConst, Result};
use emf_core_base_rs::ffi::module::native_module::{NativeModule, NativeModuleInterface};
use emf_core_base_rs::ffi::module::{
    Error, Interface, InterfaceDescriptor, ModuleHandle, ModuleInfo, ModuleStatus,
};
use emf_core_base_rs::ffi::sys::api::{GetFunctionFn, HasFunctionFn};
use emf_core_base_rs::ffi::{Bool, CBase, TypeWrapper};
use std::ptr::NonNull;

/// Interface of the module.
#[no_mangle]
#[allow(non_upper_case_globals)]
pub static emf_cbase_native_module_interface: NativeModuleInterface = NativeModuleInterface {
    load_fn: TypeWrapper(load),
    unload_fn: TypeWrapper(unload),
    initialize_fn: TypeWrapper(initialize),
    terminate_fn: TypeWrapper(terminate),
    get_interface_fn: TypeWrapper(get_interface),
    get_module_info_fn: TypeWrapper(get_module_info),
    get_load_dependencies_fn: TypeWrapper(get_load_dependencies),
    get_runtime_dependencies_fn: TypeWrapper(get_runtime_dependencies),
    get_exportable_interfaces_fn: TypeWrapper(get_exportable_interfaces),
};

/// Loads the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn load(
    handle: ModuleHandle,
    base_module: Option<NonNull<CBase>>,
    has_fn: HasFunctionFn,
    _get_fn: GetFunctionFn,
) -> Result<Option<NonNull<NativeModule>>, Error> {
    unsafe {
        // Safety: `has_fn` expects the magic value `0`.
        if handle.id != 0 || has_fn(base_module, std::mem::transmute(0i32)) == Bool::False {
            return Result::new_err(Error::ModuleHandleInvalid);
        }
    }

    let base = NonNull::from(Box::leak(Box::new(FimoBase::new())));
    Result::new_ok(Some(base.cast()))
}

/// Unloads the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn unload(module: Option<NonNull<NativeModule>>) -> Result<i8, Error> {
    match module {
        None => Result::new_err(Error::ModuleHandleInvalid),
        Some(module) => {
            let base = unsafe { Box::<FimoBase>::from_raw(module.cast().as_ptr()) };
            match base.module_status() {
                ModuleStatus::Terminated => Result::new_ok(0),
                _ => {
                    Box::leak(base);
                    Result::new_err(Error::ModuleStateInvalid)
                }
            }
        }
    }
}

/// Initializes the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn initialize(module: Option<NonNull<NativeModule>>) -> Result<i8, Error> {
    match FimoBase::from_module_mut(module) {
        None => Result::new_err(Error::ModuleHandleInvalid),
        Some(base) => base.initialize(),
    }
}

/// Terminates the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn terminate(module: Option<NonNull<NativeModule>>) -> Result<i8, Error> {
    match FimoBase::from_module_mut(module) {
        None => Result::new_err(Error::ModuleHandleInvalid),
        Some(base) => base.terminate(),
    }
}

/// Fetches an interface from the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn get_interface(
    module: Option<NonNull<NativeModule>>,
    interface: NonNullConst<InterfaceDescriptor>,
) -> Result<Interface, Error> {
    match FimoBase::from_module(module) {
        None => Result::new_err(Error::ModuleHandleInvalid),
        Some(base) => base.get_interface(unsafe { interface.as_ref() }),
    }
}

/// Fetches the module info of the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn get_module_info(
    module: Option<NonNull<NativeModule>>,
) -> Result<NonNullConst<ModuleInfo>, Error> {
    match FimoBase::from_module(module) {
        None => Result::new_err(Error::ModuleHandleInvalid),
        Some(base) => Result::new_ok(NonNullConst::from(base.get_module_info())),
    }
}

/// Fetches the load dependencies of the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn get_load_dependencies() -> ConstSpan<InterfaceDescriptor> {
    FimoBase::get_load_dependencies()
}

/// Fetches the module info of the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn get_runtime_dependencies(
    module: Option<NonNull<NativeModule>>,
) -> Result<ConstSpan<InterfaceDescriptor>, Error> {
    match FimoBase::from_module(module) {
        None => Result::new_err(Error::ModuleHandleInvalid),
        Some(base) => Result::new_ok(base.get_runtime_dependencies()),
    }
}

/// Fetches the exportable interfaces of the module.
#[allow(improper_ctypes_definitions)]
pub extern "C-unwind" fn get_exportable_interfaces(
    module: Option<NonNull<NativeModule>>,
) -> Result<ConstSpan<InterfaceDescriptor>, Error> {
    match FimoBase::from_module(module) {
        None => Result::new_err(Error::ModuleHandleInvalid),
        Some(base) => Result::new_ok(base.get_exportable_interfaces()),
    }
}
