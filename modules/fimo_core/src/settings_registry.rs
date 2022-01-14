//! Implementation of the `SettingsRegistry` type.
use fimo_core_int::rust::settings_registry::{
    SettingsEvent, SettingsEventCallback, SettingsEventCallbackId, SettingsItem,
    SettingsItemMetadata, SettingsItemType, SettingsRegistryInvalidPathError, SettingsRegistryPath,
    SettingsRegistryPathBuf, SettingsRegistryVTable,
};
use fimo_ffi::object::CoerceObject;
use fimo_ffi::vtable::ObjectID;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::ops::RangeFrom;

/// The settings registry.
#[derive(Debug)]
pub struct SettingsRegistry {
    inner: parking_lot::Mutex<SettingsRegistryInner>,
}

sa::assert_impl_all!(SettingsRegistry: Send, Sync);

impl SettingsRegistry {
    /// Constructs a new `SettingsRegistry`.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Mutex::new(SettingsRegistryInner::new()),
        }
    }
}

impl Default for SettingsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectID for SettingsRegistry {
    const OBJECT_ID: &'static str = "fimo::modules::core::settings::settings_registry";
}

impl CoerceObject<SettingsRegistryVTable> for SettingsRegistry {
    fn get_vtable() -> &'static SettingsRegistryVTable {
        static VTABLE: SettingsRegistryVTable = SettingsRegistryVTable::new::<SettingsRegistry>(
            |ptr, path| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                let path = unsafe { &*path };
                registry.inner.lock().contains(path)
            },
            |ptr, path| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                let path = unsafe { &*path };
                registry.inner.lock().item_type(path)
            },
            |ptr, path| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                let path = unsafe { &*path };
                registry.inner.lock().read(path)
            },
            |ptr, path, value| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                let path = unsafe { &*path };
                registry.inner.lock().write(path, value)
            },
            |ptr, path, value| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                let path = unsafe { &*path };
                registry.inner.lock().read_or(path, value)
            },
            |ptr, path| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                let path = unsafe { &*path };
                registry.inner.lock().remove(path)
            },
            |ptr, path, f| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                let path = unsafe { &*path };
                registry.inner.lock().register_callback(path, f)
            },
            |ptr, id| {
                let registry = unsafe { &*(ptr as *const SettingsRegistry) };
                registry.inner.lock().unregister_callback(id)
            },
        );
        &VTABLE
    }
}

#[derive(Debug, Default)]
struct Metadata {
    callbacks: RefCell<HashMap<usize, SettingsEventCallback>>,
}

impl Metadata {
    #[inline]
    fn dispatch_event(&self, path: &SettingsRegistryPath, event: SettingsEvent) {
        for c in self.callbacks.borrow_mut().values_mut() {
            c(path, &event)
        }
    }

    #[inline]
    fn combine(&mut self, other: &mut Self) {
        let mut s = self.callbacks.borrow_mut();
        let mut o = other.callbacks.borrow_mut();

        for (id, c) in o.drain() {
            s.insert(id, c);
        }
    }
}

impl Clone for Metadata {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl SettingsItemMetadata for Metadata {
    fn combine(&mut self, other: &mut Self) {
        self.combine(other)
    }

    fn on_write(&self, path: &SettingsRegistryPath, new: &SettingsItem<Self>) {
        let new = new.clone().cast();
        self.dispatch_event(path, SettingsEvent::StartWrite { new })
    }

    fn on_write_abort(&self, path: &SettingsRegistryPath) {
        self.dispatch_event(path, SettingsEvent::AbortWrite)
    }

    fn on_write_complete(&self, path: &SettingsRegistryPath, old: &Option<SettingsItem<Self>>) {
        let old = old.as_ref().map(|i| i.clone().cast());
        self.dispatch_event(path, SettingsEvent::EndWrite { old })
    }

