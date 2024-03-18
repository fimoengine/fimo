#ifndef FIMO_IMPL_INTEGERS_INTEGERS_BASE_H
#define FIMO_IMPL_INTEGERS_INTEGERS_BASE_H

#include <limits.h>
#include <stddef.h>
#include <stdint.h>

///////////////////////////////////////////////////////////////////////////////
// Integer types
///////////////////////////////////////////////////////////////////////////////

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
 * Signed integer type resulting from subtracting two pointers.
 */
typedef ptrdiff_t FimoISize;

/**
 * Signed integer type capable of containing any pointer.
 */
typedef intptr_t FimoIntPtr;

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
 * Unsigned integer guaranteed to hold any array index.
 */
typedef size_t FimoUSize;

/**
 * Unsigned integer type capable of containing any pointer.
 */
typedef uintptr_t FimoUIntPtr;

///////////////////////////////////////////////////////////////////////////////
// Integer Min Macros
///////////////////////////////////////////////////////////////////////////////

/**
 * Minimum value of an `FimoI8` integer.
 */
#define FIMO_I8_MIN INT8_MIN

/**
 * Minimum value of an `FimoI16` integer.
 */
#define FIMO_I16_MIN INT16_MIN

/**
 * Minimum value of an `FimoI32` integer.
 */
#define FIMO_I32_MIN INT32_MIN

/**
 * Minimum value of an `FimoI64` integer.
 */
#define FIMO_I64_MIN INT64_MIN

/**
 * Minimum value of an `FimoISize` integer.
 */
#define FIMO_ISIZE_MIN PTRDIFF_MIN

/**
 * Minimum value of an `FimoIntPtr` integer.
 */
#define FIMO_INTPTR_MIN INTPTR_MIN

/**
 * Minimum value of an `FimoU8` integer.
 */
#define FIMO_U8_MIN (FimoU8)0

/**
 * Minimum value of an `FimoU16` integer.
 */
#define FIMO_U16_MIN (FimoU16)0

/**
 * Minimum value of an `FimoU32` integer.
 */
#define FIMO_U32_MIN (FimoU32)0

/**
 * Minimum value of an `FimoI64` integer.
 */
#define FIMO_U64_MIN (FimoU64)0

/**
 * Minimum value of an `FimoUSize` integer.
 */
#define FIMO_USIZE_MIN (FimoUSize)0

/**
 * Minimum value of an `FimoUIntPtr` integer.
 */
#define FIMO_UINTPTR_MIN (FimoUIntPtr)0

///////////////////////////////////////////////////////////////////////////////
// Integer Max Macros
///////////////////////////////////////////////////////////////////////////////

/**
 * Maximum value of an `FimoI8` integer.
 */
#define FIMO_I8_MAX INT8_MAX

/**
 * Maximum value of an `FimoI16` integer.
 */
#define FIMO_I16_MAX INT16_MAX

/**
 * Maximum value of an `FimoI32` integer.
 */
#define FIMO_I32_MAX INT32_MAX

/**
 * Maximum value of an `FimoI64` integer.
 */
#define FIMO_I64_MAX INT64_MAX

/**
 * Maximum value of an `FimoISize` integer.
 */
#define FIMO_ISIZE_MAX PTRDIFF_MAX

/**
 * Maximum value of an `FimoIntPtr` integer.
 */
#define FIMO_INTPTR_MAX INTPTR_MAX

/**
 * Maximum value of an `FimoU8` integer.
 */
#define FIMO_U8_MAX UINT8_MAX

/**
 * Maximum value of an `FimoU16` integer.
 */
#define FIMO_U16_MAX UINT16_MAX

/**
 * Maximum value of an `FimoU32` integer.
 */
#define FIMO_U32_MAX UINT32_MAX

/**
 * Maximum value of an `FimoI64` integer.
 */
#define FIMO_U64_MAX UINT64_MAX

/**
 * Maximum value of an `FimoUSize` integer.
 */
#define FIMO_USIZE_MAX SIZE_MAX

/**
 * Maximum value of an `FimoUIntPtr` integer.
 */
#define FIMO_UINTPTR_MAX UINTPTR_MAX

///////////////////////////////////////////////////////////////////////////////
// Integer Width Macros
///////////////////////////////////////////////////////////////////////////////

