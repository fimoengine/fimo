use fimo_ffi_core::fn_wrapper::HeapFnOnce;
use fimo_ffi_core::ArrayString;
use fimo_module_core::{ModuleInterface, ModuleInterfaceDescriptor, ModuleLoader};
use fimo_version_core::Version;
use std::error::Error;
use std::marker::{PhantomData, Unsize};
use std::ops::Deref;
use std::sync::Arc;

/// Type-erased module registry.
///
/// The underlying type must implement `Send` and `Sync`.
pub struct ModuleRegistry {
    _inner: [()],
}

impl ModuleRegistry {
    /// Registers a new module loader to the `ModuleRegistry`.
    ///
    /// The registered loader will be available to the rest of the `ModuleRegistry`.
    #[inline]
    pub fn register_loader<T: ModuleLoader>(
        &self,
        r#type: &str,
        loader: &'static T,
    ) -> Result<LoaderHandle<'_, T>, Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.register_loader)(ptr, r#type, loader)
            .map(|id| unsafe { LoaderHandle::from_raw(id, loader, self) })
    }

    /// Unregisters an existing module loader from the `ModuleRegistry`.
    ///
    /// Notifies all registered callbacks before returning.
    #[inline]
    pub fn unregister_loader<T: ModuleLoader>(
        &self,
        loader: LoaderHandle<'_, T>,
    ) -> Result<&'static T, Box<dyn Error>> {
        let (id, loader, _) = loader.into_raw();
        self.unregister_loader_inner(id).map(|_| loader)
    }

    #[inline]
    fn unregister_loader_inner(
        &self,
        id: LoaderId,
    ) -> Result<&'static dyn ModuleLoader, Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.unregister_loader)(ptr, id)
    }

    /// Registers a loader-removal callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the loader is removed.
    #[inline]
    pub fn register_loader_callback<'a, F: FnOnce(&dyn ModuleLoader) + Send + Sync>(
        &'a self,
        r#type: &str,
        callback: Box<F>,
    ) -> Result<LoaderCallbackHandle<'a>, Box<dyn Error>> {
        let callback = LoaderCallback::from(callback);
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.register_loader_callback)(ptr, r#type, callback)
            .map(|id| unsafe { LoaderCallbackHandle::from_raw_parts(id, self) })
    }

    /// Unregisters a loader-removal callback from the `ModuleRegistry`.
    ///
    /// The callback will not be called.
    #[inline]
    pub fn unregister_loader_callback(
        &self,
        handle: LoaderCallbackHandle<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let (id, _) = handle.into_raw_parts();
        self.unregister_loader_callback_inner(id)
    }

    #[inline]
    fn unregister_loader_callback_inner(&self, id: LoaderCallbackId) -> Result<(), Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.unregister_loader_callback)(ptr, id)
    }

    /// Fetches the loader associated with the `type`.
    #[inline]
    pub fn get_loader_from_type(
        &self,
        r#type: &str,
    ) -> Result<&'static dyn ModuleLoader, Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_loader_from_type)(ptr, r#type)
    }

    /// Registers a new interface to the `ModuleRegistry`.
    #[inline]
    pub fn register_interface<T: ModuleInterface + Unsize<dyn ModuleInterface> + ?Sized>(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: Arc<T>,
    ) -> Result<InterfaceHandle<'_, T>, Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.register_interface)(ptr, descriptor, interface.clone())
            .map(|id| unsafe { InterfaceHandle::from_raw_parts(id, interface, self) })
    }

    /// Unregisters an existing interface from the `ModuleRegistry`.
    ///
    /// This function calls the interface-remove callbacks that are registered
    /// with the interface before removing it.
    #[inline]
    pub fn unregister_interface<T: ModuleInterface + ?Sized>(
        &self,
        handle: InterfaceHandle<'_, T>,
    ) -> Result<Arc<T>, Box<dyn Error>> {
        let (id, i, _) = handle.into_raw_parts();
        self.unregister_interface_inner(id).map(|_| i)
    }

    #[inline]
    fn unregister_interface_inner(
        &self,
        id: InterfaceId,
    ) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.unregister_interface)(ptr, id)
    }

    /// Registers an interface-removed callback to the `ModuleRegistry`.
    ///
    /// The callback will be called in case the interface is removed from the `ModuleRegistry`.
    #[inline]
    pub fn register_interface_callback<F: FnOnce(Arc<dyn ModuleInterface>) + Send + Sync>(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: Box<F>,
    ) -> Result<InterfaceCallbackHandle<'_>, Box<dyn Error>> {
        let callback = InterfaceCallback::from(callback);
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.register_interface_callback)(ptr, descriptor, callback)
            .map(|id| unsafe { InterfaceCallbackHandle::from_raw_parts(id, self) })
    }

    /// Unregisters an interface-removed callback from the `ModuleRegistry` without calling it.
    #[inline]
    pub fn unregister_interface_callback(
        &self,
        handle: InterfaceCallbackHandle<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let (id, _) = handle.into_raw_parts();
        self.unregister_interface_callback_inner(id)
    }

    #[inline]
    fn unregister_interface_callback_inner(
        &self,
        id: InterfaceCallbackId,
    ) -> Result<(), Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.unregister_interface_callback)(ptr, id)
    }

    /// Extracts an interface from the `ModuleRegistry`.
    #[inline]
    pub fn get_interface_from_descriptor(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>> {
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
        extensions: &[ArrayString<32>],
    ) -> Vec<ModuleInterfaceDescriptor> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_compatible_interface_descriptors)(ptr, name, version, extensions)
    }

    /// Splits the reference into a data- and vtable- pointer.
    #[inline]
    pub fn into_raw_parts(&self) -> (*const (), &'static ModuleRegistryVTable) {
        // safety: `&Self` has the same layout as `&[()]`
        let s: &[()] = unsafe { std::mem::transmute(self) };

        // safety: the values are properly initialized upon construction.
        let ptr = s.as_ptr();
        let vtable = unsafe { &*(s.len() as *const ModuleRegistryVTable) };

        (ptr, vtable)
    }

    /// Constructs a `*const ModuleRegistry` from a data- and vtable- pointer.
    #[inline]
    pub fn from_raw_parts(data: *const (), vtable: &'static ModuleRegistryVTable) -> *const Self {
        // `()` has size 0 and alignment 1, so it should be sound to use an
        // arbitrary ptr and length.
        let vtable_ptr = vtable as *const _ as usize;
        let s = std::ptr::slice_from_raw_parts(data, vtable_ptr);

        // safety: the types have the same layout
        unsafe { std::mem::transmute(s) }
    }
}

