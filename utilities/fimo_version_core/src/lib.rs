//! This crate implements the
//! [version specification](https://fimoengine.github.io/emf-rfcs/0004-versioning-specification.html).
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use lazy_static::lazy_static;
use numtoa::NumToA;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

lazy_static! {
    static ref VERSION_VALIDATOR: regex::Regex =
        regex::Regex::new(r"(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)(-(?P<release_type>(unstable|beta))(\.(?P<release_number>\d+))?)?(\+(?P<build>\d+))?").unwrap();
}

/// Version errors.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum VersionError {
    /// Invalid string format.
    InvalidString(String),
    /// Buffer overflow.
    BufferOverflow {
        /// Buffer length.
        buffer: usize,
        /// Needed length.
        needed: usize,
    },
}

impl Display for VersionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionError::InvalidString(str) => write!(f, "Invalid String: {}", str),
            VersionError::BufferOverflow { buffer, needed } => {
                write!(f, "Buffer overflow! buffer: {}, needed: {}", buffer, needed)
            }
        }
    }
}

impl std::error::Error for VersionError {}

/// Errors of the version api.
#[repr(i8)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ReleaseType {
    /// Stable release.
    Stable = 0,
    /// Unstable pre-release.
    Unstable = 1,
    /// API-stable pre-release.
    Beta = 2,
}

impl Display for ReleaseType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseType::Stable => write!(f, "Stable"),
            ReleaseType::Unstable => write!(f, "Unstable"),
            ReleaseType::Beta => write!(f, "Beta"),
        }
    }
}

/// A version.
#[repr(C)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Version {
    /// Major version.
    pub major: i32,
    /// Minor version.
    pub minor: i32,
    /// Patch version.
    pub patch: i32,
    /// Build number.
    pub build: i64,
    /// Release number.
    pub release_number: i8,
    /// Release type.
    pub release_type: ReleaseType,
}

impl Version {
    /// Constructs a new version.
    ///
    /// Constructs a new version with `major`, `minor` and `patch` and sets the rest to `0`.
    #[inline]
    pub const fn new_short(major: i32, minor: i32, patch: i32) -> Self {
        Version {
            major,
            minor,
            patch,
            build: 0,
            release_number: 0,
            release_type: ReleaseType::Stable,
        }
    }

    /// Constructs a new version.
    ///
    /// Constructs a new version with `major`, `minor`, `patch`, `release_type` and
    /// `release_number` and sets the rest to `0`.
    #[inline]
    pub const fn new_long(
        major: i32,
        minor: i32,
        patch: i32,
        release_type: ReleaseType,
        release_number: i8,
    ) -> Self {
        let release_number = match release_type {
            ReleaseType::Stable => 0,
            _ => release_number,
        };

        Self {
            major,
            minor,
            patch,
            build: 0,
            release_number,
            release_type,
        }
    }

    /// Constructs a new version.
    ///
    /// Constructs a new version with `major`, `minor`, `patch`, `release_type`,
    /// `release_number` and `build`.
    #[inline]
    pub const fn new_full(
        major: i32,
        minor: i32,
        patch: i32,
        release_type: ReleaseType,
        release_number: i8,
        build: i64,
    ) -> Self {
        let release_number = match release_type {
            ReleaseType::Stable => 0,
            _ => release_number,
        };

        Self {
            major,
            minor,
            patch,
            build,
            release_number,
            release_type,
        }
    }

