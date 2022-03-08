//! Specification of a settings registry.

mod item;
mod path;

pub use item::*;
pub use path::*;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;

use fimo_module::fimo_ffi::{interface, DynObj, FfiFn};

/// Type-erased settings registry.
#[interface(
    uuid = "b8b6d47f-6a26-489c-a7c9-b86a993dcb60",
    vtable = "ISettingsRegistryVTable",
    generate()
)]
pub trait ISettingsRegistry: Send + Sync {
    /// Enters the inner registry possibly locking it.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    fn enter_impl(
        &self,
        f: FfiFn<'_, dyn FnOnce(&'_ DynObj<dyn ISettingsRegistryInner + '_>) + '_>,
    );

    /// Enters the inner registry possibly locking it mutably.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    fn enter_mut_impl(
        &self,
        f: FfiFn<'_, dyn FnOnce(&'_ mut DynObj<dyn ISettingsRegistryInner + '_>) + '_>,
    );
}

/// Extension trait for implementations of [`ISettingsRegistry`].
pub trait ISettingsRegistryExt: ISettingsRegistry {
    /// Enters the inner registry possibly locking it.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    #[inline]
    fn enter<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&'_ DynObj<dyn ISettingsRegistryInner + '_>) -> R,
    {
        let mut res = MaybeUninit::uninit();
        let f = |inner: &'_ DynObj<dyn ISettingsRegistryInner + '_>| {
            res.write(f(inner));
        };
        let mut f = MaybeUninit::new(f);
        let f = unsafe { FfiFn::new_value(&mut f) };
        self.enter_impl(f);

        // safety: f either initializes the result or panicked and this point won't be reached.
        unsafe { res.assume_init() }
    }

    /// Enters the inner registry possibly locking it mutably.
    ///
    /// # Deadlock
    ///
    /// The function may only call into the registry with the provided inner reference.
    #[inline]
    fn enter_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&'_ mut DynObj<dyn ISettingsRegistryInner + '_>) -> R,
    {
        let mut res = MaybeUninit::uninit();
        let f = |inner: &'_ mut DynObj<dyn ISettingsRegistryInner + '_>| {
            res.write(f(inner));
        };
        let mut f = MaybeUninit::new(f);
        let f = unsafe { FfiFn::new_value(&mut f) };
        self.enter_mut_impl(f);

        // safety: f either initializes the result or panicked and this point won't be reached.
        unsafe { res.assume_init() }
    }

    /// Checks whether an item is contained in the registry.
    #[inline]
    fn contains<P: AsRef<SettingsPath>>(&self, path: P) -> Result<bool, SettingsInvalidPathError> {
        self.enter(move |inner| inner.contains(path.as_ref()))
    }

    /// Extracts the type of an item.
    #[inline]
    fn item_type<P: AsRef<SettingsPath>>(
        &self,
        path: P,
    ) -> Result<Option<SettingsItemType>, SettingsInvalidPathError> {
        self.enter(move |inner| inner.item_type(path.as_ref()))
    }

    /// Extracts an item from the `SettingsRegistry`.
    #[inline]
    fn read<T: TryFrom<SettingsItem>, P: AsRef<SettingsPath>>(
        &self,
        path: P,
    ) -> Result<Option<T>, SettingsRegistryError<T>> {
        self.enter(move |inner| {
            let item = match inner.read(path.as_ref()) {
                Ok(Some(i)) => i,
                Ok(None) => return Ok(None),
                Err(e) => {
                    return Err(SettingsRegistryError::PathError(e));
                }
            };

            item.try_into()
                .map(|i| Some(i))
                .map_err(|e| SettingsRegistryError::CastError(e))
        })
    }

    /// Writes into the `SettingsRegistry`.
    ///
    /// This function either overwrites an existing item or creates a new one.
    /// Afterwards the old value is extracted.
    #[inline]
    fn write<T: Into<SettingsItem>, P: AsRef<SettingsPath>>(
        &self,
        path: P,
        item: T,
    ) -> Result<Option<SettingsItem>, SettingsInvalidPathError> {
        self.enter_mut(move |inner| inner.write(path.as_ref(), item.into()))
    }

    /// Reads a copy of the entire registry.
    ///
    /// Equivalent to calling [`ISettingsRegistryExt::read`] with [`SettingsPath::root`].
    #[inline]
    fn read_all(&self) -> BTreeMap<fimo_ffi::String, SettingsItem> {
        self.read(SettingsPath::root()).unwrap().unwrap()
    }

    /// Overwrites the root object of the `SettingsRegistry` and returns
    /// the original map.
    ///
    /// Equivalent to calling [`ISettingsRegistryExt::write`] with [`SettingsPath::root`].
    #[inline]
    fn write_all(
        &self,
        value: BTreeMap<fimo_ffi::String, SettingsItem>,
    ) -> BTreeMap<fimo_ffi::String, SettingsItem> {
        self.write(SettingsPath::root(), value)
            .unwrap()
            .unwrap()
            .try_into()
            .unwrap()
    }

    /// Reads or initializes an item from the `SettingsRegistry`.
    ///
    /// See [`ISettingsRegistryExt::read`] and [`ISettingsRegistryExt::write`].
    #[inline]
    fn read_or<T: TryFrom<SettingsItem> + Into<SettingsItem>, P: AsRef<SettingsPath>>(
        &self,
        path: P,
        item: T,
    ) -> Result<T, SettingsRegistryError<T>> {
        self.enter_mut(move |inner| {
            let item = match inner.read_or(path.as_ref(), item.into()) {
                Ok(i) => i,
                Err(e) => {
                    return Err(SettingsRegistryError::PathError(e));
                }
            };

            match item.try_into() {
                Ok(i) => Ok(i),
                Err(e) => Err(SettingsRegistryError::CastError(e)),
            }
        })
    }

    /// Removes an item from the `SettingsRegistry`.
    #[inline]
    fn remove<T: TryFrom<SettingsItem>, P: AsRef<SettingsPath>>(
        &self,
        path: P,
    ) -> Result<Option<T>, SettingsRegistryError<T>> {
        self.enter_mut(move |inner| {
            let item = match inner.remove(path.as_ref()) {
                Ok(Some(i)) => i,
                Ok(None) => return Ok(None),
                Err(e) => return Err(SettingsRegistryError::PathError(e)),
            };

            match item.try_into() {
                Ok(i) => Ok(Some(i)),
                Err(e) => Err(SettingsRegistryError::CastError(e)),
            }
        })
    }

    /// Removes an item from the `SettingsRegistry`.
    ///
    /// Convenience function for [`ISettingsRegistryExt::remove::<SettingsItem, P>`].
    #[inline]
    fn remove_item<P: AsRef<SettingsPath>>(
        &self,
        path: P,
    ) -> Result<Option<SettingsItem>, SettingsRegistryError<SettingsItem>> {
        self.remove::<SettingsItem, _>(path)
    }

    /// Registers a callback to an item.
    ///
    /// # Note
    ///
    /// The callback may only call into the registry via the provided reference.
    #[inline]
    fn register_callback<F, P: AsRef<SettingsPath>>(
        &self,
        path: P,
        f: F,
    ) -> Result<SettingsEventCallbackHandle<'_, Self>, SettingsInvalidPathError>
    where
        F: FnMut(&'_ DynObj<dyn ISettingsRegistryInner + '_>, &SettingsPath, SettingsEvent)
            + Send
            + 'static,
    {
        self.enter_mut(move |inner| {
            let f = SettingsEventCallback::r#box(Box::new(f));
            let id = inner.register_callback(path.as_ref(), f)?;
            unsafe { Ok(SettingsEventCallbackHandle::from_raw_parts(id, self)) }
        })
    }

    /// Unregisters a callback from an item.
    #[inline]
    fn unregister_callback(
        &self,
        handle: SettingsEventCallbackHandle<'_, Self>,
    ) -> fimo_module::Result<()> {
        self.enter_mut(move |inner| {
            let (id, _) = handle.into_raw_parts();
            inner.unregister_callback(id)
        })
    }
}

