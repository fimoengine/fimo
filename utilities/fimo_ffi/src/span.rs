//! Implementation of the span types und utility functions.
use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::ptr::NonNull;

use crate::marshal::CTypeBridge;

/// An immutable span.
///
/// Equivalent to a `&'a [T]`
#[repr(C)]
#[derive(Copy, Clone, Default, CTypeBridge)]
pub struct ConstSpan<'a, T> {
    inner: ConstSpanPtr<T>,
    _phantom: PhantomData<&'a [T]>,
}

unsafe impl<'a, T: Send> Send for ConstSpan<'a, T> {}

unsafe impl<'a, T: Sync> Sync for ConstSpan<'a, T> {}

impl<'a, T: Unpin> Unpin for ConstSpan<'a, T> {}

impl<'a, T: RefUnwindSafe> RefUnwindSafe for ConstSpan<'a, T> {}

impl<'a, T: UnwindSafe> UnwindSafe for ConstSpan<'a, T> {}

impl<'a, T> const Deref for ConstSpan<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let ptr: *const [T] = self.inner.into();
        unsafe { &*ptr }
    }
}

impl<'a, T> const Borrow<[T]> for ConstSpan<'a, T> {
    fn borrow(&self) -> &[T] {
        let ptr: *const [T] = self.inner.into();
        unsafe { &*ptr }
    }
}

