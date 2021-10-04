//! Specification of a module registry.
use fimo_ffi_core::fn_wrapper::{HeapFnOnce, RawFnOnce};
use fimo_ffi_core::ArrayString;
use fimo_module_core::rust::ModuleInterfaceCaster;
use fimo_module_core::{
    rust::{ModuleInterface, ModuleInterfaceArc, ModuleLoader},
    DynArc, DynArcCaster, ModuleInterfaceDescriptor,
};
use fimo_version_core::Version;
use std::borrow::Borrow;
use std::error::Error;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;

/// Type-erased module registry.
///
/// The underlying type must implement `Send` and `Sync`.
pub struct ModuleRegistry {
    _inner: [()],
}

impl ModuleRegistry {
    /// Enters the inner registry.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    #[inline]
    pub fn enter_inner<F: FnOnce(&'_ mut ModuleRegistryInner)>(&self, f: F) {
        let mut wrapper = move |inner: *mut ModuleRegistryInner| {
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
    pub fn register_loader<T: Borrow<ModuleLoader>>(
        &self,
        r#type: &str,
        loader: &'static T,
    ) -> Result<LoaderHandle<'_, T>, Box<dyn Error>> {
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
    pub fn unregister_loader<T: Borrow<ModuleLoader>>(
        &self,
        loader: LoaderHandle<'_, T>,
    ) -> Result<&'static T, Box<dyn Error>> {
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
        'a,
        F: FnOnce(&mut ModuleRegistryInner, &'static ModuleLoader) + Send + Sync,
    >(
        &'a self,
        r#type: &str,
        callback: F,
    ) -> Result<LoaderCallbackHandle<'a>, Box<dyn Error>> {
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
    ) -> Result<(), Box<dyn Error>> {
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
    pub fn get_loader_from_type(
        &self,
        r#type: &str,
    ) -> Result<&'static ModuleLoader, Box<dyn Error>> {
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
    ///
    /// # Panic
    ///
    /// Panics if the implementation of [`Borrow`] does not return the same reference.
    #[inline]
    pub fn register_interface<T: Borrow<ModuleInterface> + ?Sized, C: DynArcCaster<T>>(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: DynArc<T, C>,
    ) -> Result<InterfaceHandle<'_, T, C>, Box<dyn Error>> {
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
    pub fn unregister_interface<T: Borrow<ModuleInterface> + ?Sized, C: DynArcCaster<T>>(
        &self,
        handle: InterfaceHandle<'_, T, C>,
    ) -> Result<DynArc<T, C>, Box<dyn Error>> {
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
        F: FnOnce(&mut ModuleRegistryInner, ModuleInterfaceArc) + Send + Sync,
    >(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: F,
    ) -> Result<InterfaceCallbackHandle<'_>, Box<dyn Error>> {
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
    ) -> Result<(), Box<dyn Error>> {
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
    ) -> Result<ModuleInterfaceArc, Box<dyn Error>> {
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
        extensions: &[ArrayString<32>],
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
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ModuleRegistryVTable {
    enter_inner: fn(*const (), RawFnOnce<(*mut ModuleRegistryInner,), ()>),
}

impl ModuleRegistryVTable {
    /// Constructs a new `ModuleRegistryVTable`.
    pub const fn new(
        enter_inner: fn(*const (), RawFnOnce<(*mut ModuleRegistryInner,), ()>),
    ) -> Self {
        Self { enter_inner }
    }
}

/// Type-erased module registry.
///
/// The underlying type must implement `Send` and `Sync`.
pub struct ModuleRegistryInner {
    _inner: [()],
}

impl ModuleRegistryInner {
    /// Registers a new module loader to the `ModuleRegistry`.
    ///
    /// The registered loader will be available to the rest of the `ModuleRegistry`.
    #[inline]
    pub fn register_loader<T: Borrow<ModuleLoader>>(
        &mut self,
        r#type: &str,
        loader: &'static T,
    ) -> Result<LoaderId, Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.register_loader)(ptr, r#type, loader.borrow())
    }

    /// Unregisters an existing module loader from the `ModuleRegistry`.
    ///
    /// Notifies all registered callbacks before returning.
    #[inline]
    pub fn unregister_loader(
        &mut self,
        id: LoaderId,
    ) -> Result<&'static ModuleLoader, Box<dyn Error>> {
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
        F: FnOnce(&mut ModuleRegistryInner, &'static ModuleLoader) + Send + Sync,
    >(
        &mut self,
        r#type: &str,
        callback: F,
    ) -> Result<LoaderCallbackId, Box<dyn Error>> {
        let wrapper = Box::new(move |inner: *mut ModuleRegistryInner, loader| unsafe {
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
    pub fn unregister_loader_callback(
        &mut self,
        id: LoaderCallbackId,
    ) -> Result<(), Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.unregister_loader_callback)(ptr, id)
    }

    /// Fetches the loader associated with the `type`.
    #[inline]
    pub fn get_loader_from_type(
        &self,
        r#type: &str,
    ) -> Result<&'static ModuleLoader, Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.get_loader_from_type)(ptr, r#type)
    }

    /// Registers a new interface to the `ModuleRegistry`.
    ///
    /// # Panic
    ///
    /// Panics if the implementation of [`Borrow`] does not return the same reference.
    #[inline]
    pub fn register_interface<T: Borrow<ModuleInterface> + ?Sized, C: DynArcCaster<T>>(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        interface: DynArc<T, C>,
    ) -> Result<InterfaceId, Box<dyn Error>> {
        let (ptr, vtable) = (*interface).borrow().into_raw_parts();
        assert_eq!(ptr, &*interface as *const _ as *const ());

        let (base, _) = DynArc::into_inner(interface);
        let caster = ModuleInterfaceCaster::new(vtable);
        let interface = unsafe { ModuleInterfaceArc::from_inner((base, caster)) };

        let (ptr, vtable) = self.into_raw_parts_mut();
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
    ) -> Result<ModuleInterfaceArc, Box<dyn Error>> {
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
        F: FnOnce(&mut ModuleRegistryInner, ModuleInterfaceArc) + Send + Sync,
    >(
        &mut self,
        descriptor: &ModuleInterfaceDescriptor,
        callback: F,
    ) -> Result<InterfaceCallbackId, Box<dyn Error>> {
        let wrapper = Box::new(move |inner: *mut ModuleRegistryInner, interface| unsafe {
            callback(&mut *inner, interface)
        });

        let callback = InterfaceCallback::from(wrapper);
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.register_interface_callback)(ptr, descriptor, callback)
    }

    /// Unregisters an interface-removed callback from the `ModuleRegistry` without calling it.
    #[inline]
    pub fn unregister_interface_callback(
        &mut self,
        id: InterfaceCallbackId,
    ) -> Result<(), Box<dyn Error>> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        (vtable.unregister_interface_callback)(ptr, id)
    }

    /// Extracts an interface from the `ModuleRegistry`.
    #[inline]
    pub fn get_interface_from_descriptor(
        &self,
        descriptor: &ModuleInterfaceDescriptor,
    ) -> Result<ModuleInterfaceArc, Box<dyn Error>> {
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
    pub fn into_raw_parts(&self) -> (*const (), &'static ModuleRegistryInnerVTable) {
        // safety: `&Self` has the same layout as `&[()]`
        let s: &[()] = unsafe { std::mem::transmute(self) };

        // safety: the values are properly initialized upon construction.
        let ptr = s.as_ptr();
        let vtable = unsafe { &*(s.len() as *const ModuleRegistryInnerVTable) };

        (ptr, vtable)
    }

    /// Splits the reference into a data- and vtable- pointer.
    #[inline]
    pub fn into_raw_parts_mut(&mut self) -> (*mut (), &'static ModuleRegistryInnerVTable) {
        // safety: `&Self` has the same layout as `&[()]`
        let s: &mut [()] = unsafe { std::mem::transmute(self) };

        // safety: the values are properly initialized upon construction.
        let ptr = s.as_mut_ptr();
        let vtable = unsafe { &*(s.len() as *const ModuleRegistryInnerVTable) };

        (ptr, vtable)
    }

    /// Constructs a `*const ModuleRegistryInner` from a data- and vtable- pointer.
    #[inline]
    pub fn from_raw_parts(
        data: *const (),
        vtable: &'static ModuleRegistryInnerVTable,
    ) -> *const Self {
        // `()` has size 0 and alignment 1, so it should be sound to use an
        // arbitrary ptr and length.
        let vtable_ptr = vtable as *const _ as usize;
        let s = std::ptr::slice_from_raw_parts(data, vtable_ptr);

        // safety: the types have the same layout
        unsafe { std::mem::transmute(s) }
    }

    /// Constructs a `*mut ModuleRegistryInner` from a data- and vtable- pointer.
    #[inline]
    pub fn from_raw_parts_mut(
        data: *mut (),
        vtable: &'static ModuleRegistryInnerVTable,
    ) -> *mut Self {
        // `()` has size 0 and alignment 1, so it should be sound to use an
        // arbitrary ptr and length.
        let vtable_ptr = vtable as *const _ as usize;
        let s = std::ptr::slice_from_raw_parts_mut(data, vtable_ptr);

        // safety: the types have the same layout
        unsafe { std::mem::transmute(s) }
    }
}

impl std::fmt::Debug for ModuleRegistryInner {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(ModuleRegistry)")
    }
}

unsafe impl Send for ModuleRegistryInner {}
unsafe impl Sync for ModuleRegistryInner {}

/// VTable of the [`ModuleRegistryInner`] type.
#[repr(C)]
#[allow(clippy::type_complexity)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ModuleRegistryInnerVTable {
    register_loader:
        fn(*mut (), *const str, &'static ModuleLoader) -> Result<LoaderId, Box<dyn Error>>,
    unregister_loader: fn(*mut (), LoaderId) -> Result<&'static ModuleLoader, Box<dyn Error>>,
    register_loader_callback:
        fn(*mut (), *const str, LoaderCallback) -> Result<LoaderCallbackId, Box<dyn Error>>,
    unregister_loader_callback: fn(*mut (), LoaderCallbackId) -> Result<(), Box<dyn Error>>,
    get_loader_from_type:
        fn(*const (), *const str) -> Result<&'static ModuleLoader, Box<dyn Error>>,
    register_interface: fn(
        *mut (),
        *const ModuleInterfaceDescriptor,
        ModuleInterfaceArc,
    ) -> Result<InterfaceId, Box<dyn Error>>,
    unregister_interface: fn(*mut (), InterfaceId) -> Result<ModuleInterfaceArc, Box<dyn Error>>,
    register_interface_callback: fn(
        *mut (),
        *const ModuleInterfaceDescriptor,
        InterfaceCallback,
    ) -> Result<InterfaceCallbackId, Box<dyn Error>>,
    unregister_interface_callback: fn(*mut (), InterfaceCallbackId) -> Result<(), Box<dyn Error>>,
    get_interface_from_descriptor: fn(
        *const (),
        *const ModuleInterfaceDescriptor,
    ) -> Result<ModuleInterfaceArc, Box<dyn Error>>,
    get_interface_descriptors_from_name:
        fn(*const (), *const str) -> Vec<ModuleInterfaceDescriptor>,
    get_compatible_interface_descriptors: fn(
        *const (),
        *const str,
        *const Version,
        *const [ArrayString<32>],
    ) -> Vec<ModuleInterfaceDescriptor>,
}

impl ModuleRegistryInnerVTable {
    /// Constructs a new `ModuleRegistryVTable`.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub const fn new(
        register_loader: fn(
            *mut (),
            *const str,
            &'static ModuleLoader,
        ) -> Result<LoaderId, Box<dyn Error>>,
        unregister_loader: fn(*mut (), LoaderId) -> Result<&'static ModuleLoader, Box<dyn Error>>,
        register_loader_callback: fn(
            *mut (),
            *const str,
            LoaderCallback,
        ) -> Result<LoaderCallbackId, Box<dyn Error>>,
        unregister_loader_callback: fn(*mut (), LoaderCallbackId) -> Result<(), Box<dyn Error>>,
        get_loader_from_type: fn(
            *const (),
            *const str,
        ) -> Result<&'static ModuleLoader, Box<dyn Error>>,
        register_interface: fn(
            *mut (),
            *const ModuleInterfaceDescriptor,
            ModuleInterfaceArc,
        ) -> Result<InterfaceId, Box<dyn Error>>,
        unregister_interface: fn(
            *mut (),
            InterfaceId,
        ) -> Result<ModuleInterfaceArc, Box<dyn Error>>,
        register_interface_callback: fn(
            *mut (),
            *const ModuleInterfaceDescriptor,
            InterfaceCallback,
        ) -> Result<InterfaceCallbackId, Box<dyn Error>>,
        unregister_interface_callback: fn(
            *mut (),
            InterfaceCallbackId,
        ) -> Result<(), Box<dyn Error>>,
        get_interface_from_descriptor: fn(
            *const (),
            *const ModuleInterfaceDescriptor,
        ) -> Result<ModuleInterfaceArc, Box<dyn Error>>,
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
pub struct LoaderHandle<'a, T: Borrow<ModuleLoader> + 'static> {
    id: LoaderId,
    loader: &'static T,
    registry: &'a ModuleRegistry,
}

impl<'a, T: Borrow<ModuleLoader> + 'static> LoaderHandle<'a, T> {
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

impl<'a, T: Borrow<ModuleLoader>> Deref for LoaderHandle<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &'static Self::Target {
        self.loader
    }
}

impl<T: Borrow<ModuleLoader>> Drop for LoaderHandle<'_, T> {
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
pub struct InterfaceHandle<'a, T: Borrow<ModuleInterface> + ?Sized, C: DynArcCaster<T>> {
    id: InterfaceId,
    interface: DynArc<T, C>,
    registry: &'a ModuleRegistry,
    _phantom: PhantomData<fn() -> *const T>,
}

impl<'a, T: Borrow<ModuleInterface> + ?Sized, C: DynArcCaster<T>> InterfaceHandle<'a, T, C> {
    /// Constructs a new `InterfaceHandle` from its raw parts.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that `T` is matches with the
    /// original type.
    #[inline]
    pub unsafe fn from_raw_parts(
        id: InterfaceId,
        interface: DynArc<T, C>,
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
    pub fn into_raw_parts(self) -> (InterfaceId, DynArc<T, C>, &'a ModuleRegistry) {
        let id = unsafe { std::ptr::read(&self.id) };
        let interface = unsafe { std::ptr::read(&self.interface) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, interface, registry)
    }

    /// Clones the wrapped interface.
    #[inline]
    pub fn get_interface(&self) -> DynArc<T, C> {
        self.interface.clone()
    }
}

impl<'a, T: Borrow<ModuleInterface> + ?Sized, C: DynArcCaster<T>> Deref
    for InterfaceHandle<'a, T, C>
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.interface
    }
}

impl<T: Borrow<ModuleInterface> + ?Sized, C: DynArcCaster<T>> Drop for InterfaceHandle<'_, T, C> {
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
    inner: HeapFnOnce<(*mut ModuleRegistryInner, &'static ModuleLoader), ()>,
}

impl FnOnce<(*mut ModuleRegistryInner, &'static ModuleLoader)> for LoaderCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(
        self,
        args: (*mut ModuleRegistryInner, &'static ModuleLoader),
    ) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl<F: FnOnce(*mut ModuleRegistryInner, &'static ModuleLoader) + Send + Sync> From<Box<F>>
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
    inner: HeapFnOnce<(*mut ModuleRegistryInner, ModuleInterfaceArc), ()>,
}

impl FnOnce<(*mut ModuleRegistryInner, ModuleInterfaceArc)> for InterfaceCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(
        self,
        args: (*mut ModuleRegistryInner, ModuleInterfaceArc),
    ) -> Self::Output {
        self.inner.call_once(args)
    }
}

impl<F: FnOnce(*mut ModuleRegistryInner, ModuleInterfaceArc) + Send + Sync> From<Box<F>>
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
