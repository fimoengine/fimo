use std::{
    cell::{Cell, UnsafeCell},
    fmt::{Debug, Display},
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::atomic::AtomicU8,
    time::{Duration, Instant},
};

use crate::{
    raw::WakeupData,
    runtime::{current_runtime, IRuntimeExt, IScheduler},
};
use crate::{
    sync::spin_wait::SpinWait,
    task::{Builder, JoinHandle, Task},
};
use fimo_ffi::ObjBox;
use rand::SeedableRng;

/// A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available. The
/// mutex can also be statically initialized or created via a [`new`]
/// constructor. Each mutex has a type parameter which represents the data that
/// it is protecting. The data can only be accessed through the RAII guards
/// returned from [`lock`] and [`try_lock`], which guarantees that the data is only
/// ever accessed when the mutex is locked.
///
/// [`new`]: Mutex::new
/// [`lock`]: Mutex::lock
/// [`try_lock`]: Mutex::try_lock
pub struct Mutex<T: ?Sized> {
    raw: RawMutex,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub fn new(val: T) -> Self {
        Self {
            raw: RawMutex::new(),
            data: UnsafeCell::new(val),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires a mutex, blocking the current task until it is able to do so.
    ///
    /// This function will block the local task until it is available to acquire
    /// the mutex. Upon returning, the task is the only task with the lock
    /// held. An RAII guard is returned to allow scoped unlock of the lock. When
    /// the guard goes out of scope, the mutex will be unlocked.
    ///
    /// The exact behavior on locking a mutex in the task which already holds
    /// the lock is left unspecified. However, this function will not return on
    /// the second call (it might panic or deadlock, for example).
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.raw.lock();
        MutexGuard { lock: self }
    }

    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then [`None`] is returned.
    /// Otherwise, an RAII guard is returned. The lock will be unlocked when the
    /// guard is dropped.
    ///
    /// This function does not block.
    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.raw.try_lock() {
            Some(MutexGuard { lock: self })
        } else {
            None
        }
    }

    /// Immediately drops the guard, and consequently unlocks the mutex.
    ///
    /// This function is equivalent to calling [`drop`] on the guard but is more self-documenting.
    /// Alternately, the guard will be automatically dropped when it goes out of scope.
    #[inline]
    pub fn unlock(guard: MutexGuard<'_, T>) {
        drop(guard)
    }

    /// Immediately drops the guard, and consequently unlocks the mutex.
    ///
    /// This function is equivalent to calling [`unlock_fair`](MutexGuard::unlock_fair)
    /// on the guard. Alternately, the guard will be automatically dropped when it goes out of
    /// scope, but will use the standard unlocking mechanism.
    #[inline]
    pub fn unlock_fair(guard: MutexGuard<'_, T>) {
        guard.unlock_fair()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `Mutex` mutably, no actual locking needs to
    /// take place -- the mutable borrow statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

unsafe impl<T: Send + ?Sized> Send for Mutex<T> {}
unsafe impl<T: Send + ?Sized> Sync for Mutex<T> {}

impl<T: Debug + ?Sized> Debug for Mutex<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let mut d = f.debug_struct("Mutex");

        match self.try_lock() {
            Some(guard) => {
                d.field("data", &&*guard);
            }
            None => {
                struct LockedPlaceholder;
                impl Debug for LockedPlaceholder {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("<locked>")
                    }
                }
                d.field("data", &LockedPlaceholder);
            }
        }

        d.finish()
    }
}

impl<T: Default + ?Sized> Default for Mutex<T> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for Mutex<T> {
    #[inline]
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

/// An RAII implementation of a "scoped lock" of a mutex. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// [`Deref`] and [`DerefMut`] implementations.
///
/// This structure is created by the [`lock`] and [`try_lock`] methods on
/// [`Mutex`].
///
/// [`lock`]: Mutex::lock
/// [`try_lock`]: Mutex::try_lock
#[must_use = "if unused the Mutex will immediately unlock"]
pub struct MutexGuard<'a, T: ?Sized> {
    lock: &'a Mutex<T>,
}

