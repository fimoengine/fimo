//! Implementation of the `SettingsRegistry` type.
use fimo_core_interface::rust::{
    CallbackHandle, SettingsEvent, SettingsItem, SettingsUpdateCallback,
};
use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

lazy_static! {
    static ref ITEM_IDENTIFIER: regex::Regex =
        regex::Regex::new(r"(?P<identifier>[\w\-]*)(\[(?P<index>\d+)\])?").unwrap();
}

/// The settings registry.
#[derive(Debug)]
pub struct SettingsRegistry {
    root: Item,
}

struct Item {
    value: ItemValue,
    callbacks: Vec<Box<SettingsUpdateCallback>>,
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

struct ItemIdentifier<'a> {
    identifier: &'a str,
    index: Option<usize>,
}

impl SettingsRegistry {
    /// Constructs a new `SettingsRegistry`.
    pub fn new() -> Self {
        Self {
            root: Item::new(ItemValue::Object(BTreeMap::new())),
        }
    }

    /// Extracts whether an item is [SettingsItem::Null].
    pub fn is_null(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_null())
    }

    /// Extracts whether an item is [SettingsItem::Bool].
    pub fn is_bool(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_bool())
    }

    /// Extracts whether an item is [SettingsItem::U64].
    pub fn is_u64(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_u64())
    }

    /// Extracts whether an item is [SettingsItem::F64].
    pub fn is_f64(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_f64())
    }

    /// Extracts whether an item is [SettingsItem::String].
    pub fn is_string(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_string())
    }

    /// Extracts whether an item is [SettingsItem::U64] or an [SettingsItem::F64].
    pub fn is_number(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_number())
    }

    /// Extracts whether an item is [SettingsItem::Array].
    pub fn is_array(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_array())
    }

    /// Extracts whether an item is [SettingsItem::Object].
    pub fn is_object(&self, item: impl AsRef<str>) -> Option<bool> {
        self.root.get_item(item).map(|i| i.is_object())
    }

    /// Extracts the length of an [SettingsItem::Array] item.
    pub fn array_len(&self, item: impl AsRef<str>) -> Option<usize> {
        self.root
            .get_item(item)
            .map(|i| i.array_len())
            .unwrap_or(None)
    }

    /// Extracts the root item from the `SettingsRegistry`.
    pub fn read_all(&self) -> SettingsItem {
        (&self.root.value).into()
    }

    /// Extracts an item from the `SettingsRegistry`.
    pub fn read(&self, item: impl AsRef<str>) -> Option<SettingsItem> {
        self.root.read(item)
    }

    /// Writes into the `SettingsRegistry`.
    ///
    /// This function either overwrites an existing item or creates a new one.
    /// Afterwards the old value is extracted.
    pub fn write(&mut self, item: impl AsRef<str>, value: SettingsItem) -> Option<SettingsItem> {
        self.root
            .write(item.as_ref(), item.as_ref(), value)
            .unwrap_or(None)
    }

    /// Removes an item from the `SettingsRegistry`.
    pub fn remove(&mut self, item: impl AsRef<str>) -> Option<SettingsItem> {
        self.root.remove(item.as_ref(), item.as_ref())
    }

    /// Registers a callback to an item.
    pub fn register_callback(
        &mut self,
        item: impl AsRef<str>,
        callback: Box<SettingsUpdateCallback>,
    ) -> Option<CallbackHandle<SettingsUpdateCallback>> {
        if item.as_ref() == "" {
            Some(self.root.register_callback(callback))
        } else {
            self.root
                .get_item_mut(item)
                .map(|i| i.register_callback(callback))
        }
    }

    /// Unregisters a callback from an item.
    pub fn unregister_callback(
        &mut self,
        item: impl AsRef<str>,
        handle: CallbackHandle<SettingsUpdateCallback>,
    ) {
        if item.as_ref() == "" {
            self.root.unregister_callback(handle)
        } else if let Some(i) = self.root.get_item_mut(item) {
            i.unregister_callback(handle)
        }
    }
}

