//! Standard library used by the Fimo engine.
#![feature(extend_one)]
#![feature(allocator_api)]
#![feature(maybe_uninit_slice)]
#![feature(vec_into_raw_parts)]
#![feature(min_specialization)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[doc(hidden)]
pub use paste;

pub mod allocator;
pub mod array_list;
pub mod bindings;
pub mod context;
pub mod error;
pub mod ffi;
pub mod graph;
pub mod module;
pub mod refcount;
pub mod time;
pub mod version;
