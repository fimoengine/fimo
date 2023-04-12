//! Specification of a module registry.
use fimo_module::fimo_ffi::ptr::{CastInto, IBaseExt};
use fimo_module::fimo_ffi::{interface, DynObj, FfiFn, ObjArc};
use fimo_module::{IModuleInterface, IModuleLoader, InterfaceDescriptor, InterfaceQuery};
use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;
use std::ops::Deref;

interface! {
    #![interface_cfg(uuid = "1eb872f8-a966-40a6-844d-9db4b08bf6db")]

    /// Interface to a module registry.
    pub frozen interface IModuleRegistry: marker Send + marker Sync {
        /// Enters the inner registry, possibly locking it.
        ///
        /// # Deadlock
        ///
        /// The function may only call into the registry with the provided inner reference.
        #[allow(clippy::type_complexity)]
        fn enter_impl(&self, f: FfiFn<'_, dyn FnOnce(&'_ DynObj<dyn IModuleRegistryInner + '_>) + '_>);

        /// Enters the inner registry with write access, possibly locking it.
        ///
        /// # Deadlock
        ///
        /// The function may only call into the registry with the provided inner reference.
        #[allow(clippy::type_complexity)]
        fn enter_mut_impl(
            &self,
            f: FfiFn<'_, dyn FnOnce(&'_ mut DynObj<dyn IModuleRegistryInner + '_>) + '_>,
        );
    }
}

/// Object unsafe extension trait for implementations of [`IModuleRegistry`].
pub trait IModuleRegistryExt: IModuleRegistry {
    /// Enters the inner registry, possibly locking it.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    fn enter<F, R>(&self, f: F) -> R
    where
        F: for<'r, 's> FnOnce(&'r DynObj<dyn IModuleRegistryInner + 's>) -> R,
    {
        // allocate value where the result will be written to.
        let mut res = MaybeUninit::uninit();
        let f = |inner: &'_ DynObj<dyn IModuleRegistryInner + '_>| {
            res.write(f(inner));
        };
        let mut f = MaybeUninit::new(f);
        let f = unsafe { FfiFn::new_value(&mut f) };
        self.enter_impl(f);

        // safety: f either initializes the result or panicked and this point won't be reached.
        unsafe { res.assume_init() }
    }

    /// Enters the inner registry with write access, possibly locking it.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    fn enter_mut<F, R>(&self, f: F) -> R
    where
        F: for<'r, 's> FnOnce(&'r mut DynObj<dyn IModuleRegistryInner + 's>) -> R,
    {
        // allocate value where the result will be written to.
        let mut res = MaybeUninit::uninit();
        let f = |inner: &'_ mut DynObj<dyn IModuleRegistryInner + '_>| {
            res.write(f(inner));
        };
        let mut f = MaybeUninit::new(f);
        let f = unsafe { FfiFn::new_value(&mut f) };
        self.enter_mut_impl(f);

        // safety: f either initializes the result or panicked and this point won't be reached.
        unsafe { res.assume_init() }
    }

