// Taken from the implementation of the std Barrier.
use crate::sync::{Condvar, Mutex};
use std::fmt::{Debug, Formatter};

/// A barrier enables multiple threads to synchronize the beginning
/// of some computation.
pub struct Barrier {
    lock: Mutex<BarrierState>,
    cvar: Condvar,
    num_threads: usize,
}

struct BarrierState {
    count: usize,
    generation_id: usize,
}

/// Result of a `Barrier` wait operation.
pub struct BarrierWaitResult(bool);

impl Debug for Barrier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Barrier").finish_non_exhaustive()
    }
}

impl Barrier {
    /// Constructs a new `Barrier` that can block a given number of tasks.
    ///
    /// The barrier will block the first `n-1` tasks which call [`Barrier::wait()`]
    /// and then wake up all tasks at once when the `n`th tasks calls [`Barrier::wait()`].
    pub fn new(n: usize) -> Self {
        Self {
            lock: Mutex::new(BarrierState {
                count: 0,
                generation_id: 0,
            }),
            cvar: Condvar::new(),
            num_threads: n,
        }
    }

    /// Blocks the current task until all threads have rendezvoused here.
    ///
    /// Barriers are re-usable after all task have rendezvoused once,
    /// and can be used continuously.
    ///
    /// A single (arbitrary) task will receive a BarrierWaitResult that returns
    /// true from [`BarrierWaitResult::is_leader()`] when returning from this function,
    /// and all other task will receive a result that will return false from
    /// [`BarrierWaitResult::is_leader()`].
    pub fn wait(&self) -> BarrierWaitResult {
        let mut lock = self.lock.lock();
        let local_gen = lock.generation_id;
        lock.count += 1;
        if lock.count < self.num_threads {
            // We need a while loop to guard against spurious wakeups.
            // https://en.wikipedia.org/wiki/Spurious_wakeup
            //
            // Note: The task runtime already prevents spurious wakeups,
            // but this could change in the future.
            self.cvar.wait_while(&mut lock, |l| {
                local_gen == l.generation_id && l.count < self.num_threads
            });
            BarrierWaitResult(false)
        } else {
            lock.count = 0;
            lock.generation_id = lock.generation_id.wrapping_add(1);
            self.cvar.notify_all();
            BarrierWaitResult(true)
        }
    }
}

impl Debug for BarrierWaitResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BarrierWaitResult")
            .field("is_leader", &self.is_leader())
            .finish()
    }
}

impl BarrierWaitResult {
    /// Returns true if this task is the “leader task” for the call to [Barrier::wait()].
    ///
    /// Only one task will be designated as the leader.
    pub fn is_leader(&self) -> bool {
        self.0
    }
}
