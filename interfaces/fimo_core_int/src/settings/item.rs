use crate::settings::{SettingsPath, SettingsPathBuf, SettingsPathComponentIter};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// Metadata of a `SettingsItem`.
pub trait SettingsItemMetadata: Default + Clone {
    /// Combines the metadata of `self` with `other`.
    fn combine(&mut self, other: &mut Self);

    /// Event called when a write operation been completed.
    fn on_write(&self, path: &SettingsPath);

    /// Event called when an item has been removed.
    fn on_remove(&self, path: &SettingsPath);
}

impl SettingsItemMetadata for () {
    #[inline]
    fn combine(&mut self, _other: &mut Self) {}

    #[inline]
    fn on_write(&self, _path: &SettingsPath) {}

    #[inline]
    fn on_remove(&self, _path: &SettingsPath) {}
}

/// A item from the settings registry.
#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingsItem<T: SettingsItemMetadata = ()> {
    /// Empty value.
    Null(SettingsItemVal<(), T>),
    /// Boolean value.
    Bool(SettingsItemVal<bool, T>),
    /// U64 number value.
    U64(SettingsItemVal<u64, T>),
    /// F64 number value.
    F64(SettingsItemVal<f64, T>),
    /// String value.
    String(SettingsItemVal<fimo_module::fimo_ffi::String, T>),
    /// Array of items.
    Array(SettingsItemVal<fimo_module::fimo_ffi::Vec<Self>, T>),
    /// Map of items.
    Object(SettingsItemVal<BTreeMap<fimo_module::fimo_ffi::String, Self>, T>),
}

/// Value of a [`SettingsItem`].
#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SettingsItemVal<T, M: SettingsItemMetadata = ()> {
    v: T,
    #[serde(skip)]
    m: M,
}

impl<T, M: SettingsItemMetadata> SettingsItemVal<T, M> {
    /// Constructs a new `SettingsItemVal`.
    #[inline]
    pub fn new<U: Into<T>>(v: U) -> Self {
        Self::new_with_metadata(v, Default::default())
    }

    /// Constructs a new `SettingsItemVal`.
    #[inline]
    pub fn new_with_metadata<U: Into<T>>(v: U, m: M) -> Self {
        Self { v: v.into(), m }
    }

    /// Extracts a reference to the contained data.
    #[inline]
    pub fn as_val(&self) -> &T {
        &self.v
    }

    /// Extracts a mutable reference to the contained data.
    #[inline]
    pub fn as_val_mut(&mut self) -> &mut T {
        &mut self.v
    }

    /// Consumes `self` and returns the contained data.
    #[inline]
    pub fn into_val(self) -> T {
        self.v
    }

    /// Extracts a reference to the contained metadata.
    #[inline]
    pub fn as_metadata(&self) -> &M {
        &self.m
    }

    /// Extracts a mutable reference to the contained metadata.
    #[inline]
    pub fn as_metadata_mut(&mut self) -> &mut M {
        &mut self.m
    }
}

impl<T: SettingsItemMetadata> SettingsItem<T> {
    /// Constructs a new array.
    #[inline]
    pub fn new_array() -> Self {
        Self::Array(SettingsItemVal::new(fimo_module::fimo_ffi::vec![]))
    }

    /// Constructs a new object.
    #[inline]
    pub fn new_object() -> Self {
        Self::Object(SettingsItemVal::new(BTreeMap::default()))
    }

    /// Extracts the type of the item.
    #[inline]
    pub fn item_type(&self) -> SettingsItemType {
        match self {
            SettingsItem::Null(_) => SettingsItemType::Null,
            SettingsItem::Bool(_) => SettingsItemType::Bool,
            SettingsItem::U64(_) => SettingsItemType::U64,
            SettingsItem::F64(_) => SettingsItemType::F64,
            SettingsItem::String(_) => SettingsItemType::String,
            SettingsItem::Array(v) => SettingsItemType::Array {
                len: v.as_val().len(),
            },
            SettingsItem::Object(_) => SettingsItemType::Object,
        }
    }

