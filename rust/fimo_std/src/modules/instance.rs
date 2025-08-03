//! Definition of module instances.

use crate::{
    context::{Error, Handle, Status},
    modules::{
        info::InfoView,
        parameters::{ParameterCast, ParameterRepr, ParameterType},
        symbols::{StrRef, SymbolInfo, SymbolPointer, SymbolRef},
    },
    utils::{ConstNonNull, View, Viewable},
    version::Version,
};
use std::{
    ffi::CStr,
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    ops::Deref,
    pin::Pin,
    ptr::NonNull,
};

use super::symbols::AssertSharable;

/// Information about an instance dependency.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyInfo {
    /// The instance does not own the dependency.
    Unowned,
    /// The instance acquired the dependency dynamically.
    Dynamic,
    /// The instance acquired the dependency statically.
    Static,
}

/// Shared API of all instances.
pub trait GenericInstance: Sized {
    /// Type of the parameter table.
    type Parameters: Send + Sync + 'static;

    /// Type of the resource table.
    type Resources: Send + Sync + 'static;

    /// Type of the imports table.
    type Imports: Send + Sync + 'static;

    /// Type of the exports table.
    type Exports: Send + Sync + 'static;

    /// Type of the instance state.
    type State: Send + Sync + 'static;

    /// Owned instance type.
    type Owned;

    /// Constructs an owned handle to the instance, possibly prolonging its lifetime.
    fn to_owned_instance(self) -> Self::Owned;

    /// Constructs a borrowed opaque instance handle from the typed handle.
    fn to_opaque_instance_view<'o>(self) -> Pin<&'o OpaqueInstanceView<'o>>
    where
        Self: 'o;

