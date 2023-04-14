use fimo_core_int::settings::{ISettingsRegistryExt, SettingsEvent, SettingsItem, SettingsPath};
use fimo_core_int::IFimoCore;
use fimo_ffi::provider::request_obj;
use fimo_module::context::{AppContext, IContext, IInterface, IModule};
use fimo_module::{Error, ErrorKind, QueryBuilder};
use fimo_tests::ContextBuilder;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[test]
fn load_dynamic() -> Result<(), Error> {
    let context_builder = ContextBuilder::new();

    AppContext::new().enter(|context| {
        let core_module = context.load_module(context_builder.module_path("core"), &[])?;

        println!(
            "Core info: {}, Path: {}",
            core_module.module_info(),
            core_module.module_path().display()
        );

        println!("Available interfaces: {:?}", core_module.interfaces());

        let core_descriptor = core_module
            .interfaces()
            .iter()
            .find(|interface| interface.name == "fimo::interfaces::core")
            .unwrap();
        println!("Core interface: {}", core_descriptor);

        let core_query = core_descriptor.clone().into();
        let core = context.get_interface(core_query)?;
        println!("Core version: {}", core.version());

        let _core = request_obj::<dyn IFimoCore + '_>(core)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Core interface not found"))?;
        Ok(())
    })
}

#[test]
fn settings_registry() -> Result<(), Error> {
    ContextBuilder::new().with_core().build(|context| {
        let core = context
            .get_interface(QueryBuilder.query_version::<dyn IFimoCore>(super::CORE_VERSION))?;
        let core = request_obj::<dyn IFimoCore + '_>(core)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Core interface not found"))?;

        let settings_registry = core.settings();

        let array = vec![SettingsItem::from(false), SettingsItem::from(false)];
        let object = BTreeMap::new();

        let none_path = SettingsPath::new("none").unwrap();
        let bool_path = SettingsPath::new("bool").unwrap();
        let integer_path = SettingsPath::new("integer").unwrap();
        let float_path = SettingsPath::new("float").unwrap();
        let string_path = SettingsPath::new("string").unwrap();
        let array_path = SettingsPath::new("array").unwrap();
        let object_path = SettingsPath::new("object").unwrap();

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

        let array_index_path = SettingsPath::new("array[1]").unwrap();
        let sub_object_path = object_path.join(SettingsPath::new("name").unwrap());

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
            move |_: _, path: &SettingsPath, event: SettingsEvent| {
                flag_clone.store(true, Ordering::Relaxed);
                assert_eq!(path, none_path);
                assert_eq!(event, SettingsEvent::Removed);
            },
        );
        assert_eq!(
            settings_registry.remove_item(none_path).unwrap().unwrap(),
            SettingsItem::from(())
        );
        assert!(flag.load(Ordering::Acquire));

        println!("{:?}", settings_registry.read_all());

        Ok(())
    })
}
