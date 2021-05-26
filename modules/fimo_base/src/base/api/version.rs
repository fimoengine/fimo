use crate::base::{DataGuard, Locked, Unlocked};
use emf_core_base_rs::ffi::version::{ReleaseType, Version};
use emf_core_base_rs::ownership::Owned;
use emf_core_base_rs::Error;
use fimo_version_rs::{
    as_string_full, as_string_long, as_string_short, compare, compare_strong, compare_weak,
    from_string, is_compatible, new_full, new_long, new_short, string_is_valid, string_length_full,
    string_length_long, string_length_short,
};
use std::cmp::Ordering;
use std::marker::PhantomData;

/// Implementation of the version api.
#[derive(Debug)]
pub struct VersionAPI<'i> {
    phantom: PhantomData<fn() -> &'i ()>,
}

impl Default for VersionAPI<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionAPI<'_> {
    /// Constructs a new instance.
    #[inline]
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    /// Constructs a new version.
    ///
    /// Constructs a new version with `major`, `minor` and `patch` and sets the rest to `0`.
    #[inline]
    pub const fn new_short(&self, major: i32, minor: i32, patch: i32) -> Version {
        new_short(major, minor, patch)
    }

    /// Constructs a new version.
    ///
    /// Constructs a new version with `major`, `minor`, `patch`, `release_type` and
    /// `release_number` and sets the rest to `0`.
    #[inline]
    pub const fn new_long(
        &self,
        major: i32,
        minor: i32,
        patch: i32,
        release_type: ReleaseType,
        release_number: i8,
    ) -> Version {
        new_long(major, minor, patch, release_type, release_number)
    }

    /// Constructs a new version.
    ///
    /// Constructs a new version with `major`, `minor`, `patch`, `release_type`,
    /// `release_number` and `build`.
    #[inline]
    pub const fn new_full(
        &self,
        major: i32,
        minor: i32,
        patch: i32,
        release_type: ReleaseType,
        release_number: i8,
        build: i64,
    ) -> Version {
        new_full(major, minor, patch, release_type, release_number, build)
    }

    /// Constructs a version from a string.
    ///
    /// # Failure
    ///
    /// Fails if `string_is_valid(buffer) == false`.
    #[inline]
    pub fn from_string(&self, buffer: impl AsRef<str>) -> Result<Version, Error<Owned>> {
        from_string(buffer)
    }

    /// Checks whether the version string is valid.
    #[inline]
    pub fn string_is_valid(&self, version_string: impl AsRef<str>) -> bool {
        string_is_valid(version_string)
    }

    /// Computes the length of the short version string.
    #[inline]
    pub fn string_length_short(&self, version: &Version) -> usize {
        string_length_short(version)
    }

    /// Computes the length of the long version string.
    #[inline]
    pub fn string_length_long(&self, version: &Version) -> usize {
        string_length_long(version)
    }

    /// Computes the length of the full version string.
    #[inline]
    pub fn string_length_full(&self, version: &Version) -> usize {
        string_length_full(version)
    }

    /// Represents the version as a short string.
    ///
    /// # Failure
    ///
    /// This function fails if `buffer.len() < string_length_short(version)`.
    #[inline]
    pub fn as_string_short(
        &self,
        version: &Version,
        buffer: impl AsMut<str>,
    ) -> Result<usize, Error<Owned>> {
        as_string_short(version, buffer)
    }

    /// Represents the version as a long string.
    ///
    /// # Failure
    ///
    /// This function fails if `buffer.len() < string_length_long(version)`.
    #[inline]
    pub fn as_string_long(
        &self,
        version: &Version,
        buffer: impl AsMut<str>,
    ) -> Result<usize, Error<Owned>> {
        as_string_long(version, buffer)
    }

    /// Represents the version as a full string.
    ///
    /// # Failure
    ///
    /// This function fails if `buffer.len() < string_length_full(version)`.
    #[inline]
    pub fn as_string_full(
        &self,
        version: &Version,
        buffer: impl AsMut<str>,
    ) -> Result<usize, Error<Owned>> {
        as_string_full(version, buffer)
    }

    /// Compares two versions.
    ///
    /// Compares two version, disregarding their build number.
    #[inline]
    pub fn compare(&self, lhs: &Version, rhs: &Version) -> Ordering {
        compare(lhs, rhs)
    }

    /// Compares two versions.
    ///
    /// Compares two version, disregarding their build number and release type.
    #[inline]
    pub fn compare_weak(&self, lhs: &Version, rhs: &Version) -> Ordering {
        compare_weak(lhs, rhs)
    }

    /// Compares two versions.
    #[inline]
    pub fn compare_strong(&self, lhs: &Version, rhs: &Version) -> Ordering {
        compare_strong(lhs, rhs)
    }

    /// Checks for compatibility of two versions.
    ///
    /// Two compatible versions can be used interchangeably.
    #[inline]
    pub fn is_compatible(&self, lhs: &Version, rhs: &Version) -> bool {
        is_compatible(lhs, rhs)
    }
}

