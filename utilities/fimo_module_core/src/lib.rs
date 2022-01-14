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
pub use fimo_ffi::{fimo_object, fimo_vtable};
pub use interfaces::*;

use std::fmt::Debug;

/// Module information.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct ModuleInfo<S = fimo_ffi_core::ArrayString<128>> {
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
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct ModuleInterfaceDescriptor {
    /// Name of the interface.
    pub name: fimo_ffi_core::ArrayString<128>,
    /// Version of the interface.
    pub version: fimo_version_core::Version,
    /// Available interface extensions.
    pub extensions: fimo_ffi_core::ConstSpan<fimo_ffi_core::ArrayString<128>>,
}

impl std::fmt::Display for ModuleInterfaceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// Type of a path character.
#[cfg(unix)]
pub type PathChar = u8;

/// Type of a path character.
#[cfg(windows)]
pub type PathChar = u16;

/// Marker type that implements `Send` and `Sync`.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SendSyncMarker;