    /// Checks the status of a namespace from the view of the module.
    ///
    /// Checks if the module includes the namespace. In that case, the module is allowed access
    /// to the symbols in the namespace. Additionally, this function also queries whether the
    /// include is static, i.e., it was specified by the module at load time.
    fn query_namespace(self, namespace: &CStr) -> Result<DependencyInfo, Error> {
        let this = self.to_opaque_instance_view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.query_namespace;
            let mut has_dependency = MaybeUninit::uninit();
            let mut is_static = MaybeUninit::uninit();
            f(
                this,
                StrRef::new(namespace),
                &mut has_dependency,
                &mut is_static,
            )
            .into_result()?;

            let has_dependency = has_dependency.assume_init();
            if !has_dependency {
                return Ok(DependencyInfo::Unowned);
            }

            let is_static = is_static.assume_init();
            match is_static {
                false => Ok(DependencyInfo::Dynamic),
                true => Ok(DependencyInfo::Static),
            }
        }
    }

    /// Includes a namespace by the module.
    ///
    /// Once included, the module gains access to the symbols of its dependencies that are
    /// exposed in said namespace. A namespace can not be included multiple times.
    fn add_namespace(self, namespace: &CStr) -> Result<(), Error> {
        let this = self.to_opaque_instance_view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.add_namespace;
            f(this, StrRef::new(namespace)).into_result()
        }
    }

    /// Removes a namespace include from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in
    /// said namespace. It is only possible to exclude namespaces that were manually added,
    /// whereas static namespace includes remain valid until the module is unloaded.
    fn remove_namespace(self, namespace: &CStr) -> Result<(), Error> {
        let this = self.to_opaque_instance_view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.remove_namespace;
            f(this, StrRef::new(namespace)).into_result()
        }
    }

    /// Checks if a module depends on another module.
    ///
    /// Checks if the specified module is a dependency of the current instance. In that case
    /// the instance is allowed to access the symbols exported by the module. Additionally,
    /// this function also queries whether the dependency is static, i.e., the dependency was
    /// specified by the module at load time.
    fn query_dependency<'i>(
        self,
        info: impl Viewable<Pin<&'i InfoView<'i>>>,
    ) -> Result<DependencyInfo, Error> {
        let this = self.to_opaque_instance_view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.query_dependency;
            let mut has_dependency = MaybeUninit::uninit();
            let mut is_static = MaybeUninit::uninit();
            f(this, info.view(), &mut has_dependency, &mut is_static).into_result()?;

            let has_dependency = has_dependency.assume_init();
            if !has_dependency {
                return Ok(DependencyInfo::Unowned);
            }

            let is_static = is_static.assume_init();
            match is_static {
                false => Ok(DependencyInfo::Dynamic),
                true => Ok(DependencyInfo::Static),
            }
        }
    }

    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module is allowed access to the symbols
    /// and protected parameters of said dependency. Trying to acquire a dependency to a module
    /// that is already a dependency, or to a module that would result in a circular dependency
    /// will result in an error.
    fn add_dependency<'i>(self, info: impl Viewable<Pin<&'i InfoView<'i>>>) -> Result<(), Error> {
        let this = self.to_opaque_instance_view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.add_dependency;
            f(this, info.view()).into_result()
        }
    }

    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller ensures that it does not own any
    /// references to resources originating from the former dependency, and allows for the
    /// unloading of the module. A module can only relinquish dependencies to modules that were
    /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
    fn remove_dependency<'i>(
        self,
        info: impl Viewable<Pin<&'i InfoView<'i>>>,
    ) -> Result<(), Error> {
        let this = self.to_opaque_instance_view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.remove_dependency;
            f(this, info.view()).into_result()
        }
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until the module relinquishes the
    /// dependency to the module that exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    fn load_symbol_raw(
        self,
        name: &CStr,
        namespace: &CStr,
        version: Version<'_>,
    ) -> Result<ConstNonNull<()>, Error> {
        let this = self.to_opaque_instance_view();
        let name = StrRef::new(name);
        let namespace = StrRef::new(namespace);
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.load_symbol;
            let mut out = MaybeUninit::uninit();
            f(this, name, namespace, version, &mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until the module relinquishes the
    /// dependency to the module that exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    fn load_symbol<'a, T>(self) -> Result<SymbolRef<'a, T>, Error>
    where
        Self: 'a,
        T: SymbolInfo,
        T::Type: const SymbolPointer,
    {
        let sym = self.load_symbol_raw(T::NAME, T::NAMESPACE, T::VERSION)?;
        unsafe { Ok(SymbolRef::from_opaque_ptr(sym)) }
    }

    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read access. The operation fails,
    /// if the parameter does not exist, or if the parameter does not allow reading with a
    /// dependency access.
    fn read_parameter<U: ParameterCast>(self, module: &CStr, parameter: &CStr) -> Result<U, Error> {
        let this = self.to_opaque_instance_view();
        let module = StrRef::new(module);
        let parameter = StrRef::new(parameter);
        let r#type = <U::Repr as ParameterRepr>::TYPE;
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.read_parameter;
            let mut out = MaybeUninit::<U::Repr>::uninit();
            f(
                this,
                NonNull::new_unchecked(&raw mut out).cast(),
                r#type,
                module,
                parameter,
            )
            .into_result()?;

            let out = out.assume_init();
            Ok(U::from_repr(out))
        }
    }

    /// Sets a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write access. The operation fails,
    /// if the parameter does not exist, or if the parameter does not allow writing with a
    /// dependency access.
    fn write_parameter<U: ParameterCast>(
        self,
        value: U,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<(), Error> {
        let this = self.to_opaque_instance_view();
        let module = StrRef::new(module);
        let parameter = StrRef::new(parameter);
        let value = value.into_repr();
        let r#type = <U::Repr as ParameterRepr>::TYPE;
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.write_parameter;
            f(
                this,
                ConstNonNull::new_unchecked(&raw const value).cast(),
                r#type,
                module,
                parameter,
            )
            .into_result()
        }
    }
}

