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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i8(const FimoI8 a, const FimoI8 b, FimoI8 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i16(const FimoI16 a, const FimoI16 b, FimoI16 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i32(const FimoI32 a, const FimoI32 b, FimoI32 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_i64(const FimoI64 a, const FimoI64 b, FimoI64 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_isize(const FimoISize a, const FimoISize b, FimoISize *c) {
    FIMO_DEBUG_ASSERT(c)
    FIMO_ISIZE_UNDERLYING_ c_;
    const bool tmp = FIMO_ISIZE_SWITCH_(fimo_impl_sub_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_intptr(const FimoIntPtr a, const FimoIntPtr b, FimoIntPtr *c) {
    FIMO_DEBUG_ASSERT(c)
    FIMO_INTPTR_UNDERLYING_ c_;
    const bool tmp = FIMO_INTPTR_SWITCH_(fimo_impl_sub_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u8(const FimoU8 a, const FimoU8 b, FimoU8 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u16(const FimoU16 a, const FimoU16 b, FimoU16 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u32(const FimoU32 a, const FimoU32 b, FimoU32 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_u64(const FimoU64 a, const FimoU64 b, FimoU64 *c) {
    FIMO_DEBUG_ASSERT(c)
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
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_usize(const FimoUSize a, const FimoUSize b, FimoUSize *c) {
    FIMO_DEBUG_ASSERT(c)
    FIMO_USIZE_UNDERLYING_ c_;
    const bool tmp = FIMO_USIZE_SWITCH_(fimo_impl_sub_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

/**
 * Performs a wrapping subtraction of two integers with overflow detection.
 *
 * @param a first integer
 * @param b second integer
 * @param c result
 *
 * @return Overflow.
 */
static FIMO_INLINE_ALWAYS bool fimo_impl_sub_overflow_uintptr(const FimoUIntPtr a, const FimoUIntPtr b,
                                                              FimoUIntPtr *c) {
    FIMO_DEBUG_ASSERT(c)
    FIMO_UINTPTR_UNDERLYING_ c_;
    const bool tmp = FIMO_UINTPTR_SWITCH_(fimo_impl_sub_overflow)(a, b, &c_);
    *c = c_;
    return tmp;
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_SUB_OVERFLOW_H
