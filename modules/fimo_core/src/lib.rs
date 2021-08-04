//! Core module of the Fimo Engine.
#![feature(const_fn_trait_bound)]
#![feature(allocator_api)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    broken_intra_doc_links
)]

mod core_module;

pub mod core_interface;
pub use fimo_ffi_core as ffi;

pub use core_module::FimoCore;
