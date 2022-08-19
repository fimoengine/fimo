//! Implementation of the `SettingsRegistry` type.
use fimo_core_int::settings::{
    ISettingsRegistry, ISettingsRegistryInner, SettingsEvent, SettingsEventCallback,
    SettingsEventCallbackId, SettingsInvalidPathError, SettingsItem, SettingsItemMetadata,
    SettingsItemType, SettingsPath, SettingsPathBuf,
};
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, FfiFn, Object};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::ops::RangeFrom;
use std::ptr::NonNull;

/// The settings registry.
#[derive(Debug, Object, StableTypeId)]
#[name("SettingsRegistry")]
#[uuid("4b43ec0b-04b6-4a2f-aa05-012b8be7dd2a")]
#[interfaces(ISettingsRegistry)]
pub struct SettingsRegistry {
    inner: parking_lot::RwLock<SettingsRegistryInner>,
}

impl SettingsRegistry {
    /// Constructs a new `SettingsRegistry`.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: parking_lot::RwLock::new(SettingsRegistryInner::new()),
        }
    }
}

impl Default for SettingsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ISettingsRegistry for SettingsRegistry {
    #[inline]
    fn enter_impl(
        &self,
        f: FfiFn<'_, dyn FnOnce(&'_ DynObj<dyn ISettingsRegistryInner + '_>) + '_>,
    ) {
        let inner = self.inner.read();
        f(fimo_ffi::ptr::coerce_obj(&*inner))
    }

    #[inline]
    fn enter_mut_impl(
        &self,
        f: FfiFn<'_, dyn FnOnce(&'_ mut DynObj<dyn ISettingsRegistryInner + '_>) + '_>,
    ) {
        let mut inner = self.inner.write();
        f(fimo_ffi::ptr::coerce_obj_mut(&mut *inner))
    }
}

#[derive(Debug)]
struct Metadata {
    registry: NonNull<SettingsRegistryInner>,
    callbacks: RefCell<HashMap<usize, SettingsEventCallback>>,
}

impl Metadata {
    #[inline]
    fn new(registry: &SettingsRegistryInner) -> Self {
        Self {
            registry: registry.into(),
            callbacks: RefCell::new(Default::default()),
        }
    }

    /// # Safety
    ///
    /// May only be called when a `&SettingsRegistryInner` is being held.
    #[inline]
    unsafe fn dispatch_event(&self, path: &SettingsPath, event: SettingsEvent) {
        // safety: A `Metadata` is only contained in an item of the SettingsRegistry
        // so we know that the object is alive. Further we need to ensure that
        // there isn't any aliasing mutable references. This should be sound because
        // there are only three paths which lead to calling this function.
        //
        // `SettingsRegistryInner::write(&mut)` -> `Item::write(&mut)` -> `Metadata::on_write(&self)` -> `Metadata::dispatch_event(&self)` -> `callback(&self)`
        // `SettingsRegistryInner::read_or(&mut)` -> `Item::read_or(&mut)` -> `Metadata::on_write(&self)` -> `Metadata::dispatch_event(&self)` -> `callback(&self)`
        // `SettingsRegistryInner::remove(&mut)` -> `Item::remove(&mut)` -> `Metadata::on_remove(&self)` -> `Metadata::dispatch_event(&self)` -> `callback(&self)`
        let registry = fimo_ffi::ptr::coerce_obj(self.registry.as_ref());
        for c in self.callbacks.borrow_mut().values_mut() {
            c(registry, path, event)
        }
    }

    #[inline]
    fn combine(&mut self, other: &mut Self) {
        let mut s = self.callbacks.borrow_mut();
        let mut o = other.callbacks.borrow_mut();

        self.registry = other.registry;
        for (id, c) in o.drain() {
            s.insert(id, c);
        }
    }
}

unsafe impl Sync for Metadata {}
unsafe impl Send for Metadata {}

impl Clone for Metadata {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            registry: NonNull::dangling(),
            callbacks: RefCell::new(Default::default()),
        }
    }
}

impl SettingsItemMetadata for Metadata {
    fn combine(&mut self, other: &mut Self) {
        self.combine(other)
    }

    fn on_write(&self, path: &SettingsPath) {
        // safety: any `&mut SettingsRegistryInner` must be coerced to an `&SettingsRegistryInner`
        // at this point, as
        unsafe { self.dispatch_event(path, SettingsEvent::Updated) }
    }

    fn on_remove(&self, path: &SettingsPath) {
        unsafe { self.dispatch_event(path, SettingsEvent::Removed) }
    }
}

#[derive(Debug, Object, StableTypeId)]
#[name("SettingsRegistryInner")]
#[uuid("66967a3a-0802-47e0-968f-df838c8e5651")]
#[interfaces(ISettingsRegistryInner)]
struct SettingsRegistryInner {
    root: SettingsItem<Metadata>,
    id_gen: RangeFrom<usize>,
    callback_map: BTreeMap<usize, SettingsPathBuf>,
}

impl SettingsRegistryInner {
    #[inline]
    fn new() -> Self {
        Self {
            root: SettingsItem::from(BTreeMap::<String, _>::new()),
            id_gen: (0..),
            callback_map: Default::default(),
        }
    }

    #[inline]
    fn contains(&self, path: &SettingsPath) -> Result<bool, SettingsInvalidPathError> {
        if path.is_root() {
            Ok(true)
        } else {
            self.root.contains(path)
        }
    }