    /// Registers a new module loader with the `ModuleRegistry`.
    ///
    /// The registered loader will be available to the rest of the `ModuleRegistry`.
    fn register_loader<T: ?Sized + 'static>(
        &self,
        r#type: &str,
        loader: &'static DynObj<T>,
    ) -> fimo_module::Result<LoaderHandle<'_, DynObj<T>, Self>>
    where
        T: CastInto<'static, dyn IModuleLoader>,
        DynObj<T>: IModuleLoader,
    {
        self.enter_mut(move |inner| {
            let l: &DynObj<dyn IModuleLoader> = loader.cast_super();
            let id = inner.register_loader(r#type, l)?;
            unsafe { Ok(LoaderHandle::from_raw(id, loader, self)) }
        })
    }

    /// Unregisters an existing module loader from the `ModuleRegistry`.
    ///
    /// Notifies all registered callbacks before returning.
    fn unregister_loader<T: ?Sized + 'static>(
        &self,
        loader: LoaderHandle<'_, DynObj<T>, Self>,
    ) -> fimo_module::Result<&'static DynObj<T>>
    where
        DynObj<T>: IModuleLoader,
    {
        self.enter_mut(move |inner| {
            let (id, loader, _) = loader.into_raw();
            inner.unregister_loader(id)?;
            Ok(loader)
        })
    }

    /// Registers a [`LoaderCallback`] with the `ModuleRegistry`.
    ///
    /// The callback will be called in case the loader is removed.
    ///
    /// # Deadlock
    ///
    /// The callback may only call into the registry over the provided reference.
    fn register_loader_callback<F>(
        &self,
        r#type: &str,
        f: F,
    ) -> fimo_module::Result<LoaderCallbackHandle<'_, Self>>
    where
        F: FnOnce(&mut DynObj<dyn IModuleRegistryInner>, &'static DynObj<dyn IModuleLoader>)
            + Send
            + Sync
            + 'static,
    {
        self.enter_mut(move |inner| {
            let f = LoaderCallback::r#box(Box::new(f));
            let id = inner.register_loader_callback(r#type, f)?;
            unsafe { Ok(LoaderCallbackHandle::from_raw_parts(id, self)) }
        })
    }

    /// Unregisters a [`LoaderCallback`] from the `ModuleRegistry`.
    ///
    /// The callback will not be called.
    fn unregister_loader_callback(
        &self,
        handle: LoaderCallbackHandle<'_, Self>,
    ) -> fimo_module::Result<()> {
        self.enter_mut(move |inner| {
            let (id, _) = handle.into_raw_parts();
            inner.unregister_loader_callback(id)
        })
    }

    /// Fetches the loader associated with the type.
    fn get_loader_from_type(
        &self,
        r#type: &str,
    ) -> fimo_module::Result<&'static DynObj<dyn IModuleLoader>> {
        self.enter(move |inner| inner.get_loader_from_type(r#type))
    }

    /// Registers a new service with the `ModuleRegistry`.
    fn register_service<T: ?Sized>(&self, service: &'static DynObj<T>) -> fimo_module::Result<()>
    where
        T: CastInto<'static, dyn IModuleInterface>,
        DynObj<T>: IModuleInterface,
    {
        self.enter_mut(move |inner| {
            let service = service.cast_super();
            inner.register_service(service)
        })
    }

    /// Registers a new interface with the `ModuleRegistry`.
    fn register_interface<T: ?Sized>(
        &self,
        inherit_services: bool,
        i: ObjArc<DynObj<T>>,
    ) -> fimo_module::Result<InterfaceHandle<'_, DynObj<T>, Self>>
    where
        T: CastInto<'static, dyn IModuleInterface>,
        DynObj<T>: IModuleInterface,
    {
        self.enter_mut(move |inner| {
            let i_interface = ObjArc::cast_super(i.clone());
            let id = inner.register_interface(inherit_services, i_interface)?;
            unsafe { Ok(InterfaceHandle::from_raw_parts(id, i, self)) }
        })
    }

    /// Unregisters an existing interface from the `ModuleRegistry`.
    ///
    /// This function calls the [`InterfaceCallback`] that are registered
    /// with the interface before removing it.
    fn unregister_interface<T: ?Sized>(
        &self,
        handle: InterfaceHandle<'_, DynObj<T>, Self>,
    ) -> fimo_module::Result<ObjArc<DynObj<T>>>
    where
        DynObj<T>: IModuleInterface,
    {
        self.enter_mut(move |inner| {
            let (id, i, _) = handle.into_raw_parts();
            inner.unregister_interface(id)?;
            Ok(i)
        })
    }

    /// Registers an [`InterfaceCallback`] with the `ModuleRegistry`.
    ///
    /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
    ///
    /// # Deadlock
    ///
    /// The callback may only call into the registry over the provided reference.
    fn register_interface_callback<F>(
        &self,
        desc: &InterfaceDescriptor,
        f: F,
    ) -> fimo_module::Result<InterfaceCallbackHandle<'_, Self>>
    where
        F: FnOnce(&mut DynObj<dyn IModuleRegistryInner>, ObjArc<DynObj<dyn IModuleInterface>>)
            + Send
            + Sync
            + 'static,
    {
        self.enter_mut(move |inner| {
            let f = InterfaceCallback::r#box(Box::new(f));
            let id = inner.register_interface_callback(desc, f)?;
            unsafe { Ok(InterfaceCallbackHandle::from_raw_parts(id, self)) }
        })
    }

    /// Unregisters an [`InterfaceCallback`] from the `ModuleRegistry` without calling it.
    fn unregister_interface_callback(
        &self,
        handle: InterfaceCallbackHandle<'_, Self>,
    ) -> fimo_module::Result<()> {
        self.enter_mut(move |inner| {
            let (id, _) = handle.into_raw_parts();
            inner.unregister_interface_callback(id)
        })
    }

    /// Extracts an interface from the `ModuleRegistry`.
    fn get_interface_from_descriptor(
        &self,
        desc: &InterfaceDescriptor,
    ) -> fimo_module::Result<ObjArc<DynObj<dyn IModuleInterface>>> {
        self.enter(move |inner| inner.get_interface_from_descriptor(desc))
    }

    /// Extracts all interface descriptors that match a query.
    fn query_interfaces(&self, query: &InterfaceQuery) -> Vec<InterfaceDescriptor> {
        self.enter(move |inner| inner.query_interfaces(query))
    }
}

impl<T: IModuleRegistry + ?Sized> IModuleRegistryExt for T {}

interface! {
    #![interface_cfg(uuid = "1eb872f8-a966-40a6-844d-9db4b08bf6db")]

    /// Type-erased module registry.
    pub frozen interface IModuleRegistryInner: marker Send + marker Sync {
        /// Registers a new module loader with the `ModuleRegistry`.
        ///
        /// The registered loader will be available to the rest of the `ModuleRegistry`.
        fn register_loader(
            &mut self,
            r#type: &str,
            loader: &'static DynObj<dyn IModuleLoader>,
        ) -> fimo_module::Result<LoaderId>;

        /// Unregisters an existing module loader from the `ModuleRegistry`.
        ///
        /// Notifies all registered callbacks before returning.
        fn unregister_loader(
            &mut self,
            id: LoaderId,
        ) -> fimo_module::Result<&'static DynObj<dyn IModuleLoader>>;

        /// Registers a [`LoaderCallback`] with the `ModuleRegistry`.
        ///
        /// The callback will be called in case the loader is removed.
        ///
        /// # Deadlock
        ///
        /// The callback may only call into the registry over the provided reference.
        fn register_loader_callback(
            &mut self,
            r#type: &str,
            f: LoaderCallback,
        ) -> fimo_module::Result<LoaderCallbackId>;

        /// Unregisters a [`LoaderCallback`] from the `ModuleRegistry`.
        ///
        /// The callback will not be called.
        fn unregister_loader_callback(&mut self, id: LoaderCallbackId) -> fimo_module::Result<()>;

        /// Fetches the loader associated with the type.
        fn get_loader_from_type(
            &self,
            r#type: &str,
        ) -> fimo_module::Result<&'static DynObj<dyn IModuleLoader>>;

        /// Registers a new service with the `ModuleRegistry`.
        ///
        /// The service will be bound to every existing and new interface, if
        /// the interface is registered with the inherit flag.
        fn register_service(
            &mut self,
            service: &'static DynObj<dyn IModuleInterface>,
        ) -> fimo_module::Result<()>;

        /// Registers a new interface with the `ModuleRegistry`.
        fn register_interface(
            &mut self,
            inherit_services: bool,
            i: ObjArc<DynObj<dyn IModuleInterface>>,
        ) -> fimo_module::Result<InterfaceId>;

        /// Unregisters an existing interface from the `ModuleRegistry`.
        ///
        /// This function calls the [`InterfaceCallback`] that are registered
        /// with the interface before removing it.
        fn unregister_interface(
            &mut self,
            id: InterfaceId,
        ) -> fimo_module::Result<ObjArc<DynObj<dyn IModuleInterface>>>;

        /// Registers an [`InterfaceCallback`] with the `ModuleRegistry`.
        ///
        /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
        ///
        /// # Deadlock
        ///
        /// The callback may only call into the registry over the provided reference.
        fn register_interface_callback(
            &mut self,
            desc: &InterfaceDescriptor,
            f: InterfaceCallback,
        ) -> fimo_module::Result<InterfaceCallbackId>;

        /// Unregisters an [`InterfaceCallback`] from the `ModuleRegistry` without calling it.
        fn unregister_interface_callback(&mut self, id: InterfaceCallbackId)
            -> fimo_module::Result<()>;

        /// Extracts an interface from the `ModuleRegistry`.
        fn get_interface_from_descriptor(
            &self,
            desc: &InterfaceDescriptor,
        ) -> fimo_module::Result<ObjArc<DynObj<dyn IModuleInterface>>>;

        /// Extracts all interface descriptors that match a query.
        fn query_interfaces(&self, query: &InterfaceQuery) -> Vec<InterfaceDescriptor>;
    }
}

/// Handle to a registered loader.
pub struct LoaderHandle<'a, T: IModuleLoader + ?Sized + 'static, R: IModuleRegistry + ?Sized> {
    id: LoaderId,
    loader: &'static T,
    registry: &'a R,
}

impl<'a, T: IModuleLoader + ?Sized + 'static, R: IModuleRegistry + ?Sized> LoaderHandle<'a, T, R> {
    /// Constructs a new `LoaderHandle` from from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that `T` is matches with the
    /// original type.
    #[inline]
    pub unsafe fn from_raw(id: LoaderId, loader: &'static T, registry: &'a R) -> Self {
        Self {
            id,
            loader,
            registry,
        }
    }

    /// Splits the `LoaderHandle` into its raw components.
    #[inline]
    pub fn into_raw(self) -> (LoaderId, &'static T, &'a R) {
        let id = unsafe { std::ptr::read(&self.id) };
        let loader = unsafe { std::ptr::read(&self.loader) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, loader, registry)
    }
}

impl<'a, T: IModuleLoader + ?Sized + 'static, R: IModuleRegistry + ?Sized> Debug
    for LoaderHandle<'a, T, R>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LoaderHandle").field(&self.id).finish()
    }
}

