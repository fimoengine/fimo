use crate::rust::sync::condvar::{CondvarInner, WaitAndLockData};
use crate::rust::TaskHandle;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};

/// A mutex.
pub struct Mutex<T> {
    data: parking_lot::Mutex<T>,
    condvar: CondvarInner,
}

/// A mutex guard.
pub struct MutexGuard<'a, T> {
    pub(crate) lock: &'a Mutex<T>,
}

impl<T> Mutex<T> {
    /// Constructs a new `Mutex<T>`.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn new(val: T) -> Self {
        Self {
            data: parking_lot::Mutex::new(val),
            condvar: CondvarInner::new(),
        }
    }

    /// Locks the Mutex, blocking the task until it can be acquired.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        if let Some(guard) = self.try_lock() {
            guard
        } else {
            loop {
                let mut data = WaitAndLockData {
                    locked: false,
                    mutex: self,
                };

                // try waiting if the runtime can't acquire the lock.
                self.condvar.wait_and_try_lock(&mut data);

                // in case the lock could be acquired, we can exit the function.
                if data.locked {
                    return MutexGuard { lock: self };
                }
            }
        }
    }

    /// Tries to lock the Mutex without blocking the task.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.data.try_lock().map(|guard| {
            std::mem::forget(guard);
            MutexGuard { lock: self }
        })
    }

    /// Force the unlock of the mutex.
    ///
    /// Can be used to unlock the mutex, in case the guard was
    /// forgotten.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// This function must only be called if this task logically
    /// owns a lock, which was discarded using mem::forget.
    pub unsafe fn force_unlock(&self) {
        self.data.force_unlock();
        self.condvar.notify_one();
    }

    /// Returns a raw pointer to the underlying data.
    ///
    /// This is useful when combined with mem::forget to hold a
    /// lock without the need to maintain a MutexGuard object alive,
    /// for example when dealing with FFI.
    ///
    /// # Panics
    ///
    /// **Must** be run from within a task.
    ///
    /// # Safety
    ///
    /// You must ensure that no data races occur when dereferencing
    /// the pointer.  
    pub unsafe fn data_ptr(&self) -> *mut T {
        self.data.data_ptr()
    }

    pub(crate) unsafe fn force_unlock_with_notify(&self, notify_fn: &mut dyn FnMut(TaskHandle)) {
        self.data.force_unlock();
        self.condvar.notify_one_with_function(notify_fn);
    }
}

impl<T: Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.try_lock() {
            Some(guard) => f.debug_struct("Mutex").field("data", &guard).finish(),
            None => {
                struct Placeholder;
                impl Debug for Placeholder {
                    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                        write!(f, "<locked>")
                    }
                }

                f.debug_struct("Mutex").field("data", &Placeholder).finish()
            }
        }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // unlock and notify waiters.
        unsafe { self.lock.force_unlock() };
    }
}

impl<T> !Send for MutexGuard<'_, T> {}

unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // we own the lock.
        unsafe { &*self.lock.data.data_ptr() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // we own the lock.
        unsafe { &mut *self.lock.data.data_ptr() }
    }
}

impl<T: Debug> Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display> Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}