    #[inline]
    fn item_type(
        &self,
        path: &SettingsPath,
    ) -> Result<Option<SettingsItemType>, SettingsInvalidPathError> {
        if path.is_root() {
            Ok(Some(self.root.item_type()))
        } else {
            self.root.get(path).map(|opt| opt.map(|i| i.item_type()))
        }
    }

    #[inline]
    fn read(&self, path: &SettingsPath) -> Result<Option<SettingsItem>, SettingsInvalidPathError> {
        if path.is_root() {
            Ok(Some(self.root.clone().map_metadata(|_| ())))
        } else {
            self.root
                .read(path)
                .map(|opt| opt.map(|i| i.map_metadata(|_| ())))
        }
    }

    #[inline]
    fn write(
        &mut self,
        path: &SettingsPath,
        value: SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsInvalidPathError> {
        // map the metadata of the item so that we can write it into the registry.
        let mut value = value.map_metadata(|_| Metadata::new(self));

        if path.is_root() {
            if !value.item_type().is_object() {
                Err(SettingsInvalidPathError::new(path))
            } else {
                // swap the root object with the new root object.
                std::mem::swap(&mut self.root, &mut value);

                // at this point `value` contains the old root and metadata and `item` the new one.
                // We want to retain the old metadata so we need to also swap them back.
                std::mem::swap(self.root.as_metadata_mut(), value.as_metadata_mut());
                Ok(Some(value.map_metadata(|_| ())))
            }
        } else {
            self.root
                .write(path, value)
                .map(|opt| opt.map(|i| i.map_metadata(|_| ())))
        }
    }

    #[inline]
    fn read_or(
        &mut self,
        path: &SettingsPath,
        value: SettingsItem,
    ) -> Result<SettingsItem, SettingsInvalidPathError> {
        if path.is_root() {
            Ok(self.root.clone().map_metadata(|_| ()))
        } else {
            self.root
                .read_or(path, value.map_metadata(|_| Metadata::new(self)))
                .map(|i| i.map_metadata(|_| ()))
        }
    }

    #[inline]
    fn remove(
        &mut self,
        path: &SettingsPath,
    ) -> Result<Option<SettingsItem>, SettingsInvalidPathError> {
        if path.is_root() {
            Err(SettingsInvalidPathError::new(path))
        } else {
            self.root
                .remove(path)
                .map(|opt| opt.map(|i| i.map_metadata(|_| ())))
        }
    }

    #[inline]
    fn register_callback(
        &mut self,
        path: &SettingsPath,
        f: SettingsEventCallback,
    ) -> Result<SettingsEventCallbackId, SettingsInvalidPathError> {
        let item = if path.is_root() {
            &mut self.root
        } else {
            self.root
                .get_mut(path)
                .transpose()
                .unwrap_or_else(|| Err(SettingsInvalidPathError::new(path)))?
        };

        let id = self.id_gen.next().expect("Ids exhausted");
        item.as_metadata_mut().callbacks.get_mut().insert(id, f);
        let c_id = unsafe { SettingsEventCallbackId::from_usize(id) };

        self.callback_map.insert(id, path.to_path_buf());
        Ok(c_id)
    }

    #[inline]
    fn unregister_callback(&mut self, id: SettingsEventCallbackId) -> fimo_module::Result<()> {
        let id = usize::from(id);
        let path = self.callback_map.remove(&id).ok_or_else(|| {
            fimo_ffi::error::Error::new(
                fimo_ffi::error::ErrorKind::NotFound,
                format!("invalid callback id {:?}", id),
            )
        })?;

        let item = if path.is_root() {
            Some(&mut self.root)
        } else {
            self.root.get_mut(path).unwrap_or(None)
        };

        if let Some(item) = item {
            item.as_metadata_mut().callbacks.get_mut().remove(&id);
        }

        Ok(())
    }
}

impl ISettingsRegistryInner for SettingsRegistryInner {
    fn contains(&self, path: &SettingsPath) -> Result<bool, SettingsInvalidPathError> {
        self.contains(path)
    }

    fn item_type(
        &self,
        path: &SettingsPath,
    ) -> Result<Option<SettingsItemType>, SettingsInvalidPathError> {
        self.item_type(path)
    }

    fn read(&self, path: &SettingsPath) -> Result<Option<SettingsItem>, SettingsInvalidPathError> {
        self.read(path)
    }

    fn write(
        &mut self,
        path: &SettingsPath,
        item: SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsInvalidPathError> {
        self.write(path, item)
    }

    fn read_or(
        &mut self,
        path: &SettingsPath,
        default: SettingsItem,
    ) -> Result<SettingsItem, SettingsInvalidPathError> {
        self.read_or(path, default)
    }

    fn remove(
        &mut self,
        path: &SettingsPath,
    ) -> Result<Option<SettingsItem>, SettingsInvalidPathError> {
        self.remove(path)
    }

    fn register_callback(
        &mut self,
        path: &SettingsPath,
        f: SettingsEventCallback,
    ) -> Result<SettingsEventCallbackId, SettingsInvalidPathError> {
        self.register_callback(path, f)
    }

    fn unregister_callback(&mut self, id: SettingsEventCallbackId) -> fimo_module::Result<()> {
        self.unregister_callback(id)
    }
}