/**
 * Width of an `FimoI8` integer.
 */
#define FIMO_I8_WIDTH 8

/**
 * Width of an `FimoI16` integer.
 */
#define FIMO_I16_WIDTH 16

/**
 * Width of an `FimoI32` integer.
 */
#define FIMO_I32_WIDTH 32

/**
 * Width of an `FimoI64` integer.
 */
#define FIMO_I64_WIDTH 64

/**
 * Width of an `FimoISize` integer.
 */
#if FIMO_ISIZE_MIN == FIMO_I16_MIN && FIMO_ISIZE_MAX == FIMO_I16_MAX
#define FIMO_ISIZE_WIDTH FIMO_I16_WIDTH
#elif FIMO_ISIZE_MIN == FIMO_I32_MIN && FIMO_ISIZE_MAX == FIMO_I32_MAX
#define FIMO_ISIZE_WIDTH FIMO_I32_WIDTH
#elif FIMO_ISIZE_MIN == FIMO_I64_MIN && FIMO_ISIZE_MAX == FIMO_I64_MAX
#define FIMO_ISIZE_WIDTH FIMO_I64_WIDTH
#else
#error "Unsupported FimoISize size"
#endif

/**
 * Width of an `FimoIntPtr` integer.
 */
#if FIMO_INTPTR_MIN == FIMO_I16_MIN && FIMO_INTPTR_MAX == FIMO_I16_MAX
#define FIMO_INTPTR_WIDTH FIMO_I16_WIDTH
#elif FIMO_INTPTR_MIN == FIMO_I32_MIN && FIMO_INTPTR_MAX == FIMO_I32_MAX
#define FIMO_INTPTR_WIDTH FIMO_I32_WIDTH
#elif FIMO_INTPTR_MIN == FIMO_I64_MIN && FIMO_INTPTR_MAX == FIMO_I64_MAX
#define FIMO_INTPTR_WIDTH FIMO_I64_WIDTH
#else
#error "Unsupported FimoIntPtr size"
#endif

/**
 * Width of an `FimoU8` integer.
 */
#define FIMO_U8_WIDTH 8

/**
 * Width of an `FimoU16` integer.
 */
#define FIMO_U16_WIDTH 16

/**
 * Width of an `FimoU32` integer.
 */
#define FIMO_U32_WIDTH 32

/**
 * Width of an `FimoU64` integer.
 */
#define FIMO_U64_WIDTH 64

/**
 * Width of an `FimoUSize` integer.
 */
#if FIMO_USIZE_MAX == FIMO_U16_MAX
#define FIMO_USIZE_WIDTH FIMO_U16_WIDTH
#elif FIMO_USIZE_MAX == FIMO_U32_MAX
#define FIMO_USIZE_WIDTH FIMO_U32_WIDTH
#elif FIMO_USIZE_MAX == FIMO_U64_MAX
#define FIMO_USIZE_WIDTH FIMO_U64_WIDTH
#else
#error "Unsupported FimoUSize size"
#endif

/**
 * Width of an `FimoUIntPtr` integer.
 */
#if FIMO_UINTPTR_MAX == FIMO_U16_MAX
#define FIMO_UINTPTR_WIDTH FIMO_U16_WIDTH
#elif FIMO_UINTPTR_MAX == FIMO_U32_MAX
#define FIMO_UINTPTR_WIDTH FIMO_U32_WIDTH
#elif FIMO_UINTPTR_MAX == FIMO_U64_MAX
#define FIMO_UINTPTR_WIDTH FIMO_U64_WIDTH
#else
#error "Unsupported FimoUIntPtr size"
#endif

///////////////////////////////////////////////////////////////////////////////
// Private Utility Macros
///////////////////////////////////////////////////////////////////////////////

#ifdef __APPLE__
_Static_assert(sizeof(FimoISize) == 8, "Invalid FimoISize size");
_Static_assert(sizeof(FimoIntPtr) == 8, "Invalid FimoIntPtr size");
_Static_assert(sizeof(FimoUSize) == 8, "Invalid FimoUSize size");
_Static_assert(sizeof(FimoUIntPtr) == 8, "Invalid FimoUIntPtr size");