impl std::fmt::Debug for ModuleRegistry {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(ModuleRegistry)")
    }
}

unsafe impl Send for ModuleRegistry {}
unsafe impl Sync for ModuleRegistry {}

/// VTable of the [`ModuleRegistry`] type.
#[repr(C)]
#[allow(clippy::type_complexity)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ModuleRegistryVTable {
    register_loader:
        fn(*const (), *const str, &'static dyn ModuleLoader) -> Result<LoaderId, Box<dyn Error>>,
    unregister_loader: fn(*const (), LoaderId) -> Result<&'static dyn ModuleLoader, Box<dyn Error>>,
    register_loader_callback:
        fn(*const (), *const str, LoaderCallback) -> Result<LoaderCallbackId, Box<dyn Error>>,
    unregister_loader_callback: fn(*const (), LoaderCallbackId) -> Result<(), Box<dyn Error>>,
    get_loader_from_type:
        fn(*const (), *const str) -> Result<&'static dyn ModuleLoader, Box<dyn Error>>,
    register_interface: fn(
        *const (),
        *const ModuleInterfaceDescriptor,
        Arc<dyn ModuleInterface>,
    ) -> Result<InterfaceId, Box<dyn Error>>,
    unregister_interface:
        fn(*const (), InterfaceId) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>>,
    register_interface_callback: fn(
        *const (),
        *const ModuleInterfaceDescriptor,
        InterfaceCallback,
    ) -> Result<InterfaceCallbackId, Box<dyn Error>>,
    unregister_interface_callback: fn(*const (), InterfaceCallbackId) -> Result<(), Box<dyn Error>>,
    get_interface_from_descriptor: fn(
        *const (),
        *const ModuleInterfaceDescriptor,
    ) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>>,
    get_interface_descriptors_from_name:
        fn(*const (), *const str) -> Vec<ModuleInterfaceDescriptor>,
    get_compatible_interface_descriptors: fn(
        *const (),
        *const str,
        *const Version,
        *const [ArrayString<32>],
    ) -> Vec<ModuleInterfaceDescriptor>,
}

impl ModuleRegistryVTable {
    /// Constructs a new `ModuleRegistryVTable`.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub const fn new(
        register_loader: fn(
            *const (),
            *const str,
            &'static dyn ModuleLoader,
        ) -> Result<LoaderId, Box<dyn Error>>,
        unregister_loader: fn(
            *const (),
            LoaderId,
        ) -> Result<&'static dyn ModuleLoader, Box<dyn Error>>,
        register_loader_callback: fn(
            *const (),
            *const str,
            LoaderCallback,
        ) -> Result<LoaderCallbackId, Box<dyn Error>>,
        unregister_loader_callback: fn(*const (), LoaderCallbackId) -> Result<(), Box<dyn Error>>,
        get_loader_from_type: fn(
            *const (),
            *const str,
        ) -> Result<&'static dyn ModuleLoader, Box<dyn Error>>,
        register_interface: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
            Arc<dyn ModuleInterface>,
        ) -> Result<InterfaceId, Box<dyn Error>>,
        unregister_interface: fn(
            *const (),
            InterfaceId,
        ) -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>>,
        register_interface_callback: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
            InterfaceCallback,
        ) -> Result<InterfaceCallbackId, Box<dyn Error>>,
        unregister_interface_callback: fn(
            *const (),
            InterfaceCallbackId,
        ) -> Result<(), Box<dyn Error>>,
        get_interface_from_descriptor: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
        )
            -> Result<Arc<dyn ModuleInterface>, Box<dyn Error>>,
        get_interface_descriptors_from_name: fn(
            *const (),
            *const str,
        ) -> Vec<ModuleInterfaceDescriptor>,
        get_compatible_interface_descriptors: fn(
            *const (),
            *const str,
            *const Version,
            *const [ArrayString<32>],
        ) -> Vec<ModuleInterfaceDescriptor>,
    ) -> Self {
        Self {
            register_loader,
            unregister_loader,
            register_loader_callback,
            unregister_loader_callback,
            get_loader_from_type,
            register_interface,
            unregister_interface,
            register_interface_callback,
            unregister_interface_callback,
            get_interface_from_descriptor,
            get_interface_descriptors_from_name,
            get_compatible_interface_descriptors,
        }
    }
}