/// Virtual function table of an [`InstanceView`] and [`Instance`].
#[repr(C)]
#[derive(Debug)]
pub struct InstanceVTable {
    pub acquire: unsafe extern "C" fn(handle: Pin<&OpaqueInstanceView<'_>>),
    pub release: unsafe extern "C" fn(handle: Pin<&OpaqueInstanceView<'_>>),
    pub query_namespace: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        namespace: StrRef<'_>,
        has_dependency: &mut MaybeUninit<bool>,
        is_static: &mut MaybeUninit<bool>,
    ) -> Status,
    pub add_namespace:
        unsafe extern "C" fn(handle: Pin<&OpaqueInstanceView<'_>>, namespace: StrRef<'_>) -> Status,
    pub remove_namespace:
        unsafe extern "C" fn(handle: Pin<&OpaqueInstanceView<'_>>, namespace: StrRef<'_>) -> Status,
    pub query_dependency: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        info: Pin<&InfoView<'_>>,
        has_dependency: &mut MaybeUninit<bool>,
        is_static: &mut MaybeUninit<bool>,
    ) -> Status,
    pub add_dependency: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        info: Pin<&InfoView<'_>>,
    ) -> Status,
    pub remove_dependency: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        info: Pin<&InfoView<'_>>,
    ) -> Status,
    pub load_symbol: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        name: StrRef<'_>,
        namespace: StrRef<'_>,
        version: Version<'_>,
        out: &mut MaybeUninit<ConstNonNull<()>>,
    ) -> Status,
    pub read_parameter: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        value: NonNull<()>,
        r#type: ParameterType,
        module: StrRef<'_>,
        parameter: StrRef<'_>,
    ) -> Status,
    pub write_parameter: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        value: ConstNonNull<()>,
        r#type: ParameterType,
        module: StrRef<'_>,
        parameter: StrRef<'_>,
    ) -> Status,
    _private: PhantomData<()>,
}

/// Borrowed handle of a loaded module.
///
/// A module is self-contained, and may not be passed to other modules. An instance is valid for
/// as long as the owning module remains loaded. Modules must not leak any resources outside its
/// own module, ensuring that they are destroyed upon module unloading.
#[repr(C)]
#[derive(Debug)]
pub struct InstanceView<'a, P, R, I, E, S>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    pub vtable: &'a AssertSharable<InstanceVTable>,
    pub parameters: Option<&'a P>,
    pub resources: Option<&'a R>,
    pub imports: Option<&'a I>,
    pub exports: Option<&'a E>,
    pub info: Pin<&'a InfoView<'a>>,
    pub handle: &'a Handle,
    pub state: Option<Pin<&'a S>>,
    _pinned: PhantomData<PhantomPinned>,
    _private: PhantomData<&'a ()>,
}

