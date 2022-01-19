//! Implementation of the module.
use crate::TaskRuntime;
use fimo_ffi::vtable::{IBaseInterface, VTable};
use fimo_ffi::{ObjArc, Object, Optional, StrInner};
use fimo_module::{
    impl_vtable, is_object, FimoInterface, IModuleInstance, IModuleInterfaceVTable, ModuleInfo,
};
use fimo_tasks_int::rust::{IFimoTasksVTable, TaskRuntimeInner};
use fimo_version_core::Version;

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_tasks";

struct TaskInterface {
    runtime: TaskRuntime,
    parent: ObjArc<IModuleInstance>,
}

is_object! { #![uuid(0xebf605a4, 0xa3e1, 0x47d7, 0x9533, 0xef1105a99992)] TaskInterface }

impl_vtable! {
    impl IModuleInterfaceVTable => TaskInterface {
        unsafe extern "C" fn inner(_ptr: *const ()) -> &'static IBaseInterface {
            let i: &IFimoTasksVTable = TaskInterface::get_vtable();
            i.as_base()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn version(_ptr: *const ()) -> Version {
            fimo_tasks_int::rust::IFimoTasks::VERSION
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
            let this = &*(ptr as *const TaskInterface);
            this.parent.clone()
        }
    }
}

impl_vtable! {
    impl inline IFimoTasksVTable => TaskInterface {
        |ptr| unsafe {
            &(*(ptr as *const TaskInterface)).runtime as &dyn TaskRuntimeInner as *const _
                as *const fimo_tasks_int::rust::TaskRuntime
        }
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.into(),
        version: fimo_tasks_int::rust::IFimoTasks::VERSION.into(),
    }
}
