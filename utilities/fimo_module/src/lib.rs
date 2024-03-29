//! Implementation of basic fimo module loaders.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(map_many_mut)]
#![feature(c_unwind)]
#![feature(unsize)]

use fimo_ffi::marshal::CTypeBridge;

pub mod context;
pub mod module;

pub use fimo_ffi;
pub use fimo_ffi::error::{self, Error, ErrorKind};
pub use fimo_ffi::version::{self, ReleaseType, Version};

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Result type for modules.
pub type Result<T> = std::result::Result<T, Error>;

/// FFi-safe result type for modules.
pub type FFIResult<T> = fimo_ffi::result::Result<T, Error>;

/// Module information.
#[repr(C)]
#[derive(
    Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, CTypeBridge,
)]
pub struct ModuleInfo {
    /// Module name.
    pub name: fimo_ffi::String,
    /// Module version.
    pub version: fimo_ffi::String,
}

impl std::fmt::Display for ModuleInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// Descriptor of a dependency.
#[repr(C)]
#[derive(
    Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, CTypeBridge,
)]
pub struct InterfaceDependency {
    /// Name of the interface.
    pub name: fimo_ffi::String,
    /// Version of the interface.
    pub version: Version,
    /// Available interface extensions.
    pub extensions: fimo_ffi::Vec<fimo_ffi::String>,
    /// Dependency is optional.
    pub optional: bool,
}

impl std::fmt::Display for InterfaceDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// A descriptor for a module interface.
#[repr(C)]
#[derive(
    Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, CTypeBridge,
)]
pub struct InterfaceDescriptor {
    /// Name of the interface.
    pub name: fimo_ffi::String,
    /// Version of the interface.
    pub version: Version,
    /// Available interface extensions.
    pub extensions: fimo_ffi::Vec<fimo_ffi::String>,
}

impl InterfaceDescriptor {
    /// Constructs a new descriptor.
    #[inline]
    pub fn new(
        name: fimo_ffi::String,
        version: Version,
        extensions: fimo_ffi::Vec<fimo_ffi::String>,
    ) -> Self {
        Self {
            name,
            version,
            extensions,
        }
    }
}

impl std::fmt::Display for InterfaceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// A version query.
#[repr(C, i8)]
#[derive(
    Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, CTypeBridge,
)]
pub enum VersionQuery {
    /// Query for an exact version.
    Exact(Version),
    /// Query for a minimum version.
    Minimum(Version),
    /// Query for a range of versions.
    Range {
        /// Minimum supported version.
        min: Version,
        /// Maximum supported version.
        max: Version,
    },
}

impl VersionQuery {
    /// Indicates whether a [`Version`] matches.
    ///
    /// # Note
    ///
    /// Versions before `1.0.0` behave differently.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Version;
    /// use fimo_module::VersionQuery;
    ///
    /// let q = VersionQuery::Exact(Version::new_short(1, 0, 0));
    /// assert!(q.query_matches(Version::new_short(1, 0, 0)));
    /// assert!(!q.query_matches(Version::new_short(1, 0, 5)));
    ///
    /// let q = VersionQuery::Minimum(Version::new_short(1, 0, 0));
    /// assert!(q.query_matches(Version::new_short(1, 0, 0)));
    /// assert!(q.query_matches(Version::new_short(1, 2, 0)));
    /// assert!(!q.query_matches(Version::new_short(2, 0, 0)));
    ///
    /// let q = VersionQuery::Range {
    ///     min: Version::new_short(1, 0, 0),
    ///     max: Version::new_short(1, 3, 0)
    /// };
    /// assert!(q.query_matches(Version::new_short(1, 0, 0)));
    /// assert!(q.query_matches(Version::new_short(1, 2, 3)));
    /// assert!(q.query_matches(Version::new_short(1, 3, 0)));
    /// assert!(!q.query_matches(Version::new_short(1, 3, 1)));
    /// ```
    #[inline]
    pub fn query_matches(&self, version: Version) -> bool {
        match *self {
            VersionQuery::Exact(v) => v == version,
            VersionQuery::Minimum(v) => v.is_compatible(&version),
            VersionQuery::Range { min, max } => min.is_compatible(&version) && (version <= max),
        }
    }
}

