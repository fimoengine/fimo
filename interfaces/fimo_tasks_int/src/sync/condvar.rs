use super::{mutex::RawMutex, MutexGuard};
use crate::runtime::{current_runtime, IRuntimeExt, IScheduler};
use std::{fmt::Debug, sync::atomic::AtomicPtr};

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
}

impl Condvar {
    /// Creates a new condition variable which is ready to be waited on and
    /// notified.
    #[inline]
    pub const fn new() -> Condvar {
        Self {
            state: AtomicPtr::new(std::ptr::null_mut()),
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

            let self_addr = self as *const _ as *const ();
            let task = unsafe {
                s.register_or_fetch_pseudo_task(self_addr)
                    .expect("could not fetch condvar task")
            };

            if !preexisting_mutex {
                self.state
                    .store(raw as *const _ as *mut _, atomic::Ordering::Release);
            }

            // Wait on the condvar task.
            let wait = unsafe { s.pseudo_wait_task_on(curr, task, None) };
            if let Err(e) = wait {
                // On an error we abort and panic
                if !preexisting_mutex {
                    self.state
                        .store(std::ptr::null_mut(), atomic::Ordering::Release);

                    unsafe {
                        s.unregister_pseudo_task(task)
                            .expect("could not unregister condvar task");
                    }
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
    /// Returns whether a task was woken up.
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
            let self_addr = self as *const _ as *const ();
            let task = unsafe {
                s.register_or_fetch_pseudo_task(self_addr)
                    .expect("could not fetch condvar task")
            };

            let task_woken = unsafe {
                s.pseudo_notify_one(task, crate::raw::WakeupData::None)
                    .expect("could not wake task")
            };

            // If there are no more waiting tasks we clear the mutex state.
            if let Some(remaining) = task_woken {
                if remaining == 0 {
                    unsafe {
                        s.unregister_pseudo_task(task)
                            .expect("could not unregister condvar task");
                    }

                    self.state
                        .store(std::ptr::null_mut(), atomic::Ordering::Release);
                }
                true
            } else {
                unsafe {
                    s.unregister_pseudo_task(task)
                        .expect("could not unregister condvar task");
                }

                false
            }
        })
    }

    /// Wakes up all blocked task on this condvar.
    ///
    /// Returns the number of tasks woken up.
    ///
    /// This method will ensure that any current waiters on the condition
    /// variable are awoken. Calls to `notify_all()` are not buffered in any
    /// way.
    ///
    /// To wake up only one task, see [`notify_one`].
    ///
    /// [`notify_one`]: Self::notify_one
    #[inline]
    pub fn notify_all(&self) -> usize {
        let runtime = current_runtime().unwrap();

        runtime.enter_scheduler(|s, _| {
            let self_addr = self as *const _ as *const ();
            let task = unsafe {
                s.register_or_fetch_pseudo_task(self_addr)
                    .expect("could not fetch condvar task")
            };

            let num = unsafe {
                s.pseudo_notify_all(task, crate::raw::WakeupData::None)
                    .expect("could not wake task")
            };

            // Cleanup the local state.
            unsafe {
                s.unregister_pseudo_task(task)
                    .expect("could not unregister condvar task");
            }

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
