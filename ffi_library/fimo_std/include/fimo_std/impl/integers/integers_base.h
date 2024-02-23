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

#if FIMO_ISIZE_WIDTH == 16
#define FIMO_ISIZE_SWITCH_(NAME, ...) NAME##_i16(__VA_ARGS__)
#elif FIMO_ISIZE_WIDTH == 32
#define FIMO_ISIZE_SWITCH_(NAME, ...) NAME##_i32(__VA_ARGS__)
#elif FIMO_ISIZE_WIDTH == 64
#define FIMO_ISIZE_SWITCH_(NAME, ...) NAME##_i64(__VA_ARGS__)
#else
#error "Unknown FimoISize width"
#endif

#if FIMO_INTPTR_WIDTH == 16
#define FIMO_INTPTR_SWITCH_(NAME, ...) NAME##_i16(__VA_ARGS__)
#elif FIMO_INTPTR_WIDTH == 32
#define FIMO_INTPTR_SWITCH_(NAME, ...) NAME##_i32(__VA_ARGS__)
#elif FIMO_INTPTR_WIDTH == 64
#define FIMO_INTPTR_SWITCH_(NAME, ...) NAME##_i64(__VA_ARGS__)
#else
#error "Unknown FimoIntPtr width"
#endif

#if FIMO_USIZE_WIDTH == 16
#define FIMO_USIZE_SWITCH_(NAME, ...) NAME##_u16(__VA_ARGS__)
#elif FIMO_USIZE_WIDTH == 32
#define FIMO_USIZE_SWITCH_(NAME, ...) NAME##_u32(__VA_ARGS__)
#elif FIMO_USIZE_WIDTH == 64
#define FIMO_USIZE_SWITCH_(NAME, ...) NAME##_u64(__VA_ARGS__)
#else
#error "Unknown FimoUSize width"
#endif

#if FIMO_UINTPTR_WIDTH == 16
#define FIMO_UINTPTR_SWITCH_(NAME, ...) NAME##_u16(__VA_ARGS__)
#elif FIMO_UINTPTR_WIDTH == 32
#define FIMO_UINTPTR_SWITCH_(NAME, ...) NAME##_u32(__VA_ARGS__)
#elif FIMO_UINTPTR_WIDTH == 64
#define FIMO_UINTPTR_SWITCH_(NAME, ...) NAME##_u64(__VA_ARGS__)
#else
#error "Unknown FimoUSize width"
#endif

#endif // !FIMO_IMPL_INTEGERS_INTEGERS_BASE_H