impl<'a, T: IModuleLoader + ?Sized + 'static, R: IModuleRegistry + ?Sized> Deref
    for LoaderHandle<'a, T, R>
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &'static Self::Target {
        self.loader
    }
}

impl<T: IModuleLoader + ?Sized + 'static, R: IModuleRegistry + ?Sized> Drop
    for LoaderHandle<'_, T, R>
{
    #[inline]
    fn drop(&mut self) {
        // safety: `LoaderId` is a simple `usize`.
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_mut(move |inner| {
            inner
                .unregister_loader(id)
                .expect("Can't drop the loader handle");
        });
    }
}

/// Id of a loader.
#[repr(transparent)]
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct LoaderId(usize);

impl LoaderId {
    /// Constructs a new `LoaderId` from its raw components.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_raw(id: usize) -> Self {
        Self(id)
    }
}

impl From<LoaderId> for usize {
    #[inline]
    fn from(id: LoaderId) -> Self {
        id.0
    }
}

/// Handle to a registered interface.
pub struct InterfaceHandle<'a, T: IModuleInterface + ?Sized, R: IModuleRegistry + ?Sized> {
    id: InterfaceId,
    interface: ObjArc<T>,
    registry: &'a R,
}

impl<'a, T: IModuleInterface + ?Sized, R: IModuleRegistry + ?Sized> InterfaceHandle<'a, T, R> {
    /// Constructs a new `InterfaceHandle` from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that `T` is matches with the
    /// original type.
    #[inline]
    pub unsafe fn from_raw_parts(id: InterfaceId, interface: ObjArc<T>, registry: &'a R) -> Self {
        Self {
            id,
            interface,
            registry,
        }
    }