impl<'a, P, R, I, E, S> InstanceView<'a, P, R, I, E, S>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    cfg_internal! {
        /// Constructs a new `InstanceView`.
        ///
        /// # Safety
        ///
        /// Is only safely constructible by the implementation.
        ///
        /// # Unstable
        ///
        /// **Note**: This is an [unstable API][unstable]. The public API of this type may break
        /// with any semver compatible release. See
        /// [the documentation on unstable features][unstable] for details.
        ///
        /// [unstable]: crate#unstable-features
        #[allow(clippy::too_many_arguments)]
        pub const unsafe fn new_in(
            out: Pin<&mut MaybeUninit<Self>>,
            vtable: &'a AssertSharable<InstanceVTable>,
            parameters: Option<&'a P>,
            resources: Option<&'a R>,
            imports: Option<&'a I>,
            exports: Option<&'a E>,
            info: Pin<&'a InfoView<'a>>,
            handle: &'a Handle,
            state: Option<Pin<&'a S>>,
        ) {
            let this = Self {
                vtable,
                parameters,
                resources,
                imports,
                exports,
                info,
                handle,
                state,
                _pinned: PhantomData,
                _private: PhantomData,
            };
            unsafe {
                let inner = Pin::get_unchecked_mut(out);
                inner.write(this);
            }
        }
    }

    /// Returns a reference to the parameter table of the instance.
    // noinspection RsReplaceMatchExpr
    #[inline(always)]
    pub const fn parameters(self: Pin<&Self>) -> &P {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            if const { size_of::<P>() != 0 } {
                this.parameters.unwrap_unchecked()
            } else {
                match this.parameters {
                    None => NonNull::dangling().as_ref(),
                    Some(x) => x,
                }
            }
        }
    }

    /// Returns a reference to the resource table of the instance.
    // noinspection RsReplaceMatchExpr
    #[inline(always)]
    pub const fn resources(self: Pin<&Self>) -> &R {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            if const { size_of::<R>() != 0 } {
                this.resources.unwrap_unchecked()
            } else {
                match this.resources {
                    None => NonNull::dangling().as_ref(),
                    Some(x) => x,
                }
            }
        }
    }

    /// Returns a reference to the imports table of the instance.
    // noinspection RsReplaceMatchExpr
    #[inline(always)]
    pub const fn imports(self: Pin<&Self>) -> &I {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            if const { size_of::<I>() != 0 } {
                this.imports.unwrap_unchecked()
            } else {
                match this.imports {
                    None => NonNull::dangling().as_ref(),
                    Some(x) => x,
                }
            }
        }
    }

    /// Returns a reference to the exports table of the instance.
    // noinspection RsReplaceMatchExpr
    #[inline(always)]
    pub const fn exports(self: Pin<&Self>) -> &E {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            if const { size_of::<E>() != 0 } {
                this.exports.unwrap_unchecked()
            } else {
                match this.exports {
                    None => NonNull::dangling().as_ref(),
                    Some(x) => x,
                }
            }
        }
    }

    /// Returns a reference to the instance info.
    #[inline(always)]
    pub const fn info(self: Pin<&Self>) -> Pin<&InfoView<'_>> {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            this.info
        }
    }

    /// Returns a view to the context.
    #[inline(always)]
    pub const fn handle(self: Pin<&Self>) -> &'a Handle {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            this.handle
        }
    }

    /// Returns a reference to the state of the instance.
    // noinspection RsReplaceMatchExpr
    #[inline(always)]
    pub const fn state(self: Pin<&Self>) -> Pin<&S> {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            if const { size_of::<S>() != 0 } {
                this.state.unwrap_unchecked()
            } else {
                match this.state {
                    None => Pin::new_unchecked(NonNull::dangling().as_ref()),
                    Some(x) => x,
                }
            }
        }
    }
}

impl<P, R, I, E, S> GenericInstance for Pin<&'_ InstanceView<'_, P, R, I, E, S>>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    type Parameters = P;
    type Resources = R;
    type Imports = I;
    type Exports = E;
    type State = S;

    type Owned = Instance<P, R, I, E, S>;

    #[inline(always)]
    fn to_owned_instance(self) -> Self::Owned {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let f = inner.vtable.acquire;
            f(self.to_opaque_instance_view());
            Instance(std::mem::transmute::<
                Self,
                Pin<&'_ InstanceView<'_, P, R, I, E, S>>,
            >(self))
        }
    }

    #[inline(always)]
    fn to_opaque_instance_view<'o>(self) -> Pin<&'o OpaqueInstanceView<'o>>
    where
        Self: 'o,
    {
        unsafe { std::mem::transmute(self) }
    }
}

impl<P, R, I, E, S> View for Pin<&InstanceView<'_, P, R, I, E, S>>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
}

/// A view to an unknown instance.
#[repr(transparent)]
#[derive(Debug)]
pub struct OpaqueInstanceView<'a>(pub InstanceView<'a, (), (), (), (), ()>);

