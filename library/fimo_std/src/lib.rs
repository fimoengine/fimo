//! Standard library used by the Fimo engine.
#![feature(allocator_api)]
#![no_std]

extern crate alloc;

pub mod allocator;
pub mod bindings;
pub mod error;
pub mod ffi;
