#ifndef FIMO_IMPL_INTEGERS_MUL_OVERFLOW_H
#define FIMO_IMPL_INTEGERS_MUL_OVERFLOW_H

#include <fimo_std/impl/integers/integers_base.h>
#include <fimo_std/impl/macros/has_builtin.h>
#include <fimo_std/impl/macros/inline.h>

#if defined(_WIN32) || defined(WIN32)
#include <intrin.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

///////////////////////////////////////////////////////////////////////////////
// Integer Intrinsics: Mul With Overflow detection
///////////////////////////////////////////////////////////////////////////////

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_i8(FimoI8 a, FimoI8 b, FimoI8* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    FimoI16 res;
    bool overflow = _mul_full_overflow_i8(a, b, &res);
    *c = (FimoI8)res;
    return overflow;
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_i16(FimoI16 a, FimoI16 b, FimoI16* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _mul_overflow_i16(a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_i32(FimoI32 a, FimoI32 b, FimoI32* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return _mul_overflow_i32(a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_i64(FimoI64 a, FimoI64 b, FimoI64* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && defined(_M_X64)
    return _mul_overflow_i64(a, b, c);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_isize(FimoISize a, FimoISize b, FimoISize* c)
{
    return FIMO_ISIZE_SWITCH_(fimo_impl_mul_overflow)(a, b, c);
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_intptr(FimoIntPtr a, FimoIntPtr b, FimoIntPtr* c)
{
    return FIMO_INTPTR_SWITCH_(fimo_impl_mul_overflow)(a, b, c);
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_u8(FimoU8 a, FimoU8 b, FimoU8* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    FimoU16 res;
    bool overflow = _mul_full_overflow_u8(a, b, &res);
    *c = (FimoU8)res;
    return overflow;
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_u16(FimoU16 a, FimoU16 b, FimoU16* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    FimoU16 high;
    return _mul_full_overflow_u16(a, b, c, &high);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_u32(FimoU32 a, FimoU32 b, FimoU32* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    FimoU32 high;
    return _mul_full_overflow_u32(a, b, c, &high);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_u64(FimoU64 a, FimoU64 b, FimoU64* c)
{
#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
    return __builtin_mul_overflow(a, b, c);
#elif defined(_MSC_VER) && defined(_M_X64)
    FimoU64 high;
    return _mul_full_overflow_u64(a, b, c, &high);
#else
#error "Compiler not supported"
#endif
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_usize(FimoUSize a, FimoUSize b, FimoUSize* c)
{
    return FIMO_USIZE_SWITCH_(fimo_impl_mul_overflow)(a, b, c);
}

/**
 * Performs a wrapping multiplication of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Multiplication.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_mul_overflow_uintptr(FimoUIntPtr a, FimoUIntPtr b, FimoUIntPtr* c)
{
    return FIMO_UINTPTR_SWITCH_(fimo_impl_mul_overflow)(a, b, c);
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_MUL_OVERFLOW_H