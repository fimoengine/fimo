use crate::module::{construct_module_info, get_tasks_interface_descriptor, TaskInterface};
use crate::TaskRuntime;
use fimo_core_interface::rust::SettingsItem;
use fimo_ffi_core::ArrayString;
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust_loader::{RustModule, RustModuleExt};
use fimo_module_core::{ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor};
use fimo_version_core::{ReleaseType, Version};
use std::collections::HashMap;
use std::error::Error;
use std::io::ErrorKind;
use std::sync::{Arc, Weak};

fimo_module_core::export_rust_module! {fimo_ffi_core::TypeWrapper(construct_module)}

#[allow(dead_code, improper_ctypes_definitions)]
extern "C-unwind" fn construct_module() -> Result<Box<dyn RustModuleExt>, Box<dyn Error>> {
    Ok(GenericModule::new(construct_module_info(), build_instance))
}

fn build_instance(parent: Arc<RustModule>) -> Result<Arc<GenericModuleInstance>, Box<dyn Error>> {
    let core_desc = get_tasks_interface_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(
        core_desc,
        (
            build_tasks_interface as _,
            vec![core_interface_descriptor()],
        ),
    );

    let mut pkg_versions = HashMap::new();
    pkg_versions.insert(
        String::from(fimo_tasks_interface::rust::PKG_NAME),
        String::from(fimo_tasks_interface::rust::PKG_VERSION),
    );

    Ok(GenericModuleInstance::new(parent, pkg_versions, interfaces))
}

fn build_tasks_interface(
    instance: Arc<dyn ModuleInstance>,
    dep_map: &HashMap<ModuleInterfaceDescriptor, Option<Weak<dyn ModuleInterface>>>,
) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>> {
    let core_interface = dep_map
        .get(&core_interface_descriptor())
        .map(|i| Weak::upgrade(i.as_ref().unwrap()));

    if core_interface.is_none() || core_interface.as_ref().unwrap().is_none() {
        return Err(Box::new(std::io::Error::new(
            ErrorKind::NotFound,
            "fimo-core interface not found",
        )));
    }

    let core_interface =
        unsafe { fimo_core_interface::rust::cast_interface(core_interface.unwrap().unwrap())? };

    let guard = core_interface.try_lock();
    match guard {
        Ok(mut guard) => {
            // read settings from registry.
            let settings_registry = guard.as_settings_registry_mut();
            let num_cores_val = settings_registry.read("fimo_tasks::num_cores");
            let max_tasks_val = settings_registry.read("fimo_tasks::max_tasks");
            let allocated_tasks_val = settings_registry.read("fimo_tasks::allocated_tasks");

            let num_cores = match num_cores_val {
                None => {
                    let cores = num_cpus::get();
                    settings_registry
                        .write("fimo_tasks::num_cores", SettingsItem::U64(cores as u64));
                    cores
                }
                Some(val) => match val {
                    SettingsItem::U64(n) => n as usize,
                    _ => {
                        return Err(Box::new(std::io::Error::new(
                            ErrorKind::Other,
                            "fimo_tasks::num_cores: expected u64 value",
                        )));
                    }
                },
            };

            let max_tasks = match max_tasks_val {
                None => {
                    settings_registry.write("fimo_tasks::max_tasks", SettingsItem::U64(1024));
                    1024
                }
                Some(val) => match val {
                    SettingsItem::U64(n) => n as usize,
                    _ => {
                        return Err(Box::new(std::io::Error::new(
                            ErrorKind::Other,
                            "fimo_tasks::max_tasks: expected u64 value",
                        )));
                    }
                },
            };

            let allocated_tasks = match allocated_tasks_val {
                None => {
                    settings_registry.write("fimo_tasks::allocated_tasks", SettingsItem::U64(128));
                    128
                }
                Some(val) => match val {
                    SettingsItem::U64(n) => n as usize,
                    _ => {
                        return Err(Box::new(std::io::Error::new(
                            ErrorKind::Other,
                            "fimo_tasks::allocated_tasks: expected u64 value",
                        )));
                    }
                },
            };

            Ok(Arc::new(TaskInterface {
                runtime: TaskRuntime::new(num_cores, max_tasks, allocated_tasks),
                parent: instance,
            }))
        }
        Err(_) => Err(Box::new(std::io::Error::new(
            ErrorKind::Other,
            "can not lock the fimo-core interface",
        ))),
    }
}

fn core_interface_descriptor() -> ModuleInterfaceDescriptor {
    ModuleInterfaceDescriptor {
        name: unsafe { ArrayString::from_utf8_unchecked("fimo-core".as_bytes()) },
        version: Version::new_long(0, 1, 0, ReleaseType::Unstable, 0),
        extensions: Default::default(),
    }
}
