//! Implementation of string types and utility functions.
use crate::span::SpanInner;
use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// A ‘string slice’, akin to `&str`.
///
/// String slices are always valid UTF-8.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ConstStr<'a> {
    inner: StrInner<false>,
    _phantom: PhantomData<&'a str>,
}

impl Deref for ConstStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl Borrow<str> for ConstStr<'_> {
    fn borrow(&self) -> &str {
        self.inner.borrow()
    }
}

impl Pointer for ConstStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.inner, f)
    }
}

impl From<&str> for ConstStr<'_> {
    fn from(s: &str) -> Self {
        Self {
            inner: s.into(),
            _phantom: Default::default(),
        }
    }
}

impl From<&mut str> for ConstStr<'_> {
    fn from(s: &mut str) -> Self {
        Self {
            inner: s.into(),
            _phantom: Default::default(),
        }
    }
}

impl From<MutStr<'_>> for ConstStr<'_> {
    fn from(s: MutStr<'_>) -> Self {
        Self {
            inner: s.inner.into(),
            _phantom: Default::default(),
        }
    }
}

impl<'a> From<ConstStr<'a>> for &str {
    fn from(s: ConstStr<'a>) -> Self {
        s.inner.into()
    }
}

impl Debug for ConstStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl Display for ConstStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Hash for ConstStr<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl PartialEq<ConstStr<'_>> for ConstStr<'_> {
    fn eq(&self, other: &ConstStr<'_>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl PartialEq<MutStr<'_>> for ConstStr<'_> {
    fn eq(&self, other: &MutStr<'_>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl PartialEq<str> for ConstStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.inner.eq(other)
    }
}

impl PartialEq<&str> for ConstStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.inner.eq(*other)
    }
}

impl PartialEq<&mut str> for ConstStr<'_> {
    fn eq(&self, other: &&mut str) -> bool {
        self.inner.eq(*other)
    }
}

impl Eq for ConstStr<'_> {}

impl PartialOrd<ConstStr<'_>> for ConstStr<'_> {
    fn partial_cmp(&self, other: &ConstStr<'_>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl PartialOrd<MutStr<'_>> for ConstStr<'_> {
    fn partial_cmp(&self, other: &MutStr<'_>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl PartialOrd<str> for ConstStr<'_> {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl PartialOrd<&str> for ConstStr<'_> {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl PartialOrd<&mut str> for ConstStr<'_> {
    fn partial_cmp(&self, other: &&mut str) -> Option<Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl Ord for ConstStr<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

/// A ‘string slice’, akin to `&mut str`.
///
/// String slices are always valid UTF-8.
#[repr(C)]
#[derive(Default)]
pub struct MutStr<'a> {
    inner: StrInner<true>,
    _phantom: PhantomData<&'a mut str>,
}

impl Deref for MutStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl DerefMut for MutStr<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

impl Borrow<str> for MutStr<'_> {
    fn borrow(&self) -> &str {
        self.inner.borrow()
    }
}

impl BorrowMut<str> for MutStr<'_> {
    fn borrow_mut(&mut self) -> &mut str {
        self.inner.borrow_mut()
    }
}

impl Pointer for MutStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.inner, f)
    }
}

impl From<&mut str> for MutStr<'_> {
    fn from(s: &mut str) -> Self {
        Self {
            inner: s.into(),
            _phantom: Default::default(),
        }
    }
}

impl<'a> From<MutStr<'a>> for &str {
    fn from(s: MutStr<'a>) -> Self {
        s.inner.into()
    }
}

impl<'a> From<MutStr<'a>> for &'a mut str {
    fn from(s: MutStr<'a>) -> Self {
        s.inner.into()
    }
}

impl Debug for MutStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl Display for MutStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Hash for MutStr<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl PartialEq<ConstStr<'_>> for MutStr<'_> {
    fn eq(&self, other: &ConstStr<'_>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl PartialEq<MutStr<'_>> for MutStr<'_> {
    fn eq(&self, other: &MutStr<'_>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl PartialEq<str> for MutStr<'_> {
    fn eq(&self, other: &str) -> bool {
        self.inner.eq(other)
    }
}

impl PartialEq<&str> for MutStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.inner.eq(*other)
    }
}

impl PartialEq<&mut str> for MutStr<'_> {
    fn eq(&self, other: &&mut str) -> bool {
        self.inner.eq(*other)
    }
}

