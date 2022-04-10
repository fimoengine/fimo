use fimo_ffi::FfiFn;

use super::{mutex::RawMutex, MutexGuard};
use crate::runtime::{current_runtime, IRuntimeExt, IScheduler, NotifyResult, WaitToken};
use std::{fmt::Debug, mem::MaybeUninit, sync::atomic::AtomicPtr};

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
            let wait = unsafe { s.wait_task_on(curr, task, None, WaitToken::INVALID) };
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
        // Nothing to do if there are no waiting threads
        let state = self.state.load(atomic::Ordering::Relaxed);
        if state.is_null() {
            return false;
        }

        self.notify_one_slow(state)
    }

    #[cold]
    fn notify_one_slow(&self, mutex: *mut RawMutex) -> bool {
        let runtime = current_runtime().unwrap();

        runtime.enter_scheduler(|s, _| {
            // Make sure that our atomic state still points to the same
            // mutex. If not then it means that all threads on the current
            // mutex were woken up and a new waiting thread switched to a
            // different mutex. In that case we can get away with doing
            // nothing.
            if self.state.load(atomic::Ordering::Relaxed) != mutex {
                return false;
            }

            let self_addr = self as *const _ as *const ();
            let task = unsafe {
                s.register_or_fetch_pseudo_task(self_addr)
                    .expect("could not fetch condvar task")
            };

            let callback = |result: NotifyResult| {
                if !result.has_tasks_remaining() {
                    self.state
                        .store(std::ptr::null_mut(), atomic::Ordering::Release);
                }
                crate::runtime::WakeupToken::None
            };
            let mut callback = MaybeUninit::new(callback);
            let callback = unsafe { FfiFn::new_value(&mut callback) };

            let result = unsafe { s.notify_one(task, callback).expect("could not wake task") };

            if !result.has_tasks_remaining() {
                unsafe {
                    s.unregister_pseudo_task(task)
                        .expect("could not unregister condvar task");
                }
            }

            result.has_notified_tasks()
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
        // Nothing to do if there are no waiting threads
        let state = self.state.load(atomic::Ordering::Relaxed);
        if state.is_null() {
            return 0;
        }

        self.notify_all_slow(state)
    }

    #[cold]
    fn notify_all_slow(&self, mutex: *mut RawMutex) -> usize {
        let runtime = current_runtime().unwrap();

        runtime.enter_scheduler(|s, _| {
            // Make sure that our atomic state still points to the same
            // mutex. If not then it means that all threads on the current
            // mutex were woken up and a new waiting thread switched to a
            // different mutex. In that case we can get away with doing
            // nothing.
            if self.state.load(atomic::Ordering::Relaxed) != mutex {
                return 0;
            }

            let self_addr = self as *const _ as *const ();
            let task = unsafe {
                s.register_or_fetch_pseudo_task(self_addr)
                    .expect("could not fetch condvar task")
            };

            let num = unsafe {
                s.notify_all(task, crate::runtime::WakeupToken::None)
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
