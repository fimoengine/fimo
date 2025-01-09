use fimo_std::{
    r#async::{BlockingContext, EventLoop},
    context::ContextBuilder,
    declare_items, emit_info,
    error::AnyError,
    export_module,
    module::*,
    tracing::{Config, Level, ThreadAccess, default_subscriber},
};

declare_items! {
    extern a_export_0 @ (0, 1, 0): i32;
    extern a_export_1 @ (0, 1, 0): i32;

    mod b {
        extern b_export_0 @ (0, 1, 0): i32;
        extern b_export_1 @ (0, 1, 0): i32;
    }
}

export_module! {
    mod A {
        name: "a",
        description: "Test module a",
        exports: {
            a0: AExport0 = &5,
            a1: AExport1 = &10,
        },
    }
}

export_module! {
    mod B {
        name: "b",
        description: "Test module b",
        author: "Fimo",
        exports: {
            b0: b::BExport0 = &-2,
            b1: b::BExport1 = &77,
        },
    }
}

export_module! {
    mod C {
        name: "c",
        description: "Test module c",
        author: "Fimo",
        license: "None",
        parameters: {
            pub_pub: {
                default: u32(0),
                read_group: public,
                write_group: public,
            },
            pub_dep: {
                default: u32(1),
                read_group: public,
                write_group: dependency,
            },
            pub_pri: {
                default: u32(2),
                read_group: public,
                write_group: private,
            },
            dep_pub: {
                default: u32(3),
                read_group: dependency,
                write_group: public,
            },
            dep_dep: {
                default: u32(4),
                read_group: dependency,
                write_group: dependency,
            },
            dep_pri: {
                default: u32(5),
                read_group: dependency,
                write_group: private,
            },
            pri_pub: {
                default: u32(6),
                read_group: private,
                write_group: public,
            },
            pri_dep: {
                default: u32(7),
                read_group: private,
                write_group: dependency,
            },
            pri_pri: {
                default: u32(8),
                read_group: private,
                write_group: private,
            },
        },
        resources: {
            empty: "",
            a: "a.bin",
            b: "b.txt",
            img: "c/d.img",
        },
        namespaces: [
            b::NamespaceItem,
        ],
        imports: {
            a_0: AExport0,
            a_1: AExport1,
            b_0: b::BExport0,
            b_1: b::BExport1,
        },
        constructor: CConstructor,
    }
}

struct CConstructor;

impl<'m> ModuleConstructor<C<'m>> for CConstructor {
    fn construct<'a>(
        module: ConstructorModule<'a, C<'m>>,
        set: LoadingSetView<'_>,
    ) -> Result<&'a mut <C<'m> as Module>::Data, AnyError> {
        let module = module.unwrap()?;

        let parameters = module.parameters();
        assert_eq!(parameters.pub_pub.read(), 0u32);
        assert_eq!(parameters.pub_dep.read(), 1u32);
        assert_eq!(parameters.pub_pri.read(), 2u32);
        assert_eq!(parameters.dep_pub.read(), 3u32);
        assert_eq!(parameters.dep_dep.read(), 4u32);
        assert_eq!(parameters.dep_pri.read(), 5u32);
        assert_eq!(parameters.pri_pub.read(), 6u32);
        assert_eq!(parameters.pri_dep.read(), 7u32);
        assert_eq!(parameters.pri_pri.read(), 8u32);
        parameters.pub_pub.write(0);
        parameters.pub_dep.write(1);
        parameters.pub_pri.write(2);
        parameters.dep_pub.write(3);
        parameters.dep_dep.write(4);
        parameters.dep_pri.write(5);
        parameters.pri_pub.write(6);
        parameters.pri_dep.write(7);
        parameters.pri_pri.write(8);

        let resources = module.resources();
        emit_info!(
            &module.context(),
            "empty: {}",
            resources.empty().to_string_lossy()
        );
        emit_info!(&module.context(), "a: {}", resources.a().to_string_lossy());
        emit_info!(&module.context(), "b: {}", resources.b().to_string_lossy());
        emit_info!(
            &module.context(),
            "img: {}",
            resources.img().to_string_lossy()
        );

        let imports = module.imports();
        assert_eq!(*imports.a_0(), 5);
        assert_eq!(*imports.a_1(), 10);
        assert_eq!(*imports.b_0(), -2);
        assert_eq!(*imports.b_1(), 77);

        let info = module.module_info();
        emit_info!(&module.context(), "{info}");

        <DefaultConstructor as ModuleConstructor<C<'m>>>::construct(module.into(), set)
    }

    fn destroy(module: PreModule<'_, C<'m>>, data: &mut <C<'m> as Module>::Data) {
        emit_info!(&module.context(), "dropping module: {data:?}");
        <DefaultConstructor as ModuleConstructor<C<'m>>>::destroy(module, data);
    }
}

#[test]
fn load_modules() -> Result<(), AnyError> {
    let context = <ContextBuilder>::new()
        .with_tracing_config(Config::new(
            None,
            Some(Level::Trace),
            [default_subscriber()],
        ))
        .build()?;

    let _access = ThreadAccess::new(&context);
    let _event_loop = EventLoop::new(&context)?;

    let blocking = BlockingContext::new(&context)?;
    blocking.block_on(async move {
        let _prune = PruneInstancesOnDrop::new(&context);

        let set = LoadingSet::new(&context)?;
        set.view()
            .add_modules_from_local(|_| LoadingFilterRequest::Load)?;
        set.view().commit().await?;

        let module = PseudoModule::new(&context)?;
        let a = ModuleInfo::find_by_name(&context, c"a")?;
        let b = ModuleInfo::find_by_name(&context, c"b")?;
        let c = ModuleInfo::find_by_name(&context, c"c")?;
        assert!(module.module_info().is_loaded());
        assert!(a.is_loaded());
        assert!(b.is_loaded());
        assert!(c.is_loaded());

        module.add_dependency(&a)?;
        module.add_dependency(&b)?;
        module.add_dependency(&c)?;

        let a_0 = module.load_symbol::<AExport0>()?;
        assert_eq!(*a_0, 5);

        assert!(module.load_symbol::<b::BExport0>().is_err());
        module.add_namespace(b::NamespaceItem::NAME)?;
        assert!(module.load_symbol::<b::BExport0>().is_ok());

        let info = module.module_info().to_owned();
        let _guard = info
            .try_acquire_module_strong()
            .ok_or(AnyError::new("failed to acquire module"))?;

        drop(module);
        assert!(a.is_loaded());
        assert!(b.is_loaded());
        assert!(c.is_loaded());

        Ok(())
    })
}
