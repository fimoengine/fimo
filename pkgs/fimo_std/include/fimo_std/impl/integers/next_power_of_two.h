#ifndef FIMO_IMPL_INTEGERS_NEXT_POWER_OF_TWO_H
#define FIMO_IMPL_INTEGERS_NEXT_POWER_OF_TWO_H

#include <fimo_std/impl/integers/integers_base.h>
#include <fimo_std/impl/macros/inline.h>

#ifdef __cplusplus
extern "C" {
#endif

///////////////////////////////////////////////////////////////////////////////
// Integer Intrinsics: Next Power Of Two
///////////////////////////////////////////////////////////////////////////////

/**
 * Returns the next power of two.
 *
 * If `v` is already a power of two, the value remains unchanged.
 * A value of `0` is rounded up to `1`.
 *
 * @param v value
 * @return Next power of two.
 */
static FIMO_INLINE_ALWAYS FimoU8 fimo_impl_next_power_of_two_u8(FimoU8 v) {
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
static FIMO_INLINE_ALWAYS FimoU16 fimo_impl_next_power_of_two_u16(FimoU16 v) {
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
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_next_power_of_two_u32(FimoU32 v) {
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
static FIMO_INLINE_ALWAYS FimoU64 fimo_impl_next_power_of_two_u64(FimoU64 v) {
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
 * Returns the next power of two.
 *
 * If `v` is already a power of two, the value remains unchanged.
 * A value of `0` is rounded up to `1`.
 *
 * @param v value
 * @return Next power of two.
 */
static FIMO_INLINE_ALWAYS FimoUSize fimo_impl_next_power_of_two_usize(const FimoUSize v) {
    return FIMO_USIZE_SWITCH_(fimo_impl_next_power_of_two)(v);
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
static FIMO_INLINE_ALWAYS FimoUIntPtr fimo_impl_next_power_of_two_uintptr(const FimoUIntPtr v) {
    return FIMO_UINTPTR_SWITCH_(fimo_impl_next_power_of_two)(v);
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_NEXT_POWER_OF_TWO_H
