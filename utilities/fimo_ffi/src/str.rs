//! Implementation of string types and utility functions.
use crate::marshal::CTypeBridge;
use crate::span::{ConstSpanPtr, MutSpanPtr};
use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter, Pointer};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};

/// A ‘string slice’, akin to `&str`.
///
/// String slices are always valid UTF-8.
#[repr(C)]
#[derive(Copy, Clone, Default, CTypeBridge)]
pub struct ConstStr<'a> {
    ptr: ConstStrPtr,
    _phantom: PhantomData<&'a str>,
}

impl<'a> ConstStr<'a> {
    /// Casts the reference to a [`&str`].
    pub const fn as_ref(&self) -> &'a str {
        let str = self.ptr.as_str_ptr();
        unsafe { &*str }
    }
}

unsafe impl<'a> Send for ConstStr<'a> {}

unsafe impl<'a> Sync for ConstStr<'a> {}

impl<'a> Unpin for ConstStr<'a> {}

impl<'a> RefUnwindSafe for ConstStr<'a> {}

impl<'a> UnwindSafe for ConstStr<'a> {}

impl Deref for ConstStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        let str: *const str = self.ptr.into();
        unsafe { &*str }
    }
}

impl Borrow<str> for ConstStr<'_> {
    fn borrow(&self) -> &str {
        let str: *const str = self.ptr.into();
        unsafe { &*str }
    }
}

impl From<&str> for ConstStr<'_> {
    fn from(s: &str) -> Self {
        Self {
            ptr: s.into(),
            _phantom: PhantomData,
        }
    }
}

impl From<&mut str> for ConstStr<'_> {
    fn from(s: &mut str) -> Self {
        Self {
            ptr: s.into(),
            _phantom: PhantomData,
        }
    }
}

impl From<MutStr<'_>> for ConstStr<'_> {
    fn from(s: MutStr<'_>) -> Self {
        Self {
            ptr: s.ptr.into(),
            _phantom: PhantomData,
        }
    }
}

impl<'a> From<ConstStr<'a>> for &'a str {
    fn from(s: ConstStr<'a>) -> Self {
        let str: *const str = s.ptr.into();
        unsafe { &*str }
    }
}

unsafe impl<'a> CTypeBridge for &'a str {
    type Type = ConstStr<'a>;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

impl Debug for ConstStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl Display for ConstStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl Hash for ConstStr<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl PartialEq<ConstStr<'_>> for ConstStr<'_> {
    fn eq(&self, other: &ConstStr<'_>) -> bool {
        (**self).eq(&**other)
    }
}

impl PartialEq<MutStr<'_>> for ConstStr<'_> {
    fn eq(&self, other: &MutStr<'_>) -> bool {
        (**self).eq(&**other)
    }
}

impl PartialEq<str> for ConstStr<'_> {
    fn eq(&self, other: &str) -> bool {
        (**self).eq(other)
    }
}

impl PartialEq<&str> for ConstStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        (**self).eq(*other)
    }
}

impl PartialEq<&mut str> for ConstStr<'_> {
    fn eq(&self, other: &&mut str) -> bool {
        (**self).eq(*other)
    }
}

impl Eq for ConstStr<'_> {}

impl PartialOrd<ConstStr<'_>> for ConstStr<'_> {
    fn partial_cmp(&self, other: &ConstStr<'_>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl PartialOrd<MutStr<'_>> for ConstStr<'_> {
    fn partial_cmp(&self, other: &MutStr<'_>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl PartialOrd<str> for ConstStr<'_> {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        (**self).partial_cmp(other)
    }
}

impl PartialOrd<&str> for ConstStr<'_> {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        (**self).partial_cmp(other)
    }
}

impl PartialOrd<&mut str> for ConstStr<'_> {
    fn partial_cmp(&self, other: &&mut str) -> Option<Ordering> {
        (**self).partial_cmp(other)
    }
}

impl Ord for ConstStr<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

/// A ‘string slice’, akin to `&mut str`.
///
/// String slices are always valid UTF-8.
#[repr(C)]
#[derive(Default, CTypeBridge)]
pub struct MutStr<'a> {
    ptr: MutStrPtr,
    _phantom: PhantomData<&'a mut str>,
}

impl<'a> MutStr<'a> {
    /// Casts the reference to a [`&str`].
    pub const fn as_ref(&self) -> &'a str {
        let str = self.ptr.as_str_ptr();
        unsafe { &*str }
    }

    /// Casts the reference to a [`&str`].
    pub const fn as_ref_mut(&mut self) -> &'a str {
        let str = self.ptr.as_str_ptr_mut();
        unsafe { &mut *str }
    }
}

unsafe impl<'a> Send for MutStr<'a> {}

unsafe impl<'a> Sync for MutStr<'a> {}

impl<'a> Unpin for MutStr<'a> {}

impl<'a> RefUnwindSafe for MutStr<'a> {}

impl<'a> UnwindSafe for MutStr<'a> {}

impl Deref for MutStr<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        let str: *const str = self.ptr.into();
        unsafe { &*str }
    }
}

