//! Module subsystem.

use crate::{
    context::{ContextHandle, ContextView},
    error::{AnyError, AnyResult},
    utils::{ConstCStr, ConstNonNull, OpaqueHandle, Viewable},
    version::Version,
};
use std::{
    ffi::CStr,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

pub mod exports;
pub mod info;
pub mod instance;
pub mod loading_set;
pub mod parameters;
pub mod symbols;

use exports::Export;
use info::Info;
use instance::PseudoInstance;
use loading_set::LoadingSet;
use parameters::{
    ParameterAccessGroup, ParameterCast, ParameterInfo, ParameterRepr, ParameterType,
};

/// Virtual function table of the module subsystem.
///
/// Adding fields to the vtable is a breaking change.
#[repr(C)]
#[derive(Debug)]
pub struct VTableV0 {
    pub new_pseudo_instance: unsafe extern "C" fn(
        handle: ContextHandle,
        out: &mut MaybeUninit<PseudoInstance>,
    ) -> AnyResult,
    pub new_loading_set:
        unsafe extern "C" fn(handle: ContextHandle, out: &mut MaybeUninit<LoadingSet>) -> AnyResult,
    pub find_instance_by_name: unsafe extern "C" fn(
        handle: ContextHandle,
        name: ConstCStr,
        out: &mut MaybeUninit<Info>,
    ) -> AnyResult,
    pub find_instance_by_symbol: unsafe extern "C" fn(
        handle: ContextHandle,
        name: ConstCStr,
        namespace: ConstCStr,
        version: Version,
        out: &mut MaybeUninit<Info>,
    ) -> AnyResult,
    pub namespace_exists: unsafe extern "C" fn(
        handle: ContextHandle,
        namespace: ConstCStr,
        out: &mut MaybeUninit<bool>,
    ) -> AnyResult,
    pub prune_instances: unsafe extern "C" fn(handle: ContextHandle) -> AnyResult,
    pub query_parameter: unsafe extern "C" fn(
        handle: ContextHandle,
        module: ConstCStr,
        parameter: ConstCStr,
        r#type: &mut MaybeUninit<ParameterType>,
        read_group: &mut MaybeUninit<ParameterAccessGroup>,
        write_group: &mut MaybeUninit<ParameterAccessGroup>,
    ) -> AnyResult,
    pub read_parameter: unsafe extern "C" fn(
        handle: ContextHandle,
        value: NonNull<()>,
        r#type: ParameterType,
        module: ConstCStr,
        parameter: ConstCStr,
    ) -> AnyResult,
    pub write_parameter: unsafe extern "C" fn(
        handle: ContextHandle,
        value: ConstNonNull<()>,
        r#type: ParameterType,
        module: ConstCStr,
        parameter: ConstCStr,
    ) -> AnyResult,
}

/// Definition of the module subsystem.
pub trait ModuleSubsystem: Copy {
    /// Checks for the presence of a namespace in the module backend.
    ///
    /// A namespace exists, if at least one loaded module exports one symbol in said namespace.
    fn namespace_exists(self, namespace: &CStr) -> Result<bool, AnyError>;

    /// Unloads all unused instances.
    ///
    /// After calling this function, all unreferenced instances are unloaded.
    fn prune_instances(self) -> Result<(), AnyError>;

    /// Queries the info of a module parameter.
    ///
    /// This function can be used to query the datatype, the read access, and the write access of a
    /// module parameter. This function fails, if the parameter can not be found.
    fn query_parameter(self, module: &CStr, parameter: &CStr) -> Result<ParameterInfo, AnyError>;

    /// Reads a module parameter with public read access.
    ///
    /// Reads the value of a module parameter with public read access. The operation fails, if the
    /// parameter does not exist, or if the parameter does not allow reading with a public access.
    fn read_parameter<P: ParameterCast>(
        self,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<P, AnyError>;

    /// Sets a module parameter with public write access.
    ///
    /// Sets the value of a module parameter with public write access. The operation fails, if the
    /// parameter does not exist, or if the parameter does not allow writing with a public access.
    fn write_parameter<P: ParameterCast>(
        self,
        value: P,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<(), AnyError>;
}

impl<'a, T> ModuleSubsystem for T
where
    T: Viewable<ContextView<'a>>,
{
    fn namespace_exists(self, namespace: &CStr) -> Result<bool, AnyError> {
        unsafe {
            let mut out = MaybeUninit::uninit();
            let ctx = self.view();
            let f = ctx.vtable.module_v0.namespace_exists;
            f(ctx.handle, namespace.into(), &mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    fn prune_instances(self) -> Result<(), AnyError> {
        unsafe {
            let ctx = self.view();
            let f = ctx.vtable.module_v0.prune_instances;
            f(ctx.handle).into_result()
        }
    }

    fn query_parameter(self, module: &CStr, parameter: &CStr) -> Result<ParameterInfo, AnyError> {
        unsafe {
            let mut r#type = MaybeUninit::uninit();
            let mut read_group = MaybeUninit::uninit();
            let mut write_group = MaybeUninit::uninit();
            let ctx = self.view();
            let f = ctx.vtable.module_v0.query_parameter;
            f(
                ctx.handle,
                module.into(),
                parameter.into(),
                &mut r#type,
                &mut read_group,
                &mut write_group,
            )
            .into_result()?;

            let r#type = r#type.assume_init();
            let read_group = read_group.assume_init();
            let write_group = write_group.assume_init();
            Ok(ParameterInfo {
                type_: r#type,
                read: read_group,
                write: write_group,
            })
        }
    }

    fn read_parameter<P: ParameterCast>(
        self,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<P, AnyError> {
        unsafe {
            let mut out = MaybeUninit::<P::Repr>::uninit();
            let ctx = self.view();
            (ctx.vtable.module_v0.read_parameter)(
                ctx.handle,
                NonNull::new_unchecked(out.as_mut_ptr()).cast(),
                P::Repr::TYPE,
                module.into(),
                parameter.into(),
            )
            .into_result()?;

            Ok(P::from_repr(out.assume_init()))
        }
    }

    fn write_parameter<P: ParameterCast>(
        self,
        value: P,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<(), AnyError> {
        unsafe {
            let value = ManuallyDrop::new(value.into_repr());
            let ctx = self.view();
            (ctx.vtable.module_v0.write_parameter)(
                ctx.handle,
                ConstNonNull::new_unchecked(&raw const *value).cast(),
                P::Repr::TYPE,
                module.into(),
                parameter.into(),
            )
            .into_result()
        }
    }
}

/// Helper struct that prunes all unused instances on drop.
#[derive(Debug)]
#[repr(transparent)]
pub struct PruneInstancesOnDrop<'a>(ContextView<'a>);

impl<'a> PruneInstancesOnDrop<'a> {
    /// Constructs a new instance of the dropper.
    pub fn new<T: Viewable<ContextView<'a>>>(ctx: T) -> Self {
        let view = ctx.view();
        PruneInstancesOnDrop(view)
    }
}

impl Drop for PruneInstancesOnDrop<'_> {
    fn drop(&mut self) {
        self.0.prune_instances().expect("could not prune instances");
    }
}

// Reexport the module entry function.
#[link(name = "fimo_std", kind = "static")]
unsafe extern "C" {
    #[doc(hidden)]
    pub fn fimo_impl_module_export_iterator(
        f: unsafe extern "C" fn(&Export<'_>, Option<OpaqueHandle>) -> bool,
        handle: Option<OpaqueHandle>,
    );
}