impl Item {
    fn new(value: ItemValue) -> Self {
        Self {
            value,
            callbacks: vec![],
        }
    }

    fn is_null(&self) -> bool {
        matches!(self.value, ItemValue::Null)
    }

    fn is_bool(&self) -> bool {
        matches!(self.value, ItemValue::Bool(_))
    }

    fn is_u64(&self) -> bool {
        matches!(self.value, ItemValue::U64(_))
    }

    fn is_f64(&self) -> bool {
        matches!(self.value, ItemValue::F64(_))
    }

    fn is_string(&self) -> bool {
        matches!(self.value, ItemValue::String(_))
    }

    fn is_number(&self) -> bool {
        matches!(self.value, ItemValue::U64(_) | ItemValue::F64(_))
    }

    fn is_array(&self) -> bool {
        matches!(self.value, ItemValue::Array(_))
    }

    fn is_object(&self) -> bool {
        matches!(self.value, ItemValue::Object(_))
    }

    fn array_len(&self) -> Option<usize> {
        match &self.value {
            ItemValue::Array(arr) => Some(arr.len()),
            _ => None,
        }
    }

    fn read(&self, item: impl AsRef<str>) -> Option<SettingsItem> {
        self.get_item(item).map(|i| (&i.value).into())
    }

    fn write(
        &mut self,
        path: impl AsRef<str>,
        item: impl AsRef<str>,
        value: SettingsItem,
    ) -> Option<Option<SettingsItem>> {
        let (name, rest) = match item.as_ref().split_once("::") {
            None => (item.as_ref(), None),
            Some((name, rest)) => (name, Some(rest)),
        };

        match self.get_item_mut(name) {
            // Create items.
            None => {
                let obj = match &mut self.value {
                    ItemValue::Object(obj) => obj,
                    _ => return None,
                };

                if rest.is_some() {
                    return None;
                }

                // Notify callbacks.
                for callback in &mut self.callbacks {
                    callback(path.as_ref(), SettingsEvent::Create { value: &value });
                }

                obj.insert(
                    String::from(name),
                    Item {
                        value: value.into(),
                        callbacks: vec![],
                    },
                );
                Some(None)
            }
            // Overwrite.
            Some(item) => match rest {
                // Overwrite item.
                None => {
                    // Insert temporary null value.
                    let mut tmp = Item {
                        value: ItemValue::Null,
                        callbacks: vec![],
                    };
                    std::mem::swap(item, &mut tmp);
                    item.callbacks = tmp.callbacks;

                    // Extract old value.
                    let old: SettingsItem = tmp.value.into();

                    // Notify callbacks.
                    for callback in &mut item.callbacks {
                        callback(
                            path.as_ref(),
                            SettingsEvent::Overwrite {
                                old: &old,
                                new: &value,
                            },
                        );
                    }

                    // Set new value.
                    item.value = value.into();
                    Some(Some(old))
                }
                // Overwrite sub-item.
                Some(rest) => {
                    let value = item.write(path.as_ref(), rest, value);

                    if value.is_some() {
                        // Notify callbacks.
                        for callback in &mut item.callbacks {
                            callback(path.as_ref(), SettingsEvent::InternalUpdate);
                        }
                    }

                    value
                }
            },
        }
    }