    /// Splits the `InterfaceHandle` into its raw parts.
    #[inline]
    pub fn into_raw_parts(self) -> (InterfaceId, ObjArc<T>, &'a R) {
        let id = unsafe { std::ptr::read(&self.id) };
        let interface = unsafe { std::ptr::read(&self.interface) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, interface, registry)
    }

    /// Clones the wrapped interface.
    #[inline]
    pub fn get_interface(&self) -> ObjArc<T> {
        self.interface.clone()
    }
}

impl<'a, T: IModuleInterface + ?Sized, R: IModuleRegistry + ?Sized> Debug
    for InterfaceHandle<'a, T, R>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InterfaceHandle").field(&self.id).finish()
    }
}

impl<'a, T: IModuleInterface + ?Sized, R: IModuleRegistry + ?Sized> Deref
    for InterfaceHandle<'a, T, R>
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.interface
    }
}

impl<T: IModuleInterface + ?Sized, R: IModuleRegistry + ?Sized> Drop for InterfaceHandle<'_, T, R> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_mut(move |inner| {
            inner
                .unregister_interface(id)
                .expect("Can't drop the interface handle");
        });
    }
}

/// Id of an interface.
#[repr(transparent)]
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct InterfaceId(usize);

impl InterfaceId {
    /// Constructs a new `InterfaceId` from its raw components.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_raw(id: usize) -> Self {
        Self(id)
    }
}

