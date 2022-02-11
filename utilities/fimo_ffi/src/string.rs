//! String type.
// This is a modified implementation of the `String` type found in
// the std library, which is dual-licensed under Apache 2.0 and MIT terms.
// All rights go to the contributors of the Rust project.
use crate::{Vec, Version};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::alloc::{Allocator, Global};
use std::borrow::{Borrow, BorrowMut};
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use std::cmp::Ordering;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::{
    Add, AddAssign, Deref, DerefMut, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull,
    RangeInclusive, RangeTo, RangeToInclusive,
};
use std::str::{Chars, FromStr, Utf8Error};

/// String data type.
#[repr(transparent)]
pub struct String<A: Allocator = Global> {
    vec: Vec<u8, A>,
}

impl String {
    /// Creates a new empty `String`.
    ///
    /// Given that the `String` is empty, this will not allocate any initial
    /// buffer. While that means that this initial operation is very
    /// inexpensive, it may cause excessive allocation later when you add
    /// data. If you have an idea of how much data the `String` will hold,
    /// consider the [`with_capacity`] method to prevent excessive
    /// re-allocation.
    ///
    /// [`with_capacity`]: String::with_capacity
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    /// let s = String::new();
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> String {
        String::new_in(Global)
    }

    /// Creates a new empty `String` with a particular capacity.
    ///
    /// `String`s have an internal buffer to hold their data. The capacity is
    /// the length of that buffer, and can be queried with the [`capacity`]
    /// method. This method creates an empty `String`, but one with an initial
    /// buffer that can hold `capacity` bytes. This is useful when you may be
    /// appending a bunch of data to the `String`, reducing the number of
    /// reallocations it needs to do.
    ///
    /// [`capacity`]: String::capacity
    ///
    /// If the given capacity is `0`, no allocation will occur, and this method
    /// is identical to the [`new`] method.
    ///
    /// [`new`]: String::new
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::with_capacity(10);
    ///
    /// // The String contains no chars, even though it has capacity for more
    /// assert_eq!(s.len(), 0);
    ///
    /// // These are all done without reallocating...
    /// let cap = s.capacity();
    /// for _ in 0..10 {
    ///     s.push('a');
    /// }
    ///
    /// assert_eq!(s.capacity(), cap);
    ///
    /// // ...but this may make the string reallocate
    /// s.push('a');
    /// ```
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> String {
        String::with_capacity_in(capacity, Global)
    }

    /// Decode a UTF-16–encoded vector `v` into a `String`, returning [`Err`]
    /// if `v` contains any invalid data.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    ///
    /// // 𝄞music
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0x0073, 0x0069, 0x0063];
    /// assert_eq!(String::from("𝄞music"),
    ///            String::from_utf16(v).unwrap());
    ///
    /// // 𝄞mu<invalid>ic
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0xD800, 0x0069, 0x0063];
    /// assert!(String::from_utf16(v).is_err());
    /// ```
    #[inline]
    pub fn from_utf16(v: &[u16]) -> Result<String, FromUtf16Error> {
        String::from_utf16_in(v, Global)
    }

    /// Decode a UTF-16–encoded slice `v` into a `String`, replacing
    /// invalid data with [the replacement character (`U+FFFD`)][U+FFFD].
    ///
    /// `from_utf16_lossy` returns a `String` since the UTF-16 to UTF-8
    /// conversion requires a memory allocation.
    ///
    /// [`from_utf8_lossy`]: String::from_utf8_lossy
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    ///
    /// // 𝄞mus<invalid>ic<invalid>
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0x0073, 0xDD1E, 0x0069, 0x0063,
    ///           0xD834];
    ///
    /// assert_eq!(String::from("𝄞mus\u{FFFD}ic\u{FFFD}"),
    ///            String::from_utf16_lossy(v));
    /// ```
    #[inline]
    #[must_use]
    pub fn from_utf16_lossy(v: &[u16]) -> String {
        decode_utf16(v.iter().cloned())
            .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
            .collect()
    }

    /// Creates a new `String` from a length, capacity, and pointer.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * The memory at `buf` needs to have been previously allocated by the
    ///   same allocator the standard library uses, with a required alignment of exactly 1.
    /// * `length` needs to be less than or equal to `capacity`.
    /// * `capacity` needs to be the correct value.
    /// * The first `length` bytes at `buf` need to be valid UTF-8.
    ///
    /// Violating these may cause problems like corrupting the allocator's
    /// internal data structures.
    ///
    /// The ownership of `buf` is effectively transferred to the
    /// `String` which may then deallocate, reallocate or change the
    /// contents of memory pointed to by the pointer at will. Ensure
    /// that nothing else uses the pointer after calling this
    /// function.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    /// use std::mem;
    ///
    /// unsafe {
    ///     let s = String::from("hello");
    ///
    ///     // Prevent automatically dropping the String's data
    ///     let mut s = mem::ManuallyDrop::new(s);
    ///
    ///     let ptr = s.as_mut_ptr();
    ///     let len = s.len();
    ///     let capacity = s.capacity();
    ///
    ///     let s = String::from_raw_parts(ptr, len, capacity);
    ///
    ///     assert_eq!(String::from("hello"), s);
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn from_raw_parts(buf: *mut u8, length: usize, capacity: usize) -> String {
        String::from_raw_parts_in(buf, length, capacity, Global)
    }

    /// Converts this `String` into a <code>[Box]<[str]></code>.
    ///
    /// This will drop any excess capacity.
    ///
    /// [str]: prim@str "str"
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let s = String::from("hello");
    ///
    /// let b = s.into_boxed_str();
    /// ```
    #[inline]
    #[must_use]
    pub fn into_boxed_str(self) -> Box<str> {
        let slice = self.vec.into_boxed_slice();
        unsafe { std::str::from_boxed_utf8_unchecked(slice) }
    }
}

