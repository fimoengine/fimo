use crate::module::core_bindings::scope_builder;
use crate::module::{construct_module_info, get_actix_interface_descriptor, FimoActixInterface};
use crate::FimoActixServer;
use fimo_actix_interface::ScopeBuilder;
use fimo_core_interface::rust::{
    build_interface_descriptor as core_descriptor,
    settings_registry::{SettingsItem, SettingsItemType, SettingsRegistryPath},
    FimoCore,
};
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust_loader::{RustModule, RustModuleExt};
use fimo_module_core::{ModuleInstance, ModuleInterface, ModuleInterfaceDescriptor};
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
    let core_desc = get_actix_interface_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(
        core_desc,
        (build_tasks_interface as _, vec![core_descriptor()]),
    );

    let pkg_versions = HashMap::new();
    Ok(GenericModuleInstance::new(parent, pkg_versions, interfaces))
}

fn build_tasks_interface(
    instance: Arc<dyn ModuleInstance>,
    dep_map: &HashMap<ModuleInterfaceDescriptor, Option<Weak<dyn ModuleInterface>>>,
) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>> {
    let core_interface = dep_map
        .get(&core_descriptor())
        .map(|i| Weak::upgrade(i.as_ref().unwrap()));

    if core_interface.is_none() || core_interface.as_ref().unwrap().is_none() {
        return Err(Box::new(std::io::Error::new(
            ErrorKind::NotFound,
            "fimo-core interface not found",
        )));
    }

    let core_interface =
        unsafe { fimo_core_interface::rust::cast_interface(core_interface.unwrap().unwrap())? };

    const DEFAULT_PORT: usize = 8080usize;
    const DEFAULT_ENABLE_CORE_BINDINGS: bool = true;

    let settings_path = SettingsRegistryPath::new("fimo-actix").unwrap();
    let port_path = settings_path.join(SettingsRegistryPath::new("port").unwrap());
    let enable_bindings_path =
        settings_path.join(SettingsRegistryPath::new("core_bindings").unwrap());

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

    let port = registry
        .try_read_or(port_path, DEFAULT_PORT)
        .unwrap()
        .unwrap_or(DEFAULT_PORT);
    let enable_bindings = registry
        .try_read_or(enable_bindings_path, DEFAULT_ENABLE_CORE_BINDINGS)
        .unwrap()
        .unwrap_or(DEFAULT_ENABLE_CORE_BINDINGS);

    let address = format!("127.0.0.1:{}", port);

    let server = Arc::new(FimoActixInterface {
        server: FimoActixServer::new(address),
        parent: instance,
    });

    if enable_bindings {
        bind_core(server.clone(), &*core_interface)
    }

    Ok(server)
}

fn bind_core(server: Arc<FimoActixInterface>, core: &FimoCore) {
    let (builder, _callback) = scope_builder(core);
    let scope_builder = ScopeBuilder::from(Box::new(builder));
    server.server.register_scope("/core", scope_builder);
}