impl From<InterfaceId> for usize {
    #[inline]
    fn from(id: InterfaceId) -> Self {
        id.0
    }
}

/// A callback invoked when a loader is removed from the registry.
pub type LoaderCallback = FfiFn<
    'static,
    dyn FnOnce(&mut DynObj<dyn IModuleRegistryInner>, &'static DynObj<dyn IModuleLoader>)
        + Send
        + Sync,
>;

/// A RAII guard for loader callbacks.
pub struct LoaderCallbackHandle<'a, R: IModuleRegistry + ?Sized> {
    id: LoaderCallbackId,
    registry: &'a R,
}

impl<'a, R: IModuleRegistry + ?Sized> LoaderCallbackHandle<'a, R> {
    /// Splits a `LoaderCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (LoaderCallbackId, &'a R) {
        let id = unsafe { std::ptr::read(&self.id) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, registry)
    }

    /// Constructs a new `LoaderCallbackHandle` from its raw components.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_raw_parts(id: LoaderCallbackId, registry: &'a R) -> Self {
        Self { id, registry }
    }
}

impl<R: IModuleRegistry + ?Sized> Debug for LoaderCallbackHandle<'_, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LoaderCallbackHandle")
            .field(&self.id)
            .finish()
    }
}

impl<R: IModuleRegistry + ?Sized> Drop for LoaderCallbackHandle<'_, R> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_mut(move |inner| {
            inner
                .unregister_loader_callback(id)
                .expect("Can't drop the loader callback");
        });
    }
}

/// Id of a registered [`LoaderCallback`].
#[repr(transparent)]
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct LoaderCallbackId(usize);

impl LoaderCallbackId {
    /// Constructs a new `LoaderCallbackId` from an `usize`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_usize(id: usize) -> Self {
        Self(id)
    }
}

impl From<LoaderCallbackId> for usize {
    #[inline]
    fn from(id: LoaderCallbackId) -> Self {
        id.0
    }
}

/// A callback invoked when an interface is removed from the registry.
pub type InterfaceCallback = FfiFn<
    'static,
    dyn FnOnce(&mut DynObj<dyn IModuleRegistryInner>, ObjArc<DynObj<dyn IModuleInterface>>)
        + Send
        + Sync,
>;

/// A RAII guard for a [`InterfaceCallback`].
pub struct InterfaceCallbackHandle<'a, R: IModuleRegistry + ?Sized> {
    id: InterfaceCallbackId,
    registry: &'a R,
}

impl<'a, R: IModuleRegistry + ?Sized> InterfaceCallbackHandle<'a, R> {
    /// Splits a `InterfaceCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (InterfaceCallbackId, &'a R) {
        let id = unsafe { std::ptr::read(&self.id) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, registry)
    }

    /// Constructs a new `InterfaceCallbackHandle` from its raw components.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_raw_parts(id: InterfaceCallbackId, registry: &'a R) -> Self {
        Self { id, registry }
    }
}

impl<R: IModuleRegistry + ?Sized> Debug for InterfaceCallbackHandle<'_, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InterfaceCallbackHandle")
            .field(&self.id)
            .finish()
    }
}

impl<R: IModuleRegistry + ?Sized> Drop for InterfaceCallbackHandle<'_, R> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_mut(move |inner| {
            inner
                .unregister_interface_callback(id)
                .expect("Can't drop the interface callback");
        });
    }
}

/// Id of a registered [`InterfaceCallback`].
#[repr(transparent)]
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct InterfaceCallbackId(usize);

impl InterfaceCallbackId {
    /// Constructs a new `InterfaceCallbackId` from an `usize`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_usize(id: usize) -> Self {
        Self(id)
    }
}

impl From<InterfaceCallbackId> for usize {
    #[inline]
    fn from(id: InterfaceCallbackId) -> Self {
        id.0
    }
}