impl<A: Allocator> String<A> {
    /// Creates a new empty `String`.
    ///
    /// Given that the `String` is empty, this will not allocate any initial
    /// buffer. While that means that this initial operation is very
    /// inexpensive, it may cause excessive allocation later when you add
    /// data. If you have an idea of how much data the `String` will hold,
    /// consider the [`with_capacity`] method to prevent excessive
    /// re-allocation.
    ///
    /// [`with_capacity`]: String::with_capacity
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_ffi::String;
    ///
    /// let s = String::new_in(Global);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new_in(alloc: A) -> String<A> {
        String {
            vec: Vec::new_in(alloc),
        }
    }

    /// Creates a new empty `String` with a particular capacity.
    ///
    /// `String`s have an internal buffer to hold their data. The capacity is
    /// the length of that buffer, and can be queried with the [`capacity`]
    /// method. This method creates an empty `String`, but one with an initial
    /// buffer that can hold `capacity` bytes. This is useful when you may be
    /// appending a bunch of data to the `String`, reducing the number of
    /// reallocations it needs to do.
    ///
    /// [`capacity`]: String::capacity
    ///
    /// If the given capacity is `0`, no allocation will occur, and this method
    /// is identical to the [`new`] method.
    ///
    /// [`new`]: String::new
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::with_capacity_in(10, Global);
    ///
    /// // The String contains no chars, even though it has capacity for more
    /// assert_eq!(s.len(), 0);
    ///
    /// // These are all done without reallocating...
    /// let cap = s.capacity();
    /// for _ in 0..10 {
    ///     s.push('a');
    /// }
    ///
    /// assert_eq!(s.capacity(), cap);
    ///
    /// // ...but this may make the string reallocate
    /// s.push('a');
    /// ```
    #[inline]
    #[must_use]
    pub fn with_capacity_in(capacity: usize, alloc: A) -> String<A> {
        String {
            vec: Vec::with_capacity_in(capacity, alloc),
        }
    }

    /// Converts a vector of bytes to a `String`.
    ///
    /// A string ([`String`]) is made of bytes ([`u8`]), and a vector of bytes
    /// ([`Vec<u8>`]) is made of bytes, so this function converts between the
    /// two. Not all byte slices are valid `String`s, however: `String`
    /// requires that it is valid UTF-8. `from_utf8()` checks to ensure that
    /// the bytes are valid UTF-8, and then does the conversion.
    ///
    /// If you are sure that the byte slice is valid UTF-8, and you don't want
    /// to incur the overhead of the validity check, there is an unsafe version
    /// of this function, [`from_utf8_unchecked`], which has the same behavior
    /// but skips the check.
    ///
    /// This method will take care to not copy the vector, for efficiency's
    /// sake.
    ///
    /// If you need a [`&str`] instead of a `String`, consider
    /// [`std::str::from_utf8`].
    ///
    /// The inverse of this method is [`into_bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the slice is not UTF-8 with a description as to why the
    /// provided bytes are not UTF-8. The vector you moved in is also included.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    /// // some bytes, in a vector
    /// let sparkle_heart = vec![240, 159, 146, 150];
    ///
    /// // We know these bytes are valid, so we'll use `unwrap()`.
    /// let sparkle_heart = String::from_utf8(sparkle_heart).unwrap();
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    ///
    /// Incorrect bytes:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    /// // some invalid bytes, in a vector
    /// let sparkle_heart = vec![0, 159, 146, 150];
    ///
    /// assert!(String::from_utf8(sparkle_heart).is_err());
    /// ```
    ///
    /// See the docs for [`FromUtf8Error`] for more details on what you can do
    /// with this error.
    ///
    /// [`from_utf8_unchecked`]: String::from_utf8_unchecked
    /// [`Vec<u8>`]: crate::vec::Vec "Vec"
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: String::into_bytes
    #[inline]
    pub fn from_utf8(vec: Vec<u8, A>) -> Result<String<A>, FromUtf8Error<A>> {
        match std::str::from_utf8(&vec) {
            Ok(_) => Ok(String { vec }),
            Err(err) => Err(FromUtf8Error {
                bytes: vec,
                error: err,
            }),
        }
    }
    /// Decode a UTF-16–encoded vector `v` into a `String`, returning [`Err`]
    /// if `v` contains any invalid data.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use fimo_ffi::{String, vec};
    /// use std::alloc::Global;
    ///
    /// // 𝄞music
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0x0073, 0x0069, 0x0063];
    /// assert_eq!(String::from("𝄞music"),
    ///            String::from_utf16_in(v, Global).unwrap());
    ///
    /// // 𝄞mu<invalid>ic
    /// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
    ///           0xD800, 0x0069, 0x0063];
    /// assert!(String::from_utf16_in(v, Global).is_err());
    /// ```
    pub fn from_utf16_in(v: &[u16], alloc: A) -> Result<String<A>, FromUtf16Error> {
        let mut ret = String::with_capacity_in(v.len(), alloc);
        for c in decode_utf16(v.iter().cloned()) {
            if let Ok(c) = c {
                ret.push(c);
            } else {
                return Err(FromUtf16Error(()));
            }
        }
        Ok(ret)
    }

    /// Decomposes a `String` into its raw components.
    ///
    /// Returns the raw pointer to the underlying data, the length of
    /// the string (in bytes), and the allocated capacity of the data
    /// (in bytes). These are the same arguments in the same order as
    /// the arguments to [`from_raw_parts`].
    ///
    /// After calling this function, the caller is responsible for the
    /// memory previously managed by the `String`. The only way to do
    /// this is to convert the raw pointer, length, and capacity back
    /// into a `String` with the [`from_raw_parts`] function, allowing
    /// the destructor to perform the cleanup.
    ///
    /// [`from_raw_parts`]: String::from_raw_parts
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::String;
    /// let s = String::from("hello");
    ///
    /// let (ptr, len, cap) = s.into_raw_parts();
    ///
    /// let rebuilt = unsafe { String::from_raw_parts(ptr, len, cap) };
    /// assert_eq!(rebuilt, "hello");
    /// ```
    #[must_use]
    pub fn into_raw_parts(self) -> (*mut u8, usize, usize) {
        self.vec.into_raw_parts()
    }

    /// Decomposes a `String` into its raw components.
    ///
    /// Returns the raw pointer to the underlying data, the length of
    /// the string (in bytes), and the allocated capacity of the data
    /// (in bytes). These are the same arguments in the same order as
    /// the arguments to [`from_raw_parts_in`].
    ///
    /// After calling this function, the caller is responsible for the
    /// memory previously managed by the `String`. The only way to do
    /// this is to convert the raw pointer, length, and capacity back
    /// into a `String` with the [`from_raw_parts_in`] function, allowing
    /// the destructor to perform the cleanup.
    ///
    /// [`from_raw_parts_in`]: String::from_raw_parts_in
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::String;
    /// let s = String::from("hello");
    ///
    /// let (ptr, len, cap, alloc) = s.into_raw_parts_with_alloc();
    ///
    /// let rebuilt = unsafe { String::from_raw_parts_in(ptr, len, cap, alloc) };
    /// assert_eq!(rebuilt, "hello");
    /// ```
    #[inline]
    pub fn into_raw_parts_with_alloc(self) -> (*mut u8, usize, usize, A) {
        self.vec.into_raw_parts_with_alloc()
    }

    /// Creates a new `String` from a length, capacity, and pointer.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * The memory at `buf` needs to have been previously allocated by `alloc`,
    /// with a required alignment of exactly 1.
    /// * `length` needs to be less than or equal to `capacity`.
    /// * `capacity` needs to be the correct value.
    /// * The first `length` bytes at `buf` need to be valid UTF-8.
    ///
    /// Violating these may cause problems like corrupting the allocator's
    /// internal data structures.
    ///
    /// The ownership of `buf` is effectively transferred to the
    /// `String` which may then deallocate, reallocate or change the
    /// contents of memory pointed to by the pointer at will. Ensure
    /// that nothing else uses the pointer after calling this
    /// function.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(allocator_api)]
    ///
    /// use std::alloc::Global;
    /// use fimo_ffi::String;
    /// use std::mem;
    ///
    /// unsafe {
    ///     let s = String::from("hello");
    ///
    ///     // Prevent automatically dropping the String's data
    ///     let mut s = mem::ManuallyDrop::new(s);
    ///
    ///     let ptr = s.as_mut_ptr();
    ///     let len = s.len();
    ///     let capacity = s.capacity();
    ///
    ///     let s = String::from_raw_parts_in(ptr, len, capacity, Global);
    ///
    ///     assert_eq!(String::from("hello"), s);
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn from_raw_parts_in(
        buf: *mut u8,
        length: usize,
        capacity: usize,
        alloc: A,
    ) -> String<A> {
        String {
            vec: Vec::from_raw_parts_in(buf, length, capacity, alloc),
        }
    }

    /// Converts a vector of bytes to a `String` without checking that the
    /// string contains valid UTF-8.
    ///
    /// See the safe version, [`from_utf8`], for more details.
    ///
    /// [`from_utf8`]: String::from_utf8
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not check that the bytes passed
    /// to it are valid UTF-8. If this constraint is violated, it may cause
    /// memory unsafety issues with future users of the `String`, as the rest of
    /// the standard library assumes that `String`s are valid UTF-8.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    ///
    /// // some bytes, in a vector
    /// let sparkle_heart = vec![240, 159, 146, 150];
    ///
    /// let sparkle_heart = unsafe {
    ///     String::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn from_utf8_unchecked(bytes: Vec<u8, A>) -> String<A> {
        String { vec: bytes }
    }

    /// Converts a `String` into a byte vector.
    ///
    /// This consumes the `String`, so we do not need to copy its contents.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    ///
    /// let s = String::from("hello");
    /// let bytes = s.into_bytes();
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111][..], &bytes[..]);
    /// ```
    #[inline]
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8, A> {
        self.vec
    }

    /// Extracts a string slice containing the entire `String`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let s = String::from("foo");
    ///
    /// assert_eq!("foo", s.as_str());
    /// ```
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self
    }

    /// Converts a `String` into a mutable string slice.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("foobar");
    /// let s_mut_str = s.as_mut_str();
    ///
    /// s_mut_str.make_ascii_uppercase();
    ///
    /// assert_eq!("FOOBAR", s_mut_str);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_mut_str(&mut self) -> &mut str {
        self
    }

    /// Appends a given string slice onto the end of this `String`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("foo");
    ///
    /// s.push_str("bar");
    ///
    /// assert_eq!("foobar", s);
    /// ```
    #[inline]
    pub fn push_str(&mut self, string: &str) {
        self.vec.extend_from_slice(string.as_bytes())
    }

    /// Copies elements from `src` range to the end of the string.
    ///
    /// ## Panics
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// ## Examples
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut string = String::from("abcde");
    ///
    /// string.extend_from_within(2..);
    /// assert_eq!(string, "abcdecde");
    ///
    /// string.extend_from_within(..2);
    /// assert_eq!(string, "abcdecdeab");
    ///
    /// string.extend_from_within(4..8);
    /// assert_eq!(string, "abcdecdeabecde");
    /// ```
    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, src: R) {
        let src @ Range { start, end } = std::slice::range(src, ..self.len());

        assert!(self.is_char_boundary(start));
        assert!(self.is_char_boundary(end));

        self.vec.extend_from_within(src);
    }
    /// Returns this `String`'s capacity, in bytes.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let s = String::with_capacity(10);
    ///
    /// assert!(s.capacity() >= 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// Ensures that this `String`'s capacity is at least `additional` bytes
    /// larger than its length.
    ///
    /// The capacity may be increased by more than `additional` bytes if it
    /// chooses, to prevent frequent reallocations.
    ///
    /// If you do not want this "at least" behavior, see the [`reserve_exact`]
    /// method.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows [`usize`].
    ///
    /// [`reserve_exact`]: String::reserve_exact
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::new();
    ///
    /// s.reserve(10);
    ///
    /// assert!(s.capacity() >= 10);
    /// ```
    ///
    /// This might not actually increase the capacity:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::with_capacity(10);
    /// s.push('a');
    /// s.push('b');
    ///
    /// // s now has a length of 2 and a capacity of 10
    /// assert_eq!(2, s.len());
    /// assert_eq!(10, s.capacity());
    ///
    /// // Since we already have an extra 8 capacity, calling this...
    /// s.reserve(8);
    ///
    /// // ... doesn't actually increase.
    /// assert_eq!(10, s.capacity());
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.vec.reserve(additional)
    }

    /// Ensures that this `String`'s capacity is `additional` bytes
    /// larger than its length.
    ///
    /// Consider using the [`reserve`] method unless you absolutely know
    /// better than the allocator.
    ///
    /// [`reserve`]: String::reserve
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::new();
    ///
    /// s.reserve_exact(10);
    ///
    /// assert!(s.capacity() >= 10);
    /// ```
    ///
    /// This might not actually increase the capacity:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::with_capacity(10);
    /// s.push('a');
    /// s.push('b');
    ///
    /// // s now has a length of 2 and a capacity of 10
    /// assert_eq!(2, s.len());
    /// assert_eq!(10, s.capacity());
    ///
    /// // Since we already have an extra 8 capacity, calling this...
    /// s.reserve_exact(8);
    ///
    /// // ... doesn't actually increase.
    /// assert_eq!(10, s.capacity());
    /// ```
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.vec.reserve_exact(additional)
    }

    /// Shrinks the capacity of this `String` to match its length.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("foo");
    ///
    /// s.reserve(100);
    /// assert!(s.capacity() >= 100);
    ///
    /// s.shrink_to_fit();
    /// assert_eq!(3, s.capacity());
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }
    /// Shrinks the capacity of this `String` with a lower bound.
    ///
    /// The capacity will remain at least as large as both the length
    /// and the supplied value.
    ///
    /// If the current capacity is less than the lower limit, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("foo");
    ///
    /// s.reserve(100);
    /// assert!(s.capacity() >= 100);
    ///
    /// s.shrink_to(10);
    /// assert!(s.capacity() >= 10);
    /// s.shrink_to(0);
    /// assert!(s.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.vec.shrink_to(min_capacity)
    }

    /// Appends the given [`char`] to the end of this `String`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("abc");
    ///
    /// s.push('1');
    /// s.push('2');
    /// s.push('3');
    ///
    /// assert_eq!("abc123", s);
    /// ```
    #[inline]
    pub fn push(&mut self, ch: char) {
        match ch.len_utf8() {
            1 => self.vec.push(ch as u8),
            _ => self
                .vec
                .extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes()),
        }
    }
    /// Returns a byte slice of this `String`'s contents.
    ///
    /// The inverse of this method is [`from_utf8`].
    ///
    /// [`from_utf8`]: String::from_utf8
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let s = String::from("hello");
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111], s.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.vec
    }
    /// Shortens this `String` to the specified length.
    ///
    /// If `new_len` is greater than the string's current length, this has no
    /// effect.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the string
    ///
    /// # Panics
    ///
    /// Panics if `new_len` does not lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("hello");
    ///
    /// s.truncate(2);
    ///
    /// assert_eq!("he", s);
    /// ```
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        if new_len <= self.len() {
            assert!(self.is_char_boundary(new_len));
            self.vec.truncate(new_len)
        }
    }
    /// Removes the last character from the string buffer and returns it.
    ///
    /// Returns [`None`] if this `String` is empty.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("foo");
    ///
    /// assert_eq!(s.pop(), Some('o'));
    /// assert_eq!(s.pop(), Some('o'));
    /// assert_eq!(s.pop(), Some('f'));
    ///
    /// assert_eq!(s.pop(), None);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        let ch = self.chars().rev().next()?;
        let new_len = self.len() - ch.len_utf8();
        unsafe {
            self.vec.set_len(new_len);
        }
        Some(ch)
    }

    /// Removes a [`char`] from this `String` at a byte position and returns it.
    ///
    /// This is an *O*(*n*) operation, as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than or equal to the `String`'s length,
    /// or if it does not lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("foo");
    ///
    /// assert_eq!(s.remove(0), 'f');
    /// assert_eq!(s.remove(1), 'o');
    /// assert_eq!(s.remove(0), 'o');
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        let ch = match self[idx..].chars().next() {
            Some(ch) => ch,
            None => panic!("cannot remove a char from the end of a string"),
        };

        let next = idx + ch.len_utf8();
        let len = self.len();
        unsafe {
            std::ptr::copy(
                self.vec.as_ptr().add(next),
                self.vec.as_mut_ptr().add(idx),
                len - next,
            );
            self.vec.set_len(len - (next - idx));
        }
        ch
    }

    /// Retains only the characters specified by the predicate.
    ///
    /// In other words, remove all characters `c` such that `f(c)` returns `false`.
    /// This method operates in place, visiting each character exactly once in the
    /// original order, and preserves the order of the retained characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("f_o_ob_ar");
    ///
    /// s.retain(|c| c != '_');
    ///
    /// assert_eq!(s, "foobar");
    /// ```
    ///
    /// Because the elements are visited exactly once in the original order,
    /// external state may be used to decide which elements to keep.
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("abcde");
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// s.retain(|_| *iter.next().unwrap());
    /// assert_eq!(s, "bce");
    /// ```
    #[inline]
    pub fn retain(&mut self, mut f: impl FnMut(char) -> bool) {
        struct SetLenOnDrop<'a, A: Allocator> {
            s: &'a mut String<A>,
            idx: usize,
            del_bytes: usize,
        }

        impl<'a, A: Allocator> Drop for SetLenOnDrop<'a, A> {
            fn drop(&mut self) {
                let new_len = self.idx - self.del_bytes;
                debug_assert!(new_len <= self.s.len());
                unsafe { self.s.vec.set_len(new_len) };
            }
        }

        let len = self.len();
        let mut guard = SetLenOnDrop {
            s: self,
            idx: 0,
            del_bytes: 0,
        };

        while guard.idx < len {
            let ch = unsafe {
                guard
                    .s
                    .get_unchecked(guard.idx..len)
                    .chars()
                    .next()
                    .unwrap()
            };
            let ch_len = ch.len_utf8();

            if !f(ch) {
                guard.del_bytes += ch_len;
            } else if guard.del_bytes > 0 {
                unsafe {
                    std::ptr::copy(
                        guard.s.vec.as_ptr().add(guard.idx),
                        guard.s.vec.as_mut_ptr().add(guard.idx - guard.del_bytes),
                        ch_len,
                    );
                }
            }

            // Point idx to the next char
            guard.idx += ch_len;
        }

        drop(guard);
    }

    /// Inserts a character into this `String` at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the `String`'s length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::with_capacity(3);
    ///
    /// s.insert(0, 'f');
    /// s.insert(1, 'o');
    /// s.insert(2, 'o');
    ///
    /// assert_eq!("foo", s);
    /// ```
    #[inline]
    pub fn insert(&mut self, idx: usize, ch: char) {
        assert!(self.is_char_boundary(idx));
        let mut bits = [0; 4];
        let bits = ch.encode_utf8(&mut bits).as_bytes();

        unsafe {
            self.insert_bytes(idx, bits);
        }
    }

    /// Inserts a string slice into this `String` at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the `String`'s length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("bar");
    ///
    /// s.insert_str(0, "foo");
    ///
    /// assert_eq!("foobar", s);
    /// ```
    #[inline]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        assert!(self.is_char_boundary(idx));

        unsafe {
            self.insert_bytes(idx, string.as_bytes());
        }
    }

    unsafe fn insert_bytes(&mut self, idx: usize, bytes: &[u8]) {
        let len = self.len();
        let amt = bytes.len();
        self.vec.reserve(amt);

        std::ptr::copy(
            self.vec.as_ptr().add(idx),
            self.vec.as_mut_ptr().add(idx + amt),
            len - idx,
        );
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.vec.as_mut_ptr().add(idx), amt);
        self.vec.set_len(len + amt);
    }

    /// Returns a mutable reference to the contents of this `String`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut Vec` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `String` after dropping the `&mut Vec` may violate memory
    /// safety, as the rest of the standard library assumes that `String`s are
    /// valid UTF-8.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("hello");
    ///
    /// unsafe {
    ///     let vec = s.as_mut_vec();
    ///     assert_eq!(&[104, 101, 108, 108, 111][..], &vec[..]);
    ///
    ///     vec.reverse();
    /// }
    /// assert_eq!(s, "olleh");
    /// ```
    #[inline]
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<u8, A> {
        &mut self.vec
    }

    /// Returns the length of this `String`, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let a = String::from("foo");
    /// assert_eq!(a.len(), 3);
    ///
    /// let fancy_f = String::from("ƒoo");
    /// assert_eq!(fancy_f.len(), 4);
    /// assert_eq!(fancy_f.chars().count(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns `true` if this `String` has a length of zero, and `false` otherwise.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut v = String::new();
    /// assert!(v.is_empty());
    ///
    /// v.push('a');
    /// assert!(!v.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Splits the string into two at the given byte index.
    ///
    /// Returns a newly allocated `String`. `self` contains bytes `[0, at)`, and
    /// the returned `String` contains bytes `[at, len)`. `at` must be on the
    /// boundary of a UTF-8 code point.
    ///
    /// Note that the capacity of `self` does not change.
    ///
    /// # Panics
    ///
    /// Panics if `at` is not on a `UTF-8` code point boundary, or if it is beyond the last
    /// code point of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// # fn main() {
    /// let mut hello = String::from("Hello, World!");
    /// let world = hello.split_off(7);
    /// assert_eq!(hello, "Hello, ");
    /// assert_eq!(world, "World!");
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn split_off(&mut self, at: usize) -> String<A>
    where
        A: Clone,
    {
        assert!(self.is_char_boundary(at));
        let other = self.vec.split_off(at);
        unsafe { String::from_utf8_unchecked(other) }
    }

    /// Truncates this `String`, removing all contents.
    ///
    /// While this means the `String` will have a length of zero, it does not
    /// touch its capacity.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("foo");
    ///
    /// s.clear();
    ///
    /// assert!(s.is_empty());
    /// assert_eq!(0, s.len());
    /// assert_eq!(3, s.capacity());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear()
    }

    /// Creates a draining iterator that removes the specified range in the `String`
    /// and yields the removed `chars`.
    ///
    /// Note: The element range is removed even if the iterator is not
    /// consumed until the end.
    ///
    /// # Panics
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::String;
    ///
    /// let mut s = String::from("α is alpha, β is beta");
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Remove the range up until the β from the string
    /// let t: String = s.drain(..beta_offset).collect();
    /// assert_eq!(t, "α is alpha, ");
    /// assert_eq!(s, "β is beta");
    ///
    /// // A full range clears the string
    /// s.drain(..);
    /// assert_eq!(s, "");
    /// ```
    #[inline]
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, A> {
        // Memory safety
        //
        // The String version of Drain does not have the memory safety issues
        // of the vector version. The data is just plain bytes.
        // Because the range removal happens in Drop, if the Drain iterator is leaked,
        // the removal will not happen.
        let Range { start, end } = std::slice::range(range, ..self.len());
        assert!(self.is_char_boundary(start));
        assert!(self.is_char_boundary(end));

        // Take out two simultaneous borrows. The &mut String won't be accessed
        // until iteration is over, in Drop.
        let self_ptr = self as *mut _;
        // SAFETY: `slice::range` and `is_char_boundary` do the appropriate bounds checks.
        let chars_iter = unsafe { self.get_unchecked(start..end) }.chars();

        Drain {
            start,
            end,
            iter: chars_iter,
            string: self_ptr,
        }
    }
}

