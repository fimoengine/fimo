//! Specification of a module registry.
use fimo_ffi::fn_wrapper::RawFnOnce;
use fimo_ffi::object::{CoerceObject, ObjectWrapper};
use fimo_ffi::{ArrayString, HeapFnOnce, ObjArc};
use fimo_module_core::{
    fimo_object, fimo_vtable, Error, IModuleInterface, IModuleInterfaceVTable, IModuleLoader,
    IModuleLoaderVTable, ModuleInterfaceDescriptor, SendSyncMarker,
};
use fimo_version_core::Version;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;

fimo_object! {
    /// Interface to a module registry.
    ///
    /// The underlying type must implement `Send` and `Sync`.
    pub struct IModuleRegistry<vtable = ModuleRegistryVTable>;
}

impl IModuleRegistry {
    /// Enters the inner registry.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    #[inline]
    pub fn enter_inner<F: FnOnce(&'_ mut IModuleRegistryInner)>(&self, f: F) {
        let mut wrapper = move |inner: *mut IModuleRegistryInner| {
            unsafe { f(&mut *inner) };
        };
        let wrapper_ref = unsafe { RawFnOnce::new(&mut wrapper) };

        let (ptr, vtable) = self.into_raw_parts();
        (vtable.enter_inner)(ptr, wrapper_ref)
    }

    /// Registers a new module loader to the `ModuleRegistry`.
    ///
    /// The registered loader will be available to the rest of the `ModuleRegistry`.
    #[inline]
    pub fn register_loader<T: CoerceObject<IModuleLoaderVTable>>(
        &self,
        r#type: &str,
        loader: &'static T,
    ) -> Result<LoaderHandle<'_, T>, Error> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.register_loader(r#type, loader));
            });
        }

        unsafe {
            res.assume_init()
                .map(|id| LoaderHandle::from_raw(id, loader, self))
        }
    }

    /// Unregisters an existing module loader from the `ModuleRegistry`.
    ///
    /// Notifies all registered callbacks before returning.
    #[inline]
    pub fn unregister_loader<T: CoerceObject<IModuleLoaderVTable>>(
        &self,
        loader: LoaderHandle<'_, T>,
    ) -> Result<&'static T, Error> {
        let (id, loader, _) = loader.into_raw();
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.unregister_loader(id));
            });
        }

        unsafe { res.assume_init().map(|_| loader) }
    }

    /// Registers a loader-removal callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the loader is removed.
    ///
    /// # Deadlock
    ///
    /// The callback may only call into the registry over the provided reference.
    #[inline]
    pub fn register_loader_callback<
        F: FnOnce(&mut IModuleRegistryInner, &'static IModuleLoader) + Send + Sync,
    >(
        &self,
        r#type: &str,
        callback: F,
    ) -> Result<LoaderCallbackHandle<'_>, Error> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.register_loader_callback(r#type, callback));
            });
        }

        unsafe {
            res.assume_init()
                .map(|id| LoaderCallbackHandle::from_raw_parts(id, self))
        }
    }

    /// Unregisters a loader-removal callback from the `ModuleRegistry`.
    ///
    /// The callback will not be called.
    #[inline]
    pub fn unregister_loader_callback(
        &self,
        handle: LoaderCallbackHandle<'_>,
    ) -> Result<(), Error> {
        let (id, _) = handle.into_raw_parts();
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.unregister_loader_callback(id));
            });
        }

        unsafe { res.assume_init() }
    }

    /// Fetches the loader associated with the `type`.
    #[inline]
    pub fn get_loader_from_type(&self, r#type: &str) -> Result<&'static IModuleLoader, Error> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.get_loader_from_type(r#type));
            });
        }

        unsafe { res.assume_init() }
    }

    /// Registers a new interface to the `ModuleRegistry`.
    #[inline]
    pub fn register_interface<T: CoerceObject<IModuleInterfaceVTable>>(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: ObjArc<T>,
    ) -> Result<InterfaceHandle<'_, T>, Error> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            let interface = interface.clone();
            self.enter_inner(move |inner| {
                res.write(inner.register_interface(descriptor, interface));
            });
        }

        unsafe {
            res.assume_init()
                .map(|id| InterfaceHandle::from_raw_parts(id, interface, self))
        }
    }

    /// Unregisters an existing interface from the `ModuleRegistry`.
    ///
    /// This function calls the interface-remove callbacks that are registered
    /// with the interface before removing it.
    #[inline]
    pub fn unregister_interface<T: CoerceObject<IModuleInterfaceVTable>>(
        &self,
        handle: InterfaceHandle<'_, T>,
    ) -> Result<ObjArc<T>, Error> {
        let (id, i, _) = handle.into_raw_parts();
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.unregister_interface(id));
            });
        }

        unsafe { res.assume_init().map(|_| i) }
    }

    /// Registers an interface-removed callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
    ///
    /// # Deadlock
    ///
    /// The callback may only call into the registry over the provided reference.
    #[inline]
    pub fn register_interface_callback<
        F: FnOnce(&mut IModuleRegistryInner, ObjArc<IModuleInterface>) + Send + Sync,
    >(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: F,
    ) -> Result<InterfaceCallbackHandle<'_>, Error> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.register_interface_callback(descriptor, callback));
            });
        }

        unsafe {
            res.assume_init()
                .map(|id| InterfaceCallbackHandle::from_raw_parts(id, self))
        }
    }

    /// Unregisters an interface-removed callback from the `ModuleRegistry` without calling it.
    #[inline]
    pub fn unregister_interface_callback(
        &self,
        handle: InterfaceCallbackHandle<'_>,
    ) -> Result<(), Error> {
        let (id, _) = handle.into_raw_parts();
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.unregister_interface_callback(id));
            });
        }

        unsafe { res.assume_init() }
    }

    /// Extracts an interface from the `ModuleRegistry`.
    #[inline]
    pub fn get_interface_from_descriptor(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<ObjArc<IModuleInterface>, Error> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.get_interface_from_descriptor(descriptor));
            });
        }

        unsafe { res.assume_init() }
    }

    /// Extracts all interface descriptors with the same name.
    #[inline]
    pub fn get_interface_descriptors_from_name(
        &self,
        name: &str,
    ) -> Vec<ModuleInterfaceDescriptor> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.get_interface_descriptors_from_name(name));
            });
        }

        unsafe { res.assume_init() }
    }

    /// Extracts all descriptors of compatible interfaces.
    #[inline]
    pub fn get_compatible_interface_descriptors(
        &self,
        name: &str,
        version: &Version,
        extensions: &[ArrayString<128>],
    ) -> Vec<ModuleInterfaceDescriptor> {
        let mut res = MaybeUninit::uninit();

        {
            let res = &mut res;
            self.enter_inner(move |inner| {
                res.write(inner.get_compatible_interface_descriptors(name, version, extensions));
            });
        }

        unsafe { res.assume_init() }
    }
}

