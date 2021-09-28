//! Implementation of the `SettingsRegistry` type.
use fimo_core_interface::rust::{
    SettingsEvent, SettingsEventCallback, SettingsEventCallbackId, SettingsItem, SettingsItemType,
    SettingsRegistryPath, SettingsRegistryPathBuf, SettingsRegistryPathComponent,
    SettingsRegistryPathComponentIter, SettingsRegistryPathNotFoundError, SettingsRegistryVTable,
};
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, RangeFrom};

const VTABLE: SettingsRegistryVTable = SettingsRegistryVTable::new(
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

/// The settings registry.
#[derive(Debug)]
pub struct SettingsRegistry {
    inner: parking_lot::Mutex<SettingsRegistryInner>,
}

impl SettingsRegistry {
    /// Constructs a new `SettingsRegistry`.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Mutex::new(SettingsRegistryInner::new()),
        }
    }
}

impl Deref for SettingsRegistry {
    type Target = fimo_core_interface::rust::SettingsRegistry;

    fn deref(&self) -> &Self::Target {
        let self_ptr = self as *const _ as *const ();
        let vtable = &VTABLE;

        unsafe { &*Self::Target::from_raw_parts(self_ptr, vtable) }
    }
}

impl Default for SettingsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct SettingsRegistryInner {
    root: Item,
    id_gen: RangeFrom<usize>,
    callback_map: BTreeMap<usize, SettingsRegistryPathBuf>,
}

impl SettingsRegistryInner {
    #[inline]
    fn new() -> Self {
        Self {
            root: Item::new(ItemValue::Object(Default::default())),
            id_gen: (0..),
            callback_map: Default::default(),
        }
    }

    #[inline]
    fn contains(&self, path: &SettingsRegistryPath) -> bool {
        self.root.contains(path.iter())
    }

    #[inline]
    fn item_type(&self, path: &SettingsRegistryPath) -> Option<SettingsItemType> {
        self.root
            .find_recursive(path.iter())
            .map(|item| item.item_type())
    }

    #[inline]
    fn read(&self, path: &SettingsRegistryPath) -> Option<SettingsItem> {
        self.root.read(path.iter())
    }

