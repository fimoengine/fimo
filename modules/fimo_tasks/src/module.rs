//! Implementation of the module.
use crate::TaskRuntime;
use fimo_ffi::object::CoerceObject;
use fimo_ffi::vtable::{IBaseInterface, ObjectID, VTable};
use fimo_ffi::{ArrayString, ObjArc, Object, Optional, StrInner};
use fimo_module_core::{FimoInterface, IModuleInstance, IModuleInterfaceVTable, ModuleInfo};
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

impl ObjectID for TaskInterface {
    const OBJECT_ID: &'static str = "fimo::modules::tasks::task_interface";
}

impl CoerceObject<IModuleInterfaceVTable> for TaskInterface {
    fn get_vtable() -> &'static IModuleInterfaceVTable {
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

        static VTABLE: IModuleInterfaceVTable =
            IModuleInterfaceVTable::new::<TaskInterface>(inner, version, extension, instance);
        &VTABLE
    }
}

impl CoerceObject<IFimoTasksVTable> for TaskInterface {
    fn get_vtable() -> &'static IFimoTasksVTable {
        static VTABLE: IFimoTasksVTable = IFimoTasksVTable::new::<TaskInterface>(|ptr| unsafe {
            &(*(ptr as *const TaskInterface)).runtime as &dyn TaskRuntimeInner as *const _
                as *const fimo_tasks_int::rust::TaskRuntime
        });
        &VTABLE
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(
                String::from(&fimo_tasks_int::rust::IFimoTasks::VERSION).as_bytes(),
            )
        },
    }
}
