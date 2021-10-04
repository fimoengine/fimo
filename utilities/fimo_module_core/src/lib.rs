//! Implementation of basic fimo module loaders.
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_fn_trait_bound)]
#![feature(get_mut_unchecked)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
extern crate static_assertions as sa;

mod dyn_arc;
pub use dyn_arc::*;

pub mod rust;

/// Module information.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct ModuleInfo {
    /// Module name.
    pub name: fimo_ffi_core::ArrayString<32>,
    /// Module version.
    pub version: fimo_ffi_core::ArrayString<32>,
}

impl std::fmt::Display for ModuleInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// A descriptor for a module interface.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct ModuleInterfaceDescriptor {
    /// Name of the interface.
    pub name: fimo_ffi_core::ArrayString<32>,
    /// Version of the interface.
    pub version: fimo_version_core::Version,
    /// Available interface extensions.
    pub extensions: fimo_ffi_core::ConstSpan<fimo_ffi_core::ArrayString<32>>,
}

impl std::fmt::Display for ModuleInterfaceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

/// A raw pointer to module internals.
#[repr(C, i8)]
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub enum ModulePtr {
    /// A single pointer.
    Slim(*const u8),
    /// Two pointers.
    Fat((*const u8, *const u8)),
    /// Unspecified layout.
    Other([u8; 32]),
}
