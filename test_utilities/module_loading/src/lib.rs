#![allow(unused_imports)]
use fimo_ffi::ObjArc;
use fimo_module_core::rust_loader::{RustLoader, MODULE_LOADER_TYPE};
use fimo_module_core::Error;
use fimo_module_core::{FimoInterface, IModule, IModuleInstance, IModuleInterface};
use std::sync::Arc;

use core_interface::rust::IFimoCore;
use fimo_core_int as core_interface;

#[cfg(feature = "tasks_module")]
use fimo_tasks_int as tasks_interface;
#[cfg(feature = "tasks_module")]
use tasks_interface::rust::IFimoTasks;

#[cfg(feature = "actix_module")]
use actix_interface::IFimoActix;
#[cfg(feature = "actix_module")]
use fimo_actix_int as actix_interface;

#[allow(clippy::type_complexity)]
pub fn get_core_interface() -> Result<(ObjArc<IModuleInstance>, ObjArc<IFimoCore>), Error> {
    let module_loader = RustLoader::new();
    let core_module = unsafe {
        module_loader.load_module_raw(module_paths::core_module_path().unwrap().as_path())?
    };
    let core_instance = core_module.new_instance()?;
    let core_descriptor = core_instance
        .available_interfaces()
        .iter()
        .find(|interface| interface.name == IFimoCore::NAME)
        .unwrap();

    let interface = core_instance.interface(core_descriptor).into_rust()?;
    let interface = IModuleInterface::try_downcast_arc(interface)?;
    Ok((core_instance, interface))
}

#[cfg(feature = "tasks_module")]
pub fn get_tasks_interface(
    core_instance: &ObjArc<IModuleInstance>,
    core_interface: &ObjArc<IFimoCore>,
) -> Result<ObjArc<IFimoTasks>, Error> {
    let module_registry = core_interface.get_module_registry();
    let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
    let tasks_module = unsafe {
        loader
            .load_module_raw(module_paths::tasks_module_path().unwrap().as_path())
            .into_rust()?
    };

    let tasks_instance = tasks_module.new_instance().into_rust()?;
    let tasks_descriptor = tasks_instance
        .available_interfaces()
        .iter()
        .find(|interface| interface.name == IFimoTasks::NAME)
        .unwrap();
    let core_descriptor = core_instance
        .available_interfaces()
        .iter()
        .find(|interface| interface.name == IFimoCore::NAME)
        .unwrap();

    let i = core_instance.interface(core_descriptor).into_rust()?;

    tasks_instance.set_core(core_descriptor, i).into_rust()?;
    let tasks_interface = tasks_instance.interface(tasks_descriptor).into_rust()?;
    IModuleInterface::try_downcast_arc(tasks_interface)
}

#[cfg(feature = "actix_module")]
pub fn get_actix_interface(
    core_instance: &ObjArc<IModuleInstance>,
    core_interface: &ObjArc<IFimoCore>,
) -> Result<ObjArc<IFimoActix>, Error> {
    let module_registry = core_interface.get_module_registry();
    let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
    let actix_module = unsafe {
        loader
            .load_module_raw(module_paths::actix_module_path().unwrap().as_path())
            .into_rust()?
    };

    let actix_instance = actix_module.new_instance().into_rust()?;
    let actix_descriptor = actix_instance
        .available_interfaces()
        .iter()
        .find(|interface| interface.name == IFimoActix::NAME)
        .unwrap();
    let core_descriptor = core_instance
        .available_interfaces()
        .iter()
        .find(|interface| interface.name == IFimoCore::NAME)
        .unwrap();

    let i = core_instance.interface(core_descriptor).into_rust()?;

    actix_instance.set_core(core_descriptor, i).into_rust()?;
    let actix_interface = actix_instance.interface(actix_descriptor).into_rust()?;
    IModuleInterface::try_downcast_arc(actix_interface)
}
