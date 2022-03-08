//! Implementation of basic fimo module loaders.
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(unsize)]
#![feature(c_unwind)]

mod interfaces;

pub mod loader;
pub mod module;

pub use interfaces::*;

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
#[derive(Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
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

/// A descriptor for a module interface.
#[repr(C)]
#[derive(Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInterfaceDescriptor {
    /// Name of the interface.
    pub name: fimo_ffi::String,
    /// Version of the interface.
    pub version: Version,
    /// Available interface extensions.
    pub extensions: fimo_ffi::Vec<fimo_ffi::String>,
}

impl ModuleInterfaceDescriptor {
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

impl std::fmt::Display for ModuleInterfaceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// A version query.
#[repr(C, i8)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionQuery {
    /// Matches any version.
    Any,
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
    /// let q = VersionQuery::Any;
    /// assert!(q.query_matches(Version::new_short(1, 0, 0)));
    /// assert!(q.query_matches(Version::new_short(0, 0, 5)));
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
            VersionQuery::Any => true,
            VersionQuery::Exact(v) => v == version,
            VersionQuery::Minimum(v) => v.is_compatible(&version),
            VersionQuery::Range { min, max } => min.is_compatible(&version) && (version <= max),
        }
    }
}

/// A query for an interface.
#[repr(C)]
#[derive(Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl InterfaceQuery {
    /// Checks whether a [`ModuleInterfaceDescriptor`] matches the query.
    pub fn query_matches(&self, desc: &ModuleInterfaceDescriptor) -> bool {
        if desc.name == self.name && self.version.query_matches(desc.version) {
            return self
                .extensions
                .iter()
                .all(|ext| desc.extensions.contains(ext));
        }

        false
    }
}

/// Type of a path character.
#[cfg(unix)]
pub type PathChar = u8;

/// Type of a path character.
#[cfg(windows)]
pub type PathChar = u16;