    /// Extracts a reference to a `bool`, if it is contained.
    #[inline]
    pub fn as_bool(&self) -> Option<&bool> {
        match self {
            SettingsItem::Bool(v) => Some(v.as_val()),
            _ => None,
        }
    }

    /// Extracts a mutable reference to a `bool`, if it is contained.
    #[inline]
    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            SettingsItem::Bool(v) => Some(v.as_val_mut()),
            _ => None,
        }
    }

    /// Extracts a `bool`, if it is contained.
    #[inline]
    pub fn into_bool(self) -> Option<bool> {
        match self {
            SettingsItem::Bool(v) => Some(v.into_val()),
            _ => None,
        }
    }

    /// Extracts a reference to a `u64`, if it is contained.
    #[inline]
    pub fn as_u64(&self) -> Option<&u64> {
        match self {
            SettingsItem::U64(v) => Some(v.as_val()),
            _ => None,
        }
    }

    /// Extracts a mutable reference to a `u64`, if it is contained.
    #[inline]
    pub fn as_u64_mut(&mut self) -> Option<&mut u64> {
        match self {
            SettingsItem::U64(v) => Some(v.as_val_mut()),
            _ => None,
        }
    }

    /// Extracts a `u64`, if it is contained.
    #[inline]
    pub fn into_u64(self) -> Option<u64> {
        match self {
            SettingsItem::U64(v) => Some(v.into_val()),
            _ => None,
        }
    }

    /// Extracts a reference to a `f64`, if it is contained.
    #[inline]
    pub fn as_f64(&self) -> Option<&f64> {
        match self {
            SettingsItem::F64(v) => Some(v.as_val()),
            _ => None,
        }
    }

    /// Extracts a mutable reference to a `f64`, if it is contained.
    #[inline]
    pub fn as_f64_mut(&mut self) -> Option<&mut f64> {
        match self {
            SettingsItem::F64(v) => Some(v.as_val_mut()),
            _ => None,
        }
    }

    /// Extracts a `f64`, if it is contained.
    #[inline]
    pub fn into_f64(self) -> Option<f64> {
        match self {
            SettingsItem::F64(v) => Some(v.into_val()),
            _ => None,
        }
    }

    /// Extracts a reference to a [`String`](fimo_module::fimo_ffi::String), if it is contained.
    #[inline]
    pub fn as_string(&self) -> Option<&fimo_module::fimo_ffi::String> {
        match self {
            SettingsItem::String(v) => Some(v.as_val()),
            _ => None,
        }
    }

    /// Extracts a mutable reference to a [`String`](fimo_module::fimo_ffi::String), if it is contained.
    #[inline]
    pub fn as_string_mut(&mut self) -> Option<&mut fimo_module::fimo_ffi::String> {
        match self {
            SettingsItem::String(v) => Some(v.as_val_mut()),
            _ => None,
        }
    }

    /// Extracts a [`String`](fimo_module::fimo_ffi::String), if it is contained.
    #[inline]
    pub fn into_string(self) -> Option<fimo_module::fimo_ffi::String> {
        match self {
            SettingsItem::String(v) => Some(v.into_val()),
            _ => None,
        }
    }

    /// Extracts a reference to a [`Vec`](fimo_module::fimo_ffi::Vec), if it is contained.
    #[inline]
    pub fn as_vec(&self) -> Option<&fimo_module::fimo_ffi::Vec<Self>> {
        match self {
            SettingsItem::Array(v) => Some(v.as_val()),
            _ => None,
        }
    }

    /// Extracts a mutable reference to a [`Vec`](fimo_module::fimo_ffi::Vec), if it is contained.
    #[inline]
    pub fn as_vec_mut(&mut self) -> Option<&mut fimo_module::fimo_ffi::Vec<Self>> {
        match self {
            SettingsItem::Array(v) => Some(v.as_val_mut()),
            _ => None,
        }
    }

    /// Extracts a [`Vec`](fimo_module::fimo_ffi::Vec), if it is contained.
    #[inline]
    pub fn into_vec(self) -> Option<fimo_module::fimo_ffi::Vec<Self>> {
        match self {
            SettingsItem::Array(v) => Some(v.into_val()),
            _ => None,
        }
    }

    /// Extracts a reference to a [`BTreeMap`], if it is contained.
    #[inline]
    pub fn as_map(&self) -> Option<&BTreeMap<fimo_module::fimo_ffi::String, Self>> {
        match self {
            SettingsItem::Object(v) => Some(v.as_val()),
            _ => None,
        }
    }

    /// Extracts a mutable reference to a [`BTreeMap`], if it is contained.
    #[inline]
    pub fn as_map_mut(&mut self) -> Option<&mut BTreeMap<fimo_module::fimo_ffi::String, Self>> {
        match self {
            SettingsItem::Object(v) => Some(v.as_val_mut()),
            _ => None,
        }
    }

    /// Extracts a [`BTreeMap`], if it is contained.
    #[inline]
    pub fn into_map(self) -> Option<BTreeMap<fimo_module::fimo_ffi::String, Self>> {
        match self {
            SettingsItem::Object(v) => Some(v.into_val()),
            _ => None,
        }
    }

    /// Extracts a reference to the contained metadata.
    #[inline]
    pub fn as_metadata(&self) -> &T {
        match self {
            SettingsItem::Null(v) => v.as_metadata(),
            SettingsItem::Bool(v) => v.as_metadata(),
            SettingsItem::U64(v) => v.as_metadata(),
            SettingsItem::F64(v) => v.as_metadata(),
            SettingsItem::String(v) => v.as_metadata(),
            SettingsItem::Array(v) => v.as_metadata(),
            SettingsItem::Object(v) => v.as_metadata(),
        }
    }

    /// Extracts a mutable reference to the contained metadata.
    #[inline]
    pub fn as_metadata_mut(&mut self) -> &mut T {
        match self {
            SettingsItem::Null(v) => v.as_metadata_mut(),
            SettingsItem::Bool(v) => v.as_metadata_mut(),
            SettingsItem::U64(v) => v.as_metadata_mut(),
            SettingsItem::F64(v) => v.as_metadata_mut(),
            SettingsItem::String(v) => v.as_metadata_mut(),
            SettingsItem::Array(v) => v.as_metadata_mut(),
            SettingsItem::Object(v) => v.as_metadata_mut(),
        }
    }

    /// Map the metadata of the `SettingsItem`.
    #[inline]
    pub fn map_metadata<U: SettingsItemMetadata, F: FnOnce(T) -> U + Clone>(
        self,
        f: F,
    ) -> SettingsItem<U> {
        match self {
            SettingsItem::Null(v) => {
                SettingsItem::Null(SettingsItemVal::new_with_metadata((), f(v.m)))
            }
            SettingsItem::Bool(v) => {
                SettingsItem::Bool(SettingsItemVal::new_with_metadata(v.v, f(v.m)))
            }
            SettingsItem::U64(v) => {
                SettingsItem::U64(SettingsItemVal::new_with_metadata(v.v, f(v.m)))
            }
            SettingsItem::F64(v) => {
                SettingsItem::F64(SettingsItemVal::new_with_metadata(v.v, f(v.m)))
            }
            SettingsItem::String(v) => {
                SettingsItem::String(SettingsItemVal::new_with_metadata(v.v, f(v.m)))
            }
            SettingsItem::Array(v) => {
                let v: fimo_module::fimo_ffi::Vec<_> =
                    v.v.into_iter()
                        .map(move |i| i.map_metadata(f.clone()))
                        .collect();
                SettingsItem::from(v)
            }
            SettingsItem::Object(v) => {
                let v: BTreeMap<_, _> =
                    v.v.into_iter()
                        .map(move |(s, i)| (s, i.map_metadata(f.clone())))
                        .collect();
                SettingsItem::from(v)
            }
        }
    }

    /// Checks if an item is contained.
    #[inline]
    pub fn contains<P: AsRef<SettingsPath>>(
        &self,
        path: P,
    ) -> Result<bool, SettingsInvalidPathError> {
        self.get(path).map(|i| i.is_some())
    }

    /// Extracts a reference to an item.
    #[inline]
    pub fn get<P: AsRef<SettingsPath>>(
        &self,
        path: P,
    ) -> Result<Option<&Self>, SettingsInvalidPathError> {
        let path = path.as_ref();
        self.get_inner(path, path.iter(), |_| {})
    }

    /// Extracts a mutable reference to an item.
    #[inline]
    pub fn get_mut<P: AsRef<SettingsPath>>(
        &mut self,
        path: P,
    ) -> Result<Option<&mut Self>, SettingsInvalidPathError> {
        let path = path.as_ref();
        self.get_inner_mut(path, path.iter(), |_| {})
    }

    /// Extracts an item.
    #[inline]
    pub fn read<P: AsRef<SettingsPath>>(
        &self,
        path: P,
    ) -> Result<Option<Self>, SettingsInvalidPathError> {
        self.get(path).map(|i| i.cloned())
    }

    /// Writes a value into the `SettingsItem`.
    ///
    /// If the parent of `path` is an array, it is extended to the required
    /// length with `SettingsItem::Null`.
    #[inline]
    pub fn write<P: AsRef<SettingsPath>>(
        &mut self,
        path: P,
        value: Self,
    ) -> Result<Option<Self>, SettingsInvalidPathError> {
        let f = |item: &Self| {
            let path = path.as_ref();
            let metadata = item.as_metadata();
            metadata.on_write(path);
        };

        self.write_inner(path.as_ref(), value, f)
    }

    #[inline]
    fn write_inner<P: AsRef<SettingsPath>>(
        &mut self,
        path: P,
        mut value: Self,
        f: impl FnMut(&Self),
    ) -> Result<Option<Self>, SettingsInvalidPathError> {
        let (parent, component) = self.get_parent_mut(path.as_ref(), |_| {})?;
        if component.is_root() {
            return Err(SettingsInvalidPathError::new(path.as_ref()));
        }

        let old = if let Some(item) = parent.get_mut(component)? {
            std::mem::swap(item, &mut value);
            item.as_metadata_mut().combine(value.as_metadata_mut());
            let old = Some(value);
            item.as_metadata_mut().on_write(path.as_ref());
            old
        } else {
            let component_name = component.iter().next().unwrap();
            if let Some(name) = component_name.name() {
                let map = parent.as_map_mut();
                if map.is_none() {
                    return Err(SettingsInvalidPathError::new(path.as_ref()));
                }
                let map = map.unwrap();

                if let Some(idx) = component_name.index() {
                    let mut vec = vec![Default::default(); idx];
                    vec[idx] = value;
                    map.insert(fimo_module::fimo_ffi::String::from(name), vec.into());
                } else {
                    map.insert(fimo_module::fimo_ffi::String::from(name), value);
                }
            } else {
                let idx = component_name.index().unwrap();

                let vec = parent.as_vec_mut();
                if vec.is_none() {
                    return Err(SettingsInvalidPathError::new(path.as_ref()));
                }
                let vec = vec.unwrap();
                if idx >= vec.len() {
                    vec.resize_with(idx + 1, Default::default);
                }
                vec[idx] = value;
            }

            None
        };

        self.get_parent_mut(path.as_ref(), f)?;
        Ok(old)
    }

    /// Reads a value from the `SettingsItem`.
    ///
    /// It is initialized with `default`, if the item does not exist.
    #[inline]
    pub fn read_or<P: AsRef<SettingsPath>>(
        &mut self,
        path: P,
        default: Self,
    ) -> Result<Self, SettingsInvalidPathError> {
        let (parent, component) = self.get_parent_mut(path.as_ref(), |_| {})?;
        if component.is_root() {
            return Err(SettingsInvalidPathError::new(path.as_ref()));
        }

        if let Some(i) = parent.read(component)? {
            Ok(i)
        } else {
            let res = parent.write_inner(component, default.clone(), |_| {});

            if res.is_ok() {
                let f = |item: &Self| {
                    let path = path.as_ref();
                    let metadata = item.as_metadata();
                    metadata.on_write(path);
                };
                self.get_parent_mut(path.as_ref(), f)?;
            }

            res.map(|_| default)
        }
    }

    /// Removes a `SettingsItem` value.
    #[inline]
    pub fn remove<P: AsRef<SettingsPath>>(
        &mut self,
        path: P,
    ) -> Result<Option<Self>, SettingsInvalidPathError> {
        let (parent, component) = self.get_parent_mut(path.as_ref(), |_| {})?;
        if component.is_root() {
            return Err(SettingsInvalidPathError::new(path.as_ref()));
        }

        let component_name = component.iter().next().unwrap();
        let mut old = if let Some(name) = component_name.name() {
            let map = parent.as_map_mut();
            if map.is_none() {
                return Err(SettingsInvalidPathError::new(path.as_ref()));
            }
            let map = map.unwrap();
            map.remove(name)
        } else {
            let idx = component_name.index().unwrap();

            let vec = parent.as_vec_mut();
            if vec.is_none() {
                return Err(SettingsInvalidPathError::new(path.as_ref()));
            }
            let vec = vec.unwrap();
            if vec.get(idx).is_some() {
                Some(vec.remove(idx))
            } else {
                None
            }
        };

        if old.is_some() {
            let f = |item: &Self| {
                let path = path.as_ref();
                let metadata = item.as_metadata();
                metadata.on_remove(path);
            };

            f(old.as_mut().unwrap());
            self.get_parent_mut(path.as_ref(), f)?;
        }

        Ok(old)
    }

    #[inline]
    fn get_parent_mut<'a>(
        &mut self,
        path: &'a SettingsPath,
        mut f: impl FnMut(&Self),
    ) -> Result<(&mut Self, &'a SettingsPath), SettingsInvalidPathError> {
        let (parent, component) = path.split_parent();
        match parent {
            None => {
                f(self);
                Ok((self, component))
            }
            Some(p) if p.is_root() => {
                f(self);
                Ok((self, component))
            }
            Some(parent) => {
                let item = self.get_inner_mut(parent, parent.iter(), f);
                match item {
                    Ok(None) | Err(_) => Err(SettingsInvalidPathError::new(path)),
                    Ok(Some(i)) => Ok((i, component)),
                }
            }
        }
    }

    #[inline]
    fn get_inner(
        &self,
        path: &SettingsPath,
        mut components: SettingsPathComponentIter<'_>,
        mut f: impl FnMut(&Self),
    ) -> Result<Option<&Self>, SettingsInvalidPathError> {
        f(self);

        let component = components.next();
        if component.is_none() || component.unwrap().as_path().is_root() {
            return Err(SettingsInvalidPathError::new(path));
        }
        let component = component.unwrap();
        let item = if let Some(name) = component.name() {
            let map = match self.as_map() {
                None => return Ok(None),
                Some(m) => m,
            };

            let item = match map.get(name) {
                None => return Ok(None),
                Some(i) => i,
            };

            if let Some(idx) = component.index() {
                f(item);

                let vec = match item.as_vec() {
                    None => return Ok(None),
                    Some(v) => v,
                };
                match vec.get(idx) {
                    None => return Ok(None),
                    Some(i) => i,
                }
            } else {
                item
            }
        } else {
            let idx = component.index().unwrap();
            let vec = match self.as_vec() {
                None => return Ok(None),
                Some(v) => v,
            };
            match vec.get(idx) {
                None => return Ok(None),
                Some(i) => i,
            }
        };

        match components.peekable().peek() {
            None => {
                f(item);
                Ok(Some(item))
            }
            Some(_) => item.get_inner(path, components, f),
        }
    }

    #[inline]
    fn get_inner_mut(
        &mut self,
        path: &SettingsPath,
        mut components: SettingsPathComponentIter<'_>,
        mut f: impl FnMut(&Self),
    ) -> Result<Option<&mut Self>, SettingsInvalidPathError> {
        f(self);

        let component = components.next();
        if component.is_none() || component.unwrap().as_path().is_root() {
            return Err(SettingsInvalidPathError::new(path));
        }
        let component = component.unwrap();
        let item = if let Some(name) = component.name() {
            let map = match self.as_map_mut() {
                None => return Ok(None),
                Some(m) => m,
            };

            let item = match map.get_mut(name) {
                None => return Ok(None),
                Some(i) => i,
            };

            if let Some(idx) = component.index() {
                f(item);

                let vec = match item.as_vec_mut() {
                    None => return Ok(None),
                    Some(v) => v,
                };
                match vec.get_mut(idx) {
                    None => return Ok(None),
                    Some(i) => i,
                }
            } else {
                item
            }
        } else {
            let idx = component.index().unwrap();
            let vec = match self.as_vec_mut() {
                None => return Ok(None),
                Some(v) => v,
            };
            match vec.get_mut(idx) {
                None => return Ok(None),
                Some(i) => i,
            }
        };

        match components.peekable().peek() {
            None => {
                f(item);
                Ok(Some(item))
            }
            Some(_) => item.get_inner_mut(path, components, f),
        }
    }
}