impl OpaqueInstanceView<'_> {
    /// Returns a reference to the parameter table of the instance.
    #[inline(always)]
    pub const fn parameters(self: Pin<&Self>) -> &() {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let inner = Pin::new_unchecked(&inner.0);
            inner.parameters()
        }
    }

    /// Returns a reference to the resource table of the instance.
    #[inline(always)]
    pub const fn resources(self: Pin<&Self>) -> &() {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let inner = Pin::new_unchecked(&inner.0);
            inner.resources()
        }
    }

    /// Returns a reference to the imports table of the instance.
    #[inline(always)]
    pub const fn imports(self: Pin<&Self>) -> &() {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let inner = Pin::new_unchecked(&inner.0);
            inner.imports()
        }
    }

    /// Returns a reference to the exports table of the instance.
    #[inline(always)]
    pub const fn exports(self: Pin<&Self>) -> &() {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let inner = Pin::new_unchecked(&inner.0);
            inner.exports()
        }
    }

    /// Returns a reference to the instance info.
    #[inline(always)]
    pub const fn info(self: Pin<&Self>) -> Pin<&InfoView<'_>> {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let inner = Pin::new_unchecked(&inner.0);
            inner.info()
        }
    }

    /// Returns a view to the context.
    #[inline(always)]
    pub const fn handle(self: Pin<&Self>) -> &'_ Handle {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let inner = Pin::new_unchecked(&inner.0);
            inner.handle()
        }
    }

    /// Returns a reference to the state of the instance.
    #[inline(always)]
    pub const fn state(self: Pin<&Self>) -> Pin<&()> {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let inner = Pin::new_unchecked(&inner.0);
            inner.state()
        }
    }
}

impl GenericInstance for Pin<&'_ OpaqueInstanceView<'_>> {
    type Parameters = ();
    type Resources = ();
    type Imports = ();
    type Exports = ();
    type State = ();

    type Owned = Instance<(), (), (), (), ()>;

    #[inline(always)]
    fn to_owned_instance(self) -> Self::Owned {
        let view = unsafe {
            std::mem::transmute::<Self, Pin<&InstanceView<'_, (), (), (), (), ()>>>(self)
        };
        view.to_owned_instance()
    }

    #[inline(always)]
    fn to_opaque_instance_view<'o>(self) -> Pin<&'o OpaqueInstanceView<'o>>
    where
        Self: 'o,
    {
        self
    }
}

impl View for Pin<&OpaqueInstanceView<'_>> {}

impl<'a> Deref for OpaqueInstanceView<'a> {
    type Target = InstanceView<'a, (), (), (), (), ()>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Owned handle of a loaded module.
///
/// A module is self-contained, and may not be passed to other modules. An instance is valid for
/// as long as the owning module remains loaded. Modules must not leak any resources outside its
/// own module, ensuring that they are destroyed upon module unloading.
#[derive(Debug)]
#[repr(transparent)]
pub struct Instance<P, R, I, E, S>(Pin<&'static InstanceView<'static, P, R, I, E, S>>)
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static;

impl<P, R, I, E, S> Instance<P, R, I, E, S>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    /// Returns a reference to the parameter table of the instance.
    #[inline(always)]
    pub const fn parameters(&self) -> &P {
        self.0.parameters()
    }

    /// Returns a reference to the resource table of the instance.
    #[inline(always)]
    pub const fn resources(&self) -> &R {
        self.0.resources()
    }

    /// Returns a reference to the imports table of the instance.
    #[inline(always)]
    pub const fn imports(&self) -> &I {
        self.0.imports()
    }

    /// Returns a reference to the exports table of the instance.
    #[inline(always)]
    pub const fn exports(&self) -> &E {
        self.0.exports()
    }

