//! Implementation of basic fimo module loaders.
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_fn_trait_bound)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]

mod error;
mod interfaces;

pub mod rust_loader;

pub use error::{Error, ErrorKind, Result};
pub use fimo_ffi::{fimo_marker, fimo_object, fimo_vtable, impl_vtable, is_object};
pub use interfaces::*;

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Module information.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo<S = fimo_ffi::String> {
    /// Module name.
    pub name: S,
    /// Module version.
    pub version: S,
}

impl<S: std::fmt::Display> std::fmt::Display for ModuleInfo<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// A descriptor for a module interface.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInterfaceDescriptor<N = fimo_ffi::String, E = fimo_ffi::Vec<N>> {
    /// Name of the interface.
    pub name: N,
    /// Version of the interface.
    pub version: fimo_version_core::Version,
    /// Available interface extensions.
    pub extensions: E,
}

impl<N: AsRef<str>, M: AsRef<[N]>> ModuleInterfaceDescriptor<N, M> {
    /// Constructs a borrowed query.
    pub fn as_borrowed(&self) -> ModuleInterfaceDescriptor<&str, &[N]> {
        ModuleInterfaceDescriptor {
            name: self.name.as_ref(),
            version: self.version,
            extensions: self.extensions.as_ref(),
        }
    }
}

impl<N: std::fmt::Display> std::fmt::Display for ModuleInterfaceDescriptor<N> {
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
    Exact(fimo_version_core::Version),
    /// Query for a minimum version.
    Minimum(fimo_version_core::Version),
    /// Query for a range of versions.
    Range {
        /// Minimum supported version.
        min: fimo_version_core::Version,
        /// Maximum supported version.
        max: fimo_version_core::Version,
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
    /// use fimo_version_core::Version;
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
    ///
    /// [`Version`]: fimo_version_core::Version
    #[inline]
    pub fn query_matches(&self, version: fimo_version_core::Version) -> bool {
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
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceQuery<N = fimo_ffi::String, E = fimo_ffi::Vec<N>> {
    /// Name of the interface.
    pub name: N,
    /// Version of the interface.
    pub version: VersionQuery,
    /// Required extensions.
    pub extensions: E,
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

impl<N: AsRef<str>, M: AsRef<[N]>> InterfaceQuery<N, M> {
    /// Constructs a borrowed query.
    pub fn as_borrowed(&self) -> InterfaceQuery<&str, &[N]> {
        InterfaceQuery {
            name: self.name.as_ref(),
            version: self.version,
            extensions: self.extensions.as_ref(),
        }
    }

    /// Checks whether a [`ModuleInterfaceDescriptor`] matches the query.
    pub fn query_matches<O: AsRef<str>, P: AsRef<[N]>>(
        &self,
        desc: &ModuleInterfaceDescriptor<O, P>,
    ) -> bool
    where
        N: PartialEq,
    {
        let borrowed = self.as_borrowed();
        if desc.name.as_ref() == borrowed.name && borrowed.version.query_matches(desc.version) {
            let extensions = desc.extensions.as_ref();
            return borrowed
                .extensions
                .iter()
                .all(|ext| extensions.contains(ext));
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
