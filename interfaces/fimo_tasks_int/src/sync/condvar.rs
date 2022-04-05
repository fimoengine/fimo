use std::{fmt::Debug, pin::Pin, sync::atomic::AtomicPtr};

use fimo_ffi::ObjBox;

use crate::{
    runtime::{current_runtime, IRuntimeExt, IScheduler},
    task::{Builder, JoinHandle, Task},
};

use super::{mutex::RawMutex, MutexGuard};

/// A Condition Variable
///
/// Condition variables represent the ability to block a task such that it
/// consumes no CPU time while waiting for an event to occur. Condition
/// variables are typically associated with a boolean predicate (a condition)
/// and a mutex. The predicate is always verified inside of the mutex before
/// determining that a task must block.
///
/// Functions in this module will block the current **task**.
/// Note that any attempt to use multiple mutexes on the same condition
/// variable may result in a runtime panic.
pub struct Condvar {
    state: AtomicPtr<RawMutex>,
    raw: JoinHandle<Pin<ObjBox<Task<'static, ()>>>>,
}

impl Condvar {
    /// Creates a new condition variable which is ready to be waited on and
    /// notified.
    #[inline]
    pub fn new() -> Condvar {
        let raw = Builder::new()
            .blocked()
            .spawn(|| {}, &[])
            .expect("could not create condvar task");

        Self {
            state: AtomicPtr::new(std::ptr::null_mut()),
            raw,
        }
    }

    /// Blocks the current task until this condition variable receives a
    /// notification.
    ///
    /// This function will atomically unlock the mutex specified (represented by
    /// `guard`) and block the current task. This means that any calls
    /// to [`notify_one`] or [`notify_all`] which happen logically after the
    /// mutex is unlocked are candidates to wake this task up. When this
    /// function call returns, the lock specified will have been re-acquired.
    ///
    /// Unlike the implementation in the std library, this function is not
    /// susceptible to spurious wakeups.
    ///
    /// [`notify_one`]: Self::notify_one
    /// [`notify_all`]: Self::notify_all
    #[inline]
    pub fn wait<'a, T>(&self, guard: &mut MutexGuard<'a, T>) {
        let runtime = current_runtime().unwrap();
        let raw = guard.as_raw();

        runtime.yield_and_enter(|s, curr| {
            let state = self.state.load(atomic::Ordering::Relaxed);
            let state_const = state as *const RawMutex;

            let preexisting_mutex = !state_const.is_null();

            // Check that we are using the Condvar with only one mutex.
            if preexisting_mutex && state_const != raw as *const RawMutex {
                panic!("can not wait with differing mutexes")
            }

            if !preexisting_mutex {
                self.state
                    .store(raw as *const _ as *mut _, atomic::Ordering::Release);
            }

            // Wait on the condvar task.
            let wait = unsafe { s.wait_task_on(curr, self.raw.as_raw(), None) };
            if let Err(e) = wait {
                // On an error we abort and panic
                if !preexisting_mutex {
                    self.state
                        .store(std::ptr::null_mut(), atomic::Ordering::Release);
                }

                panic!("{}", e)
            }

            // Unlock the mutex.
            unsafe { raw.unlock_condvar(s) };
        });

        // Lock the mutex again.
        raw.lock()
    }

    /// Blocks the current task until this condition variable receives a
    /// notification and the provided condition is false.
    ///
    /// This function will atomically unlock the mutex specified (represented by
    /// `guard`) and block the current task. This means that any calls
    /// to [`notify_one`] or [`notify_all`] which happen logically after the
    /// mutex is unlocked are candidates to wake this task up. When this
    /// function call returns, the lock specified will have been re-acquired.
    ///
    /// [`notify_one`]: Self::notify_one
    /// [`notify_all`]: Self::notify_all
    #[inline]
    pub fn wait_while<'a, T, F>(&self, guard: &mut MutexGuard<'a, T>, mut condition: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        while condition(&mut *guard) {
            self.wait(guard);
        }
    }

    /// Wakes up one blocked task on this condvar.
    ///
    /// Returns whether a thread was woken up.
    ///
    /// If there is a blocked task on this condition variable, then it will
    /// be woken up from its call to [`wait`]. Calls to `notify_one` are
    /// not buffered in any way.
    ///
    /// To wake up all task, see [`notify_all`].
    ///
    /// [`wait`]: Self::wait
    /// [`notify_all`]: Self::notify_all
    #[inline]
    pub fn notify_one(&self) -> bool {
        let runtime = current_runtime().unwrap();

        runtime.enter_scheduler(|s, _| {
            let task_woken = unsafe {
                s.notify_one(self.raw.as_raw(), crate::raw::WakeupData::None)
                    .expect("could not wake task")
            };

            // If there are no more waiting tasks we clear the mutex state.
            if let Some(remaining) = task_woken {
                if remaining == 0 {
                    self.state
                        .store(std::ptr::null_mut(), atomic::Ordering::Release);
                }
                true
            } else {
                false
            }
        })
    }

    /// Wakes up all blocked task on this condvar.
    ///
    /// Returns the number of threads woken up.
    ///
    /// This method will ensure that any current waiters on the condition
    /// variable are awoken. Calls to `notify_all()` are not buffered in any
    /// way.
    ///
    /// To wake up only one thread, see [`notify_one`].
    ///
    /// [`notify_one`]: Self::notify_one
    #[inline]
    pub fn notify_all(&self) -> usize {
        let runtime = current_runtime().unwrap();

        runtime.enter_scheduler(|s, _| {
            let num = unsafe {
                s.notify_all(self.raw.as_raw(), crate::raw::WakeupData::None)
                    .expect("could not wake task")
            };

            // Cleanup the local state.
            self.state
                .store(std::ptr::null_mut(), atomic::Ordering::Release);

            num
        })
    }
}

unsafe impl Send for Condvar {}
unsafe impl Sync for Condvar {}

impl Default for Condvar {
    #[inline]
    fn default() -> Self {
        Condvar::new()
    }
}

impl Debug for Condvar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Condvar").finish_non_exhaustive()
    }
}

impl Drop for Condvar {
    fn drop(&mut self) {
        self.raw
            .unblock()
            .expect("could not unblock the condvar task")
    }
}
