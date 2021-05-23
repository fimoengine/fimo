//! Base module of the Fimo Engine
//! Implements the `emf-core-base` interface.
#![feature(const_fn_trait_bound)]
#![feature(maybe_uninit_ref)]
#![feature(const_fn_union)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    broken_intra_doc_links
)]

mod base_interface;
mod fimo_base;
mod key_generator;

pub use fimo_base::FimoBase;
pub use key_generator::KeyGenerator;

pub mod base_api;
pub mod module_interface;