impl<'a, T> const From<&'a [T]> for ConstSpan<'a, T> {
    fn from(s: &'a [T]) -> Self {
        Self {
            inner: s.into(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T> const From<&'a mut [T]> for ConstSpan<'a, T> {
    fn from(s: &'a mut [T]) -> Self {
        Self {
            inner: s.into(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T> const From<ConstSpan<'a, T>> for &'a [T] {
    fn from(s: ConstSpan<'a, T>) -> Self {
        let ptr: *const [T] = s.inner.into();
        unsafe { &*ptr }
    }
}

impl<'a, T> const From<MutSpan<'a, T>> for ConstSpan<'a, T> {
    fn from(s: MutSpan<'a, T>) -> Self {
        Self {
            inner: s.inner.into(),
            _phantom: PhantomData,
        }
    }
}

unsafe impl<'a, T> const CTypeBridge for &'a [T] {
    type Type = ConstSpan<'a, T>;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

impl<'a, T> AsRef<[T]> for ConstSpan<'a, T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<'a, T: Debug> Debug for ConstSpan<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<'a, T: Hash> Hash for ConstSpan<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: PartialEq<U>, U> PartialEq<ConstSpan<'_, U>> for ConstSpan<'_, T> {
    fn eq(&self, other: &ConstSpan<'_, U>) -> bool {
        (**self).eq(&**other)
    }
}

impl<T: PartialEq<U>, U> PartialEq<MutSpan<'_, U>> for ConstSpan<'_, T> {
    fn eq(&self, other: &MutSpan<'_, U>) -> bool {
        (**self).eq(&**other)
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for ConstSpan<'_, T> {
    fn eq(&self, other: &[U; N]) -> bool {
        (**self).eq(other)
    }
}

impl<T: Eq> Eq for ConstSpan<'_, T> {}

impl<T: PartialOrd<T>> PartialOrd<ConstSpan<'_, T>> for ConstSpan<'_, T> {
    fn partial_cmp(&self, other: &ConstSpan<'_, T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: PartialOrd<T>> PartialOrd<MutSpan<'_, T>> for ConstSpan<'_, T> {
    fn partial_cmp(&self, other: &MutSpan<'_, T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: Ord> Ord for ConstSpan<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

/// A mutable span.
///
/// Equivalent to a `&'a mut [T]`
#[repr(C)]
#[derive(Default, CTypeBridge)]
pub struct MutSpan<'a, T> {
    inner: MutSpanPtr<T>,
    _phantom: PhantomData<&'a mut [T]>,
}

unsafe impl<'a, T: Send> Send for MutSpan<'a, T> {}

unsafe impl<'a, T: Sync> Sync for MutSpan<'a, T> {}

impl<'a, T: Unpin> Unpin for MutSpan<'a, T> {}

impl<'a, T: RefUnwindSafe> RefUnwindSafe for MutSpan<'a, T> {}

impl<'a, T: UnwindSafe> UnwindSafe for MutSpan<'a, T> {}

impl<'a, T> const Deref for MutSpan<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let ptr: *const [T] = self.inner.into();
        unsafe { &*ptr }
    }
}

impl<'a, T> const DerefMut for MutSpan<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr: *mut [T] = self.inner.into();
        unsafe { &mut *ptr }
    }
}

impl<'a, T> const Borrow<[T]> for MutSpan<'a, T> {
    fn borrow(&self) -> &[T] {
        let ptr: *const [T] = self.inner.into();
        unsafe { &*ptr }
    }
}

impl<'a, T> const BorrowMut<[T]> for MutSpan<'a, T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        let ptr: *mut [T] = self.inner.into();
        unsafe { &mut *ptr }
    }
}

impl<'a, T> const From<&'a mut [T]> for MutSpan<'a, T> {
    fn from(s: &'a mut [T]) -> Self {
        Self {
            inner: s.into(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T> const From<MutSpan<'a, T>> for &'a [T] {
    fn from(s: MutSpan<'a, T>) -> Self {
        let ptr: *const [T] = s.inner.into();
        unsafe { &*ptr }
    }
}

impl<'a, T> const From<MutSpan<'a, T>> for &'a mut [T] {
    fn from(s: MutSpan<'a, T>) -> Self {
        let ptr: *mut [T] = s.inner.into();
        unsafe { &mut *ptr }
    }
}

unsafe impl<'a, T> const CTypeBridge for &'a mut [T] {
    type Type = MutSpan<'a, T>;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

impl<'a, T> AsRef<[T]> for MutSpan<'a, T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<'a, T> AsMut<[T]> for MutSpan<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'a, T: Debug> Debug for MutSpan<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<'a, T: Hash> Hash for MutSpan<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<T: PartialEq<U>, U> PartialEq<ConstSpan<'_, U>> for MutSpan<'_, T> {
    fn eq(&self, other: &ConstSpan<'_, U>) -> bool {
        (**self).eq(&**other)
    }
}

impl<T: PartialEq<U>, U> PartialEq<MutSpan<'_, U>> for MutSpan<'_, T> {
    fn eq(&self, other: &MutSpan<'_, U>) -> bool {
        (**self).eq(&**other)
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for MutSpan<'_, T> {
    fn eq(&self, other: &[U; N]) -> bool {
        (**self).eq(other)
    }
}

impl<T: Eq> Eq for MutSpan<'_, T> {}

impl<T: PartialOrd<T>> PartialOrd<ConstSpan<'_, T>> for MutSpan<'_, T> {
    fn partial_cmp(&self, other: &ConstSpan<'_, T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: PartialOrd<T>> PartialOrd<MutSpan<'_, T>> for MutSpan<'_, T> {
    fn partial_cmp(&self, other: &MutSpan<'_, T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: Ord> Ord for MutSpan<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

/// A span pointer.
///
/// Equivalent to a `*const [T]`
#[repr(C)]
#[derive(CTypeBridge)]
pub struct ConstSpanPtr<T> {
    ptr: *const T,
    len: usize,
}

impl<T> ConstSpanPtr<T> {
    /// Returns the length of the `ConstSpanPtr`.
    #[inline]
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns a pointer to the first element of the `ConstSpanPtr`.
    #[inline]
    pub const fn as_ptr(self) -> *const T {
        self.ptr
    }

    /// Dereferences the `ConstSpanPtr` to a [`ConstSpan`].
    ///
    /// # Safety
    ///
    /// This function performs the same operation as a pointer dereference.
    #[inline]
    pub const unsafe fn deref<'a>(self) -> ConstSpan<'a, T> {
        ConstSpan {
            inner: self,
            _phantom: PhantomData,
        }
    }
}

impl<T> const Copy for ConstSpanPtr<T> {}

impl<T> const Clone for ConstSpanPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            len: self.len,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.ptr = source.ptr;
        self.len = source.len;
    }
}

impl<T> Hash for ConstSpanPtr<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr: *const [T] = (*self).into();
        Hash::hash(&ptr, state)
    }
}

impl<T> PartialEq for ConstSpanPtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let ptr: *const [T] = (*self).into();
        let other: *const [T] = (*other).into();
        PartialEq::eq(&ptr, &other)
    }
}

impl<T> Eq for ConstSpanPtr<T> {}

impl<T> PartialOrd for ConstSpanPtr<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ptr: *const [T] = (*self).into();
        let other: *const [T] = (*other).into();
        PartialOrd::partial_cmp(&ptr, &other)
    }
}

impl<T> Ord for ConstSpanPtr<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let ptr: *const [T] = (*self).into();
        let other: *const [T] = (*other).into();
        Ord::cmp(&ptr, &other)
    }
}

