use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::ops::Deref;

lazy_static! {
    static ref PATH_COMPONENT_VALIDATOR: regex::Regex = regex::Regex::new(
        r"\A(?:(?P<name>[^:\[\]]+)|(?P<arr_name>[^:\[\]]+)?(?:\[(?P<index>\d+)\]))\z"
    )
    .unwrap();
    static ref PATH_VALIDATOR: regex::Regex = regex::Regex::new(
        r"\A(?:[^:\[\]]+|(?:[^:\[\]]+)?(?:\[\d+\])+)(?:::[^:\[\]]+(?:\[\d+\])*)*\z"
    )
    .unwrap();
}

/// Path to an item.
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize)]
pub struct SettingsRegistryPath {
    path: str,
}

impl SettingsRegistryPath {
    /// Returns the root `SettingsRegistryPath`.
    #[inline]
    pub const fn root() -> &'static Self {
        unsafe { SettingsRegistryPath::new_unchecked(":") }
    }

    /// Constructs a new `SettingsRegistryPath`.
    ///
    /// # Format
    ///
    /// A path is valid if consists of one or more components, separated by `::`.
    /// A component is defined as a string that matches the following regular expression:
    ///
    /// `\A([^:\[\]]+|([^:\[\]]+)?(\[\d+\])+)(::[^:\[\]]+(\[\d+\])*)*\z`
    ///
    /// ## Examples
    ///
    /// - Simple path: `"name"`
    /// - Array path: `"array[15]"`
    /// - Nested path: `"object::array[0][12]::name"`
    #[inline]
    pub fn new(path: &str) -> Result<&Self, SettingsRegistryPathConstructionError<&'_ str>> {
        if path.is_empty() {
            return Err(SettingsRegistryPathConstructionError { path });
        }

        if !PATH_VALIDATOR.is_match(path) {
            return Err(SettingsRegistryPathConstructionError { path });
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
    pub const fn as_str(&self) -> &str {
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
        self == Self::root()
    }

    /// Returns the `SettingsRegistryPath` without its final component, if there is one.
    ///
    /// Returns [`None`] if the path terminates in a root.
    #[inline]
    pub fn parent(&self) -> Option<&Self> {
        let (parent, _) = self.split_parent();
        parent
    }

    /// Splits the `SettingsRegistryPath` into the first and its remaining components.
    #[inline]
    pub fn split_component(&self) -> (&Self, Option<&Self>) {
        let split_arr = |idx: usize| {
            let (component, rest) = self.path.split_at(idx + 1);
            let component = unsafe { Self::new_unchecked(component) };
            let rest = if rest.is_empty() {
                None
            } else if let Some(rest) = rest.strip_prefix("::") {
                unsafe { Some(Self::new_unchecked(rest)) }
            } else {
                unsafe { Some(Self::new_unchecked(rest)) }
            };

            (component, rest)
        };

        if let Some((first, rest)) = self.path.split_once("::") {
            if let Some(idx) = first.find(']') {
                split_arr(idx)
            } else {
                unsafe { (Self::new_unchecked(first), Some(Self::new_unchecked(rest))) }
            }
        } else if let Some(idx) = self.path.find(']') {
            split_arr(idx)
        } else {
            (self, None)
        }
    }

    /// Splits the `SettingsRegistryPath` into the parent `SettingsRegistryPath` and
    /// the last component, if they exist.
    #[inline]
    pub fn split_parent(&self) -> (Option<&Self>, &Self) {
        if self.is_root() {
            (None, self)
        } else if self.path.ends_with(']') {
            let component_idx = self.path.rfind('[').unwrap();
            let (parent, component) = self.path.split_at(component_idx);
            let parent = match parent {
                "" => Self::root(),
                s => unsafe { Self::new_unchecked(s) },
            };

            unsafe { (Some(parent), Self::new_unchecked(component)) }
        } else if let Some((parent, component)) = self.path.rsplit_once("::") {
            unsafe {
                (
                    Some(Self::new_unchecked(parent)),
                    Self::new_unchecked(component),
                )
            }
        } else {
            (Some(Self::root()), self)
        }
    }

    /// Joins two paths.
    #[inline]
    pub fn join<P: AsRef<SettingsRegistryPath>>(&self, path: P) -> SettingsRegistryPathBuf {
        let mut buf = self.to_path_buf();
        buf.push(path);
        buf
    }

    /// Joins the path with a string.
    #[inline]
    pub fn join_str<'a>(
        &self,
        string: &'a str,
    ) -> Result<SettingsRegistryPathBuf, SettingsRegistryPathConstructionError<&'a str>> {
        let path = SettingsRegistryPath::new(string)?;
        Ok(self.join(path))
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

