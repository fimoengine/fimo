use std::{
    cell::{Cell, UnsafeCell},
    fmt::{Debug, Display},
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    sync::atomic::AtomicUsize,
    time::{Duration, Instant},
};

use fimo_ffi::FfiFn;
use rand::SeedableRng;

use crate::runtime::{
    current_runtime, IRuntimeExt, IScheduler, NotifyFilterOp, NotifyResult, WaitToken, WakeupToken,
};

use super::spin_wait::SpinWait;

/// A reader-writer lock
///
/// This type of lock allows a number of readers or at most one writer at any
/// point in time. The write portion of this lock typically allows modification
/// of the underlying data (exclusive access) and the read portion of this lock
/// typically allows for read-only access (shared access).
///
/// In comparison, a [`Mutex`] does not distinguish between readers or writers
/// that acquire the lock, therefore blocking any tasks waiting for the lock to
/// become available. An `RwLock` will allow any number of readers to acquire the
/// lock as long as a writer is not holding the lock.
///
/// This lock uses a task-fair locking policy which avoids both reader and
/// writer starvation. This means that readers trying to acquire the lock will
/// block even if the lock is unlocked when there are writers waiting to acquire
/// the lock. Because of this, attempts to recursively acquire a read lock
/// within a single task may result in a deadlock.
///
/// The type parameter `T` represents the data that this lock protects. It is
/// required that `T` satisfies [`Send`] to be shared across tasks and
/// [`Sync`] to allow concurrent access through readers. The RAII guards
/// returned from the locking methods implement [`Deref`] (and [`DerefMut`]
/// for the `write` methods) to allow access to the content of the lock.
///
/// [`Mutex`]: super::Mutex
pub struct RwLock<T: ?Sized> {
    raw: RawRwLock,
    data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    /// Creates a new instance of an `RwLock<T>` which is unlocked.
    #[inline]
    pub fn new(t: T) -> RwLock<T> {
        Self {
            raw: RawRwLock::new(),
            data: UnsafeCell::new(t),
        }
    }

    /// Consumes this `RwLock`, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Locks this rwlock with shared read access, blocking the current task
    /// until it can be acquired.
    ///
    /// The calling task will be blocked until there are no more writers which
    /// hold the lock. There may be other readers currently inside the lock when
    /// this method returns.
    ///
    /// Returns an RAII guard which will release this task's shared access
    /// once it is dropped.
    ///
    /// # Panics
    ///
    /// This function might panic when called if the lock is already held by the current task.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.raw.lock_read();
        RwLockReadGuard { lock: self }
    }

    /// Attempts to acquire this rwlock with shared read access.
    ///
    /// If the access could not be granted at this time, then `None` is returned.
    /// Otherwise, an RAII guard is returned which will release the shared access
    /// when it is dropped.
    ///
    /// This function does not block.
    #[inline]
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        if self.raw.try_lock_read() {
            Some(RwLockReadGuard { lock: self })
        } else {
            None
        }
    }

    /// Locks this rwlock with exclusive write access, blocking the current
    /// task until it can be acquired.
    ///
    /// This function will not return while other writers or other readers
    /// currently have access to the lock.
    ///
    /// Returns an RAII guard which will drop the write access of this rwlock
    /// when dropped.
    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.raw.lock_write();
        RwLockWriteGuard { lock: self }
    }

    /// Attempts to lock this rwlock with exclusive write access.
    ///
    /// If the lock could not be acquired at this time, then `None` is returned.
    /// Otherwise, an RAII guard is returned which will release the lock when
    /// it is dropped.
    ///
    /// This function does not block.
    #[inline]
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        if self.raw.try_lock_write() {
            Some(RwLockWriteGuard { lock: self })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `RwLock` mutably, no actual locking needs to
    /// take place -- the mutable borrow statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// Checks whether this `RwLock` is currently locked in any way.
    #[inline]
    pub fn is_locked(&self) -> bool {
        self.raw.is_locked()
    }

    /// Check if this `RwLock` is currently exclusively locked.
    #[inline]
    pub fn is_locked_exclusive(&self) -> bool {
        self.raw.is_locked_exclusive()
    }
}

