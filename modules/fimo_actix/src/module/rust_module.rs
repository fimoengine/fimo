use crate::module::core_bindings::scope_builder;
use crate::module::{construct_module_info, get_actix_interface_descriptor, FimoActixInterface};
use crate::FimoActixServer;
use fimo_actix_interface::ScopeBuilder;
use fimo_core_interface::rust::{FimoCore, InterfaceGuard, SettingsItem};
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
    let core_desc = get_actix_interface_descriptor();

    let mut interfaces = HashMap::new();
    interfaces.insert(
        core_desc,
        (
            build_tasks_interface as _,
            vec![core_interface_descriptor()],
        ),
    );

    let pkg_versions = HashMap::new();
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
            if !settings_registry.is_object("fimo-actix").unwrap_or(false) {
                settings_registry.write(
                    "fimo-actix",
                    SettingsItem::Object {
                        0: Default::default(),
                    },
                );
            }
            let port_setting = settings_registry.read("fimo-actix::port");
            let core_bindings_setting = settings_registry.read("fimo-actix::core_bindings");

            let port = match port_setting {
                None => {
                    let port = 8080usize;
                    settings_registry.write("fimo-actix::port", SettingsItem::U64(port as u64));
                    port
                }
                Some(SettingsItem::U64(n)) => n as usize,
                _ => {
                    return Err(Box::new(std::io::Error::new(
                        ErrorKind::Other,
                        "fimo-actix::port: expected u64 value",
                    )));
                }
            };

            let core_bindings = match core_bindings_setting {
                None => {
                    let core_bindings = true;
                    settings_registry.write(
                        "fimo-actix::core_bindings",
                        SettingsItem::Bool(core_bindings),
                    );
                    core_bindings
                }
                Some(SettingsItem::Bool(n)) => n,
                _ => {
                    return Err(Box::new(std::io::Error::new(
                        ErrorKind::Other,
                        "fimo-actix::core_bindings: expected bool value",
                    )));
                }
            };

            let address = format!("127.0.0.1:{}", port);

            let server = Arc::new(FimoActixInterface {
                server: FimoActixServer::new(address),
                parent: instance,
            });

            if core_bindings {
                bind_core(server.clone(), guard)
            }

            Ok(server)
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

fn bind_core(server: Arc<FimoActixInterface>, core: InterfaceGuard<'_, dyn FimoCore>) {
    let (builder, _callback) = scope_builder(core);
    let scope_builder = ScopeBuilder::from(Box::new(builder));
    server.server.register_scope("/core", scope_builder);
}
