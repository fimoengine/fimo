//! Synchronization primitives
//!
//! This module provides Runtime-aware implementations
//! of `Mutex`, `RWLock` and `Condvar`.

mod condvar;
mod mutex;
mod spin_wait;

pub use condvar::Condvar;
pub use mutex::{Mutex, MutexGuard};