fimo_vtable! {
    /// VTable of the [`IModuleRegistry`] type.
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct ModuleRegistryVTable<id = "fimo::interfaces::core::module::module_registry", marker = SendSyncMarker> {
        enter_inner: fn(*const (), RawFnOnce<(*mut IModuleRegistryInner,), ()>),
    }
}

fimo_object! {
    /// Type-erased module registry.
    ///
    /// The underlying type must implement `Send` and `Sync`.
    pub struct IModuleRegistryInner<vtable = ModuleRegistryInnerVTable>;
}

impl IModuleRegistryInner {
    /// Registers a new module loader to the `ModuleRegistry`.
    ///
    /// The registered loader will be available to the rest of the `ModuleRegistry`.
    #[inline]
    pub fn register_loader<T: CoerceObject<IModuleLoaderVTable>>(
        &mut self,
        r#type: &str,
        loader: &'static T,
    ) -> Result<LoaderId, Error> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        let loader = loader.coerce_obj();
        let loader = IModuleLoader::from_object(loader);
        (vtable.register_loader)(ptr, r#type, loader)
    }

    /// Unregisters an existing module loader from the `ModuleRegistry`.
    ///
    /// Notifies all registered callbacks before returning.
    #[inline]
    pub fn unregister_loader(&mut self, id: LoaderId) -> Result<&'static IModuleLoader, Error> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.unregister_loader)(ptr, id)
    }

    /// Registers a loader-removal callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the loader is removed.
    ///
    /// # Deadlock
    ///
    /// The callback may only call into the registry over the provided reference.
    #[inline]
    pub fn register_loader_callback<
        F: FnOnce(&mut IModuleRegistryInner, &'static IModuleLoader) + Send + Sync,
    >(
        &mut self,
        r#type: &str,
        callback: F,
    ) -> Result<LoaderCallbackId, Error> {
        let wrapper = Box::new(move |inner: *mut IModuleRegistryInner, loader| unsafe {
            callback(&mut *inner, loader)
        });

        let callback = LoaderCallback::from(wrapper);
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.register_loader_callback)(ptr, r#type, callback)
    }

    /// Unregisters a loader-removal callback from the `ModuleRegistry`.
    ///
    /// The callback will not be called.
    #[inline]
    pub fn unregister_loader_callback(&mut self, id: LoaderCallbackId) -> Result<(), Error> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.unregister_loader_callback)(ptr, id)
    }

    /// Fetches the loader associated with the type.
    #[inline]
    pub fn get_loader_from_type(&self, r#type: &str) -> Result<&'static IModuleLoader, Error> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_loader_from_type)(ptr, r#type)
    }

    /// Registers a new interface to the `ModuleRegistry`.
    #[inline]
    pub fn register_interface<T: CoerceObject<IModuleInterfaceVTable>>(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: ObjArc<T>,
    ) -> Result<InterfaceId, Error> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        let interface = ObjArc::coerce_object(interface);
        (vtable.register_interface)(ptr, descriptor, interface)
    }

    /// Unregisters an existing interface from the `ModuleRegistry`.
    ///
    /// This function calls the interface-remove callbacks that are registered
    /// with the interface before removing it.
    #[inline]
    pub fn unregister_interface(
        &mut self,
        id: InterfaceId,
    ) -> Result<ObjArc<IModuleInterface>, Error> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.unregister_interface)(ptr, id)
    }

    /// Registers an interface-removed callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
    ///
    /// # Deadlock
    ///
    /// The callback may only call into the registry over the provided reference.
    #[inline]
    pub fn register_interface_callback<
        F: FnOnce(&mut IModuleRegistryInner, ObjArc<IModuleInterface>) + Send + Sync,
    >(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: F,
    ) -> Result<InterfaceCallbackId, Error> {
        let wrapper = Box::new(move |inner: *mut IModuleRegistryInner, interface| unsafe {
            callback(&mut *inner, interface)
        });

        let callback = InterfaceCallback::from(wrapper);
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.register_interface_callback)(ptr, descriptor, callback)
    }

    /// Unregisters an interface-removed callback from the `ModuleRegistry` without calling it.
    #[inline]
    pub fn unregister_interface_callback(&mut self, id: InterfaceCallbackId) -> Result<(), Error> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.unregister_interface_callback)(ptr, id)
    }

    /// Extracts an interface from the `ModuleRegistry`.
    #[inline]
    pub fn get_interface_from_descriptor(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<ObjArc<IModuleInterface>, Error> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_interface_from_descriptor)(ptr, descriptor)
    }

    /// Extracts all interface descriptors with the same name.
    #[inline]
    pub fn get_interface_descriptors_from_name(
        &self,
        name: &str,
    ) -> Vec<ModuleInterfaceDescriptor> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_interface_descriptors_from_name)(ptr, name)
    }

    /// Extracts all descriptors of compatible interfaces.
    #[inline]
    pub fn get_compatible_interface_descriptors(
        &self,
        name: &str,
        version: &Version,
        extensions: &[ArrayString<128>],
    ) -> Vec<ModuleInterfaceDescriptor> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_compatible_interface_descriptors)(ptr, name, version, extensions)
    }
}