unsafe impl<T: Send + ?Sized> Send for RwLock<T> {}
unsafe impl<T: Send + Sync + ?Sized> Sync for RwLock<T> {}

impl<T: Default> Default for RwLock<T> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: Debug + ?Sized> Debug for RwLock<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("RwLock");
        match self.try_read() {
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

impl<T> From<T> for RwLock<T> {
    #[inline]
    fn from(t: T) -> Self {
        Self::new(t)
    }
}

/// RAII structure used to release the shared read access of a lock when
/// dropped.
///
/// This structure is created by the [`read`] and [`try_read`] methods on
/// [`RwLock`].
///
/// [`read`]: RwLock::read
/// [`try_read`]: RwLock::try_read
#[must_use = "if unused the RwLock will immediately unlock"]
pub struct RwLockReadGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> RwLockReadGuard<'_, T> {
    /// Unlocks the `RwLock`.
    ///
    /// Convenience function for `drop(self)`.
    #[inline]
    pub fn unlock(self) {
        drop(self)
    }

    /// Unlocks the `RwLock` using a fair unlock protocol.
    #[inline]
    pub fn unlock_fair(self) {
        // SAFETY: We know that we own a lock.
        let this = ManuallyDrop::new(self);
        unsafe { this.lock.raw.unlock_read_fair() }
    }
}

impl<T: ?Sized> !Send for RwLockReadGuard<'_, T> {}
unsafe impl<T: Sync + ?Sized> Sync for RwLockReadGuard<'_, T> {}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: Debug> Debug for RwLockReadGuard<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display + ?Sized> Display for RwLockReadGuard<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: We own a read lock.
        unsafe { self.lock.raw.unlock_read() }
    }
}

/// RAII structure used to release the exclusive write access of a lock when
/// dropped.
///
/// This structure is created by the [`write`] and [`try_write`] methods
/// on [`RwLock`].
///
/// [`write`]: RwLock::write
/// [`try_write`]: RwLock::try_write
#[must_use = "if unused the RwLock will immediately unlock"]
pub struct RwLockWriteGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> RwLockWriteGuard<'_, T> {
    /// Unlocks the `RwLock`.
    ///
    /// Convenience function for `drop(self)`.
    #[inline]
    pub fn unlock(self) {
        drop(self)
    }

    /// Unlocks the `RwLock` using a fair unlock protocol.
    #[inline]
    pub fn unlock_fair(self) {
        // SAFETY: We know that we own a lock.
        let this = ManuallyDrop::new(self);
        unsafe { this.lock.raw.unlock_write_fair() }
    }
}

impl<T: ?Sized> !Send for RwLockWriteGuard<'_, T> {}
unsafe impl<T: Sync + ?Sized> Sync for RwLockWriteGuard<'_, T> {}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: Debug> Debug for RwLockWriteGuard<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display + ?Sized> Display for RwLockWriteGuard<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: We own the write lock.
        unsafe { self.lock.raw.unlock_write() }
    }
}

// Based on the implementation in the parking_lot crate.
// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
struct RawRwLock {
    state: AtomicUsize,
    fair_timeout: Cell<Instant>,
    rng: UnsafeCell<rand::rngs::SmallRng>,
}

impl RawRwLock {
    const STATE_INIT: usize = 0;

    const UNINIT_TOKEN: *const () = std::ptr::null();
    const HANDOFF_LOCK: *const () = std::ptr::invalid(1);

    // There is at least one task in the main queue.
    const WAITING_BIT: usize = 0b0001;
    // There is a waiting task holding WRITER_BIT. WRITER_BIT must be set.
    const WRITER_WAITING_BIT: usize = 0b0010;
    // If the reader count is zero: a writer is currently holding an exclusive lock.
    // Otherwise: a writer is waiting for the remaining readers to exit the lock.
    const WRITER_BIT: usize = 0b1000;
    // Mask of bits used to count readers.
    const READERS_MASK: usize = !0b1111;
    // Base unit for counting readers.
    const ONE_READER: usize = 0b10000;