/// Handle to a loader.
#[derive(Debug)]
pub struct LoaderHandle<'a, T: ModuleLoader + 'static> {
    id: LoaderId,
    loader: &'static T,
    registry: &'a ModuleRegistry,
}

impl<'a, T: ModuleLoader + 'static> LoaderHandle<'a, T> {
    /// Constructs a new `LoaderHandle` from from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that `T` is matches with the
    /// original type.
    #[inline]
    pub unsafe fn from_raw(id: LoaderId, loader: &'static T, registry: &'a ModuleRegistry) -> Self {
        Self {
            id,
            loader,
            registry,
        }
    }

    /// Splits the `LoaderHandle` into its raw components.
    #[inline]
    pub fn into_raw(self) -> (LoaderId, &'static T, &'a ModuleRegistry) {
        let id = unsafe { std::ptr::read(&self.id) };
        let loader = unsafe { std::ptr::read(&self.loader) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, loader, registry)
    }
}

impl<'a, T: ModuleLoader> Deref for LoaderHandle<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &'static Self::Target {
        self.loader
    }
}

impl<T: ModuleLoader> Drop for LoaderHandle<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // safety: `LoaderId` is a simple `usize`.
        let id = unsafe { std::ptr::read(&self.id) };
        let _ = self.registry.unregister_loader_inner(id);
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
pub struct InterfaceHandle<'a, T: ModuleInterface + ?Sized> {
    id: InterfaceId,
    interface: Arc<T>,
    registry: &'a ModuleRegistry,
    _phantom: PhantomData<fn() -> *const T>,
}

impl<'a, T: ModuleInterface + ?Sized> InterfaceHandle<'a, T> {
    /// Constructs a new `InterfaceHandle` from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that `T` is matches with the
    /// original type.
    #[inline]
    pub unsafe fn from_raw_parts(
        id: InterfaceId,
        interface: Arc<T>,
        registry: &'a ModuleRegistry,
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
    pub fn into_raw_parts(self) -> (InterfaceId, Arc<T>, &'a ModuleRegistry) {
        let id = unsafe { std::ptr::read(&self.id) };
        let interface = unsafe { std::ptr::read(&self.interface) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, interface, registry)
    }
}

impl<'a, T: ModuleInterface + ?Sized> Deref for InterfaceHandle<'a, T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.interface
    }
}

impl<T: ModuleInterface + ?Sized> Drop for InterfaceHandle<'_, T> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        let _ = self.registry.unregister_interface_inner(id);
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
    registry: &'a ModuleRegistry,
}

impl<'a> LoaderCallbackHandle<'a> {
    /// Splits a `LoaderCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (LoaderCallbackId, &'a ModuleRegistry) {
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
    pub unsafe fn from_raw_parts(id: LoaderCallbackId, registry: &'a ModuleRegistry) -> Self {
        Self { id, registry }
    }
}

impl Drop for LoaderCallbackHandle<'_> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        let _ = self.registry.unregister_loader_callback_inner(id);
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
    inner: HeapFnOnce<(&'static dyn ModuleLoader,), ()>,
}

impl FnOnce<(&'static dyn ModuleLoader,)> for LoaderCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(self, args: (&'static dyn ModuleLoader,)) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl<F: FnOnce(&'static dyn ModuleLoader) + Send + Sync> From<Box<F>> for LoaderCallback {
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
    registry: &'a ModuleRegistry,
}

impl<'a> InterfaceCallbackHandle<'a> {
    /// Splits a `InterfaceCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (InterfaceCallbackId, &'a ModuleRegistry) {
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
    pub unsafe fn from_raw_parts(id: InterfaceCallbackId, registry: &'a ModuleRegistry) -> Self {
        Self { id, registry }
    }
}

impl Drop for InterfaceCallbackHandle<'_> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        let _ = self.registry.unregister_interface_callback_inner(id);
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
    inner: HeapFnOnce<(Arc<dyn ModuleInterface>,), ()>,
}

impl FnOnce<(Arc<dyn ModuleInterface>,)> for InterfaceCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(self, args: (Arc<dyn ModuleInterface>,)) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl<F: FnOnce(Arc<dyn ModuleInterface>) + Send + Sync> From<Box<F>> for InterfaceCallback {
    #[inline]
    fn from(data: Box<F>) -> Self {
        Self {
            inner: HeapFnOnce::new_boxed(data),
        }
    }
}

unsafe impl Send for InterfaceCallback {}
unsafe impl Sync for InterfaceCallback {}
