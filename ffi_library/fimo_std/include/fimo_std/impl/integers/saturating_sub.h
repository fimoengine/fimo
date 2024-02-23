#ifndef FIMO_IMPL_INTEGERS_SATURATING_SUB_H
#define FIMO_IMPL_INTEGERS_SATURATING_SUB_H

#include <fimo_std/impl/integers/integers_base.h>
#include <fimo_std/impl/integers/sub_overflow.h>
#include <fimo_std/impl/macros/has_builtin.h>
#include <fimo_std/impl/macros/inline.h>

#if defined(_WIN32) || defined(WIN32)
#include <intrin.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

///////////////////////////////////////////////////////////////////////////////
// Integer Intrinsics: Saturating Sub
///////////////////////////////////////////////////////////////////////////////

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoI8 fimo_impl_saturating_sub_i8(FimoI8 a, FimoI8 b)
{
    FimoI8 res;
    bool overflow = fimo_impl_sub_overflow_i8(a, b, &res);
    if (overflow) {
        return (b > 0) ? FIMO_I8_MIN : FIMO_I8_MAX;
    }
    return res;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoI16 fimo_impl_saturating_sub_i16(FimoI16 a, FimoI16 b)
{
    FimoI16 res;
    bool overflow = fimo_impl_sub_overflow_i16(a, b, &res);
    if (overflow) {
        return (b > 0) ? FIMO_I16_MIN : FIMO_I16_MAX;
    }
    return res;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoI32 fimo_impl_saturating_sub_i32(FimoI32 a, FimoI32 b)
{
    FimoI32 res;
    bool overflow = fimo_impl_sub_overflow_i32(a, b, &res);
    if (overflow) {
        return (b > 0) ? FIMO_I32_MIN : FIMO_I32_MAX;
    }
    return res;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoI64 fimo_impl_saturating_sub_i64(FimoI64 a, FimoI64 b)
{
    FimoI64 res;
    bool overflow = fimo_impl_sub_overflow_i64(a, b, &res);
    if (overflow) {
        return (b > 0) ? FIMO_I64_MIN : FIMO_I64_MAX;
    }
    return res;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoISize fimo_impl_saturating_sub_isize(FimoISize a, FimoISize b)
{
    return FIMO_ISIZE_SWITCH_(fimo_impl_saturating_sub, a, b);
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoIntPtr fimo_impl_saturating_sub_intptr(FimoIntPtr a, FimoIntPtr b)
{
    return FIMO_INTPTR_SWITCH_(fimo_impl_saturating_sub, a, b);
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoU8 fimo_impl_saturating_sub_u8(FimoU8 a, FimoU8 b)
{
    return (a < b) ? FIMO_U8_MIN : a - b;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoU16 fimo_impl_saturating_sub_u16(FimoU16 a, FimoU16 b)
{
    return (a < b) ? FIMO_U16_MIN : a - b;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_saturating_sub_u32(FimoU32 a, FimoU32 b)
{
    return (a < b) ? FIMO_U32_MIN : a - b;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoU64 fimo_impl_saturating_sub_u64(FimoU64 a, FimoU64 b)
{
    return (a < b) ? FIMO_U64_MIN : a - b;
}

/**
 * Performs a saturating subtraction of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Subtraction.
 */
static FIMO_INLINE_ALWAYS FimoUSize fimo_impl_saturating_sub_usize(FimoUSize a, FimoUSize b)
{
    return FIMO_USIZE_SWITCH_(fimo_impl_saturating_sub, a, b);
}

static FIMO_INLINE_ALWAYS FimoUIntPtr fimo_impl_saturating_sub_uintptr(FimoUIntPtr a, FimoUIntPtr b)
{
    return FIMO_UINTPTR_SWITCH_(fimo_impl_saturating_sub, a, b);
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_SATURATING_SUB_H