//! Implementation of the `Span<T>` type.
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::slice::{Iter, IterMut};

/// An immutable span.
pub type ConstSpan<T> = Span<T, false>;

/// A mutable span.
pub type MutSpan<T> = Span<T, true>;

/// A view over a continuous region of data, akin to `&[T]` and `&mut [T]`.
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, Debug)]
pub struct Span<T, const MUT: bool>
where
    T: Copy + Sized,
{
    data: *const T,
    length: usize,
}

impl<T, const MUT: bool> Span<T, MUT>
where
    T: Copy + Sized,
{
    /// Create a new empty span.
    #[inline]
    pub fn new() -> Self {
        Self {
            data: std::ptr::null(),
            length: 0,
        }
    }

    /// Creates a new span from a mutable pointer and a length.
    ///
    /// # Safety
    ///
    /// Same restrictions as [from_raw_parts_mut](std::slice::from_raw_parts_mut) apply.
    #[inline]
    pub unsafe fn from_raw_parts_mut(ptr: *mut T, length: usize) -> Self {
        Self { data: ptr, length }
    }

    /// Fetches an immutable pointer of the elements the span points to.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.data
    }

    /// Retrieves the length of the span.
    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Checks if the span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_null() || self.length == 0
    }

    /// Constructs an iterator.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.as_ref().iter()
    }
}

impl<T> Span<T, false>
where
    T: Copy + Sized,
{
    /// Creates a new span from a pointer and a length.
    ///
    /// # Safety
    ///
    /// Same restrictions as [from_raw_parts](std::slice::from_raw_parts) apply.
    #[inline]
    pub unsafe fn from_raw_parts(ptr: *const T, length: usize) -> Self {
        Self { data: ptr, length }
    }
}

impl<T> Span<T, true>
where
    T: Copy + Sized,
{
    /// Fetches a mutable pointer of the elements the span points to.
    #[inline]
    pub fn as_ptr_mut(&mut self) -> *mut T {
        self.data as *mut T
    }

    /// Constructs a mutable iterator.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.as_mut().iter_mut()
    }
}

unsafe impl<T, const MUT: bool> Send for Span<T, MUT> where T: Copy + Sized + Send {}
unsafe impl<T, const MUT: bool> Sync for Span<T, MUT> where T: Copy + Sized + Sync {}

impl<T, const MUT: bool> AsRef<[T]> for Span<T, MUT>
where
    T: Copy + Sized,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        unsafe { slice_from_raw_parts(self.data, self.length) }
    }
}

impl<T> AsMut<[T]> for Span<T, true>
where
    T: Copy + Sized,
{
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { slice_from_raw_parts_mut(self.data as *mut T, self.length) }
    }
}

impl<T, const MUT: bool> Default for Span<T, MUT>
where
    T: Copy + Sized,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const MUT: bool> Deref for Span<T, MUT>
where
    T: Copy + Sized,
{
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Span<T, true>
where
    T: Copy + Sized,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T, const MUT: bool> Hash for Span<T, MUT>
where
    T: Copy + Hash + Sized,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<'a, T, const MUT: bool> IntoIterator for &'a Span<T, MUT>
where
    T: Copy + Sized,
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Span<T, true>
where
    T: Copy + Sized,
{
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, const MUT: bool> PartialEq for Span<T, MUT>
where
    T: Copy + PartialEq + Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<T> From<&T> for Span<T, false>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &T) -> Self {
        Self {
            data: p as *const T,
            length: 1,
        }
    }
}

impl<T, const MUT: bool> From<&mut T> for Span<T, MUT>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &mut T) -> Self {
        Self {
            data: p as *const T,
            length: 1,
        }
    }
}

impl<T> From<&[T]> for Span<T, false>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &[T]) -> Self {
        Self {
            data: p.as_ptr(),
            length: p.len(),
        }
    }
}

impl<T, const MUT: bool> From<&mut [T]> for Span<T, MUT>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &mut [T]) -> Self {
        Self {
            data: p.as_ptr(),
            length: p.len(),
        }
    }
}

impl<T, const N: usize> From<&[T; N]> for Span<T, false>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &[T; N]) -> Self {
        Self {
            data: p.as_ptr(),
            length: p.len(),
        }
    }
}

impl<T, const MUT: bool, const N: usize> From<&mut [T; N]> for Span<T, MUT>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &mut [T; N]) -> Self {
        Self {
            data: p.as_ptr(),
            length: p.len(),
        }
    }
}

impl<T> From<&Vec<T>> for Span<T, false>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &Vec<T>) -> Self {
        Self {
            data: p.as_ptr(),
            length: p.len(),
        }
    }
}

impl<T, const MUT: bool> From<&mut Vec<T>> for Span<T, MUT>
where
    T: Copy + Sized,
{
    #[inline]
    fn from(p: &mut Vec<T>) -> Self {
        Self {
            data: p.as_ptr(),
            length: p.len(),
        }
    }
}

impl From<&str> for Span<u8, false> {
    #[inline]
    fn from(p: &str) -> Self {
        let bytes = p.as_bytes();
        Self {
            data: bytes.as_ptr(),
            length: bytes.len(),
        }
    }
}

impl<const MUT: bool> From<&mut str> for Span<u8, MUT> {
    #[inline]
    fn from(p: &mut str) -> Self {
        let bytes = unsafe { p.as_bytes_mut() };
        Self {
            data: bytes.as_ptr(),
            length: bytes.len(),
        }
    }
}

impl From<&String> for Span<u8, false> {
    #[inline]
    fn from(p: &String) -> Self {
        let bytes = p.as_bytes();
        Self {
            data: bytes.as_ptr(),
            length: bytes.len(),
        }
    }
}

impl<const MUT: bool> From<&mut String> for Span<u8, MUT> {
    #[inline]
    fn from(p: &mut String) -> Self {
        let bytes = unsafe { p.as_bytes_mut() };
        Self {
            data: bytes.as_ptr(),
            length: bytes.len(),
        }
    }
}

/// Constructs a slice from a pointer and its length.
///
/// # Safety
///
/// The length must match the number of elements in the slice.
pub unsafe fn slice_from_raw_parts<'a, T>(ptr: *const T, length: usize) -> &'a [T] {
    if ptr.is_null() {
        std::slice::from_raw_parts(NonNull::dangling().as_ptr(), 0)
    } else {
        std::slice::from_raw_parts(ptr, length)
    }
}

/// Constructs a mutable slice from a pointer and its length.
///
/// # Safety
///
/// The length must match the number of elements in the slice.
pub unsafe fn slice_from_raw_parts_mut<'a, T>(ptr: *mut T, length: usize) -> &'a mut [T] {
    if ptr.is_null() {
        std::slice::from_raw_parts_mut(NonNull::dangling().as_ptr(), 0)
    } else {
        std::slice::from_raw_parts_mut(ptr, length)
    }
}
