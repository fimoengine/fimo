//! Implementation of the module.
use crate::TaskRuntime;
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    rust::{ModuleInstanceArc, ModuleInterfaceVTable},
    ModuleInfo, ModuleInterfaceDescriptor,
};
use fimo_tasks_interface::rust::{FimoTasks, TaskRuntimeInner};
use fimo_version_core::Version;
use std::any::Any;

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_tasks";

const INTERFACE_VTABLE: ModuleInterfaceVTable = ModuleInterfaceVTable::new(
    |ptr| {
        let interface = unsafe { &*(ptr as *const TaskInterface) };
        fimo_tasks_interface::fimo_tasks_interface_impl! {to_ptr, interface}
    },
    |_ptr| {
        fimo_tasks_interface::fimo_tasks_interface_impl! {id}
    },
    |ptr| {
        let interface = unsafe { &*(ptr as *const TaskInterface) };
        interface.parent.clone()
    },
);

struct TaskInterface {
    runtime: TaskRuntime,
    parent: ModuleInstanceArc,
}

impl FimoTasks for TaskInterface {
    fn get_interface_version(&self) -> Version {
        fimo_tasks_interface::INTERFACE_VERSION
    }

    fn find_extension(&self, _extension: &str) -> Option<&(dyn Any + 'static)> {
        None
    }

    fn find_extension_mut(&mut self, _extension: &str) -> Option<&mut (dyn Any + 'static)> {
        None
    }

    fn as_task_runtime(&self) -> &fimo_tasks_interface::rust::TaskRuntime {
        let inner = &self.runtime as &dyn TaskRuntimeInner;
        unsafe { &*(inner as *const _ as *const fimo_tasks_interface::rust::TaskRuntime) }
    }

    fn as_any(&self) -> &(dyn Any + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn Any + 'static) {
        self
    }
}

#[allow(dead_code)]
fn construct_module_info() -> ModuleInfo {
    ModuleInfo {
        name: unsafe { ArrayString::from_utf8_unchecked(MODULE_NAME.as_bytes()) },
        version: unsafe {
            ArrayString::from_utf8_unchecked(
                String::from(&fimo_tasks_interface::INTERFACE_VERSION).as_bytes(),
            )
        },
    }
}

#[allow(dead_code)]
fn get_tasks_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe {
            ArrayString::from_utf8_unchecked(fimo_tasks_interface::INTERFACE_NAME.as_bytes())
        },
        version: fimo_tasks_interface::INTERFACE_VERSION,
        extensions: Default::default(),
    }
}
