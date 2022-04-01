//! Implementation of the `fimo-tasks` interface.
#![forbid(clippy::undocumented_unsafe_blocks)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(thread_local)]
#![feature(c_unwind)]

mod runtime;
mod scheduler;
mod spin_wait;
mod worker_pool;

#[cfg(feature = "module")]
mod module;

pub use runtime::{Builder, Runtime};
pub use scheduler::TaskScheduler;