    /// Constructs a version from a string.
    ///
    /// # Failure
    ///
    /// Fails if `string_is_valid(buffer) == false`.
    #[inline]
    pub fn from_string(buffer: impl AsRef<str>) -> Result<Self, VersionError> {
        let captures = Self::validate_string(&buffer)?;

        let major = captures["major"].parse().unwrap();
        let minor = captures["minor"].parse().unwrap();
        let patch = captures["patch"].parse().unwrap();
        let release_type = match captures.name("release_type") {
            Some(release_type) => match release_type.as_str() {
                "beta" => ReleaseType::Beta,
                "unstable" => ReleaseType::Unstable,
                _ => unreachable!(),
            },
            None => ReleaseType::Stable,
        };
        let release_number = match captures.name("release_number") {
            Some(release_number) => release_number.as_str().parse().unwrap(),
            None => 0,
        };
        let build = match captures.name("build") {
            Some(build) => build.as_str().parse().unwrap(),
            None => 0,
        };

        Ok(Self {
            major,
            minor,
            patch,
            build,
            release_number,
            release_type,
        })
    }

    /// Checks whether the version string is valid.
    pub fn string_is_valid(version_string: impl AsRef<str>) -> bool {
        Self::validate_string(&version_string).is_ok()
    }

    /// Checks whether the version string is valid.
    ///
    /// Returns the matches of the regex.
    fn validate_string(
        version_string: &impl AsRef<str>,
    ) -> Result<regex::Captures<'_>, VersionError> {
        VERSION_VALIDATOR.captures(version_string.as_ref()).map_or(
            Err(VersionError::InvalidString(
                version_string.as_ref().to_string(),
            )),
            |v| {
                if version_string.as_ref().len() == v[0].len() {
                    Ok(v)
                } else {
                    Err(VersionError::InvalidString(
                        version_string.as_ref().to_string(),
                    ))
                }
            },
        )
    }

    /// Computes the length of the short version string.
    pub fn string_length_short(&self) -> usize {
        let mut buffer = [0u8; 20];
        self.major.numtoa_str(10, &mut buffer).len()
            + 1
            + self.minor.numtoa_str(10, &mut buffer).len()
            + 1
            + self.patch.numtoa_str(10, &mut buffer).len()
    }

    /// Computes the length of the long version string.
    pub fn string_length_long(&self) -> usize {
        let mut length = self.string_length_short();

        length += match self.release_type {
            ReleaseType::Stable => return length,
            ReleaseType::Beta => "-beta".len(),
            ReleaseType::Unstable => "-unstable".len(),
        };

        if self.release_number != 0 {
            let mut buffer = [0u8; 20];
            length += 1 + self.release_number.numtoa_str(10, &mut buffer).len();
        }

        length
    }

    /// Computes the length of the full version string.
    pub fn string_length_full(&self) -> usize {
        let mut length = self.string_length_long();

        if self.build != 0 {
            let mut buffer = [0u8; 20];
            length += 1 + self.build.numtoa_str(10, &mut buffer).len();
        }

        length
    }

    /// Represents the version as a short string.
    ///
    /// # Failure
    ///
    /// This function fails if `buffer.len() < string_length_short(version)`.
    pub fn as_string_short(&self, mut buffer: impl AsMut<str>) -> Result<usize, VersionError> {
        let mut digit_buffer = [0u8; 20];
        let buffer = unsafe { buffer.as_mut().as_bytes_mut() };

        let mut length = 0;
        let major_buff = self.major.numtoa(10, &mut digit_buffer);
        if length + major_buff.len() + 1 >= buffer.len() {
            return Err(VersionError::BufferOverflow {
                buffer: buffer.len(),
                needed: self.string_length_short(),
            });
        }
        buffer[length..length + major_buff.len()].copy_from_slice(&major_buff);
        length += major_buff.len();

        buffer[length] = b'.';
        length += 1;

        let minor_buff = self.minor.numtoa(10, &mut digit_buffer);
        if length + minor_buff.len() + 1 >= buffer.len() {
            return Err(VersionError::BufferOverflow {
                buffer: buffer.len(),
                needed: self.string_length_short(),
            });
        }
        buffer[length..length + minor_buff.len()].copy_from_slice(&minor_buff);
        length += minor_buff.len();

        buffer[length] = b'.';
        length += 1;

        let patch_buff = self.patch.numtoa(10, &mut digit_buffer);
        if length + patch_buff.len() > buffer.len() {
            return Err(VersionError::BufferOverflow {
                buffer: buffer.len(),
                needed: self.string_length_short(),
            });
        }
        buffer[length..length + patch_buff.len()].copy_from_slice(&patch_buff);
        length += patch_buff.len();

        Ok(length)
    }

    /// Represents the version as a long string.
    ///
    /// # Failure
    ///
    /// This function fails if `buffer.len() < string_length_long(version)`.
    pub fn as_string_long(&self, mut buffer: impl AsMut<str>) -> Result<usize, VersionError> {
        let mut length = self.as_string_short(&mut buffer)?;
        let buffer = unsafe { buffer.as_mut().as_bytes_mut() };

        let release_type = match self.release_type {
            ReleaseType::Stable => return Ok(length),
            ReleaseType::Beta => "-beta",
            ReleaseType::Unstable => "-unstable",
        };

        if length + release_type.len() > buffer.len() {
            return Err(VersionError::BufferOverflow {
                buffer: buffer.len(),
                needed: self.string_length_long(),
            });
        }
        buffer[length..length + release_type.len()].copy_from_slice(release_type.as_bytes());
        length += release_type.len();

        if self.release_number > 0 {
            if length + 1 > buffer.len() {
                return Err(VersionError::BufferOverflow {
                    buffer: buffer.len(),
                    needed: self.string_length_long(),
                });
            }
            buffer[length] = b'.';
            length += 1;

            let mut digit_buffer = [0u8; 20];
            let release_number_buff = self.release_number.numtoa(10, &mut digit_buffer);
            if length + release_number_buff.len() > buffer.len() {
                return Err(VersionError::BufferOverflow {
                    buffer: buffer.len(),
                    needed: self.string_length_long(),
                });
            }
            buffer[length..length + release_number_buff.len()]
                .copy_from_slice(&release_number_buff);
            length += release_number_buff.len();
        }

        Ok(length)
    }

    /// Represents the version as a full string.
    ///
    /// # Failure
    ///
    /// This function fails if `buffer.len() < string_length_full(version)`.
    pub fn as_string_full(&self, mut buffer: impl AsMut<str>) -> Result<usize, VersionError> {
        let mut length = self.as_string_long(&mut buffer)?;

        if self.build > 0 {
            let buffer = unsafe { buffer.as_mut().as_bytes_mut() };

            if length + 1 > buffer.len() {
                return Err(VersionError::BufferOverflow {
                    buffer: buffer.len(),
                    needed: self.string_length_full(),
                });
            }
            buffer[length] = b'+';
            length += 1;

            let mut digit_buffer = [0u8; 20];
            let build_buff = self.build.numtoa(10, &mut digit_buffer);
            if length + build_buff.len() > buffer.len() {
                return Err(VersionError::BufferOverflow {
                    buffer: buffer.len(),
                    needed: self.string_length_full(),
                });
            }
            buffer[length..length + build_buff.len()].copy_from_slice(&build_buff);
            length += build_buff.len();
        }

        Ok(length)
    }

    /// Compares two versions.
    ///
    /// Compares two version, disregarding their build number.
    pub fn compare(&self, other: &Self) -> Ordering {
        match self.compare_weak(other) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => {
                // Order of the release types.
                // Stable (idx 0)
                // Beta (idx 2)
                // Unstable (idx 1)
                const ORDERINGS: [usize; 3] = [2, 0, 1];
                match ORDERINGS[self.release_type as usize]
                    .cmp(&ORDERINGS[other.release_type as usize])
                {
                    Ordering::Less => Ordering::Less,
                    Ordering::Equal => self.release_number.cmp(&other.release_number),
                    Ordering::Greater => Ordering::Greater,
                }
            }
            Ordering::Greater => Ordering::Greater,
        }
    }

    /// Compares two versions.
    ///
    /// Compares two version, disregarding their build number and release type.
    pub fn compare_weak(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => self.patch.cmp(&other.patch),
                Ordering::Greater => Ordering::Greater,
            },
            Ordering::Greater => Ordering::Greater,
        }
    }

    /// Compares two versions.
    pub fn compare_strong(&self, other: &Self) -> Ordering {
        match self.compare(other) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => self.build.cmp(&other.build),
            Ordering::Greater => Ordering::Greater,
        }
    }

    /// Checks for compatibility of two versions.
    ///
    /// Two compatible versions can be used interchangeably.
    pub fn is_compatible(&self, other: &Self) -> bool {
        if self.major != other.major || (self.major == 0 && self.minor != other.minor) {
            false
        } else {
            let comparison = self.compare(other);
            match comparison {
                Ordering::Less | Ordering::Equal => match self.release_type {
                    ReleaseType::Stable => true,
                    ReleaseType::Unstable => comparison == Ordering::Equal,
                    ReleaseType::Beta => match self.compare_weak(other) {
                        Ordering::Equal => true,
                        Ordering::Less | Ordering::Greater => false,
                    },
                },
                Ordering::Greater => false,
            }
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.release_type {
            ReleaseType::Stable => {
                write!(
                    f,
                    "{}.{}.{} Build {}",
                    self.major, self.minor, self.patch, self.build
                )
            }
            _ => {
                write!(
                    f,
                    "{}.{}.{} {} {} Build {}",
                    self.major,
                    self.minor,
                    self.patch,
                    self.release_type,
                    self.release_number,
                    self.build
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ReleaseType, Version};
    use std::cmp::Ordering;

    #[test]
    fn new_test() {
        assert_eq!(
            Version::new_short(0, 1, 0),
            Version {
                major: 0,
                minor: 1,
                patch: 0,
                build: 0,
                release_number: 0,
                release_type: ReleaseType::Stable
            }
        );

        assert_eq!(
            Version::new_long(0, 1, 0, ReleaseType::Unstable, 7),
            Version {
                major: 0,
                minor: 1,
                patch: 0,
                build: 0,
                release_number: 7,
                release_type: ReleaseType::Unstable
            }
        );

        assert_eq!(
            Version::new_long(0, 1, 0, ReleaseType::Stable, 7),
            Version {
                major: 0,
                minor: 1,
                patch: 0,
                build: 0,
                release_number: 0,
                release_type: ReleaseType::Stable
            }
        );

        assert_eq!(
            Version::new_full(0, 1, 0, ReleaseType::Unstable, 7, 4156),
            Version {
                major: 0,
                minor: 1,
                patch: 0,
                build: 4156,
                release_number: 7,
                release_type: ReleaseType::Unstable
            }
        );

        assert_eq!(
            Version::new_full(0, 1, 0, ReleaseType::Stable, 7, 4156),
            Version {
                major: 0,
                minor: 1,
                patch: 0,
                build: 4156,
                release_number: 0,
                release_type: ReleaseType::Stable
            }
        );
    }

    #[test]
    fn validate() {
        assert_eq!(Version::string_is_valid("1.0.0"), true);
        assert_eq!(Version::string_is_valid("1.0.0+512"), true);
        assert_eq!(Version::string_is_valid("1.0.0-unstable"), true);
        assert_eq!(Version::string_is_valid("1.0.0-unstable+1112"), true);
        assert_eq!(Version::string_is_valid("1.0.0-beta.12"), true);
        assert_eq!(Version::string_is_valid("1.0.0-beta.12+1215120"), true);

        assert_eq!(Version::string_is_valid("1"), false);
        assert_eq!(Version::string_is_valid("1.0"), false);
        assert_eq!(Version::string_is_valid("1.0.0-"), false);
        assert_eq!(Version::string_is_valid("1.0.0-stable"), false);
        assert_eq!(Version::string_is_valid("1.0.0-unstable."), false);
        assert_eq!(Version::string_is_valid("1.0.0-unstable.0+"), false);
    }

    #[test]
    fn from_str() {
        assert_eq!(
            Version::from_string("1.0.0"),
            Ok(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 0,
                release_number: 0,
                release_type: ReleaseType::Stable
            })
        );
        assert_eq!(
            Version::from_string("1.0.0+512"),
            Ok(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 512,
                release_number: 0,
                release_type: ReleaseType::Stable
            })
        );
        assert_eq!(
            Version::from_string("1.0.0-unstable"),
            Ok(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 0,
                release_number: 0,
                release_type: ReleaseType::Unstable
            })
        );
        assert_eq!(
            Version::from_string("1.0.0-unstable+1112"),
            Ok(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 1112,
                release_number: 0,
                release_type: ReleaseType::Unstable
            })
        );
        assert_eq!(
            Version::from_string("1.0.0-beta.12"),
            Ok(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 0,
                release_number: 12,
                release_type: ReleaseType::Beta
            })
        );
        assert_eq!(
            Version::from_string("1.0.0-beta.12+1215120"),
            Ok(Version {
                major: 1,
                minor: 0,
                patch: 0,
                build: 1215120,
                release_number: 12,
                release_type: ReleaseType::Beta
            })
        );
    }

    #[test]
    fn string_length() {
        let version_strings = [
            ("1.0.0", 5, 5, 5),
            ("1.0.0+512", 5, 5, 9),
            ("1.0.0-unstable", 5, 14, 14),
            ("1.0.0-unstable+1112", 5, 14, 19),
            ("1.0.0-beta.12", 5, 13, 13),
            ("1.0.0-beta.12+1215120", 5, 13, 21),
        ];

        for version_string in version_strings.iter() {
            let version = Version::from_string(version_string.0).unwrap();
            assert_eq!(Version::string_length_short(&version), version_string.1);
            assert_eq!(Version::string_length_long(&version), version_string.2);
            assert_eq!(Version::string_length_full(&version), version_string.3);
        }
    }

    #[test]
    fn as_string() {
        let version_strings = [
            ("1.0.0", "1.0.0", "1.0.0", "1.0.0"),
            ("1.0.0+512", "1.0.0", "1.0.0", "1.0.0+512"),
            (
                "1.0.0-unstable",
                "1.0.0",
                "1.0.0-unstable",
                "1.0.0-unstable",
            ),
            (
                "1.0.0-unstable+1112",
                "1.0.0",
                "1.0.0-unstable",
                "1.0.0-unstable+1112",
            ),
            ("1.0.0-beta.12", "1.0.0", "1.0.0-beta.12", "1.0.0-beta.12"),
            (
                "1.0.0-beta.12+1215120",
                "1.0.0",
                "1.0.0-beta.12",
                "1.0.0-beta.12+1215120",
            ),
        ];

        let mut buffer = [0u8; 40];
        let mut str = unsafe { std::str::from_utf8_unchecked_mut(&mut buffer) };
        for version_string in version_strings.iter() {
            let version = Version::from_string(version_string.0).unwrap();
            assert_eq!(
                Version::as_string_short(&version, &mut str),
                Ok(version_string.1.len())
            );
            assert_eq!(&str[..version_string.1.len()], version_string.1);
            assert_eq!(
                Version::as_string_long(&version, &mut str),
                Ok(version_string.2.len())
            );
            assert_eq!(&str[..version_string.2.len()], version_string.2);
            assert_eq!(
                Version::as_string_full(&version, &mut str),
                Ok(version_string.3.len())
            );
            assert_eq!(&str[..version_string.3.len()], version_string.3);
        }
    }

    #[test]
    fn comparisons() {
        let v1 = Version::new_short(0, 1, 0);
        let v2 = Version::new_short(0, 1, 1);
        let v3 = Version::new_short(1, 0, 0);
        let v4 = Version::new_long(1, 0, 0, ReleaseType::Beta, 1);

        assert_eq!(Version::compare_weak(&v1, &v2), Ordering::Less);
        assert_eq!(Version::compare_weak(&v2, &v3), Ordering::Less);
        assert_eq!(Version::compare_weak(&v3, &v4), Ordering::Equal);

        assert_eq!(Version::compare_weak(&v1, &v1), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v2, &v2), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v3, &v3), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v4, &v4), Ordering::Equal);

        assert_eq!(Version::compare_weak(&v2, &v1), Ordering::Greater);
        assert_eq!(Version::compare_weak(&v3, &v2), Ordering::Greater);
        assert_eq!(Version::compare_weak(&v4, &v3), Ordering::Equal);

        let v1 = Version::new_long(0, 1, 0, ReleaseType::Unstable, 0);
        let v2 = Version::new_long(0, 1, 0, ReleaseType::Unstable, 2);
        let v3 = Version::new_long(0, 1, 0, ReleaseType::Beta, 0);
        let v4 = Version::new_long(1, 0, 0, ReleaseType::Stable, 0);
        let v5 = Version::new_full(1, 0, 0, ReleaseType::Stable, 0, 15132);

        assert_eq!(Version::compare(&v1, &v2), Ordering::Less);
        assert_eq!(Version::compare(&v2, &v3), Ordering::Less);
        assert_eq!(Version::compare(&v3, &v4), Ordering::Less);
        assert_eq!(Version::compare(&v4, &v5), Ordering::Equal);

        assert_eq!(Version::compare_weak(&v1, &v1), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v2, &v2), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v3, &v3), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v4, &v4), Ordering::Equal);

        assert_eq!(Version::compare(&v2, &v1), Ordering::Greater);
        assert_eq!(Version::compare(&v3, &v2), Ordering::Greater);
        assert_eq!(Version::compare(&v4, &v3), Ordering::Greater);
        assert_eq!(Version::compare(&v5, &v4), Ordering::Equal);

        let v1 = Version::new_full(1, 0, 0, ReleaseType::Beta, 3, 50);
        let v2 = Version::new_full(1, 0, 0, ReleaseType::Beta, 3, 51);
        let v3 = Version::new_full(1, 0, 0, ReleaseType::Stable, 0, 0);

        assert_eq!(Version::compare_strong(&v1, &v2), Ordering::Less);
        assert_eq!(Version::compare_strong(&v2, &v3), Ordering::Less);

        assert_eq!(Version::compare_weak(&v1, &v1), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v2, &v2), Ordering::Equal);
        assert_eq!(Version::compare_weak(&v3, &v3), Ordering::Equal);

        assert_eq!(Version::compare_strong(&v2, &v1), Ordering::Greater);
        assert_eq!(Version::compare_strong(&v3, &v2), Ordering::Greater);
    }

    #[test]
    fn compatibility() {
        let v1 = Version::new_full(0, 1, 0, ReleaseType::Stable, 0, 0);
        let v2 = Version::new_full(0, 1, 5, ReleaseType::Stable, 0, 0);

        let v3 = Version::new_full(1, 0, 5, ReleaseType::Unstable, 1, 1000);
        let v4 = Version::new_full(1, 0, 5, ReleaseType::Unstable, 1, 1151);

        let v5 = Version::new_full(1, 0, 5, ReleaseType::Beta, 2, 1000);
        let v6 = Version::new_full(1, 0, 5, ReleaseType::Beta, 5, 1);

        let v7 = Version::new_full(1, 1, 0, ReleaseType::Stable, 0, 0);
        let v8 = Version::new_full(1, 2, 7, ReleaseType::Stable, 0, 0);

        assert_eq!(Version::is_compatible(&v1, &v2), true);
        assert_eq!(Version::is_compatible(&v3, &v4), true);
        assert_eq!(Version::is_compatible(&v5, &v6), true);
        assert_eq!(Version::is_compatible(&v7, &v8), true);
    }
}
