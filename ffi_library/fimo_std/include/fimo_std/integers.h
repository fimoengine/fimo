#ifndef FIMO_INTEGERS_H
#define FIMO_INTEGERS_H

#include <limits.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include <fimo_std/impl/integers/add_overflow.h>
#include <fimo_std/impl/integers/count_ones.h>
#include <fimo_std/impl/integers/integers_base.h>
#include <fimo_std/impl/integers/mul_overflow.h>
#include <fimo_std/impl/integers/next_power_of_two.h>
#include <fimo_std/impl/integers/saturating_add.h>
#include <fimo_std/impl/integers/saturating_sub.h>
#include <fimo_std/impl/integers/sub_overflow.h>

#include <fimo_std/impl/macros/inline.h>

#ifdef __cplusplus
extern "C" {
#endif

///////////////////////////////////////////////////////////////////////////////
// Integer Macro
///////////////////////////////////////////////////////////////////////////////

#define FIMO_INT_MACROS_(T, SUFFIX, AFFIX, MIN, MAX)                                                                   \
    typedef struct FimoIntOverflowCheck##SUFFIX {                                                                      \
        T value;                                                                                                       \
        bool overflow;                                                                                                 \
    } FimoIntOverflowCheck##SUFFIX;                                                                                    \
    typedef struct FimoIntOption##SUFFIX {                                                                             \
        bool has_value;                                                                                                \
        union {                                                                                                        \
            T value;                                                                                                   \
            FimoU8 empty_;                                                                                             \
        } data;                                                                                                        \
    } FimoIntOption##SUFFIX;                                                                                           \
                                                                                                                       \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_add(T a, T b) {                  \
        T value;                                                                                                       \
        bool overflow = fimo_impl_add_overflow_##AFFIX(a, b, &value);                                                  \
        return (FimoIntOverflowCheck##SUFFIX){.value = value, .overflow = overflow};                                   \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_sub(T a, T b) {                  \
        T value;                                                                                                       \
        bool overflow = fimo_impl_sub_overflow_##AFFIX(a, b, &value);                                                  \
        return (FimoIntOverflowCheck##SUFFIX){.value = value, .overflow = overflow};                                   \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_mul(T a, T b) {                  \
        T value;                                                                                                       \
        bool overflow = fimo_impl_mul_overflow_##AFFIX(a, b, &value);                                                  \
        return (FimoIntOverflowCheck##SUFFIX){.value = value, .overflow = overflow};                                   \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_div(T a, T b) {                  \
        if ((a == MIN) && (b == -1)) {                                                                                 \
            return (FimoIntOverflowCheck##SUFFIX){.value = a, .overflow = true};                                       \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOverflowCheck##SUFFIX){.value = a / b, .overflow = false};                                  \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_add(T a, T b) {                             \
        FimoIntOverflowCheck##SUFFIX tmp = fimo_##AFFIX##_overflowing_add(a, b);                                       \
        if (tmp.overflow) {                                                                                            \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = tmp.value}};                           \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_sub(T a, T b) {                             \
        FimoIntOverflowCheck##SUFFIX tmp = fimo_##AFFIX##_overflowing_sub(a, b);                                       \
        if (tmp.overflow) {                                                                                            \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = tmp.value}};                           \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_mul(T a, T b) {                             \
        FimoIntOverflowCheck##SUFFIX tmp = fimo_##AFFIX##_overflowing_mul(a, b);                                       \
        if (tmp.overflow) {                                                                                            \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = tmp.value}};                           \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_div(T a, T b) {                             \
        if ((b == 0) || ((a == MIN) && (b == -1))) {                                                                   \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = a / b}};                               \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_add(T a, T b) {                                              \
        return fimo_impl_saturating_add_##AFFIX(a, b);                                                                 \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_sub(T a, T b) {                                              \
        return fimo_impl_saturating_sub_##AFFIX(a, b);                                                                 \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_mul(T a, T b) {                                              \
        FimoIntOption##SUFFIX tmp = fimo_##AFFIX##_checked_mul(a, b);                                                  \
        if (tmp.has_value) {                                                                                           \
            return tmp.data.value;                                                                                     \
        }                                                                                                              \
        else {                                                                                                         \
            return (a < 0) == (b < 0) ? MAX : MIN;                                                                     \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_div(T a, T b) {                                              \
        FimoIntOverflowCheck##SUFFIX tmp = fimo_##AFFIX##_overflowing_div(a, b);                                       \
        if (!tmp.overflow) {                                                                                           \
            return tmp.value;                                                                                          \
        }                                                                                                              \
        else {                                                                                                         \
            return MAX;                                                                                                \
        }                                                                                                              \
    }

#define FIMO_UINT_MACROS_(T, SUFFIX, AFFIX, MIN, MAX)                                                                  \
    typedef struct FimoIntOverflowCheck##SUFFIX {                                                                      \
        T value;                                                                                                       \
        bool overflow;                                                                                                 \
    } FimoIntOverflowCheck##SUFFIX;                                                                                    \
    typedef struct FimoIntOption##SUFFIX {                                                                             \
        bool has_value;                                                                                                \
        union {                                                                                                        \
            T value;                                                                                                   \
            FimoU8 empty_;                                                                                             \
        } data;                                                                                                        \
    } FimoIntOption##SUFFIX;                                                                                           \
                                                                                                                       \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_add(T a, T b) {                  \
        T value;                                                                                                       \
        bool overflow = fimo_impl_add_overflow_##AFFIX(a, b, &value);                                                  \
        return (FimoIntOverflowCheck##SUFFIX){.value = value, .overflow = overflow};                                   \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_sub(T a, T b) {                  \
        T value;                                                                                                       \
        bool overflow = fimo_impl_sub_overflow_##AFFIX(a, b, &value);                                                  \
        return (FimoIntOverflowCheck##SUFFIX){.value = value, .overflow = overflow};                                   \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_mul(T a, T b) {                  \
        T value;                                                                                                       \
        bool overflow = fimo_impl_mul_overflow_##AFFIX(a, b, &value);                                                  \
        return (FimoIntOverflowCheck##SUFFIX){.value = value, .overflow = overflow};                                   \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOverflowCheck##SUFFIX fimo_##AFFIX##_overflowing_div(T a, T b) {                  \
        return (FimoIntOverflowCheck##SUFFIX){.value = a / b, .overflow = false};                                      \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_add(T a, T b) {                             \
        FimoIntOverflowCheck##SUFFIX tmp = fimo_##AFFIX##_overflowing_add(a, b);                                       \
        if (tmp.overflow) {                                                                                            \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = tmp.value}};                           \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_sub(T a, T b) {                             \
        FimoIntOverflowCheck##SUFFIX tmp = fimo_##AFFIX##_overflowing_sub(a, b);                                       \
        if (tmp.overflow) {                                                                                            \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = tmp.value}};                           \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_mul(T a, T b) {                             \
        FimoIntOverflowCheck##SUFFIX tmp = fimo_##AFFIX##_overflowing_mul(a, b);                                       \
        if (tmp.overflow) {                                                                                            \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = tmp.value}};                           \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoIntOption##SUFFIX fimo_##AFFIX##_checked_div(T a, T b) {                             \
        if (b == 0) {                                                                                                  \
            return (FimoIntOption##SUFFIX){.has_value = false, .data = {.empty_ = 0}};                                 \
        }                                                                                                              \
        else {                                                                                                         \
            return (FimoIntOption##SUFFIX){.has_value = true, .data = {.value = a / b}};                               \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_add(T a, T b) {                                              \
        return fimo_impl_saturating_add_##AFFIX(a, b);                                                                 \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_sub(T a, T b) {                                              \
        return fimo_impl_saturating_sub_##AFFIX(a, b);                                                                 \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_mul(T a, T b) {                                              \
        FimoIntOption##SUFFIX tmp = fimo_##AFFIX##_checked_mul(a, b);                                                  \
        if (tmp.has_value) {                                                                                           \
            return tmp.data.value;                                                                                     \
        }                                                                                                              \
        else {                                                                                                         \
            return MAX;                                                                                                \
        }                                                                                                              \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_saturating_div(T a, T b) { return a / b; }                              \
    static FIMO_INLINE_ALWAYS T fimo_##AFFIX##_next_power_of_two(T v) {                                                \
        return fimo_impl_next_power_of_two_##AFFIX(v);                                                                 \
    }                                                                                                                  \
    static FIMO_INLINE_ALWAYS FimoU32 fimo_##AFFIX##_count_ones(T v) { return fimo_impl_count_ones_##AFFIX(v); }       \
    static FIMO_INLINE_ALWAYS FimoU32 fimo_##AFFIX##_count_zeros(T v) { return fimo_##AFFIX##_count_ones((T)~v); }

FIMO_INT_MACROS_(FimoI8, I8, i8, FIMO_I8_MIN, FIMO_I8_MAX)
FIMO_INT_MACROS_(FimoI16, I16, i16, FIMO_I16_MIN, FIMO_I16_MAX)
FIMO_INT_MACROS_(FimoI32, I32, i32, FIMO_I32_MIN, FIMO_I32_MAX)
FIMO_INT_MACROS_(FimoI64, I64, i64, FIMO_I64_MIN, FIMO_I64_MAX)
FIMO_INT_MACROS_(FimoISize, ISize, isize, FIMO_ISIZE_MIN, FIMO_ISIZE_MAX)
FIMO_INT_MACROS_(FimoIntPtr, IntPtr, intptr, FIMO_INTPTR_MIN, FIMO_INTPTR_MAX)

FIMO_UINT_MACROS_(FimoU8, U8, u8, FIMO_U8_MIN, FIMO_U8_MAX)
FIMO_UINT_MACROS_(FimoU16, U16, u16, FIMO_U16_MIN, FIMO_U16_MAX)
FIMO_UINT_MACROS_(FimoU32, U32, u32, FIMO_U32_MIN, FIMO_U32_MAX)
FIMO_UINT_MACROS_(FimoU64, U64, u64, FIMO_U64_MIN, FIMO_U64_MAX)
FIMO_UINT_MACROS_(FimoUSize, USize, usize, FIMO_USIZE_MIN, FIMO_USIZE_MAX)
FIMO_UINT_MACROS_(FimoUIntPtr, UIntPtr, uintptr, FIMO_UINTPTR_MIN, FIMO_UINTPTR_MAX)

#ifdef __cplusplus
}
#endif

#endif // FIMO_INTEGERS_H
