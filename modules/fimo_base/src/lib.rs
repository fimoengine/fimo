//! Base module of the Fimo Engine
//! Implements the `emf-core-base` interface.
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    broken_intra_doc_links
)]

mod base_interface;
mod fimo_base;

pub use fimo_base::FimoBase;

pub mod base_api;
pub mod module_interface;