fimo_vtable! {
    /// VTable of the [`IModuleRegistryInner`] type.
    #[allow(clippy::type_complexity)]
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct ModuleRegistryInnerVTable<id = "fimo::interfaces::core::module::module_registry", marker = SendSyncMarker> {
        /// Registers a new module loader to the `ModuleRegistry`.
        ///
        /// The registered loader will be available to the rest of the `ModuleRegistry`.
        pub register_loader:
            fn(*mut (), *const str, &'static IModuleLoader) -> Result<LoaderId, Error>,
        /// Unregisters an existing module loader from the `ModuleRegistry`.
        ///
        /// Notifies all registered callbacks before returning.
        pub unregister_loader: fn(*mut (), LoaderId) -> Result<&'static IModuleLoader, Error>,
        /// Registers a loader-removal callback to the `ModuleRegistry`.
        ///
        /// The callback will be called in case the loader is removed.
        ///
        /// # Deadlock
        ///
        /// The callback may only call into the registry over the provided reference.
        pub register_loader_callback:
            fn(*mut (), *const str, LoaderCallback) -> Result<LoaderCallbackId, Error>,
        /// Unregisters a loader-removal callback from the `ModuleRegistry`.
        ///
        /// The callback will not be called.
        pub unregister_loader_callback: fn(*mut (), LoaderCallbackId) -> Result<(), Error>,
        /// Fetches the loader associated with the type.
        pub get_loader_from_type:
            fn(*const (), *const str) -> Result<&'static IModuleLoader, Error>,
        /// Registers a new interface to the `ModuleRegistry`.
        pub register_interface: fn(
            *mut (),
            *const ModuleInterfaceDescriptor,
            ObjArc<IModuleInterface>,
        ) -> Result<InterfaceId, Error>,
        /// Unregisters an existing interface from the `ModuleRegistry`.
        ///
        /// This function calls the interface-remove callbacks that are registered
        /// with the interface before removing it.
        pub unregister_interface:
            fn(*mut (), InterfaceId) -> Result<ObjArc<IModuleInterface>, Error>,
        /// Registers an interface-removed callback to the `ModuleRegistry`.
        ///
        /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
        ///
        /// # Deadlock
        ///
        /// The callback may only call into the registry over the provided reference.
        pub register_interface_callback: fn(
            *mut (),
            *const ModuleInterfaceDescriptor,
            InterfaceCallback,
        ) -> Result<InterfaceCallbackId, Error>,
        /// Unregisters an interface-removed callback from the `ModuleRegistry` without calling it.
        pub unregister_interface_callback: fn(*mut (), InterfaceCallbackId) -> Result<(), Error>,
        /// Extracts an interface from the `ModuleRegistry`.
        pub get_interface_from_descriptor: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
        ) -> Result<ObjArc<IModuleInterface>, Error>,
        /// Extracts all interface descriptors with the same name.
        pub get_interface_descriptors_from_name:
            fn(*const (), *const str) -> Vec<ModuleInterfaceDescriptor>,
        /// Extracts all descriptors of compatible interfaces.
        pub get_compatible_interface_descriptors: fn(
            *const (),
            *const str,
            *const Version,
            *const [ArrayString<128>],
        ) -> Vec<ModuleInterfaceDescriptor>,
    }
}