impl PartialEq<str> for SettingsRegistryPath {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}
impl PartialEq<&'_ str> for SettingsRegistryPath {
    #[inline]
    fn eq(&self, other: &&'_ str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<String> for SettingsRegistryPath {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<SettingsRegistryPathBuf> for SettingsRegistryPath {
    #[inline]
    fn eq(&self, other: &SettingsRegistryPathBuf) -> bool {
        self.eq(other.as_path())
    }
}

/// Possible error of the [`SettingsRegistryPath::new`] function.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SettingsRegistryPathConstructionError<T: ?Sized> {
    path: T,
}

impl<'a> SettingsRegistryPathConstructionError<&'a str> {
    /// Extracts the path from the error.
    #[inline]
    pub fn path(&self) -> &'a str {
        self.path
    }
}

impl SettingsRegistryPathConstructionError<String> {
    /// Extracts the path from the error.
    #[inline]
    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}

/// Path component iterator.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SettingsRegistryPathComponentIter<'a> {
    path: Option<&'a SettingsRegistryPath>,
}

impl<'a> SettingsRegistryPathComponentIter<'a> {
    /// Constructs a new `SettingsRegistryPathComponentIter`.
    #[inline]
    pub fn new(path: &'a SettingsRegistryPath) -> Self {
        Self { path: Some(path) }
    }
}

impl<'a> Iterator for SettingsRegistryPathComponentIter<'a> {
    type Item = SettingsRegistryPathComponent<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let path = self.path?;
        let (component, rest) = path.split_component();
        if component.is_root() {
            return None;
        }
        self.path = rest;

        // we know that it matches.
        let captures = PATH_COMPONENT_VALIDATOR
            .captures(component.as_str())
            .unwrap();

        if let Some(name) = captures.name("name") {
            let name = name.as_str();
            Some(SettingsRegistryPathComponent::Item {
                name,
                path: component,
            })
        } else {
            let name = captures.name("arr_name").map(|n| n.as_str());
            let index: usize = captures.name("index").unwrap().as_str().parse().unwrap();
            Some(SettingsRegistryPathComponent::ArrayItem {
                name,
                index,
                path: component,
            })
        }
    }
}

/// Component of a path.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum SettingsRegistryPathComponent<'a> {
    /// Normal item.
    Item {
        /// Name of the item.
        name: &'a str,
        /// Path representation of the component.
        path: &'a SettingsRegistryPath,
    },
    /// An array item.
    ArrayItem {
        /// Name of the array.
        name: Option<&'a str>,
        /// Index of the element.
        index: usize,
        /// Path representation of the component.
        path: &'a SettingsRegistryPath,
    },
}

impl SettingsRegistryPathComponent<'_> {
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

    /// Extracts the name of the component.
    #[inline]
    pub fn name(&self) -> Option<&str> {
        match *self {
            SettingsRegistryPathComponent::Item { name, .. } => Some(name),
            SettingsRegistryPathComponent::ArrayItem { name, .. } => name,
        }
    }

    /// Extracts the index of an array component.
    #[inline]
    pub fn index(&self) -> Option<usize> {
        match *self {
            SettingsRegistryPathComponent::Item { .. } => None,
            SettingsRegistryPathComponent::ArrayItem { index, .. } => Some(index),
        }
    }

    /// Extracts the path of the component.
    #[inline]
    pub fn as_path(&self) -> &SettingsRegistryPath {
        match *self {
            SettingsRegistryPathComponent::Item { path, .. } => path,
            SettingsRegistryPathComponent::ArrayItem { path, .. } => path,
        }
    }
}

/// An owned mutable path.
#[derive(Debug, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct SettingsRegistryPathBuf {
    path: String,
}

impl SettingsRegistryPathBuf {
    /// Allocates an empty `SettingsRegistryPathBuf`.
    #[inline]
    pub fn new() -> Self {
        Self {
            path: String::from(SettingsRegistryPath::root().as_str()),
        }
    }

    /// Constructs a new `SettingsRegistryPathBuf`.
    #[inline]
    pub fn from_string<S: AsRef<str>>(
        path: S,
    ) -> Result<Self, SettingsRegistryPathConstructionError<String>> {
        match SettingsRegistryPath::new(path.as_ref()) {
            Ok(p) => Ok(p.to_path_buf()),
            Err(e) => Err(SettingsRegistryPathConstructionError {
                path: String::from(e.path),
            }),
        }
    }