impl<A: Allocator> Add<&'_ str> for String<A> {
    type Output = String<A>;

    #[inline]
    fn add(mut self, rhs: &'_ str) -> Self::Output {
        self.push_str(rhs);
        self
    }
}

impl<A: Allocator> AddAssign<&'_ str> for String<A> {
    #[inline]
    fn add_assign(&mut self, rhs: &'_ str) {
        self.push_str(rhs)
    }
}

impl<A: Allocator> AsRef<str> for String<A> {
    fn as_ref(&self) -> &str {
        self
    }
}

impl<A: Allocator> AsMut<str> for String<A> {
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self
    }
}

impl<A: Allocator> AsRef<[u8]> for String<A> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<A: Allocator> Borrow<str> for String<A> {
    #[inline]
    fn borrow(&self) -> &str {
        self
    }
}

impl<A: Allocator> BorrowMut<str> for String<A> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        self
    }
}

impl<A: Allocator + Clone> Clone for String<A> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            vec: self.vec.clone(),
        }
    }
}

impl<A: Allocator> Debug for String<A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl Default for String {
    #[inline]
    fn default() -> Self {
        String::new()
    }
}

impl<A: Allocator> Deref for String<A> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { std::str::from_utf8_unchecked(&self.vec) }
    }
}

impl<A: Allocator> DerefMut for String<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::str::from_utf8_unchecked_mut(&mut self.vec) }
    }
}

