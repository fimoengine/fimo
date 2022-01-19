use ci::rust::settings_registry::{SettingsEvent, SettingsItem, SettingsRegistryPath};
use fimo_core_int as ci;
use fimo_core_int::rust::IFimoCore;
use fimo_ffi::ObjArc;
use fimo_module_core::{Error, FimoInterface, IModuleInterface};
use std::alloc::System;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[global_allocator]
static A: System = System;

#[test]
#[cfg(feature = "rust_module")]
fn load_dynamic() -> Result<(), Error> {
    let core_path = module_loading::core_path();

    let module_loader = fimo_module_core::rust_loader::RustLoader::new();
    let core_module = unsafe { module_loader.load_module_raw(core_path)? };

    println!(
        "Core info: {}, Path: {}",
        core_module.module_info(),
        core_module.module_path_buf().display()
    );

    let core_instance = core_module.new_instance()?;

    println!(
        "Available interfaces: {:?}",
        core_instance.available_interfaces()
    );

    let core_descriptor = core_instance
        .available_interfaces()
        .iter()
        .find(|interface| interface.name == IFimoCore::NAME)
        .unwrap();

    println!("Core interface: {}", core_descriptor);
    println!(
        "Core dependencies: {:?}",
        core_instance.dependencies(core_descriptor).into_rust()?
    );

    let core_interface = core_instance.interface(core_descriptor).into_rust()?;
    print!("Core version: {}", core_interface.version());

    let _: ObjArc<IFimoCore> = IModuleInterface::try_downcast_arc(core_interface)?;
    Ok(())
}

#[test]
#[cfg(feature = "rust_module")]
fn settings_registry() -> Result<(), Error> {
    let db = module_loading::ModuleDatabase::new()?;
    let core = db.core_interface();

    let settings_registry = core.get_settings_registry();

    let array = vec![SettingsItem::from(false), SettingsItem::from(false)];
    let object = BTreeMap::new();

    let none_path = SettingsRegistryPath::new("none").unwrap();
    let bool_path = SettingsRegistryPath::new("bool").unwrap();
    let integer_path = SettingsRegistryPath::new("integer").unwrap();
    let float_path = SettingsRegistryPath::new("float").unwrap();
    let string_path = SettingsRegistryPath::new("string").unwrap();
    let array_path = SettingsRegistryPath::new("array").unwrap();
    let object_path = SettingsRegistryPath::new("object").unwrap();

    let _ = settings_registry.write(none_path, ()).unwrap();
    let _ = settings_registry.write(bool_path, false).unwrap();
    let _ = settings_registry.write(integer_path, 65u32).unwrap();
    let _ = settings_registry.write(float_path, 20.0).unwrap();
    let _ = settings_registry
        .write(string_path, String::from("Hey!"))
        .unwrap();
    let _ = settings_registry.write(array_path, array.clone()).unwrap();
    let _ = settings_registry
        .write(object_path, object.clone())
        .unwrap();

    assert_eq!(
        settings_registry
            .read::<SettingsItem, _>(none_path)
            .unwrap()
            .unwrap(),
        SettingsItem::from(())
    );
    assert!(!settings_registry
        .read::<bool, _>(bool_path)
        .unwrap()
        .unwrap());
    assert_eq!(
        settings_registry
            .read::<u32, _>(integer_path)
            .unwrap()
            .unwrap(),
        65u32
    );
    assert!(
        (settings_registry
            .read::<f64, _>(float_path)
            .unwrap()
            .unwrap()
            - 20.0)
            .abs()
            < f64::EPSILON
    );
    assert_eq!(
        settings_registry
            .read::<String, _>(string_path)
            .unwrap()
            .unwrap(),
        String::from("Hey!")
    );
    assert_eq!(
        settings_registry
            .read::<Vec<SettingsItem>, _>(array_path)
            .unwrap()
            .unwrap(),
        array
    );
    assert_eq!(
        settings_registry
            .read::<BTreeMap<String, SettingsItem>, _>(object_path)
            .unwrap()
            .unwrap(),
        object
    );

    let array_index_path = SettingsRegistryPath::new("array[1]").unwrap();
    let sub_object_path = object_path.join(SettingsRegistryPath::new("name").unwrap());

    let _ = settings_registry.write(array_index_path, true);
    let _ = settings_registry.write(&sub_object_path, true);

    assert!(settings_registry
        .read::<bool, _>(array_index_path)
        .unwrap()
        .unwrap(),);
    assert!(settings_registry
        .read::<bool, _>(sub_object_path)
        .unwrap()
        .unwrap(),);

    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&flag);
    let _callback = settings_registry.register_callback(
        none_path,
        Box::new(move |path: &SettingsRegistryPath, event: &SettingsEvent| {
            flag_clone.store(true, Ordering::Relaxed);
            assert_eq!(path, none_path);
            assert!(matches!(event, SettingsEvent::Remove { .. }));
        }),
    );
    assert_eq!(
        settings_registry.remove(none_path).unwrap().unwrap(),
        SettingsItem::from(())
    );
    assert!(flag.load(Ordering::Acquire));

    println!("{:?}", settings_registry.read_all());

    Ok(())
}
