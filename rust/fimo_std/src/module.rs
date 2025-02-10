//! Module subsystem.

use crate::{
    context::{ContextHandle, ContextView, TypeId},
    error::{AnyError, AnyResult},
    utils::{ConstNonNull, OpaqueHandle, Unsafe, Viewable},
    version::Version,
};
use std::{
    ffi::CStr,
    marker::PhantomData,
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
use symbols::{SliceRef, StrRef};

/// Virtual function table of the module subsystem.
///
/// Adding fields to the vtable is a breaking change.
#[repr(C)]
#[derive(Debug)]
pub struct VTableV0 {
    pub profile: unsafe extern "C" fn(handle: ContextHandle) -> Profile,
    pub features: unsafe extern "C" fn(
        handle: ContextHandle,
        &mut MaybeUninit<Option<ConstNonNull<FeatureStatus>>>,
    ) -> usize,
    pub new_pseudo_instance: unsafe extern "C" fn(
        handle: ContextHandle,
        out: &mut MaybeUninit<PseudoInstance>,
    ) -> AnyResult,
    pub new_loading_set:
        unsafe extern "C" fn(handle: ContextHandle, out: &mut MaybeUninit<LoadingSet>) -> AnyResult,
    pub find_instance_by_name: unsafe extern "C" fn(
        handle: ContextHandle,
        name: StrRef<'_>,
        out: &mut MaybeUninit<Info>,
    ) -> AnyResult,
    pub find_instance_by_symbol: unsafe extern "C" fn(
        handle: ContextHandle,
        name: StrRef<'_>,
        namespace: StrRef<'_>,
        version: Version<'_>,
        out: &mut MaybeUninit<Info>,
    ) -> AnyResult,
    pub namespace_exists: unsafe extern "C" fn(
        handle: ContextHandle,
        namespace: StrRef<'_>,
        out: &mut MaybeUninit<bool>,
    ) -> AnyResult,
    pub prune_instances: unsafe extern "C" fn(handle: ContextHandle) -> AnyResult,
    pub query_parameter: unsafe extern "C" fn(
        handle: ContextHandle,
        module: StrRef<'_>,
        parameter: StrRef<'_>,
        r#type: &mut MaybeUninit<ParameterType>,
        read_group: &mut MaybeUninit<ParameterAccessGroup>,
        write_group: &mut MaybeUninit<ParameterAccessGroup>,
    ) -> AnyResult,
    pub read_parameter: unsafe extern "C" fn(
        handle: ContextHandle,
        value: NonNull<()>,
        r#type: ParameterType,
        module: StrRef<'_>,
        parameter: StrRef<'_>,
    ) -> AnyResult,
    pub write_parameter: unsafe extern "C" fn(
        handle: ContextHandle,
        value: ConstNonNull<()>,
        r#type: ParameterType,
        module: StrRef<'_>,
        parameter: StrRef<'_>,
    ) -> AnyResult,
}

/// Definition of the module subsystem.
pub trait ModuleSubsystem: Copy {
    /// Returns the active profile of the module subsystem.
    fn profile(self) -> Profile;

    /// Returns the status of all features known to the subsystem.
    fn features<'this>(self) -> &'this [FeatureStatus]
    where
        Self: 'this;

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
    fn profile(self) -> Profile {
        unsafe {
            let ctx = self.view();
            (ctx.vtable.module_v0.profile)(ctx.handle)
        }
    }

    fn features<'this>(self) -> &'this [FeatureStatus]
    where
        Self: 'this,
    {
        unsafe {
            let mut out = MaybeUninit::uninit();
            let ctx = self.view();
            let len = (ctx.vtable.module_v0.features)(ctx.handle, &mut out);
            let ptr = out.assume_init();
            if len == 0 {
                &[]
            } else {
                let ptr = ptr.unwrap();
                std::slice::from_raw_parts(ptr.as_ptr(), len)
            }
        }
    }

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

/// Profile of the module subsystem.
///
/// Each profile enables a set of default features.
#[repr(i32)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Profile {
    #[cfg_attr(not(debug_assertions), default)]
    Release,
    #[cfg_attr(debug_assertions, default)]
    Dev,
}

/// Optional features recognized by the module subsystem.
///
/// Some features may be mutually exclusive.
#[repr(u16)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureTag {
    // Remove once the first feature is added.
    #[doc(hidden)]
    _Private,
}

/// Request flag for an optional feature.
#[repr(u16)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureRequestFlag {
    Required,
    On,
    Off,
}

/// Request for an optional feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
pub struct FeatureRequest {
    pub tag: FeatureTag,
    pub flag: FeatureRequestFlag,
}

/// Status flag of an optional feature.
#[repr(u16)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureStatusFlag {
    On,
    Off,
}

/// Status of an optional feature.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureStatus {
    pub tag: FeatureTag,
    pub flag: FeatureStatusFlag,
}

/// Configuration of the module subsystem.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Config<'a> {
    /// # Safety
    ///
    /// Must be [`TypeId::ModuleConfig`].
    pub id: Unsafe<TypeId>,
    pub next: Option<OpaqueHandle<dyn Send + Sync + 'a>>,
    pub profile: Profile,
    pub features: SliceRef<'a, FeatureRequest>,
    _private: PhantomData<()>,
}

impl<'a> Config<'a> {
    /// Creates the default config.
    pub const fn new() -> Self {
        unsafe {
            Self {
                id: Unsafe::new(TypeId::ModuleConfig),
                next: None,
                profile: if cfg!(debug_assertions) {
                    Profile::Dev
                } else {
                    Profile::Release
                },
                features: SliceRef::new(&[]),
                _private: PhantomData,
            }
        }
    }

    /// Sets a custom profile.
    pub const fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = profile;
        self
    }

    /// Sets a custom list of feature requests.
    pub const fn with_features(mut self, features: &'a [FeatureRequest]) -> Self {
        self.features = SliceRef::new(features);
        self
    }

    /// Returns a slice of all subscribers.
    pub const fn features(&self) -> &[FeatureRequest] {
        self.features.as_slice()
    }
}

impl Default for Config<'_> {
    fn default() -> Self {
        Self::new()
    }
}