/// An error that can occur from an update operation.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum SettingsRegistryError<T: TryFrom<SettingsItem>> {
    /// An invalid path was used.
    PathError(SettingsInvalidPathError),
    /// The casting operation failed.
    CastError(T::Error),
}

impl<T: ISettingsRegistry + ?Sized> ISettingsRegistryExt for T {}

/// Type-erased settings registry.
#[interface(
    uuid = "824e6374-cb96-4177-a08b-03aee57ad246",
    vtable = "ISettingsRegistryInnerVTable",
    generate()
)]
pub trait ISettingsRegistryInner: Send + Sync {
    /// Checks whether an item is contained in the registry.
    fn contains(&self, path: &SettingsPath) -> Result<bool, SettingsInvalidPathError>;

    /// Extracts the type of an item.
    fn item_type(
        &self,
        path: &SettingsPath,
    ) -> Result<Option<SettingsItemType>, SettingsInvalidPathError>;

    /// Extracts an item from the `SettingsRegistry`.
    fn read(&self, path: &SettingsPath) -> Result<Option<SettingsItem>, SettingsInvalidPathError>;

    /// Writes into the `SettingsRegistry`.
    ///
    /// This function either overwrites an existing item or creates a new one.
    /// Afterwards the old value is extracted.
    fn write(
        &mut self,
        path: &SettingsPath,
        item: SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsInvalidPathError>;

    /// Reads or initializes an item from the `SettingsRegistry`.
    ///
    /// See [`ISettingsRegistryInner::read`] and [`ISettingsRegistryInner::write`].
    fn read_or(
        &mut self,
        path: &SettingsPath,
        default: SettingsItem,
    ) -> Result<SettingsItem, SettingsInvalidPathError>;

    /// Removes an item from the `SettingsRegistry`.
    fn remove(
        &mut self,
        path: &SettingsPath,
    ) -> Result<Option<SettingsItem>, SettingsInvalidPathError>;

    /// Registers a callback to an item.
    ///
    /// # Note
    ///
    /// The callback may only call into the registry via the provided reference.
    fn register_callback(
        &mut self,
        path: &SettingsPath,
        f: SettingsEventCallback,
    ) -> Result<SettingsEventCallbackId, SettingsInvalidPathError>;

    /// Unregisters a callback from an item.
    fn unregister_callback(&mut self, id: SettingsEventCallbackId) -> fimo_module::Result<()>;
}

/// An event callback with read access to the registry.
pub type SettingsEventCallback = FfiFn<
    'static,
    dyn FnMut(&'_ DynObj<dyn ISettingsRegistryInner + '_>, &SettingsPath, SettingsEvent) + Send,
>;

/// A RAII guard for event callbacks.
pub struct SettingsEventCallbackHandle<'a, R: ISettingsRegistry + ?Sized> {
    id: SettingsEventCallbackId,
    registry: &'a R,
}

impl<'a, R: ISettingsRegistry + ?Sized> SettingsEventCallbackHandle<'a, R> {
    /// Splits a `SettingsEventCallbackHandle` into its raw components.
    #[inline]
    pub fn into_raw_parts(self) -> (SettingsEventCallbackId, &'a R) {
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
    pub unsafe fn from_raw_parts(id: SettingsEventCallbackId, registry: &'a R) -> Self {
        Self { id, registry }
    }
}

impl<R: ISettingsRegistry + ?Sized> Debug for SettingsEventCallbackHandle<'_, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SettingsEventCallbackHandle")
            .field(&self.id)
            .finish()
    }
}

impl<R: ISettingsRegistry + ?Sized> Drop for SettingsEventCallbackHandle<'_, R> {
    fn drop(&mut self) {
        let id = unsafe { std::ptr::read(&self.id) };
        self.registry.enter_mut(move |inner| {
            inner
                .unregister_callback(id)
                .expect("Can't drop the callback handle");
        })
    }
}

/// Id of a setting event callback.
#[derive(Debug)]
#[repr(transparent)]
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

/// Event types from the settings registry.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum SettingsEvent {
    /// An item was removed.
    Removed,
    /// An item was updated or created.
    Updated,
}
