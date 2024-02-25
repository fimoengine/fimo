#ifndef FIMO_IMPL_INTEGERS_COUNT_ONES_H
#define FIMO_IMPL_INTEGERS_COUNT_ONES_H

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
// Integer Intrinsics: Next Power Of Two
///////////////////////////////////////////////////////////////////////////////

/**
 * Returns the number of ones in the binary representation of the number.
 *
 * @param v value
 * @return Ones count.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_count_ones_u8(FimoU8 v)
{
#if FIMO_HAS_BUILTIN(__builtin_popcount)
    _Static_assert(sizeof(FimoU8) <= sizeof(unsigned int), "Size mismatch");
    return __builtin_popcount(v);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return __popcnt16(v);
#elif defined(_MSC_VER) && (defined(_M_ARM) || defined(_M_ARM64))
    return _CountOneBits(v);
#else
#error "Compiler not supported"
#endif
}

/**
 * Returns the number of ones in the binary representation of the number.
 *
 * @param v value
 * @return Ones count.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_count_ones_u16(FimoU16 v)
{
#if FIMO_HAS_BUILTIN(__builtin_popcount)
    _Static_assert(sizeof(FimoU16) <= sizeof(unsigned int), "Size mismatch");
    return __builtin_popcount(v);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return __popcnt16(v);
#elif defined(_MSC_VER) && (defined(_M_ARM) || defined(_M_ARM64))
    return _CountOneBits(v);
#else
#error "Compiler not supported"
#endif
}

/**
 * Returns the number of ones in the binary representation of the number.
 *
 * @param v value
 * @return Ones count.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_count_ones_u32(FimoU32 v)
{
#if FIMO_HAS_BUILTIN(__builtin_popcount)
    _Static_assert(sizeof(FimoU32) <= sizeof(unsigned int), "Size mismatch");
    return __builtin_popcount(v);
#elif defined(_MSC_VER) && (defined(_M_IX86) || defined(_M_X64))
    return __popcnt(v);
#elif defined(_MSC_VER) && (defined(_M_ARM) || defined(_M_ARM64))
    return _CountOneBits(v);
#else
#error "Compiler not supported"
#endif
}

/**
 * Returns the number of ones in the binary representation of the number.
 *
 * @param v value
 * @return Ones count.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_count_ones_u64(FimoU64 v)
{
#if FIMO_HAS_BUILTIN(__builtin_popcountll)
    _Static_assert(sizeof(FimoU64) <= sizeof(unsigned long long), "Size mismatch");
    return __builtin_popcountll(v);
#elif defined(_MSC_VER) && defined(_M_X64)
    return (FimoU32)__popcnt64(v);
#elif defined(_MSC_VER) && (defined(_M_ARM) || defined(_M_ARM64))
    return _CountOneBits64(v);
#else
#error "Compiler not supported"
#endif
}

/**
 * Returns the number of ones in the binary representation of the number.
 *
 * @param v value
 * @return Ones count.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_count_ones_usize(FimoUSize v)
{
    return FIMO_USIZE_SWITCH_(fimo_impl_count_ones)(v);
}

/**
 * Returns the number of ones in the binary representation of the number.
 *
 * @param v value
 * @return Ones count.
 */
static FIMO_INLINE_ALWAYS FimoU32 fimo_impl_count_ones_uintptr(FimoUIntPtr v)
{
    return FIMO_UINTPTR_SWITCH_(fimo_impl_count_ones)(v);
}

#ifdef __cplusplus
}
#endif

#endif // !FIMO_IMPL_INTEGERS_COUNT_ONES_H
