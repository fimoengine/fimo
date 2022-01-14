use crate::core_module::{construct_module_info, CoreWrapper};
use fimo_core_interface::rust::build_interface_descriptor;
use fimo_ffi::{ObjArc, ObjWeak};
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust_loader::{IRustModuleInner, IRustModuleParent};
use fimo_module_core::{Error, IModuleInterface, ModuleInterfaceDescriptor};
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
    let core_desc = build_interface_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(core_desc, (build_core_interface as _, vec![]));
    Ok(GenericModuleInstance::new(parent, interfaces))
}

fn build_core_interface(
    instance: ObjArc<GenericModuleInstance>,
    _dep_map: &HashMap<ModuleInterfaceDescriptor, Option<ObjWeak<IModuleInterface>>>,
) -> Result<ObjArc<IModuleInterface>, Error> {
    let base = ObjArc::new(CoreWrapper {
        interface: Default::default(),
        parent: ObjArc::coerce_object(instance),
    });

    Ok(ObjArc::coerce_object(base))
}