impl<T: Default, M: SettingsItemMetadata> Default for SettingsItemVal<T, M> {
    #[inline]
    fn default() -> Self {
        Self {
            v: T::default(),
            m: M::default(),
        }
    }
}

impl<T: SettingsItemMetadata> Default for SettingsItem<T> {
    #[inline]
    fn default() -> Self {
        SettingsItem::Null(SettingsItemVal::default())
    }
}

impl<T: SettingsItemMetadata> From<()> for SettingsItem<T> {
    #[inline]
    fn from(_: ()) -> Self {
        Default::default()
    }
}

impl<T: SettingsItemMetadata> From<bool> for SettingsItem<T> {
    #[inline]
    fn from(val: bool) -> Self {
        SettingsItem::Bool(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<u8> for SettingsItem<T> {
    #[inline]
    fn from(val: u8) -> Self {
        SettingsItem::U64(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<u16> for SettingsItem<T> {
    #[inline]
    fn from(val: u16) -> Self {
        SettingsItem::U64(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<u32> for SettingsItem<T> {
    #[inline]
    fn from(val: u32) -> Self {
        SettingsItem::U64(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<u64> for SettingsItem<T> {
    #[inline]
    fn from(val: u64) -> Self {
        SettingsItem::U64(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<usize> for SettingsItem<T> {
    #[inline]
    fn from(val: usize) -> Self {
        SettingsItem::U64(SettingsItemVal::new(val as u64))
    }
}

impl<T: SettingsItemMetadata> From<f32> for SettingsItem<T> {
    #[inline]
    fn from(val: f32) -> Self {
        SettingsItem::F64(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<f64> for SettingsItem<T> {
    #[inline]
    fn from(val: f64) -> Self {
        SettingsItem::F64(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<&'_ str> for SettingsItem<T> {
    #[inline]
    fn from(val: &'_ str) -> Self {
        SettingsItem::String(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> From<String> for SettingsItem<T> {
    #[inline]
    fn from(val: String) -> Self {
        SettingsItem::from(fimo_module::fimo_ffi::String::from(val))
    }
}

impl<T: SettingsItemMetadata> From<fimo_module::fimo_ffi::String> for SettingsItem<T> {
    #[inline]
    fn from(val: fimo_module::fimo_ffi::String) -> Self {
        SettingsItem::String(SettingsItemVal::new(val))
    }
}

impl<T: Into<SettingsItem<U>>, U: SettingsItemMetadata, const LEN: usize> From<[T; LEN]>
    for SettingsItem<U>
{
    #[inline]
    fn from(val: [T; LEN]) -> Self {
        let vec: fimo_module::fimo_ffi::Vec<_> = val.into_iter().map(|v| v.into()).collect();
        Self::Array(SettingsItemVal::new(vec))
    }
}

impl<T: Into<SettingsItem<U>> + Clone, U: SettingsItemMetadata> From<&[T]> for SettingsItem<U> {
    #[inline]
    fn from(val: &[T]) -> Self {
        let vec: fimo_module::fimo_ffi::Vec<_> = val.iter().map(|v| v.clone().into()).collect();
        Self::Array(SettingsItemVal::new(vec))
    }
}

impl<T: Into<SettingsItem<U>>, U: SettingsItemMetadata> From<Vec<T>> for SettingsItem<U> {
    #[inline]
    fn from(val: Vec<T>) -> Self {
        let vec: fimo_module::fimo_ffi::Vec<_> = val.into_iter().map(|v| v.into()).collect();
        Self::Array(SettingsItemVal::new(vec))
    }
}

impl<T: Into<SettingsItem<U>>, U: SettingsItemMetadata> From<fimo_module::fimo_ffi::Vec<T>>
    for SettingsItem<U>
{
    #[inline]
    fn from(val: fimo_module::fimo_ffi::Vec<T>) -> Self {
        let vec: fimo_module::fimo_ffi::Vec<_> = val.into_iter().map(|v| v.into()).collect();
        Self::Array(SettingsItemVal::new(vec))
    }
}

impl<U: SettingsItemMetadata> From<BTreeMap<fimo_module::fimo_ffi::String, SettingsItem<U>>>
    for SettingsItem<U>
{
    #[inline]
    fn from(val: BTreeMap<fimo_module::fimo_ffi::String, SettingsItem<U>>) -> Self {
        SettingsItem::Object(SettingsItemVal::new(val))
    }
}

impl<U: SettingsItemMetadata> From<BTreeMap<String, SettingsItem<U>>> for SettingsItem<U> {
    #[inline]
    fn from(val: BTreeMap<String, SettingsItem<U>>) -> Self {
        let val: BTreeMap<_, _> = val.into_iter().map(|(k, v)| (k.into(), v)).collect();
        SettingsItem::Object(SettingsItemVal::new(val))
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for () {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Null { .. } => Ok(()),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for bool {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Bool(v) => Ok(v.v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for u8 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v.v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for u16 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v.v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for u32 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v.v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for u64 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v.v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for usize {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v.v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for f32 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::F64(v) => Ok(v.v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for f64 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::F64(v) => Ok(v.v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for String {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::String(v) => Ok(v.v.into()),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for fimo_module::fimo_ffi::String {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::String(v) => Ok(v.v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for Vec<SettingsItem<T>> {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Array(v) => Ok(v.v.into()),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}
impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>>
    for fimo_module::fimo_ffi::Vec<SettingsItem<T>>
{
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Array(v) => Ok(v.v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>> for BTreeMap<String, SettingsItem<T>> {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        let map: BTreeMap<fimo_module::fimo_ffi::String, _> = value.try_into()?;
        Ok(map.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<T: SettingsItemMetadata> TryFrom<SettingsItem<T>>
    for BTreeMap<fimo_module::fimo_ffi::String, SettingsItem<T>>
{
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem<T>) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Object(v) => Ok(v.v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

/// Possible error when casting with the [`TryFrom::try_from`] operation.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum SettingsItemTryFromError {
    /// Type does not match with the item type.
    InvalidType,
}

impl Display for SettingsItemTryFromError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsItemTryFromError::InvalidType => write!(f, "Invalid type conversion"),
        }
    }
}

/// A item type from the settings registry.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum SettingsItemType {
    /// Empty value.
    Null,
    /// Boolean value.
    Bool,
    /// U64 number value.
    U64,
    /// F64 number value.
    F64,
    /// String value.
    String,
    /// Array of items.
    Array {
        /// Length of the array.
        len: usize,
    },
    /// Map of items.
    Object,
}

impl SettingsItemType {
    /// Extracts whether an item is [SettingsItemType::Null].
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(*self, SettingsItemType::Null)
    }

    /// Extracts whether an item is [SettingsItemType::Bool].
    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(*self, SettingsItemType::Bool)
    }

    /// Extracts whether an item is [SettingsItemType::U64].
    #[inline]
    pub fn is_u64(&self) -> bool {
        matches!(*self, SettingsItemType::U64)
    }

    /// Extracts whether an item is [SettingsItemType::F64].
    #[inline]
    pub fn is_f64(&self) -> bool {
        matches!(*self, SettingsItemType::F64)
    }

    /// Extracts whether an item is [SettingsItemType::String].
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(*self, SettingsItemType::String)
    }

    /// Extracts whether an item is [SettingsItemType::U64] or an [SettingsItemType::F64].
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(*self, SettingsItemType::U64 | SettingsItemType::F64)
    }

    /// Extracts whether an item is [SettingsItemType::Array].
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(*self, SettingsItemType::Array { .. })
    }

    /// Extracts whether an item is [SettingsItemType::Object].
    #[inline]
    pub fn is_object(&self) -> bool {
        matches!(*self, SettingsItemType::Object)
    }

    /// Extracts the length of an [SettingsItemType::Array] item.
    #[inline]
    pub fn array_len(&self) -> Option<usize> {
        match *self {
            SettingsItemType::Array { len } => Some(len),
            _ => None,
        }
    }
}

/// Error from using an invalid path.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SettingsInvalidPathError {
    path: SettingsPathBuf,
}

impl SettingsInvalidPathError {
    /// Constructs a new `SettingsPathNotFoundError`.
    #[inline]
    pub fn new<P: AsRef<SettingsPath>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Coerces `self` to a [`SettingsPath`] slice.
    #[inline]
    pub fn path(&self) -> &SettingsPath {
        self.path.as_path()
    }

    /// Consumes `self` and returns the contained [`SettingsPathBuf`].
    #[inline]
    pub fn to_path_buffer(self) -> SettingsPathBuf {
        self.path
    }
}

#[cfg(test)]
mod tests {
    use super::{SettingsItem, SettingsPath};

    #[test]
    fn write_obj() {
        let path = SettingsPath::new("element").unwrap();

        let mut obj = <SettingsItem>::new_object();
        assert!(!obj.contains(path).unwrap());

        let old = obj.write(path, 5usize.into()).unwrap();
        assert_eq!(old, None);
        assert!(obj.contains(path).unwrap());

        let old = obj.remove(path).unwrap();
        assert_eq!(old, Some(5usize.into()));
        assert!(!obj.contains(path).unwrap());
    }

    #[test]
    fn write_array() {
        let path = SettingsPath::new("[9]").unwrap();

        let mut arr = <SettingsItem>::new_array();
        assert!(arr.as_vec().unwrap().is_empty());

        arr.write(path, 5usize.into()).unwrap();
        assert_eq!(arr.as_vec().unwrap().len(), 10);

        let path = SettingsPath::new("[5]").unwrap();
        let element = arr.remove(path).unwrap();
        assert_eq!(element, Some(().into()));
        assert_eq!(arr.as_vec().unwrap().len(), 9);

        let path = SettingsPath::new("[8]").unwrap();
        let element = arr.remove(path).unwrap();
        assert_eq!(element, Some(5usize.into()));
        assert_eq!(arr.as_vec().unwrap().len(), 8);
    }

    #[test]
    fn write_nested_obj() {
        let obj_path = SettingsPath::new("obj").unwrap();
        let path = obj_path.join_str("element").unwrap();

        let mut obj = <SettingsItem>::new_object();
        assert!(!obj.contains(&path).unwrap());

        obj.write(obj_path, SettingsItem::new_object()).unwrap();
        obj.write(&path, ().into()).unwrap();
        assert!(obj.contains(&path).unwrap());

        let element = obj.remove(&path).unwrap();
        assert!(!obj.contains(path).unwrap());
        assert_eq!(element, Some(().into()));
    }

    #[test]
    fn write_nested_array() {
        let arr_path = SettingsPath::new("[0]").unwrap();
        let path = arr_path.join_str("[2]").unwrap();

        let mut arr = <SettingsItem>::new_array();
        assert!(!arr.contains(&path).unwrap());

        arr.write(arr_path, SettingsItem::new_array()).unwrap();
        arr.write(&path, 5usize.into()).unwrap();
        assert!(arr.contains(&path).unwrap());

        let element = arr.remove(&path).unwrap();
        assert!(!arr.contains(path).unwrap());
        assert_eq!(element, Some(5usize.into()));
    }

    #[test]
    fn read_items() {
        let path = SettingsPath::new("element").unwrap();

        let mut obj = <SettingsItem>::new_object();
        assert!(!obj.contains(path).unwrap());
        assert_eq!(obj.read(path).unwrap(), None);

        let val = obj.read_or(path, 5usize.into()).unwrap();
        assert_eq!(val, 5usize.into());
        assert!(obj.contains(path).unwrap());
        assert_eq!(obj.read(path).unwrap(), Some(val));

        let val = obj.read_or(path, 0usize.into()).unwrap();
        assert_eq!(val, 5usize.into());
        assert_eq!(obj.read(path).unwrap(), Some(val));
    }
}