    /// Creates a new `SettingsRegistryPathBuf` with a given capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = SettingsRegistryPath::root().as_str().len().max(capacity);
        let mut path = String::with_capacity(capacity);
        path.push_str(SettingsRegistryPath::root().as_str());

        Self { path }
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
            if !path.as_str().starts_with('[') {
                self.path.push_str("::");
            }
        } else {
            self.path.clear()
        }

        self.path.push_str(path.as_str());
    }

    /// Tries to extend `self` with `string`.
    ///
    /// Helper function for calling [`SettingsRegistryPathBuf::push`] without explicitly
    /// constructing a [`SettingsRegistryPath`].
    #[inline]
    pub fn push_str<'a>(
        &mut self,
        string: &'a str,
    ) -> Result<(), SettingsRegistryPathConstructionError<&'a str>> {
        match SettingsRegistryPath::new(string) {
            Ok(p) => {
                self.push(p);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Truncates `self` to [`self.parent`].
    ///
    /// Returns `false` and does nothing if [`self.parent`] is [`None`].
    /// Otherwise, returns `true`.
    #[inline]
    pub fn pop(&mut self) -> bool {
        if let Some(parent) = self.parent() {
            if parent.is_root() {
                self.path.clear();
                self.path.push_str(SettingsRegistryPath::root().as_str());
            } else {
                let parent_bytes = parent.as_str().as_bytes();
                let parent_bytes_len = parent_bytes.len();
                let _ = self.path.drain(parent_bytes_len..);
            }

            true
        } else {
            false
        }
    }

    /// Returns the length of the path in bytes.
    #[inline]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.path.len()
    }

    /// Returns the capacity of the `SettingsRegistryPathBuf`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.path.capacity()
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

impl Borrow<SettingsRegistryPath> for SettingsRegistryPathBuf {
    #[inline]
    fn borrow(&self) -> &SettingsRegistryPath {
        self.as_path()
    }
}

impl PartialEq<str> for SettingsRegistryPathBuf {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_path().eq(other)
    }
}

impl PartialEq<&'_ str> for SettingsRegistryPathBuf {
    #[inline]
    fn eq(&self, other: &&'_ str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<String> for SettingsRegistryPathBuf {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_path().eq(other)
    }
}

impl PartialEq<SettingsRegistryPath> for SettingsRegistryPathBuf {
    #[inline]
    fn eq(&self, other: &SettingsRegistryPath) -> bool {
        self.as_path().eq(other)
    }
}

impl PartialEq<&'_ SettingsRegistryPath> for SettingsRegistryPathBuf {
    #[inline]
    fn eq(&self, other: &&'_ SettingsRegistryPath) -> bool {
        self.eq(*other)
    }
}

#[cfg(test)]
mod test {
    use crate::rust::settings_registry::SettingsRegistryPath;

    #[test]
    fn new_path() {
        let _ = SettingsRegistryPath::new("object").unwrap();
        let _ = SettingsRegistryPath::new("[0]").unwrap();
        let _ = SettingsRegistryPath::new("arr[2]").unwrap();
        let _ = SettingsRegistryPath::new("sub[2][5]").unwrap();
        let _ = SettingsRegistryPath::new("map::element").unwrap();
        let _ = SettingsRegistryPath::new("map::arr[2]").unwrap();
        let _ = SettingsRegistryPath::new("map::sub[2][5]").unwrap();

        assert!(SettingsRegistryPath::new("").is_err());
        assert!(SettingsRegistryPath::new(":").is_err());
        assert!(SettingsRegistryPath::new("::").is_err());
        assert!(SettingsRegistryPath::new("map::[5]").is_err());
        assert!(SettingsRegistryPath::new("map::element[5]element").is_err());
    }

    #[test]
    fn root() {
        let root = SettingsRegistryPath::root();
        assert!(root.is_root());

        let p = SettingsRegistryPath::new("object").unwrap();
        assert!(!p.is_root());
    }

    #[test]
    fn split_components() {
        let root = SettingsRegistryPath::root();
        let (first, rest) = root.split_component();
        assert_eq!(first, root);
        assert_eq!(rest, None);

        let p1 = SettingsRegistryPath::new("map::arr[0][1][2]::element::a[0]").unwrap();

        let (first, rest) = p1.split_component();
        let rest = rest.unwrap();
        assert_eq!(first, "map");
        assert_eq!(rest, "arr[0][1][2]::element::a[0]");

        let (first, rest) = rest.split_component();
        let rest = rest.unwrap();
        assert_eq!(first, "arr[0]");
        assert_eq!(rest, "[1][2]::element::a[0]");

        let (first, rest) = rest.split_component();
        let rest = rest.unwrap();
        assert_eq!(first, "[1]");
        assert_eq!(rest, "[2]::element::a[0]");

        let (first, rest) = rest.split_component();
        let rest = rest.unwrap();
        assert_eq!(first, "[2]");
        assert_eq!(rest, "element::a[0]");

        let (first, rest) = rest.split_component();
        let rest = rest.unwrap();
        assert_eq!(first, "element");
        assert_eq!(rest, "a[0]");

        let (first, rest) = rest.split_component();
        assert_eq!(first, "a[0]");
        assert_eq!(rest, None);
    }

