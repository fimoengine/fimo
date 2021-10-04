//! Specification of a settings registry.
use fimo_ffi_core::fn_wrapper::HeapFnMut;
use fimo_module_core::rust::ModuleObject;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ops::Deref;

/// Path to the root element.
pub const ROOT_PATH: &SettingsRegistryPath = unsafe { SettingsRegistryPath::new_unchecked("") };

lazy_static! {
    static ref PATH_COMPONENT_VALIDATOR: regex::Regex =
        regex::Regex::new(r"\A(?P<name>[^:\[\]]+)(?:\[(?P<index>\d+)\])?\z").unwrap();
}

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
    pub fn contains<P: AsRef<SettingsRegistryPath>>(&self, path: P) -> bool {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.contains)(ptr, path.as_ref())
    }

    /// Extracts the type of an item.
    #[inline]
    pub fn item_type<P: AsRef<SettingsRegistryPath>>(&self, path: P) -> Option<SettingsItemType> {
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
    ) -> Option<T> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.read)(ptr, path.as_ref())
            .map(|i| T::try_from(i).unwrap_or_else(|_| panic!("invalid cast")))
    }

    /// Extracts an item from the `SettingsRegistry`.
    ///
    /// Equivalent to calling [`SettingsRegistry::read::<SettingsItem>`] and mapping the result.
    #[inline]
    pub fn try_read<T: TryFrom<SettingsItem>, P: AsRef<SettingsRegistryPath>>(
        &self,
        path: P,
    ) -> Option<Result<T, <T as TryFrom<SettingsItem>>::Error>> {
        let item: Option<SettingsItem> = self.read(path);
        item.map(T::try_from)
    }

    /// Reads a copy of the entire registry.
    ///
    /// Equivalent to calling [`SettingsRegistry::read`] with [`ROOT_PATH`].
    #[inline]
    pub fn read_all(&self) -> BTreeMap<String, SettingsItem> {
        self.read(ROOT_PATH).unwrap()
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
    ) -> Result<Option<SettingsItem>, SettingsRegistryPathNotFoundError> {
        let (ptr, vtable) = self.into_raw_parts();
        (vtable.write)(ptr, path.as_ref(), item.into())
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
    ) -> Result<T, SettingsRegistryPathNotFoundError> {
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
    ) -> Result<Result<T, <T as TryFrom<SettingsItem>>::Error>, SettingsRegistryPathNotFoundError>
    {
        self.read_or(path.as_ref(), item.into()).map(T::try_from)
    }

    /// Removes an item from the `SettingsRegistry`.
    #[inline]
    pub fn remove<P: AsRef<SettingsRegistryPath>>(&self, path: P) -> Option<SettingsItem> {
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
        F: FnMut(&'_ SettingsRegistryPath, SettingsEvent<'_>) + Send + Sync,
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
pub struct SettingsRegistryPathNotFoundError {
    path: SettingsRegistryPathBuf,
}

impl SettingsRegistryPathNotFoundError {
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
    contains: fn(*const (), *const SettingsRegistryPath) -> bool,
    item_type: fn(*const (), *const SettingsRegistryPath) -> Option<SettingsItemType>,
    read: fn(*const (), *const SettingsRegistryPath) -> Option<SettingsItem>,
    write: fn(
        *const (),
        *const SettingsRegistryPath,
        SettingsItem,
    ) -> Result<Option<SettingsItem>, SettingsRegistryPathNotFoundError>,
    read_or: fn(
        *const (),
        *const SettingsRegistryPath,
        SettingsItem,
    ) -> Result<SettingsItem, SettingsRegistryPathNotFoundError>,
    remove: fn(*const (), *const SettingsRegistryPath) -> Option<SettingsItem>,
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
        contains: fn(*const (), *const SettingsRegistryPath) -> bool,
        item_type: fn(*const (), *const SettingsRegistryPath) -> Option<SettingsItemType>,
        read: fn(*const (), *const SettingsRegistryPath) -> Option<SettingsItem>,
        write: fn(
            *const (),
            *const SettingsRegistryPath,
            SettingsItem,
        ) -> Result<Option<SettingsItem>, SettingsRegistryPathNotFoundError>,
        read_or: fn(
            *const (),
            *const SettingsRegistryPath,
            SettingsItem,
        ) -> Result<SettingsItem, SettingsRegistryPathNotFoundError>,
        remove: fn(*const (), *const SettingsRegistryPath) -> Option<SettingsItem>,
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

/// A item from the settings registry.
#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingsItem {
    /// Empty value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// U64 number value.
    U64(u64),
    /// F64 number value.
    F64(f64),
    /// String value.
    String(String),
    /// Array of items.
    Array(Vec<SettingsItem>),
    /// Map of items.
    Object(BTreeMap<String, SettingsItem>),
}

impl SettingsItem {
    /// Extracts whether an item is [SettingsItem::Null].
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, SettingsItem::Null)
    }

    /// Extracts whether an item is [SettingsItem::Bool].
    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(*self, SettingsItem::Bool(_))
    }

    /// Extracts whether an item is [SettingsItem::U64].
    #[inline]
    pub fn is_u64(&self) -> bool {
        matches!(self, SettingsItem::U64(_))
    }

    /// Extracts whether an item is [SettingsItem::F64].
    #[inline]
    pub fn is_f64(&self) -> bool {
        matches!(self, SettingsItem::F64(_))
    }

    /// Extracts whether an item is [SettingsItem::String].
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, SettingsItem::String(_))
    }

    /// Extracts whether an item is [SettingsItem::U64] or an [SettingsItem::F64].
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self, SettingsItem::U64(_) | SettingsItem::F64(_))
    }

    /// Extracts whether an item is [SettingsItem::Array].
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self, SettingsItem::Array(_))
    }

    /// Extracts whether an item is [SettingsItem::Object].
    #[inline]
    pub fn is_object(&self) -> bool {
        matches!(self, SettingsItem::Object(_))
    }

    /// Extracts the length of an [SettingsItem::Array] item.
    #[inline]
    pub fn array_len(&self) -> Option<usize> {
        match self {
            SettingsItem::Array(arr) => Some(arr.len()),
            _ => None,
        }
    }
}

