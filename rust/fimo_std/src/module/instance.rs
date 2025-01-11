use crate::{
    error::{AnyError, AnyResult},
    ffi::{ConstCStr, ConstNonNull, VTablePtr, View, Viewable},
    module::{InfoView, ParameterCast, ParameterRepr, ParameterType},
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
pub trait GenericInstance<
    'a,
    P: Send + Sync + 'a,
    R: Send + Sync + 'a,
    I: Send + Sync + 'a,
    E: Send + Sync + 'a,
    S: Send + Sync + 'a,
>: Viewable<Pin<&'a InstanceView<'a, P, R, I, E, S>>>
{
    /// Owned instance type.
    type Owned<'o>
    where
        P: 'o,
        R: 'o,
        I: 'o,
        E: 'o,
        S: 'o;

    /// Constructs an owned handle to the instance, possibly prolonging its lifetime.
    fn to_owned_instance<'o>(self) -> Self::Owned<'o>
    where
        P: 'o,
        R: 'o,
        I: 'o,
        E: 'o,
        S: 'o;

    /// Checks the status of a namespace from the view of the module.
    ///
    /// Checks if the module includes the namespace. In that case, the module is allowed access
    /// to the symbols in the namespace. Additionally, this function also queries whether the
    /// include is static, i.e., it was specified by the module at load time.
    fn query_namespace(self, namespace: &CStr) -> Result<DependencyInfo, AnyError> {
        let this = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.query_namespace;
            let mut has_dependency = MaybeUninit::uninit();
            let mut is_static = MaybeUninit::uninit();
            f(
                this.view(),
                ConstCStr::new(namespace),
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
    fn add_namespace(self, namespace: &CStr) -> Result<(), AnyError> {
        let this = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.add_namespace;
            f(this.view(), ConstCStr::new(namespace)).into_result()
        }
    }

    /// Removes a namespace include from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in
    /// said namespace. It is only possible to exclude namespaces that were manually added,
    /// whereas static namespace includes remain valid until the module is unloaded.
    fn remove_namespace(self, namespace: &CStr) -> Result<(), AnyError> {
        let this = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.remove_namespace;
            f(this.view(), ConstCStr::new(namespace)).into_result()
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
    ) -> Result<DependencyInfo, AnyError> {
        let this = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.query_dependency;
            let mut has_dependency = MaybeUninit::uninit();
            let mut is_static = MaybeUninit::uninit();
            f(
                this.view(),
                info.view(),
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

    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module is allowed access to the symbols
    /// and protected parameters of said dependency. Trying to acquire a dependency to a module
    /// that is already a dependency, or to a module that would result in a circular dependency
    /// will result in an error.
    fn add_dependency<'i>(
        self,
        info: impl Viewable<Pin<&'i InfoView<'i>>>,
    ) -> Result<(), AnyError> {
        let this = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.add_dependency;
            f(this.view(), info.view()).into_result()
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
    ) -> Result<(), AnyError> {
        let this = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.remove_dependency;
            f(this.view(), info.view()).into_result()
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
        version: Version,
    ) -> Result<ConstNonNull<()>, AnyError> {
        let this = self.view();
        let name = ConstCStr::new(name);
        let namespace = ConstCStr::new(namespace);
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.load_symbol;
            let mut out = MaybeUninit::uninit();
            f(this.view(), name, namespace, version, &mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read access. The operation fails,
    /// if the parameter does not exist, or if the parameter does not allow reading with a
    /// dependency access.
    fn read_parameter<U: ParameterCast>(
        self,
        module: &CStr,
        parameter: &CStr,
    ) -> Result<U, AnyError> {
        let this = self.view();
        let module = ConstCStr::new(module);
        let parameter = ConstCStr::new(parameter);
        let r#type = <U::Repr as ParameterRepr>::TYPE;
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.read_parameter;
            let mut out = MaybeUninit::<U::Repr>::uninit();
            f(
                this.view(),
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
    ) -> Result<(), AnyError> {
        let this = self.view();
        let module = ConstCStr::new(module);
        let parameter = ConstCStr::new(parameter);
        let value = value.into_repr();
        let r#type = <U::Repr as ParameterRepr>::TYPE;
        unsafe {
            let inner = Pin::into_inner_unchecked(this);
            let f = inner.vtable.write_parameter;
            f(
                this.view(),
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
        namespace: ConstCStr,
        has_dependency: &mut MaybeUninit<bool>,
        is_static: &mut MaybeUninit<bool>,
    ) -> AnyResult,
    pub add_namespace: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        namespace: ConstCStr,
    ) -> AnyResult,
    pub remove_namespace: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        namespace: ConstCStr,
    ) -> AnyResult,
    pub query_dependency: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        info: Pin<&InfoView<'_>>,
        has_dependency: &mut MaybeUninit<bool>,
        is_static: &mut MaybeUninit<bool>,
    ) -> AnyResult,
    pub add_dependency: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        info: Pin<&InfoView<'_>>,
    ) -> AnyResult,
    pub remove_dependency: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        info: Pin<&InfoView<'_>>,
    ) -> AnyResult,
    pub load_symbol: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        name: ConstCStr,
        namespace: ConstCStr,
        version: Version,
        out: &mut MaybeUninit<ConstNonNull<()>>,
    ) -> AnyResult,
    pub read_parameter: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        value: NonNull<()>,
        r#type: ParameterType,
        module: ConstCStr,
        parameter: ConstCStr,
    ) -> AnyResult,
    pub write_parameter: unsafe extern "C" fn(
        handle: Pin<&OpaqueInstanceView<'_>>,
        value: ConstNonNull<()>,
        r#type: ParameterType,
        module: ConstCStr,
        parameter: ConstCStr,
    ) -> AnyResult,
    pub(crate) _private: PhantomData<()>,
}

/// Borrowed handle of a loaded module.
///
/// A module is self-contained, and may not be passed to other modules. An instance is valid for
/// as long as the owning module remains loaded. Modules must not leak any resources outside its
/// own module, ensuring that they are destroyed upon module unloading.
#[repr(C)]
#[derive(Debug)]
pub struct InstanceView<
    'a,
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
> {
    pub vtable: VTablePtr<'a, InstanceVTable>,
    pub parameters: Option<&'a P>,
    pub resources: Option<&'a R>,
    pub imports: Option<&'a I>,
    pub exports: Option<&'a E>,
    pub info: Pin<&'a InfoView<'a>>,
    pub state: Option<Pin<&'a S>>,
    pub(crate) _pinned: PhantomData<PhantomPinned>,
    pub(crate) _private: PhantomData<&'a ()>,
}

impl<'a, P, R, I, E, S> InstanceView<'a, P, R, I, E, S>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
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
            vtable: &'a InstanceVTable,
            parameters: Option<&'a P>,
            resources: Option<&'a R>,
            imports: Option<&'a I>,
            exports: Option<&'a E>,
            info: Pin<&'a InfoView<'a>>,
            state: Option<Pin<&'a S>>,
        ) {
            let this = Self {
                vtable: VTablePtr::new(vtable),
                parameters,
                resources,
                imports,
                exports,
                info,
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
    pub const fn info(self: Pin<&Self>) -> Pin<&InfoView<'_>> {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            this.info
        }
    }

    /// Returns a reference to the state of the instance.
    // noinspection RsReplaceMatchExpr
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

