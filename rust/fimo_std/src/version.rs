//! Implementation of versioning facilities.

use core::fmt::Display;

use crate::{
    bindings,
    error::{to_result, to_result_indirect_in_place, Error},
    ffi::FFITransferable,
};

/// Constructs a new [`Version`].
#[macro_export]
macro_rules! version {
    ($major:literal, $minor:literal, $patch:literal $(,)?) => {{
        $crate::version::Version::new($major, $minor, $patch)
    }};
    ($major:literal, $minor:literal, $patch:literal, $build:literal$(,)?) => {{
        $crate::version::Version::new_long($major, $minor, $patch, $build)
    }};
    ($version:literal) => {{
        $crate::version::Version::try_from($version)
    }};
}

/// A version specifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Version(pub(crate) bindings::FimoVersion);

impl Version {
    /// Maximum length of a formatted `Version` string.
    pub const MAX_STR_LENGTH: usize = bindings::FIMO_VERSION_MAX_STR_LENGTH as usize;

    /// Maximum length of a fully formatted `Version` string.
    pub const MAX_LONG_STR_LENGTH: usize = bindings::FIMO_VERSION_LONG_MAX_STR_LENGTH as usize;

    /// Constructs a new `Version`.
    ///
    /// The `build` number is set to `0`.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self(bindings::FimoVersion {
            major,
            minor,
            patch,
            build: 0,
        })
    }

    /// Constructs a new `Version`.
    pub const fn new_long(major: u32, minor: u32, patch: u32, build: u64) -> Self {
        Self(bindings::FimoVersion {
            major,
            minor,
            patch,
            build,
        })
    }

    /// Returns the length required to format the `Version`.
    ///
    /// Returns the minimum required buffer length to format
    /// the `Version` using [`Self::write_str`].
    pub fn str_len(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_version_str_len(&self.0) }
    }

    /// Returns the length required to format the `Version`.
    ///
    /// Returns the minimum required buffer length to format
    /// the `Version` using [`Self::write_str_long`].
    pub fn str_len_long(&self) -> usize {
        // Safety: The pointer is valid.
        unsafe { bindings::fimo_version_str_len_full(&self.0) }
    }

    /// Formats the `Version` into a buffer.
    ///
    /// Formats a string representation of the `Version`,
    /// containing its major-, minor-, and patch numbers
    /// into the provided buffer. Use [`Self::str_len`]
    /// to query the minimum buffer size required by this
    /// function. A size of at least [`Self::MAX_STR_LENGTH`]
    /// is guaranteed to work, regardless of the contents
    /// of the `Version`.
    pub fn write_str<'a>(&self, buff: &'a mut [u8]) -> Result<&'a mut str, Error> {
        let mut written = 0usize;
        // Safety: The pointers are valid.
        let error = unsafe {
            bindings::fimo_version_write_str(
                &self.0,
                buff.as_mut_ptr().cast(),
                buff.len(),
                &mut written,
            )
        };
        // Safety:
        unsafe {
            to_result(error)?;
        }

        let (str_buf, _) = buff.split_at_mut(written);

        // Safety: The formatting function guarantees that the result is
        // valid UTF-8.
        unsafe { Ok(core::str::from_utf8_unchecked_mut(str_buf)) }
    }

    /// Formats the `Version` into a buffer.
    ///
    /// Formats a string representation of the `Version`,
    /// containing its major-, minor-, patch- and build numbers
    /// into the provided buffer. Use [`Self::str_len_long`]
    /// to query the minimum buffer size required by this
    /// function. A size of at least [`Self::MAX_LONG_STR_LENGTH`]
    /// is guaranteed to work, regardless of the contents
    /// of the `Version`.
    pub fn write_str_long<'a>(&self, buff: &'a mut [u8]) -> Result<&'a mut str, Error> {
        let mut written = 0usize;
        // Safety: The pointers are valid.
        let error = unsafe {
            bindings::fimo_version_write_str_long(
                &self.0,
                buff.as_mut_ptr().cast(),
                buff.len(),
                &mut written,
            )
        };
        // Safety:
        unsafe {
            to_result(error)?;
        }

        let (str_buf, _) = buff.split_at_mut(written);

        // Safety: The formatting function guarantees that the result is
        // valid UTF-8.
        unsafe { Ok(core::str::from_utf8_unchecked_mut(str_buf)) }
    }

    /// Compares two `Versions`.
    ///
    /// Works like the implementation of [`Ord`], but also
    /// considers the build numbers of the two `Version`s.
    pub fn cmp_long(&self, other: &Self) -> core::cmp::Ordering {
        // Safety: The pointers are valid.
        let cmp = unsafe { bindings::fimo_version_cmp_long(&self.0, &other.0) };
        match cmp {
            -1 => core::cmp::Ordering::Less,
            0 => core::cmp::Ordering::Equal,
            1 => core::cmp::Ordering::Greater,
            _ => unreachable!(),
        }
    }

    /// Checks for the compatibility of two `Version`s.
    ///
    /// If the `Version` is compatible with `required`, it
    /// indicates that an object versioned with `self` can
    /// be used where one would expect an object of the same
    /// type, versioned with `required`.
    ///
    /// The compatibility is determined by the following algorithm:
    ///
    /// 1. The major numbers of `self` and `required` must be equal.
    /// 2. If the major number is `0`, then the minor numbers must be equal.
    /// 3. `self >= required` without the build number.
    pub fn compatible(&self, required: &Self) -> bool {
        // Safety: The pointers are valid.
        unsafe { bindings::fimo_version_compatible(&self.0, &required.0) }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp_long(other), core::cmp::Ordering::Equal)
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Safety: The pointers are valid.
        let cmp = unsafe { bindings::fimo_version_cmp(&self.0, &other.0) };
        match cmp {
            -1 => core::cmp::Ordering::Less,
            0 => core::cmp::Ordering::Equal,
            1 => core::cmp::Ordering::Greater,
            _ => unreachable!(),
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut buff = [0; Self::MAX_LONG_STR_LENGTH];
        let formatted = self
            .write_str_long(&mut buff)
            .expect("version string should fit into the string");
        write!(f, "{formatted}")
    }
}

impl TryFrom<&str> for Version {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Safety: The value is initialized when there is no error.
        let version = unsafe {
            to_result_indirect_in_place(|err, ver| {
                *err = bindings::fimo_version_parse_str(
                    value.as_ptr().cast(),
                    value.len(),
                    ver.as_mut_ptr(),
                );
            })
        }?;
        Ok(Self(version))
    }
}

impl FFITransferable<bindings::FimoVersion> for Version {
    fn into_ffi(self) -> bindings::FimoVersion {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoVersion) -> Self {
        Self(ffi)
    }
}
