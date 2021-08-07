//! Core module of the Fimo Engine.
#![feature(const_fn_trait_bound)]
#![feature(maybe_uninit_extra)]
#![feature(allocator_api)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]

pub mod core_interface;
#[cfg(feature = "module")]
pub mod core_module;

pub use core_interface::CoreInterface;
#[cfg(feature = "module")]
pub use core_module::FimoCore;