    fn remove(&mut self, path: impl AsRef<str>, item: impl AsRef<str>) -> Option<SettingsItem> {
        let (name, rest) = match item.as_ref().split_once("::") {
            None => (item.as_ref(), None),
            Some((name, rest)) => (name, Some(rest)),
        };

        if let Some(rest) = rest {
            let item = match self.get_item_mut(name) {
                None => return None,
                Some(item) => item,
            };

            let removed = item.remove(path.as_ref(), rest);
            if removed.is_some() {
                for callback in &mut self.callbacks {
                    callback(path.as_ref(), SettingsEvent::InternalRemoval);
                }
            }
            removed
        } else {
            let removed = match &mut self.value {
                ItemValue::Object(obj) => obj.remove(name).map(|i| (i.value.into(), i.callbacks)),
                _ => return None,
            };

            if let Some((removed, callbacks)) = removed {
                for mut callback in callbacks {
                    callback(path.as_ref(), SettingsEvent::Remove { value: &removed })
                }

                for callback in &mut self.callbacks {
                    callback(path.as_ref(), SettingsEvent::InternalRemoval);
                }

                Some(removed)
            } else {
                None
            }
        }
    }

    fn register_callback(
        &mut self,
        callback: Box<SettingsUpdateCallback>,
    ) -> CallbackHandle<SettingsUpdateCallback> {
        let callback_ptr = &*callback as *const _;
        self.callbacks.push(callback);
        CallbackHandle::new(callback_ptr)
    }

    fn unregister_callback(&mut self, handle: CallbackHandle<SettingsUpdateCallback>) {
        self.callbacks.retain(|x| handle.as_ptr() != &*x)
    }

    fn get_item(&self, item: impl AsRef<str>) -> Option<&Item> {
        let (identifier, rest) = match item.as_ref().split_once("::") {
            None => (get_item_identifier(item.as_ref()), None),
            Some((first, rest)) => (get_item_identifier(first), Some(rest)),
        };

        identifier.as_ref()?;

        let obj = match &self.value {
            ItemValue::Object(obj) => obj,
            _ => return None,
        };

        let identifier = identifier.unwrap();
        let sub_item = match obj.get(identifier.identifier) {
            None => return None,
            Some(item) => match identifier.index {
                None => item,
                Some(index) => {
                    if let ItemValue::Array(arr) = &item.value {
                        match arr.get(index) {
                            None => return None,
                            Some(item) => item,
                        }
                    } else {
                        return None;
                    }
                }
            },
        };

        if let Some(sub_identifier) = rest {
            sub_item.get_item(sub_identifier)
        } else {
            Some(sub_item)
        }
    }

    fn get_item_mut(&mut self, item: impl AsRef<str>) -> Option<&mut Item> {
        let (identifier, rest) = match item.as_ref().split_once("::") {
            None => (get_item_identifier(item.as_ref()), None),
            Some((first, rest)) => (get_item_identifier(first), Some(rest)),
        };

        identifier.as_ref()?;

        let obj = match &mut self.value {
            ItemValue::Object(obj) => obj,
            _ => return None,
        };

        let identifier = identifier.unwrap();
        let sub_item = match obj.get_mut(identifier.identifier) {
            None => return None,
            Some(item) => match identifier.index {
                None => item,
                Some(index) => {
                    if let ItemValue::Array(arr) = &mut item.value {
                        match arr.get_mut(index) {
                            None => return None,
                            Some(item) => item,
                        }
                    } else {
                        return None;
                    }
                }
            },
        };

        if let Some(sub_identifier) = rest {
            sub_item.get_item_mut(sub_identifier)
        } else {
            Some(sub_item)
        }
    }
}

impl Default for SettingsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.value, f)
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
                        callbacks: vec![],
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
                                callbacks: vec![],
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
                        callbacks: vec![],
                    })
                    .collect();
                ItemValue::Array(v)
            }
            _ => From::from(&item),
        }
    }
}

fn get_item_identifier(identifier: &str) -> Option<ItemIdentifier<'_>> {
    if let Some(capture) = ITEM_IDENTIFIER.captures(identifier) {
        let identifier = capture.name("identifier").unwrap().as_str();
        return if let Some(index) = capture.name("index") {
            let index: usize = index.as_str().parse().unwrap();
            Some(ItemIdentifier {
                identifier,
                index: Some(index),
            })
        } else {
            Some(ItemIdentifier {
                identifier,
                index: None,
            })
        };
    }

    None
}