impl DerefMut for MutStr<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let str: *mut str = self.ptr.into();
        unsafe { &mut *str }
    }
}

impl Borrow<str> for MutStr<'_> {
    fn borrow(&self) -> &str {
        let str: *const str = self.ptr.into();
        unsafe { &*str }
    }
}

impl BorrowMut<str> for MutStr<'_> {
    fn borrow_mut(&mut self) -> &mut str {
        let str: *mut str = self.ptr.into();
        unsafe { &mut *str }
    }
}

impl From<&mut str> for MutStr<'_> {
    fn from(s: &mut str) -> Self {
        Self {
            ptr: s.into(),
            _phantom: PhantomData,
        }
    }
}

impl<'a> From<MutStr<'a>> for &'a str {
    fn from(s: MutStr<'a>) -> Self {
        let str: *const str = s.ptr.into();
        unsafe { &*str }
    }
}

impl<'a> From<MutStr<'a>> for &'a mut str {
    fn from(s: MutStr<'a>) -> Self {
        let str: *mut str = s.ptr.into();
        unsafe { &mut *str }
    }
}

unsafe impl<'a> CTypeBridge for &'a mut str {
    type Type = MutStr<'a>;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

impl Debug for MutStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl Display for MutStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl Hash for MutStr<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl PartialEq<ConstStr<'_>> for MutStr<'_> {
    fn eq(&self, other: &ConstStr<'_>) -> bool {
        (**self).eq(&**other)
    }
}

impl PartialEq<MutStr<'_>> for MutStr<'_> {
    fn eq(&self, other: &MutStr<'_>) -> bool {
        (**self).eq(&**other)
    }
}

impl PartialEq<str> for MutStr<'_> {
    fn eq(&self, other: &str) -> bool {
        (**self).eq(other)
    }
}

impl PartialEq<&str> for MutStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        (**self).eq(*other)
    }
}

impl PartialEq<&mut str> for MutStr<'_> {
    fn eq(&self, other: &&mut str) -> bool {
        (**self).eq(*other)
    }
}

impl Eq for MutStr<'_> {}

impl PartialOrd<ConstStr<'_>> for MutStr<'_> {
    fn partial_cmp(&self, other: &ConstStr<'_>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl PartialOrd<MutStr<'_>> for MutStr<'_> {
    fn partial_cmp(&self, other: &MutStr<'_>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl PartialOrd<str> for MutStr<'_> {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        (**self).partial_cmp(other)
    }
}

impl PartialOrd<&str> for MutStr<'_> {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        (**self).partial_cmp(other)
    }
}

impl PartialOrd<&mut str> for MutStr<'_> {
    fn partial_cmp(&self, other: &&mut str) -> Option<Ordering> {
        (**self).partial_cmp(other)
    }
}

impl Ord for MutStr<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

/// A str pointer.
///
/// Equivalent to a `*const str`
#[repr(C)]
#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Default, CTypeBridge)]
pub struct ConstStrPtr {
    ptr: ConstSpanPtr<u8>,
}

impl ConstStrPtr {
    /// Constructs a [`*const str`] from the current pointer.
    #[inline]
    pub const fn as_str_ptr(self) -> *const str {
        std::ptr::from_raw_parts(self.ptr.as_ptr() as _, self.ptr.len())
    }

    /// Dereferences the `ConstStrPtr` to a [`ConstStr`].
    ///
    /// # Safety
    ///
    /// This function performs the same operation as a pointer dereference.
    #[inline]
    pub const unsafe fn deref<'a>(self) -> ConstStr<'a> {
        ConstStr {
            ptr: self,
            _phantom: PhantomData,
        }
    }
}

impl Debug for ConstStrPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.ptr, f)
    }
}

impl Pointer for ConstStrPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.ptr, f)
    }
}

impl From<&'_ str> for ConstStrPtr {
    fn from(s: &'_ str) -> Self {
        ptr_from_raw_parts(s.as_ptr(), s.len())
    }
}

impl From<&'_ mut str> for ConstStrPtr {
    fn from(s: &'_ mut str) -> Self {
        ptr_from_raw_parts(s.as_ptr(), s.len())
    }
}

impl From<*const str> for ConstStrPtr {
    fn from(s: *const str) -> Self {
        ptr_from_raw_parts(s as *const _ as *const u8, std::ptr::metadata(s) as _)
    }
}

impl From<*mut str> for ConstStrPtr {
    fn from(s: *mut str) -> Self {
        ptr_from_raw_parts(s as *const _ as *const u8, std::ptr::metadata(s) as _)
    }
}

impl From<ConstStrPtr> for *const str {
    fn from(s: ConstStrPtr) -> Self {
        std::ptr::from_raw_parts(s.ptr.as_ptr() as _, s.ptr.len())
    }
}

unsafe impl CTypeBridge for *const str {
    type Type = ConstStrPtr;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

/// A mutable str pointer.
///
/// Equivalent to a `*mut str`
#[repr(C)]
#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Default, CTypeBridge)]
pub struct MutStrPtr {
    ptr: MutSpanPtr<u8>,
}

impl MutStrPtr {
    /// Constructs a [`ConstStrPtr`] from the current pointer.
    #[inline]
    pub const fn as_const_ptr(self) -> ConstStrPtr {
        ConstStrPtr {
            ptr: self.ptr.as_const_span_ptr(),
        }
    }

