//! Atomic and non atomic reference counts.

use core::cell::UnsafeCell;

use crate::{bindings, error::to_result, ffi::FFITransferable};

#[repr(transparent)]
#[derive(Debug)]
pub struct RefCount(UnsafeCell<bindings::FimoRefCount>);

impl RefCount {
    /// Constructs a new `RefCount` with one strong reference.
    pub fn new() -> Self {
        Self(UnsafeCell::new(bindings::FimoRefCount {
            strong_refs: 1,
            weak_refs: 1,
        }))
    }

    /// Returns the number of strong references for this instance.
    pub fn strong_count(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_strong_count(self.0.get()) }
    }

    /// Returns the number of weak references for this instance.
    ///
    /// The function ensures that there is at least one strong reference,
    /// otherwise it returns `0`.
    pub fn weak_count_guarded(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_weak_count_guarded(self.0.get()) }
    }

    /// Returns the number of weak references for this instance.
    pub fn weak_count_unguarded(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_weak_count_unguarded(self.0.get()) }
    }

    /// Increases the strong reference count by one.
    ///
    /// This function may abort the program, if the strong value is saturated.
    ///
    /// # Safety
    ///
    /// The caller must own a strong reference to the object guarded by
    /// the reference count.
    pub unsafe fn increase_strong_count(&self) {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_increase_strong_count(self.0.get()) }
    }

    /// Decreases the strong reference count by one.
    ///
    /// Returns whether the strong reference count has reached zero.
    ///
    /// # Safety
    ///
    /// May only be called, if one owns a strong reference to the object
    /// guarded by the reference count.
    pub unsafe fn decrease_strong_count(&self) -> bool {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_decrease_strong_count(self.0.get()) }
    }

    /// Decreases the weak reference count by one.
    ///
    /// Returns whether the weak reference count has reached zero.
    ///
    /// # Safety
    ///
    /// May only be called, if one owns a weak reference to the object
    /// guarded by the reference count.
    pub unsafe fn decrease_weak_count(&self) -> bool {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_decrease_weak_count(self.0.get()) }
    }

    /// Upgrades a weak reference to a strong reference.
    ///
    /// The weak count is not modified.
    ///
    /// # Safety
    ///
    /// The caller must own a weak reference to the object guarded by
    /// the reference count.
    pub unsafe fn upgrade(&self) -> crate::error::Result {
        // Safety: The pointer is valid and we own a weak reference.
        let error = unsafe { bindings::fimo_upgrade_refcount(self.0.get()) };
        to_result(error)
    }

    /// Downgrades a strong reference to a weak reference.
    ///
    /// The strong count is not modified.
    ///
    /// # Safety
    ///
    /// The caller must own a strong reference to the object guarded by
    /// the reference count.
    pub unsafe fn downgrade(&self) -> crate::error::Result {
        // Safety: The pointer is valid and we own a weak reference.
        let error = unsafe { bindings::fimo_downgrade_refcount(self.0.get()) };
        to_result(error)
    }

    /// Returns whether there is only one strong reference left.
    pub fn is_unique(&self) -> bool {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_refcount_is_unique(self.0.get()) }
    }
}

impl Default for RefCount {
    fn default() -> Self {
        Self::new()
    }
}

impl FFITransferable<*mut bindings::FimoRefCount> for &'_ RefCount {
    fn into_ffi(self) -> *mut bindings::FimoRefCount {
        self.0.get()
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoRefCount) -> Self {
        // Safety: The two types have an identical layout.
        unsafe { &*(ffi as *const RefCount) }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct ARefCount(UnsafeCell<bindings::FimoAtomicRefCount>);

impl ARefCount {
    /// Constructs a new `RefCount` with one strong reference.
    pub fn new() -> Self {
        Self(UnsafeCell::new(bindings::FimoAtomicRefCount {
            strong_refs: 1,
            weak_refs: 1,
        }))
    }

    /// Returns the number of strong references for this instance.
    pub fn strong_count(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_strong_count_atomic(self.0.get()) }
    }

    /// Returns the number of weak references for this instance.
    pub fn weak_count_unguarded(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_weak_count_atomic_unguarded(self.0.get()) }
    }

    /// Returns the number of weak references for this instance.
    ///
    /// The function ensures that there is at least one strong reference,
    /// otherwise it returns `0`.
    pub fn weak_count_guarded(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_weak_count_atomic_guarded(self.0.get()) }
    }

    /// Increases the strong reference count by one.
    ///
    /// This function may abort the program, if the strong value is saturated.
    ///
    /// # Safety
    ///
    /// The caller must own a strong reference to the object guarded by
    /// the reference count.
    pub unsafe fn increase_strong_count(&self) {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_increase_strong_count_atomic(self.0.get()) }
    }

    /// Decreases the strong reference count by one.
    ///
    /// Returns whether the strong reference count has reached zero.
    ///
    /// # Safety
    ///
    /// May only be called, if one owns a strong reference to the object
    /// guarded by the reference count.
    pub unsafe fn decrease_strong_count(&self) -> bool {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_decrease_strong_count_atomic(self.0.get()) }
    }

    /// Decreases the weak reference count by one.
    ///
    /// Returns whether the weak reference count has reached zero.
    ///
    /// # Safety
    ///
    /// May only be called, if one owns a weak reference to the object
    /// guarded by the reference count.
    pub unsafe fn decrease_weak_count(&self) -> bool {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_decrease_weak_count_atomic(self.0.get()) }
    }

    /// Upgrades a weak reference to a strong reference.
    ///
    /// The weak count is not modified.
    ///
    /// # Safety
    ///
    /// The caller must own a weak reference to the object guarded by
    /// the reference count.
    pub unsafe fn upgrade(&self) -> crate::error::Result {
        // Safety: The pointer is valid and we own a weak reference.
        let error = unsafe { bindings::fimo_upgrade_refcount_atomic(self.0.get()) };
        to_result(error)
    }

    /// Downgrades a strong reference to a weak reference.
    ///
    /// The strong count is not modified.
    ///
    /// # Safety
    ///
    /// The caller must own a strong reference to the object guarded by
    /// the reference count.
    pub unsafe fn downgrade(&self) -> crate::error::Result {
        // Safety: The pointer is valid and we own a weak reference.
        let error = unsafe { bindings::fimo_downgrade_refcount_atomic(self.0.get()) };
        to_result(error)
    }

    /// Returns whether there is only one strong reference left.
    pub fn is_unique(&self) -> bool {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_refcount_atomic_is_unique(self.0.get()) }
    }
}

impl Default for ARefCount {
    fn default() -> Self {
        Self::new()
    }
}

impl FFITransferable<*mut bindings::FimoAtomicRefCount> for &'_ ARefCount {
    fn into_ffi(self) -> *mut bindings::FimoAtomicRefCount {
        self.0.get()
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoAtomicRefCount) -> Self {
        // Safety: The two types have an identical layout.
        unsafe { &*(ffi as *const ARefCount) }
    }
}
