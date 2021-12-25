//! Implementation of the span types und utility functions.
use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

/// An immutable span.
///
/// Equivalent of an `&'a [T]`
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ConstSpan<'a, T> {
    inner: SpanInner<T, false>,
    _phantom: PhantomData<&'a [T]>,
}

impl<'a, T> Deref for ConstSpan<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, T> Borrow<[T]> for ConstSpan<'a, T> {
    fn borrow(&self) -> &[T] {
        self.inner.borrow()
    }
}

impl<'a, T> Pointer for ConstSpan<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.inner, f)
    }
}

impl<'a, T> From<&'a [T]> for ConstSpan<'a, T> {
    fn from(s: &'a [T]) -> Self {
        Self {
            inner: s.into(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, T> From<&'a mut [T]> for ConstSpan<'a, T> {
    fn from(s: &'a mut [T]) -> Self {
        Self {
            inner: s.into(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, T> From<ConstSpan<'a, T>> for &'a [T] {
    fn from(s: ConstSpan<'a, T>) -> Self {
        s.inner.into()
    }
}

impl<'a, T> From<MutSpan<'a, T>> for ConstSpan<'a, T> {
    fn from(s: MutSpan<'a, T>) -> Self {
        Self {
            inner: s.inner.into(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, T> AsRef<[T]> for ConstSpan<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<'a, T: Debug> Debug for ConstSpan<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl<'a, T: Hash> Hash for ConstSpan<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T: PartialEq<U>, U> PartialEq<ConstSpan<'_, U>> for ConstSpan<'_, T> {
    fn eq(&self, other: &ConstSpan<'_, U>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: PartialEq<U>, U> PartialEq<MutSpan<'_, U>> for ConstSpan<'_, T> {
    fn eq(&self, other: &MutSpan<'_, U>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for ConstSpan<'_, T> {
    fn eq(&self, other: &[U; N]) -> bool {
        self.inner.eq(other)
    }
}

impl<T: Eq> Eq for ConstSpan<'_, T> {}

impl<T: PartialOrd<T>> PartialOrd<ConstSpan<'_, T>> for ConstSpan<'_, T> {
    fn partial_cmp(&self, other: &ConstSpan<'_, T>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: PartialOrd<T>> PartialOrd<MutSpan<'_, T>> for ConstSpan<'_, T> {
    fn partial_cmp(&self, other: &MutSpan<'_, T>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: Ord> Ord for ConstSpan<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

/// A mutable span.
///
/// Equivalent of an `&'a mut [T]`
#[repr(C)]
#[derive(Default)]
pub struct MutSpan<'a, T> {
    inner: SpanInner<T, true>,
    _phantom: PhantomData<&'a mut [T]>,
}

impl<'a, T> Deref for MutSpan<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, T> DerefMut for MutSpan<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

impl<'a, T> Borrow<[T]> for MutSpan<'a, T> {
    fn borrow(&self) -> &[T] {
        self.inner.borrow()
    }
}

impl<'a, T> BorrowMut<[T]> for MutSpan<'a, T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self.inner.borrow_mut()
    }
}

impl<'a, T> Pointer for MutSpan<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.inner, f)
    }
}

impl<'a, T> From<&'a mut [T]> for MutSpan<'a, T> {
    fn from(s: &'a mut [T]) -> Self {
        Self {
            inner: s.into(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, T> From<MutSpan<'a, T>> for &'a [T] {
    fn from(s: MutSpan<'a, T>) -> Self {
        s.inner.into()
    }
}

impl<'a, T> From<MutSpan<'a, T>> for &'a mut [T] {
    fn from(s: MutSpan<'a, T>) -> Self {
        s.inner.into()
    }
}

impl<'a, T> AsRef<[T]> for MutSpan<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.inner.as_ref()
    }
}

impl<'a, T> AsMut<[T]> for MutSpan<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.inner.as_mut()
    }
}

impl<'a, T: Debug> Debug for MutSpan<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl<'a, T: Hash> Hash for MutSpan<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T: PartialEq<U>, U> PartialEq<ConstSpan<'_, U>> for MutSpan<'_, T> {
    fn eq(&self, other: &ConstSpan<'_, U>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: PartialEq<U>, U> PartialEq<MutSpan<'_, U>> for MutSpan<'_, T> {
    fn eq(&self, other: &MutSpan<'_, U>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for MutSpan<'_, T> {
    fn eq(&self, other: &[U; N]) -> bool {
        self.inner.eq(other)
    }
}

impl<T: Eq> Eq for MutSpan<'_, T> {}

impl<T: PartialOrd<T>> PartialOrd<ConstSpan<'_, T>> for MutSpan<'_, T> {
    fn partial_cmp(&self, other: &ConstSpan<'_, T>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: PartialOrd<T>> PartialOrd<MutSpan<'_, T>> for MutSpan<'_, T> {
    fn partial_cmp(&self, other: &MutSpan<'_, T>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: Ord> Ord for MutSpan<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

/// A view over a continuous region of data, akin to `&[T]` and `&mut [T]`.
///
/// # Safety
///
/// Usage of this type is unsafe, as it does not track the lifetime of the contained data.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SpanInner<T, const MUT: bool> {
    ptr: NonNull<T>,
    len: usize,
}

unsafe impl<T: Send, const MUT: bool> Send for SpanInner<T, MUT> {}
unsafe impl<T: Sync, const MUT: bool> Sync for SpanInner<T, MUT> {}

impl<T, const MUT: bool> Deref for SpanInner<T, MUT> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // It is guaranteed that the invariants are met.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for SpanInner<T, true> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // It is guaranteed that the invariants are met.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T, const MUT: bool> Borrow<[T]> for SpanInner<T, MUT> {
    fn borrow(&self) -> &[T] {
        (&**self).borrow()
    }
}

impl<T> BorrowMut<[T]> for SpanInner<T, true> {
    fn borrow_mut(&mut self) -> &mut [T] {
        (&mut **self).borrow_mut()
    }
}

impl<T, const MUT: bool> Pointer for SpanInner<T, MUT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (&**self).fmt(f)
    }
}

impl<T> From<SpanInner<T, true>> for SpanInner<T, false> {
    fn from(s: SpanInner<T, true>) -> Self {
        SpanInner {
            ptr: s.ptr,
            len: s.len,
        }
    }
}

impl<T> From<&[T]> for SpanInner<T, false> {
    fn from(s: &[T]) -> Self {
        SpanInner {
            ptr: unsafe { NonNull::new_unchecked(s.as_ptr() as *mut _) },
            len: s.len(),
        }
    }
}

impl<T, const MUT: bool> From<&mut [T]> for SpanInner<T, MUT> {
    fn from(s: &mut [T]) -> Self {
        SpanInner {
            ptr: unsafe { NonNull::new_unchecked(s.as_mut_ptr()) },
            len: s.len(),
        }
    }
}

impl<T, const MUT: bool> From<SpanInner<T, MUT>> for &[T] {
    fn from(s: SpanInner<T, MUT>) -> Self {
        // It is guaranteed that the invariants are met.
        unsafe { std::slice::from_raw_parts(s.ptr.as_ptr(), s.len) }
    }
}

impl<T> From<SpanInner<T, true>> for &mut [T] {
    fn from(s: SpanInner<T, true>) -> Self {
        // It is guaranteed that the invariants are met.
        unsafe { std::slice::from_raw_parts_mut(s.ptr.as_ptr(), s.len) }
    }
}

impl<T, const MUT: bool> AsRef<[T]> for SpanInner<T, MUT> {
    fn as_ref(&self) -> &[T] {
        &**self
    }
}

impl<T> AsMut<[T]> for SpanInner<T, true> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut **self
    }
}

impl<T: Debug, const MUT: bool> Debug for SpanInner<T, MUT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (&**self).fmt(f)
    }
}

impl<T, const MUT: bool> Default for SpanInner<T, MUT> {
    fn default() -> Self {
        SpanInner {
            ptr: NonNull::dangling(),
            len: 0,
        }
    }
}

impl<T: Hash, const MUT: bool> Hash for SpanInner<T, MUT> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&**self).hash(state)
    }
}

impl<T: PartialEq<U>, U, const MUT: bool, const MUT_U: bool> PartialEq<SpanInner<U, MUT_U>>
    for SpanInner<T, MUT>
{
    fn eq(&self, other: &SpanInner<U, MUT_U>) -> bool {
        (&**self).eq(&**other)
    }
}

impl<T: PartialEq<U>, U, const MUT: bool, const N: usize> PartialEq<[U; N]> for SpanInner<T, MUT> {
    fn eq(&self, other: &[U; N]) -> bool {
        (&**self).eq(other)
    }
}

impl<T: Eq, const MUT: bool> Eq for SpanInner<T, MUT> {}

impl<T: PartialOrd<T>, const MUT: bool, const MUT_2: bool> PartialOrd<SpanInner<T, MUT_2>>
    for SpanInner<T, MUT>
{
    fn partial_cmp(&self, other: &SpanInner<T, MUT_2>) -> Option<Ordering> {
        (&**self).partial_cmp(&**other)
    }
}

impl<T: Ord, const MUT: bool> Ord for SpanInner<T, MUT> {
    fn cmp(&self, other: &Self) -> Ordering {
        (&**self).cmp(&**other)
    }
}

/// Converts a reference to `T` into a [`SpanInner<T, false>`] of length 1 (without copying).
///
/// # Safety
///
/// A [`SpanInner<T>`] does not track the lifetime of `T`.
pub unsafe fn from_ref_inner<T>(s: &T) -> SpanInner<T, false> {
    std::slice::from_ref(s).into()
}

/// Converts a reference to `T` into a [`SpanInner<T, true>`] of length 1 (without copying).
///
/// # Safety
///
/// A [`SpanInner<T>`] does not track the lifetime of `T`.
pub unsafe fn from_mut_inner<T>(s: &mut T) -> SpanInner<T, true> {
    std::slice::from_mut(s).into()
}

/// Forms s [`SpanInner<T, false>`] from a pointer and a length.
///
/// # Safety
///
/// - A [`SpanInner<T>`] does not track the lifetime of `T`.
/// - See [std::slice::from_raw_parts] for more details.
pub unsafe fn from_raw_parts_inner<T>(data: *const T, len: usize) -> SpanInner<T, false> {
    std::slice::from_raw_parts(data, len).into()
}

/// Performs the same functionality as [from_raw_parts_inner], except
/// that a mutable [`SpanInner<T, true>`] is returned.
///
/// # Safety
///
/// - A [`SpanInner<T>`] does not track the lifetime of `T`.
/// - See [std::slice::from_raw_parts] for more details.
pub unsafe fn from_raw_parts_mut_inner<T>(data: *mut T, len: usize) -> SpanInner<T, true> {
    std::slice::from_raw_parts_mut(data, len).into()
}

/// Converts a reference to `T` into a [`ConstSpan<T>`] of length 1 (without copying).
pub fn from_ref<T>(s: &T) -> ConstSpan<'_, T> {
    std::slice::from_ref(s).into()
}

/// Converts a reference to `T` into a [`MutSpan<T>`] of length 1 (without copying).
pub fn from_mut<T>(s: &mut T) -> MutSpan<'_, T> {
    std::slice::from_mut(s).into()
}

/// Converts a [`SpanInner<T, false>`] to a [`ConstSpan<T>`].
///
/// # Safety
///
/// This function can assign an arbitrary lifetime to the returned span.
pub unsafe fn from_inner<'a, T, const MUT: bool>(s: SpanInner<T, MUT>) -> ConstSpan<'a, T> {
    ConstSpan {
        // safety: they have the same layout.
        inner: std::mem::transmute(s),
        _phantom: Default::default(),
    }
}

/// Converts a [`SpanInner<T, true>`] to a [`MutSpan<T>`].
///
/// # Safety
///
/// This function can assign an arbitrary lifetime to the returned span.
pub unsafe fn from_inner_mut<'a, T>(s: SpanInner<T, true>) -> MutSpan<'a, T> {
    MutSpan {
        inner: s,
        _phantom: Default::default(),
    }
}

/// Forms s [`ConstSpan<T>`] from a pointer and a length.
///
/// # Safety
///
/// - See [std::slice::from_raw_parts] for more details.
pub unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> ConstSpan<'a, T> {
    from_inner(from_raw_parts_inner(data, len))
}

/// Performs the same functionality as [from_raw_parts], except
/// that a mutable [`MutSpan<T>`] is returned.
///
/// # Safety
///
/// - See [std::slice::from_raw_parts] for more details.
pub unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> MutSpan<'a, T> {
    from_inner_mut(from_raw_parts_mut_inner(data, len))
}
