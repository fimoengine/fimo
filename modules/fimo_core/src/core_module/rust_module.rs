use crate::core_module::{construct_module_info, get_core_interface_descriptor, MutexWrapper};
use crate::CoreInterface;
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust_loader::{RustModule, RustModuleExt};
use fimo_module_core::{ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Weak};

fimo_module_core::export_rust_module! {fimo_ffi_core::TypeWrapper(construct_module)}

#[allow(dead_code, improper_ctypes_definitions)]
extern "C-unwind" fn construct_module() -> Result<Box<dyn RustModuleExt>, Box<dyn Error>> {
    Ok(GenericModule::new(construct_module_info(), build_instance))
}

fn build_instance(parent: Arc<RustModule>) -> Result<Arc<GenericModuleInstance>, Box<dyn Error>> {
    let core_desc = get_core_interface_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(core_desc, (build_core_interface as _, vec![]));

    Ok(GenericModuleInstance::new(parent, interfaces))
}

fn build_core_interface(
    instance: Arc<dyn ModuleInstance>,
    _dep_map: &HashMap<ModuleInterfaceDescriptor, Option<Weak<dyn ModuleInterface>>>,
) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>> {
    Ok(Arc::new(MutexWrapper {
        data: Mutex::new(CoreInterface::new()),
        parent: instance,
    }))
}