    fn on_removal(&self, path: &SettingsRegistryPath, old: &SettingsItem<Self>) {
        let old = old.clone().cast();
        self.dispatch_event(path, SettingsEvent::Remove { old })
    }
}

#[derive(Debug)]
struct SettingsRegistryInner {
    root: SettingsItem<Metadata>,
    id_gen: RangeFrom<usize>,
    callback_map: BTreeMap<usize, SettingsRegistryPathBuf>,
}

impl SettingsRegistryInner {
    #[inline]
    fn new() -> Self {
        Self {
            root: SettingsItem::from(BTreeMap::new()),
            id_gen: (0..),
            callback_map: Default::default(),
        }
    }

    #[inline]
    fn contains(
        &self,
        path: &SettingsRegistryPath,
    ) -> Result<bool, SettingsRegistryInvalidPathError> {
        if path.is_root() {
            Ok(true)
        } else {
            self.root.contains(path)
        }
    }

    #[inline]
    fn item_type(
        &self,
        path: &SettingsRegistryPath,
    ) -> Result<Option<SettingsItemType>, SettingsRegistryInvalidPathError> {
        if path.is_root() {
            Ok(Some(self.root.item_type()))
        } else {
            self.root.get(path).map(|opt| opt.map(|i| i.item_type()))
        }
    }

    #[inline]
    fn read(
        &self,
        path: &SettingsRegistryPath,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError> {
        if path.is_root() {
            Ok(Some(self.root.clone().cast()))
        } else {
            self.root.read(path).map(|opt| opt.map(|i| i.cast()))
        }
    }

    #[inline]
    fn write(
        &mut self,
        path: &SettingsRegistryPath,
        value: SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError> {
        if path.is_root() {
            if !value.item_type().is_object() {
                Err(SettingsRegistryInvalidPathError::new(path))
            } else {
                let mut metadata = Metadata::default();
                metadata.combine(self.root.as_metadata_mut());
                self.callback_map
                    .retain(|id, _| metadata.callbacks.get_mut().contains_key(id));

                let mut value = value.cast();
                *value.as_metadata_mut() = metadata;
                std::mem::swap(&mut self.root, &mut value);
                Ok(Some(value.cast()))
            }
        } else {
            self.root
                .write(path, value.cast())
                .map(|opt| opt.map(|i| i.cast()))
        }
    }

    #[inline]
    fn read_or(
        &mut self,
        path: &SettingsRegistryPath,
        value: SettingsItem,
    ) -> Result<SettingsItem, SettingsRegistryInvalidPathError> {
        if path.is_root() {
            Ok(self.root.clone().cast())
        } else {
            self.root.read_or(path, value.cast()).map(|i| i.cast())
        }
    }

    #[inline]
    fn remove(
        &mut self,
        path: &SettingsRegistryPath,
    ) -> Result<Option<SettingsItem>, SettingsRegistryInvalidPathError> {
        if path.is_root() {
            Err(SettingsRegistryInvalidPathError::new(path))
        } else {
            self.root.remove(path).map(|opt| opt.map(|i| i.cast()))
        }
    }

    #[inline]
    fn register_callback(
        &mut self,
        path: &SettingsRegistryPath,
        f: SettingsEventCallback,
    ) -> Option<SettingsEventCallbackId> {
        let item = if path.is_root() {
            Some(&mut self.root)
        } else {
            self.root.get_mut(path).unwrap_or(None)
        };

        item.and_then(|i| {
            self.id_gen.next().map(|id| {
                i.as_metadata_mut().callbacks.get_mut().insert(id, f);
                let c_id = unsafe { SettingsEventCallbackId::from_usize(id) };

                self.callback_map.insert(id, path.to_path_buf());
                c_id
            })
        })
    }

    #[inline]
    fn unregister_callback(&mut self, id: SettingsEventCallbackId) {
        let id = usize::from(id);
        let path = self
            .callback_map
            .remove(&id)
            .unwrap_or_else(|| panic!("invalid callback id {:?}", id));

        let item = if path.is_root() {
            Some(&mut self.root)
        } else {
            self.root.get_mut(path).unwrap_or(None)
        };

        if let Some(item) = item {
            item.as_metadata_mut().callbacks.get_mut().remove(&id);
        }
    }
}