impl<T: ?Sized> MutexGuard<'_, T> {
    /// Unlocks the mutex using a fair unlock protocol.
    #[inline]
    pub fn unlock_fair(self) {
        // SAFETY: We know that we own a lock.
        let m = ManuallyDrop::new(self);
        unsafe { m.lock.raw.unlock_fair() }
    }
}

impl<T: ?Sized> !Send for MutexGuard<'_, T> {}
unsafe impl<T: Sync + ?Sized> Sync for MutexGuard<'_, T> {}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: We are the sole owners of the data until the lock
        // is dropped.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We are the sole owners of the data until the lock
        // is dropped.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: Debug + ?Sized> Debug for MutexGuard<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display + ?Sized> Display for MutexGuard<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: We know that we own a lock.
        unsafe { self.lock.raw.unlock() }
    }
}

// Based on the implementation in the parking_lot crate.
// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
struct RawMutex {
    state: AtomicU8,
    fair_timeout: Cell<Instant>,
    rng: UnsafeCell<rand::rngs::SmallRng>,
    task: JoinHandle<Pin<ObjBox<Task<'static, ()>>>>,
}

impl RawMutex {
    const STATE_INIT: u8 = 0;

    const LOCKED_BIT: u8 = 0b01;
    const WAITERS_BIT: u8 = 0b10;

    const HANDOFF_LOCK: *const () = 1 as *const ();

    #[inline]
    fn new() -> Self {
        let runtime = current_runtime().unwrap();

        let task = Builder::new()
            .blocked()
            .spawn_complex::<_, fn(), _, fn()>(runtime, None, None, &[])
            .expect("could not create mutex task");

        let rng = rand::rngs::SmallRng::from_entropy();

        Self {
            task,
            rng: UnsafeCell::new(rng),
            state: AtomicU8::new(Self::STATE_INIT),
            fair_timeout: Cell::new(Instant::now()),
        }
    }

    #[inline]
    fn lock(&self) {
        // Try to acquire the lock by performing a CAS operation on the state.
        // The `Acquire` synchronizes with the `Release` in unlock and unlock_fair.
        if self
            .state
            .compare_exchange_weak(
                Self::STATE_INIT,
                Self::LOCKED_BIT,
                atomic::Ordering::Acquire,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            self.lock_slow()
        }
    }

    #[inline]
    fn lock_slow(&self) {
        let runtime = current_runtime().unwrap();
        let mut spinwait = SpinWait::new();
        let mut state = self.state.load(atomic::Ordering::Relaxed);
        loop {
            // Grab the lock if it isn't locked, even if there is a queue on it
            if state & Self::LOCKED_BIT == 0 {
                match self.state.compare_exchange_weak(
                    state,
                    state | Self::LOCKED_BIT,
                    atomic::Ordering::Acquire,
                    atomic::Ordering::Relaxed,
                ) {
                    Ok(_) => return,
                    Err(x) => state = x,
                }
                continue;
            }

            // If there is no queue, try spinning a few times
            if state & Self::WAITERS_BIT == 0 && spinwait.spin(|| runtime.yield_now()) {
                state = self.state.load(atomic::Ordering::Relaxed);
                continue;
            }

            // Set the parked bit
            if state & Self::WAITERS_BIT == 0 {
                if let Err(x) = self.state.compare_exchange_weak(
                    state,
                    state | Self::WAITERS_BIT,
                    atomic::Ordering::Relaxed,
                    atomic::Ordering::Relaxed,
                ) {
                    state = x;
                    continue;
                }
            }

            // Block our tasks until we are woken up by an unlock
            let mut data = MaybeUninit::uninit();
            runtime.yield_and_enter(|s, curr| {
                let state = self.state.load(atomic::Ordering::Relaxed);
                if state == Self::LOCKED_BIT | Self::WAITERS_BIT {
                    unsafe {
                        s.wait_task_on(curr, self.task.as_raw(), Some(&mut data))
                            .expect("can not wait on mutex");
                    }
                }
            });

            // SAFETY: We were woken up by the runtime, so the data must contain some valid message.
            match unsafe { data.assume_init() } {
                // Normal wakeup, retry.
                WakeupData::None => (),

                // Handoff, we now own the lock.
                WakeupData::Custom(Self::HANDOFF_LOCK) => return,

                // Internal error.
                WakeupData::Custom(_) => unreachable!(),
            }

            // Loop back and try locking again
            spinwait.reset();
            state = self.state.load(atomic::Ordering::Relaxed);
        }
    }