/// Handle to a loader.
#[derive(Debug)]
pub struct LoaderHandle<'a, T: CoerceObject<IModuleLoaderVTable> + 'static> {
    id: LoaderId,
    loader: &'static T,
    registry: &'a IModuleRegistry,
}

impl<'a, T: CoerceObject<IModuleLoaderVTable> + 'static> LoaderHandle<'a, T> {
    /// Constructs a new `LoaderHandle` from from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that `T` is matches with the
    /// original type.
    #[inline]
    pub unsafe fn from_raw(
        id: LoaderId,
        loader: &'static T,
        registry: &'a IModuleRegistry,
    ) -> Self {
        Self {
            id,
            loader,
            registry,
        }
    }

    /// Splits the `LoaderHandle` into its raw components.
    #[inline]
    pub fn into_raw(self) -> (LoaderId, &'static T, &'a IModuleRegistry) {
        let id = unsafe { std::ptr::read(&self.id) };
        let loader = unsafe { std::ptr::read(&self.loader) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, loader, registry)
    }
}

impl<'a, T: CoerceObject<IModuleLoaderVTable>> Deref for LoaderHandle<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &'static Self::Target {
        self.loader
    }
}

impl<T: CoerceObject<IModuleLoaderVTable>> Drop for LoaderHandle<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // safety: `LoaderId` is a simple `usize`.
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_inner(move |inner| {
            let _ = inner.unregister_loader(id);
        });
    }
}

/// Id of a loader.
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

/// Handle to a interface.
#[derive(Debug)]
pub struct InterfaceHandle<'a, T: CoerceObject<IModuleInterfaceVTable>> {
    id: InterfaceId,
    interface: ObjArc<T>,
    registry: &'a IModuleRegistry,
    _phantom: PhantomData<fn() -> *const T>,
}

