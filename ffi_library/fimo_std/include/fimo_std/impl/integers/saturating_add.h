#ifndef FIMO_IMPL_INTEGERS_SATURATING_ADD_H
#define FIMO_IMPL_INTEGERS_SATURATING_ADD_H

#include <fimo_std/impl/integers/add_overflow.h>
#include <fimo_std/impl/integers/integers_base.h>
#include <fimo_std/impl/macros/inline.h>

#ifdef __cplusplus
extern "C" {
#endif

///////////////////////////////////////////////////////////////////////////////
// Integer Intrinsics: Saturating Add
///////////////////////////////////////////////////////////////////////////////

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoI8 fimo_impl_saturating_add_i8(const FimoI8 a, const FimoI8 b) {
    FimoI8 res;
    const bool overflow = fimo_impl_add_overflow_i8(a, b, &res);
    if (overflow) {
        return (b < 0) ? FIMO_I8_MIN : FIMO_I8_MAX;
    }
    return res;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoI16 fimo_impl_saturating_add_i16(const FimoI16 a, const FimoI16 b) {
    FimoI16 res;
    const bool overflow = fimo_impl_add_overflow_i16(a, b, &res);
    if (overflow) {
        return (b < 0) ? FIMO_I16_MIN : FIMO_I16_MAX;
    }
    return res;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoI32 fimo_impl_saturating_add_i32(const FimoI32 a, const FimoI32 b) {
    FimoI32 res;
    const bool overflow = fimo_impl_add_overflow_i32(a, b, &res);
    if (overflow) {
        return (b < 0) ? FIMO_I32_MIN : FIMO_I32_MAX;
    }
    return res;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoI64 fimo_impl_saturating_add_i64(const FimoI64 a, const FimoI64 b) {
    FimoI64 res;
    const bool overflow = fimo_impl_add_overflow_i64(a, b, &res);
    if (overflow) {
        return (b < 0) ? FIMO_I64_MIN : FIMO_I64_MAX;
    }
    return res;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoISize fimo_impl_saturating_add_isize(const FimoISize a, const FimoISize b) {
    return FIMO_ISIZE_SWITCH_(fimo_impl_saturating_add)(a, b);
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoIntPtr fimo_impl_saturating_add_intptr(const FimoIntPtr a, const FimoIntPtr b) {
    return FIMO_INTPTR_SWITCH_(fimo_impl_saturating_add)(a, b);
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoU8 fimo_impl_saturating_add_u8(const FimoU8 a, const FimoU8 b) {
    return (a > FIMO_U8_MAX - b) ? FIMO_U8_MAX : a + b;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoU16 fimo_impl_saturating_add_u16(const FimoU16 a, const FimoU16 b) {
    return (a > FIMO_U16_MAX - b) ? FIMO_U16_MAX : a + b;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_saturating_add_u32(const FimoU32 a, const FimoU32 b) {
    return (a > FIMO_U32_MAX - b) ? FIMO_U32_MAX : a + b;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoU64 fimo_impl_saturating_add_u64(const FimoU64 a, const FimoU64 b) {
    return (a > FIMO_U64_MAX - b) ? FIMO_U64_MAX : a + b;
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoUSize fimo_impl_saturating_add_usize(const FimoUSize a, const FimoUSize b) {
    return FIMO_USIZE_SWITCH_(fimo_impl_saturating_add)(a, b);
}

/**
 * Performs a saturating addition of two integers.
 *
 * @param a first integer
 * @param b second integer
 *
 * @return Addition.
 */
static FIMO_INLINE_ALWAYS FimoUIntPtr fimo_impl_saturating_add_uintptr(const FimoUIntPtr a, const FimoUIntPtr b) {
    return FIMO_UINTPTR_SWITCH_(fimo_impl_saturating_add)(a, b);
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_SATURATING_ADD_H