impl<T> Debug for ConstSpanPtr<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.ptr, f)
    }
}

impl<T> Pointer for ConstSpanPtr<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.ptr, f)
    }
}

impl<T> const Default for ConstSpanPtr<T> {
    #[inline]
    fn default() -> Self {
        ptr_from_raw_parts(NonNull::dangling().as_ptr(), 0)
    }
}

impl<T> const From<&'_ [T]> for ConstSpanPtr<T> {
    #[inline]
    fn from(s: &'_ [T]) -> Self {
        ptr_from_raw_parts(s as *const _ as *const T, s.len())
    }
}

impl<T> const From<&'_ mut [T]> for ConstSpanPtr<T> {
    #[inline]
    fn from(s: &'_ mut [T]) -> Self {
        ptr_from_raw_parts(s as *mut _ as *mut T, s.len())
    }
}

impl<T> const From<*const [T]> for ConstSpanPtr<T> {
    #[inline]
    fn from(s: *const [T]) -> Self {
        ptr_from_raw_parts(s as *const T, s.len())
    }
}

impl<T> const From<*mut [T]> for ConstSpanPtr<T> {
    #[inline]
    fn from(s: *mut [T]) -> Self {
        ptr_from_raw_parts(s as *mut T, s.len())
    }
}

impl<T> const From<ConstSpanPtr<T>> for *const [T] {
    #[inline]
    fn from(s: ConstSpanPtr<T>) -> Self {
        std::ptr::slice_from_raw_parts(s.ptr, s.len)
    }
}

unsafe impl<T> const CTypeBridge for *const [T] {
    type Type = ConstSpanPtr<T>;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

/// A mutable span pointer.
///
/// Equivalent to a `*mut [T]`
#[repr(C)]
#[derive(CTypeBridge)]
pub struct MutSpanPtr<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> MutSpanPtr<T> {
    /// Returns the length of the `MutSpanPtr`.
    #[inline]
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns a pointer to the first element of the `MutSpanPtr`.
    #[inline]
    pub const fn as_ptr(self) -> *mut T {
        self.ptr
    }

    /// Dereferences the `MutSpanPtr` to a [`ConstSpan`].
    ///
    /// # Safety
    ///
    /// This function performs the same operation as a pointer dereference.
    #[inline]
    pub const unsafe fn deref<'a>(self) -> ConstSpan<'a, T> {
        ConstSpan {
            inner: self.into(),
            _phantom: PhantomData,
        }
    }

    /// Dereferences the `MutSpanPtr` to a [`MutSpan`].
    ///
    /// # Safety
    ///
    /// This function performs the same operation as a pointer dereference.
    #[inline]
    pub const unsafe fn deref_mut<'a>(self) -> MutSpan<'a, T> {
        MutSpan {
            inner: self,
            _phantom: PhantomData,
        }
    }
}

impl<T> const Copy for MutSpanPtr<T> {}

impl<T> const Clone for MutSpanPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            len: self.len,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.ptr = source.ptr;
        self.len = source.len;
    }
}

impl<T> Hash for MutSpanPtr<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr: *const [T] = (*self).into();
        Hash::hash(&ptr, state)
    }
}

impl<T> PartialEq for MutSpanPtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let ptr: *const [T] = (*self).into();
        let other: *const [T] = (*other).into();
        PartialEq::eq(&ptr, &other)
    }
}

