use crate::module::{construct_module_info, TaskInterface};
use crate::TaskRuntime;
use fimo_core_interface::rust::{
    build_interface_descriptor as core_descriptor,
    settings_registry::{SettingsItem, SettingsItemType, SettingsRegistryPath},
    FimoCore,
};
use fimo_ffi::{ObjArc, ObjWeak};
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust_loader::{IRustModuleInner, IRustModuleParent};
use fimo_module_core::{Error, ErrorKind, IModuleInterface, ModuleInterfaceDescriptor};
use fimo_tasks_interface::rust::build_interface_descriptor as tasks_descriptor;
use std::collections::HashMap;

fimo_module_core::rust_module! {construct_module}

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn construct_module() -> Result<ObjArc<IRustModuleInner>, Error> {
    Ok(GenericModule::new_inner(
        construct_module_info(),
        build_instance,
    ))
}

fn build_instance(
    parent: ObjArc<IRustModuleParent>,
) -> Result<ObjArc<GenericModuleInstance>, Error> {
    let core_desc = tasks_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(
        core_desc,
        (build_tasks_interface as _, vec![core_descriptor()]),
    );

    Ok(GenericModuleInstance::new(parent, interfaces))
}

fn build_tasks_interface(
    instance: ObjArc<GenericModuleInstance>,
    dep_map: &HashMap<ModuleInterfaceDescriptor, Option<ObjWeak<IModuleInterface>>>,
) -> Result<ObjArc<IModuleInterface>, Error> {
    let core_interface = dep_map
        .get(&core_descriptor())
        .map(|i| i.as_ref().unwrap().upgrade());

    if core_interface.is_none() || core_interface.as_ref().unwrap().is_none() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "fimo-core interface not found",
        ));
    }

    let core_interface = core_interface.unwrap().unwrap();
    let core_interface: ObjArc<FimoCore> = IModuleInterface::try_downcast_arc(core_interface)?;

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
        .unwrap_or(None)
        .unwrap_or(SettingsItemType::Null)
        .is_object()
    {
        registry
            .write(settings_path, SettingsItem::new_object())
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

    let base = ObjArc::new(TaskInterface {
        runtime: TaskRuntime::new(num_cores, max_tasks, allocated_tasks),
        parent: ObjArc::coerce_object(instance),
    });

    Ok(ObjArc::coerce_object(base))
}
