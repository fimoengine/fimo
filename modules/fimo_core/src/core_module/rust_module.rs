use crate::core_module::{construct_module_info, CoreWrapper, INTERFACE_VTABLE};
use fimo_core_interface::rust::build_interface_descriptor;
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust::module_loader::{RustModule, RustModuleInnerArc};
use fimo_module_core::rust::{ModuleInterfaceArc, ModuleInterfaceCaster, ModuleInterfaceWeak};
use fimo_module_core::ModuleInterfaceDescriptor;
use std::collections::HashMap;
use std::error::Error;
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
    let core_desc = build_interface_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(core_desc, (build_core_interface as _, vec![]));
    Ok(GenericModuleInstance::new(parent, interfaces))
}

fn build_core_interface(
    instance: Arc<GenericModuleInstance>,
    _dep_map: &HashMap<ModuleInterfaceDescriptor, Option<ModuleInterfaceWeak>>,
) -> Result<ModuleInterfaceArc, Box<dyn Error>> {
    let base = Arc::new(CoreWrapper {
        interface: Default::default(),
        parent: GenericModuleInstance::as_module_instance_arc(instance),
    });

    let caster = ModuleInterfaceCaster::new(&INTERFACE_VTABLE);
    unsafe { Ok(ModuleInterfaceArc::from_inner((base, caster))) }
}
