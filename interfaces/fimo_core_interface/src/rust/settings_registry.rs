//! Specification of a settings registry.
use fimo_ffi_core::fn_wrapper::HeapFnMut;
use fimo_module_core::rust::ModuleObject;
use std::collections::BTreeMap;
use std::convert::TryFrom;

mod settings_item;
mod settings_registry_path;

pub use settings_item::*;
pub use settings_registry_path::*;

/// Type-erased settings registry.
///
/// The underlying type must implement `Send` and `Sync`.
#[repr(transparent)]
pub struct SettingsRegistry {
    inner: ModuleObject<SettingsRegistryVTable>,
}

impl SettingsRegistry {
    /// Checks whether an item is contained.
    #[inline]
    pub fn contains<P: AsRef<SettingsRegistryPath>>(
        &self,
        path: P,
    ) -> Result<bool, SettingsRegistryInvalidPathError> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.contains)(ptr, path.as_ref())
    }

    /// Extracts the type of an item.
    #[inline]
    pub fn item_type<P: AsRef<SettingsRegistryPath>>(
        &self,
        path: P,
    ) -> Result<Option<SettingsItemType>, SettingsRegistryInvalidPathError> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.item_type)(ptr, path.as_ref())
    }

    /// Extracts an item from the `SettingsRegistry`.
    ///
    /// # Panics
    ///
    /// May panic if the item can not be cast.
    #[inline]
    pub fn read<T: TryFrom<SettingsItem>, P: AsRef<SettingsRegistryPath>>(
        &self,
        path: P,
    ) -> Result<Option<T>, SettingsRegistryInvalidPathError> {
        self.try_read::<T, P>(path)
            .map(|opt| opt.map(|i| i.unwrap_or_else(|_| panic!("invalid cast"))))
    }

    /// Extracts an item from the `SettingsRegistry`.
    ///
    /// Equivalent to calling [`SettingsRegistry::read::<SettingsItem>`] and mapping the result.
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn try_read<T: TryFrom<SettingsItem>, P: AsRef<SettingsRegistryPath>>(
        &self,
        path: P,
    ) -> Result<
        Option<Result<T, <T as TryFrom<SettingsItem>>::Error>>,
        SettingsRegistryInvalidPathError,
    > {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.read)(ptr, path.as_ref()).map(|opt| opt.map(T::try_from))
    }

    /// Reads a copy of the entire registry.
    ///
    /// Equivalent to calling [`SettingsRegistry::read`] with [`SettingsRegistryPath::root`].
    #[inline]
    pub fn read_all(&self) -> BTreeMap<String, SettingsItem> {
        self.read(SettingsRegistryPath::root()).unwrap().unwrap()
    }

    /// Writes into the `SettingsRegistry`.
    ///
    /// This function either overwrites an existing item or creates a new one.
    /// Afterwards the old value is extracted.
    #[inline]
    pub fn write<T: Into<SettingsItem>, P: AsRef<SettingsRegistryPath>>(
        &self,
        path: P,
        item: T,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.write)(ptr, path.as_ref(), item.into())
    }

    /// Overwrites the root object of the `SettingsRegistry` and returns
    /// the original map.
    ///
    /// Equivalent to calling [`SettingsRegistry::write`] with [`SettingsRegistryPath::root`].
    #[inline]
    pub fn write_all(
        &self,
        value: BTreeMap<String, SettingsItem>,
    ) -> BTreeMap<String, SettingsItem> {
        self.write(SettingsRegistryPath::root(), SettingsItem::from(value))
            .unwrap()
            .unwrap()
            .try_into()
            .unwrap()
    }

    /// Reads or initializes an item from the `SettingsRegistry`.
    ///
    /// See [`SettingsRegistry::read`] and [`SettingsRegistry::write`].
    ///
    /// # Panics
    ///
    /// May panic if the item can not be cast.
    #[inline]
    pub fn read_or<
        T: TryFrom<SettingsItem> + Into<SettingsItem>,
        P: AsRef<SettingsRegistryPath>,
    >(
        &self,
        path: P,
        item: T,
    ) -> Result<T, SettingsRegistryInvalidPathError> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.read_or)(ptr, path.as_ref(), item.into())
            .map(|i| T::try_from(i).unwrap_or_else(|_| panic!("invalid cast")))
    }

    /// Reads or initializes an item from the `SettingsRegistry`.
    ///
    /// Equivalent to calling [`SettingsRegistry::read_or`] and mapping the result.
    #[inline]
    pub fn try_read_or<
        T: TryFrom<SettingsItem> + Into<SettingsItem>,
        P: AsRef<SettingsRegistryPath>,
    >(
        &self,
        path: P,
        item: T,
    ) -> Result<Result<T, <T as TryFrom<SettingsItem>>::Error>, SettingsRegistryInvalidPathError>
    {
        self.read_or(path.as_ref(), item.into()).map(T::try_from)
    }

    /// Removes an item from the `SettingsRegistry`.
    #[inline]
    pub fn remove<P: AsRef<SettingsRegistryPath>>(
        &self,
        path: P,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.remove)(ptr, path.as_ref())
    }

    /// Registers a callback to an item.
    ///
    /// # Note
    ///
    /// The callback may not call into the `SettingsRegistry`.
    #[inline]
    pub fn register_callback<
        F: FnMut(&'_ SettingsRegistryPath, &'_ SettingsEvent) + Send,
        P: AsRef<SettingsRegistryPath>,
    >(
        &self,
        path: P,
        f: Box<F>,
    ) -> Option<SettingsEventCallbackHandle<'_>> {
        let f = SettingsEventCallback::from(f);
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.register_callback)(ptr, path.as_ref(), f)
            .map(|id| unsafe { SettingsEventCallbackHandle::from_raw_parts(id, self) })
    }

    /// Unregisters a callback from an item.
    #[inline]
    pub fn unregister_callback(&self, handle: SettingsEventCallbackHandle<'_>) {
        drop(handle);
    }

    #[inline]
    fn unregister_callback_inner(&self, id: SettingsEventCallbackId) {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.unregister_callback)(ptr, id)
    }

    /// Splits the reference into a data- and vtable- pointer.
    #[inline]
    pub fn into_raw_parts(&self) -> (*const (), &'static SettingsRegistryVTable) {
        self.inner.into_raw_parts()
    }

    /// Constructs a `*const ModuleRegistry` from a data- and vtable- pointer.
    #[inline]
    pub fn from_raw_parts(data: *const (), vtable: &'static SettingsRegistryVTable) -> *const Self {
        ModuleObject::from_raw_parts(data, vtable) as *const Self
    }
}

