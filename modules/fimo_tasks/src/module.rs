//! Implementation of the module.
use crate::TaskRuntime;
use fimo_ffi_core::ArrayString;
use fimo_module_core::{
    ModuleInfo, ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor, ModulePtr,
};
use fimo_tasks_interface::rust::{FimoTasks, TaskRuntimeInner};
use fimo_version_core::Version;
use std::any::Any;
use std::sync::Arc;

#[cfg(feature = "rust_module")]
mod rust_module;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_tasks";

struct TaskInterface {
    runtime: TaskRuntime,
    parent: Arc<dyn ModuleInstance>,
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

impl ModuleInterface for TaskInterface {
    fimo_tasks_interface::fimo_tasks_interface_impl! {}

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