    // Token indicating what type of lock a queued task is trying to acquire
    const TOKEN_SHARED: WaitToken = WaitToken(std::ptr::invalid(Self::ONE_READER));
    const TOKEN_EXCLUSIVE: WaitToken = WaitToken(std::ptr::invalid(Self::WRITER_BIT));

    #[inline]
    fn new() -> Self {
        let rng = rand::rngs::SmallRng::from_entropy();

        Self {
            rng: UnsafeCell::new(rng),
            state: AtomicUsize::new(Self::STATE_INIT),
            fair_timeout: Cell::new(Instant::now()),
        }
    }

    #[inline]
    fn is_locked(&self) -> bool {
        let state = self.state.load(atomic::Ordering::Relaxed);
        state & (Self::WRITER_BIT | Self::READERS_MASK) != 0
    }

    #[inline]
    fn is_locked_exclusive(&self) -> bool {
        let state = self.state.load(atomic::Ordering::Relaxed);
        state & (Self::WRITER_BIT) != 0
    }

    #[inline]
    fn lock_write(&self) {
        if self
            .state
            .compare_exchange_weak(
                Self::STATE_INIT,
                Self::WRITER_BIT,
                atomic::Ordering::Acquire,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            self.lock_write_slow()
        }
    }

    #[inline]
    fn try_lock_write(&self) -> bool {
        self.state
            .compare_exchange(
                Self::STATE_INIT,
                Self::WRITER_BIT,
                atomic::Ordering::Acquire,
                atomic::Ordering::Relaxed,
            )
            .is_ok()
    }

    #[inline]
    unsafe fn unlock_write(&self) {
        if self
            .state
            .compare_exchange(
                Self::WRITER_BIT,
                Self::STATE_INIT,
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            return;
        }
        self.unlock_write_slow(false);
    }

    #[inline]
    unsafe fn unlock_write_fair(&self) {
        if self
            .state
            .compare_exchange(
                Self::WRITER_BIT,
                Self::STATE_INIT,
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            return;
        }
        self.unlock_write_slow(true);
    }

    #[inline]
    fn lock_read(&self) {
        if !self.try_lock_read_fast() {
            self.lock_read_slow();
        }
    }

    #[inline]
    fn try_lock_read(&self) -> bool {
        if self.try_lock_read_fast() {
            true
        } else {
            self.try_lock_read_slow()
        }
    }

    #[inline]
    unsafe fn unlock_read(&self) {
        let state = self
            .state
            .fetch_sub(Self::ONE_READER, atomic::Ordering::Release);

        if state & (Self::READERS_MASK | Self::WRITER_WAITING_BIT)
            == (Self::ONE_READER | Self::WRITER_WAITING_BIT)
        {
            self.unlock_read_slow();
        }
    }

    #[inline]
    unsafe fn unlock_read_fair(&self) {
        // Shared unlocking is always fair in this implementation.
        self.unlock_read();
    }
}

impl RawRwLock {
    #[cold]
    fn lock_write_slow(&self) {
        let try_lock = |state: &mut usize| {
            loop {
                if *state & Self::WRITER_BIT != 0 {
                    return false;
                }

                // Grab WRITER_BIT if it isn't set, even if there are waiting tasks.
                match self.state.compare_exchange_weak(
                    *state,
                    *state | Self::WRITER_BIT,
                    atomic::Ordering::Acquire,
                    atomic::Ordering::Relaxed,
                ) {
                    Ok(_) => return true,
                    Err(x) => *state = x,
                }
            }
        };

        // Step 1: grab exclusive ownership of WRITER_BIT
        self.lock_common(Self::TOKEN_EXCLUSIVE, try_lock, Self::WRITER_BIT);

        // Step 2: wait for all remaining readers to exit the lock.
        self.wait_for_readers()
    }

