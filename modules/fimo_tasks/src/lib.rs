//! Implementation of the `fimo-tasks` interface.
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
#![feature(thread_local)]

mod runtime;
mod scheduler;
mod spin_wait;
mod stack_allocator;
mod task_manager;
mod worker_pool;

pub use runtime::{Builder, Runtime};
pub use scheduler::TaskScheduler;