impl Default for SettingsItem {
    #[inline]
    fn default() -> Self {
        SettingsItem::Null
    }
}

impl From<()> for SettingsItem {
    #[inline]
    fn from(_: ()) -> Self {
        SettingsItem::Null
    }
}

impl From<bool> for SettingsItem {
    #[inline]
    fn from(val: bool) -> Self {
        SettingsItem::Bool(val)
    }
}

impl From<u8> for SettingsItem {
    #[inline]
    fn from(val: u8) -> Self {
        SettingsItem::U64(val as u64)
    }
}

impl From<u16> for SettingsItem {
    #[inline]
    fn from(val: u16) -> Self {
        SettingsItem::U64(val as u64)
    }
}

impl From<u32> for SettingsItem {
    #[inline]
    fn from(val: u32) -> Self {
        SettingsItem::U64(val as u64)
    }
}

impl From<u64> for SettingsItem {
    #[inline]
    fn from(val: u64) -> Self {
        SettingsItem::U64(val)
    }
}

impl From<usize> for SettingsItem {
    #[inline]
    fn from(val: usize) -> Self {
        SettingsItem::U64(val as u64)
    }
}

impl From<f32> for SettingsItem {
    #[inline]
    fn from(val: f32) -> Self {
        SettingsItem::F64(val as f64)
    }
}

impl From<f64> for SettingsItem {
    #[inline]
    fn from(val: f64) -> Self {
        SettingsItem::F64(val)
    }
}

impl From<&'_ str> for SettingsItem {
    #[inline]
    fn from(val: &'_ str) -> Self {
        SettingsItem::String(String::from(val))
    }
}

impl From<String> for SettingsItem {
    #[inline]
    fn from(val: String) -> Self {
        SettingsItem::String(val)
    }
}

impl<T: Into<SettingsItem>, const LEN: usize> From<[T; LEN]> for SettingsItem {
    #[inline]
    fn from(val: [T; LEN]) -> Self {
        let vec = val.into_iter().map(|v| v.into()).collect();
        Self::Array(vec)
    }
}

impl<T: Into<SettingsItem> + Clone> From<&[T]> for SettingsItem {
    #[inline]
    fn from(val: &[T]) -> Self {
        let vec = val.iter().map(|v| v.clone().into()).collect();
        Self::Array(vec)
    }
}