    #[test]
    fn split_parent() {
        let root = SettingsRegistryPath::root();
        let (parent, component) = root.split_parent();
        assert_eq!(parent, None);
        assert_eq!(component, root);

        let p1 = SettingsRegistryPath::new("map::arr[0][1][2]::element::a[0]").unwrap();

        let (parent, component) = p1.split_parent();
        let parent = parent.unwrap();
        assert_eq!(parent, "map::arr[0][1][2]::element::a");
        assert_eq!(component, "[0]");

        let (parent, component) = parent.split_parent();
        let parent = parent.unwrap();
        assert_eq!(parent, "map::arr[0][1][2]::element");
        assert_eq!(component, "a");

        let (parent, component) = parent.split_parent();
        let parent = parent.unwrap();
        assert_eq!(parent, "map::arr[0][1][2]");
        assert_eq!(component, "element");

        let (parent, component) = parent.split_parent();
        let parent = parent.unwrap();
        assert_eq!(parent, "map::arr[0][1]");
        assert_eq!(component, "[2]");

        let (parent, component) = parent.split_parent();
        let parent = parent.unwrap();
        assert_eq!(parent, "map::arr[0]");
        assert_eq!(component, "[1]");

        let (parent, component) = parent.split_parent();
        let parent = parent.unwrap();
        assert_eq!(parent, "map::arr");
        assert_eq!(component, "[0]");

        let (parent, component) = parent.split_parent();
        let parent = parent.unwrap();
        assert_eq!(parent, "map");
        assert_eq!(component, "arr");

        let (parent, component) = parent.split_parent();
        assert_eq!(parent, Some(root));
        assert_eq!(component, "map");
    }

    #[test]
    fn join_paths() {
        let root = SettingsRegistryPath::root();
        let p1 = SettingsRegistryPath::new("obj").unwrap();
        let p2 = SettingsRegistryPath::new("obj2").unwrap();
        let p3 = SettingsRegistryPath::new("arr[2][7]").unwrap();
        let p4 = SettingsRegistryPath::new("[0]").unwrap();

        let j = root.join(root);
        assert_eq!(j, root);

        let j = p1.join(root);
        assert_eq!(j, p1);

        let j = p1.join(p2);
        assert_eq!(j, "obj::obj2");

        let j = p3.join(p4);
        assert_eq!(j, "arr[2][7][0]");

        let j = p2.join(p4);
        assert_eq!(j, "obj2[0]");
    }

    #[test]
    fn push_path() {
        let mut path = SettingsRegistryPath::root().to_path_buf();
        assert_eq!(path, SettingsRegistryPath::root());

        path.push_str("[7]").unwrap();
        assert_eq!(path, "[7]");
        path.push_str("map").unwrap();
        assert_eq!(path, "[7]::map");
        path.push_str("arr[0]").unwrap();
        assert_eq!(path, "[7]::map::arr[0]");
        path.push_str("[1]").unwrap();
        assert_eq!(path, "[7]::map::arr[0][1]");
        path.push_str("element").unwrap();
        assert_eq!(path, "[7]::map::arr[0][1]::element");
    }

    #[test]
    fn pop_path() {
        let mut path = SettingsRegistryPath::new("map::arr[0][1][2]::element::a[0]")
            .unwrap()
            .to_path_buf();
        assert_eq!(path, "map::arr[0][1][2]::element::a[0]");

        assert!(path.pop());
        assert_eq!(path, "map::arr[0][1][2]::element::a");
        assert!(path.pop());
        assert_eq!(path, "map::arr[0][1][2]::element");
        assert!(path.pop());
        assert_eq!(path, "map::arr[0][1][2]");
        assert!(path.pop());
        assert_eq!(path, "map::arr[0][1]");
        assert!(path.pop());
        assert_eq!(path, "map::arr[0]");
        assert!(path.pop());
        assert_eq!(path, "map::arr");
        assert!(path.pop());
        assert_eq!(path, "map");
        assert!(path.pop());
        assert_eq!(path, SettingsRegistryPath::root());
        assert!(!path.pop());
    }
}