impl<A: Allocator> Display for String<A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl<'a, A: Allocator> Extend<&'a char> for String<A> {
    #[inline]
    fn extend<T: IntoIterator<Item = &'a char>>(&mut self, iter: T) {
        self.extend(iter.into_iter().cloned())
    }
}

impl<'a, A: Allocator> Extend<&'a str> for String<A> {
    #[inline]
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(s));
    }
}

impl<A: Allocator, B: Allocator> Extend<Box<str, B>> for String<A> {
    #[inline]
    fn extend<T: IntoIterator<Item = Box<str, B>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl<A: Allocator, B: Allocator> Extend<String<B>> for String<A> {
    #[inline]
    fn extend<T: IntoIterator<Item = String<B>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl<A: Allocator> Extend<char> for String<A> {
    #[inline]
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        let iterator = iter.into_iter();
        let (lower_bound, _) = iterator.size_hint();
        self.reserve(lower_bound);
        iterator.for_each(move |c| self.push(c));
    }
}

impl<A: Allocator + Clone> From<&'_ String<A>> for String<A> {
    #[inline]
    fn from(s: &'_ String<A>) -> Self {
        s.clone()
    }
}

impl From<&'_ str> for String {
    #[inline]
    fn from(s: &'_ str) -> Self {
        let mut str = String::with_capacity(s.len());
        str.push_str(s);
        str
    }
}