macro_rules! impl_guarded_version {
    ($l:ty) => {
        impl<'a> DataGuard<'a, VersionAPI<'_>, $l> {
            /// Constructs a new version.
            ///
            /// Constructs a new version with `major`, `minor` and `patch` and sets the rest to `0`.
            #[inline]
            pub const fn new_short(&self, major: i32, minor: i32, patch: i32) -> Version {
                self.data.new_short(major, minor, patch)
            }

            /// Constructs a new version.
            ///
            /// Constructs a new version with `major`, `minor`, `patch`, `release_type` and
            /// `release_number` and sets the rest to `0`.
            #[inline]
            pub const fn new_long(
                &self,
                major: i32,
                minor: i32,
                patch: i32,
                release_type: ReleaseType,
                release_number: i8,
            ) -> Version {
                self.data
                    .new_long(major, minor, patch, release_type, release_number)
            }

            /// Constructs a new version.
            ///
            /// Constructs a new version with `major`, `minor`, `patch`, `release_type`,
            /// `release_number` and `build`.
            #[inline]
            pub const fn new_full(
                &self,
                major: i32,
                minor: i32,
                patch: i32,
                release_type: ReleaseType,
                release_number: i8,
                build: i64,
            ) -> Version {
                self.data
                    .new_full(major, minor, patch, release_type, release_number, build)
            }

            /// Constructs a version from a string.
            ///
            /// # Failure
            ///
            /// Fails if `string_is_valid(buffer) == false`.
            #[inline]
            pub fn from_string(&self, buffer: impl AsRef<str>) -> Result<Version, Error<Owned>> {
                self.data.from_string(buffer)
            }

            /// Checks whether the version string is valid.
            #[inline]
            pub fn string_is_valid(&self, version_string: impl AsRef<str>) -> bool {
                self.data.string_is_valid(version_string)
            }

            /// Computes the length of the short version string.
            #[inline]
            pub fn string_length_short(&self, version: &Version) -> usize {
                self.data.string_length_short(version)
            }

            /// Computes the length of the long version string.
            #[inline]
            pub fn string_length_long(&self, version: &Version) -> usize {
                self.data.string_length_long(version)
            }

            /// Computes the length of the full version string.
            #[inline]
            pub fn string_length_full(&self, version: &Version) -> usize {
                self.data.string_length_full(version)
            }

            /// Represents the version as a short string.
            ///
            /// # Failure
            ///
            /// This function fails if `buffer.len() < string_length_short(version)`.
            #[inline]
            pub fn as_string_short(
                &self,
                version: &Version,
                buffer: impl AsMut<str>,
            ) -> Result<usize, Error<Owned>> {
                self.data.as_string_short(version, buffer)
            }

            /// Represents the version as a long string.
            ///
            /// # Failure
            ///
            /// This function fails if `buffer.len() < string_length_long(version)`.
            #[inline]
            pub fn as_string_long(
                &self,
                version: &Version,
                buffer: impl AsMut<str>,
            ) -> Result<usize, Error<Owned>> {
                self.data.as_string_long(version, buffer)
            }

            /// Represents the version as a full string.
            ///
            /// # Failure
            ///
            /// This function fails if `buffer.len() < string_length_full(version)`.
            #[inline]
            pub fn as_string_full(
                &self,
                version: &Version,
                buffer: impl AsMut<str>,
            ) -> Result<usize, Error<Owned>> {
                self.data.as_string_full(version, buffer)
            }

            /// Compares two versions.
            ///
            /// Compares two version, disregarding their build number.
            #[inline]
            pub fn compare(&self, lhs: &Version, rhs: &Version) -> Ordering {
                self.data.compare(lhs, rhs)
            }

            /// Compares two versions.
            ///
            /// Compares two version, disregarding their build number and release type.
            #[inline]
            pub fn compare_weak(&self, lhs: &Version, rhs: &Version) -> Ordering {
                self.data.compare_weak(lhs, rhs)
            }

            /// Compares two versions.
            #[inline]
            pub fn compare_strong(&self, lhs: &Version, rhs: &Version) -> Ordering {
                self.data.compare_strong(lhs, rhs)
            }

            /// Checks for compatibility of two versions.
            ///
            /// Two compatible versions can be used interchangeably.
            #[inline]
            pub fn is_compatible(&self, lhs: &Version, rhs: &Version) -> bool {
                self.data.is_compatible(lhs, rhs)
            }
        }
    };
}

impl_guarded_version!(Unlocked);
impl_guarded_version!(Locked);