impl<T: Into<SettingsItem>> From<Vec<T>> for SettingsItem {
    #[inline]
    fn from(val: Vec<T>) -> Self {
        let vec = val.into_iter().map(|v| v.into()).collect();
        Self::Array(vec)
    }
}

impl From<BTreeMap<String, SettingsItem>> for SettingsItem {
    #[inline]
    fn from(val: BTreeMap<String, SettingsItem>) -> Self {
        SettingsItem::Object(val)
    }
}

impl TryFrom<SettingsItem> for () {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Null => Ok(()),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for bool {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Bool(v) => Ok(v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for u8 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for u16 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for u32 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for u64 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for usize {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::U64(v) => Ok(v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for f32 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::F64(v) => Ok(v as _),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for f64 {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::F64(v) => Ok(v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for String {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::String(v) => Ok(v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for Vec<SettingsItem> {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Array(v) => Ok(v),
            _ => Err(SettingsItemTryFromError::InvalidType),
        }
    }
}

impl TryFrom<SettingsItem> for BTreeMap<String, SettingsItem> {
    type Error = SettingsItemTryFromError;

    #[inline]
    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        match value {
            SettingsItem::Object(v) => Ok(v),
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

/// Path to an item.
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SettingsRegistryPath {
    path: str,
}

impl SettingsRegistryPath {
    /// Constructs a new `SettingsRegistryPath`.
    ///
    /// # Format
    ///
    /// A path is valid if consists of zero or more components, separated by `::`.
    /// A component is defined as a string that matches the following regular expression:
    ///
    /// `\A[^:\[\]]+\[\d+\]?\z`
    ///
    /// ## Examples
    ///
    /// - Empty path: `""`
    /// - Simple path: `"name"`
    /// - Array path: `"array[15]"`
    /// - Nested path: `"object::array[0]::name"`
    #[inline]
    pub fn new(path: &str) -> Result<&Self, SettingsRegistryPathConstructionError<'_>> {
        // empty path is valid.
        if !path.is_empty() {
            // validate each component
            for component in path.split("::") {
                // component must match the regex.
                if !PATH_COMPONENT_VALIDATOR.is_match(component) {
                    return Err(SettingsRegistryPathConstructionError { path, component });
                }
            }
        }

        // safety: the path has been validated
        unsafe { Ok(Self::new_unchecked(path)) }
    }

    /// Constructs a new `SettingsRegistryPath` without checking its validity.
    ///
    /// # Safety
    ///
    /// See [`SettingsRegistryPath::new`] for more info.
    #[inline]
    pub const unsafe fn new_unchecked(path: &str) -> &Self {
        // is just a wrapper around a `str`.
        std::mem::transmute(path)
    }

    /// Returns an iterator over the path.
    #[inline]
    pub fn iter(&self) -> SettingsRegistryPathComponentIter<'_> {
        SettingsRegistryPathComponentIter::new(self)
    }

    /// Coerces to a `str` slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.path
    }

    /// Constructs a `SettingsRegistryPathBuf` with the path.
    #[inline]
    pub fn to_path_buf(&self) -> SettingsRegistryPathBuf {
        SettingsRegistryPathBuf::from(self)
    }

    /// Checks if the path is the root path.
    #[inline]
    pub fn is_root(&self) -> bool {
        self == ROOT_PATH
    }

    /// Returns the `SettingsRegistryPath` without its final component, if there is one.
    ///
    /// Returns [`None`] if the path terminates in a root.
    #[inline]
    pub fn parent(&self) -> Option<&Self> {
        if self.is_root() {
            None
        } else if let Some((parent, _)) = self.path.rsplit_once("::") {
            unsafe { Some(Self::new_unchecked(parent)) }
        } else {
            unsafe { Some(Self::new_unchecked(&self.path[..0])) }
        }
    }

    /// Joins two paths.
    #[inline]
    pub fn join<P: AsRef<SettingsRegistryPath>>(&self, path: P) -> SettingsRegistryPathBuf {
        let mut buf = self.to_path_buf();
        buf.push(path);
        buf
    }
}

impl AsRef<SettingsRegistryPath> for &'_ SettingsRegistryPath {
    fn as_ref(&self) -> &SettingsRegistryPath {
        self
    }
}

impl ToOwned for SettingsRegistryPath {
    type Owned = SettingsRegistryPathBuf;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        SettingsRegistryPathBuf::from(self)
    }
}

impl<'a> IntoIterator for &'a SettingsRegistryPath {
    type Item = SettingsRegistryPathComponent<'a>;
    type IntoIter = SettingsRegistryPathComponentIter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Possible error of the [`SettingsRegistryPath::new`] function.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SettingsRegistryPathConstructionError<'a> {
    path: &'a str,
    component: &'a str,
}

impl<'a> SettingsRegistryPathConstructionError<'a> {
    /// Extracts the path from the error.
    #[inline]
    pub fn path(&self) -> &'a str {
        self.path
    }

    /// Extracts the path faulty path component.
    #[inline]
    pub fn component(&self) -> &'a str {
        self.component
    }
}

/// Path component iterator.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SettingsRegistryPathComponentIter<'a> {
    path: &'a SettingsRegistryPath,
}

impl<'a> SettingsRegistryPathComponentIter<'a> {
    /// Constructs a new `SettingsRegistryPathComponentIter`.
    #[inline]
    pub fn new(path: &'a SettingsRegistryPath) -> Self {
        Self { path }
    }
}

impl<'a> Iterator for SettingsRegistryPathComponentIter<'a> {
    type Item = SettingsRegistryPathComponent<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.path.is_root() {
            return None;
        }