/// A query for an interface.
#[repr(C)]
#[derive(
    Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize, CTypeBridge,
)]
pub struct InterfaceQuery {
    /// Name of the interface.
    pub name: fimo_ffi::String,
    /// Version of the interface.
    pub version: VersionQuery,
    /// Required extensions.
    pub extensions: fimo_ffi::Vec<fimo_ffi::String>,
}

impl InterfaceQuery {
    /// Constructs a new `InterfaceQuery`.
    #[inline]
    pub fn new(name: &str, version: VersionQuery) -> Self {
        Self {
            name: name.into(),
            version,
            extensions: Default::default(),
        }
    }

    /// Adds an extension to the query.
    #[inline]
    pub fn with(mut self, extension: &str) -> Self {
        self.extensions.push(extension.into());
        self
    }

    /// Checks whether a [`InterfaceDescriptor`] matches the query.
    pub fn query_matches(&self, desc: &InterfaceDescriptor) -> bool {
        if desc.name == self.name && self.version.query_matches(desc.version) {
            return self
                .extensions
                .iter()
                .all(|ext| desc.extensions.contains(ext));
        }

        false
    }
}

impl From<InterfaceDescriptor> for InterfaceQuery {
    fn from(v: InterfaceDescriptor) -> Self {
        Self {
            name: v.name,
            version: VersionQuery::Minimum(v.version),
            extensions: v.extensions,
        }
    }
}

impl From<&'_ InterfaceDescriptor> for InterfaceQuery {
    fn from(v: &InterfaceDescriptor) -> Self {
        Self {
            name: v.name.clone(),
            version: VersionQuery::Minimum(v.version),
            extensions: v.extensions.clone(),
        }
    }
}

/// Type of a path character.
#[cfg(unix)]
pub type PathChar = u8;

/// Type of a path character.
#[cfg(windows)]
pub type PathChar = u16;

/// Helper trait for building a [`InterfaceQuery`].
pub trait Queryable {
    /// Name of the interface.
    const NAME: &'static str;

    /// Currently defined version of the interface.
    const CURRENT_VERSION: Version;

    /// Extensions available to the interface.
    const EXTENSIONS: &'static [(Option<Version>, &'static str)];
}

/// Helper type for building an [`InterfaceQuery`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryBuilder;

impl QueryBuilder {
    /// Name of the [`Queryable`].
    pub const fn name<Q: Queryable + ?Sized>(&self) -> &'static str {
        Q::NAME
    }

    /// Current defined version of the [`Queryable`].
    pub const fn current_version<Q: Queryable + ?Sized>(&self) -> Version {
        Q::CURRENT_VERSION
    }

    /// Construct an [`InterfaceQuery`] that is compatible with the current version.
    pub fn query_current<Q: Queryable + ?Sized>(&self) -> InterfaceQuery {
        self.query_version::<Q>(VersionQuery::Minimum(Q::CURRENT_VERSION))
    }

    /// Construct an [`InterfaceQuery`] that only matches the current version.
    pub fn query_current_exact<Q: Queryable + ?Sized>(&self) -> InterfaceQuery {
        self.query_version::<Q>(VersionQuery::Exact(Q::CURRENT_VERSION))
    }

    /// Constructs an [`InterfaceQuery`] that matches a [`VersionQuery`].
    pub fn query_version<Q: Queryable + ?Sized>(&self, version: VersionQuery) -> InterfaceQuery {
        let extensions_version = match version {
            VersionQuery::Exact(x) | VersionQuery::Minimum(x) => {
                assert!(x <= Q::CURRENT_VERSION);
                x
            }
            VersionQuery::Range { min, max } => {
                assert!(min.is_compatible(&max));
                assert!(max <= Q::CURRENT_VERSION);
                min
            }
        };

        let mut query = InterfaceQuery::new(Q::NAME, version);
        for (ext_version, ext_name) in Q::EXTENSIONS {
            if let Some(ext_version) = ext_version {
                if ext_version.is_compatible(&extensions_version) {
                    query = query.with(ext_name);
                }
            }
        }

        query
    }
}