impl std::fmt::Debug for SettingsRegistry {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(SettingsRegistry)")
    }
}

unsafe impl Send for SettingsRegistry {}
unsafe impl Sync for SettingsRegistry {}

/// Error from using an invalid path.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SettingsRegistryInvalidPathError {
    path: SettingsRegistryPathBuf,
}

impl SettingsRegistryInvalidPathError {
    /// Constructs a new `SettingsRegistryPathNotFoundError`.
    #[inline]
    pub fn new<P: AsRef<SettingsRegistryPath>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Coerces `self` to a [`SettingsRegistryPath`] slice.
    #[inline]
    pub fn path(&self) -> &SettingsRegistryPath {
        self.path.as_path()
    }

    /// Consumes `self` and returns the contained [`SettingsRegistryPathBuf`].
    #[inline]
    pub fn to_path_buffer(self) -> SettingsRegistryPathBuf {
        self.path
    }
}

/// VTable of the [`SettingsRegistry`] type.
#[repr(C)]
#[allow(clippy::type_complexity)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SettingsRegistryVTable {
    contains: fn(
        *const (),
        *const SettingsRegistryPath,
    ) -> Result<bool, SettingsRegistryInvalidPathError>,
    item_type: fn(
        *const (),
        *const SettingsRegistryPath,
    ) -> Result<Option<SettingsItemType>, SettingsRegistryInvalidPathError>,
    read: fn(
        *const (),
        *const SettingsRegistryPath,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError>,
    write: fn(
        *const (),
        *const SettingsRegistryPath,
        SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError>,
    read_or: fn(
        *const (),
        *const SettingsRegistryPath,
        SettingsItem,
    ) -> Result<SettingsItem, SettingsRegistryInvalidPathError>,
    remove: fn(
        *const (),
        *const SettingsRegistryPath,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError>,
    register_callback: fn(
        *const (),
        *const SettingsRegistryPath,
        SettingsEventCallback,
    ) -> Option<SettingsEventCallbackId>,
    unregister_callback: fn(*const (), SettingsEventCallbackId),
}

impl SettingsRegistryVTable {
    /// Constructs a new `SettingsRegistryVTable`.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        contains: fn(
            *const (),
            *const SettingsRegistryPath,
        ) -> Result<bool, SettingsRegistryInvalidPathError>,
        item_type: fn(
            *const (),
            *const SettingsRegistryPath,
        )
            -> Result<Option<SettingsItemType>, SettingsRegistryInvalidPathError>,
        read: fn(
            *const (),
            *const SettingsRegistryPath,
        ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError>,
        write: fn(
            *const (),
            *const SettingsRegistryPath,
            SettingsItem,
        ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError>,
        read_or: fn(
            *const (),
            *const SettingsRegistryPath,
            SettingsItem,
        ) -> Result<SettingsItem, SettingsRegistryInvalidPathError>,
        remove: fn(
            *const (),
            *const SettingsRegistryPath,
        ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError>,
        register_callback: fn(
            *const (),
            *const SettingsRegistryPath,
            SettingsEventCallback,
        ) -> Option<SettingsEventCallbackId>,
        unregister_callback: fn(*const (), SettingsEventCallbackId),
    ) -> Self {
        Self {
            contains,
            item_type,
            read,
            write,
            read_or,
            remove,
            register_callback,
            unregister_callback,
        }
    }
}

/// Event types from the settings registry.
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum SettingsEvent {
    /// Item removed.
    ///
    /// # Note
    ///
    /// Is signaled after the item has been removed.
    Remove {
        /// Removed value.
        old: SettingsItem,
    },
    /// Signals the start of a `write` operation.
    ///
    /// # Note
    ///
    /// Is signaled before the new item is inserted.
    StartWrite {
        /// Value to be inserted.
        new: SettingsItem,
    },
    /// Signals the end of a `write` operation.
    ///
    /// # Note
    ///
    /// Is signaled after the new item is inserted.
    EndWrite {
        /// Old value.
        old: Option<SettingsItem>,
    },
    /// The write operation was aborted.
    AbortWrite,
}

/// A RAII guard for update callbacks.
#[derive(Debug)]
pub struct SettingsEventCallbackHandle<'a> {
    id: SettingsEventCallbackId,
    registry: &'a SettingsRegistry,
}

impl<'a> SettingsEventCallbackHandle<'a> {
    /// Splits a `SettingsEventCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (SettingsEventCallbackId, &'a SettingsRegistry) {
        let id = unsafe { std::ptr::read(&self.id) };
        let registry = self.registry;
        std::mem::forget(self);

        (id, registry)
    }

    /// Constructs a new `SettingsEventCallbackHandle` from its raw components.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_raw_parts(
        id: SettingsEventCallbackId,
        registry: &'a SettingsRegistry,
    ) -> Self {
        Self { id, registry }
    }
}