impl Eq for MutStr<'_> {}

impl PartialOrd<ConstStr<'_>> for MutStr<'_> {
    fn partial_cmp(&self, other: &ConstStr<'_>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl PartialOrd<MutStr<'_>> for MutStr<'_> {
    fn partial_cmp(&self, other: &MutStr<'_>) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl PartialOrd<str> for MutStr<'_> {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl PartialOrd<&str> for MutStr<'_> {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl PartialOrd<&mut str> for MutStr<'_> {
    fn partial_cmp(&self, other: &&mut str) -> Option<Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl Ord for MutStr<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

/// A ‘string slice’, akin to `&str` and `&mut str`.
///
/// String slices are always valid UTF-8.
///
/// # Safety
///
/// Usage of this type is unsafe, as it does not track the lifetime of the contained string.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StrInner<const MUT: bool> {
    inner: SpanInner<u8, MUT>,
}

impl<const MUT: bool> Deref for StrInner<MUT> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // It is guaranteed that the invariants are met.
        unsafe { std::str::from_utf8_unchecked(self.inner.into()) }
    }
}

impl DerefMut for StrInner<true> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // It is guaranteed that the invariants are met.
        unsafe { std::str::from_utf8_unchecked_mut(self.inner.into()) }
    }
}

impl<const MUT: bool> Borrow<str> for StrInner<MUT> {
    fn borrow(&self) -> &str {
        (&**self).borrow()
    }
}

impl BorrowMut<str> for StrInner<true> {
    fn borrow_mut(&mut self) -> &mut str {
        (&mut **self).borrow_mut()
    }
}

impl<const MUT: bool> Pointer for StrInner<MUT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&&**self, f)
    }
}

impl From<StrInner<true>> for StrInner<false> {
    fn from(s: StrInner<true>) -> Self {
        Self {
            inner: s.inner.into(),
        }
    }
}

impl From<&str> for StrInner<false> {
    fn from(s: &str) -> Self {
        Self {
            inner: s.as_bytes().into(),
        }
    }
}

impl<const MUT: bool> From<&mut str> for StrInner<MUT> {
    fn from(s: &mut str) -> Self {
        Self {
            inner: unsafe { s.as_bytes_mut().into() },
        }
    }
}

impl<const MUT: bool> From<StrInner<MUT>> for &str {
    fn from(s: StrInner<MUT>) -> Self {
        // It is guaranteed that the invariants are met.
        unsafe { std::str::from_utf8_unchecked(s.inner.into()) }
    }
}

impl From<StrInner<true>> for &mut str {
    fn from(s: StrInner<true>) -> Self {
        // It is guaranteed that the invariants are met.
        unsafe { std::str::from_utf8_unchecked_mut(s.inner.into()) }
    }
}

impl<const MUT: bool> Debug for StrInner<MUT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<const MUT: bool> Display for StrInner<MUT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<const MUT: bool> Default for StrInner<MUT> {
    fn default() -> Self {
        <&mut str as Default>::default().into()
    }
}

impl<const MUT: bool> Hash for StrInner<MUT> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&**self).hash(state)
    }
}

impl<const MUT: bool, const MUT_2: bool> PartialEq<StrInner<MUT_2>> for StrInner<MUT> {
    fn eq(&self, other: &StrInner<MUT_2>) -> bool {
        (&**self).eq(&**other)
    }
}

impl<const MUT: bool> PartialEq<str> for StrInner<MUT> {
    fn eq(&self, other: &str) -> bool {
        (&**self).eq(other)
    }
}

impl<const MUT: bool> PartialEq<&str> for StrInner<MUT> {
    fn eq(&self, other: &&str) -> bool {
        (&**self).eq(*other)
    }
}

impl<const MUT: bool> PartialEq<&mut str> for StrInner<MUT> {
    fn eq(&self, other: &&mut str) -> bool {
        (&**self).eq(*other)
    }
}

impl<const MUT: bool> Eq for StrInner<MUT> {}

impl<const MUT: bool, const MUT_2: bool> PartialOrd<StrInner<MUT_2>> for StrInner<MUT> {
    fn partial_cmp(&self, other: &StrInner<MUT_2>) -> Option<Ordering> {
        (&**self).partial_cmp(&**other)
    }
}

impl<const MUT: bool> PartialOrd<str> for StrInner<MUT> {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        (&**self).partial_cmp(other)
    }
}