    #[cold]
    fn unlock_write_slow(&self, force_fair: bool) {
        // There are tasks to unpark. Try to unpark as many as we can.
        let callback = |mut new_state, result: NotifyResult| {
            // If we are using a fair unlock then we should keep the
            // rwlock locked and hand it off to the notified tasks.
            if result.has_notified_tasks() && (force_fair || self.use_fair_unlock()) {
                if result.has_tasks_remaining() {
                    new_state |= Self::WAITING_BIT;
                }
                self.state.store(new_state, atomic::Ordering::Release);
                WakeupToken::Custom(Self::HANDOFF_LOCK)
            } else {
                // Clear the waiting bit if there are no more waiting tasks.
                if result.has_tasks_remaining() {
                    self.state
                        .store(Self::WAITING_BIT, atomic::Ordering::Release);
                } else {
                    self.state.store(0, atomic::Ordering::Release);
                }
                WakeupToken::None
            }
        };

        // SAFETY: `callback` does not panic or call into the scheduler.
        unsafe { self.wake_parked_tasks(0, callback) };
    }

    #[cold]
    fn lock_read_slow(&self) {
        let try_lock = |state: &mut usize| {
            let mut spinwait_shared = SpinWait::new();
            loop {
                // This is the same condition as try_lock_shared_fast
                if *state & Self::WRITER_BIT != 0 {
                    return false;
                }

                if self
                    .state
                    .compare_exchange_weak(
                        *state,
                        state
                            .checked_add(Self::ONE_READER)
                            .expect("RwLock reader count overflow"),
                        atomic::Ordering::Acquire,
                        atomic::Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return true;
                }

                // If there is high contention on the reader count then we want
                // to leave some time between attempts to acquire the lock to
                // let other tasks make progress.
                spinwait_shared.spin(|| {});
                *state = self.state.load(atomic::Ordering::Relaxed);
            }
        };
        self.lock_common(Self::TOKEN_SHARED, try_lock, Self::WRITER_BIT)
    }

    #[cold]
    fn unlock_read_slow(&self) {
        // At this point WRITER_WAITING_BIT is set and READER_MASK is empty. We
        // just need to wake up a potentially sleeping pending writer.
        // Using the 2nd key at addr + 1
        let runtime = current_runtime().unwrap();
        runtime.enter_scheduler(|s, _| unsafe {
            let queue_addr = (self as *const RawRwLock).addr() + 1;
            let queue_addr = (self as *const _ as *const ()).with_addr(queue_addr);
            let queue = s
                .register_or_fetch_pseudo_task(queue_addr)
                .expect("could not fetch rwlock queue task");

            let callback = |_| {
                // Clear the WRITER_WAITING_BIT here since there can only be one
                // waiting writer task.
                self.state
                    .fetch_and(!Self::WRITER_WAITING_BIT, atomic::Ordering::Relaxed);
                WakeupToken::None
            };
            let mut callback = MaybeUninit::new(callback);
            let callback = FfiFn::new_value(&mut callback);

            let res = s
                .pseudo_notify_one(queue, callback)
                .expect("could not wake task");

            if !res.has_tasks_remaining() {
                // If there are no more waiters we unregister the task.
                s.unregister_pseudo_task(queue)
                    .expect("could not unregister rwlock queue task")
            }
        });
    }

    #[inline(always)]
    fn try_lock_read_fast(&self) -> bool {
        let state = self.state.load(atomic::Ordering::Relaxed);

        // We can't allow grabbing a shared lock if there is a writer, even if
        // the writer is still waiting for the remaining readers to exit.
        if state & Self::WRITER_BIT != 0 {
            return false;
        }

        if let Some(new_state) = state.checked_add(Self::ONE_READER) {
            self.state
                .compare_exchange_weak(
                    state,
                    new_state,
                    atomic::Ordering::Acquire,
                    atomic::Ordering::Relaxed,
                )
                .is_ok()
        } else {
            false
        }
    }

    #[cold]
    fn try_lock_read_slow(&self) -> bool {
        let mut state = self.state.load(atomic::Ordering::Relaxed);
        loop {
            // This mirrors the condition in try_lock_shared_fast
            if state & Self::WRITER_BIT != 0 {
                return false;
            }
            match self.state.compare_exchange_weak(
                state,
                state
                    .checked_add(Self::ONE_READER)
                    .expect("RwLock reader count overflow"),
                atomic::Ordering::Acquire,
                atomic::Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(x) => state = x,
            }
        }
    }