#define FIMO_ISIZE_UNDERLYING_ FimoI64
#define FIMO_INTPTR_UNDERLYING_ FimoI64
#define FIMO_USIZE_UNDERLYING_ FimoU64
#define FIMO_UINTPTR_UNDERLYING_ FimoU64

#define FIMO_ISIZE_SWITCH_(NAME) NAME##_i64
#define FIMO_INTPTR_SWITCH_(NAME) NAME##_i64
#define FIMO_USIZE_SWITCH_(NAME) NAME##_u64
#define FIMO_UINTPTR_SWITCH_(NAME) NAME##_u64
#elif defined(__cplusplus)
#include <type_traits>

template<typename I>
struct FimoUnderlying_ {
    using T = I;
    static_assert(false, "Unknown type");
};
template<>
struct FimoUnderlying_<FimoI8> {
    using T = FimoI8;
};
template<>
struct FimoUnderlying_<FimoI16> {
    using T = FimoI16;
};
template<>
struct FimoUnderlying_<FimoI32> {
    using T = FimoI32;
};
template<>
struct FimoUnderlying_<FimoI64> {
    using T = FimoI64;
};
template<>
struct FimoUnderlying_<FimoU8> {
    using T = FimoU8;
};
template<>
struct FimoUnderlying_<FimoU16> {
    using T = FimoU16;
};
template<>
struct FimoUnderlying_<FimoU32> {
    using T = FimoU32;
};
template<>
struct FimoUnderlying_<FimoU64> {
    using T = FimoU64;
};

#define FIMO_ISIZE_UNDERLYING_ FimoUnderlying_<FimoISize>::T
#define FIMO_INTPTR_UNDERLYING_ FimoUnderlying_<FimoIntPtr>::T
#define FIMO_USIZE_UNDERLYING_ FimoUnderlying_<FimoUSize>::T
#define FIMO_UINTPTR_UNDERLYING_ FimoUnderlying_<FimoUIntPtr>::T