impl Drop for SettingsEventCallbackHandle<'_> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.unregister_callback_inner(id)
    }
}

/// Id of a setting event callback.
#[derive(Debug)]
pub struct SettingsEventCallbackId(usize);

impl SettingsEventCallbackId {
    /// Constructs a new `SettingsEventCallbackId` from an `usize`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee, that the id is valid.
    #[inline]
    pub unsafe fn from_usize(id: usize) -> Self {
        Self(id)
    }
}

impl From<SettingsEventCallbackId> for usize {
    #[inline]
    fn from(id: SettingsEventCallbackId) -> Self {
        id.0
    }
}

/// A loader removed callback.
#[derive(Debug)]
pub struct SettingsEventCallback {
    inner: HeapFnMut<(&'static SettingsRegistryPath, &'static SettingsEvent), ()>,
}

impl FnOnce<(&'_ SettingsRegistryPath, &'_ SettingsEvent)> for SettingsEventCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(
        mut self,
        args: (&'_ SettingsRegistryPath, &'_ SettingsEvent),
    ) -> Self::Output {
        self.call_mut(args)
    }
}

impl FnMut<(&'_ SettingsRegistryPath, &'_ SettingsEvent)> for SettingsEventCallback {
    #[inline]
    extern "rust-call" fn call_mut(
        &mut self,
        args: (&'_ SettingsRegistryPath, &'_ SettingsEvent),
    ) -> Self::Output {
        let args = unsafe { std::mem::transmute(args) };
        self.inner.call_mut(args)
    }
}

impl<F: FnMut(&'_ SettingsRegistryPath, &'_ SettingsEvent) + Send> From<Box<F>>
    for SettingsEventCallback
{
    #[inline]
    fn from(f: Box<F>) -> Self {
        let inner = HeapFnMut::new_boxed(f);

        Self {
            inner: unsafe { std::mem::transmute(inner) },
        }
    }
}

unsafe impl Send for SettingsEventCallback {}
unsafe impl Sync for SettingsEventCallback {}
