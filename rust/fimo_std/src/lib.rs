//! Standard library used by the Fimo engine.
#![feature(extend_one)]
#![feature(thread_local)]
#![feature(allocator_api)]
#![feature(unsafe_fields)]
#![feature(panic_update_hook)]
#![feature(result_flattening)]
#![feature(maybe_uninit_slice)]
#![feature(vec_into_raw_parts)]
#![feature(min_specialization)]
#![feature(anonymous_lifetime_in_impl_trait)]

#![allow(incomplete_features)]

#[doc(hidden)]
pub use paste;

pub mod bindings;

pub mod allocator;
pub mod context;
pub mod error;
pub mod ffi;
pub mod panic;
pub mod time;
pub mod version;

pub mod r#async;
pub mod module;
pub mod tracing;