        let path_str = self.path.as_str();
        let (component, rest) = match path_str.split_once("::") {
            None => (path_str, &path_str[path_str.len()..]),
            Some((component, rest)) => (component, rest),
        };

        // we know that it matches.
        let captures = PATH_COMPONENT_VALIDATOR.captures(component).unwrap();
        let name = captures.name("name").unwrap().as_str();
        let index: Option<usize> = captures.name("index").map(|i| i.as_str().parse().unwrap());

        let item = if let Some(index) = index {
            Some(SettingsRegistryPathComponent::ArrayItem { name, index })
        } else {
            Some(SettingsRegistryPathComponent::Item { name })
        };

        // safety: we know, that every component is valid.
        let path = unsafe { SettingsRegistryPath::new_unchecked(rest) };
        self.path = path;
        item
    }
}

/// Component of a path.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum SettingsRegistryPathComponent<'a> {
    /// Normal item.
    Item {
        /// Name of the item.
        name: &'a str,
    },
    /// An array item.
    ArrayItem {
        /// Name of the array.
        name: &'a str,
        /// Index of the element.
        index: usize,
    },
}

impl<'a> SettingsRegistryPathComponent<'_> {
    /// Checks whether the component points to a normal item.
    #[inline]
    pub fn is_item(&self) -> bool {
        matches!(self, SettingsRegistryPathComponent::Item { .. })
    }

    /// Checks whether the component points to an array item.
    #[inline]
    pub fn is_array_item(&self) -> bool {
        matches!(self, SettingsRegistryPathComponent::ArrayItem { .. })
    }
}

impl AsRef<str> for SettingsRegistryPathComponent<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        match *self {
            SettingsRegistryPathComponent::Item { name } => name,
            SettingsRegistryPathComponent::ArrayItem { name, .. } => name,
        }
    }
}

/// An owned mutable path.
#[derive(Debug, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SettingsRegistryPathBuf {
    path: String,
}

impl SettingsRegistryPathBuf {
    /// Allocates an empty `SettingsRegistryPathBuf`.
    #[inline]
    pub fn new() -> Self {
        Self {
            path: String::new(),
        }
    }

