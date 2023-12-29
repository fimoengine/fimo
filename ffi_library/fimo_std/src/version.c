#include <fimo_std/version.h>

#include <ctype.h>
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

static FimoError parse_str_u32(const char** str, size_t str_len, FimoU32* num)
{
    // 2^32 requires up a maximum of 10 digits, with 1 extra for
    // the 0 termination and another one to detect out of range
    // numbers.
    errno = 0;
    char tmp[12] = { 0 };
    size_t copy_count = (str_len < 11) ? str_len : 11;

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable : 4996)
#endif
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    strncpy(tmp, *str, copy_count);
#ifdef _MSC_VER
#pragma warning(pop)
#endif

    char* end = NULL;
    unsigned long value = strtoul(*str, &end, 10);
    if (errno == ERANGE || value > UINT32_MAX) {
        errno = 0;
        return FIMO_ERANGE;
    }
    if (*str == end) {
        return FIMO_EINVAL;
    }

    *str = end;
    *num = (FimoU32)value;
    return FIMO_EOK;
}

static FimoError parse_str_u64(const char** str, size_t str_len, FimoU64* num)
{
    // 2^64 requires up a maximum of 20 digits, with 1 extra for
    // the 0 termination and another one to detect out of range
    // numbers.
    errno = 0;
    char tmp[22] = { 0 };
    size_t copy_count = (str_len < 21) ? str_len : 21;

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable : 4996)
#endif
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    strncpy(tmp, *str, copy_count);
#ifdef _MSC_VER
#pragma warning(pop)
#endif

    char* end
        = NULL;
    unsigned long long value = strtoull(*str, &end, 10);
    if (errno == ERANGE || value > UINT64_MAX) {
        errno = 0;
        return FIMO_ERANGE;
    }
    if (*str == end) {
        return FIMO_EINVAL;
    }

    *str = end;
    *num = (FimoU64)value;
    return FIMO_EOK;
}

FIMO_MUST_USE FimoError fimo_version_parse_str(const char* str, size_t str_len, FimoVersion* version)
{
    if (!str || str_len == 0 || isspace(*str) || !version) {
        return FIMO_EINVAL;
    }

    const char* current = str;
    const char* str_end = str + str_len;

    FimoError error = parse_str_u32(&current, str_end - current, &version->major);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }
    if (current == str_end || *current != '.') {
        return FIMO_EINVAL;
    }

    current++;
    error = parse_str_u32(&current, str_end - current, &version->minor);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }
    if (current == str_end || *current != '.') {
        return FIMO_EINVAL;
    }

    current++;
    error = parse_str_u32(&current, str_end - current, &version->patch);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    if (current == str_end) {
        return FIMO_EOK;
    } else if (*current != '+') {
        return FIMO_EINVAL;
    }

    current++;
    error = parse_str_u64(&current, str_end - current, &version->build);
    return error;
}

FIMO_MUST_USE size_t fimo_version_str_len(const FimoVersion* version)
{
    if (!version) {
        return 0;
    }

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable : 4996)
#endif
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    return snprintf(NULL, 0, "%" PRIu32 ".%" PRIu32 ".%" PRIu32, version->major, version->minor, version->patch);
#ifdef _MSC_VER
#pragma warning(pop)
#endif
}

FIMO_MUST_USE size_t fimo_version_str_len_full(const FimoVersion* version)
{
    if (!version) {
        return 0;
    }

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable : 4996)
#endif
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    return snprintf(NULL, 0, "%" PRIu32 ".%" PRIu32 ".%" PRIu32 "+%" PRIu64,
        version->major, version->minor, version->patch, version->build);
#ifdef _MSC_VER
#pragma warning(pop)
#endif
}

FIMO_MUST_USE FimoError fimo_version_write_str(const FimoVersion* version, char* str, size_t str_len, size_t* written)
{
    if (!version || !str || str_len == 0) {
        return FIMO_EINVAL;
    }

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable : 4996)
#endif
    char tmp[FIMO_VERSION_MAX_STR_LENGTH + 1] = { 0 };
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    int req_len = snprintf(tmp, sizeof tmp, "%" PRIu32 ".%" PRIu32 ".%" PRIu32,
        version->major, version->minor, version->patch);
    if ((size_t)req_len > str_len) {
        if (written) {
            *written = 0;
        }
        return FIMO_EINVAL;
    }
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    strncpy(str, tmp, req_len);
    if ((size_t)req_len < str_len) {
        str[req_len] = 0;
    }

    if (written) {
        *written = (size_t)req_len;
    }

    return FIMO_EOK;
#ifdef _MSC_VER
#pragma warning(pop)
#endif
}

FIMO_MUST_USE FimoError fimo_version_write_str_long(const FimoVersion* version, char* str, size_t str_len, size_t* written)
{
    if (!version || !str || str_len == 0) {
        return FIMO_EINVAL;
    }

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable : 4996)
#endif
    char tmp[FIMO_VERSION_LONG_MAX_STR_LENGTH + 1] = { 0 };
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    int req_len = snprintf(tmp, sizeof tmp, "%" PRIu32 ".%" PRIu32 ".%" PRIu32 "+%" PRIu64,
        version->major, version->minor, version->patch, version->build);
    if ((size_t)req_len > str_len) {
        if (written) {
            *written = 0;
        }
        return FIMO_EINVAL;
    }
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    strncpy(str, tmp, req_len);
    if ((size_t)req_len < str_len) {
        str[req_len] = 0;
    }

    if (written) {
        *written = (size_t)req_len;
    }

    return FIMO_EOK;
#ifdef _MSC_VER
#pragma warning(pop)
#endif
}

FIMO_MUST_USE int fimo_version_cmp(const FimoVersion* lhs, const FimoVersion* rhs)
{
    if (lhs->major < rhs->major) {
        return -1;
    } else if (lhs->major > rhs->major) {
        return 1;
    }

    if (lhs->minor < rhs->minor) {
        return -1;
    } else if (lhs->minor > rhs->minor) {
        return 1;
    }

    if (lhs->patch < rhs->patch) {
        return -1;
    } else if (lhs->patch > rhs->patch) {
        return 1;
    }

    return 0;
}

FIMO_MUST_USE int fimo_version_cmp_long(const FimoVersion* lhs, const FimoVersion* rhs)
{
    int res = fimo_version_cmp(lhs, rhs);
    if (res != 0) {
        return res;
    }

    if (lhs->build < rhs->build) {
        return -1;
    } else if (lhs->build > rhs->build) {
        return 1;
    }

    return 0;
}

FIMO_MUST_USE bool fimo_version_compatible(const FimoVersion* got, const FimoVersion* required)
{
    if (required->major != got->major || (required->major == 0 && required->minor != got->minor)) {
        return false;
    }
    return fimo_version_cmp(required, got) <= 0;
}
