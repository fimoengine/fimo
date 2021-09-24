#![allow(unused_imports)]
use fimo_module_core::rust_loader::{RustLoader, MODULE_LOADER_TYPE};
use fimo_module_core::{DynArc, ModuleLoader};
use std::error::Error;
use std::sync::Arc;

use core_interface::rust::{FimoCore, FimoModuleInstanceExt, InterfaceMutex};
use fimo_core_interface as core_interface;

#[cfg(feature = "tasks_module")]
use fimo_tasks_interface as tasks_interface;
#[cfg(feature = "tasks_module")]
use tasks_interface::rust::FimoTasks;

#[cfg(feature = "actix_module")]
use actix_interface::{FimoActix, FimoActixCaster};
#[cfg(feature = "actix_module")]
use fimo_actix_interface as actix_interface;

#[allow(clippy::type_complexity)]
pub fn get_core_interface() -> Result<
    (
        Arc<dyn FimoModuleInstanceExt>,
        Arc<InterfaceMutex<dyn FimoCore>>,
    ),
    Box<dyn Error>,
> {
    let module_loader = RustLoader::new();
    let core_module = unsafe {
        module_loader.load_module_library(module_paths::core_module_path().unwrap().as_path())?
    };
    let core_instance =
        unsafe { core_interface::rust::cast_instance(core_module.create_instance()?)? };
    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();

    let interface = unsafe {
        core_interface::rust::cast_interface(core_instance.get_interface(core_descriptor)?)?
    };
    Ok((core_instance, interface))
}

#[cfg(feature = "tasks_module")]
pub fn get_tasks_interface(
    core_instance: &Arc<dyn FimoModuleInstanceExt>,
    core_interface: &Arc<InterfaceMutex<dyn FimoCore>>,
) -> Result<Arc<dyn FimoTasks>, Box<dyn Error>> {
    let tasks_module = {
        let mut guard = core_interface.lock();
        let module_registry = guard.as_module_registry_mut();
        let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
        unsafe { loader.load_module_library(module_paths::tasks_module_path().unwrap().as_path())? }
    };

    let tasks_instance =
        unsafe { core_interface::rust::cast_instance(tasks_module.create_instance()?)? };
    let tasks_descriptor = tasks_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == tasks_interface::INTERFACE_NAME)
        .unwrap();
    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();

    let i = core_instance.get_interface(core_descriptor)?;
    tasks_instance.set_dependency(core_descriptor, i)?;
    unsafe {
        Ok(tasks_interface::rust::cast_interface(
            tasks_instance.get_interface(tasks_descriptor)?,
        )?)
    }
}

#[cfg(feature = "actix_module")]
pub fn get_actix_interface(
    core_instance: &Arc<dyn FimoModuleInstanceExt>,
    core_interface: &Arc<InterfaceMutex<dyn FimoCore>>,
) -> Result<DynArc<FimoActix, FimoActixCaster>, Box<dyn Error>> {
    let actix_module = {
        let mut guard = core_interface.lock();
        let module_registry = guard.as_module_registry_mut();
        let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
        unsafe { loader.load_module_library(module_paths::actix_module_path().unwrap().as_path())? }
    };

    let actix_instance =
        unsafe { core_interface::rust::cast_instance(actix_module.create_instance()?)? };
    let actix_descriptor = actix_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == actix_interface::INTERFACE_NAME)
        .unwrap();
    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();

    let i = core_instance.get_interface(core_descriptor)?;
    actix_instance.set_dependency(core_descriptor, i)?;
    unsafe {
        Ok(actix_interface::cast_interface(
            actix_instance.get_interface(actix_descriptor)?,
        )?)
    }
}
