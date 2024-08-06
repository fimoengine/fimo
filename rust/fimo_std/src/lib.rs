//! Standard library used by the Fimo engine.
#![feature(extend_one)]
#![feature(thread_local)]
#![feature(allocator_api)]
#![feature(panic_update_hook)]
#![feature(result_flattening)]
#![feature(strict_provenance)]
#![feature(maybe_uninit_slice)]
#![feature(vec_into_raw_parts)]
#![feature(min_specialization)]
#![feature(const_refs_to_static)]

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
pub mod panic;
pub mod refcount;
pub mod time;
pub mod tracing;
pub mod version;