impl<'a, P, R, I, E, S> GenericInstance<'a, P, R, I, E, S>
    for Pin<&'a InstanceView<'a, P, R, I, E, S>>
where
    P: Send + Sync + 'a,
    R: Send + Sync + 'a,
    I: Send + Sync + 'a,
    E: Send + Sync + 'a,
    S: Send + Sync + 'a,
{
    type Owned<'o>
        = Instance<'o, P, R, I, E, S>
    where
        P: 'o,
        R: 'o,
        I: 'o,
        E: 'o,
        S: 'o;

    #[inline(always)]
    fn to_owned_instance<'o>(self) -> Self::Owned<'o>
    where
        P: 'o,
        R: 'o,
        I: 'o,
        E: 'o,
        S: 'o,
    {
        unsafe {
            let inner = Pin::into_inner_unchecked(self);
            let f = inner.vtable.acquire;
            f(self.view());
            Instance(std::mem::transmute::<
                Self,
                Pin<&'_ InstanceView<'_, P, R, I, E, S>>,
            >(self))
        }
    }
}

impl<P, R, I, E, S> View for Pin<&InstanceView<'_, P, R, I, E, S>>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
{
}

impl<'a, P, R, I, E, S> Viewable<Pin<&'a OpaqueInstanceView<'a>>>
    for Pin<&'a InstanceView<'a, P, R, I, E, S>>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
{
    #[inline(always)]
    fn view(self) -> Pin<&'a OpaqueInstanceView<'a>> {
        unsafe { std::mem::transmute(self) }
    }
}

/// A view to an unknown instance.
#[repr(transparent)]
#[derive(Debug)]
pub struct OpaqueInstanceView<'a>(pub InstanceView<'a, (), (), (), (), ()>);

impl<'a> GenericInstance<'a, (), (), (), (), ()> for Pin<&'a OpaqueInstanceView<'a>> {
    type Owned<'o> = Instance<'o, (), (), (), (), ()>;

    fn to_owned_instance<'o>(self) -> Self::Owned<'o> {
        let view: Pin<&InstanceView<'_, (), (), (), (), ()>> = self.view();
        view.to_owned_instance()
    }
}

impl View for Pin<&OpaqueInstanceView<'_>> {}

impl<'a> Viewable<Pin<&'a InstanceView<'a, (), (), (), (), ()>>>
    for Pin<&'a OpaqueInstanceView<'a>>
{
    #[inline(always)]
    fn view(self) -> Pin<&'a InstanceView<'a, (), (), (), (), ()>> {
        unsafe { std::mem::transmute(self) }
    }
}

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
pub struct Instance<'a, P, R, I, E, S>(Pin<&'a InstanceView<'a, P, R, I, E, S>>)
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync;

