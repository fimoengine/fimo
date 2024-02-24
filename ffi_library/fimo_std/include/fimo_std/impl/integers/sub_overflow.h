#ifndef FIMO_IMPL_INTEGERS_SUB_OVERFLOW_H
#define FIMO_IMPL_INTEGERS_SUB_OVERFLOW_H

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
// Integer Intrinsics: Sub With Overflow detection
///////////////////////////////////////////////////////////////////////////////

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i8(FimoI8 a, FimoI8 b, FimoI8* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _sub_overflow_i8(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i16(FimoI16 a, FimoI16 b, FimoI16* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _sub_overflow_i16(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i32(FimoI32 a, FimoI32 b, FimoI32* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _sub_overflow_i32(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i64(FimoI64 a, FimoI64 b, FimoI64* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#elif defined(_MSC_VER) && defined(_M_X64)
    return _sub_overflow_i64(0, a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_isize(FimoISize a, FimoISize b, FimoISize* c)
{
    return FIMO_ISIZE_SWITCH_(fimo_impl_sub_overflow)(a, b, c);
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_intptr(FimoIntPtr a, FimoIntPtr b, FimoIntPtr* c)
{
    return FIMO_INTPTR_SWITCH_(fimo_impl_sub_overflow)(a, b, c);
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u8(FimoU8 a, FimoU8 b, FimoU8* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#else
    *c = a - b;
    return (a < b);
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u16(FimoU16 a, FimoU16 b, FimoU16* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#else
    *c = a - b;
    return (a < b);
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u32(FimoU32 a, FimoU32 b, FimoU32* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#else
    *c = a - b;
    return (a < b);
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u64(FimoU64 a, FimoU64 b, FimoU64* c)
{
#if FIMO_HAS_BUILTIN(__builtin_sub_overflow)
    return __builtin_sub_overflow(a, b, c);
#else
    *c = a - b;
    return (a < b);
#endif
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_usize(FimoUSize a, FimoUSize b, FimoUSize* c)
{
    return FIMO_USIZE_SWITCH_(fimo_impl_sub_overflow)(a, b, c);
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_uintptr(FimoUIntPtr a, FimoUIntPtr b, FimoUIntPtr* c)
{
    return FIMO_UINTPTR_SWITCH_(fimo_impl_sub_overflow)(a, b, c);
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_SUB_OVERFLOW_H