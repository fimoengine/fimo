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
pub struct ModuleInterfaceDescriptor<N = fimo_ffi::String, E = fimo_ffi::Vec<fimo_ffi::String>> {
    /// Name of the interface.
    pub name: N,
    /// Version of the interface.
    pub version: fimo_version_core::Version,
    /// Available interface extensions.
    pub extensions: E,
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