impl<P, R, I, E, S> Instance<'_, P, R, I, E, S>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
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

    /// Returns a reference to the state of the instance.
    #[inline(always)]
    pub const fn state(&self) -> Pin<&S> {
        self.0.state()
    }
}

impl<'a, P, R, I, E, S> GenericInstance<'a, P, R, I, E, S> for &'a Instance<'a, P, R, I, E, S>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
{
    type Owned<'o>
        = Instance<'o, P, R, I, E, S>
    where
        P: 'o,
        R: 'o,
        I: 'o,
        E: 'o,
        S: 'o;

    #[inline(always)]
    fn to_owned_instance<'o>(self) -> Self::Owned<'o> {
        self.view().to_owned_instance()
    }
}

impl<'a, 'b: 'a, P, R, I, E, S> Viewable<Pin<&'a InstanceView<'a, P, R, I, E, S>>>
    for &'b Instance<'b, P, R, I, E, S>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
{
    #[inline(always)]
    fn view(self) -> Pin<&'a InstanceView<'a, P, R, I, E, S>> {
        self.0
    }
}

impl<P, R, I, E, S> Clone for Instance<'_, P, R, I, E, S>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
{
    fn clone(&self) -> Self {
        self.to_owned_instance()
    }
}

impl<P, R, I, E, S> Drop for Instance<'_, P, R, I, E, S>
where
    P: Send + Sync,
    R: Send + Sync,
    I: Send + Sync,
    E: Send + Sync,
    S: Send + Sync,
{
    fn drop(&mut self) {
        let view = self.view();
        unsafe {
            let inner = Pin::into_inner_unchecked(view);
            let f = inner.vtable.release;
            f(view.view());
        }
    }
}

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
        $view_vis struct $view<'a>($crate::module::InstanceView<'a, $p, $r, $i, $e, $s>);

        impl $view<'_> {
            #[inline(always)]
            const fn view_inner(self: core::pin::Pin<&Self>)
                -> core::pin::Pin<&$crate::module::InstanceView<'_, $p, $r, $i, $e, $s>> {
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
                -> core::pin::Pin<&$crate::module::InfoView<'_>> {
                self.view_inner().info()
            }

            /// Returns a reference to the state of the instance.
            #[inline(always)]
            pub const fn state(self: core::pin::Pin<&Self>) -> core::pin::Pin<&$s> {
                self.view_inner().state()
            }
        }

        impl<'a> $crate::module::GenericInstance<'a, $p, $r, $i, $e, $s> for core::pin::Pin<&'a $view<'a>> {
            type Owned<'o>
                = $owned<'o>
            where
                $p: 'o,
                $r: 'o,
                $i: 'o,
                $e: 'o,
                $s: 'o;

            #[inline(always)]
            fn to_owned_instance<'o>(self) -> Self::Owned<'o> {
                $owned(self.view_inner().to_owned_instance())
            }
        }

        impl $crate::ffi::View for core::pin::Pin<&'_ $view<'_>> {}

        impl<'a> $crate::ffi::Viewable<core::pin::Pin<&'a $crate::module::InstanceView<'a, $p, $r, $i, $e, $s>>> for core::pin::Pin<&'a $view<'a>> {
            #[inline(always)]
            fn view(self)
                -> core::pin::Pin<&'a $crate::module::InstanceView<'a, $p, $r, $i, $e, $s>> {
                self.view_inner()
            }
        }

        /// Owned handle to an instance.
        #[repr(transparent)]
        #[derive(Debug, Clone)]
        $owned_vis struct $owned<'a>($crate::module::Instance<'a, $p, $r, $i, $e, $s>);

        impl $owned<'_> {
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
            pub const fn info(&self) -> core::pin::Pin<&$crate::module::InfoView<'_>> {
                self.0.info()
            }

            /// Returns a reference to the state of the instance.
            #[inline(always)]
            pub const fn state(&self) -> core::pin::Pin<&$s> {
                self.0.state()
            }
        }

        impl<'a> $crate::module::GenericInstance<'a, $p, $r, $i, $e, $s> for &'a $owned<'a> {
            type Owned<'o>
                = $owned<'o>
            where
                $p: 'o,
                $r: 'o,
                $i: 'o,
                $e: 'o,
                $s: 'o;

            #[inline(always)]
            fn to_owned_instance<'o>(self) -> Self::Owned<'o> {
                $owned(self.0.to_owned_instance())
            }
        }

        impl<'a, 'b: 'a> $crate::ffi::Viewable<core::pin::Pin<&'a $view<'a>>> for &'a $owned<'a> {
            #[inline(always)]
            fn view(self) -> core::pin::Pin<&'a $view<'a>> {
                unsafe { std::mem::transmute(self) }
            }
        }

        impl<'a, 'b: 'a> $crate::ffi::Viewable<core::pin::Pin<&'a $crate::module::InstanceView<'a, $p, $r, $i, $e, $s>>> for &'a $owned<'a> {
            #[inline(always)]
            fn view(self)
                -> core::pin::Pin<&'a $crate::module::InstanceView<'a, $p, $r, $i, $e, $s>> {
                self.0.view()
            }
        }
    };
}