    #[inline]
    fn lock_common(
        &self,
        token: WaitToken,
        mut try_lock: impl FnMut(&mut usize) -> bool,
        validate_flags: usize,
    ) {
        let runtime = current_runtime().unwrap();
        let mut spinwait = SpinWait::new();
        let mut state = self.state.load(atomic::Ordering::Relaxed);
        loop {
            // Attempt to grab the lock
            if try_lock(&mut state) {
                return;
            }

            // If there are no waiting tasks, try spinning a few times.
            if state & (Self::WAITING_BIT | Self::WRITER_WAITING_BIT) == 0
                && spinwait.spin(|| runtime.yield_now())
            {
                state = self.state.load(atomic::Ordering::Relaxed);
                continue;
            }

            // Set the waiting bit
            if state & Self::WAITING_BIT == 0 {
                if let Err(x) = self.state.compare_exchange_weak(
                    state,
                    state | Self::WAITING_BIT,
                    atomic::Ordering::Relaxed,
                    atomic::Ordering::Relaxed,
                ) {
                    state = x;
                    continue;
                }
            }

            // Park our task until we are woken up by an unlock
            let mut data = MaybeUninit::new(WakeupToken::Custom(Self::UNINIT_TOKEN));
            let waited = runtime.yield_and_enter(|s, curr| {
                let state = self.state.load(atomic::Ordering::Relaxed);
                if state & Self::WAITING_BIT != 0 && (state & validate_flags != 0) {
                    unsafe {
                        // SAFETY: We controll the address of the rwlock and it won't be moved until there are no more
                        // tasks trying to lock it.
                        let self_addr = self as *const _ as *const ();
                        let task = s
                            .register_or_fetch_pseudo_task(self_addr)
                            .expect("can not create rwlock task");

                        // Try to wait on the task.
                        match s.pseudo_wait_task_on(curr, task, Some(&mut data), token) {
                            Ok(_) => (),
                            Err(_) => {
                                // If we could not wait we must check whether the rwlock has other
                                // waiters and deallocate it's task otherwise.
                                assert!(s
                                    .unregister_pseudo_task_if_empty(task)
                                    .expect("could not unregister rwlock task"));
                                panic!("can not wait on rwlock")
                            }
                        }
                    }

                    true
                } else {
                    false
                }
            });

            if waited {
                // SAFETY: We were woken up by the runtime, so the data must contain some valid message.
                match unsafe { data.assume_init() } {
                    // Normal wakeup, retry.
                    WakeupToken::None => (),

                    // Handoff, we now own the lock.
                    WakeupToken::Custom(d) => {
                        debug_assert_eq!(d, Self::HANDOFF_LOCK);
                        return;
                    }

                    // Wait operation can never be skipped.
                    WakeupToken::Skipped => unreachable!(),

                    // Time out is not currently supported
                    WakeupToken::TimedOut => panic!("RWLock time out not supported"),
                }
            }

            // Loop back and try locking again
            spinwait.reset();
            state = self.state.load(atomic::Ordering::Relaxed);
        }
    }