    /// Returns a reference to the instance info.
    #[inline(always)]
    pub const fn info(&self) -> Pin<&InfoView<'_>> {
        self.0.info()
    }

    /// Returns a view to the context.
    #[inline(always)]
    pub const fn handle(&self) -> &'_ Handle {
        self.0.handle()
    }

    /// Returns a reference to the state of the instance.
    #[inline(always)]
    pub const fn state(&self) -> Pin<&S> {
        self.0.state()
    }
}

impl<P, R, I, E, S> GenericInstance for &'_ Instance<P, R, I, E, S>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    type Parameters = P;
    type Resources = R;
    type Imports = I;
    type Exports = E;
    type State = S;

    type Owned = Instance<P, R, I, E, S>;

    #[inline(always)]
    fn to_owned_instance(self) -> Self::Owned {
        self.view().to_owned_instance()
    }

    #[inline(always)]
    fn to_opaque_instance_view<'o>(self) -> Pin<&'o OpaqueInstanceView<'o>>
    where
        Self: 'o,
    {
        self.view().to_opaque_instance_view()
    }
}

impl<'a, P, R, I, E, S> Viewable<Pin<&'a InstanceView<'a, P, R, I, E, S>>>
    for &'a Instance<P, R, I, E, S>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    #[inline(always)]
    fn view(self) -> Pin<&'a InstanceView<'a, P, R, I, E, S>> {
        self.0
    }
}

impl<P, R, I, E, S> Clone for Instance<P, R, I, E, S>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        self.to_owned_instance()
    }
}

impl<P, R, I, E, S> Drop for Instance<P, R, I, E, S>
where
    P: Send + Sync + 'static,
    R: Send + Sync + 'static,
    I: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Send + Sync + 'static,
{
    fn drop(&mut self) {
        let view = self.to_opaque_instance_view();
        unsafe {
            let inner = Pin::into_inner_unchecked(view);
            let f = inner.vtable.release;
            f(view);
        }
    }
}

/// A module instance that can be created to gain access to the module subsystem.
#[derive(Debug)]
#[repr(transparent)]
pub struct RootInstance(Pin<&'static OpaqueInstanceView<'static>>);

impl RootInstance {
    /// Constructs a new `RootInstance`.
    ///
    /// The functions of the module subsystem require that the caller owns a reference to their own
    /// module. This is a problem, as the constructor of the context won't be assigned a module
    /// instance during bootstrapping. As a workaround, we allow for the creation of root instances,
    /// i.e., module handles without an associated module.
    pub fn new() -> Result<Self, Error> {
        unsafe {
            let mut out = MaybeUninit::uninit();
            let handle = Handle::get_handle();
            let f = handle.modules_v0.new_root_instance;
            f(&mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Returns a reference to the parameter table of the instance.
    #[inline(always)]
    pub const fn parameters(&self) -> &() {
        self.0.parameters()
    }

    /// Returns a reference to the resource table of the instance.
    #[inline(always)]
    pub const fn resources(&self) -> &() {
        self.0.resources()
    }

    /// Returns a reference to the imports table of the instance.
    #[inline(always)]
    pub const fn imports(&self) -> &() {
        self.0.imports()
    }

    /// Returns a reference to the exports table of the instance.
    #[inline(always)]
    pub const fn exports(&self) -> &() {
        self.0.exports()
    }

    /// Returns a reference to the instance info.
    #[inline(always)]
    pub const fn info(&self) -> Pin<&InfoView<'_>> {
        self.0.info()
    }

    /// Returns a view to the context.
    #[inline(always)]
    pub const fn handle(&self) -> &'_ Handle {
        self.0.handle()
    }

    /// Returns a reference to the state of the instance.
    #[inline(always)]
    pub const fn state(&self) -> Pin<&()> {
        self.0.state()
    }
}

impl GenericInstance for &'_ RootInstance {
    type Parameters = ();

    type Resources = ();

    type Imports = ();

    type Exports = ();

    type State = ();

    type Owned = Instance<(), (), (), (), ()>;

