//! Standard library used by the Fimo engine.
#![feature(allocator_api)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod allocator;
pub mod bindings;
pub mod error;
pub mod ffi;