    #[inline]
    fn try_lock(&self) -> bool {
        // Relaxed is sufficient, as the CAS-loop ensures that we eventually land
        // with the correct state.
        let mut state = self.state.load(atomic::Ordering::Relaxed);
        loop {
            // If the locked bit is set we know that the mutex is locked and return false.
            if state & Self::LOCKED_BIT != 0 {
                return false;
            }

            // The `Acquire` synchronizes with the `Release` in unlock and unlock_fair.
            match self.state.compare_exchange_weak(
                state,
                state | Self::LOCKED_BIT,
                atomic::Ordering::Acquire,
                atomic::Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(x) => state = x,
            }
        }
    }

    #[inline]
    unsafe fn unlock(&self) {
        if self
            .state
            .compare_exchange_weak(
                Self::LOCKED_BIT,
                Self::STATE_INIT,
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            self.unlock_slow(false)
        }
    }

    #[inline]
    unsafe fn unlock_fair(&self) {
        if self
            .state
            .compare_exchange_weak(
                Self::LOCKED_BIT,
                Self::STATE_INIT,
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            self.unlock_slow(true)
        }
    }

    #[inline]
    fn use_fair_unlock(&self) -> bool {
        use rand::Rng;

        let now = Instant::now();
        if now > self.fair_timeout.get() {
            let rng = self.rng.get();

            // Time between 0 and 1ms.
            // SAFETY: Is only used while the scheduler is locked.
            let nanos = unsafe { (*rng).gen_range(0..1_000_000) };
            self.fair_timeout.set(now + Duration::new(0, nanos));

            true
        } else {
            false
        }
    }

    #[inline]
    unsafe fn unlock_slow(&self, fair: bool) {
        let runtime = current_runtime().unwrap();

        // The slow path uses the runtime the synchronize and wake sleeping
        // tasks.
        runtime.enter_scheduler(move |s, _| {
            let num_waiters = s.num_waiting_tasks(self.task.as_raw());
            let one_waiter = num_waiters == 1;
            let has_waiters = num_waiters != 0;

            // If we are using a fair unlock then we simply hand the ownership of the lock
            // to the newly woken up task.
            if has_waiters && (fair || self.use_fair_unlock()) {
                let data = WakeupData::Custom(Self::HANDOFF_LOCK);
                s.notify_one(self.task.as_raw(), data)
                    .expect("could not wake task");

                // Clear the waiters bit if there are no more waiting tasks.
                if one_waiter {
                    self.state
                        .store(Self::LOCKED_BIT, atomic::Ordering::Relaxed)
                }
                return;
            }

            let num_waiters = s
                .notify_one(self.task.as_raw(), WakeupData::None)
                .expect("could not wake task")
                .unwrap_or(0);

            if num_waiters != 0 {
                // If there are still waiters we only remove the locked bit.
                self.state
                    .store(Self::WAITERS_BIT, atomic::Ordering::Release)
            } else {
                // If there are no more waiters we set the state to the initial value.
                self.state
                    .store(Self::STATE_INIT, atomic::Ordering::Release)
            }
        });
    }
}

unsafe impl Send for RawMutex {}
unsafe impl Sync for RawMutex {}

impl Drop for RawMutex {
    fn drop(&mut self) {
        self.task.unblock().expect("could not unblock mutex task")
    }
}