impl<'a, T: CoerceObject<IModuleInterfaceVTable>> InterfaceHandle<'a, T> {
    /// Constructs a new `InterfaceHandle` from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that `T` is matches with the
    /// original type.
    #[inline]
    pub unsafe fn from_raw_parts(
        id: InterfaceId,
        interface: ObjArc<T>,
        registry: &'a IModuleRegistry,
    ) -> Self {
        Self {
            id,
            interface,
            registry,
            _phantom: Default::default(),
        }
    }

    /// Splits the `InterfaceHandle` into its raw parts.
    #[inline]
    pub fn into_raw_parts(self) -> (InterfaceId, ObjArc<T>, &'a IModuleRegistry) {
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

impl<'a, T: CoerceObject<IModuleInterfaceVTable>> Deref for InterfaceHandle<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.interface
    }
}

impl<T: CoerceObject<IModuleInterfaceVTable>> Drop for InterfaceHandle<'_, T> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_inner(move |inner| {
            let _ = inner.unregister_interface(id);
        });
    }
}

/// Id of an interface.
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

/// A RAII guard for loader callbacks.
#[derive(Debug)]
pub struct LoaderCallbackHandle<'a> {
    id: LoaderCallbackId,
    registry: &'a IModuleRegistry,
}

impl<'a> LoaderCallbackHandle<'a> {
    /// Splits a `LoaderCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (LoaderCallbackId, &'a IModuleRegistry) {
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
    pub unsafe fn from_raw_parts(id: LoaderCallbackId, registry: &'a IModuleRegistry) -> Self {
        Self { id, registry }
    }
}

impl Drop for LoaderCallbackHandle<'_> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_inner(move |inner| {
            let _ = inner.unregister_loader_callback(id);
        });
    }
}

/// Id of a loader callback.
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

/// A loader removed callback.
#[derive(Debug)]
pub struct LoaderCallback {
    inner: HeapFnOnce<(*mut IModuleRegistryInner, &'static IModuleLoader), ()>,
}

impl FnOnce<(*mut IModuleRegistryInner, &'static IModuleLoader)> for LoaderCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(
        self,
        args: (*mut IModuleRegistryInner, &'static IModuleLoader),
    ) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl<F: FnOnce(*mut IModuleRegistryInner, &'static IModuleLoader) + Send + Sync> From<Box<F>>
    for LoaderCallback
{
    #[inline]
    fn from(data: Box<F>) -> Self {
        Self {
            inner: HeapFnOnce::new_boxed(data),
        }
    }
}

unsafe impl Send for LoaderCallback {}
unsafe impl Sync for LoaderCallback {}

/// A RAII guard for interface callbacks.
#[derive(Debug)]
pub struct InterfaceCallbackHandle<'a> {
    id: InterfaceCallbackId,
    registry: &'a IModuleRegistry,
}

impl<'a> InterfaceCallbackHandle<'a> {
    /// Splits a `InterfaceCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (InterfaceCallbackId, &'a IModuleRegistry) {
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
    pub unsafe fn from_raw_parts(id: InterfaceCallbackId, registry: &'a IModuleRegistry) -> Self {
        Self { id, registry }
    }
}

impl Drop for InterfaceCallbackHandle<'_> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_inner(move |inner| {
            let _ = inner.unregister_interface_callback(id);
        });
    }
}

/// Id of a interface callback.
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

/// A loader removed callback.
#[derive(Debug)]
pub struct InterfaceCallback {
    inner: HeapFnOnce<(*mut IModuleRegistryInner, ObjArc<IModuleInterface>), ()>,
}

impl FnOnce<(*mut IModuleRegistryInner, ObjArc<IModuleInterface>)> for InterfaceCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(
        self,
        args: (*mut IModuleRegistryInner, ObjArc<IModuleInterface>),
    ) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl<F: FnOnce(*mut IModuleRegistryInner, ObjArc<IModuleInterface>) + Send + Sync> From<Box<F>>
    for InterfaceCallback
{
    #[inline]
    fn from(data: Box<F>) -> Self {
        Self {
            inner: HeapFnOnce::new_boxed(data),
        }
    }
}

unsafe impl Send for InterfaceCallback {}
unsafe impl Sync for InterfaceCallback {}
