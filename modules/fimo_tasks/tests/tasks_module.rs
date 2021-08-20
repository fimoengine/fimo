use fimo_core_interface as ci;
use fimo_core_interface::rust::{FimoCore, FimoModuleInstanceExt, InterfaceMutex};
use fimo_module_core::rust_loader::{RustLoader, MODULE_LOADER_TYPE};
use fimo_module_core::ModuleLoader;
use fimo_tasks_interface::rust::FimoTasks;
use fimo_tasks_interface::INTERFACE_NAME;
use std::alloc::System;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

#[global_allocator]
static A: System = System;

#[cfg(test)]
mod runtime;
#[cfg(test)]
mod sync;
#[cfg(test)]
mod tasks;

fn core_module_path() -> Result<PathBuf, Box<dyn Error>> {
    let artifact_dir = PathBuf::from(std::env::current_exe()?.parent().unwrap().parent().unwrap());

    let core_path = if cfg!(target_os = "windows") {
        artifact_dir.join("fimo_core.dll").canonicalize()?
    } else if cfg!(target_os = "linux") {
        artifact_dir.join("libfimo_core.so").canonicalize()?
    } else if cfg!(target_os = "macos") {
        artifact_dir.join("libfimo_core.dylib").canonicalize()?
    } else {
        unimplemented!()
    };

    Ok(core_path)
}

fn tasks_module_path() -> Result<PathBuf, Box<dyn Error>> {
    let artifact_dir = PathBuf::from(std::env::current_exe()?.parent().unwrap().parent().unwrap());

    let tasks_path = if cfg!(target_os = "windows") {
        artifact_dir.join("fimo_tasks.dll").canonicalize()?
    } else if cfg!(target_os = "linux") {
        artifact_dir.join("libfimo_tasks.so").canonicalize()?
    } else if cfg!(target_os = "macos") {
        artifact_dir.join("libfimo_tasks.dylib").canonicalize()?
    } else {
        unimplemented!()
    };

    Ok(tasks_path)
}

#[allow(clippy::type_complexity)]
fn get_core_interface() -> Result<
    (
        Arc<dyn FimoModuleInstanceExt>,
        Arc<InterfaceMutex<dyn FimoCore>>,
    ),
    Box<dyn Error>,
> {
    let module_loader = RustLoader::new();
    let core_module =
        unsafe { module_loader.load_module_library(core_module_path().unwrap().as_path())? };
    let core_instance = unsafe { ci::rust::cast_instance(core_module.create_instance()?)? };
    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();

    let interface =
        unsafe { ci::rust::cast_interface(core_instance.get_interface(core_descriptor)?)? };
    Ok((core_instance, interface))
}

fn get_tasks_interface(
    core_instance: &Arc<dyn FimoModuleInstanceExt>,
    core_interface: &Arc<InterfaceMutex<dyn FimoCore>>,
) -> Result<Arc<dyn FimoTasks>, Box<dyn Error>> {
    let tasks_module = {
        let mut guard = core_interface.lock();
        let module_registry = guard.as_module_registry_mut();
        let loader = module_registry.get_loader_from_type(MODULE_LOADER_TYPE)?;
        unsafe { loader.load_module_library(tasks_module_path().unwrap().as_path())? }
    };

    let tasks_instance = unsafe { ci::rust::cast_instance(tasks_module.create_instance()?)? };
    let tasks_descriptor = tasks_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == INTERFACE_NAME)
        .unwrap();
    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();

    let i = core_instance.get_interface(core_descriptor)?;
    tasks_instance.set_dependency(core_descriptor, i)?;
    unsafe {
        Ok(fimo_tasks_interface::rust::cast_interface(
            tasks_instance.get_interface(tasks_descriptor)?,
        )?)
    }
}

fn initialize() -> Result<Arc<dyn FimoTasks>, Box<dyn Error>> {
    let (core_instance, core_interface) = get_core_interface()?;
    get_tasks_interface(&core_instance, &core_interface)
}
