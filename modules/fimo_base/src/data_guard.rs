use std::panic::RefUnwindSafe;

/// An unlocked resource.
#[derive(Debug, Default)]
pub struct Unlocked {}

/// A locked resource.
#[derive(Debug, Default)]
pub struct Locked {}

/// A guarded resource.
#[derive(Debug)]
pub struct DataGuard<'a, T, L = Unlocked> {
    pub(crate) data: &'a mut T,
    lock_type: L,
}

impl<'a, T, L> DataGuard<'a, T, L> {
    /// Creates a new guard.
    pub fn new(data: &'a mut T) -> DataGuard<'a, T, Unlocked> {
        DataGuard {
            data,
            lock_type: Unlocked {},
        }
    }
}

impl<'a, T> DataGuard<'a, T, Unlocked> {
    /// Assumes that the contained data is locked.
    ///
    /// # Safety
    ///
    /// The assumption is not checked.
    pub unsafe fn assume_locked(self) -> DataGuard<'a, T, Locked> {
        DataGuard {
            data: self.data,
            lock_type: Locked {},
        }
    }
}

impl<'a, T> DataGuard<'a, T, Locked> {
    /// Assumes that the contained data is unlocked.
    ///
    /// # Safety
    ///
    /// The assumption is not checked.
    pub unsafe fn assume_unlocked(self) -> DataGuard<'a, T, Unlocked> {
        DataGuard {
            data: self.data,
            lock_type: Unlocked {},
        }
    }
}

impl<'a, T, L> RefUnwindSafe for DataGuard<'a, T, L> {}