impl From<&'_ mut str> for String {
    #[inline]
    fn from(s: &'_ mut str) -> Self {
        String::from(s as &str)
    }
}

impl From<char> for String {
    #[inline]
    fn from(ch: char) -> Self {
        let mut s = String::new();
        s.push(ch);
        s
    }
}

impl<A: Allocator> From<Box<str, A>> for String<A> {
    #[inline]
    fn from(b: Box<str, A>) -> Self {
        let len = b.len();
        let (ptr, alloc) = Box::into_raw_with_allocator(b);
        unsafe { String::from_raw_parts_in(ptr as *mut u8, len, len, alloc) }
    }
}

impl From<&Version> for String {
    fn from(version: &Version) -> Self {
        let req = version.string_length_full();
        let mut buff = Vec::with_capacity(req);
        // Safety:
        let mut str = unsafe {
            buff.set_len(req);
            String::from_utf8_unchecked(buff)
        };

        version.as_string_full(&mut str).unwrap();

        str
    }
}

impl From<Version> for String {
    fn from(version: Version) -> Self {
        From::from(&version)
    }
}

impl From<String> for Box<str> {
    #[inline]
    fn from(s: String) -> Self {
        s.into_boxed_str()
    }
}

impl<A: Allocator> From<String<A>> for Vec<u8, A> {
    #[inline]
    fn from(s: String<A>) -> Self {
        s.vec
    }
}

