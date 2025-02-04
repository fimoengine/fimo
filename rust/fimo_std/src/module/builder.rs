//! Utilities for constructing an [`Export`](crate::module::exports::Export).

use core::panic;
use std::{
    ffi::CStr,
    fmt::{Debug, Display},
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
};

use crate::{
    ffi::View,
    module::{
        ParameterAccessGroup, ParameterCast, ParameterData,
        instance::{GenericInstance, UninitInstanceView},
        loading_set::LoadingSetView,
        symbols::SymbolInfo,
    },
};

/// Builder for an [`Export`](crate::module::exports::Export).
pub struct Builder<InstanceView, OwnedInstance>(PhantomData<fn(InstanceView, OwnedInstance)>)
where
    for<'a> Pin<&'a InstanceView>: GenericInstance + View,
    for<'a> &'a OwnedInstance: GenericInstance;

impl<InstanceView, OwnedInstance> Builder<InstanceView, OwnedInstance>
where
    for<'a> Pin<&'a InstanceView>: GenericInstance + View,
    for<'a> &'a OwnedInstance: GenericInstance,
{
    /// Initializes a new `Builder`.
    pub const fn new(_name: &CStr) -> Self {
        Self(PhantomData)
    }

    /// Adds a description to the module.
    pub const fn with_description(&mut self, _description: &CStr) -> &mut Self {
        self
    }

    /// Adds an author to the module.
    pub const fn with_author(&mut self, _author: &'static CStr) -> &mut Self {
        self
    }

    /// Adds a license to the module.
    pub const fn with_license(&mut self, _license: &'static CStr) -> &mut Self {
        self
    }

    /// Adds a new parameter to the module.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub const fn with_parameter<T: const ParameterCast>(
        &mut self,
        _table_name: &str,
        _name: &CStr,
        _default_value: T,
        _read_group: Option<ParameterAccessGroup>,
        _write_group: Option<ParameterAccessGroup>,
        _read: Option<fn(ParameterData<'_, T::Repr>) -> T::Repr>,
        _write: Option<fn(ParameterData<'_, T::Repr>, T::Repr)>,
    ) -> &mut Self {
        #[allow(clippy::mem_forget)]
        std::mem::forget(_default_value);
        self
    }

    /// Adds a resource path to the module.
    pub const fn with_resource(&mut self, _table_name: &str, _path: &CStr) -> &mut Self {
        self
    }

    /// Adds a namespace import to the module.
    ///
    /// A namespace may be imported multiple times.
    pub const fn with_namespace(&mut self, _name: &CStr) -> &mut Self {
        self
    }

    /// Adds an import to the module.
    ///
    /// Automatically imports the required namespace.
    pub const fn with_import<T: SymbolInfo>(&mut self, _table_name: &str) -> &mut Self {
        self
    }

    /// Adds a static export to the module.
    pub const fn with_export<T: SymbolInfo>(
        &mut self,
        _table_name: &str,
        _value: T::Type,
    ) -> &mut Self {
        self
    }

    /// Adds a static export to the module.
    #[allow(clippy::type_complexity)]
    pub const fn with_dynamic_export<T, E>(
        &mut self,
        _table_name: &str,
        _init: fn(Pin<&UninitInstanceView<'_, InstanceView>>) -> Result<T::Type, E>,
        _deinit: fn(T::Type),
    ) -> &mut Self
    where
        T: SymbolInfo,
        E: Debug + Display,
    {
        self
    }

    /// Adds a state to the module.
    #[allow(clippy::type_complexity)]
    pub const fn with_state<T, E>(
        &mut self,
        _init: fn(
            Pin<&UninitInstanceView<'_, InstanceView>>,
            LoadingSetView<'_>,
        ) -> Result<NonNull<T>, E>,
        _deinit: fn(Pin<&UninitInstanceView<'_, InstanceView>>, NonNull<T>),
    ) -> &mut Self
    where
        T: Send + Sync + 'static,
        E: Debug + Display,
    {
        self
    }

    pub const fn build(&mut self) {
        panic!("the builder must be consumed by the `#[export_module]` macro")
    }

    #[doc(hidden)]
    pub const fn __private_build(&mut self) -> __PrivateBuildToken {
        __PrivateBuildToken(())
    }
}

#[doc(hidden)]
pub struct __PrivateBuildToken(());
