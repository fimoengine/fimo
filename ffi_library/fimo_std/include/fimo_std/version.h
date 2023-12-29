#ifndef FIMO_VERSION_H
#define FIMO_VERSION_H

#include <stddef.h>

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * A version specifier.
 */
typedef struct FimoVersion {
    FimoU32 major;
    FimoU32 minor;
    FimoU32 patch;
    FimoU64 build;
} FimoVersion;

/**
 * Major version of fimo std.
 */
#define FIMO_VERSION_MAJOR 0

/**
 * Minor version of fimo std.
 */
#define FIMO_VERSION_MINOR 1

/**
 * Patch version of fimo std.
 */
#define FIMO_VERSION_PATCH 0

/**
 * Build number of fimo std.
 */
#define FIMO_VERSION_BUILD_NUMBER 0

/**
 * Maximum required string length (without zero-terminator) required to represent
 * a version without the build number.
 */
#define FIMO_VERSION_MAX_STR_LENGTH 32

/**
 * Maximum required string length (without zero-terminator) required to represent
 * a version with the build number.
 */
#define FIMO_VERSION_LONG_MAX_STR_LENGTH 53

/**
 * Constructs a new `FimoVersion`.
 */
#define FIMO_VERSION_LONG(major_num, minor_num, patch_num, build_num)                          \
    {                                                                                          \
        .major = (major_num), .minor = (minor_num), .patch = (patch_num), .build = (build_num) \
    }

/**
 * Constructs a new `FimoVersion`.
 */
#define FIMO_VERSION(major_num, minor_num, patch_num) FIMO_VERSION_LONG(major_num, minor_num, patch_num, 0)

/**
 * Parses a string into a `FimoVersion`.
 *
 * The string must be of the form "major.minor.patch" or "major.minor.patch+build".
 *
 * @param str string to parse
 * @param str_len length of the string
 * @param version pointer to the parsed version.
 *
 * @return Status code
 */
FIMO_MUST_USE FimoError fimo_version_parse_str(const char* str, size_t str_len, FimoVersion* version);

/**
 * Calculates the string length required to represent the version as a string.
 *
 * If `version` is `NULL`, this function returns `0`. The returned length is
 * large enough for a call to @ref fimo_version_write_str with the same version
 * instance. The returned length does not include the zero-terminator.
 *
 * @param version version to check
 *
 * @return Required string length
 */
FIMO_MUST_USE size_t fimo_version_str_len(const FimoVersion* version);

/**
 * Calculates the string length required to represent the version as a string.
 *
 * If `version` is `NULL`, this function returns `0`. The returned length is
 * large enough for a call to @ref fimo_version_write_str_long with the same
 * version instance. The returned length does not include the zero-terminator.
 *
 * @param version version to check
 *
 * @return Required string length
 */
FIMO_MUST_USE size_t fimo_version_str_len_full(const FimoVersion* version);

/**
 * Represents the version as a string.
 *
 * Writes a string of the form "major.minor.patch" into `str`. If `str` is
 * large enough to store a zero-terminator, it is appended at the end of the
 * written characters. If `written` is not `NULL`, it is set to the number of
 * characters written without the zero-terminator.
 *
 * @param version version to write out
 * @param str destination string
 * @param str_len destination string length
 * @param written pointer to the written character count
 *
 * @return Status code
 */
FIMO_MUST_USE FimoError fimo_version_write_str(const FimoVersion* version, char* str, size_t str_len, size_t* written);

/**
 * Represents the version as a string.
 *
 * Writes a string of the form "major.minor.patch+build" into `str`. If `str`
 * is large enough to store a zero-terminator, it is appended at the end of the
 * written characters. If `written` is not `NULL`, it is set to the number of
 * characters written without the zero-terminator.
 *
 * @param version version to write out
 * @param str destination string
 * @param str_len destination string length
 * @param written pointer to the written character count
 *
 * @return Status code
 */
FIMO_MUST_USE FimoError fimo_version_write_str_long(const FimoVersion* version, char* str, size_t str_len, size_t* written);

/**
 * Compares two versions.
 *
 * Returns an ordering of the two versions, without taking into consideration
 * the build numbers. Returns `-1` if `lhs < rhs`, `0` if `lhs == rhs`, or
 * `1` if `lhs > rhs`.
 *
 * @param lhs first version (not `NULL`)
 * @param rhs second version (not `NULL`)
 *
 * @return Version ordering
 */
FIMO_MUST_USE int fimo_version_cmp(const FimoVersion* lhs, const FimoVersion* rhs);

/**
 * Compares two versions.
 *
 * Returns an ordering of the two versions, taking into consideration the build
 * numbers. Returns `-1` if `lhs < rhs`, `0` if `lhs == rhs`, or `1` if `lhs > rhs`.
 *
 * @param lhs first version (not `NULL`)
 * @param rhs second version (not `NULL`)
 *
 * @return Version ordering
 */
FIMO_MUST_USE int fimo_version_cmp_long(const FimoVersion* lhs, const FimoVersion* rhs);

/**
 * Checks for the compatibility of two versions.
 *
 * If `got` is compatible with `required` it indicates that an object which is
 * versioned with the version `got` can be used in stead of an object of the
 * same type carrying the version `required`.
 *
 * The compatibility of `got` with `required` is determined by the following
 * algorithm:
 *
 *      1. The major versions of `got` and `required` must be equal.
 *      2. If the major version is `0`, the minor versions must be equal.
 *      3. `got >= required` without the build number.
 *
 * @param got version to check for compatibility
 * @param required required version
 *
 * @return `true` if is compatible, `false` otherwise.
 */
FIMO_MUST_USE bool fimo_version_compatible(const FimoVersion* got, const FimoVersion* required);

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_VERSION_H
