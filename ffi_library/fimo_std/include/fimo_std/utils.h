#ifndef FIMO_UTILS_H
#define FIMO_UTILS_H

#include <limits.h>
#include <stddef.h>
#include <stdint.h>
#include <fimo_std/impl/macros/has_builtin.h>
#include <fimo_std/impl/macros/inline.h>

// Include the sal header on windows.
#ifdef _MSC_VER
#ifdef _USE_ATTRIBUTES_FOR_SAL
#undef _USE_ATTRIBUTES_FOR_SAL
#endif
#define _USE_ATTRIBUTES_FOR_SAL 1
#include <sal.h>
#endif

/**
 * Marks a parameter as a printf format string.
 */
#if defined(__GNUC__)
#define FIMO_PRINT_F_FORMAT /**/
#elif defined(_MSC_VER)
#define FIMO_PRINT_F_FORMAT _Printf_format_string_
#endif

/**
 * Marks that a function accepts a printf format string and
 * corresponding arguments or a `va_list` that contains these
 * arguments.
 *
 * @param format_param index of the format string parameter
 * @param dots_param index of the arguments parameter
 */
#if defined(__GNUC__)
#define FIMO_PRINT_F_FORMAT_ATTR(format_param, dots_param) \
    __attribute__((__format__(__printf__, format_param, dots_param)))
#elif defined(_MSC_VER)
#define FIMO_PRINT_F_FORMAT_ATTR(format_param, dots_param) /**/
#endif

/**
 * Adds a `must_use` attribute to a function return value.
 */
#if defined(__GNUC__) && (__GNUC__ >= 4)
#define FIMO_MUST_USE __attribute__((warn_unused_result))
#elif defined(_MSC_VER) && (_MSC_VER >= 1700)
#define FIMO_MUST_USE _Check_return_
#else
#define FIMO_MUST_USE
#endif

/**
 * Stringifies the identifier, for internal use.
 *
 * @param a identifier
 */
#define FIMO_STRINGIFY_(a) #a

/**
 * Stringifies the identifier.
 *
 * @param a identifier
 */
#define FIMO_STRINGIFY(a) FIMO_STRINGIFY_(a)

/**
 * Concatenates the two identifiers, for internal use.
 *
 * @param a first identifier
 * @param b second identifier
 */
#define FIMO_CONCAT_(a, b) a##b

/**
 * Concatenates the two identifiers.
 *
 * @param a first identifier
 * @param b second identifier
 */
#define FIMO_CONCAT(a, b) FIMO_CONCAT_(a, b)

/**
 * Internal macro for generating unique identifiers.
 */
#ifdef __COUNTER__
#define FIMO_UNIQUE_() __COUNTER__
#else
#define FIMO_UNIQUE_() __LINE__
#endif // __COUNTER__

/**
 * Generates an unique identifier with the given prefix.
 *
 * @param name prefix
 */
#define FIMO_VAR(name) FIMO_CONCAT(name, FIMO_UNIQUE_())

/**
 * Write a pointer to the structure containing the member into a variable,
 * for internal use.
 *
 * @param ptr pointer to the member
 * @param type type of the structure
 * @param member name of the member within the struct
 * @param out identifier of the result
 * @param tmp identifier for the tmp variable
 */
#define FIMO_CONTAINER_OF_(ptr, type, member, out, tmp) \
    do {                                                \
        char* tmp = (char*)(ptr);                       \
        out = ((type*)(tmp - offsetof(type, member)));  \
    } while (0)

/**
 * Write a pointer to the structure containing the member into a variable.
 *
 * @param ptr pointer to the member
 * @param type type of the structure
 * @param member name of the member within the struct
 * @param out identifier of the result
 */
#define FIMO_CONTAINER_OF(ptr, type, member, out) FIMO_CONTAINER_OF_(ptr, type, member, out, FIMO_VAR(_tmp_ptr))

/**
 * 8-bit integer.
 */
typedef int8_t FimoI8;

/**
 * 16-bit integer.
 */
typedef int16_t FimoI16;

/**
 * 32-bit integer.
 */
typedef int32_t FimoI32;

/**
 * 64-bit integer.
 */
typedef int64_t FimoI64;

/**
 * 8-bit unsigned integer.
 */
typedef uint8_t FimoU8;

/**
 * 16-bit unsigned integer.
 */
typedef uint16_t FimoU16;

/**
 * 32-bit unsigned integer.
 */
typedef uint32_t FimoU32;

/**
 * 64-bit unsigned integer.
 */
typedef uint64_t FimoU64;

/**
 * Returns the next power of two.
 *
 * If `v` is already a power of two, the value remains unchanged.
 * A value of `0` is rounded up to `1`.
 *
 * @param v value
 * @return Next power of two.
 */
static inline FimoU8 fimo_next_power_of_two_u8(FimoU8 v)
{
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    return v;
}

/**
 * Returns the next power of two.
 *
 * If `v` is already a power of two, the value remains unchanged.
 * A value of `0` is rounded up to `1`.
 *
 * @param v value
 * @return Next power of two.
 */
static inline FimoU16 fimo_next_power_of_two_u16(FimoU16 v)
{
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    return v;
}

/**
 * Returns the next power of two.
 *
 * If `v` is already a power of two, the value remains unchanged.
 * A value of `0` is rounded up to `1`.
 *
 * @param v value
 * @return Next power of two.
 */
static inline FimoU32 fimo_next_power_of_two_u32(FimoU32 v)
{
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    return v;
}

/**
 * Returns the next power of two.
 *
 * If `v` is already a power of two, the value remains unchanged.
 * A value of `0` is rounded up to `1`.
 *
 * @param v value
 * @return Next power of two.
 */
static inline FimoU64 fimo_next_power_of_two_u64(FimoU64 v)
{
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v |= v >> 32;
    v++;
    return v;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static inline FimoU8 fimo_saturating_add_u8(FimoU8 a, FimoU8 b)
{
    return (a > UINT8_MAX - b) ? UINT8_MAX : a + b;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static inline FimoU16 fimo_saturating_add_u16(FimoU16 a, FimoU16 b)
{
    return (a > UINT16_MAX - b) ? UINT16_MAX : a + b;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static inline FimoU32 fimo_saturating_add_u32(FimoU32 a, FimoU32 b)
{
    return (a > UINT32_MAX - b) ? UINT32_MAX : a + b;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static inline FimoU64 fimo_saturating_add_u64(FimoU64 a, FimoU64 b)
{
    return (a > UINT64_MAX - b) ? UINT64_MAX : a + b;
}

#endif // FIMO_UTILS_H