impl<A: Allocator + Send + Sync + 'static> From<String<A>> for Box<dyn Error + Send + Sync> {
    fn from(err: String<A>) -> Self {
        struct StringError<A: Allocator>(String<A>);

        impl<A: Allocator> Error for StringError<A> {
            #[allow(deprecated)]
            fn description(&self) -> &str {
                &self.0
            }
        }

        impl<A: Allocator> Display for StringError<A> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.0, f)
            }
        }

        // Purposefully skip printing "StringError(..)"
        impl<A: Allocator> Debug for StringError<A> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Debug::fmt(&self.0, f)
            }
        }

        Box::new(StringError(err))
    }
}

impl From<String> for Box<dyn Error> {
    #[inline]
    fn from(str_err: String) -> Self {
        let err1: Box<dyn Error + Send + Sync> = From::from(str_err);
        let err2: Box<dyn Error> = err1;
        err2
    }
}

impl FromIterator<char> for String {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a char> for String {
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<&'a str> for String {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<Box<str>> for String {
    fn from_iter<T: IntoIterator<Item = Box<str>>>(iter: T) -> Self {
        let mut buf = String::new();
        buf.extend(iter);
        buf
    }
}

impl<A: Allocator + Default> FromIterator<String<A>> for String<A> {
    fn from_iter<T: IntoIterator<Item = String<A>>>(iter: T) -> Self {
        let mut iterator = iter.into_iter();

        // Because we're iterating over `String`s, we can avoid at least
        // one allocation by getting the first string from the iterator
        // and appending to it all the subsequent strings.
        match iterator.next() {
            None => String::new_in(Default::default()),
            Some(mut buf) => {
                buf.extend(iterator);
                buf
            }
        }
    }
}

impl FromStr for String {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(String::from(s))
    }
}

