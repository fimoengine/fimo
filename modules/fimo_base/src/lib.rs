//! Base module of the Fimo Engine
//! Implements the `emf-core-base` interface.
#![feature(const_fn_trait_bound)]
#![feature(allocator_api)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    broken_intra_doc_links
)]

mod data_guard;
mod fimo_base;
mod key_generator;

pub mod base_interface;
pub mod module_interface;
pub use fimo_ffi_core as ffi;

pub(crate) use data_guard::{DataGuard, Locked, Unlocked};
pub use fimo_base::FimoBase;
pub(crate) use key_generator::KeyGenerator;
