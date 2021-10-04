use crate::module::{construct_module_info, TaskInterface, INTERFACE_VTABLE};
use crate::TaskRuntime;
use fimo_core_interface::rust::{
    build_interface_descriptor as core_descriptor,
    settings_registry::{SettingsItem, SettingsItemType, SettingsRegistryPath},
};
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust::module_loader::{RustModule, RustModuleInnerArc};
use fimo_module_core::rust::ModuleInterfaceCaster;
use fimo_module_core::{
    rust::{ModuleInterfaceArc, ModuleInterfaceWeak},
    ModuleInterfaceDescriptor,
};
use fimo_tasks_interface::rust::build_interface_descriptor as tasks_descriptor;
use std::collections::HashMap;
use std::error::Error;
use std::io::ErrorKind;
use std::sync::Arc;

fimo_module_core::export_rust_module! {construct_module}

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn construct_module() -> Result<RustModuleInnerArc, Box<dyn Error>> {
    Ok(GenericModule::new_inner(
        construct_module_info(),
        build_instance,
    ))
}

fn build_instance(parent: Arc<RustModule>) -> Result<Arc<GenericModuleInstance>, Box<dyn Error>> {
    let core_desc = tasks_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(
        core_desc,
        (build_tasks_interface as _, vec![core_descriptor()]),
    );

    Ok(GenericModuleInstance::new(parent, interfaces))
}

fn build_tasks_interface(
    instance: Arc<GenericModuleInstance>,
    dep_map: &HashMap<ModuleInterfaceDescriptor, Option<ModuleInterfaceWeak>>,
) -> Result<ModuleInterfaceArc, Box<dyn Error>> {
    let core_interface = dep_map
        .get(&core_descriptor())
        .map(|i| i.as_ref().unwrap().upgrade());

    if core_interface.is_none() || core_interface.as_ref().unwrap().is_none() {
        return Err(Box::new(std::io::Error::new(
            ErrorKind::NotFound,
            "fimo-core interface not found",
        )));
    }

    let core_interface =
        unsafe { fimo_core_interface::rust::cast_interface(core_interface.unwrap().unwrap())? };

    #[allow(non_snake_case)]
    let NUM_CORES = num_cpus::get();
    const MAX_TASKS: usize = 1024;
    const ALLOCATED_TASKS: usize = 128;

    let settings_path = SettingsRegistryPath::new("fimo-tasks").unwrap();
    let num_cores_path = settings_path.join(SettingsRegistryPath::new("num_cores").unwrap());
    let max_tasks_path = settings_path.join(SettingsRegistryPath::new("max_tasks").unwrap());
    let allocated_tasks_path =
        settings_path.join(SettingsRegistryPath::new("allocated_tasks").unwrap());

    let registry = core_interface.get_settings_registry();
    if !registry
        .item_type(settings_path)
        .unwrap_or(SettingsItemType::Null)
        .is_object()
    {
        registry
            .write(settings_path, SettingsItem::Object(Default::default()))
            .unwrap();
    }

    let num_cores = registry
        .try_read_or(num_cores_path, NUM_CORES)
        .unwrap()
        .unwrap_or(NUM_CORES);
    let max_tasks = registry
        .try_read_or(max_tasks_path, MAX_TASKS)
        .unwrap()
        .unwrap_or(MAX_TASKS);
    let allocated_tasks = registry
        .try_read_or(allocated_tasks_path, ALLOCATED_TASKS)
        .unwrap()
        .unwrap_or(ALLOCATED_TASKS);

    let base = Arc::new(TaskInterface {
        runtime: TaskRuntime::new(num_cores, max_tasks, allocated_tasks),
        parent: GenericModuleInstance::as_module_instance_arc(instance),
    });

    let caster = ModuleInterfaceCaster::new(&INTERFACE_VTABLE);
    unsafe { Ok(ModuleInterfaceArc::from_inner((base, caster))) }
}
