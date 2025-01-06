//! FFI helpers.

use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

/// A pointer to a virtual function table.
#[repr(transparent)]
pub struct VTablePtr<T: Send + Sync>(NonNull<T>);

impl<T: Send + Sync> VTablePtr<T> {
    /// Constructs a new pointer from a static reference.
    pub fn new(value: &'static T) -> Self {
        Self(NonNull::from(value))
    }

    /// Constructs a new pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure, that `value` can be dereferenced for the lifetime of the constructed
    /// instance.
    pub unsafe fn new_unchecked(value: *const T) -> Self {
        unsafe { Self(NonNull::new_unchecked(value.cast_mut())) }
    }
}

impl<T: Send + Sync> Deref for VTablePtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

unsafe impl<T: Send + Sync> Send for VTablePtr<T> {}
unsafe impl<T: Send + Sync> Sync for VTablePtr<T> {}

impl<T: Send + Sync> Debug for VTablePtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VTablePtr").field(&self.0).finish()
    }
}

impl<T: Send + Sync> Display for VTablePtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:p}", self.0)
    }
}

impl<T: Send + Sync> Copy for VTablePtr<T> {}

impl<T: Send + Sync> Clone for VTablePtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync> PartialEq for VTablePtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: Send + Sync> Eq for VTablePtr<T> {}

impl<T: Send + Sync> PartialOrd for VTablePtr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Send + Sync> Ord for VTablePtr<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Send + Sync> Hash for VTablePtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Internal handle to some opaque data.
#[repr(transparent)]
pub struct OpaqueHandle<T: ?Sized = *mut ()>(NonNull<std::ffi::c_void>, PhantomData<T>);

unsafe impl<T: Send> Send for OpaqueHandle<T> {}
unsafe impl<T: Sync> Sync for OpaqueHandle<T> {}

impl<T: ?Sized> Debug for OpaqueHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OpaqueHandle").field(&self.0).finish()
    }
}

impl<T: ?Sized> Display for OpaqueHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:p}", self.0)
    }
}

impl<T: ?Sized> Copy for OpaqueHandle<T> {}

impl<T: ?Sized> Clone for OpaqueHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> PartialEq for OpaqueHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: ?Sized> Eq for OpaqueHandle<T> {}

impl<T: ?Sized> PartialOrd for OpaqueHandle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for OpaqueHandle<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Hash for OpaqueHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Helper trait for types that can be borrowed.
pub trait Viewable {
    /// View type.
    type View<'a>: for<'v> Viewable<View<'v> = Self::View<'v>>
    where
        Self: 'a;

    /// Borrows a view to the data.
    fn view(&self) -> Self::View<'_>;
}

/// Used to transfer ownership to and from a ffi interface.
///
/// The ownership of a type is transferred by calling [`Self::into_ffi`] and
/// is transferred back by calling [`Self::from_ffi`].
pub trait FFITransferable<FfiType: Sized> {
    /// Transfers the ownership from a Rust type to a ffi type.
    fn into_ffi(self) -> FfiType;

    /// Assumes ownership of a ffi type.
    ///
    /// # Safety
    ///
    /// The caller must ensure to have the ownership of the ffi type.
    unsafe fn from_ffi(ffi: FfiType) -> Self;
}

/// Used to share ownership with and from a ffi interface.
///
/// The ownership of a type is shared by calling [`Self::share_to_ffi`] and
/// is borrowed by calling [`Self::borrow_from_ffi`].
pub trait FFISharable<FfiType: Sized> {
    type BorrowedView<'a>: 'a;

    /// Shares the value of a Rust type with a ffi type.
    fn share_to_ffi(&self) -> FfiType;

    /// Borrows the ownership of a ffi type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all invariants of the type are conserved.
    unsafe fn borrow_from_ffi<'a>(ffi: FfiType) -> Self::BorrowedView<'a>;
}
