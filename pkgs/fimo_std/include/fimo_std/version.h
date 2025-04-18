#ifndef FIMO_VERSION_H
#define FIMO_VERSION_H

#include <stddef.h>

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

/// A version specifier following the Semantic Versioning 2.0.0 specification.
typedef struct FimoVersion {
    FimoUSize major;
    FimoUSize minor;
    FimoUSize patch;
    const char *pre;
    FimoUSize pre_len;
    const char *build;
    FimoUSize build_len;
} FimoVersion;

/// Parses a string into a `FimoVersion`.
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_version_parse_str(const char *str, size_t str_len, FimoVersion *version);

/// Calculates the string length required to represent the version as a string.
///
/// The returned length is large enough for a call to `fimo_version_write_str` with the same
/// version instance. The returned length does not include the zero-terminator.
FIMO_EXPORT
FIMO_MUST_USE
size_t fimo_version_str_len(const FimoVersion *version);

/// Calculates the string length required to represent the version as a string.
///
/// The returned length is large enough for a call to `fimo_version_write_str_full` with the same
/// version instance. The returned length does not include the zero-terminator.
FIMO_EXPORT
FIMO_MUST_USE
size_t fimo_version_str_len_full(const FimoVersion *version);

/// Represents the version as a string.
///
/// Writes a string of the form "major.minor.patch" into `str`. If `str` is large enough to store a
/// zero-terminator, it is appended at the end of the written characters. If `written` is not
/// `NULL`, it is set to the number of characters written without the zero-terminator.
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_version_write_str(const FimoVersion *version, char *str, size_t str_len, size_t *written);

/// Represents the version as a string.
///
/// Writes a string representation of the version into `str`. If `str` is large enough to
/// store a zero-terminator, it is appended at the end of the written characters. If `written` is
/// not `NULL`, it is set to the number of characters written without the zero-terminator.
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_version_write_str_full(const FimoVersion *version, char *str, size_t str_len, size_t *written);

/// Compares two versions.
///
/// Returns an ordering of the two versions, without taking into consideration the build numbers.
/// Returns `-1` if `lhs < rhs`, `0` if `lhs == rhs`, or `1` if `lhs > rhs`.
FIMO_EXPORT
FIMO_MUST_USE
int fimo_version_cmp(const FimoVersion *lhs, const FimoVersion *rhs);

/// Checks for the compatibility of two versions.
///
/// If `got` is compatible with `required` it indicates that an object which is versioned with the
/// version `got` can be used in stead of an object of the same type carrying the version
/// `required`.
///
/// The compatibility of `got` with `required` is determined by the following algorithm:
///
/// 1. The major versions of `got` and `required` must be equal.
/// 2. If the major version is `0`, the minor versions must be equal.
/// 3. `got >= required`.
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_version_compatible(const FimoVersion *got, const FimoVersion *required);

#ifdef __cplusplus
}
#endif

#endif // FIMO_VERSION_H