    /// Creates a new `SettingsRegistryPathBuf` with a given capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            path: String::with_capacity(capacity),
        }
    }

    /// Coerces to a [`SettingsRegistryPath`] reference.
    #[inline]
    pub fn as_path(&self) -> &SettingsRegistryPath {
        // safety: the string is always valid.
        unsafe { SettingsRegistryPath::new_unchecked(self.path.as_str()) }
    }

    /// Extends `self` with `path`.
    #[inline]
    pub fn push<P: AsRef<SettingsRegistryPath>>(&mut self, path: P) {
        let path = path.as_ref();
        if path.is_root() {
            return;
        }

        if !self.is_root() {
            self.path.push_str("::");
        }

        self.path.push_str(path.as_str())
    }

    /// Truncates `self` to [`self.parent`].
    ///
    /// Returns `false` and does nothing if [`self.parent`] is [`None`].
    /// Otherwise, returns `true`.
    #[inline]
    pub fn pop(&mut self) -> bool {
        if let Some(parent) = self.parent() {
            let parent_bytes = parent.as_str().as_bytes();
            let parent_bytes_len = parent_bytes.len();
            let _ = self.path.drain(parent_bytes_len..);

            true
        } else {
            false
        }
    }

    /// Invokes [`reserve`](String::reserve) on the underlying
    /// instance of [`String`].
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.path.reserve(additional)
    }

    /// Invokes [`reserve_exact`](String::reserve_exact) on the
    /// underlying instance of [`String`].
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.path.reserve_exact(additional)
    }

    /// Invokes [`shrink_to_fit`](String::shrink_to_fit) on the
    /// underlying instance of [`String`].
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.path.shrink_to_fit()
    }
}

impl Default for SettingsRegistryPathBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for SettingsRegistryPathBuf {
    type Target = SettingsRegistryPath;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}

impl From<&'_ SettingsRegistryPath> for SettingsRegistryPathBuf {
    fn from(val: &'_ SettingsRegistryPath) -> Self {
        // a path is always valid, so we can simply clone the contents.
        Self {
            path: String::from(val.as_str()),
        }
    }
}

impl AsRef<SettingsRegistryPath> for SettingsRegistryPathBuf {
    #[inline]
    fn as_ref(&self) -> &SettingsRegistryPath {
        self.as_path()
    }
}

impl std::borrow::Borrow<SettingsRegistryPath> for SettingsRegistryPathBuf {
    #[inline]
    fn borrow(&self) -> &SettingsRegistryPath {
        self.as_path()
    }
}

/// Event types from the settings registry.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum SettingsEvent<'a> {
    /// Item removed.
    ///
    /// # Note
    ///
    /// Is signaled after the item has been removed.
    Remove {
        /// Removed value.
        value: &'a SettingsItem,
    },
    /// Signals the start of a `write` operation.
    ///
    /// # Note
    ///
    /// Is signaled before the new item is inserted.
    StartWrite {
        /// Value to be inserted.
        new: &'a SettingsItem,
    },
    /// Signals the end of a `write` operation.
    ///
    /// # Note
    ///
    /// Is signaled after the new item is inserted.
    EndWrite {
        /// Old value.
        old: &'a Option<SettingsItem>,
    },
    /// The write operation was aborted.
    AbortWrite,
    /// Signals the start of a `read_or` operation.
    ///
    /// # Note
    ///
    /// Is signaled before the new item is inserted.
    StartReadOr {
        /// Value to be inserted.
        value: &'a SettingsItem,
    },
    /// Signals the end of a `read_or` operation.
    ///
    /// # Note
    ///
    /// Is signaled after the new item is inserted.
    EndReadOr {
        /// Value to be inserted.
        value: &'a SettingsItem,
    },
    /// The `read_or` operation was aborted.
    AbortReadOr,
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
    inner: HeapFnMut<(&'static SettingsRegistryPath, SettingsEvent<'static>), ()>,
}

impl FnOnce<(&'_ SettingsRegistryPath, SettingsEvent<'_>)> for SettingsEventCallback {
    type Output = ();

    #[inline]
    extern "rust-call" fn call_once(
        mut self,
        args: (&'_ SettingsRegistryPath, SettingsEvent<'_>),
    ) -> Self::Output {
        self.call_mut(args)
    }
}

impl FnMut<(&'_ SettingsRegistryPath, SettingsEvent<'_>)> for SettingsEventCallback {
    #[inline]
    extern "rust-call" fn call_mut(
        &mut self,
        args: (&'_ SettingsRegistryPath, SettingsEvent<'_>),
    ) -> Self::Output {
        let args = unsafe { std::mem::transmute(args) };
        self.inner.call_mut(args)
    }
}

impl<F: FnMut(&'_ SettingsRegistryPath, SettingsEvent<'_>) + Send + Sync> From<Box<F>>
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
