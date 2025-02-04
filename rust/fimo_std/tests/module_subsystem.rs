#![feature(used_with_arg)]
#![feature(const_trait_impl)]

use std::{pin::Pin, ptr::NonNull};

use fimo_std::{
    r#async::{BlockingContext, EventLoop},
    context::ContextBuilder,
    emit_info,
    error::AnyError,
    ffi::Viewable,
    module::{
        exports::Builder,
        info::Info,
        instance::{GenericInstance, PseudoInstance, UninitInstanceView},
        loading_set::{LoadingFilterRequest, LoadingSet, LoadingSetView},
        parameters::ParameterAccessGroup,
        symbols::SymbolInfo,
        *,
    },
    symbol,
    tracing::{Config, Level, ThreadAccess, default_subscriber},
};

symbol! {
    symbol A0 @ (0, 1, 0) = a_export_0: *const i32;
    symbol A1 @ (0, 1, 0) = a_export_1: *const i32;
    symbol B0 @ (0, 1, 0) = "b"::b_export_0: *const i32;
    symbol B1 @ (0, 1, 0) = "b"::b_export_1: *const i32;
}

#[fimo_std::module::exports::export_module]
const _: &exports::Export<'_> = Builder::<AView<'_>, A>::new(c"a")
    .with_description(c"Test module a")
    .with_author(c"fimo")
    .with_export::<crate::A0>("a0", &5 as *const _)
    .with_export::<crate::A1>("a1", &10 as *const _)
    .build();

#[fimo_std::module::exports::export_module]
const _: &exports::Export<'_> = Builder::<BView<'_>, B>::new(c"b")
    .with_description(c"Test module b")
    .with_author(c"fimo")
    .with_export::<crate::B0>("b0", &-2 as *const _)
    .with_export::<crate::B1>("b1", &77 as *const _)
    .build();

#[fimo_std::module::exports::export_module]
const _: &exports::Export<'_> = Builder::<CView<'_>, C>::new(c"c")
    .with_description(c"Test module c")
    .with_author(c"fimo")
    .with_license(c"none")
    .with_parameter::<u32>(
        "pub_pub",
        c"pub_pub",
        0,
        Some(ParameterAccessGroup::Public),
        Some(ParameterAccessGroup::Public),
        None,
        None,
    )
    .with_parameter::<u32>(
        "pub_dep",
        c"pub_dep",
        1,
        Some(ParameterAccessGroup::Public),
        Some(ParameterAccessGroup::Dependency),
        None,
        None,
    )
    .with_parameter::<u32>(
        "pub_pri",
        c"pub_pri",
        2,
        Some(ParameterAccessGroup::Public),
        None,
        None,
        None,
    )
    .with_parameter::<u32>(
        "dep_pub",
        c"dep_pub",
        3,
        Some(ParameterAccessGroup::Dependency),
        Some(ParameterAccessGroup::Public),
        None,
        None,
    )
    .with_parameter::<u32>(
        "dep_dep",
        c"dep_dep",
        4,
        Some(ParameterAccessGroup::Dependency),
        Some(ParameterAccessGroup::Dependency),
        None,
        None,
    )
    .with_parameter::<u32>(
        "dep_pri",
        c"dep_pri",
        5,
        Some(ParameterAccessGroup::Dependency),
        None,
        None,
        None,
    )
    .with_parameter::<u32>(
        "pri_pub",
        c"pri_pub",
        6,
        None,
        Some(ParameterAccessGroup::Public),
        None,
        None,
    )
    .with_parameter::<u32>(
        "pri_dep",
        c"pri_dep",
        7,
        None,
        Some(ParameterAccessGroup::Dependency),
        None,
        None,
    )
    .with_parameter::<u32>("pri_pri", c"pri_pri", 8, None, None, None, None)
    .with_resource("empty", c"")
    .with_resource("a", c"a.bin")
    .with_resource("b", c"b.txt")
    .with_resource("img", c"c/d.img")
    .with_import::<crate::A0>("a0")
    .with_import::<crate::A1>("a1")
    .with_import::<crate::B0>("b0")
    .with_import::<crate::B1>("b1")
    .with_state::<crate::CState, _>(CState::init, CState::deinit)
    .build();

#[derive(Debug)]
struct CState;

impl CState {
    fn init(
        instance: Pin<&UninitInstanceView<'_, CView<'_>>>,
        _set: LoadingSetView<'_>,
    ) -> Result<NonNull<Self>, std::convert::Infallible> {
        let parameters = instance.parameters();
        assert_eq!(parameters.pub_pub().read(), 0u32);
        assert_eq!(parameters.pub_dep().read(), 1u32);
        assert_eq!(parameters.pub_pri().read(), 2u32);
        assert_eq!(parameters.dep_pub().read(), 3u32);
        assert_eq!(parameters.dep_dep().read(), 4u32);
        assert_eq!(parameters.dep_pri().read(), 5u32);
        assert_eq!(parameters.pri_pub().read(), 6u32);
        assert_eq!(parameters.pri_dep().read(), 7u32);
        assert_eq!(parameters.pri_pri().read(), 8u32);
        parameters.pub_pub().write(0);
        parameters.pub_dep().write(1);
        parameters.pub_pri().write(2);
        parameters.dep_pub().write(3);
        parameters.dep_dep().write(4);
        parameters.dep_pri().write(5);
        parameters.pri_pub().write(6);
        parameters.pri_dep().write(7);
        parameters.pri_pri().write(8);

        let resources = instance.resources();
        emit_info!(instance.context(), "empty: {}", resources.empty());
        emit_info!(instance.context(), "a: {}", resources.a());
        emit_info!(instance.context(), "b: {}", resources.b());
        emit_info!(instance.context(), "img: {}", resources.img());

        let imports = instance.imports();
        assert_eq!(*imports.a0(), 5);
        assert_eq!(*imports.a1(), 10);
        // assert_eq!(*imports.b0(), -2);
        // assert_eq!(*imports.b1(), 77);

        let info = instance.info();
        emit_info!(instance.context(), "{info:?}");

        Ok(NonNull::from(&CState))
    }

    fn deinit(_instance: Pin<&UninitInstanceView<'_, CView<'_>>>, _value: NonNull<Self>) {}
}

#[test]
fn load_modules() -> Result<(), AnyError> {
    let context = ContextBuilder::new()
        .with_tracing_config(
            Config::default()
                .with_max_level(Level::Trace)
                .with_subscribers(&[default_subscriber()]),
        )
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

        let instance = PseudoInstance::new(&context)?;
        let a = Info::find_by_name(&context, c"a")?;
        let b = Info::find_by_name(&context, c"b")?;
        let c = Info::find_by_name(&context, c"c")?;
        assert!(instance.info().is_loaded());
        assert!(a.view().is_loaded());
        assert!(b.view().is_loaded());
        assert!(c.view().is_loaded());

        instance.add_dependency(&a)?;
        instance.add_dependency(&b)?;
        instance.add_dependency(&c)?;

        let a_0 = instance.load_symbol::<A0>()?;
        assert_eq!(*a_0, 5);

        assert!(instance.load_symbol::<B0>().is_err());
        instance.add_namespace(B0::NAMESPACE)?;
        assert!(instance.load_symbol::<B0>().is_ok());

        let info = instance.info().to_info();
        if !info.view().try_ref_instance_strong() {
            return Err(AnyError::new("failed to acquire module"));
        }

        drop(instance);
        assert!(a.view().is_loaded());
        assert!(b.view().is_loaded());
        assert!(c.view().is_loaded());

        unsafe {
            info.view().unref_instance_strong();
        }

        Ok(())
    })
}
