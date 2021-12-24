use crate::rust::sync::condvar::CondvarInner;
use crate::rust::sync::SpinWait;
use crate::rust::WaitOnFn;
use std::fmt::{Debug, Display, Formatter};
use std::mem::forget;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};

/// A reader-writer lock.
pub struct RwLock<T> {
    data: parking_lot::RwLock<T>,
    condvar: CondvarInner,
}

/// RAII structure used to release the
/// shared read access of a lock when dropped.
pub struct RwLockReadGuard<'a, T> {
    pub(crate) lock: &'a RwLock<T>,
}

/// RAII structure used to release the
/// exclusive write access of a lock when dropped.
pub struct RwLockWriteGuard<'a, T> {
    pub(crate) lock: &'a RwLock<T>,
}

impl<T> RwLock<T> {
    /// Constructs a new `RwLock<T>`.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    #[inline]
    pub fn new(val: T) -> Self {
        Self {
            data: parking_lot::RwLock::new(val),
            condvar: CondvarInner::new(),
        }
    }

    /// Consumes the lock and returns the inner type.
    #[inline]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// Locks the `RwLock` with shared read access, blocking
    /// the task until it can be acquired.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        if let Some(g) = self.try_read() {
            g
        } else {
            self.read_slow()
        }
    }

    /// Tries to lock the `RwLock` with shared read access,
    /// without blocking the task.
    #[inline]
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        self.data.try_read().map(|g| {
            std::mem::forget(g);
            RwLockReadGuard { lock: self }
        })
    }

    /// Locks the `RwLock` with exclusive write access, blocking
    /// the task until it can be acquired.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        if let Some(g) = self.try_write() {
            g
        } else {
            self.write_slow()
        }
    }

    /// Tries to lock the `RwLock` with exclusive write access,
    /// without blocking the task.
    #[inline]
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        self.data.try_write().map(|g| {
            std::mem::forget(g);
            RwLockWriteGuard { lock: self }
        })
    }

    /// Returns a mutable reference to the underlying data.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// Force the unlock of the read lock.
    ///
    /// Can be used to unlock the `RwLock`, in case the guard was
    /// forgotten.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// This function must only be called if this task logically
    /// owns a read lock, which was discarded using mem::forget.
    #[inline]
    pub unsafe fn force_unlock_read(&self) {
        self.data.force_unlock_read();
        self.condvar.notify_one();
    }

    /// Force the unlock of the write lock.
    ///
    /// Can be used to unlock the `RwLock`, in case the guard was
    /// forgotten.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// This function must only be called if this task logically
    /// owns a write lock, which was discarded using mem::forget.
    #[inline]
    pub unsafe fn force_unlock_write(&self) {
        self.data.force_unlock_write();
        self.condvar.notify_one();
    }

    /// Returns a raw pointer to the underlying data.
    #[inline]
    pub fn data_ptr(&self) -> *mut T {
        self.data.data_ptr()
    }

    #[cold]
    fn read_slow(&self) -> RwLockReadGuard<'_, T> {
        let mut spin = SpinWait::new();

        loop {
            // try spinning instead of waiting.
            if spin.spin() {
                if let Some(g) = self.try_read() {
                    return g;
                }

                continue;
            }

            // after a few iterations, try waiting.
            struct WaitData<'a, T> {
                pub locked: bool,
                pub lock: &'a RwLock<T>,
            }

            let try_lock = |data: usize| {
                let data = unsafe { &mut *(data as *mut WaitData<'_, T>) };
                if let Some(g) = data.lock.try_read() {
                    // we could acquire the lock and don't need to wait.
                    forget(g);
                    data.locked = true;
                    false
                } else {
                    // continue with locking.
                    true
                }
            };

            let mut data = WaitData {
                locked: false,
                lock: self,
            };

            // wait if the runtime can't acquire the lock.
            self.condvar.wait_on_if(Some(WaitOnFn {
                data: &mut data as *mut _ as usize,
                validate: try_lock,
                after_sleep: |_, _| {},
            }));

            // in case the lock could be acquired, we can exit the function.
            if data.locked {
                return RwLockReadGuard { lock: self };
            }
            spin.reset();
        }
    }

    #[cold]
    fn write_slow(&self) -> RwLockWriteGuard<'_, T> {
        let mut spin = SpinWait::new();

        loop {
            // try spinning instead of waiting.
            if spin.spin() {
                if let Some(g) = self.try_write() {
                    return g;
                }

                continue;
            }

            // after a few iterations, try waiting.
            struct WaitData<'a, T> {
                pub locked: bool,
                pub lock: &'a RwLock<T>,
            }

            let try_lock = |data: usize| {
                let data = unsafe { &mut *(data as *mut WaitData<'_, T>) };
                if let Some(g) = data.lock.try_write() {
                    // we could acquire the lock and don't need to wait.
                    forget(g);
                    data.locked = true;
                    false
                } else {
                    // continue with locking.
                    true
                }
            };

            let mut data = WaitData {
                locked: false,
                lock: self,
            };

            // wait if the runtime can't acquire the lock.
            self.condvar.wait_on_if(Some(WaitOnFn {
                data: &mut data as *mut _ as usize,
                validate: try_lock,
                after_sleep: |_, _| {},
            }));

            // in case the lock could be acquired, we can exit the function.
            if data.locked {
                return RwLockWriteGuard { lock: self };
            }
            spin.reset();
        }
    }
}

impl<T: Debug> Debug for RwLock<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.try_read() {
            Some(guard) => f.debug_struct("RwLock").field("data", &guard).finish(),
            None => {
                struct Placeholder;
                impl Debug for Placeholder {
                    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                        write!(f, "<locked>")
                    }
                }

                f.debug_struct("RwLock")
                    .field("data", &Placeholder)
                    .finish()
            }
        }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl<T> RefUnwindSafe for RwLock<T> {}

unsafe impl<T: Send> Send for RwLock<T> {}

unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> UnwindSafe for RwLock<T> {}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        // unlock and notify waiters.
        unsafe { self.lock.force_unlock_read() };
    }
}

impl<T> !Send for RwLockReadGuard<'_, T> {}

unsafe impl<T: Sync> Sync for RwLockReadGuard<'_, T> {}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // we own the read lock.
        unsafe { &*self.lock.data.data_ptr() }
    }
}

impl<T: Debug> Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display> Display for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        // unlock and notify waiters.
        unsafe { self.lock.force_unlock_write() };
    }
}

impl<T> !Send for RwLockWriteGuard<'_, T> {}

unsafe impl<T: Sync> Sync for RwLockWriteGuard<'_, T> {}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // we own the write lock.
        unsafe { &*self.lock.data.data_ptr() }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // we own the write lock.
        unsafe { &mut *self.lock.data.data_ptr() }
    }
}

impl<T: Debug> Debug for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display> Display for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}