    #[inline(always)]
    fn to_owned_instance(self) -> Self::Owned {
        self.0.to_owned_instance()
    }

    #[inline(always)]
    fn to_opaque_instance_view<'o>(self) -> Pin<&'o OpaqueInstanceView<'o>>
    where
        Self: 'o,
    {
        self.0
    }
}

impl<'a> Viewable<Pin<&'a OpaqueInstanceView<'a>>> for &'a RootInstance {
    #[inline(always)]
    fn view(self) -> Pin<&'a OpaqueInstanceView<'a>> {
        self.0
    }
}

impl Drop for RootInstance {
    fn drop(&mut self) {
        let info = self.info();
        info.mark_unloadable();
    }
}

/// A view to an instance that does not have its state or exports initialized.
pub type Stage0InstanceView<'a, T> = InstanceView<
    'a,
    <Pin<&'a T> as GenericInstance>::Parameters,
    <Pin<&'a T> as GenericInstance>::Resources,
    <Pin<&'a T> as GenericInstance>::Imports,
    MaybeUninit<<Pin<&'a T> as GenericInstance>::Exports>,
    MaybeUninit<<Pin<&'a T> as GenericInstance>::State>,
>;

/// A view to an instance that does not have its exports initialized.
pub type Stage1InstanceView<'a, T> = InstanceView<
    'a,
    <Pin<&'a T> as GenericInstance>::Parameters,
    <Pin<&'a T> as GenericInstance>::Resources,
    <Pin<&'a T> as GenericInstance>::Imports,
    MaybeUninit<<Pin<&'a T> as GenericInstance>::Exports>,
    <Pin<&'a T> as GenericInstance>::State,
>;

