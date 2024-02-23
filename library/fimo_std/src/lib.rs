//! Standard library used by the Fimo engine.
#![feature(extend_one)]
#![feature(allocator_api)]
#![feature(maybe_uninit_slice)]
#![feature(vec_into_raw_parts)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod allocator;
pub mod array_list;
pub mod bindings;
pub mod context;
pub mod error;
pub mod ffi;
pub mod graph;
pub mod refcount;
pub mod version;
