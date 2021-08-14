use fimo_core_interface as ci;
use fimo_core_interface::rust::SettingsEvent;
use fimo_module_core as module;
use fimo_module_core::ModuleLoader;
use std::alloc::System;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[global_allocator]
static A: System = System;

fn module_path() -> Result<PathBuf, Box<dyn Error>> {
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

#[test]
#[cfg(feature = "rust_module")]
fn load_dynamic() -> Result<(), Box<dyn Error>> {
    let core_path = module_path()?;

    let module_loader = module::rust_loader::RustLoader::new();
    let core_module = unsafe { module_loader.load_module_library(core_path.as_path())? };

    println!(
        "Core info: {}, Path: {}",
        core_module.get_module_info(),
        core_module.get_module_path().display()
    );

    let core_instance = unsafe { ci::rust::cast_instance(core_module.create_instance()?)? };

    println!(
        "Available interfaces: {:?}",
        core_instance.get_available_interfaces()
    );

    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();

    println!("Core interface: {}", core_descriptor);
    println!(
        "Core dependencies: {:?}",
        core_instance.get_interface_dependencies(core_descriptor)?
    );

    let core = unsafe { ci::rust::cast_interface(core_instance.get_interface(core_descriptor)?)? };
    print!("Core version: {}", core.lock().get_interface_version());

    Ok(())
}

#[test]
#[cfg(feature = "rust_module")]
fn settings_registry() -> Result<(), Box<dyn Error>> {
    let core_path = module_path()?;
    let module_loader = module::rust_loader::RustLoader::new();
    let core_module = unsafe { module_loader.load_module_library(core_path.as_path())? };
    let core_instance = unsafe { ci::rust::cast_instance(core_module.create_instance()?)? };
    let core_descriptor = core_instance
        .get_available_interfaces()
        .iter()
        .find(|interface| interface.name == "fimo-core")
        .unwrap();
    let core = unsafe { ci::rust::cast_interface(core_instance.get_interface(core_descriptor)?)? };

    let mut guard = core.lock();
    let settings_registry = guard.as_settings_registry_mut();

    let none = ci::rust::SettingsItem::Null;
    let bool = ci::rust::SettingsItem::Bool(false);
    let integer = ci::rust::SettingsItem::U64(65);
    let float = ci::rust::SettingsItem::F64(20.0);
    let string = ci::rust::SettingsItem::String(String::from("Hey!"));
    let array = ci::rust::SettingsItem::Array(vec![
        ci::rust::SettingsItem::Bool(false),
        ci::rust::SettingsItem::Bool(false),
    ]);
    let object = ci::rust::SettingsItem::Object {
        0: Default::default(),
    };

    settings_registry.write("none", none.clone());
    settings_registry.write("bool", bool.clone());
    settings_registry.write("integer", integer.clone());
    settings_registry.write("float", float.clone());
    settings_registry.write("string", string.clone());
    settings_registry.write("array", array.clone());
    settings_registry.write("object", object.clone());

    assert_eq!(settings_registry.read("none").unwrap(), none);
    assert_eq!(settings_registry.read("bool").unwrap(), bool);
    assert_eq!(settings_registry.read("integer").unwrap(), integer);
    assert_eq!(settings_registry.read("float").unwrap(), float);
    assert_eq!(settings_registry.read("string").unwrap(), string);
    assert_eq!(settings_registry.read("array").unwrap(), array);
    assert_eq!(settings_registry.read("object").unwrap(), object);

    settings_registry.write("array[1]", ci::rust::SettingsItem::Bool(true));
    settings_registry.write("object::name", ci::rust::SettingsItem::Bool(true));

    assert_eq!(
        settings_registry.read("array[1]").unwrap(),
        ci::rust::SettingsItem::Bool(true)
    );
    assert_eq!(
        settings_registry.read("object::name").unwrap(),
        ci::rust::SettingsItem::Bool(true)
    );

    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&flag);
    settings_registry.register_callback(
        "none",
        Box::new(move |path, event| {
            flag_clone.store(true, Ordering::Relaxed);
            assert_eq!(path, "none");
            assert!(matches!(event, SettingsEvent::Remove { .. }));
        }),
    );
    assert_eq!(settings_registry.remove("none").unwrap(), none);
    assert!(flag.load(Ordering::Acquire));

    println!("{:?}", settings_registry.read_all());

    Ok(())
}