    // Common code for waiting for readers to exit the lock after acquiring
    // WRITER_BIT.
    #[inline]
    fn wait_for_readers(&self) {
        let runtime = current_runtime().unwrap();

        // At this point WRITER_BIT is already set, we just need to wait for the
        // remaining readers to exit the lock.
        let mut spinwait = SpinWait::new();
        let mut state = self.state.load(atomic::Ordering::Acquire);
        while state & Self::READERS_MASK != 0 {
            // Spin a few times to wait for readers to exit
            if spinwait.spin(|| runtime.yield_now()) {
                state = self.state.load(atomic::Ordering::Acquire);
                continue;
            }

            // Set the waiting bit
            if state & Self::WRITER_WAITING_BIT == 0 {
                if let Err(x) = self.state.compare_exchange_weak(
                    state,
                    state | Self::WRITER_WAITING_BIT,
                    atomic::Ordering::Relaxed,
                    atomic::Ordering::Relaxed,
                ) {
                    state = x;
                    continue;
                }
            }

            // Park our task until we are woken up by an unlock
            // Using the 2nd key at addr + 1
            let mut data = MaybeUninit::new(WakeupToken::Custom(Self::UNINIT_TOKEN));
            let waited = runtime.yield_and_enter(|s, curr| unsafe {
                let state = self.state.load(atomic::Ordering::Relaxed);
                if state & Self::READERS_MASK != 0 && state & Self::WRITER_WAITING_BIT != 0 {
                    let queue_addr = (self as *const RawRwLock).addr() + 1;
                    let queue_addr = (self as *const _ as *const ()).with_addr(queue_addr);
                    let queue = s
                        .register_or_fetch_pseudo_task(queue_addr)
                        .expect("could not fetch rwlock queue task");

                    // Try to wait on the task.
                    match s.pseudo_wait_task_on(curr, queue, Some(&mut data), Self::TOKEN_EXCLUSIVE)
                    {
                        Ok(_) => (),
                        Err(_) => {
                            // If we could not wait we must check whether the queue has other
                            // waiters and deallocate it's task otherwise.
                            assert!(s
                                .unregister_pseudo_task_if_empty(queue)
                                .expect("could not unregister rwlock queue task"));
                            panic!("can not wait on rwlock readers")
                        }
                    }

                    true
                } else {
                    false
                }
            });

            if waited {
                // SAFETY: We were woken up by the runtime, so the data must contain some valid message.
                match unsafe { data.assume_init() } {
                    // We still need to re-check the state if we are notified
                    // since a previous writer timing-out could have allowed
                    // another reader to sneak in before we waited.
                    //
                    // Note: Not currently needed, will be required once we implement
                    // time outs.
                    WakeupToken::None | WakeupToken::Custom(_) => {
                        state = self.state.load(atomic::Ordering::Acquire);
                        continue;
                    }

                    // Wait operation can never be skipped.
                    WakeupToken::Skipped => unreachable!(),

                    // Time out is not currently supported
                    WakeupToken::TimedOut => panic!("RwLock time out not supported"),
                }
            }
        }
    }

    /// Common code for waking up waiting tasks after releasing WRITER_BIT.
    ///
    /// # Safety
    ///
    /// `callback` must uphold the requirements of the `callback` parameter to
    /// `pseudo_notify_filter`. Meaning no panics or calls into any function in
    /// the runtime.
    unsafe fn wake_parked_tasks(
        &self,
        new_state: usize,
        callback: impl FnOnce(usize, NotifyResult) -> WakeupToken,
    ) {
        // We must wake up at least one upgrader or writer if there is one,
        // otherwise they may end up waiting indefinitely since unlock_read
        // does not call wake_parked_tasks.
        let new_state = Cell::new(new_state);
        let filter = |WaitToken(token)| {
            let s = new_state.get();

            // If we are waking up a writer, don't wake anything else.
            if s & Self::WRITER_BIT != 0 {
                return NotifyFilterOp::Stop;
            }

            // Otherwise wake *all* readers and one upgrader/writer.
            let token = token.addr();
            new_state.set(s + token);
            NotifyFilterOp::Notify
        };
        let mut filter = MaybeUninit::new(filter);
        let filter = FfiFn::new_value(&mut filter);

        let runtime = current_runtime().unwrap();
        runtime.enter_scheduler(|s, _| {
            let self_addr = self as *const _ as *const ();
            let task = s
                .register_or_fetch_pseudo_task(self_addr)
                .expect("could not fetch rwlock task");

            let callback = |result| callback(new_state.get(), result);
            let mut callback = MaybeUninit::new(callback);
            let callback = FfiFn::new_value(&mut callback);

            let res = s
                .pseudo_notify_filter(task, filter, callback)
                .expect("could not wake task");

            if !res.has_tasks_remaining() {
                // If there are no more waiters we unregister the task.
                s.unregister_pseudo_task(task)
                    .expect("could not unregister rwlock task")
            }
        });
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
}

unsafe impl Send for RawRwLock {}
unsafe impl Sync for RawRwLock {}