/// Defines two new instance newtypes, one for borrowed instances and one for owned instances.
///
/// # Examples
///
/// ```
/// use fimo_std::instance;
///
/// #[repr(C)]
/// #[derive(Debug)]
/// pub struct Imports {
///     // list of imports
///     // ...
/// }
///
/// #[repr(C)]
/// #[derive(Debug)]
/// pub struct Exports {
///     // list of exports
///     // ...
/// }
///
/// // Creates the structs `MyBorrowed` and `MyOwned`.
/// instance! {
///     pub type MyBorrowed;
///     pub type MyOwned;
///     with
///         Parameters = (),
///         Resources = (),
///         Imports = Imports,
///         Exports = Exports,
///         State = i32,
/// }
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! instance {
    (
        $view_vis:vis type $view:ident;
        $owned_vis:vis type $owned:ident;
        with
            Parameters = $p:ty,
            Resources = $r:ty,
            Imports = $i:ty,
            Exports = $e:ty,
            State = $s:ty,
    ) => {
        /// Borrowed handle to an instance.
        #[derive(Debug)]
        #[repr(transparent)]
        $view_vis struct $view<'a>($crate::modules::instance::InstanceView<'a, $p, $r, $i, $e, $s>);

        #[allow(unused)]
        impl $view<'_> {
            #[inline(always)]
            const fn view_inner(self: core::pin::Pin<&Self>)
                -> core::pin::Pin<&$crate::modules::instance::InstanceView<'_, $p, $r, $i, $e, $s>> {
                unsafe { std::mem::transmute(self) }
            }

            /// Returns a reference to the parameter table of the instance.
            #[inline(always)]
            pub const fn parameters(self: core::pin::Pin<&Self>) -> &$p {
                self.view_inner().parameters()
            }

            /// Returns a reference to the resource table of the instance.
            #[inline(always)]
            pub const fn resources(self: core::pin::Pin<&Self>) -> &$r {
                self.view_inner().resources()
            }

            /// Returns a reference to the imports table of the instance.
            #[inline(always)]
            pub const fn imports(self: core::pin::Pin<&Self>) -> &$i {
                self.view_inner().imports()
            }

            /// Returns a reference to the exports table of the instance.
            #[inline(always)]
            pub const fn exports(self: core::pin::Pin<&Self>) -> &$e {
                self.view_inner().exports()
            }

            /// Returns a reference to the instance info.
            #[inline(always)]
            pub const fn info(self: core::pin::Pin<&Self>)
                -> core::pin::Pin<&$crate::modules::info::InfoView<'_>> {
                self.view_inner().info()
            }

            /// Returns a view to the context.
            #[inline(always)]
            pub const fn handle(self: core::pin::Pin<&Self>) -> &'_ $crate::context::Handle {
                self.view_inner().handle()
            }

            /// Returns a reference to the state of the instance.
            #[inline(always)]
            pub const fn state(self: core::pin::Pin<&Self>) -> core::pin::Pin<&$s> {
                self.view_inner().state()
            }
        }

        impl $crate::modules::instance::GenericInstance for core::pin::Pin<&'_ $view<'_>> {
            type Parameters = $p;
            type Resources = $r;
            type Imports = $i;
            type Exports = $e;
            type State = $s;
            type Owned = $owned;

            #[inline(always)]
            fn to_owned_instance(self) -> Self::Owned {
                $owned(self.view_inner().to_owned_instance())
            }

            #[inline(always)]
            fn to_opaque_instance_view<'o>(self) -> core::pin::Pin<&'o $crate::modules::instance::OpaqueInstanceView<'o>>
            where
                Self: 'o,
            {
                self.view_inner().to_opaque_instance_view()
            }
        }

        impl $crate::utils::View for core::pin::Pin<&'_ $view<'_>> {}

        /// Owned handle to an instance.
        #[repr(transparent)]
        #[derive(Debug, Clone)]
        $owned_vis struct $owned($crate::modules::instance::Instance<$p, $r, $i, $e, $s>);

        #[allow(unused)]
        impl $owned {
            /// Returns a reference to the parameter table of the instance.
            #[inline(always)]
            pub const fn parameters(&self) -> &$p {
                self.0.parameters()
            }

            /// Returns a reference to the resource table of the instance.
            #[inline(always)]
            pub const fn resources(&self) -> &$r {
                self.0.resources()
            }

            /// Returns a reference to the imports table of the instance.
            #[inline(always)]
            pub const fn imports(&self) -> &$i {
                self.0.imports()
            }

            /// Returns a reference to the exports table of the instance.
            #[inline(always)]
            pub const fn exports(&self) -> &$e {
                self.0.exports()
            }

            /// Returns a reference to the instance info.
            #[inline(always)]
            pub const fn info(&self) -> core::pin::Pin<&$crate::modules::info::InfoView<'_>> {
                self.0.info()
            }

            /// Returns a view to the context.
            #[inline(always)]
            pub const fn handle(&self) -> &'_ $crate::context::Handle {
                self.0.handle()
            }

            /// Returns a reference to the state of the instance.
            #[inline(always)]
            pub const fn state(&self) -> core::pin::Pin<&$s> {
                self.0.state()
            }
        }

        impl $crate::modules::instance::GenericInstance for &'_ $owned {
            type Parameters = $p;
            type Resources = $r;
            type Imports = $i;
            type Exports = $e;
            type State = $s;
            type Owned = $owned;

            #[inline(always)]
            fn to_owned_instance(self) -> Self::Owned {
                $owned(self.0.to_owned_instance())
            }

            #[inline(always)]
            fn to_opaque_instance_view<'o>(self) -> core::pin::Pin<&'o $crate::modules::instance::OpaqueInstanceView<'o>>
            where
                Self: 'o,
            {
                self.0.to_opaque_instance_view()
            }
        }

        impl<'a> $crate::utils::Viewable<core::pin::Pin<&'a $view<'a>>> for &'a $owned {
            #[inline(always)]
            fn view(self) -> core::pin::Pin<&'a $view<'a>> {
                unsafe { std::mem::transmute(self) }
            }
        }
    };
}
