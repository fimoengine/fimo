#ifndef FIMO_IMPL_INTEGERS_ADD_OVERFLOW_H
#define FIMO_IMPL_INTEGERS_ADD_OVERFLOW_H

#include <fimo_std/impl/integers/integers_base.h>
#include <fimo_std/impl/macros/has_builtin.h>
#include <fimo_std/impl/macros/inline.h>

#include <stdbool.h>

#if defined(_WIN32) || defined(WIN32)
#include <intrin.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

///////////////////////////////////////////////////////////////////////////////
// Integer Intrinsics: Add With Overflow detection
///////////////////////////////////////////////////////////////////////////////

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_i8(FimoI8 a, FimoI8 b, FimoI8* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _add_overflow_i8(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_i16(FimoI16 a, FimoI16 b, FimoI16* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _add_overflow_i16(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_i32(FimoI32 a, FimoI32 b, FimoI32* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _add_overflow_i32(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_i64(FimoI64 a, FimoI64 b, FimoI64* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#elif defined(_MSC_VER) && defined(_M_X64)
    return _add_overflow_i64(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_isize(FimoISize a, FimoISize b, FimoISize* c)
{
    FIMO_ISIZE_UNDERLYING_ c_;
    bool tmp = FIMO_ISIZE_SWITCH_(fimo_impl_add_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_intptr(FimoIntPtr a, FimoIntPtr b, FimoIntPtr* c)
{
    FIMO_INTPTR_UNDERLYING_ c_;
    bool tmp = FIMO_INTPTR_SWITCH_(fimo_impl_add_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_u8(FimoU8 a, FimoU8 b, FimoU8* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#else
    *c = a + b;
    return (a > FIMO_U8_MAX - b);
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_u16(FimoU16 a, FimoU16 b, FimoU16* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#else
    *c = a + b;
    return (a > FIMO_U16_MAX - b);
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_u32(FimoU32 a, FimoU32 b, FimoU32* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#else
    *c = a + b;
    return (a > FIMO_U32_MAX - b);
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_u64(FimoU64 a, FimoU64 b, FimoU64* c)
{
#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
    return __builtin_add_overflow(a, b, c);
#else
    *c = a + b;
    return (a > FIMO_U64_MAX - b);
#endif
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_usize(FimoUSize a, FimoUSize b, FimoUSize* c)
{
    FIMO_USIZE_UNDERLYING_ c_;
    bool tmp = FIMO_USIZE_SWITCH_(fimo_impl_add_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

/**
 * Performs a wrapping addition of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_add_overflow_uintptr(FimoUIntPtr a, FimoUIntPtr b, FimoUIntPtr* c)
{
    FIMO_UINTPTR_UNDERLYING_ c_;
    bool tmp = FIMO_UINTPTR_SWITCH_(fimo_impl_add_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_ADD_OVERFLOW_H
