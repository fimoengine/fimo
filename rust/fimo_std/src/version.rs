//! Implementation of versioning facilities.

use crate::{
    error::{AnyError, AnyResult},
    modules::symbols::SliceRef,
    utils::ConstNonNull,
};
use core::{fmt::Display, panic};
use std::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

unsafe extern "C" {
    fn fimo_version_parse_str(
        str: ConstNonNull<u8>,
        len: usize,
        out: &mut MaybeUninit<Version<'static>>,
    ) -> AnyResult;
    fn fimo_version_str_len(version: &Version<'_>) -> usize;
    fn fimo_version_str_len_full(version: &Version<'_>) -> usize;
    fn fimo_version_write_str(
        version: &Version<'_>,
        str: NonNull<u8>,
        len: usize,
        written: Option<&mut MaybeUninit<usize>>,
    ) -> AnyResult;
    fn fimo_version_write_str_full(
        version: &Version<'_>,
        str: NonNull<u8>,
        len: usize,
        written: Option<&mut MaybeUninit<usize>>,
    ) -> AnyResult;
    fn fimo_version_cmp(lhs: &Version<'_>, rhs: &Version<'_>) -> std::ffi::c_int;
    fn fimo_version_compatible(got: &Version<'_>, required: &Version<'_>) -> bool;
}

pub use fimo_std_macros::version;

/// A version specifier.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Version<'a> {
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
    pub pre: SliceRef<'a, u8>,
    pub build: SliceRef<'a, u8>,
    _private: PhantomData<()>,
}

impl<'a> Version<'a> {
    /// Constructs a new `Version`.
    pub const fn new(major: usize, minor: usize, patch: usize) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: SliceRef::new(&[]),
            build: SliceRef::new(&[]),
            _private: PhantomData,
        }
    }

    /// Constructs a new `Version`.
    pub const fn new_full(
        major: usize,
        minor: usize,
        patch: usize,
        pre: Option<&'a str>,
        build: Option<&'a str>,
    ) -> Self {
        let pre = match pre {
            Some(s) => SliceRef::new(s.as_bytes()),
            None => SliceRef::new(&[]),
        };
        let build = match build {
            Some(s) => SliceRef::new(s.as_bytes()),
            None => SliceRef::new(&[]),
        };

        Self {
            major,
            minor,
            patch,
            pre,
            build,
            _private: PhantomData,
        }
    }

    #[doc(hidden)]
    pub const fn __private_new(
        major: u64,
        minor: u64,
        patch: u64,
        pre: Option<&'a str>,
        build: Option<&'a str>,
    ) -> Self {
        if size_of::<usize>() < size_of::<u64>()
            && (major > usize::MAX as u64 || minor > usize::MAX as u64 || patch > usize::MAX as u64)
        {
            panic!("overflow");
        }

        let major = major as _;
        let minor = minor as _;
        let patch = patch as _;
        let pre = match pre {
            Some(s) => SliceRef::new(s.as_bytes()),
            None => SliceRef::new(&[]),
        };
        let build = match build {
            Some(s) => SliceRef::new(s.as_bytes()),
            None => SliceRef::new(&[]),
        };

        Self {
            major,
            minor,
            patch,
            pre,
            build,
            _private: PhantomData,
        }
    }

    /// Returns the length required to format the `Version`.
    ///
    /// Returns the minimum required buffer length to format the `Version` using
    /// [`Self::write_str`].
    pub fn str_len(&self) -> usize {
        unsafe { fimo_version_str_len(self) }
    }

    /// Returns the length required to format the `Version`.
    ///
    /// Returns the minimum required buffer length to format the `Version` using
    /// [`Self::write_str_full`].
    pub fn str_len_full(&self) -> usize {
        unsafe { fimo_version_str_len_full(self) }
    }

    /// Formats the `Version` into a buffer.
    ///
    /// Formats a string representation of the `Version`, containing its major-, minor-, and patch
    /// numbers into the provided buffer. Use [`Self::str_len`] to query the minimum buffer size
    /// required by this function.
    pub fn write_str<'b>(&self, buff: &'b mut [u8]) -> Result<&'b mut str, AnyError> {
        let mut written = MaybeUninit::uninit();
        unsafe {
            fimo_version_write_str(
                self,
                NonNull::new_unchecked(buff.as_mut_ptr()),
                buff.len(),
                Some(&mut written),
            )
            .into_result()?;

            let written = written.assume_init();
            let (str_buf, _) = buff.split_at_mut(written);
            Ok(core::str::from_utf8_unchecked_mut(str_buf))
        }
    }

    /// Formats the `Version` into a buffer.
    ///
    /// Formats a string representation of the `Version` into the provided buffer. Use
    /// [`Self::str_len_full`] to query the minimum buffer size required by this function.
    pub fn write_str_full<'b>(&self, buff: &'b mut [u8]) -> Result<&'b mut str, AnyError> {
        let mut written = MaybeUninit::uninit();
        unsafe {
            fimo_version_write_str_full(
                self,
                NonNull::new_unchecked(buff.as_mut_ptr()),
                buff.len(),
                Some(&mut written),
            )
            .into_result()?;

            let written = written.assume_init();
            let (str_buf, _) = buff.split_at_mut(written);
            Ok(core::str::from_utf8_unchecked_mut(str_buf))
        }
    }

    /// Checks for the compatibility of two `Version`s.
    ///
    /// If the `Version` is compatible with `required`, it indicates that an object versioned with
    /// `self` can be used where one would expect an object of the same type, versioned with
    /// `required`.
    ///
    /// The compatibility is determined by the following algorithm:
    ///
    /// 1. The major numbers of `self` and `required` must be equal.
    /// 2. If the major number is `0`, then the minor numbers must be equal.
    /// 3. `self >= required`.
    pub fn compatible(&self, required: &Self) -> bool {
        unsafe { fimo_version_compatible(self, required) }
    }
}

impl PartialEq for Version<'_> {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), core::cmp::Ordering::Equal)
    }
}

impl Eq for Version<'_> {}

impl PartialOrd for Version<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let cmp = unsafe { fimo_version_cmp(self, other) };
        match cmp {
            -1 => core::cmp::Ordering::Less,
            0 => core::cmp::Ordering::Equal,
            1 => core::cmp::Ordering::Greater,
            _ => unreachable!(),
        }
    }
}

impl Display for Version<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut buff = [0; 256];
        let formatted = self
            .write_str_full(&mut buff)
            .expect("version string should fit into the string");
        write!(f, "{formatted}")
    }
}

impl<'a> TryFrom<&'a str> for Version<'a> {
    type Error = AnyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut out = MaybeUninit::uninit();
        unsafe {
            fimo_version_parse_str(
                ConstNonNull::new_unchecked(value.as_ptr()),
                value.len(),
                &mut out,
            )
            .into_result()?;
            Ok(out.assume_init())
        }
    }
}