#define FIMO_ISIZE_SWITCH_(NAME)                                                                                       \
    [](auto x) {                                                                                                       \
        if constexpr (std::is_same<FimoISize, FimoI8>::value) {                                                        \
            return [](auto... args) { return NAME##_i8(args...); };                                                    \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoISize, FimoI16>::value) {                                                  \
            return [](auto... args) { return NAME##_i16(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoISize, FimoI32>::value) {                                                  \
            return [](auto... args) { return NAME##_i32(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoISize, FimoI64>::value) {                                                  \
            return [](auto... args) { return NAME##_i64(args...); };                                                   \
        }                                                                                                              \
        else {                                                                                                         \
            static_assert(false, "Invalid FimoISize type");                                                            \
        }                                                                                                              \
    }()
#define FIMO_INTPTR_SWITCH_(NAME)                                                                                      \
    []() {                                                                                                             \
        if constexpr (std::is_same<FimoIntPtr, FimoI8>::value) {                                                       \
            return [](auto... args) { return NAME##_i8(args...); };                                                    \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoIntPtr, FimoI16>::value) {                                                 \
            return [](auto... args) { return NAME##_i16(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoIntPtr, FimoI32>::value) {                                                 \
            return [](auto... args) { return NAME##_i32(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoIntPtr, FimoI64>::value) {                                                 \
            return [](auto... args) { return NAME##_i64(args...); };                                                   \
        }                                                                                                              \
        else {                                                                                                         \
            static_assert(false, "Invalid FimoIntPtr type");                                                           \
        }                                                                                                              \
    }()
#define FIMO_USIZE_SWITCH_(NAME)                                                                                       \
    []() {                                                                                                             \
        if constexpr (std::is_same<FimoUSize, FimoU8>::value) {                                                        \
            return [](auto... args) { return NAME##_u8(args...); };                                                    \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoUSize, FimoU16>::value) {                                                  \
            return [](auto... args) { return NAME##_u16(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoUSize, FimoU32>::value) {                                                  \
            return [](auto... args) { return NAME##_u32(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoUSize, FimoU64>::value) {                                                  \
            return [](auto... args) { return NAME##_u64(args...); };                                                   \
        }                                                                                                              \
        else {                                                                                                         \
            static_assert(false, "Invalid FimoUSize type");                                                            \
        }                                                                                                              \
    }()
#define FIMO_INTPTR_SWITCH_(NAME)                                                                                      \
    []() {                                                                                                             \
        if constexpr (std::is_same<FimoUIntPtr, FimoU8>::value) {                                                      \
            return [](auto... args) { return NAME##_u8(args...); };                                                    \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoUIntPtr, FimoU16>::value) {                                                \
            return [](auto... args) { return NAME##_u16(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoUIntPtr, FimoU32>::value) {                                                \
            return [](auto... args) { return NAME##_u32(args...); };                                                   \
        }                                                                                                              \
        else if constexpr (std::is_same<FimoUIntPtr, FimoU64>::value) {                                                \
            return [](auto... args) { return NAME##_u64(args...); };                                                   \
        }                                                                                                              \
        else {                                                                                                         \
            static_assert(false, "Invalid FimoUIntPtr type");                                                          \
        }                                                                                                              \
    }()
#else
#if FIMO_ISIZE_WIDTH == 8
#define FIMO_ISIZE_UNDERLYING_ FimoI8
#elif FIMO_ISIZE_WIDTH == 16
#define FIMO_ISIZE_UNDERLYING_ FimoI16
#elif FIMO_ISIZE_WIDTH == 32
#define FIMO_ISIZE_UNDERLYING_ FimoI32
#elif FIMO_ISIZE_WIDTH == 64
#define FIMO_ISIZE_UNDERLYING_ FimoI64
#else
#error "Unknown type"
#endif
#if FIMO_INTPTR_WIDTH == 8
#define FIMO_INTPTR_UNDERLYING_ FimoI8
#elif FIMO_INTPTR_WIDTH == 16
#define FIMO_INTPTR_UNDERLYING_ FimoI16
#elif FIMO_INTPTR_WIDTH == 32
#define FIMO_INTPTR_UNDERLYING_ FimoI32
#elif FIMO_INTPTR_WIDTH == 64
#define FIMO_INTPTR_UNDERLYING_ FimoI64
#else
#error "Unknown type"
#endif
#if FIMO_USIZE_WIDTH == 8
#define FIMO_USIZE_UNDERLYING_ FimoU8
#elif FIMO_USIZE_WIDTH == 16
#define FIMO_USIZE_UNDERLYING_ FimoU16
#elif FIMO_USIZE_WIDTH == 32
#define FIMO_USIZE_UNDERLYING_ FimoU32
#elif FIMO_USIZE_WIDTH == 64
#define FIMO_USIZE_UNDERLYING_ FimoU64
#else
#error "Unknown type"
#endif
#if FIMO_UINTPTR_WIDTH == 8
#define FIMO_UINTPTR_UNDERLYING_ FimoU8
#elif FIMO_UINTPTR_WIDTH == 16
#define FIMO_UINTPTR_UNDERLYING_ FimoU16
#elif FIMO_UINTPTR_WIDTH == 32
#define FIMO_UINTPTR_UNDERLYING_ FimoU32
#elif FIMO_UINTPTR_WIDTH == 64
#define FIMO_UINTPTR_UNDERLYING_ FimoU64
#else
#error "Unknown type"
#endif

#define FIMO_ISIZE_SWITCH_(NAME)                                                                                       \
    _Generic((FimoISize)0, FimoI8: NAME##_i8, FimoI16: NAME##_i16, FimoI32: NAME##_i32, FimoI64: NAME##_i64)
#define FIMO_INTPTR_SWITCH_(NAME)                                                                                      \
    _Generic((FimoIntPtr)0, FimoI8: NAME##_i8, FimoI16: NAME##_i16, FimoI32: NAME##_i32, FimoI64: NAME##_i64)
#define FIMO_USIZE_SWITCH_(NAME)                                                                                       \
    _Generic((FimoUSize)0, FimoU8: NAME##_u8, FimoU16: NAME##_u16, FimoU32: NAME##_u32, FimoU64: NAME##_u64)
#define FIMO_UINTPTR_SWITCH_(NAME)                                                                                     \
    _Generic((FimoUIntPtr)0, FimoU8: NAME##_u8, FimoU16: NAME##_u16, FimoU32: NAME##_u32, FimoU64: NAME##_u64)
#endif

#endif // !FIMO_IMPL_INTEGERS_INTEGERS_BASE_H