impl<A: Allocator> Hash for String<A> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<A: Allocator> Index<Range<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl<A: Allocator> Index<RangeFrom<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl<A: Allocator> Index<RangeFull> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, _index: RangeFull) -> &Self::Output {
        unsafe { std::str::from_utf8_unchecked(&self.vec) }
    }
}

impl<A: Allocator> Index<RangeInclusive<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<A: Allocator> Index<RangeTo<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self[..][index]
    }
}

impl<A: Allocator> Index<RangeToInclusive<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: RangeToInclusive<usize>) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<A: Allocator> IndexMut<Range<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl<A: Allocator> IndexMut<RangeFrom<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl<A: Allocator> IndexMut<RangeFull> for String<A> {
    #[inline]
    fn index_mut(&mut self, _index: RangeFull) -> &mut Self::Output {
        unsafe { std::str::from_utf8_unchecked_mut(&mut self.vec) }
    }
}

impl<A: Allocator> IndexMut<RangeInclusive<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: RangeInclusive<usize>) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<A: Allocator> IndexMut<RangeTo<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        &mut self[..][index]
    }
}

impl<A: Allocator> IndexMut<RangeToInclusive<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: RangeToInclusive<usize>) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<A: Allocator> PartialEq for String<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self[..], &other[..])
    }
}

macro_rules! impl_eq {
    ($lhs:ty, $rhs: ty) => {
        #[allow(unused_lifetimes)]
        impl<'a, 'b, A: Allocator> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }

        #[allow(unused_lifetimes)]
        impl<'a, 'b, A: Allocator> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }
    };
}

impl_eq! { String<A>, str }
impl_eq! { String<A>, &'a str }

impl<A: Allocator> Eq for String<A> {}

impl<A: Allocator> PartialOrd for String<A> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&self.vec, &other.vec)
    }
}

impl<A: Allocator> Ord for String<A> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&self.vec, &other.vec)
    }
}

impl<A: Allocator> ToSocketAddrs for String<A> {
    type Iter = std::vec::IntoIter<SocketAddr>;

    #[inline]
    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        (**self).to_socket_addrs()
    }
}

impl<A: Allocator> Write for String<A> {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.push_str(s);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.push(c);
        Ok(())
    }
}

impl<A: Allocator> Serialize for String<A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self)
    }
}

struct StringVisitor<A: Allocator + Default>(PhantomData<fn() -> A>);
struct StringInPlaceVisitor<'a, A: Allocator + Default>(&'a mut String<A>);

impl<'de, A: Allocator + Default> Visitor<'de> for StringVisitor<A> {
    type Value = String<A>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut s = String::new_in(Default::default());
        s.push_str(v);
        Ok(s)
    }

    fn visit_string<E>(self, v: std::string::String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut s = String::new_in(Default::default());
        s.push_str(v.as_str());
        Ok(s)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match std::str::from_utf8(v) {
            Ok(s) => {
                let mut buf = String::new_in(Default::default());
                buf.push_str(s);
                Ok(buf)
            }
            Err(_) => Err(E::invalid_value(serde::de::Unexpected::Bytes(v), &self)),
        }
    }

    fn visit_byte_buf<E>(self, v: std::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match std::str::from_utf8(v.as_slice()) {
            Ok(s) => {
                let mut buf = String::new_in(Default::default());
                buf.push_str(s);
                Ok(buf)
            }
            Err(_) => Err(E::invalid_value(
                serde::de::Unexpected::Bytes(v.as_slice()),
                &self,
            )),
        }
    }
}

impl<'a, 'de, A: Allocator + Default> Visitor<'de> for StringInPlaceVisitor<'a, A> {
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.0.clear();
        self.0.push_str(v);
        Ok(())
    }

    fn visit_string<E>(self, v: std::string::String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.0.clear();
        self.0.push_str(v.as_str());
        Ok(())
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match std::str::from_utf8(v) {
            Ok(s) => {
                self.0.clear();
                self.0.push_str(s);
                Ok(())
            }
            Err(_) => Err(E::invalid_value(serde::de::Unexpected::Bytes(v), &self)),
        }
    }

    fn visit_byte_buf<E>(self, v: std::vec::Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match std::str::from_utf8(v.as_slice()) {
            Ok(s) => {
                self.0.clear();
                self.0.push_str(s);
                Ok(())
            }
            Err(_) => Err(E::invalid_value(
                serde::de::Unexpected::Bytes(v.as_slice()),
                &self,
            )),
        }
    }
}

impl<'de, A: Allocator + Default> Deserialize<'de> for String<A> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(StringVisitor(PhantomData))
    }

    fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(StringInPlaceVisitor(place))
    }
}