    /// Constructs a [`*const str`] from the current pointer.
    #[inline]
    pub const fn as_str_ptr(self) -> *const str {
        std::ptr::from_raw_parts(self.ptr.as_ptr() as _, self.ptr.len())
    }

    /// Constructs a [`*mut str`] from the current pointer.
    #[inline]
    pub const fn as_str_ptr_mut(self) -> *mut str {
        std::ptr::from_raw_parts_mut(self.ptr.as_ptr() as _, self.ptr.len())
    }

    /// Dereferences the `MutStrPtr` to a [`ConstStr`].
    ///
    /// # Safety
    ///
    /// This function performs the same operation as a pointer dereference.
    #[inline]
    pub const unsafe fn deref<'a>(self) -> ConstStr<'a> {
        ConstStr {
            ptr: self.as_const_ptr(),
            _phantom: PhantomData,
        }
    }

    /// Dereferences the `MutStrPtr` to a [`MutStr`].
    ///
    /// # Safety
    ///
    /// This function performs the same operation as a pointer dereference.
    #[inline]
    pub const unsafe fn deref_mut<'a>(self) -> MutStr<'a> {
        MutStr {
            ptr: self,
            _phantom: PhantomData,
        }
    }
}

impl Debug for MutStrPtr {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.ptr, f)
    }
}

impl Pointer for MutStrPtr {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.ptr, f)
    }
}

impl From<&'_ mut str> for MutStrPtr {
    #[inline]
    fn from(s: &'_ mut str) -> Self {
        From::from(s as *mut _)
    }
}

impl From<*mut str> for MutStrPtr {
    #[inline]
    fn from(s: *mut str) -> Self {
        ptr_from_raw_parts_mut(s as *mut u8, std::ptr::metadata(s) as _)
    }
}

impl From<MutStrPtr> for ConstStrPtr {
    #[inline]
    fn from(s: MutStrPtr) -> Self {
        ptr_from_raw_parts(s.ptr.as_ptr(), s.ptr.len())
    }
}

impl From<MutStrPtr> for *const str {
    #[inline]
    fn from(s: MutStrPtr) -> Self {
        std::ptr::from_raw_parts(s.ptr.as_ptr() as _, s.ptr.len())
    }
}

impl From<MutStrPtr> for *mut str {
    #[inline]
    fn from(s: MutStrPtr) -> Self {
        std::ptr::from_raw_parts_mut(s.ptr.as_ptr() as _, s.ptr.len())
    }
}

unsafe impl CTypeBridge for *mut str {
    type Type = MutStrPtr;

    fn marshal(self) -> Self::Type {
        self.into()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        x.into()
    }
}

/// Converts a slice of bytes to a string slice.
///
/// See [`std::str::from_utf8`].
#[inline]
pub fn from_utf8(v: &[u8]) -> Result<ConstStr<'_>, std::str::Utf8Error> {
    match std::str::from_utf8(v) {
        Ok(_) => unsafe { Ok(from_utf8_unchecked(v)) },
        Err(e) => Err(e),
    }
}

/// Converts a slice of bytes to a string slice.
///
/// See [`std::str::from_utf8_mut`].
#[inline]
pub fn from_utf8_mut(v: &mut [u8]) -> Result<MutStr<'_>, std::str::Utf8Error> {
    match std::str::from_utf8_mut(v) {
        Ok(_) => unsafe { Ok(from_utf8_unchecked_mut(v)) },
        Err(e) => Err(e),
    }
}

/// Converts a slice of bytes to a string slice.
///
/// # Safety
///
/// See [`std::str::from_utf8_unchecked`].
#[inline]
pub const unsafe fn from_utf8_unchecked(v: &[u8]) -> ConstStr<'_> {
    ptr_from_raw_parts(v.as_ptr(), v.len()).deref()
}

/// Converts a slice of bytes to a string slice.
///
/// # Safety
///
/// See [`std::str::from_utf8_unchecked_mut`].
#[inline]
pub const unsafe fn from_utf8_unchecked_mut(v: &mut [u8]) -> MutStr<'_> {
    ptr_from_raw_parts_mut(v as *mut _ as *mut _, v.len()).deref_mut()
}

/// Constructs a [`ConstStrPtr`] from a pointer and a length.
///
/// See [`from_raw_parts`](std::ptr::from_raw_parts).
#[inline]
pub const fn ptr_from_raw_parts(data: *const u8, len: usize) -> ConstStrPtr {
    ConstStrPtr {
        ptr: crate::span::ptr_from_raw_parts(data, len),
    }
}

/// Constructs a [`MutStrPtr`] from a pointer and a length.
///
/// See [`from_raw_parts_mut`](std::ptr::from_raw_parts_mut).
#[inline]
pub const fn ptr_from_raw_parts_mut(data: *mut u8, len: usize) -> MutStrPtr {
    MutStrPtr {
        ptr: crate::span::ptr_from_raw_parts_mut(data, len),
    }
}