impl<T> Eq for MutSpanPtr<T> {}

impl<T> PartialOrd for MutSpanPtr<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ptr: *const [T] = (*self).into();
        let other: *const [T] = (*other).into();
        PartialOrd::partial_cmp(&ptr, &other)
    }
}

impl<T> Ord for MutSpanPtr<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let ptr: *const [T] = (*self).into();
        let other: *const [T] = (*other).into();
        Ord::cmp(&ptr, &other)
    }
}

impl<T> Debug for MutSpanPtr<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.ptr, f)
    }
}

impl<T> Pointer for MutSpanPtr<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.ptr, f)
    }
}

impl<T> const Default for MutSpanPtr<T> {
    #[inline]
    fn default() -> Self {
        ptr_from_raw_parts_mut(NonNull::dangling().as_ptr(), 0)
    }
}

impl<T> const From<&'_ mut [T]> for MutSpanPtr<T> {
    #[inline]
    fn from(s: &'_ mut [T]) -> Self {
        ptr_from_raw_parts_mut(s as *mut _ as *mut T, s.len())
    }
}

impl<T> const From<*mut [T]> for MutSpanPtr<T> {
    #[inline]
    fn from(s: *mut [T]) -> Self {
        ptr_from_raw_parts_mut(s as *mut T, s.len())
    }
}

impl<T> const From<MutSpanPtr<T>> for ConstSpanPtr<T> {
    #[inline]
    fn from(s: MutSpanPtr<T>) -> Self {
        ptr_from_raw_parts(s.ptr, s.len)
    }
}

impl<T> const From<MutSpanPtr<T>> for *const [T] {
    #[inline]
    fn from(s: MutSpanPtr<T>) -> Self {
        std::ptr::slice_from_raw_parts(s.ptr, s.len)
    }
}

impl<T> const From<MutSpanPtr<T>> for *mut [T] {
    #[inline]
    fn from(s: MutSpanPtr<T>) -> Self {
        std::ptr::slice_from_raw_parts_mut(s.ptr, s.len)
    }
}

unsafe impl<T> const CTypeBridge for *mut [T] {
    type Type = MutSpanPtr<T>;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

/// Converts a reference to `T` into a [`ConstSpan<T>`] of length 1 (without copying).
#[inline]
pub const fn from_ref<T>(s: &T) -> ConstSpan<'_, T> {
    unsafe { ptr_from_raw_parts(s, 1).deref() }
}

/// Converts a reference to `T` into a [`MutSpan<T>`] of length 1 (without copying).
#[inline]
pub fn from_mut<T>(s: &mut T) -> MutSpan<'_, T> {
    unsafe { ptr_from_raw_parts_mut(s, 1).deref_mut() }
}

/// Forms s [`ConstSpan<T>`] from a pointer and a length.
///
/// # Safety
///
/// - See [`from_raw_parts`](std::slice::from_raw_parts) for more details.
#[inline]
pub const unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> ConstSpan<'a, T> {
    ptr_from_raw_parts(data, len).deref()
}

/// Performs the same functionality as [`from_raw_parts`], except
/// that a mutable [`MutSpan<T>`] is returned.
///
/// # Safety
///
/// - See [`from_raw_parts_mut`](std::slice::from_raw_parts_mut) for more details.
#[inline]
pub unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> MutSpan<'a, T> {
    ptr_from_raw_parts_mut(data, len).deref_mut()
}

/// Constructs a [`ConstSpanPtr`] from a pointer and a length.
///
/// See [`slice_from_raw_parts`](std::ptr::slice_from_raw_parts).
#[inline]
pub const fn ptr_from_raw_parts<T>(data: *const T, len: usize) -> ConstSpanPtr<T> {
    ConstSpanPtr { ptr: data, len }
}

/// Constructs a [`MutSpanPtr`] from a pointer and a length.
///
/// See [`slice_from_raw_parts_mut`](std::ptr::slice_from_raw_parts_mut).
#[inline]
pub const fn ptr_from_raw_parts_mut<T>(data: *mut T, len: usize) -> MutSpanPtr<T> {
    MutSpanPtr { ptr: data, len }
}
