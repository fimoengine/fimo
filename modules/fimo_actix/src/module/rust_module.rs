use crate::module::core_bindings::scope_builder;
use crate::module::{construct_module_info, FimoActixInterface};
use crate::FimoActixServer;
use fimo_actix_int::{IFimoActix, ScopeBuilder};
use fimo_core_int::rust::{
    settings_registry::{SettingsItem, SettingsItemType, SettingsRegistryPath},
    IFimoCore,
};
use fimo_ffi::{ObjArc, ObjWeak};
use fimo_generic_module::{GenericModule, GenericModuleInstance};
use fimo_module_core::rust_loader::{IRustModuleInner, IRustModuleParent};
use fimo_module_core::{
    Error, ErrorKind, FimoInterface, IModuleInterface, ModuleInterfaceDescriptor,
};
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
    let core_desc = IFimoActix::new_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(
        core_desc,
        (
            build_tasks_interface as _,
            vec![IFimoCore::new_descriptor()],
        ),
    );

    Ok(GenericModuleInstance::new(parent, interfaces))
}

fn build_tasks_interface(
    instance: ObjArc<GenericModuleInstance>,
    dep_map: &HashMap<ModuleInterfaceDescriptor, Option<ObjWeak<IModuleInterface>>>,
) -> Result<ObjArc<IModuleInterface>, Error> {
    let core_interface = dep_map
        .get(&IFimoCore::new_descriptor())
        .map(|i| i.as_ref().unwrap().upgrade());

    if core_interface.is_none() || core_interface.as_ref().unwrap().is_none() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "fimo-core interface not found",
        ));
    }

    let core_interface = core_interface.unwrap().unwrap();
    let core_interface: ObjArc<IFimoCore> = IModuleInterface::try_downcast_arc(core_interface)?;

    const DEFAULT_PORT: usize = 8080usize;
    const DEFAULT_ENABLE_CORE_BINDINGS: bool = true;

    let settings_path = SettingsRegistryPath::new("fimo-actix").unwrap();
    let port_path = settings_path.join(SettingsRegistryPath::new("port").unwrap());
    let enable_bindings_path =
        settings_path.join(SettingsRegistryPath::new("core_bindings").unwrap());

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

    let port = registry
        .try_read_or(port_path, DEFAULT_PORT)
        .unwrap()
        .unwrap_or(DEFAULT_PORT);
    let enable_bindings = registry
        .try_read_or(enable_bindings_path, DEFAULT_ENABLE_CORE_BINDINGS)
        .unwrap()
        .unwrap_or(DEFAULT_ENABLE_CORE_BINDINGS);

    let address = format!("127.0.0.1:{}", port);

    let mut server = ObjArc::new(FimoActixInterface {
        server: FimoActixServer::new(address),
        parent: ObjArc::coerce_object(instance),
        core: None,
    });

    if enable_bindings {
        server = bind_core(server, core_interface)
    }

    Ok(ObjArc::coerce_object(server))
}

fn bind_core(
    mut server: ObjArc<FimoActixInterface>,
    core: ObjArc<IFimoCore>,
) -> ObjArc<FimoActixInterface> {
    let (builder, callback) = scope_builder(&*core);
    let scope_builder = ScopeBuilder::from(Box::new(builder));
    server.server.register_scope("/core", scope_builder);

    let inner = ObjArc::get_mut(&mut server).unwrap();
    let (id, _) = callback.into_raw_parts();
    inner.core = Some((core, id));

    server
}