/// A possible error value when converting a `String` from a UTF-8 byte vector.
///
/// This type is the error type for the [`from_utf8`] method on [`String`]. It
/// is designed in such a way to carefully avoid reallocations: the
/// [`into_bytes`] method will give back the byte vector that was used in the
/// conversion attempt.
///
/// [`from_utf8`]: String::from_utf8
/// [`into_bytes`]: FromUtf8Error::into_bytes
///
/// The [`Utf8Error`] type provided by [`std::str`] represents an error that may
/// occur when converting a slice of [`u8`]s to a [`&str`]. In this sense, it's
/// an analogue to `FromUtf8Error`, and you can get one from a `FromUtf8Error`
/// through the [`utf8_error`] method.
///
/// [`Utf8Error`]: std::str::Utf8Error "std::str::Utf8Error"
/// [`&str`]: prim@str "&str"
/// [`utf8_error`]: FromUtf8Error::utf8_error
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use fimo_ffi::{String, vec};
/// // some invalid bytes, in a vector
/// let bytes = vec![0, 159];
///
/// let value = String::from_utf8(bytes);
///
/// assert!(value.is_err());
/// assert_eq!(vec![0, 159], value.unwrap_err().into_bytes());
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct FromUtf8Error<A: Allocator = Global> {
    bytes: Vec<u8, A>,
    error: Utf8Error,
}

impl<A: Allocator> FromUtf8Error<A> {
    /// Returns a slice of [`u8`]s bytes that were attempted to convert to a `String`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    /// // some invalid bytes, in a vector
    /// let bytes = vec![0, 159];
    ///
    /// let value = String::from_utf8(bytes);
    ///
    /// assert_eq!(&[0, 159], value.unwrap_err().as_bytes());
    /// ```
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the bytes that were attempted to convert to a `String`.
    ///
    /// This method is carefully constructed to avoid allocation. It will
    /// consume the error, moving out the bytes, so that a copy of the bytes
    /// does not need to be made.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    /// // some invalid bytes, in a vector
    /// let bytes = vec![0, 159];
    ///
    /// let value = String::from_utf8(bytes);
    ///
    /// assert_eq!(vec![0, 159], value.unwrap_err().into_bytes());
    /// ```
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8, A> {
        self.bytes
    }

    /// Fetch a `Utf8Error` to get more details about the conversion failure.
    ///
    /// The [`Utf8Error`] type provided by [`std::str`] represents an error that may
    /// occur when converting a slice of [`u8`]s to a [`&str`]. In this sense, it's
    /// an analogue to `FromUtf8Error`. See its documentation for more details
    /// on using it.
    ///
    /// [`std::str`]: core::str "std::str"
    /// [`&str`]: prim@str "&str"
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use fimo_ffi::{String, vec};
    /// // some invalid bytes, in a vector
    /// let bytes = vec![0, 159];
    ///
    /// let error = String::from_utf8(bytes).unwrap_err().utf8_error();
    ///
    /// // the first byte is invalid here
    /// assert_eq!(1, error.valid_up_to());
    /// ```
    #[must_use]
    pub fn utf8_error(&self) -> Utf8Error {
        self.error
    }
}

impl<A: Allocator> Debug for FromUtf8Error<A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromUtf8Error")
            .field("bytes", &self.bytes)
            .field("error", &self.error)
            .finish()
    }
}

impl<A: Allocator> Display for FromUtf8Error<A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl<A: Allocator> Error for FromUtf8Error<A> {}

/// A possible error value when converting a `String` from a UTF-16 byte slice.
///
/// This type is the error type for the [`from_utf16`] method on [`String`].
///
/// [`from_utf16`]: String::from_utf16
/// # Examples
///
/// Basic usage:
///
/// ```
/// use fimo_ffi::String;
/// // 𝄞mu<invalid>ic
/// let v = &[0xD834, 0xDD1E, 0x006d, 0x0075,
///           0xD800, 0x0069, 0x0063];
///
/// assert!(String::from_utf16(v).is_err());
/// ```
#[derive(Debug)]
pub struct FromUtf16Error(());

impl Display for FromUtf16Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt("invalid utf-16: lone surrogate found", f)
    }
}

/// A draining iterator for `String`.
///
/// This struct is created by the [`drain`] method on [`String`]. See its
/// documentation for more.
///
/// [`drain`]: String::drain
pub struct Drain<'a, A: Allocator = Global> {
    string: *mut String<A>,
    start: usize,
    end: usize,
    iter: Chars<'a>,
}

impl<'a, A: Allocator> Drain<'a, A> {
    /// Returns the remaining (sub)string of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::String;
    /// let mut s = String::from("abc");
    /// let mut drain = s.drain(..);
    /// assert_eq!(drain.as_str(), "abc");
    /// let _ = drain.next().unwrap();
    /// assert_eq!(drain.as_str(), "bc");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.iter.as_str()
    }
}

unsafe impl<A: Allocator + Send> Send for Drain<'_, A> {}

unsafe impl<A: Allocator + Sync> Sync for Drain<'_, A> {}

impl<'a, A: Allocator> AsRef<[u8]> for Drain<'a, A> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl<'a, A: Allocator> AsRef<str> for Drain<'a, A> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<A: Allocator> Debug for Drain<'_, A> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Drain").field(&self.as_str()).finish()
    }
}

impl<A: Allocator> Iterator for Drain<'_, A> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn last(mut self) -> Option<char> {
        self.next_back()
    }
}

impl<A: Allocator> DoubleEndedIterator for Drain<'_, A> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<A: Allocator> FusedIterator for Drain<'_, A> {}

impl<A: Allocator> Drop for Drain<'_, A> {
    fn drop(&mut self) {
        unsafe {
            // Use Vec::drain. "Reaffirm" the bounds checks to avoid
            // panic code being inserted again.
            let self_vec = (*self.string).as_mut_vec();
            if self.start <= self.end && self.end <= self_vec.len() {
                self_vec.drain(self.start..self.end);
            }
        }
    }
}
