//! Synchronization primitives
//!
//! This module provides Runtime-aware implementations
//! of `Mutex`, `RWLock` and `Condvar`.

mod mutex;
mod spin_wait;

pub use mutex::{Mutex, MutexGuard};