impl<const MUT: bool> PartialOrd<&str> for StrInner<MUT> {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        (&**self).partial_cmp(other)
    }
}

impl<const MUT: bool> PartialOrd<&mut str> for StrInner<MUT> {
    fn partial_cmp(&self, other: &&mut str) -> Option<Ordering> {
        (&**self).partial_cmp(other)
    }
}

impl<const MUT: bool> Ord for StrInner<MUT> {
    fn cmp(&self, other: &Self) -> Ordering {
        (&**self).cmp(&**other)
    }
}

/// Converts a slice of bytes to a string slice.
///
/// See [`std::str::from_utf8`].
///
/// # Safety
///
/// A [`SpanInner<T>`] does not track the lifetime of `T`.
pub unsafe fn from_utf8_inner(data: &[u8]) -> Result<StrInner<false>, std::str::Utf8Error> {
    std::str::from_utf8(data).map(|s| s.into())
}

/// Converts a slice of bytes to a string slice.
///
/// See [`std::str::from_utf8_mut`].
///
/// # Safety
///
/// A [`SpanInner<T>`] does not track the lifetime of `T`.
pub unsafe fn from_utf8_mut_inner(data: &mut [u8]) -> Result<StrInner<true>, std::str::Utf8Error> {
    std::str::from_utf8_mut(data).map(|s| s.into())
}

/// Converts a slice of bytes to a string slice.
///
/// # Safety
///
/// - A [`SpanInner<T>`] does not track the lifetime of `T`.
/// - See [`std::str::from_utf8_unchecked`].
pub const unsafe fn from_utf8_unchecked_inner(data: &[u8]) -> StrInner<false> {
    StrInner {
        inner: crate::span::from_raw_parts_inner(data.as_ptr(), data.len()),
    }
}

/// Converts a slice of bytes to a string slice.
///
/// # Safety
///
/// - A [`SpanInner<T>`] does not track the lifetime of `T`.
/// - See [`std::str::from_utf8_unchecked_mut`].
pub unsafe fn from_utf8_unchecked_mut_inner(data: &mut [u8]) -> StrInner<true> {
    std::str::from_utf8_unchecked_mut(data).into()
}

/// Converts a [`StrInner<false>`] to a [`ConstStr`].
///
/// # Safety
///
/// This function can assign an arbitrary lifetime to the returned string.
pub const unsafe fn from_inner<'a, const MUT: bool>(s: StrInner<MUT>) -> ConstStr<'a> {
    ConstStr {
        // safety: they have the same layout.
        inner: std::mem::transmute(s),
        _phantom: PhantomData,
    }
}

/// Converts a [`StrInner<true>`] to a [`MutStr`].
///
/// # Safety
///
/// This function can assign an arbitrary lifetime to the returned string.
pub unsafe fn from_inner_mut<'a>(s: StrInner<true>) -> MutStr<'a> {
    MutStr {
        inner: s,
        _phantom: Default::default(),
    }
}

/// Converts a slice of bytes to a string slice.
///
/// See [`std::str::from_utf8`].
///
/// # Safety
///
/// A [`SpanInner<T>`] does not track the lifetime of `T`.
pub fn from_utf8(data: &[u8]) -> Result<ConstStr<'_>, std::str::Utf8Error> {
    unsafe { from_utf8_inner(data).map(|s| from_inner(s)) }
}

/// Converts a slice of bytes to a string slice.
///
/// See [`std::str::from_utf8_mut`].
pub fn from_utf8_mut(data: &mut [u8]) -> Result<MutStr<'_>, std::str::Utf8Error> {
    unsafe { from_utf8_mut_inner(data).map(|s| from_inner_mut(s)) }
}

/// Converts a slice of bytes to a string slice.
///
/// # Safety
///
/// See [`std::str::from_utf8_unchecked`].
pub const unsafe fn from_utf8_unchecked(data: &[u8]) -> ConstStr<'_> {
    from_inner(from_utf8_unchecked_inner(data))
}

/// Converts a slice of bytes to a string slice.
///
/// # Safety
///
/// See [`std::str::from_utf8_unchecked_mut`].
pub unsafe fn from_utf8_unchecked_mut(data: &mut [u8]) -> MutStr<'_> {
    from_inner_mut(from_utf8_unchecked_mut_inner(data))
}