    #[inline]
    fn write(
        &mut self,
        path: &SettingsRegistryPath,
        value: SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsRegistryPathNotFoundError> {
        self.root.write(path, path.iter(), value)
    }

    #[inline]
    fn read_or(
        &mut self,
        path: &SettingsRegistryPath,
        value: SettingsItem,
    ) -> Result<SettingsItem, SettingsRegistryPathNotFoundError> {
        self.root.read_or(path, path.iter(), value)
    }

    #[inline]
    fn remove(&mut self, path: &SettingsRegistryPath) -> Option<SettingsItem> {
        self.root.remove(path, path.iter())
    }

    #[inline]
    fn register_callback(
        &mut self,
        path: &SettingsRegistryPath,
        f: SettingsEventCallback,
    ) -> Option<SettingsEventCallbackId> {
        let item = self.root.find_recursive_mut(path.iter());
        item.and_then(|i| {
            self.id_gen.next().map(|id| {
                i.callbacks.insert(id, f);
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

        if let Some(item) = self.root.find_recursive_mut(path.iter()) {
            item.callbacks.remove(&id);
        }
    }
}

struct Item {
    value: ItemValue,
    callbacks: BTreeMap<usize, SettingsEventCallback>,
}

impl Item {
    #[inline]
    fn new(value: ItemValue) -> Self {
        Self {
            value,
            callbacks: BTreeMap::default(),
        }
    }

    #[inline]
    fn item_type(&self) -> SettingsItemType {
        match &self.value {
            ItemValue::Null => SettingsItemType::Null,
            ItemValue::Bool(_) => SettingsItemType::Bool,
            ItemValue::U64(_) => SettingsItemType::U64,
            ItemValue::F64(_) => SettingsItemType::F64,
            ItemValue::String(_) => SettingsItemType::String,
            ItemValue::Array(arr) => SettingsItemType::Array { len: arr.len() },
            ItemValue::Object(_) => SettingsItemType::Object,
        }
    }

    #[inline]
    fn unwrap_array(&self) -> &Vec<Item> {
        match &self.value {
            ItemValue::Array(vec) => vec,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn unwrap_array_mut(&mut self) -> &mut Vec<Item> {
        match &mut self.value {
            ItemValue::Array(vec) => vec,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn unwrap_object(&self) -> &BTreeMap<String, Item> {
        match &self.value {
            ItemValue::Object(obj) => obj,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn unwrap_object_mut(&mut self) -> &mut BTreeMap<String, Item> {
        match &mut self.value {
            ItemValue::Object(obj) => obj,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn contains(&self, mut components: SettingsRegistryPathComponentIter<'_>) -> bool {
        let item = self.get_item(&mut components);
        item.map(|item| {
            if components.peekable().peek().is_none() {
                true
            } else {
                item.contains(components)
            }
        })
        .unwrap_or(false)
    }

    #[inline]
    fn read(&self, mut components: SettingsRegistryPathComponentIter<'_>) -> Option<SettingsItem> {
        let item = self.get_item(&mut components);
        item.and_then(|item| {
            if components.peekable().peek().is_none() {
                Some(SettingsItem::from(&item.value))
            } else {
                item.read(components)
            }
        })
    }

    #[inline]
    fn write(
        &mut self,
        path: &SettingsRegistryPath,
        mut components: SettingsRegistryPathComponentIter<'_>,
        value: SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsRegistryPathNotFoundError> {
        self.dispatch_event(path, SettingsEvent::StartWrite { new: &value });

        let self_ptr: *mut _ = self as _;

        let components_copy = components;
        let item = self.get_item_mut(&mut components);

        let res = {
            if let Some(item) = item {
                if components.peekable().peek().is_none() {
                    let item_ptr = item as *mut Item;
                    if item_ptr != self_ptr {
                        item.dispatch_event(path, SettingsEvent::StartWrite { new: &value });
                    }

                    let old = Some(SettingsItem::from(&item.value));
                    item.value = value.into();

                    if item_ptr != self_ptr {
                        item.dispatch_event(path, SettingsEvent::EndWrite { old: &old });
                    }
                    Ok(old)
                } else {
                    item.write(path, components, value)
                }
            } else {
                self.insert_item(path, components_copy, value).map(|_| None)
            }
        };

        let event = res.as_ref().map_or_else(
            |_| SettingsEvent::AbortWrite,
            |old| SettingsEvent::EndWrite { old },
        );
        self.dispatch_event(path, event);

        res
    }

    #[inline]
    fn read_or(
        &mut self,
        path: &SettingsRegistryPath,
        mut components: SettingsRegistryPathComponentIter<'_>,
        value: SettingsItem,
    ) -> Result<SettingsItem, SettingsRegistryPathNotFoundError> {
        self.dispatch_event(path, SettingsEvent::StartReadOr { value: &value });

        let self_ptr: *mut _ = self as _;

        let components_copy = components;
        let item = self.get_item_mut(&mut components);

        let res = {
            if let Some(item) = item {
                if components.peekable().peek().is_none() {
                    let item_ptr = item as *mut Item;
                    if item_ptr != self_ptr {
                        item.dispatch_event(path, SettingsEvent::StartReadOr { value: &value });
                    }

                    let value = SettingsItem::from(&item.value);
                    if item_ptr != self_ptr {
                        item.dispatch_event(path, SettingsEvent::EndReadOr { value: &value });
                    }

                    Ok(value)
                } else {
                    item.read_or(path, components, value)
                }
            } else {
                self.insert_item(path, components_copy, value.clone())
                    .map(|_| value)
            }
        };

        let event = res.as_ref().map_or_else(
            |_| SettingsEvent::AbortReadOr,
            |value| SettingsEvent::EndReadOr { value },
        );
        self.dispatch_event(path, event);

        res
    }

    #[inline]
    fn remove(
        &mut self,
        path: &SettingsRegistryPath,
        mut components: SettingsRegistryPathComponentIter<'_>,
    ) -> Option<SettingsItem> {
        let name = {
            let mut components = components;
            match components.next() {
                None => return None,
                // check that it is the last component.
                Some(name) => components.next().or(Some(name)),
            }
        };

        let res = {
            // remove last component directly
            if let Some(name) = name {
                if !self.item_type().is_object() {
                    return None;
                }

                let obj = self.unwrap_object_mut();

                let item = match name {
                    SettingsRegistryPathComponent::Item { name } => obj.remove(name).map(|i| i),
                    SettingsRegistryPathComponent::ArrayItem { name, index } => {
                        let arr = obj.get_mut(name);
                        arr.and_then(|arr| {
                            if !arr.item_type().is_array() {
                                return None;
                            }

                            let arr = arr.unwrap_array_mut();
                            if arr.len() <= index {
                                None
                            } else {
                                Some(arr.remove(index))
                            }
                        })
                    }
                };

                if let Some(mut item) = item {
                    let mut value = ItemValue::Null;
                    std::mem::swap(&mut item.value, &mut value);

                    let value = value.into();
                    item.dispatch_event(path, SettingsEvent::Remove { value: &value });

                    Some(value)
                } else {
                    None
                }
            } else {
                self.get_item_mut(&mut components)
                    .and_then(|item| item.remove(path, components))
            }
        };

        if let Some(value) = res.as_ref() {
            self.dispatch_event(path, SettingsEvent::Remove { value })
        }

        res
    }

    #[inline]
    fn insert_item(
        &mut self,
        path: &SettingsRegistryPath,
        mut components: SettingsRegistryPathComponentIter<'_>,
        value: SettingsItem,
    ) -> Result<(), SettingsRegistryPathNotFoundError> {
        let item_name = components.next().unwrap();
        let last = components.next().is_none();

        if self.item_type().is_object() && item_name.is_item() && last {
            let obj = self.unwrap_object_mut();
            obj.insert(
                String::from(item_name.as_ref()),
                Item::new(From::from(value)),
            );

            Ok(())
        } else {
            Err(SettingsRegistryPathNotFoundError::new(path))
        }
    }

    #[inline]
    fn get_item(&self, components: &mut SettingsRegistryPathComponentIter<'_>) -> Option<&Self> {
        if let Some(component) = components.next() {
            if !self.item_type().is_object() {
                return None;
            }

            let obj = self.unwrap_object();

            match component {
                SettingsRegistryPathComponent::Item { name } => obj.get(name),
                SettingsRegistryPathComponent::ArrayItem { name, index } => {
                    let item = obj.get(name);
                    item.and_then(|i| {
                        if !i.item_type().is_array() {
                            return None;
                        }

                        let array = i.unwrap_array();
                        array.get(index)
                    })
                }
            }
        } else {
            Some(self)
        }
    }

    #[inline]
    fn get_item_mut(
        &mut self,
        components: &mut SettingsRegistryPathComponentIter<'_>,
    ) -> Option<&mut Self> {
        if let Some(component) = components.next() {
            if !self.item_type().is_object() {
                return None;
            }

            let obj = self.unwrap_object_mut();

            match component {
                SettingsRegistryPathComponent::Item { name } => obj.get_mut(name),
                SettingsRegistryPathComponent::ArrayItem { name, index } => {
                    let item = obj.get_mut(name);
                    item.and_then(|i| {
                        if !i.item_type().is_array() {
                            return None;
                        }

                        let array = i.unwrap_array_mut();
                        array.get_mut(index)
                    })
                }
            }
        } else {
            Some(self)
        }
    }

    #[inline]
    fn find_recursive(
        &self,
        mut components: SettingsRegistryPathComponentIter<'_>,
    ) -> Option<&Self> {
        let item = self.get_item(&mut components);
        item.and_then(|item| {
            if components.peekable().peek().is_some() {
                item.find_recursive(components)
            } else {
                Some(item)
            }
        })
    }

    #[inline]
    fn find_recursive_mut(
        &mut self,
        mut components: SettingsRegistryPathComponentIter<'_>,
    ) -> Option<&mut Self> {
        let item = self.get_item_mut(&mut components);
        item.and_then(|item| {
            if components.peekable().peek().is_some() {
                item.find_recursive_mut(components)
            } else {
                Some(item)
            }
        })
    }

    #[inline]
    fn dispatch_event<P: AsRef<SettingsRegistryPath>>(
        &mut self,
        path: P,
        event: SettingsEvent<'_>,
    ) {
        for c in self.callbacks.values_mut() {
            c(path.as_ref(), event)
        }
    }
}

impl Debug for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

#[derive(Debug)]
enum ItemValue {
    Null,
    Bool(bool),
    U64(u64),
    F64(f64),
    String(String),
    Array(Vec<Item>),
    Object(BTreeMap<String, Item>),
}

impl From<Item> for SettingsItem {
    fn from(item: Item) -> Self {
        SettingsItem::from(item.value)
    }
}

impl From<&ItemValue> for SettingsItem {
    fn from(item: &ItemValue) -> Self {
        match item {
            ItemValue::Null => SettingsItem::Null,
            ItemValue::Bool(v) => SettingsItem::Bool(*v),
            ItemValue::U64(v) => SettingsItem::U64(*v),
            ItemValue::F64(v) => SettingsItem::F64(*v),
            ItemValue::String(v) => SettingsItem::String(v.clone()),
            ItemValue::Array(v) => {
                let v = v.iter().map(|v| (&v.value).into()).collect();
                SettingsItem::Array(v)
            }
            ItemValue::Object(v) => {
                let v = v
                    .iter()
                    .map(|(k, v)| (k.clone(), (&v.value).into()))
                    .collect();
                SettingsItem::Object(v)
            }
        }
    }
}

impl From<ItemValue> for SettingsItem {
    fn from(item: ItemValue) -> Self {
        match item {
            ItemValue::String(v) => SettingsItem::String(v),
            ItemValue::Array(v) => {
                let v = v.into_iter().map(|i| i.value.into()).collect();
                SettingsItem::Array(v)
            }
            _ => From::from(&item),
        }
    }
}

impl From<&SettingsItem> for ItemValue {
    fn from(item: &SettingsItem) -> Self {
        match item {
            SettingsItem::Null => ItemValue::Null,
            SettingsItem::Bool(v) => ItemValue::Bool(*v),
            SettingsItem::U64(v) => ItemValue::U64(*v),
            SettingsItem::F64(v) => ItemValue::F64(*v),
            SettingsItem::String(v) => ItemValue::String(v.clone()),
            SettingsItem::Array(v) => {
                let v = v
                    .iter()
                    .map(|v| Item {
                        value: v.into(),
                        callbacks: BTreeMap::default(),
                    })
                    .collect();
                ItemValue::Array(v)
            }
            SettingsItem::Object(v) => {
                let v = v
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            Item {
                                value: v.into(),
                                callbacks: BTreeMap::default(),
                            },
                        )
                    })
                    .collect();
                ItemValue::Object(v)
            }
        }
    }
}

impl From<SettingsItem> for ItemValue {
    fn from(item: SettingsItem) -> Self {
        match item {
            SettingsItem::String(v) => ItemValue::String(v),
            SettingsItem::Array(v) => {
                let v = v
                    .into_iter()
                    .map(|v| Item {
                        value: v.into(),
                        callbacks: BTreeMap::default(),
                    })
                    .collect();
                ItemValue::Array(v)
            }
            _ => From::from(&item),
        }
    }
}
