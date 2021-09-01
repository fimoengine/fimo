// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::rust::get_runtime;

const SLEEP_THRESHOLD: usize = 10;
const YIELD_THRESHOLDS: usize = 3;

/// A counter used to perform exponential backoff in spin loops.
#[derive(Debug, Default)]
pub struct SpinWait {
    count: usize,
}

impl SpinWait {
    /// Creates a new `SpinWait`.
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// Resets a `SpinWait` to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.count = 0;
    }

    /// Spins until the sleep threshold has been reached.
    ///
    /// This function returns whether the sleep threshold has been reached, at
    /// which point further spinning has diminishing returns and the thread
    /// should be parked instead.
    ///
    /// The spin strategy will initially use a CPU-bound loop but will fall back
    /// to yielding the CPU to the runtime after a few iterations.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    #[inline]
    pub fn spin(&mut self) -> bool {
        if self.count >= SLEEP_THRESHOLD {
            return false;
        }

        self.count += 1;
        if self.count <= YIELD_THRESHOLDS {
            Self::spin_loop(1 << self.count);
        } else {
            get_runtime().yield_now()
        }

        true
    }

    /// Spins until the sleep threshold has been reached.
    ///
    /// This function returns whether the sleep threshold has been reached, at
    /// which point further spinning has diminishing returns and the thread
    /// should be parked instead.
    ///
    /// The spin strategy will initially use a CPU-bound loop but will fall back
    /// to yielding the CPU to the OS after a few iterations.
    #[inline]
    pub fn spin_thread(&mut self) -> bool {
        if self.count >= SLEEP_THRESHOLD {
            return false;
        }

        self.count += 1;
        if self.count <= YIELD_THRESHOLDS {
            Self::spin_loop(1 << self.count);
        } else {
            std::thread::yield_now();
        }

        true
    }

    #[inline]
    fn spin_loop(iterations: usize) {
        for _ in 0..iterations {
            std::hint::spin_loop()
        }
    }
}
