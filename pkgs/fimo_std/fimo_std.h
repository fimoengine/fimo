/// fimo_std - v0.2
///
/// NOTE: In order to use this library you must define
/// the following macro in exactly one file, _before_ including fimo_std.h:
///
/// #define FIMO_STD_IMPLEMENTATION
/// #include "fimo_std.h"
///
/// LICENSE
///
///     See end of file for license information.
///
/// Naming conventions:
///
///     - All declarations are prefixed with the `fstd_` or `FSTD_` prefix.
///     - Private declarations start with the `fstd__` or `FSTD__` prefix.
///     - User generated private declarations (such as private variables) start with the `fstd___` prefix.
///     - The `FSTD___` prefix may be utilized internally.
///     - Macros are defined in UPPER_CASE with a couple of exceptions:
///         - Keyword macros are written in lower_case, e.g. fstd_internal.
///         - Type macros follow the naming convention for type definitions.
///         - Simple function call wrapper macros follow the function naming convention.
///     - Type declarations are in CamelCase, e.g. `typedef struct {...} FSTD_MyType`.
///     - Function declarations are in snake_case, e.g. `void fstd_foo()`.
///     - Constants are in CamelCase, prefixed by the type, e.g. `FSTD_MyEnum_Val`.
///
/// USER-DEFINED MACROS:
///
///     - FSTD_NO_STDIO:
///         Don't include the `stdio.h` header
///     - FSTD_NO_STDLIB:
///         Don't include the `stdlib.h` header
///     - FSTD_TRAP:
///         Custom trap utility used by the assertions.
///         Must conform to the signature `int trap(const char *err)`.
///         Must be defined if `FSTD_NO_STDIO` or `FSTD_NO_STDLIB` are defined.
///     - FSTD_PRINT_BUFF:
///         Custom formatter function for the default logging utilities.
///         Must be compatible with `vsnprintf`, but may accept additional additional formatting specifiers.
///     - FSTD_NO_DEBUG:
///         Compile in release mode. May disable some debug-only utilities. Notably, removed debug assertions.
///     - FSTD_FAST:
///         Compile with the `fast` option. Disables non-debug assertions and changes the default tracing level.
///     - FSTD_STATIC:
///         Force all symbols to be declared as `static`.
///     - FSTD_TRACING_SCOPE:
///         Specifies a custom tracing scope, is defined as `FSTD_TRACING_DEFAULT_SCOPE` unless already defined.
///         Note that the logging utilities accept `_scoped` variants which allow to specify custom scopes.
///         Example usage:
///
///             #undef FSTD_TRACING_SCOPE
///             #define FSTD_TRACING_SCOPE "my custom scope"
///
///             void foo() {
///                 fstd_log_trace("example log %d %d", 10, 9);
///                 ...
///             }
///
///             #undef FSTD_TRACING_SCOPE
///             #define FSTD_TRACING_SCOPE FSTD_TRACING_DEFAULT_SCOPE
///     - FSTD_TRACING_TARGET:
///         Specifies a custom tracing target, is defined as `FSTD_TRACING_DEFAULT_TARGET` unless already defined.
///         Example usage:
///
///             #undef FSTD_TRACING_TARGET
///             #define FSTD_TRACING_TARGET "my custom target"
///
///             void foo() {
///                 fstd_log_trace("example log %d %d", 10, 9);
///                 ...
///             }
///
///             #undef FSTD_TRACING_TARGET
///             #define FSTD_TRACING_TARGET FSTD_TRACING_DEFAULT_TARGET
///     - FSTD_TRACING_MAX_LEVEL:
///         Maximum tracing event level.
///         Expected to be one of the defined `FSTD_TRACING_LEVEL_*` macros.
///         Tracing events above this value will be converted to noops at compile time if possible.

// -----------------------------------------
// HEADER DECLATATIONS ---------------------
// -----------------------------------------

#ifndef FIMO_STD_HEADER
#define FIMO_STD_HEADER

#include <limits.h>
#include <stdalign.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifndef FSTD_NO_STDIO
#include <stdio.h>
#endif

#ifndef FSTD_NO_STDLIB
#include <stdlib.h>
#endif

#if defined(_WIN32)
#define FSTD_PLATFORM_WINDOWS
#elif defined(__APPLE__)
#define FSTD_PLATFORM_APPLE
#define FSTD_PLATFORM_POSIX
#elif defined(__linux__)
#define FSTD_PLATFORM_LINUX
#define FSTD_PLATFORM_POSIX
#else
#error "unknown platform"
#endif

#if defined(__clang__)
#define FSTD_COMPILER_CLANG 1
#define FSTD_COMPILER_GCC_COMPATIBLE 1
#if defined(_MSC_VER)
#define FSTD_COMPILER_MSC_COMPATIBLE 1
#endif
#elif defined(__GNUC__)
#define FSTD_COMPILER_GCC 1
#define FSTD_COMPILER_GCC_COMPATIBLE 1
#elif defined(_MSC_VER)
#define FSTD_COMPILER_MSC 1
#define FSTD_COMPILER_MSC_COMPATIBLE 1
#else
#error "unknown compiler"
#endif

#ifdef FSTD_COMPILER_MSC
#include <intrin.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

// -----------------------------------------
// global utilities ------------------------
// -----------------------------------------

#define FSTD_PRAGMA(x) _Pragma(#x)

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
#define FSTD_PRAGMA_GCC(x) FSTD_PRAGMA(x)
#else
#define FSTD_PRAGMA_GCC(x)
#endif

#if defined(FSTD_COMPILER_GCC_COMPATIBLE) && !defined(FSTD_COMPILER_MSC_COMPATIBLE)
#define FSTD_PRAGMA_GCC_STRICT(x) FSTD_PRAGMA(x)
#else
#define FSTD_PRAGMA_GCC_STRICT(x)
#endif

#if defined(FSTD_COMPILER_MSC_COMPATIBLE)
#define FSTD_PRAGMA_MSVC(x) FSTD_PRAGMA(x)
#else
#define FSTD_PRAGMA_MSVC(x)
#endif

FSTD_PRAGMA_GCC(GCC diagnostic push)
FSTD_PRAGMA_GCC(GCC diagnostic ignored "-Wunused-function")

#if defined(__clang__)
FSTD_PRAGMA_GCC(GCC diagnostic ignored "-Wgnu-alignof-expression")
#endif

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
#define FSTD_PRINT_F_FMT_ATTR(fmt, dots) __attribute__((__format__(__printf__, fmt, dots)))
#else
#define FSTD_PRINT_F_FMT_ATTR(fmt, dots)
#endif

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
#define FSTD_ALLOC __attribute__((malloc))
#elif defined(FSTD_COMPILER_MSC_COMPATIBLE)
#define FSTD_ALLOC __declspec(restrict)
#else
#define FSTD_ALLOC
#endif

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
#define FSTD_CHECK_USE __attribute__((warn_unused_result))
#else
#define FSTD_CHECK_USE
#endif

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
#define fstd__typeof(x) typeof(x)
#define fstd__alignof(x) alignof(x)
#else
#ifdef __cplusplus
#define fstd__typeof(x) decltype(x)
#else
#define fstd__typeof(x) __typeof__(x)
#endif
#define fstd__alignof(x) alignof(fstd__typeof(x))
#endif

#define fstd_internal static
#define fstd_external extern
#define fstd_util static

#ifdef FSTD_STATIC
#define fstd__glob static
#define fstd__func static
#define fstd__glob_impl static
#define fstd__func_impl static
#else
#define fstd__glob extern
#define fstd__func extern
#define fstd__glob_impl
#define fstd__func_impl
#endif

#define FSTD_MAYBE_NULL

#define FSTD__MIN(a, b) ((a) < (b) ? (a) : (b))
#define FSTD__MAX(a, b) ((a) > (b) ? (a) : (b))

#define FSTD__STRINGIFY(a) #a
#define FSTD_STRINGIFY(a) FSTD__STRINGIFY(a)

#define FSTD__CONCAT(a, b) a##b
#define FSTD_CONCAT(a, b) FSTD__CONCAT(a, b)

#ifdef __COUNTER__
#define FSTD__UNIQUE __COUNTER__
#else
#define FSTD__UNIQUE __LINE__
#endif
#define FSTD_IDENT(x) FSTD_CONCAT(x, FSTD__UNIQUE)

#ifndef FSTD_TRAP
#define FSTD_TRAP(error) (fputs(error, stderr), abort(), 0)
#endif

#if !defined(NDEBUG) && !defined(FSTD_NO_DEBUG)
#define FSTD_DEBUG
#endif

#if defined(__cplusplus)
#define fstd_static_assert(expression, message) static_assert(expression, message)
#else
#define fstd_static_assert(expression, message) _Static_assert(expression, message)
#endif

#ifdef FSTD_FAST
#define fstd_assert(condition) ((void)0)
#define fstd_nassert(condition) ((void)0)
#else
#define fstd_assert(condition)                                                                                         \
    (void)(!!(condition) ||                                                                                            \
           FSTD_TRAP("assertion error in " __FILE__ ":" FSTD_STRINGIFY(__LINE__) ": fstd_assert(" #condition ")"))

#define fstd_nassert(condition)                                                                                        \
    (void)(!(condition) ||                                                                                             \
           FSTD_TRAP("assertion error in " __FILE__ ":" FSTD_STRINGIFY(__LINE__) ": fstd_nassert(" #condition ")"))
#endif

#ifdef FSTD_DEBUG
#define fstd_dbg_assert(condition)                                                                                     \
    (void)(!!(condition) ||                                                                                            \
           FSTD_TRAP("assertion error in " __FILE__ ":" FSTD_STRINGIFY(__LINE__) ": fstd_dbg_assert(" #condition ")"))

#define fstd_dbg_nassert(condition)                                                                                    \
    (void)(!(condition) || FSTD_TRAP("assertion error in " __FILE__                                                    \
                                     ":" FSTD_STRINGIFY(__LINE__) ": fstd_dbg_nassert(" #condition ")"))
#else
#define fstd_dbg_assert(condition) ((void)0)
#define fstd_dbg_nassert(condition) ((void)0)
#endif

typedef int8_t FSTD_I8;
typedef int16_t FSTD_I16;
typedef int32_t FSTD_I32;
typedef int64_t FSTD_I64;
typedef ptrdiff_t FSTD_ISize;

typedef uint8_t FSTD_U8;
typedef uint16_t FSTD_U16;
typedef uint32_t FSTD_U32;
typedef uint64_t FSTD_U64;
typedef size_t FSTD_USize;

fstd_static_assert(sizeof(ptrdiff_t) == sizeof(intptr_t), "invalid intptr_t size");
fstd_static_assert(sizeof(size_t) == sizeof(uintptr_t), "invalid uintptr_t size");
fstd_static_assert(sizeof(void *) == sizeof(uintptr_t), "invalid pointer size");

#define FSTD_I8_MIN INT8_MIN
#define FSTD_I16_MIN INT16_MIN
#define FSTD_I32_MIN INT32_MIN
#define FSTD_I64_MIN INT64_MIN
#define FSTD_ISIZE_MIN PTRDIFF_MIN

#define FSTD_U8_MIN (FSTD_U8)0
#define FSTD_U16_MIN (FSTD_U16)0
#define FSTD_U32_MIN (FSTD_U32)0
#define FSTD_U64_MIN (FSTD_U64)0
#define FSTD_USIZE_MIN (FSTD_USize)0

#define FSTD_I8_MAX INT8_MAX
#define FSTD_I16_MAX INT16_MAX
#define FSTD_I32_MAX INT32_MAX
#define FSTD_I64_MAX INT64_MAX
#define FSTD_ISIZE_MAX PTRDIFF_MAX

#define FSTD_U8_MAX UINT8_MAX
#define FSTD_U16_MAX UINT16_MAX
#define FSTD_U32_MAX UINT32_MAX
#define FSTD_U64_MAX UINT64_MAX
#define FSTD_USIZE_MAX UINTPTR_MAX

#if FSTD_USIZE_MAX == FSTD_U64_MAX
#define FSTD_PTR_64
#elif FSTD_USIZE_MAX == FSTD_U32_MAX
#define FSTD_PTR_32
#else
#error "platform not supported"
#endif

fstd_util FSTD_U8 fstd_next_power_of_two_u8(FSTD_U8 v) {
    fstd_dbg_assert(v > 0);
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    return v + 1;
}

fstd_util FSTD_U16 fstd_next_power_of_two_u16(FSTD_U16 v) {
    fstd_dbg_assert(v > 0);
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    return v + 1;
}

fstd_util FSTD_U32 fstd_next_power_of_two_u32(FSTD_U32 v) {
    fstd_dbg_assert(v > 0);
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    return v + 1;
}

fstd_util FSTD_U64 fstd_next_power_of_two_u64(FSTD_U64 v) {
    fstd_dbg_assert(v > 0);
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v |= v >> 32;
    return v + 1;
}

fstd_util FSTD_USize fstd_next_power_of_two_usize(FSTD_USize v) {
    fstd_dbg_assert(v > 0);
    v--;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
#ifdef FSTD_PTR_64
    v |= v >> 32;
#endif
    return v + 1;
}

fstd_util bool fstd_is_power_of_two_i8(FSTD_U8 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_i16(FSTD_U16 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_i32(FSTD_U32 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_i64(FSTD_U64 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_isize(FSTD_USize v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}

fstd_util bool fstd_is_power_of_two_u8(FSTD_U8 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_u16(FSTD_U16 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_u32(FSTD_U32 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_u64(FSTD_U64 v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}
fstd_util bool fstd_is_power_of_two_usize(FSTD_USize v) {
    fstd_dbg_assert(v > 0);
    return ((v & (v - 1)) == 0);
}

fstd_util FSTD_U8 fstd_align_backwards_u8(FSTD_U8 value, FSTD_U8 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return value & ~(alignment - 1);
}
fstd_util FSTD_U16 fstd_align_backwards_u16(FSTD_U16 value, FSTD_U16 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return value & ~(alignment - 1);
}
fstd_util FSTD_U32 fstd_align_backwards_u32(FSTD_U32 value, FSTD_U32 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return value & ~(alignment - 1);
}
fstd_util FSTD_U64 fstd_align_backwards_u64(FSTD_U64 value, FSTD_U64 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return value & ~(alignment - 1);
}
fstd_util FSTD_USize fstd_align_backwards_usize(FSTD_USize value, FSTD_USize alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return value & ~(alignment - 1);
}

fstd_util FSTD_U8 fstd_align_forwards_u8(FSTD_U8 value, FSTD_U8 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return fstd_align_backwards_u8(value + (alignment - 1), alignment);
}
fstd_util FSTD_U16 fstd_align_forwards_u16(FSTD_U16 value, FSTD_U16 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return fstd_align_backwards_u16(value + (alignment - 1), alignment);
}
fstd_util FSTD_U32 fstd_align_forwards_u32(FSTD_U32 value, FSTD_U32 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return fstd_align_backwards_u32(value + (alignment - 1), alignment);
}
fstd_util FSTD_U64 fstd_align_forwards_u64(FSTD_U64 value, FSTD_U64 alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return fstd_align_backwards_u64(value + (alignment - 1), alignment);
}
fstd_util FSTD_USize fstd_align_forwards_usize(FSTD_USize value, FSTD_USize alignment) {
    fstd_dbg_assert(fstd_is_power_of_two_u8(alignment));
    return fstd_align_backwards_usize(value + (alignment - 1), alignment);
}

#define FSTD_UNUSED(...) fstd__unused(0 __VA_OPT__(, ) __VA_ARGS__)
fstd_util void fstd__unused(int arg0, ...) { (void)arg0; }

#define fstd_parent_of(parent, name, ptr) (parent *)fstd__parent_of((ptr), offsetof(parent, name))
fstd_util void *fstd__parent_of(void *ptr, FSTD_USize offset) {
    char *tmp = (char *)ptr;
    tmp -= offset;
    return tmp;
}

#define fstd_parent_of_const(parent, name, ptr) (const parent *)fstd__parent_of_const((ptr), offsetof(parent, name))
fstd_util const void *fstd__parent_of_const(const void *ptr, FSTD_USize offset) {
    char *tmp = (char *)ptr;
    tmp -= offset;
    return tmp;
}

#ifdef __cplusplus
#define fstd_nullptr nullptr
#define FSTD_INIT(t) t
#define FSTD_DEFAULT_STRUCT                                                                                            \
    {                                                                                                                  \
    }
#else
#define fstd_nullptr NULL
#define FSTD_INIT(t) (t)
#define FSTD_DEFAULT_STRUCT {0}
#endif

/// A slice of mutable entries.
#define FSTD_Slice(t)                                                                                                  \
    struct {                                                                                                           \
        t *FSTD_MAYBE_NULL ptr;                                                                                        \
        FSTD_USize len;                                                                                                \
    }

/// A slice of constant entries.
#define FSTD_SliceConst(t)                                                                                             \
    struct {                                                                                                           \
        const t *FSTD_MAYBE_NULL ptr;                                                                                  \
        FSTD_USize len;                                                                                                \
    }

#define FSTD_SLICE_EMPTY FSTD_DEFAULT_STRUCT
#define FSTD_SLICE_INIT_ARRAY(arr) {.ptr = (arr), .len = (sizeof((arr)) / sizeof((arr)[0]))}

#define FSTD__ENSURE_STR_LIT(x) ("" x "")
#define FSTD__STR_LEN(x) ((sizeof(x) / sizeof((x)[0])) - sizeof((x)[0]))
#define FSTD_STR_LEN(x) FSTD__STR_LEN(FSTD__ENSURE_STR_LIT(x))

typedef FSTD_Slice(char) FSTD_Str;
typedef FSTD_SliceConst(char) FSTD_StrConst;

#define FSTD_STR(x) {.ptr = (x), .len = FSTD_STR_LEN(x)}

fstd_util FSTD_USize fstd__strcpy(FSTD_Str dst, FSTD_StrConst src) {
    const char *read = src.ptr;
    char *write = dst.ptr;
    const FSTD_USize write_len = FSTD__MIN(src.len, dst.len);
    for (FSTD_USize i = 0; i < write_len; i++) {
        *write++ = *read++;
    }
    return write_len;
}

typedef union {
    struct {
        /// First group 8 hexadecimal digits.
        FSTD_U32 group1;
        /// Second group of 4 hexadecimal digits.
        FSTD_U16 group2;
        /// Third group of 4 hexadecimal digits.
        FSTD_U16 group3;
        /// Fourth group of 4 hexadecimal digits.
        FSTD_U16 group4;
        /// Fifth group of 12 hexadecimal digits.
        FSTD_U8 group5[6];
    };
    FSTD_U8 bytes[16];
    FSTD_U32 dwords[4];
    FSTD_U64 qwords[2];
} FSTD_Uuid;

typedef struct {
    const void *handle;
    FSTD_USize count;
} FSTD__RefCountedHandle;

#ifdef FSTD_PTR_64
#define FSTD__REF_COUNTED_HANDLE_LOCKED ((FSTD_USize)1) << 63
#elif defined(FSTD_PTR_32)
#define FSTD__REF_COUNTED_HANDLE_LOCKED ((FSTD_USize)1) << 31
#else
#error "unsupported platform"
#endif

fstd_util void fstd__ref_counted_handle_register(FSTD__RefCountedHandle *ref, const void *handle) {
    FSTD_USize locked = FSTD__REF_COUNTED_HANDLE_LOCKED;
#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
    while ((__atomic_or_fetch(&ref->count, locked, __ATOMIC_ACQUIRE) & locked) != 0) {
    }
#elif FSTD_COMPILER_MSC_COMPATIBLE
#ifdef FSTD_PTR_64
    while (_InterlockedOr64((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)locked) < 0) {
    }
#elif defined(FSTD_PTR_32)
    while (_InterlockedOr((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)locked) < 0) {
    }
#endif
#else
#error "unknown compiler"
#endif

    FSTD_USize count = ref->count & ~locked;
    fstd_dbg_assert(count < locked - 1);
    fstd_dbg_assert(ref->handle == fstd_nullptr || ref->handle == handle);
    fstd_dbg_assert(ref->handle == fstd_nullptr || count == 0);
    ref->handle = handle;
    ref->count += 1;

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
    __atomic_and_fetch(&ref->count, ~locked, __ATOMIC_RELEASE);
#elif FSTD_COMPILER_MSC_COMPATIBLE
#ifdef FSTD_PTR_64
    _InterlockedAnd64((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)~locked);
#elif defined(FSTD_PTR_32)
    _InterlockedAnd((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)~locked);
#endif
#endif
}

fstd_util void fstd__ref_counted_handle_unregister(FSTD__RefCountedHandle *ref) {
    FSTD_USize locked = FSTD__REF_COUNTED_HANDLE_LOCKED;
#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
    while ((__atomic_or_fetch(&ref->count, locked, __ATOMIC_ACQUIRE) & locked) != 0) {
    }
#elif FSTD_COMPILER_MSC_COMPATIBLE
#ifdef FSTD_PTR_64
    while (_InterlockedOr64((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)locked) < 0) {
    }
#else
    while (_InterlockedOr((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)locked) < 0) {
    }
#endif
#else
#error "unknown compiler"
#endif

    FSTD_USize count = ref->count & ~locked;
    fstd_dbg_assert(count > 0);
    fstd_dbg_assert(ref->handle != fstd_nullptr);
    ref->count -= 1;
    if (ref->count == 0)
        ref->handle = fstd_nullptr;

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
    __atomic_and_fetch(&ref->count, ~locked, __ATOMIC_RELEASE);
#elif FSTD_COMPILER_MSC_COMPATIBLE
#ifdef FSTD_PTR_64
    _InterlockedAnd64((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)~locked);
#else
    _InterlockedAnd((volatile FSTD_ISize *)&ref->count, (FSTD_ISize)~locked);
#endif
#endif
}

// -----------------------------------------
// memory ----------------------------------
// -----------------------------------------

typedef FSTD_Slice(FSTD_U8) FSTD_MemortSlice;

typedef struct {
    /// Allocates a new buffer.
    void *FSTD_MAYBE_NULL (*alloc)(void *FSTD_MAYBE_NULL data, FSTD_USize len, FSTD_USize align);
    /// Tries to resize the buffer in place.
    bool (*resize)(void *FSTD_MAYBE_NULL data, FSTD_MemortSlice memory, FSTD_USize align, FSTD_USize new_len);
    /// Resizes the buffer, allowing relocation.
    void *FSTD_MAYBE_NULL (*remap)(void *FSTD_MAYBE_NULL data, FSTD_MemortSlice memory, FSTD_USize align,
                                   FSTD_USize new_len);
    /// Frees a previously allocated buffer.
    void (*free)(void *FSTD_MAYBE_NULL data, FSTD_MemortSlice memory, FSTD_USize align);
} FSTD_AllocatorVtable;

/// General purpose allocator api.
typedef struct {
    void *FSTD_MAYBE_NULL ptr;
    const FSTD_AllocatorVtable *vtable;
} FSTD_Allocator;

#define fstd_allocator_create(alloc, type) fstd_allocator_alloc(alloc, type, 1)
#define fstd_allocator_destroy(alloc, ptr) fstd_allocator_free(alloc, ptr, 1)

#define fstd_allocator_alloc(alloc, type, n) (type *)fstd__allocator_alloc((alloc), sizeof(type) * (n), alignof(type))
FSTD_ALLOC fstd_util void *FSTD_MAYBE_NULL fstd__allocator_alloc(FSTD_Allocator alloc, FSTD_USize len,
                                                                 FSTD_USize align) {
    return alloc.vtable->alloc(alloc.ptr, len, align);
}

#define fstd_allocator_resize(alloc, ptr, n, new_n)                                                                    \
    fstd__allocator_resize((alloc), (ptr), sizeof(*ptr) * (n), fstd__alignof(*ptr), sizeof(*ptr) * (new_n))
fstd_util bool fstd__allocator_resize(FSTD_Allocator alloc, void *ptr, FSTD_USize len, FSTD_USize align,
                                      FSTD_USize new_len) {
    FSTD_MemortSlice memory = {.ptr = (FSTD_U8 *)ptr, .len = len};
    return alloc.vtable->resize(alloc.ptr, memory, align, new_len);
}

#define fstd_allocator_remap(alloc, ptr, n, new_n)                                                                     \
    (type *)fstd__allocator_remap((alloc), (ptr), sizeof(*ptr) * (n), fstd__alignof(*ptr), sizeof(*ptr) * (new_n))
FSTD_ALLOC fstd_util void *FSTD_MAYBE_NULL fstd__allocator_remap(FSTD_Allocator alloc, void *ptr, FSTD_USize len,
                                                                 FSTD_USize align, FSTD_USize new_len) {
    FSTD_MemortSlice memory = {.ptr = (FSTD_U8 *)ptr, .len = len};
    return alloc.vtable->remap(alloc.ptr, memory, align, new_len);
}

#define fstd_allocator_free(alloc, ptr, n) fstd__allocator_free((alloc), (ptr), sizeof(*ptr) * (n), fstd__alignof(*ptr))
fstd_util void fstd__allocator_free(FSTD_Allocator alloc, void *ptr, FSTD_USize len, FSTD_USize align) {
    FSTD_MemortSlice memory = {.ptr = (FSTD_U8 *)ptr, .len = len};
    alloc.vtable->free(alloc.ptr, memory, align);
}

FSTD_ALLOC fstd_util void *FSTD_MAYBE_NULL fstd__allocator_null_alloc(void *FSTD_MAYBE_NULL arg0, FSTD_USize arg1,
                                                                      FSTD_USize arg2) {
    FSTD_UNUSED(arg0, arg1, arg2);
    return fstd_nullptr;
}
fstd_util bool fstd__allocator_null_resize(void *FSTD_MAYBE_NULL arg0, FSTD_MemortSlice arg1, FSTD_USize arg2,
                                           FSTD_USize arg3) {
    FSTD_UNUSED(arg0, arg1, arg2, arg3);
    return false;
}
FSTD_ALLOC fstd_util FSTD_MAYBE_NULL void *fstd__allocator_null_remap(void *FSTD_MAYBE_NULL arg0, FSTD_MemortSlice arg1,
                                                                      FSTD_USize arg2, FSTD_USize arg3) {
    FSTD_UNUSED(arg0, arg1, arg2, arg3);
    return fstd_nullptr;
}
fstd_util void fstd__allocator_null_free(void *FSTD_MAYBE_NULL arg0, FSTD_MemortSlice arg1, FSTD_USize arg2) {
    FSTD_UNUSED(arg0, arg1, arg2);
}

fstd_internal const FSTD_AllocatorVtable FSTD__AllocatorVtable_Null = {
        .alloc = fstd__allocator_null_alloc,
        .resize = fstd__allocator_null_resize,
        .remap = fstd__allocator_null_remap,
        .free = fstd__allocator_null_free,
};

/// An allocator which does not allocate or free any memory.
fstd_internal const FSTD_Allocator FSTD_Allocator_Null = {
        .ptr = fstd_nullptr,
        .vtable = &FSTD__AllocatorVtable_Null,
};

/// A growable non thread-safe memory arena.
typedef struct {
    FSTD_USize reserve_len;
    FSTD_USize commit_len;
    void *FSTD_MAYBE_NULL ptr;
    FSTD_USize pos;
} FSTD_Arena;

/// Temporary scope of a memory arena.
typedef struct {
    FSTD_Arena *arena;
    FSTD_USize pos;
} FSTD_TmpArena;

/// A growable thread-safe memory arena.
typedef struct {
    FSTD_U32 grow_futex;
    FSTD_USize reserve_len;
    FSTD_USize commit_len;
    void *FSTD_MAYBE_NULL ptr;
    FSTD_USize pos;
} FSTD_SharedArena;

/// Temporary scope of a shared memory arena.
typedef struct {
    FSTD_SharedArena *arena;
    FSTD_USize pos;
} FSTD_TmpSharedArena;

// -----------------------------------------
// errors ----------------------------------
// -----------------------------------------

/// Status code.
///
/// All positive values are interpreted as successfull operations.
typedef FSTD_I32 FSTD_Status;
enum {
    FSTD_Status_Ok = (FSTD_Status)0,
    FSTD_Status_Failure = (FSTD_Status)-1,
    FSTD_Status_FailureNoReport = (FSTD_Status)-2,
    FSTD__Status_ = FSTD_I32_MAX,
};

typedef FSTD_USize FSTD__Error;
enum {
    FSTD__Error_OutOfMemory = (FSTD__Error)0,
    FSTD__Error_ = FSTD_I32_MAX,
};

/// Error type returned from the platform apis.
#ifdef FSTD_PLATFORM_WINDOWS
typedef FSTD_U32 FSTD_PlatformError;
#else
typedef int FSTD_PlatformError;
#endif
fstd_static_assert(sizeof(FSTD_PlatformError) <= sizeof(void *), "invalid FSTD_PlatformError size");

typedef struct {
    // NOTE: Unique identifier of the error class.
    // Can be utilized to convey the type information.
    FSTD_Uuid cls;
    void (*FSTD_MAYBE_NULL deinit)(void *data);
    FSTD_USize (*write)(void *FSTD_MAYBE_NULL data, FSTD_Str dst, FSTD_USize offset, FSTD_USize *remaining);
} FSTD_ResultVtable;

fstd_internal const FSTD_Uuid FSTD_ResultCls_Unknown = FSTD_DEFAULT_STRUCT;
fstd_internal const FSTD_Uuid FSTD_ResultCls_Ok = {.qwords = {FSTD_U64_MAX, FSTD_U64_MAX}};
fstd_external const FSTD_ResultVtable FSTD__ResultVTable_PlatformError;

fstd_internal const FSTD_StrConst FSTD__Result_OkDescription = FSTD_STR("ok");
fstd_util FSTD_USize fstd__result_vtable_ok_write(void *FSTD_MAYBE_NULL arg0, FSTD_Str dst, FSTD_USize offset,
                                                  FSTD_USize *remaining) {
    FSTD_UNUSED(arg0);
    fstd_dbg_assert(offset <= FSTD__Result_OkDescription.len);
    FSTD_StrConst remaining_str = {
            .ptr = FSTD__Result_OkDescription.ptr + offset,
            .len = FSTD__Result_OkDescription.len - offset,
    };
    FSTD_USize written = fstd__strcpy(dst, remaining_str);
    *remaining = remaining_str.len - written;
    return written;
}
fstd_internal const FSTD_ResultVtable FSTD__ResultVtable_Ok = {
        .cls = FSTD_ResultCls_Ok,
        .deinit = fstd_nullptr,
        .write = fstd__result_vtable_ok_write,
};

fstd_util FSTD_USize fstd__result_vtable_error_write(void *FSTD_MAYBE_NULL data, FSTD_Str dst, FSTD_USize offset,
                                                     FSTD_USize *remaining) {
    FSTD_StrConst src;
    const FSTD__Error error = (FSTD__Error)((FSTD_USize)data);
    switch (error) {
        case FSTD__Error_OutOfMemory:
            src = FSTD_INIT(FSTD_StrConst) FSTD_STR("out of memory");
            break;
        default:
            fstd_dbg_assert(false);
    }
    fstd_dbg_assert(offset <= src.len);
    src.ptr += offset;
    src.len -= offset;
    FSTD_USize written = fstd__strcpy(dst, src);
    *remaining = src.len - written;
    return written;
}
fstd_internal const FSTD_ResultVtable FSTD__ResultVtable_Error = {
        .cls = FSTD_ResultCls_Unknown,
        .deinit = fstd_nullptr,
        .write = fstd__result_vtable_error_write,
};

/// A type-erased result value.
typedef struct {
    // NOTE: NULL indicates success.
    void *FSTD_MAYBE_NULL data;
    const FSTD_ResultVtable *vtable;
} FSTD_Result;

/// A wrapper around a result and a specified type.
#define FSTD_Fallible(t)                                                                                               \
    struct {                                                                                                           \
        FSTD_Result result;                                                                                            \
        t value;                                                                                                       \
    }

/// A result instance indicating no error.
fstd_internal const FSTD_Result FSTD_Result_Ok = {
        .data = fstd_nullptr,
        .vtable = &FSTD__ResultVtable_Ok,
};

fstd_util FSTD_Result fstd_result_init_platform_error(FSTD_PlatformError error) {
    return FSTD_INIT(FSTD_Result){
            // NOLINTNEXTLINE(performance-no-int-to-ptr)
            .data = (void *)((FSTD_USize)error),
            .vtable = &FSTD__ResultVTable_PlatformError,
    };
}

fstd_util FSTD_Result fstd__result_init_error(FSTD__Error error) {
    return FSTD_INIT(FSTD_Result){
            // NOLINTNEXTLINE(performance-no-int-to-ptr)
            .data = (void *)((FSTD_USize)error),
            .vtable = &FSTD__ResultVtable_Error,
    };
}

fstd_util void fstd_result_deinit(FSTD_Result result) {
    if (result.vtable->deinit) {
        result.vtable->deinit(result.data);
    }
}

fstd_util bool fstd_result_is_ok(FSTD_Result result) {
    return result.vtable->cls.qwords[0] == FSTD_U64_MAX && result.vtable->cls.qwords[1] == FSTD_U64_MAX;
}
fstd_util bool fstd_result_is_err(FSTD_Result result) { return !fstd_result_is_ok(result); }

fstd_util FSTD_USize fstd_result_write(FSTD_Result result, FSTD_Str dst, FSTD_USize offset, FSTD_USize *remaining) {
    return result.vtable->write(result.data, dst, offset, remaining);
}

// -----------------------------------------
// version ---------------------------------
// -----------------------------------------

#define FSTD_VERSION(major, minor, patch) FSTD_VERSION_PB(major, minor, patch, "", "")
#define FSTD_VERSION_P(major, minor, patch, pre) FSTD_VERSION_PB(major, minor, patch, pre, "")
#define FSTD_VERSION_B(major, minor, patch, build) FSTD_VERSION_PB(major, minor, patch, "", build)
#define FSTD_VERSION_PB(major, minor, patch, pre, build)                                                               \
    {.major = major, .minor = minor, .patch = patch, .pre = FSTD_STR(pre), .build = FSTD_STR(build)}

/// A version specifier following the Semantic Versioning 2.0.0 specification.
typedef struct {
    FSTD_USize major;
    FSTD_USize minor;
    FSTD_USize patch;
    FSTD_StrConst pre;
    FSTD_StrConst build;
} FSTD_Version;

/// Initializes the version from a string.
///
/// NOTE: The string must outlive the version.
fstd_external FSTD_Result fstd_version_init_str(FSTD_Version *version, FSTD_StrConst version_str);

/// Calculates the string length required to represent the version as a string
/// without pre-release and build specifiers.
fstd_external FSTD_USize fstd_version_str_len(const FSTD_Version *version);

/// Calculates the string length required to represent the version as a string.
fstd_external FSTD_USize fstd_version_str_len_full(const FSTD_Version *version);

/// Represents the version as a string.
///
/// Writes a string of the form "major.minor.patch" into `str`. If `written` is not
/// `NULL`, it is set to the number of characters written.
fstd_external FSTD_Result fstd_version_write_str(const FSTD_Version *version, FSTD_Str dst,
                                                 FSTD_USize *FSTD_MAYBE_NULL written);

/// Represents the version as a string.
///
/// Writes a string representation of the version into `str`. If `written` is
/// not `NULL`, it is set to the number of characters written.
fstd_external FSTD_Result fstd_version_write_full_str(const FSTD_Version *version, FSTD_Str dst,
                                                      FSTD_USize *FSTD_MAYBE_NULL written);

/// Compares two versions.
///
/// Returns an ordering of the two versions, without taking into consideration the build numbers.
/// Returns `-1` if `lhs < rhs`, `0` if `lhs == rhs`, or `1` if `lhs > rhs`.
fstd_external FSTD_I32 fstd_version_order(const FSTD_Version *lhs, const FSTD_Version *rhs);

/// Checks for the compatibility of two versions.
///
/// If `got` sattisfies `required` it indicates that an object which is versioned with the
/// version `got` can be used instead of an object of the same type carrying the version
/// `required`.
///
/// The compatibility of `got` with `required` is determined by the following algorithm:
///
/// 1. The major versions of `got` and `required` must be equal.
/// 2. If the major version is `0`, the minor versions must be equal.
/// 3. `got >= required`.
fstd_external bool fstd_version_sattisfies(const FSTD_Version *got, const FSTD_Version *required);

// -----------------------------------------
// time ------------------------------------
// -----------------------------------------

#define FSTD_MILLIS_PER_SEC 100
#define FSTD_MICROS_PER_SEC 1000000
#define FSTD_NANOS_PER_SEC 1000000000
#define FSTD_MICROS_PER_MILLIS 1000
#define FSTD_NANOS_PER_MILLIS 1000000
#define FSTD_NANOS_PER_MICROS 1000

/// A 96bit integer, able to represent any timepoint or duration.
typedef struct {
    FSTD_U64 low;
    FSTD_U32 high;
} FSTD_TimeInt;

/// An duration of time.
typedef struct {
    /// Number of seconds.
    FSTD_U64 secs;
    /// Number of nanoseconds.
    /// NOTE: Must be less than `FSTD_NANOS_PER_SEC`
    FSTD_U32 nanos;
} FSTD_Duration;

/// A point in time since the unix epoch using the system clock.
typedef struct {
    /// Number of seconds.
    FSTD_U64 secs;
    /// Number of nanoseconds.
    /// NOTE: Must be less than `FSTD_NANOS_PER_SEC`
    FSTD_U32 nanos;
} FSTD_Time;

/// A monotonically increasing point in time.
///
/// The starting point is undefined.
typedef struct {
    /// Number of seconds.
    FSTD_U64 secs;
    /// Number of nanoseconds.
    /// NOTE: Must be less than `FSTD_NANOS_PER_SEC`
    FSTD_U32 nanos;
} FSTD_Instant;

#define FSTD_SECONDS(s) {.secs = (s), .nanos = 0}
#define FSTD_MILLIS(ms)                                                                                                \
    {.secs = (ms) / FSTD_MILLIS_PER_SEC, .nanos = ((ms) % FSTD_MILLIS_PER_SEC) * FSTD_NANOS_PER_MILLIS}
#define FSTD_MICROS(us)                                                                                                \
    {.secs = (us) / FSTD_MICROS_PER_SEC, .nanos = ((us) % FSTD_MICROS_PER_SEC) * FSTD_NANOS_PER_MICROS}
#define FSTD_NANOS(ns) {.secs = (ns) / FSTD_NANOS_PER_SEC, .nanos = (us) % FSTD_NANOS_PER_SEC}

#define FSTD_DURATION_ZERO FSTD_DEFAULT_STRUCT
#define FSTD_TIME_EPOCH FSTD_DEFAULT_STRUCT

#define FSTD_DURATION_MIN FSTD_DEFAULT_STRUCT
#define FSTD_TIME_MIN FSTD_DEFAULT_STRUCT
#define FSTD_INSTANT_MIN FSTD_DEFAULT_STRUCT

#define FSTD_DURATION_MAX {.secs = FSTD_U64_MAX, .nanos = 999999999}
#define FSTD_TIME_MAX {.secs = FSTD_U64_MAX, .nanos = 999999999}
#define FSTD_INSTANT_MAX {.secs = FSTD_U64_MAX, .nanos = 999999999}

fstd_util FSTD_U64 fstd_duration_secs(FSTD_Duration duration) { return duration.secs; }
fstd_util FSTD_U32 fstd_duration_subsec_millis(FSTD_Duration duration) {
    return duration.nanos / FSTD_NANOS_PER_MILLIS;
}
fstd_util FSTD_U32 fstd_duration_subsec_micros(FSTD_Duration duration) {
    return duration.nanos / FSTD_NANOS_PER_MICROS;
}
fstd_util FSTD_U32 fstd_duration_subsec_nanos(FSTD_Duration duration) { return duration.nanos; }

fstd_external FSTD_TimeInt fstd_duration_millis(FSTD_Duration duration);
fstd_external FSTD_TimeInt fstd_duration_micros(FSTD_Duration duration);
fstd_external FSTD_TimeInt fstd_duration_nanos(FSTD_Duration duration);
fstd_external FSTD_I32 fstd_duration_order(FSTD_Duration lhs, FSTD_Duration rhs);
FSTD_CHECK_USE fstd_external FSTD_Status fstd_duration_add(FSTD_Duration *out, FSTD_Duration lhs, FSTD_Duration rhs);
fstd_external FSTD_Duration fstd_duration_add_saturating(FSTD_Duration lhs, FSTD_Duration rhs);
FSTD_CHECK_USE fstd_external FSTD_Status fstd_duration_sub(FSTD_Duration *out, FSTD_Duration lhs, FSTD_Duration rhs);
fstd_external FSTD_Duration fstd_duration_sub_saturating(FSTD_Duration *out, FSTD_Duration lhs, FSTD_Duration rhs);

fstd_external FSTD_Time fstd_time_now(void);
fstd_external FSTD_I32 fstd_time_order(FSTD_Time lhs, FSTD_Time rhs);
fstd_external FSTD_Result fstd_time_elapsed(FSTD_Duration *elapsed, FSTD_Time from);
fstd_external FSTD_Result fstd_time_duration_since(FSTD_Duration *elapsed, FSTD_Time since, FSTD_Time to);
fstd_external FSTD_Result fstd_time_add(FSTD_Time *out, FSTD_Time time, FSTD_Duration duration);
fstd_external FSTD_Time fstd_time_add_saturating(FSTD_Time time, FSTD_Duration duration);
fstd_external FSTD_Result fstd_time_sub(FSTD_Time *out, FSTD_Time time, FSTD_Duration duration);
fstd_external FSTD_Time fstd_time_sub_saturating(FSTD_Time time, FSTD_Duration duration);


fstd_external FSTD_Instant fstd_instant_now(void);
fstd_external FSTD_I32 fstd_instant_order(FSTD_Instant lhs, FSTD_Instant rhs);
fstd_external FSTD_Result fstd_instant_elapsed(FSTD_Duration *elapsed, FSTD_Instant from);
fstd_external FSTD_Result fstd_instant_duration_since(FSTD_Duration *elapsed, FSTD_Instant since, FSTD_Instant to);
fstd_external FSTD_Result fstd_instant_add(FSTD_Instant *out, FSTD_Instant time, FSTD_Duration duration);
fstd_external FSTD_Instant fstd_instant_add_saturating(FSTD_Instant time, FSTD_Duration duration);
fstd_external FSTD_Result fstd_instant_sub(FSTD_Instant *out, FSTD_Instant time, FSTD_Duration duration);
fstd_external FSTD_Instant fstd_instant_sub_saturating(FSTD_Instant time, FSTD_Duration duration);

// -----------------------------------------
// paths -----------------------------------
// -----------------------------------------

/// A growable filesystem path encoded as UTF-8.
typedef struct {
    char *FSTD_MAYBE_NULL ptr;
    FSTD_USize len;
    FSTD_USize capacity;
} FSTD_PathBuf;

/// An owned filesystem path encoded as UTF-8.
///
/// NOTE: The underlying string is not null-terminated.
typedef FSTD_Slice(char) FSTD_OwnedPath;

/// A filesystem path encoded as UTF-8.
///
/// NOTE: The underlying string is not null-terminated.
typedef FSTD_SliceConst(char) FSTD_Path;

/// Character type for paths used by the native os apis.
#ifdef FSTD_PLATFORM_WINDOWS
typedef wchar_t FSTD_OsPathChar;
#else
typedef char FSTD_OsPathChar;
#endif

/// An owned path that may be passed to the native os apis.
///
/// On Posix systems, the string encoding is unspecified.
/// On Windows systems, the strings are encoded as UTF-16.
/// The string is null-terminated.
typedef FSTD_Slice(FSTD_OsPathChar) FSTD_OwnedOsPath;

/// A path that may be passed to the native os apis.
///
/// On Posix systems, the string encoding is unspecified.
/// On Windows systems, the strings are encoded as UTF-16.
/// The string is null-terminated.
typedef FSTD_SliceConst(FSTD_OsPathChar) FSTD_OsPath;

typedef FSTD_I32 FSTD_Win32PathPrefixTag;
enum {
    FSTD_Win32PathPrefixTag_Verbatim = (FSTD_Win32PathPrefixTag)0,
    FSTD_Win32PathPrefixTag_VerbatimUnc = (FSTD_Win32PathPrefixTag)1,
    FSTD_Win32PathPrefixTag_VerbatimDisk = (FSTD_Win32PathPrefixTag)2,
    FSTD_Win32PathPrefixTag_DeviceNs = (FSTD_Win32PathPrefixTag)3,
    FSTD_Win32PathPrefixTag_Unc = (FSTD_Win32PathPrefixTag)4,
    FSTD_Win32PathPrefixTag_Disk = (FSTD_Win32PathPrefixTag)5,
    FSTD__Win32PathPrefixTag_ = FSTD_I32_MAX,
};

/// A Windows path prefix.
typedef struct {
    FSTD_Win32PathPrefixTag tag;
    union {
        /// `\\?\prefix`
        FSTD_Path verbatim;
        /// `\\?\UNC\hostname\share_name`
        struct {
            FSTD_Path hostname;
            FSTD_Path share_name;
        } verbatim_unc;
        /// `\\?\C:`
        char verbatim_disk;
        // `\\.\NS`
        FSTD_Path device_ns;
        /// `\\hostname\share_name`
        struct {
            FSTD_Path hostname;
            FSTD_Path share_name;
        } unc;
        /// `C:`
        char disk;
    } variant;
} FSTD_Win32PathPrefix;

typedef FSTD_I32 FSTD_PathComponentTag;
enum {
    FSTD_PathComponentTag_Win32Prefix = (FSTD_PathComponentTag)0,
    FSTD_PathComponentTag_RootDir = (FSTD_PathComponentTag)1,
    FSTD_PathComponentTag_CurDir = (FSTD_PathComponentTag)2,
    FSTD_PathComponentTag_ParentDir = (FSTD_PathComponentTag)3,
    FSTD_PathComponentTag_Normal = (FSTD_PathComponentTag)4,
    FSTD__PathComponentTag_ = FSTD_I32_MAX,
};

/// Definition of all possible path components.
typedef struct {
    FSTD_PathComponentTag tag;
    union {
        struct {
            FSTD_Path raw;
            FSTD_Win32PathPrefix prefix;
        } win32_prefix;
        FSTD_U8 root_dir;
        FSTD_U8 cur_dir;
        FSTD_U8 parent_dir;
        FSTD_Path normal;
    } variant;
} FSTD_PathComponent;

typedef FSTD_I32 FSTD__PathIterState;
enum {
    FSTD__PathIterState_Prefix = (FSTD__PathIterState)0,
    FSTD__PathIterState_StartDir = (FSTD__PathIterState)1,
    FSTD__PathIterState_Body = (FSTD__PathIterState)2,
    FSTD__PathIterState_Done = (FSTD__PathIterState)3,
    FSTD__PathIterState_ = FSTD_I32_MAX,
};

/// Iterator over the components of a path.
typedef struct {
    FSTD_Path current;
    bool has_prefix;
    FSTD_Win32PathPrefix win32_prefix;
    bool has_root_separator;
    FSTD__PathIterState front_state;
    FSTD__PathIterState back_state;
} FSTD_PathIter;

/// Initializes the given path buffer with the provided capacity.
fstd_util FSTD_Result fstd_path_buf_init_capacity(FSTD_PathBuf *buffer, FSTD_Allocator alloc, FSTD_USize capacity) {
    char *memory = fstd_allocator_alloc(alloc, char, capacity);
    if (!memory) {
        return fstd__result_init_error(FSTD__Error_OutOfMemory);
    }
    *buffer = FSTD_INIT(FSTD_PathBuf){
            .ptr = memory,
            .len = 0,
            .capacity = capacity,
    };
    return FSTD_Result_Ok;
}

/// Deallocates the path buffer.
fstd_util void fstd_path_buf_deinit(FSTD_PathBuf *buffer, FSTD_Allocator alloc) {
    fstd_allocator_free(alloc, buffer->ptr, buffer->capacity);
#ifndef FSTD_FAST
    *buffer = FSTD_INIT(FSTD_PathBuf) FSTD_DEFAULT_STRUCT;
#endif
}

fstd_util FSTD_Path fstd_path_buf_as_path(FSTD_PathBuf buffer) {
    return FSTD_INIT(FSTD_Path){
            .ptr = buffer.ptr,
            .len = buffer.len,
    };
}

/// Extends the path buffer with a path.
///
/// If `path` is absolute, it replaces the current path.
///
/// On Windows:
///
/// - if `path` has a root but no prefix (e.g., `\windows`), it replaces everything except for
///   the prefix (if any) of `buf`.
/// - if `path` has a prefix but no root, it replaces `buf`.
/// - if `buf` has a verbatim prefix (e.g. `\\?\C:\windows`) and `path` is not empty, the new
///   path is normalized: all references to `.` and `..` are removed`.
fstd_external FSTD_Result fstd_path_buf_push_alloc(FSTD_PathBuf *buffer, FSTD_Allocator alloc, FSTD_Path path);

/// Extends the path buffer with a path.
///
/// If `path` is absolute, it replaces the current path.
/// Fails if there is not enough capacity to perform the operation.
///
/// On Windows:
///
/// - if `path` has a root but no prefix (e.g., `\windows`), it replaces everything except for
///   the prefix (if any) of `buf`.
/// - if `path` has a prefix but no root, it replaces `buf`.
/// - if `buf` has a verbatim prefix (e.g. `\\?\C:\windows`) and `path` is not empty, the new
///   path is normalized: all references to `.` and `..` are removed`.
fstd_util FSTD_Result fstd_path_buf_push(FSTD_PathBuf *buffer, FSTD_Path path) {
    return fstd_path_buf_push_alloc(buffer, FSTD_Allocator_Null, path);
}

/// Extends the path buffer with an utf8 string.
///
/// If `path` is absolute, it replaces the current path.
///
/// On Windows:
///
/// - if `path` has a root but no prefix (e.g., `\windows`), it replaces everything except for
///   the prefix (if any) of `buf`.
/// - if `path` has a prefix but no root, it replaces `buf`.
/// - if `buf` has a verbatim prefix (e.g. `\\?\C:\windows`) and `path` is not empty, the new
///   path is normalized: all references to `.` and `..` are removed`.
fstd_external FSTD_Result fstd_path_buf_push_str_alloc(FSTD_PathBuf *buffer, FSTD_Allocator alloc, FSTD_StrConst path);

/// Extends the path buffer with an utf8 string.
///
/// If `path` is absolute, it replaces the current path.
/// Fails if there is not enough capacity to perform the operation.
///
/// On Windows:
///
/// - if `path` has a root but no prefix (e.g., `\windows`), it replaces everything except for
///   the prefix (if any) of `buf`.
/// - if `path` has a prefix but no root, it replaces `buf`.
/// - if `buf` has a verbatim prefix (e.g. `\\?\C:\windows`) and `path` is not empty, the new
///   path is normalized: all references to `.` and `..` are removed`.
fstd_util FSTD_Result fstd_path_buf_push_str(FSTD_PathBuf *buffer, FSTD_StrConst path) {
    return fstd_path_buf_push_str_alloc(buffer, FSTD_Allocator_Null, path);
}

/// Truncates the path buffer to its parent.
///
/// Returns `false` and does nothing if there is no parent. Otherwise, returns `true`.
fstd_external bool fstd_path_buf_pop(FSTD_PathBuf *buffer);

/// Initializes a new path with a string.
///
/// Ensures that the string is encoded as utf8.
fstd_external FSTD_Result fstd_path_init(FSTD_Path *path, FSTD_StrConst path_str);

/// Returns whether the path is absolute, i.e., if it is independent of the current directory.
fstd_external bool fstd_path_is_absolute(FSTD_Path path);

/// Returns whether the path is relative, i.e., if it is dependent of the current directory.
fstd_external bool fstd_path_is_relative(FSTD_Path path);

/// Returns if the path has a root.
fstd_external bool fstd_path_has_root(FSTD_Path path);

/// Returns the path without its final component, if there is one.
fstd_external bool fstd_path_parent(FSTD_Path path, FSTD_Path *parent);

/// Returns the final component of the path, if there is one.
fstd_external bool fstd_path_file_name(FSTD_Path path, FSTD_Path *file_name);

/// Constructs an iterator over the components of a path.
fstd_external FSTD_PathIter fstd_path_iter_new(FSTD_Path path);

/// Extracts a path corresponding to the portion of the path remaining for iteration.
fstd_external FSTD_Path fstd_path_iter_as_path(const FSTD_PathIter *iter);

/// Performs an iteration step.
///
/// Extracts the next component from the front of the iterator.
fstd_external bool fstd_path_iter_next(FSTD_PathIter *iter, FSTD_PathComponent *component);

/// Performs an iteration step.
///
/// Extracts the next component from the back of the iterator.
fstd_external bool fstd_path_iter_next_back(FSTD_PathIter *iter, FSTD_PathComponent *component);

/// Extracts the underlying path.
fstd_external FSTD_Path fstd_path_component_as_path(const FSTD_PathComponent *component);

// -----------------------------------------
// context api -----------------------------
// -----------------------------------------

#define FSTD_CTX_VERSION_MAJOR 0
#define FSTD_CTX_VERSION_MINOR 2
#define FSTD_CTX_VERSION_PATCH 0

#ifndef FSTD_CTX_VERSION_PRE
#define FSTD_CTX_VERSION_PRE ""
#endif

#ifndef FSTD_CTX_VERSION_BUILD
#define FSTD_CTX_VERSION_BUILD ""
#endif

#define FSTD_CTX_VERSION                                                                                               \
    FSTD_VERSION_PB(FSTD_CTX_VERSION_MAJOR, FSTD_CTX_VERSION_MINOR, FSTD_CTX_VERSION_PATCH, FSTD_CTX_VERSION_PRE,      \
                    FSTD_CTX_VERSION_BUILD)

typedef FSTD_I32 FSTD_CfgId;
enum {
    FSTD__CfgId_Unknown = (FSTD_CfgId)0,
    FSTD_CfgId_Tracing = (FSTD_CfgId)1,
    FSTD_CfgId_Modules = (FSTD_CfgId)2,
    FSTD__CfgId_ = FSTD_I32_MAX,
};

/// Common member of all config structures.
typedef struct {
    FSTD_CfgId id;
} FSTD_Cfg;

/// Handle to the global functions implemented by the context.
///
/// Is not intended to be instantiated outside of the current module, as it may gain additional
/// fields without being considered a breaking change.
typedef struct FSTD_Ctx FSTD_Ctx;

typedef union {
    struct {
        FSTD_Ctx *ctx;
        FSTD_USize count;
    };
    FSTD__RefCountedHandle handle;
} FSTD__Ctx;

/// Fetches the current active context.
///
/// May only be called after registering a context.
fstd__func FSTD_Ctx *fstd_ctx_get(void);

/// Registers the context as active.
///
/// May panic if a different context is already active.
/// May be called multiple times.
fstd__func void fstd_ctx_register(FSTD_Ctx *ctx);

/// Unregisters the context.
///
/// Must be paired up with a `fstd_ctx_register` call.
fstd__func void fstd_ctx_unregister(void);

typedef struct {
    void (*deinit)(void);
    bool (*has_error_result)(void);
    FSTD_Result (*replace_result)(FSTD_Result new_result);
} FSTD_CoreVtable;

typedef FSTD_SliceConst(FSTD_Cfg *const) FSTD_Cfgs;

/// Initializes a new context with the given options.
///
/// The initialized context is written to `ctx`.
/// Only one context may be initialized at any given moment.
FSTD_CHECK_USE fstd_external FSTD_Status fstd_ctx_init(FSTD_Ctx **ctx, FSTD_Cfgs cfgs);

/// Deinitializes the global context.
///
/// May block until all resources owned by the context are shut down.
fstd__func void fstd_ctx_deinit(void);

/// Returns the version of the initialized context.
///
/// May differ from the one specified during compilation.
fstd__func FSTD_Version fstd_ctx_get_version(void);

/// Checks whether the context has an error stored for the current thread.
fstd__func bool fstd_ctx_has_error_result(void);

/// Replaces the thread-local result stored in the context with a new one.
///
/// The old result is returned.
fstd__func FSTD_Result fstd_ctx_replace_result(FSTD_Result new_result);

/// Swaps out the thread-local result with the `ok` result.
fstd__func FSTD_Result fstd_ctx_take_result(void);

/// Clears the thread-local result.
fstd__func void fstd_ctx_clear_result(void);

/// Sets the thread-local result, destroying the old one.
fstd__func void fstd_ctx_set_result(FSTD_Result new_result);

// -----------------------------------------
// async subsystem -------------------------
// -----------------------------------------

typedef struct {
    void (*ref)(void *data);
    void (*unref)(void *data);
    void (*wake_unref)(void *data);
    void (*wake)(void *data);
} FSTD_TaskWakerVtable;

/// Handle to a task continuation.
///
/// A waker is provides a way to notify a blocked task, that it may retry the operation.
typedef struct {
    void *FSTD_MAYBE_NULL data;
    const FSTD_TaskWakerVtable *vtable;
} FSTD_TaskWaker;

/// Increases the reference count of the waker.
fstd_util FSTD_TaskWaker fstd_task_waker_ref(FSTD_TaskWaker waker) {
    waker.vtable->ref(waker.data);
    return waker;
}

/// Decreases the reference count of the waker.
fstd_util void fstd_task_waker_unref(FSTD_TaskWaker waker) { waker.vtable->unref(waker.data); }

/// Wakes the task associated with the current waker and decreases the wakers reference count.
fstd_util void fstd_task_waker_wake_unref(FSTD_TaskWaker waker) { waker.vtable->wake_unref(waker.data); }

/// Wakes the task associated with the current waker, without decreasing the reference count of
/// the waker.
fstd_util void fstd_task_waker_wake(FSTD_TaskWaker waker) { waker.vtable->wake(waker.data); }

typedef struct {
    void (*deinit)(void *data);
    FSTD_TaskWaker (*waker)(void *data);
    void (*block)(void *data);
} FSTD_TaskWaiterVtable;

/// A waiter that blocks the current thread until it is notified.
///
/// The waiter is intended to be used by threads other than the event loop thread, as they are not
/// bound to a waker. Using this waiter inside the event loop will result in a deadlock.
typedef struct {
    void *FSTD_MAYBE_NULL data;
    const FSTD_TaskWaiterVtable *vtable;
} FSTD_TaskWaiter;

/// Initializes a new waiter.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_waiter_init(FSTD_TaskWaiter *waiter);

/// Deinitializes the waiter.
fstd_util void fstd_waiter_deinit(FSTD_TaskWaiter waiter) { waiter.vtable->deinit(waiter.data); }

/// Returns a reference to the waker for the waiter.
///
/// The caller does not own the waker.
fstd_util FSTD_TaskWaker fstd_waiter_waker(FSTD_TaskWaiter waiter) { return waiter.vtable->waker(waiter.data); }

/// Blocks the current thread until it has been notified.
///
/// The thread can be notified through the waker of the waiter.
fstd_util void fstd_waiter_block(FSTD_TaskWaiter waiter) { waiter.vtable->block(waiter.data); }

typedef void (*FSTD_TaskDeinitFn)(void *FSTD_MAYBE_NULL);
typedef bool (*FSTD_TaskWaiterPollFn)(void *FSTD_MAYBE_NULL, FSTD_TaskWaker, void *);

/// Blocks the current thread until the future is completed.
#define fstd_waiter_await(waiter, future, result)                                                                      \
    fstd__waiter_await(waiter, &future->data, (FSTD_TaskWaiterPollFn)future->poll, result)
fstd_util void fstd__waiter_await(FSTD_TaskWaiter waiter, void *FSTD_MAYBE_NULL data, FSTD_TaskWaiterPollFn poll,
                                  void *result) {
    FSTD_TaskWaker waker = fstd_waiter_waker(waiter);
    while (!poll(data, waker, result)) {
        fstd_waiter_block(waiter);
    }
}

/// A future with the specified state and return types.
///
/// Futures follow a simple execution model. Each future consists of three main components. A
/// state, a function to poll the future, and an optional cleanup function.
///
/// The poll function takes a pointer to the state and tries to make some progress. The future may
/// not progress if is not polled. The function must either return `false`, signaling that the
/// future has not yet been completed, or return `true` and write its result in the provided
/// pointer.
///
/// The second parameter of the poll function is a waker for the calling task. The waker is not
/// owned by the callee, and it may not release it without first acquiring it. If the poll function
/// signals a pending future, the caller is allowed to put itself in a suspended state until it is
/// notified by the waker. It is the responsibility of the poll function to notify the caller
/// through the waker, once further progress can be made. Failure of doing so may result in a
/// deadlock.
///
/// Polling a completed future will result in undefined behavior. The future may not be moved once
/// it has been polled, as its state may be self-referential.
#define FSTD_Future(t, r)                                                                                              \
    struct {                                                                                                           \
        t data;                                                                                                        \
        bool (*poll)(t * FSTD_MAYBE_NULL data, FSTD_TaskWaker waker, r *result);                                       \
        void (*FSTD_MAYBE_NULL deinit)(t * FSTD_MAYBE_NULL data);                                                      \
    }

/// A future with an opaque handle and specified return type.
#define FSTD_OpaqueFuture(r) FSTD_Future(void *, r)

/// Type of an enqueued future.
typedef FSTD_OpaqueFuture(void) FSTD_EnqueuedFuture;

typedef struct {
    FSTD_Status (*waiter_init)(FSTD_TaskWaiter *waiter);
    FSTD_Status (*future_enqueue)(const void *FSTD_MAYBE_NULL data, FSTD_USize data_size, FSTD_USize data_alignment,
                                  FSTD_USize result_size, FSTD_USize result_alignment, FSTD_TaskWaiterPollFn poll,
                                  FSTD_TaskDeinitFn FSTD_MAYBE_NULL deinit_data,
                                  FSTD_TaskDeinitFn FSTD_MAYBE_NULL deinit_result, FSTD_EnqueuedFuture *future);
} FSTD_TasksVtable;

/// Moves the future on the async executor.
///
/// Polling the new future will block the current task.
/// NOTE: The deinit functions may be null.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_future_enqueue(const void *FSTD_MAYBE_NULL data, FSTD_USize data_size,
                                                          FSTD_USize data_alignment, FSTD_USize result_size,
                                                          FSTD_USize result_alignment, FSTD_TaskWaiterPollFn poll,
                                                          FSTD_TaskDeinitFn FSTD_MAYBE_NULL deinit_data,
                                                          FSTD_TaskDeinitFn FSTD_MAYBE_NULL deinit_result,
                                                          FSTD_EnqueuedFuture *future);

// -----------------------------------------
// tracing subsystem -----------------------
// -----------------------------------------

#define FSTD_TRACING_LEVEL_OFF 0
#define FSTD_TRACING_LEVEL_ERROR 1
#define FSTD_TRACING_LEVEL_WARN 2
#define FSTD_TRACING_LEVEL_INFO 3
#define FSTD_TRACING_LEVEL_DEBUG 4
#define FSTD_TRACING_LEVEL_TRACE 5

#define FSTD_TRACING_DEFAULT_SCOPE ""
#ifndef FSTD_TRACING_SCOPE
#define FSTD_TRACING_SCOPE FSTD_TRACING_DEFAULT_SCOPE
#endif

#ifdef __GNUC_
#define FSTD_TRACING_DEFAULT_TARGET __FILE_NAME__
#else
#define FSTD_TRACING_DEFAULT_TARGET ""
#endif
#ifndef FSTD_TRACING_TARGET
#define FSTD_TRACING_TARGET FSTD_TRACING_DEFAULT_TARGET
#endif

#ifndef FSTD_DEBUG
#ifdef FSTD_FAST
#define FSTD_TRACING_DEFAULT_LEVEL FSTD_TRACING_LEVEL_OFF
#else
#define FSTD_TRACING_DEFAULT_LEVEL FSTD_TRACING_LEVEL_WARN
#endif
#else
#ifdef FSTD_FAST
#define FSTD_TRACING_DEFAULT_LEVEL FSTD_TRACING_LEVEL_DEBUG
#else
#define FSTD_TRACING_DEFAULT_LEVEL FSTD_TRACING_LEVEL_TRACE
#endif
#endif

#ifndef FSTD_TRACING_MAX_LEVEL
#define FSTD_TRACING_MAX_LEVEL FSTD_TRACING_DEFAULT_LEVEL
#endif

/// Tracing levels.
///
/// The levels are ordered such that given two levels `lvl1` and `lvl2`, where `lvl1 >= lvl2`, then
/// an event with level `lvl2` will be traced in a context where the maximum tracing level is
/// `lvl1`.
typedef FSTD_I32 FSTD_TracingLevel;
enum {
    FSTD_TracingLevel_Off = (FSTD_TracingLevel)0,
    FSTD_TracingLevel_Error = (FSTD_TracingLevel)1,
    FSTD_TracingLevel_Warn = (FSTD_TracingLevel)2,
    FSTD_TracingLevel_Info = (FSTD_TracingLevel)3,
    FSTD_TracingLevel_Debug = (FSTD_TracingLevel)4,
    FSTD_TracingLevel_Trace = (FSTD_TracingLevel)5,
    FSTD__TracingLevel_ = FSTD_I32_MAX,
};

/// Basic information regarding a tracing event.
///
/// The subsystem expects instances of this struct to have a static lifetime.
typedef struct {
    const char *name;
    const char *target;
    const char *scope;
    const char *FSTD_MAYBE_NULL file_name;
    /// `-1` if unknown.
    FSTD_I32 line_number;
    FSTD_TracingLevel level;
} FSTD_TracingEventInfo;

/// Common member of all tracing events.
typedef FSTD_I32 FSTD_TracingEventTag;
enum {
    FSTD_TracingEventTag_Start = (FSTD_TracingEventTag)0,
    FSTD_TracingEventTag_Finish = (FSTD_TracingEventTag)1,
    FSTD_TracingEventTag_RegisterThread = (FSTD_TracingEventTag)2,
    FSTD_TracingEventTag_UnregisterThread = (FSTD_TracingEventTag)3,
    FSTD_TracingEventTag_CreateCallStack = (FSTD_TracingEventTag)4,
    FSTD_TracingEventTag_DestroyCallStack = (FSTD_TracingEventTag)5,
    FSTD_TracingEventTag_UnblockCallStack = (FSTD_TracingEventTag)6,
    FSTD_TracingEventTag_SuspendCallStack = (FSTD_TracingEventTag)7,
    FSTD_TracingEventTag_ResumeCallStack = (FSTD_TracingEventTag)8,
    FSTD_TracingEventTag_EnterSpan = (FSTD_TracingEventTag)9,
    FSTD_TracingEventTag_ExitSpan = (FSTD_TracingEventTag)10,
    FSTD_TracingEventTag_LogMessage = (FSTD_TracingEventTag)11,
    FSTD_TracingEventTag_DeclareEventInfo = (FSTD_TracingEventTag)12,
    FSTD_TracingEventTag_StartThread = (FSTD_TracingEventTag)13,
    FSTD_TracingEventTag_StopThread = (FSTD_TracingEventTag)14,
    FSTD_TracingEventTag_LoadImage = (FSTD_TracingEventTag)15,
    FSTD_TracingEventTag_UnloadImage = (FSTD_TracingEventTag)16,
    FSTD_TracingEventTag_ContextSwitch = (FSTD_TracingEventTag)17,
    FSTD_TracingEventTag_ThreadWakeup = (FSTD_TracingEventTag)18,
    FSTD_TracingEventTag_CallStackSample = (FSTD_TracingEventTag)19,
    FSTD__TracingEventTag_ = FSTD_I32_MAX,
};

/// System cpu architecture.
typedef FSTD_U8 FSTD_CpuArch;
// NOLINTNEXTLINE
enum {
    FSTD_CpuArch_Unknown = (FSTD_CpuArch)0,
    FSTD_CpuArch_X86_64 = (FSTD_CpuArch)1,
    FSTD_CpuArch_Aarch64 = (FSTD_CpuArch)2,
};

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_Time epoch;
    FSTD_Duration resolution;
    FSTD_USize available_memory;
    FSTD_USize process_id;
    FSTD_USize num_cores;
    FSTD_CpuArch cpu_arch;
    FSTD_U8 cpu_id;
    FSTD_StrConst cpu_vendor;
    FSTD_StrConst app_name;
    FSTD_StrConst host_info;
} FSTD_TracingEventStart;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
} FSTD_TracingEventFinish;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize thread_id;
} FSTD_TracingEventRegisterThread;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize thread_id;
} FSTD_TracingEventUnregisterThread;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
} FSTD_TracingEventCreateCallStack;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
} FSTD_TracingEventDestroyCallStack;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
} FSTD_TracingEventUnblockCallStack;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
    bool mark_blocked;
} FSTD_TracingEventSuspendCallStack;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
    FSTD_USize thread_id;
} FSTD_TracingEventResumeCallStack;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
    const FSTD_TracingEventInfo *info;
    FSTD_StrConst message;
} FSTD_TracingEventEnterSpan;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
    bool is_unwinding;
} FSTD_TracingEventExitSpan;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    void *stack;
    const FSTD_TracingEventInfo *info;
    FSTD_StrConst message;
} FSTD_TracingEventLogMessage;

typedef struct {
    FSTD_TracingEventTag tag;
    const FSTD_TracingEventInfo *info;
} FSTD_TracingEventDeclareEventInfo;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize thread_id;
    FSTD_USize process_id;
} FSTD_TracingEventStartThread;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize thread_id;
    FSTD_USize process_id;
} FSTD_TracingEventStopThread;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize image_base;
    FSTD_USize image_size;
    FSTD_Path image_path;
} FSTD_TracingEventLoadImage;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize image_base;
} FSTD_TracingEventUnloadImage;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize old_thread_id;
    FSTD_USize new_thread_id;
    FSTD_U8 cpu;
    FSTD_U8 old_thread_wait_reason;
    FSTD_U8 old_thread_state;
    FSTD_U8 previous_cstate;
    FSTD_U8 new_thread_priority;
    FSTD_U8 old_thread_priority;
} FSTD_TracingEventContextSwitch;

typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize thread_id;
    FSTD_U8 cpu;
    FSTD_I8 adjust_reason;
    FSTD_I8 adjust_increment;
} FSTD_TracingEventThreadWakeup;

typedef FSTD_SliceConst(FSTD_USize) FSTD_TracingEventCallStackSampleCallStack;
typedef struct {
    FSTD_TracingEventTag tag;
    FSTD_Instant time;
    FSTD_USize thread_id;
    FSTD_TracingEventCallStackSampleCallStack call_stack;
} FSTD_TracingEventCallStackSample;

/// A subscriber for tracing events.
///
/// The main function of the tracing subsystem is managing and routing tracing events to
/// subscribers. Therefore it does not consume any events on its own, which is the task of the
/// subscribers. Subscribers may utilize the events in any way they deem fit.
typedef struct {
    void *FSTD_MAYBE_NULL data;
    void (*on_event)(void *FSTD_MAYBE_NULL data, const FSTD_TracingEventTag *event);
} FSTD_Subscriber;

fstd_util void fstd_subscriber_start(FSTD_Subscriber sub, FSTD_TracingEventStart ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_Start);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_finish(FSTD_Subscriber sub, FSTD_TracingEventFinish ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_Finish);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_register_thread(FSTD_Subscriber sub, FSTD_TracingEventRegisterThread ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_RegisterThread);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_unregister_thread(FSTD_Subscriber sub, FSTD_TracingEventUnregisterThread ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_UnregisterThread);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_create_call_stack(FSTD_Subscriber sub, FSTD_TracingEventCreateCallStack ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_CreateCallStack);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_destroy_call_stack(FSTD_Subscriber sub, FSTD_TracingEventDestroyCallStack ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_DestroyCallStack);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_unblock_call_stack(FSTD_Subscriber sub, FSTD_TracingEventUnblockCallStack ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_UnblockCallStack);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_suspend_call_stack(FSTD_Subscriber sub, FSTD_TracingEventSuspendCallStack ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_SuspendCallStack);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_resume_call_stack(FSTD_Subscriber sub, FSTD_TracingEventResumeCallStack ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_ResumeCallStack);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_enter_span(FSTD_Subscriber sub, FSTD_TracingEventEnterSpan ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_EnterSpan);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_exit_span(FSTD_Subscriber sub, FSTD_TracingEventExitSpan ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_ExitSpan);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_log_message(FSTD_Subscriber sub, FSTD_TracingEventLogMessage ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_LogMessage);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_declare_event_info(FSTD_Subscriber sub, FSTD_TracingEventDeclareEventInfo ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_DeclareEventInfo);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_start_thread(FSTD_Subscriber sub, FSTD_TracingEventStartThread ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_StartThread);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_stop_thread(FSTD_Subscriber sub, FSTD_TracingEventStopThread ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_StopThread);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_load_image(FSTD_Subscriber sub, FSTD_TracingEventLoadImage ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_LoadImage);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_unload_image(FSTD_Subscriber sub, FSTD_TracingEventUnloadImage ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_UnloadImage);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_context_switch(FSTD_Subscriber sub, FSTD_TracingEventContextSwitch ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_ContextSwitch);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_thread_wakeup(FSTD_Subscriber sub, FSTD_TracingEventThreadWakeup ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_ThreadWakeup);
    sub.on_event(sub.data, &ev.tag);
}

fstd_util void fstd_subscriber_call_stack_sample(FSTD_Subscriber sub, FSTD_TracingEventCallStackSample ev) {
    fstd_dbg_assert(ev.tag == FSTD_TracingEventTag_CallStackSample);
    sub.on_event(sub.data, &ev.tag);
}

/// Creates a new subscriber, which logs the messages to the stderr file.
fstd_external FSTD_Subscriber fstd_stderr_logger_init(void);

/// Destroys the priorly created subscriber.
fstd_external void fstd_stderr_logger_deinit(FSTD_Subscriber sub);

/// A call stack.
///
/// Each call stack represents a unit of computation, like a thread. A call stack is active on only
/// one thread at any given time. The active call stack of a thread can be swapped, which is useful
/// for tracing where a `M:N` threading model is used. In that case, one would create one stack for
/// each task, and activate it when the task is resumed.
typedef struct FSTD_CallStack FSTD_CallStack;

/// Creates a new empty call stack.
///
/// The call stack is marked as suspended.
fstd__func FSTD_CallStack *fstd_call_stack_init(void);

/// Destroys an empty call stack.
///
/// Marks the completion of a task. Before calling this function, the call stack must be empty,
/// i.e., there must be no active spans on the stack, and must not be active. The call stack may
/// not be used afterwards. The active call stack of the thread is destroyed automatically, on
/// thread exit or during destruction of the context.
fstd__func void fstd_call_stack_finish(FSTD_CallStack *stack);

/// Unwinds and destroys the call stack.
///
/// Marks that the task was aborted. Before calling this function, the call stack must not be
/// active. The call stack may not be used afterwards.
fstd__func void fstd_call_stack_abort(FSTD_CallStack *stack);

/// Replaces the call stack of the current thread.
///
/// This call stack will be used as the active call stack of the calling thread. The old call
/// stack is returned, enabling the caller to switch back to it afterwards. This call stack
/// must be in a suspended, but unblocked, state and not be active. The active call stack must
/// also be in a suspended state, but may also be blocked.
fstd__func FSTD_CallStack *fstd_call_stack_replace_current(FSTD_CallStack *stack);

/// Unblocks the blocked call stack.
///
/// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
/// marked as blocked.
fstd__func void fstd_call_stack_unblock(FSTD_CallStack *stack);

/// Marks the current call stack as being suspended.
///
/// While suspended, the call stack can not be utilized for tracing messages. The call stack
/// optionally also be marked as being blocked. In that case, the call stack must be unblocked
/// prior to resumption.
fstd__func void fstd_call_stack_suspend_current(bool mark_blocked);

/// Marks the current call stack as being resumed.
///
/// Once resumed, the context can be used to trace messages. To be successful, the current call
/// stack must be suspended and unblocked.
fstd__func void fstd_call_stack_resume_current(void);

/// Type of a formatter function.
///
/// The formatter function is allowed to format only part of the message, if it would not fit into
/// the buffer. Must return the number of bytes written.
typedef FSTD_USize (*fstd_tracing_fmt_fn)(void *FSTD_MAYBE_NULL data, char *FSTD_MAYBE_NULL buffer,
                                          FSTD_USize buffer_len);

fstd_util FSTD_USize fstd__tracing_fmt_null(void *FSTD_MAYBE_NULL arg0, char *FSTD_MAYBE_NULL arg1, FSTD_USize arg2) {
    FSTD_UNUSED(arg0, arg1, arg2);
    return 0;
}

#if !defined(FSTD_NO_STDIO) && !defined(FSTD_PRINT_BUFF)
#if FSTD_COMPILER_GCC_COMPATIBLE
#define FSTD_PRINT_BUFF __builtin_vsnprintf
#else
#define FSTD_PRINT_BUFF vsnprintf
#endif
#endif

typedef struct {
    const char *fmt;
    va_list *vlist;
} FSTD__TracingFmtPrintArgs;

fstd_util FSTD_USize fstd__tracing_fmt_print(void *data, char *FSTD_MAYBE_NULL buffer, FSTD_USize buffer_len) {
    FSTD_PRAGMA_MSVC(warning(push))
    FSTD_PRAGMA_MSVC(warning(disable : 4996))
    FSTD__TracingFmtPrintArgs *args = (FSTD__TracingFmtPrintArgs *)data;
    return FSTD_PRINT_BUFF(buffer, buffer_len, args->fmt, *args->vlist);
    FSTD_PRAGMA_MSVC(warning(pop))
}

typedef FSTD_SliceConst(FSTD_Subscriber) FSTD_TracingCfgSubscribers;

/// Configuration for the tracing subsystem.
typedef struct {
    FSTD_Cfg id;
    FSTD_USize format_buffer_len;
    FSTD_TracingLevel max_level;
    FSTD_TracingCfgSubscribers subscribers;
    bool register_thread;
    FSTD_StrConst app_name;
} FSTD_TracingCfg;

typedef struct {
    bool (*is_enabled)(void);
    void (*register_thread)(void);
    void (*unregister_thread)(void);
    FSTD_CallStack *(*init_call_stack)(void);
    void (*deinit_call_stack)(FSTD_CallStack *stack, bool do_abort);
    FSTD_CallStack *(*replace_current_call_stack)(FSTD_CallStack *stack);
    void (*unblock_call_stack)(FSTD_CallStack *stack);
    void (*suspend_current_call_stack)(bool mark_blocked);
    void (*resume_current_call_stack)(void);
    void (*enter_span)(const FSTD_TracingEventInfo *info, fstd_tracing_fmt_fn fmt, const void *fmt_data);
    void (*exit_span)(const FSTD_TracingEventInfo *info);
    void (*log_message)(const FSTD_TracingEventInfo *info, fstd_tracing_fmt_fn fmt, const void *fmt_data);
} FSTD_TracingVtable;

/// Checks whether the tracing subsystem is enabled.
///
/// This function can be used to check whether to call into the subsystem at all. Calling this
/// function is not necessary, as the remaining functions of the subsystem are guaranteed to return
/// default values, in case the subsystem is disabled.
fstd__func void fstd_tracing_is_enabled(void);

/// Registers the calling thread with the tracing subsystem.
///
/// The instrumentation is opt-in on a per thread basis, where unregistered threads will
/// behave as if the subsystem was disabled. Once registered, the calling thread gains access to
/// the tracing subsystem and is assigned a new empty call stack. A registered thread must be
/// unregistered from the tracing subsystem before the context is destroyed, by terminating the
/// tread, or by manually unregistering it. A registered thread may not try to register itself.
fstd__func void fstd_tracing_register_thread(void);

/// Unregisters the calling thread from the tracing subsystem.
///
/// Once unregistered, the calling thread looses access to the tracing subsystem until it is
/// registered again. The thread can not be unregistered until the call stack is empty.
fstd__func void fstd_tracing_unregister_thread(void);

/// Enters the span.
///
/// Once entered, the span is used as the context for succeeding events. Each `enter` operation
/// must be accompanied with a `exit` operation in reverse entering order. A span may be entered
/// multiple times. The formatting function may be used to assign a name to the entered span.
fstd__func void fstd_tracing_enter_span(const FSTD_TracingEventInfo *info, fstd_tracing_fmt_fn fmt,
                                        const void *FSTD_MAYBE_NULL fmt_data);

/// Enters the span.
///
/// Once entered, the span is used as the context for succeeding events. Each `enter` operation
/// must be accompanied with a `exit` operation in reverse entering order. A span may be entered
/// multiple times. The formatting function may be used to assign a name to the entered span.
///
/// Uses a formatter accepting a `printf` format string.
FSTD_PRINT_F_FMT_ATTR(2, 3)
fstd_util void fstd_tracing_enter_span_fmt(const FSTD_TracingEventInfo *info, const char *fmt, ...) {
    va_list vlist;
    va_start(vlist, fmt);
    FSTD__TracingFmtPrintArgs args = {.fmt = fmt, .vlist = &vlist};
    fstd_tracing_enter_span(info, fstd__tracing_fmt_print, &args);
    va_end(vlist);
}

/// Exits an entered span.
///
/// The events won't occur inside the context of the exited span anymore. The span must be the
/// span at the top of the current call stack.
fstd__func void fstd_tracing_exit_span(const FSTD_TracingEventInfo *info);

/// Logs a message with a custom format function.
fstd__func void fstd_tracing_log_message(const FSTD_TracingEventInfo *info, fstd_tracing_fmt_fn fmt,
                                         const void *FSTD_MAYBE_NULL fmt_data);

/// Logs a message with a formatter accepting a `printf` format string.
FSTD_PRINT_F_FMT_ATTR(2, 3)
fstd__func void fstd_tracing_log_message_fmt(const FSTD_TracingEventInfo *info, const char *fmt, ...) {
    va_list vlist;
    va_start(vlist, fmt);
    FSTD__TracingFmtPrintArgs args = {.fmt = fmt, .vlist = &vlist};
    fstd_tracing_log_message(info, fstd__tracing_fmt_print, &args);
    va_end(vlist);
}

/// Logs a message using the specified scope and level, and the active target.
#define fstd_log(info, scope_, lvl, fmt, ...)                                                                          \
    FSTD_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FSTD_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-zero-length")                                                     \
    static const FSTD_TracingEventInfo info = {                                                                        \
            .name = __func__,                                                                                          \
            .target = FSTD_TRACING_TARGET,                                                                             \
            .scope = (scope_),                                                                                         \
            .file_name = __FILE__,                                                                                     \
            .line_number = __LINE__,                                                                                   \
            .level = lvl,                                                                                              \
    };                                                                                                                 \
    if (lvl <= FSTD_TRACING_MAX_LEVEL)                                                                                 \
        fstd_tracing_log_message_fmt(&info, fmt __VA_OPT__(, ) __VA_ARGS__);                                           \
    FSTD_PRAGMA_GCC(GCC diagnostic pop)

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_ERROR
#define fstd_log_err(fmt, ...) fstd_log_err_scoped(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_err_scoped(scope, fmt, ...)                                                                           \
    fstd_log(FSTD_IDENT(fstd__log_), scope, FSTD_TracingLevel_Error, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_log_err(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_err_scoped(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_WARN
#define fstd_log_warn(fmt, ...) fstd_log_warn_scoped(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_warn_scoped(scope, fmt, ...)                                                                          \
    fstd_log(FSTD_IDENT(fstd__log_), scope, FSTD_TracingLevel_Warn, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_log_warn(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_warn_scoped(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_INFO
#define fstd_log_info(fmt, ...) fstd_log_info_scoped(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_info_scoped(scope, fmt, ...)                                                                          \
    fstd_log(FSTD_IDENT(fstd__log_), scope, FSTD_TracingLevel_Info, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_log_info(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_info_scoped(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_DEBUG
#define fstd_log_debug(fmt, ...) fstd_log_debug_scoped(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_debug_scoped(scope, fmt, ...)                                                                         \
    fstd_log(FSTD_IDENT(fstd__log_), scope, FSTD_TracingLevel_Debug, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_log_debug(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_debug_scoped(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_TRACE
#define fstd_log_trace(fmt, ...) fstd_log_trace_scoped(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_trace_scoped(scope, fmt, ...)                                                                         \
    fstd_log(FSTD_IDENT(fstd__log_), scope, FSTD_TracingLevel_Trace, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_log_trace(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_log_trace_scoped(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#define fstd_span_push(scope_, lvl, fmt, ...)                                                                          \
    FSTD_PRAGMA_MSVC(warning(push))                                                                                    \
    FSTD_PRAGMA_MSVC(warning(disable : 4456))                                                                          \
    FSTD_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FSTD_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-zero-length")                                                     \
    static const FSTD_TracingEventInfo fstd___span = {                                                                 \
            .name = __func__,                                                                                          \
            .target = FSTD_TRACING_TARGET,                                                                             \
            .scope = (scope_),                                                                                         \
            .file_name = __FILE__,                                                                                     \
            .line_number = __LINE__,                                                                                   \
            .level = lvl,                                                                                              \
    };                                                                                                                 \
    if (lvl <= FSTD_TRACING_MAX_LEVEL)                                                                                 \
        fstd_tracing_enter_span_fmt(&fstd___span, fmt __VA_OPT__(, ) __VA_ARGS__);                                     \
    FSTD_PRAGMA_GCC(GCC diagnostic pop)                                                                                \
    FSTD_PRAGMA_MSVC(warning(pop))
#define fstd_span_pop()                                                                                                \
    if (fstd___span.lvl <= FSTD_TRACING_MAX_LEVEL)                                                                     \
        fstd_tracing_exit_span(&fstd___span);

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_ERROR
#define fstd_span_err() fstd_span_err_scoped(FSTD_TRACING_SCOPE)
#define fstd_span_err_scoped(scope) fstd_span_err_scoped_named(scope, "")
#define fstd_span_err_named(fmt, ...) fstd_span_err_scoped_named(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_err_scoped_named(scope, fmt, ...)                                                                    \
    fstd_span_push(scope, FSTD_TracingLevel_Error, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_err_pop() fstd_span_pop()
#else
#define fstd_span_err()
#define fstd_span_err_scoped(scope) FSTD_UNUSED(scope)
#define fstd_span_err_named(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_err_scoped_named(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_err_pop()
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_WARN
#define fstd_span_warn() fstd_span_warn_scoped(FSTD_TRACING_SCOPE)
#define fstd_span_warn_scoped(scope) fstd_span_warn_scoped_named(scope, "")
#define fstd_span_warn_named(fmt, ...) fstd_span_warn_scoped_named(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_warn_scoped_named(scope, fmt, ...)                                                                   \
    fstd_span_push(scope, FSTD_TracingLevel_Warn, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_warn_pop() fstd_span_pop()
#else
#define fstd_span_warn()
#define fstd_span_warn_scoped(scope) FSTD_UNUSED(scope)
#define fstd_span_warn_named(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_warn_scoped_named(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_warn_pop()
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_INFO
#define fstd_span_info() fstd_span_info_scoped(FSTD_TRACING_SCOPE)
#define fstd_span_info_scoped(scope) fstd_span_info_scoped_named(scope, "")
#define fstd_span_info_named(fmt, ...) fstd_span_info_scoped_named(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_info_scoped_named(scope, fmt, ...)                                                                   \
    fstd_span_push(scope, FSTD_TracingLevel_Info, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_info_pop() fstd_span_pop()
#else
#define fstd_span_info()
#define fstd_span_info_scoped(scope) FSTD_UNUSED(scope)
#define fstd_span_info_named(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_info_scoped_named(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_info_pop()
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_DEBUG
#define fstd_span_debug() fstd_span_debug_scoped(FSTD_TRACING_SCOPE)
#define fstd_span_debug_scoped(scope) fstd_span_debug_scoped_named(scope, "")
#define fstd_span_debug_named(fmt, ...) fstd_span_debug_scoped_named(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_debug_scoped_named(scope, fmt, ...)                                                                  \
    fstd_span_push(scope, FSTD_TracingLevel_Debug, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_debug_pop() fstd_span_pop()
#else
#define fstd_span_debug()
#define fstd_span_debug_scoped(scope) FSTD_UNUSED(scope)
#define fstd_span_debug_named(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_debug_scoped_named(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_debug_pop()
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_TRACE
#define fstd_span_trace() fstd_span_trace_scoped(FSTD_TRACING_SCOPE)
#define fstd_span_trace_scoped(scope) fstd_span_trace_scoped_named(scope, "")
#define fstd_span_trace_named(fmt, ...) fstd_span_trace_scoped_named(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_trace_scoped_named(scope, fmt, ...)                                                                  \
    fstd_span_push(scope, FSTD_TracingLevel_Trace, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_trace_pop() fstd_span_pop()
#else
#define fstd_span_trace()
#define fstd_span_trace_scoped(scope) FSTD_UNUSED(scope)
#define fstd_span_trace_named(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_trace_scoped_named(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_trace_pop()
#endif

#ifdef __cplusplus
#define fstd__span_auto(n, scope_, lvl, fmt, ...)                                                                      \
    FSTD_PRAGMA_MSVC(warning(push))                                                                                    \
    FSTD_PRAGMA_MSVC(warning(disable : 4456))                                                                          \
    FSTD_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FSTD_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-zero-length")                                                     \
    static const FSTD_TracingEventInfo n = {                                                                           \
            .name = __func__,                                                                                          \
            .target = FSTD_TRACING_TARGET,                                                                             \
            .scope = (scope_),                                                                                         \
            .file_name = __FILE__,                                                                                     \
            .line_number = __LINE__,                                                                                   \
            .level = lvl,                                                                                              \
    };                                                                                                                 \
    struct {                                                                                                           \
        struct inner {                                                                                                 \
            ~inner() {                                                                                                 \
                if (lvl <= FSTD_TRACING_MAX_LEVEL)                                                                     \
                    fstd_tracing_exit_span(&n);                                                                        \
            }                                                                                                          \
        };                                                                                                             \
        inner i;                                                                                                       \
    } FSTD_IDENT(FSTD_CONCAT(n, _)){};                                                                                 \
    if (lvl <= FSTD_TRACING_MAX_LEVEL)                                                                                 \
        fstd_tracing_enter_span_fmt(&n, fmt __VA_OPT__(, ) __VA_ARGS__);                                               \
    FSTD_PRAGMA_GCC(GCC diagnostic pop)                                                                                \
    FSTD_PRAGMA_MSVC(warning(pop))
#elif defined(FSTD_COMPILER_GCC_COMPATIBLE)
#define fstd__span_auto(n, scope_, lvl, fmt, ...)                                                                      \
    FSTD_PRAGMA_MSVC(warning(push))                                                                                    \
    FSTD_PRAGMA_MSVC(warning(disable : 4456))                                                                          \
    FSTD_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FSTD_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-zero-length")                                                     \
    static const FSTD_TracingEventInfo n = {                                                                           \
            .name = __func__,                                                                                          \
            .target = FSTD_TRACING_TARGET,                                                                             \
            .scope = (scope_),                                                                                         \
            .file_name = __FILE__,                                                                                     \
            .line_number = __LINE__,                                                                                   \
            .level = lvl,                                                                                              \
    };                                                                                                                 \
    __attribute__((cleanup(fstd__span_cleanup_gcc))) const FSTD_TracingEventInfo *FSTD_IDENT(FSTD_CONCAT(n, _)) = &n;  \
    if (lvl <= FSTD_TRACING_MAX_LEVEL)                                                                                 \
        fstd_tracing_enter_span_fmt(&n, fmt __VA_OPT__(, ) __VA_ARGS__);                                               \
    FSTD_PRAGMA_GCC(GCC diagnostic pop)                                                                                \
    FSTD_PRAGMA_MSVC(warning(pop))
fstd_util void fstd__span_cleanup_gcc(void *info_p) {
    const FSTD_TracingEventInfo **info = (const FSTD_TracingEventInfo **)info_p;
    if ((*info)->level <= FSTD_TRACING_MAX_LEVEL)
        fstd_tracing_exit_span(*info);
}
#else
#define fstd__span_auto(n, scope, lvl, fmt, ...) #error "auto span not supported with current compiler"
#endif

#define fstd_span_auto(scope, lvl, fmt, ...)                                                                           \
    fstd__span_auto(FSTD_IDENT(fstd___span_), scope, lvl, fmt __VA_OPT__(, ) __VA_ARGS__)

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_ERROR
#define fstd_span_err_auto() fstd_span_err_scoped_auto(FSTD_TRACING_SCOPE)
#define fstd_span_err_scoped_auto(scope) fstd_span_err_scoped_named_auto(scope, "")
#define fstd_span_err_named_auto(fmt, ...)                                                                             \
    fstd_span_err_scoped_named_auto(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_err_scoped_named_auto(scope, fmt, ...)                                                               \
    fstd_span_auto(scope, FSTD_TracingLevel_Error, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_span_err_auto() fstd_span_er
#define fstd_span_err_scoped_auto(scope) FSTD_UNUSED(scope)
#define fstd_span_err_named_auto(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_err_scoped_named_auto(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_WARN
#define fstd_span_warn_auto() fstd_span_warn_scoped_auto(FSTD_TRACING_SCOPE)
#define fstd_span_warn_scoped_auto(scope) fstd_span_warn_scoped_named_auto(scope, "")
#define fstd_span_warn_named_auto(fmt, ...)                                                                            \
    fstd_span_warn_scoped_named_auto(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_warn_scoped_named_auto(scope, fmt, ...)                                                              \
    fstd_span_auto(scope, FSTD_TracingLevel_Warn, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_span_warn_auto()
#define fstd_span_warn_scoped_auto(scope) FSTD_UNUSED(scope)
#define fstd_span_warn_named_auto(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_warn_scoped_named_auto(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_INFO
#define fstd_span_info_auto() fstd_span_info_scoped_auto(FSTD_TRACING_SCOPE)
#define fstd_span_info_scoped_auto(scope) fstd_span_info_scoped_named_auto(scope, "")
#define fstd_span_info_named_auto(fmt, ...)                                                                            \
    fstd_span_info_scoped_named_auto(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_info_scoped_named_auto(scope, fmt, ...)                                                              \
    fstd_span_auto(scope, FSTD_TracingLevel_Info, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_info_pop() fstd_span_pop()
#else
#define fstd_span_info_auto()
#define fstd_span_info_scoped_auto(scope) FSTD_UNUSED(scope)
#define fstd_span_info_named_auto(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_info_scoped_named_auto(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_DEBUG
#define fstd_span_debug_auto() fstd_span_debug_scoped_auto(FSTD_TRACING_SCOPE)
#define fstd_span_debug_scoped_auto(scope) fstd_span_debug_scoped_named_auto(scope, "")
#define fstd_span_debug_named_auto(fmt, ...)                                                                           \
    fstd_span_debug_scoped_named_auto(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_debug_scoped_named_auto(scope, fmt, ...)                                                             \
    fstd_span_auto(scope, FSTD_TracingLevel_Debug, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_span_debug_auto()
#define fstd_span_debug_scoped_auto(scope) FSTD_UNUSED(scope)
#define fstd_span_debug_named_auto(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_debug_scoped_named_auto(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

#if FSTD_TRACING_MAX_LEVEL >= FSTD_TRACING_LEVEL_TRACE
#define fstd_span_trace_auto() fstd_span_trace_scoped_auto(FSTD_TRACING_SCOPE)
#define fstd_span_trace_scoped_auto(scope) fstd_span_trace_scoped_named_auto(scope, "")
#define fstd_span_trace_named_auto(fmt, ...)                                                                           \
    fstd_span_trace_scoped_named_auto(FSTD_TRACING_SCOPE, fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_trace_scoped_named_auto(scope, fmt, ...)                                                             \
    fstd_span_auto(scope, FSTD_TracingLevel_Trace, fmt __VA_OPT__(, ) __VA_ARGS__)
#else
#define fstd_span_trace_auto()
#define fstd_span_trace_scoped_auto(scope) FSTD_UNUSED(scope)
#define fstd_span_trace_named_auto(fmt, ...) FSTD_UNUSED(fmt __VA_OPT__(, ) __VA_ARGS__)
#define fstd_span_trace_scoped_named_auto(scope, fmt, ...) FSTD_UNUSED(scope, fmt __VA_OPT__(, ) __VA_ARGS__)
#endif

// -----------------------------------------
// modules subsystem -----------------------
// -----------------------------------------

/// Data type of a module parameter.
typedef FSTD_I32 FSTD_ModuleParamTag;
enum {
    FSTD_ModuleParamTag_U8 = (FSTD_ModuleParamTag)0,
    FSTD_ModuleParamTag_U16 = (FSTD_ModuleParamTag)1,
    FSTD_ModuleParamTag_U32 = (FSTD_ModuleParamTag)2,
    FSTD_ModuleParamTag_U64 = (FSTD_ModuleParamTag)3,
    FSTD_ModuleParamTag_I8 = (FSTD_ModuleParamTag)4,
    FSTD_ModuleParamTag_I16 = (FSTD_ModuleParamTag)5,
    FSTD_ModuleParamTag_I32 = (FSTD_ModuleParamTag)6,
    FSTD_ModuleParamTag_I64 = (FSTD_ModuleParamTag)7,
    FSTD__ModuleParamTag_ = FSTD_I32_MAX,
};

/// Access group for a module parameter.
typedef FSTD_I32 FSTD_ModuleAccesGroup;
enum {
    FSTD_ModuleAccessGroup_Public = (FSTD_ModuleAccesGroup)0,
    FSTD_ModuleAccessGroup_Dependency = (FSTD_ModuleAccesGroup)1,
    FSTD_ModuleAccessGroup_Private = (FSTD_ModuleAccesGroup)2,
    FSTD__ModuleAccessGroup_ = FSTD_I32_MAX,
};

/// Data type and access groups of a module parameter.
typedef struct {
    FSTD_ModuleParamTag tag;
    FSTD_ModuleAccesGroup read_group;
    FSTD_ModuleAccesGroup write_group;
} FSTD_ModuleParamInfo;

/// A type-erased module parameter.
typedef struct FSTD_ModuleParam FSTD_ModuleParam;
typedef struct FSTD__ModuleParam {
    FSTD_ModuleParamTag (*tag)(const struct FSTD__ModuleParam *param);
    void (*read)(const struct FSTD__ModuleParam *param, void *value);
    void (*write)(struct FSTD__ModuleParam *param, const void *value);
} FSTD__ModuleParam;

/// Returns the value type of the parameter.
fstd_util FSTD_ModuleParamTag fstd_module_param_opaque_tag(const FSTD_ModuleParam *param) {
    const FSTD__ModuleParam *p = (const FSTD__ModuleParam *)param;
    return p->tag(p);
}

/// Reads the value from the parameter.
fstd_util void fstd_module_param_opaque_read(const FSTD_ModuleParam *param, void *value) {
    const FSTD__ModuleParam *p = (const FSTD__ModuleParam *)param;
    p->read(p, value);
}

/// Writes the value into the parameter.
fstd_util void fstd_module_param_opaque_write(FSTD_ModuleParam *param, const void *value) {
    FSTD__ModuleParam *p = (FSTD__ModuleParam *)param;
    p->write(p, value);
}

#define FSTD__MODULE_PARAM(int, int_lower)                                                                             \
    FSTD___MODULE_PARAM(FSTD_CONCAT(FSTD_ModuleParam, int), FSTD_CONCAT(fstd_module_param_, int_lower),                \
                        FSTD_CONCAT(FSTD_, int), FSTD_CONCAT(FSTD_ModuleParamTag_, int))
#define FSTD___MODULE_PARAM(name, prefix, int, tag)                                                                    \
    typedef struct name name;                                                                                          \
    fstd_util FSTD_ModuleParamTag FSTD_CONCAT(prefix, _tag)(const name *param) {                                       \
        return fstd_module_param_opaque_tag((const FSTD_ModuleParam *)param);                                          \
    }                                                                                                                  \
    fstd_util int FSTD_CONCAT(prefix, _read)(const name *param) {                                                      \
        fstd_dbg_assert(FSTD_CONCAT(prefix, _tag)(param) == tag);                                                      \
        int value;                                                                                                     \
        fstd_module_param_opaque_read((const FSTD_ModuleParam *)param, &value);                                        \
        return value;                                                                                                  \
    }                                                                                                                  \
    fstd_util void FSTD_CONCAT(prefix, _write)(name * param, int value) {                                              \
        fstd_dbg_assert(FSTD_CONCAT(prefix, _tag)(param) == tag);                                                      \
        fstd_module_param_opaque_write((FSTD_ModuleParam *)param, &value);                                             \
    }

FSTD__MODULE_PARAM(U8, u8)
FSTD__MODULE_PARAM(U16, u16)
FSTD__MODULE_PARAM(U32, u32)
FSTD__MODULE_PARAM(U64, u64)
FSTD__MODULE_PARAM(I8, i8)
FSTD__MODULE_PARAM(I16, i16)
FSTD__MODULE_PARAM(I32, i32)
FSTD__MODULE_PARAM(I64, i64)

typedef struct {
    FSTD_ModuleParamTag (*tag)(void *param);
    void (*read)(void *param, void *value);
    void (*write)(void *param, const void *value);
} FSTD_ModuleParamDataVtable;

/// Internal handle to a parameter.
typedef struct {
    void *FSTD_MAYBE_NULL data;
    const FSTD_ModuleParamDataVtable *vtable;
} FSTD_ModuleParamData;

/// Returns the value type of the parameter data.
fstd_util FSTD_ModuleParamTag fstd_module_param_data_tag(FSTD_ModuleParamData data) {
    return data.vtable->tag(data.data);
}

/// Reads the value from the parameter data.
fstd_util void fstd_module_param_data_opaque_read(FSTD_ModuleParamData data, void *value) {
    data.vtable->read(data.data, value);
}

/// Writes the value into the parameter data.
fstd_util void fstd_module_param_data_opaque_write(FSTD_ModuleParamData data, const void *value) {
    data.vtable->write(data.data, value);
}

#define FSTD__MODULE_PARAM_DATA(int, int_lower)                                                                        \
    FSTD___MODULE_PARAM_DATA(FSTD_CONCAT(FSTD_, int), int_lower, FSTD_CONCAT(FSTD_ModuleParamTag_, int))
#define FSTD___MODULE_PARAM_DATA(int, suffix, tag)                                                                     \
    fstd_util int FSTD_CONCAT(fstd_module_param_data_read_, suffix)(FSTD_ModuleParamData data) {                       \
        fstd_dbg_assert(fstd_module_param_data_tag(data) == tag);                                                      \
        int value;                                                                                                     \
        fstd_module_param_data_opaque_read(data, &value);                                                              \
        return value;                                                                                                  \
    }                                                                                                                  \
    fstd_util void FSTD_CONCAT(fstd_module_param_data_write_, suffix)(FSTD_ModuleParamData data, int value) {          \
        fstd_dbg_assert(fstd_module_param_data_tag(data) == tag);                                                      \
        fstd_module_param_data_opaque_write(data, &value);                                                             \
    }

FSTD__MODULE_PARAM_DATA(U8, u8)
FSTD__MODULE_PARAM_DATA(U16, u16)
FSTD__MODULE_PARAM_DATA(U32, u32)
FSTD__MODULE_PARAM_DATA(U64, u64)
FSTD__MODULE_PARAM_DATA(I8, i8)
FSTD__MODULE_PARAM_DATA(I16, i16)
FSTD__MODULE_PARAM_DATA(I32, i32)
FSTD__MODULE_PARAM_DATA(I64, i64)

/// Global symbol namespace.
#define FSTD_DEFAULT_NS FSTD_STR("")

/// Identifier of a symbol.
typedef struct {
    FSTD_StrConst name;
    FSTD_StrConst ns;
    FSTD_Version version;
} FSTD_ModuleSymbol;

/// Shared handle to a loaded instance.
typedef struct FSTD_ModuleHandle FSTD_ModuleHandle;
typedef struct FSTD__ModuleHandle {
    FSTD_StrConst name;
    FSTD_StrConst description;
    FSTD_StrConst author;
    FSTD_StrConst license;
    FSTD_Path module_path;
    void (*ref)(struct FSTD__ModuleHandle *handle);
    void (*unref)(struct FSTD__ModuleHandle *handle);
    void (*mark_unloadable)(struct FSTD__ModuleHandle *handle);
    bool (*is_loaded)(struct FSTD__ModuleHandle *handle);
    bool (*try_ref_instance_strong)(struct FSTD__ModuleHandle *handle);
    void (*unref_instance_strong)(struct FSTD__ModuleHandle *handle);
} FSTD__ModuleHandle;

/// Returns the name of the module.
fstd_util FSTD_StrConst fstd_module_handle_name(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    return h->name;
}

/// Returns the description of the module.
fstd_util FSTD_StrConst fstd_module_handle_description(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    return h->description;
}

/// Returns the author of the module.
fstd_util FSTD_StrConst fstd_module_handle_author(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    return h->author;
}

/// Returns the license of the module.
fstd_util FSTD_StrConst fstd_module_handle_license(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    return h->license;
}

/// Returns the path of the module.
fstd_util FSTD_Path fstd_module_handle_module_path(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    return h->module_path;
}

/// Increases the reference count of the handle.
fstd_util void fstd_module_handle_ref(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    h->ref(h);
}

/// Decreases the reference count of the handle.
fstd_util void fstd_module_handle_unref(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    h->unref(h);
}

/// Signals that the owning instance may be unloaded.
///
/// The instance will be unloaded once it is no longer actively used by another instance.
fstd_util void fstd_module_handle_mark_unloadable(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    h->mark_unloadable(h);
}

/// Returns whether the owning instance is still loaded.
fstd_util bool fstd_module_handle_is_loaded(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    return h->is_loaded(h);
}

/// Tries to increase the strong reference count of the owning instance.
///
/// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
/// between modules, without registering the dependency with the subsystem.
///
/// NOTE: Use with caution. Prefer structuring your code in a way that does not necessitate
/// dependency tracking.
fstd_util bool fstd_module_handle_try_ref_instance_strong(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    return h->try_ref_instance_strong(h);
}

/// Decreases the strong reference count of the owning instance.
///
/// May only be called after the reference count of the instance has been increased.
fstd_util void fstd_module_handle_unref_instance_strong(FSTD_ModuleHandle *handle) {
    FSTD__ModuleHandle *h = (FSTD__ModuleHandle *)handle;
    h->unref_instance_strong(h);
}

/// Searches for a module by its name.
///
/// Queries a module by its unique name.
/// The returned handle will have its reference count increased.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_module_handle_find_by_name(FSTD_ModuleHandle **handle, FSTD_StrConst module);

/// Searches for a module by a symbol it exports.
///
/// Queries the module that exported the specified symbol.
/// The returned handle will have its reference count increased.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_module_handle_find_by_symbol(FSTD_ModuleHandle **handle,
                                                                        FSTD_ModuleSymbol symbol);

typedef FSTD_I32 FSTD_ModuleDependency;
enum {
    FSTD_ModuleDependency_None,
    FSTD_ModuleDependency_Static,
    FSTD_ModuleDependency_Dynamic,
    FSTD__ModuleDependency_ = FSTD_I32_MAX,
};

typedef struct FSTD_ModuleInstance FSTD_ModuleInstance;
typedef struct FSTD__ModuleInstance FSTD__ModuleInstance;
typedef struct {
    void (*ref)(FSTD__ModuleInstance *ctx);
    void (*unref)(FSTD__ModuleInstance *ctx);
    FSTD_Status (*query_namespace)(FSTD__ModuleInstance *ctx, FSTD_StrConst ns, FSTD_ModuleDependency *dependency);
    FSTD_Status (*add_namespace)(FSTD__ModuleInstance *ctx, FSTD_StrConst ns);
    FSTD_Status (*remove_namespace)(FSTD__ModuleInstance *ctx, FSTD_StrConst ns);
    FSTD_Status (*query_dependency)(FSTD__ModuleInstance *ctx, FSTD_ModuleHandle *handle,
                                    FSTD_ModuleDependency *dependency);
    FSTD_Status (*add_dependency)(FSTD__ModuleInstance *ctx, FSTD_ModuleHandle *handle);
    FSTD_Status (*remove_dependency)(FSTD__ModuleInstance *ctx, FSTD_ModuleHandle *handle);
    FSTD_Status (*load_symbol)(FSTD__ModuleInstance *ctx, FSTD_ModuleSymbol symbol, const void **loaded);
    FSTD_Status (*read_parameter)(FSTD__ModuleInstance *ctx, FSTD_ModuleParamTag tag, FSTD_StrConst module,
                                  FSTD_StrConst parameter, void *value);
    FSTD_Status (*write_parameter)(FSTD__ModuleInstance *ctx, FSTD_ModuleParamTag tag, FSTD_StrConst module,
                                   FSTD_StrConst parameter, const void *value);
} FSTD_ModuleInstanceVtable;

struct FSTD__ModuleInstance {
    const FSTD_ModuleInstanceVtable *const vtable;
    FSTD_ModuleParam *const *const FSTD_MAYBE_NULL parameters;
    const FSTD_Path *const FSTD_MAYBE_NULL resources;
    const void *const *const FSTD_MAYBE_NULL imports;
    const void *const *const FSTD_MAYBE_NULL exports;
    FSTD_ModuleHandle *const FSTD_MAYBE_NULL handle;
    FSTD_Ctx *const ctx_handle;
    const void *const FSTD_MAYBE_NULL state;
};

/// Returns the parameter table of the module.
fstd_util FSTD_ModuleParam *const *FSTD_MAYBE_NULL fstd_module_instance_parameters(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->parameters;
}

/// Returns the resource table of the module.
fstd_util const FSTD_Path *FSTD_MAYBE_NULL fstd_module_instance_resources(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->resources;
}

/// Returns the imports table of the module.
///
/// Imports are ordered the in declaration order of the module export.
fstd_util const void *const *FSTD_MAYBE_NULL fstd_module_instance_imports(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->imports;
}

/// Returns the exports table of the module.
///
/// Exports are ordered the in declaration order of the module export.
/// The exports are populated in declaration order and depopulated in reverse declaration order.
fstd_util const void *const *FSTD_MAYBE_NULL fstd_module_instance_exports(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->exports;
}

/// Returns the shared handle of the module.
///
/// NOTE: The reference count is not modified.
fstd_util FSTD_ModuleHandle *fstd_module_instance_handle(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->handle;
}

/// Returns the handle to the context.
fstd_util FSTD_Ctx *fstd_module_instance_ctx_handle(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->ctx_handle;
}

/// Returns the state of the module.
///
/// NOTE: Return value is undefined until after the execution of the module constructor and
/// after the execution of the module destructor.
fstd_util const void *FSTD_MAYBE_NULL fstd_module_instance_state(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->state;
}

/// Increases the strong reference count of the module instance.
///
/// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
/// between modules, without registering the dependency with the subsystem.
///
/// NOTE: Use with caution. Prefer structuring your code in a way that does not necessitate
/// dependency tracking.
fstd_util void fstd_module_instance_ref(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    ctx_->vtable->ref(ctx_);
}

/// Decreases the strong reference count of the module instance.
///
/// May only be called after the reference count has been increased.
fstd_util void fstd_module_instance_unref(FSTD_ModuleInstance *ctx) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    ctx_->vtable->unref(ctx_);
}

/// Checks the status of a namespace from the view of the module.
///
/// Checks if the module includes the namespace. In that case, the module is allowed access
/// to the symbols in the namespace. Additionally, this function also queries whether the
/// include is static, i.e., it was specified by the module at load time.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_query_namespace(FSTD_ModuleInstance *ctx, FSTD_StrConst ns,
                                                                          FSTD_ModuleDependency *dependency) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->query_namespace(ctx_, ns, dependency);
}

/// Adds a namespace dependency to the module.
///
/// Once added, the module gains access to the symbols of its dependencies that are
/// exposed in said namespace. A namespace can not be added multiple times.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_add_namespace(FSTD_ModuleInstance *ctx, FSTD_StrConst ns) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->add_namespace(ctx_, ns);
}

/// Removes a namespace dependency from the module.
///
/// Once excluded, the caller guarantees to relinquish access to the symbols contained in
/// said namespace. It is only possible to exclude namespaces that were manually added,
/// whereas static namespace dependencies remain valid until the module is unloaded.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_remove_namespace(FSTD_ModuleInstance *ctx, FSTD_StrConst ns) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->remove_namespace(ctx_, ns);
}

/// Checks if a module depends on another module.
///
/// Checks if the specified module is a dependency of the current instance. In that case
/// the instance is allowed to access the symbols exported by the module. Additionally,
/// this function also queries whether the dependency is static, i.e., the dependency was
/// specified by the module at load time.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_query_dependency(FSTD_ModuleInstance *ctx,
                                                                           FSTD_ModuleHandle *handle,
                                                                           FSTD_ModuleDependency *dependency) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->query_dependency(ctx_, handle, dependency);
}

/// Adds another module as a dependency.
///
/// After adding a module as a dependency, the module is allowed access to the symbols
/// and protected parameters of said dependency. Trying to adding a dependency to a module
/// that is already a dependency, or to a module that would result in a circular dependency
/// will result in an error.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_add_dependency(FSTD_ModuleInstance *ctx,
                                                                         FSTD_ModuleHandle *handle) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->add_dependency(ctx_, handle);
}

/// Removes a module as a dependency.
///
/// By removing a module as a dependency, the caller ensures that it does not own any
/// references to resources originating from the former dependency, and allows for the
/// unloading of the module. A module can only relinquish dependencies to modules that were
/// acquired dynamically, as static dependencies remain valid until the module is unloaded.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_remove_dependency(FSTD_ModuleInstance *ctx,
                                                                            FSTD_ModuleHandle *handle) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->remove_dependency(ctx_, handle);
}

/// Loads a symbol from the module subsystem.
///
/// The caller can query the subsystem for a symbol of a loaded module. This is useful for
/// loading optional symbols, or for loading symbols after the creation of a module. The
/// symbol, if it exists, is returned, and can be used until the module relinquishes the
/// dependency to the module that exported the symbol. This function fails, if the module
/// containing the symbol is not a dependency of the module.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_load_symbol(FSTD_ModuleInstance *ctx,
                                                                      FSTD_ModuleSymbol symbol, const void **loaded) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->load_symbol(ctx_, symbol, loaded);
}

/// Reads a module parameter with dependency read access.
///
/// Reads the value of a module parameter with dependency read access. The operation fails,
/// if the parameter does not exist, or if the parameter does not allow reading with a
/// dependency access.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_read_parameter_opaque(FSTD_ModuleInstance *ctx,
                                                                                FSTD_ModuleParamTag tag,
                                                                                FSTD_StrConst module,
                                                                                FSTD_StrConst parameter, void *value) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->read_parameter(ctx_, tag, module, parameter, value);
}

/// Sets a module parameter with dependency write access.
///
/// Sets the value of a module parameter with dependency write access. The operation fails,
/// if the parameter does not exist, or if the parameter does not allow writing with a
/// dependency access.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_instance_write_parameter_opaque(FSTD_ModuleInstance *ctx,
                                                                                 FSTD_ModuleParamTag tag,
                                                                                 FSTD_StrConst module,
                                                                                 FSTD_StrConst parameter,
                                                                                 const void *value) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->write_parameter(ctx_, tag, module, parameter, value);
}

#define FSTD__MODULE_INSTANCE_PARAM(int, int_lower)                                                                    \
    FSTD___MODULE_INSTANCE_PARAM(FSTD_CONCAT(FSTD_, int), int_lower, FSTD_CONCAT(FSTD_ModuleParamTag_, int))
#define FSTD___MODULE_INSTANCE_PARAM(int, suffix, tag)                                                                 \
    FSTD_CHECK_USE fstd_util FSTD_Status FSTD_CONCAT(fstd_module_instance_read_parameter_, suffix)(                    \
            FSTD_ModuleInstance * ctx, FSTD_StrConst module, FSTD_StrConst parameter, int *value) {                    \
        return fstd_module_instance_read_parameter_opaque(ctx, tag, module, parameter, &value);                        \
    }                                                                                                                  \
    FSTD_CHECK_USE fstd_util FSTD_Status FSTD_CONCAT(fstd_module_instance_write_parameter_, suffix)(                   \
            FSTD_ModuleInstance * ctx, FSTD_StrConst module, FSTD_StrConst parameter, int value) {                     \
        return fstd_module_instance_write_parameter_opaque(ctx, tag, module, parameter, &value);                       \
    }

FSTD__MODULE_INSTANCE_PARAM(U8, u8)
FSTD__MODULE_INSTANCE_PARAM(U16, u16)
FSTD__MODULE_INSTANCE_PARAM(U32, u32)
FSTD__MODULE_INSTANCE_PARAM(U64, u64)
FSTD__MODULE_INSTANCE_PARAM(I8, i8)
FSTD__MODULE_INSTANCE_PARAM(I16, i16)
FSTD__MODULE_INSTANCE_PARAM(I32, i32)
FSTD__MODULE_INSTANCE_PARAM(I64, i64)

/// A root instance is a dynamically created "fake" module, which can not be depended from
/// by any other module. By their nature, root instances can not export any symbols, but can
/// depend on other modules and import their symbols dynamically.
typedef struct FSTD_ModuleRootInstance FSTD_ModuleRootInstance;

/// Constructs a new root instance.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_module_root_instance_init(FSTD_ModuleRootInstance **ctx);

/// Destroys the root module.
///
/// The handle may not be used afterwards.
fstd_util void fstd_module_root_instance_deinit(FSTD_ModuleRootInstance *ctx) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    fstd_module_handle_mark_unloadable(fstd_module_instance_handle(ctx_));
}

/// Checks the status of a namespace from the view of the module.
///
/// Checks if the module includes the namespace. In that case, the module is allowed access
/// to the symbols in the namespace. Additionally, this function also queries whether the
/// include is static, i.e., it was specified by the module at load time.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_query_namespace(FSTD_ModuleRootInstance *ctx,
                                                                               FSTD_StrConst ns,
                                                                               FSTD_ModuleDependency *dependency) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_query_namespace(ctx_, ns, dependency);
}

/// Adds a namespace dependency to the module.
///
/// Once added, the module gains access to the symbols of its dependencies that are
/// exposed in said namespace. A namespace can not be added multiple times.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_add_namespace(FSTD_ModuleRootInstance *ctx,
                                                                             FSTD_StrConst ns) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_add_namespace(ctx_, ns);
}

/// Removes a namespace dependency from the module.
///
/// Once excluded, the caller guarantees to relinquish access to the symbols contained in
/// said namespace. It is only possible to exclude namespaces that were manually added,
/// whereas static namespace dependencies remain valid until the module is unloaded.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_remove_namespace(FSTD_ModuleRootInstance *ctx,
                                                                                FSTD_StrConst ns) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_remove_namespace(ctx_, ns);
}

/// Checks if a module depends on another module.
///
/// Checks if the specified module is a dependency of the current instance. In that case
/// the instance is allowed to access the symbols exported by the module. Additionally,
/// this function also queries whether the dependency is static, i.e., the dependency was
/// specified by the module at load time.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_query_dependency(FSTD_ModuleRootInstance *ctx,
                                                                                FSTD_ModuleHandle *handle,
                                                                                FSTD_ModuleDependency *dependency) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_query_dependency(ctx_, handle, dependency);
}

/// Adds another module as a dependency.
///
/// After adding a module as a dependency, the module is allowed access to the symbols
/// and protected parameters of said dependency. Trying to adding a dependency to a module
/// that is already a dependency, or to a module that would result in a circular dependency
/// will result in an error.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_add_dependency(FSTD_ModuleRootInstance *ctx,
                                                                              FSTD_ModuleHandle *handle) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_add_dependency(ctx_, handle);
}

/// Removes a module as a dependency.
///
/// By removing a module as a dependency, the caller ensures that it does not own any
/// references to resources originating from the former dependency, and allows for the
/// unloading of the module. A module can only relinquish dependencies to modules that were
/// acquired dynamically, as static dependencies remain valid until the module is unloaded.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_remove_dependency(FSTD_ModuleRootInstance *ctx,
                                                                                 FSTD_ModuleHandle *handle) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_remove_dependency(ctx_, handle);
}

/// Loads a symbol from the module subsystem.
///
/// The caller can query the subsystem for a symbol of a loaded module. This is useful for
/// loading optional symbols, or for loading symbols after the creation of a module. The
/// symbol, if it exists, is returned, and can be used until the module relinquishes the
/// dependency to the module that exported the symbol. This function fails, if the module
/// containing the symbol is not a dependency of the module.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_load_symbol(FSTD_ModuleRootInstance *ctx,
                                                                           FSTD_ModuleSymbol symbol,
                                                                           const void **loaded) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_load_symbol(ctx_, symbol, loaded);
}

/// Reads a module parameter with dependency read access.
///
/// Reads the value of a module parameter with dependency read access. The operation fails,
/// if the parameter does not exist, or if the parameter does not allow reading with a
/// dependency access.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_read_parameter_opaque(FSTD_ModuleRootInstance *ctx,
                                                                                     FSTD_ModuleParamTag tag,
                                                                                     FSTD_StrConst module,
                                                                                     FSTD_StrConst parameter,
                                                                                     void *value) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_read_parameter_opaque(ctx_, tag, module, parameter, value);
}

/// Sets a module parameter with dependency write access.
///
/// Sets the value of a module parameter with dependency write access. The operation fails,
/// if the parameter does not exist, or if the parameter does not allow writing with a
/// dependency access.
FSTD_CHECK_USE fstd_util FSTD_Status fstd_module_root_instance_write_parameter_opaque(FSTD_ModuleRootInstance *ctx,
                                                                                      FSTD_ModuleParamTag tag,
                                                                                      FSTD_StrConst module,
                                                                                      FSTD_StrConst parameter,
                                                                                      const void *value) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_write_parameter_opaque(ctx_, tag, module, parameter, value);
}

#define FSTD__MODULE_ROOT_INSTANCE_PARAM(int, int_lower)                                                               \
    FSTD___MODULE_ROOT_INSTANCE_PARAM(FSTD_CONCAT(FSTD_, int), int_lower, FSTD_CONCAT(FSTD_ModuleParamTag_, int))
#define FSTD___MODULE_ROOT_INSTANCE_PARAM(int, suffix, tag)                                                            \
    FSTD_CHECK_USE fstd_util FSTD_Status FSTD_CONCAT(fstd_module_root_instance_read_parameter_, suffix)(               \
            FSTD_ModuleRootInstance * ctx, FSTD_StrConst module, FSTD_StrConst parameter, int *value) {                \
        return fstd_module_root_instance_read_parameter_opaque(ctx, tag, module, parameter, &value);                   \
    }                                                                                                                  \
    FSTD_CHECK_USE fstd_util FSTD_Status FSTD_CONCAT(fstd_module_root_instance_write_parameter_, suffix)(              \
            FSTD_ModuleRootInstance * ctx, FSTD_StrConst module, FSTD_StrConst parameter, int value) {                 \
        return fstd_module_root_instance_write_parameter_opaque(ctx, tag, module, parameter, &value);                  \
    }

FSTD__MODULE_ROOT_INSTANCE_PARAM(U8, u8)
FSTD__MODULE_ROOT_INSTANCE_PARAM(U16, u16)
FSTD__MODULE_ROOT_INSTANCE_PARAM(U32, u32)
FSTD__MODULE_ROOT_INSTANCE_PARAM(U64, u64)
FSTD__MODULE_ROOT_INSTANCE_PARAM(I8, i8)
FSTD__MODULE_ROOT_INSTANCE_PARAM(I16, i16)
FSTD__MODULE_ROOT_INSTANCE_PARAM(I32, i32)
FSTD__MODULE_ROOT_INSTANCE_PARAM(I64, i64)

/// Declaration of a module.
typedef struct FSTD_ModuleExport FSTD_ModuleExport;


/// Handle to a module loader.
///
/// Modules can only be loaded after all of their dependencies have been resolved uniquely.
/// A module loader batches the loading of multiple modules, procedurally determining an appropriate
/// loading order for as many modules as possible.
typedef struct FSTD_ModuleLoader FSTD_ModuleLoader;

typedef struct {
    FSTD_MAYBE_NULL FSTD_ModuleHandle *handle;
    const FSTD_ModuleExport *module;
} FSTD_ModuleLoaderResolvedModule;
typedef FSTD_Fallible(FSTD_ModuleLoaderResolvedModule) FSTD_ModuleLoaderPollModuleResult;

/// Operation of the filter function.
typedef FSTD_I32 FSTD_ModuleLoaderFilterRequest;
enum {
    FSTD_ModuleLoaderFilterRequest_Skip = (FSTD_ModuleLoaderFilterRequest)0,
    FSTD_ModuleLoaderFilterRequest_Load = (FSTD_ModuleLoaderFilterRequest)1,
    FSTD__ModuleLoaderFilterRequest_ = FSTD_I32_MAX,
};

typedef FSTD_ModuleLoaderFilterRequest (*FSTD_ModuleLoaderFilter)(FSTD_MAYBE_NULL void *data,
                                                                  const FSTD_ModuleExport *module);
typedef FSTD_OpaqueFuture(FSTD_Result) FSTD_ModuleLoaderCommitResult;

/// Constructs a new loader.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_module_loader_init(FSTD_ModuleLoader **loader);

/// Drops the loader.
///
/// Scheduled operations will be completed, but the caller invalidates their reference to the handle.
fstd__func void fstd_module_loader_deinit(FSTD_ModuleLoader *loader);

/// Checks whether the loader contains some module.
fstd__func bool fstd_module_loader_contains_module(FSTD_ModuleLoader *loader, FSTD_StrConst module);

/// Checks whether the loader contains some symbol.
fstd__func bool fstd_module_loader_contains_symbol(FSTD_ModuleLoader *loader, FSTD_ModuleSymbol symbol);

/// Polls the loader for the state of the specified module.
///
/// If the module has not been processed at the time of calling, the waker will be
/// signaled once the function can be polled again.
fstd__func bool fstd_module_loader_poll_module(FSTD_ModuleLoader *loader, FSTD_TaskWaker waker, FSTD_StrConst module,
                                               FSTD_ModuleLoaderPollModuleResult *result);

/// Adds a module to the loader.
///
/// Adds a module to the loader, so that it may be loaded by a future call to `commit`. Trying to
/// include an invalid module, a module with duplicate exports or duplicate name will result in
/// an error. This function allows for the loading of dynamic modules, i.e. modules that are
/// created at runtime, like non-native modules, which may require a runtime to be executed in.
/// The new module inherits a strong reference to the same binary as the caller's module.
///
/// Note that the new module is not setup to automatically depend on the owner, but may prevent
/// it from being unloaded while the loader exists.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_module_loader_add_module(FSTD_ModuleLoader *loader,
                                                                    FSTD_ModuleInstance *owner,
                                                                    const FSTD_ModuleExport *module);

/// Adds modules to the loader.
///
/// Opens up a module binary to select which modules to load.
/// If the path points to a file, the function will try to load the file.
/// If it points to a directory, it will search for a file named `module.fimo_module` in the same
/// directory.
///
/// The filter function can determine which modules to load.
/// Trying to load a module with duplicate exports or duplicate name will result in an error.
/// Invalid modules may not get passed to the filter function, and should therefore not be utilized
/// to list the modules contained in a binary.
///
/// This function returns an error, if the binary does not contain the symbols necessary to query
/// the exported modules, but does not return an error, if it does not export any modules.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_module_loader_add_modules_from_path(FSTD_ModuleLoader *loader,
                                                                               FSTD_Path path,
                                                                               FSTD_MAYBE_NULL void *filter_data,
                                                                               FSTD_ModuleLoaderFilter filter);

/// Adds modules to the loader.
///
/// Iterates over the exported modules of the current binary.
///
/// The filter function can determine which modules to load.
/// Trying to load a module with duplicate exports or duplicate name will result in an error.
/// Invalid modules may not get passed to the filter function, and should therefore not be utilized
/// to list the modules contained in a binary.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_module_loader_add_modules_from_iter(FSTD_ModuleLoader *loader,
                                                                               FSTD_MAYBE_NULL void *filter_data,
                                                                               FSTD_ModuleLoaderFilter filter);

/// Loads the modules contained in the loader.
///
/// If the returned future is successfull, the contained modules and their resources are made
/// available to the remaining modules. Some conditions may hinder the loading of some module,
/// like missing dependencies, duplicates, and other loading errors. In those cases, the
/// modules will be skipped without erroring.
///
/// It is possible to submit multiple concurrent commit requests, even from the same  loader.
/// In that case, the requests will be handled atomically, in an unspecified order.
FSTD_CHECK_USE fstd__func FSTD_ModuleLoaderCommitResult fstd_module_loader_commit(FSTD_ModuleLoader *loader);

typedef struct {
    FSTD_StrConst name;
    FSTD_ModuleParamTag tag;
    FSTD_ModuleAccesGroup read_group;
    FSTD_ModuleAccesGroup write_group;
    void (*read)(FSTD_ModuleParamData data, void *value);
    void (*write)(FSTD_ModuleParamData data, const void *value);
    union {
        FSTD_U8 u8;
        FSTD_U16 u16;
        FSTD_U32 u32;
        FSTD_U64 u64;
        FSTD_I8 i8;
        FSTD_I16 i16;
        FSTD_I32 i32;
        FSTD_I64 i64;
    };
} FSTD_ModuleExportParameter;

typedef FSTD_I32 FSTD_ModuleExportSymbolType;
enum {
    FSTD_ModuleExportSymbolType_Static = (FSTD_ModuleExportSymbolType)0,
    FSTD_ModuleExportSymbolType_Dynamic = (FSTD_ModuleExportSymbolType)1,
    FSTD__ModuleExportSymbolType_ = FSTD_I32_MAX,
};

typedef FSTD_I32 FSTD_ModuleExportSymbolLinkage;
enum {
    FSTD_ModuleExportSymbolLinkage_Global = (FSTD_ModuleExportSymbolLinkage)0,
    FSTD__ModuleExportSymbolLinkage_ = FSTD_I32_MAX,
};

typedef FSTD_Fallible(void *) FSTD_ModuleExportDynamicSymbolInitResult;
typedef struct {
    FSTD_ModuleSymbol symbol;
    FSTD_ModuleExportSymbolType type;
    FSTD_ModuleExportSymbolLinkage linkage;
    union {
        const void *static_value;
        struct {
            bool (*poll_init)(FSTD_ModuleInstance *ctx, FSTD_TaskWaker waker,
                              FSTD_ModuleExportDynamicSymbolInitResult *result);
            bool (*FSTD_MAYBE_NULL poll_deinit)(FSTD_ModuleInstance *ctx, FSTD_TaskWaker waker, void *value);
        } dynamic_value;
    };
} FSTD_ModuleExportSymbolExport;

/// Common member of all module events.
///
/// If a module supports an event, it must respond to the event by writing some
/// data into the provided event buffer.
typedef FSTD_I32 FSTD_ModuleExportEventTag;
enum {
    FSTD_ModuleExportEventTag_Init = (FSTD_ModuleExportEventTag)0,
    FSTD_ModuleExportEventTag_Deinit = (FSTD_ModuleExportEventTag)1,
    FSTD_ModuleExportEventTag_Start = (FSTD_ModuleExportEventTag)2,
    FSTD_ModuleExportEventTag_Stop = (FSTD_ModuleExportEventTag)3,
    FSTD_ModuleExportEventTag_DeinitExport = (FSTD_ModuleExportEventTag)4,
    FSTD_ModuleExportEventTag_Dependencies = (FSTD_ModuleExportEventTag)5,
    FSTD__ModuleExportEventTag_ = FSTD_I32_MAX,
};

typedef FSTD_Fallible(void *FSTD_MAYBE_NULL) FSTD_ModuleExportEventInitResult;
typedef struct {
    FSTD_ModuleExportEventTag tag;
    bool (*FSTD_MAYBE_NULL poll)(FSTD_ModuleInstance *ctx, FSTD_ModuleLoader *loader, FSTD_TaskWaker waker,
                                 FSTD_ModuleExportEventInitResult *state);
} FSTD_ModuleExportEventInit;

typedef struct {
    FSTD_ModuleExportEventTag tag;
    bool (*FSTD_MAYBE_NULL poll)(FSTD_ModuleInstance *ctx, FSTD_TaskWaker waker, void *state);
} FSTD_ModuleExportEventDeinit;

typedef struct {
    FSTD_ModuleExportEventTag tag;
    bool (*FSTD_MAYBE_NULL poll)(FSTD_ModuleInstance *ctx, FSTD_TaskWaker waker, FSTD_Result *result);
} FSTD_ModuleExportEventStart;

typedef struct {
    FSTD_ModuleExportEventTag tag;
    bool (*FSTD_MAYBE_NULL poll)(FSTD_ModuleInstance *ctx, FSTD_TaskWaker waker);
} FSTD_ModuleExportEventStop;

typedef struct {
    FSTD_ModuleExportEventTag tag;
    void *FSTD_MAYBE_NULL data;
    void (*FSTD_MAYBE_NULL deinit)(void *data);
} FSTD_ModuleExportEventDeinitExport;

typedef FSTD_SliceConst(FSTD_ModuleHandle *) FSTD_ModuleExportEventDependenciesHandles;
typedef struct {
    FSTD_ModuleExportEventTag tag;
    FSTD_ModuleExportEventDependenciesHandles handles;
} FSTD_ModuleExportEventDependencies;

typedef FSTD_SliceConst(FSTD_ModuleExportParameter) FSTD_ModuleExportParameters;
typedef FSTD_SliceConst(FSTD_Path) FSTD_ModuleExportResources;
typedef FSTD_SliceConst(FSTD_StrConst) FSTD_ModuleExportNamespaces;
typedef FSTD_SliceConst(FSTD_ModuleSymbol) FSTD_ModuleExportSymbolImports;
typedef FSTD_SliceConst(FSTD_ModuleExportSymbolExport) FSTD_ModuleExportSymbolExports;

struct FSTD_ModuleExport {
    FSTD_Version version;
    FSTD_StrConst name;
    FSTD_StrConst description;
    FSTD_StrConst author;
    FSTD_StrConst license;
    FSTD_ModuleExportParameters parameters;
    FSTD_ModuleExportResources resources;
    FSTD_ModuleExportNamespaces namespaces;
    FSTD_ModuleExportSymbolImports imports;
    FSTD_ModuleExportSymbolExports exports;
    void (*on_event)(const FSTD_ModuleExport *module, FSTD_ModuleExportEventTag *tag);
};

fstd_util FSTD_ModuleExportEventInit fstd_module_export_event_init(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventInit ev = {.tag = FSTD_ModuleExportEventTag_Init};
    module->on_event(module, &ev.tag);
    return ev;
}

fstd_util FSTD_ModuleExportEventDeinit fstd_module_export_event_deinit(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventDeinit ev = {.tag = FSTD_ModuleExportEventTag_Deinit};
    module->on_event(module, &ev.tag);
    return ev;
}

fstd_util FSTD_ModuleExportEventStart fstd_module_export_event_start(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventStart ev = {.tag = FSTD_ModuleExportEventTag_Start};
    module->on_event(module, &ev.tag);
    return ev;
}

fstd_util FSTD_ModuleExportEventStop fstd_module_export_event_stop(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventStop ev = {.tag = FSTD_ModuleExportEventTag_Stop};
    module->on_event(module, &ev.tag);
    return ev;
}

fstd_util FSTD_ModuleExportEventDeinitExport fstd_module_export_event_deinit_export(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventDeinitExport ev = {.tag = FSTD_ModuleExportEventTag_DeinitExport};
    module->on_event(module, &ev.tag);
    return ev;
}

fstd_util FSTD_ModuleExportEventDependencies fstd_module_export_event_dependencies(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventDependencies ev = {.tag = FSTD_ModuleExportEventTag_Dependencies};
    module->on_event(module, &ev.tag);
    return ev;
}

#ifdef FSTD_PLATFORM_WINDOWS
// With the MSVC we have no way to get the start and end of
// a section, so we use three different sections. According
// to the documentation, the linker orders the entries with
// the same section prefix by the section name. Therefore,
// we can get the same result by allocating all entries in
// the middle section.
#pragma section("fi_mod$a", read)
#pragma section("fi_mod$u", read)
#pragma section("fi_mod$z", read)

#define FSTD__MODULE_SECTION "fi_mod$u"
#elif defined(FSTD_PLATFORM_APPLE)
#define FSTD__MODULE_SECTION "__DATA,fimo_module"
#elif defined(FSTD_PLATFORM_LINUX)
#define FSTD__MODULE_SECTION "fimo_module"
#endif

#define FSTD_SYMBOL(prefix, type, name)                                                                                \
    fstd__func const type *FSTD_CONCAT(FSTD_CONCAT(prefix, name), _get)(void);                                         \
    fstd__func void FSTD_CONCAT(FSTD_CONCAT(prefix, name), _register)(const type *ptr);                                \
    fstd__func void FSTD_CONCAT(FSTD_CONCAT(prefix, name), _unregister)(void);                                         \
    fstd__glob FSTD__RefCountedHandle FSTD_CONCAT(prefix, name);

#define FSTD_SYMBOL_FN(prefix, ret, name, ...)                                                                         \
    fstd__func ret (*FSTD_CONCAT(FSTD_CONCAT(prefix, name), _get)(__VA_ARGS__))(void);                                 \
    fstd__func void FSTD_CONCAT(FSTD_CONCAT(prefix, name), _register)(ret(*const ptr)(__VA_ARGS__));                   \
    fstd__func void FSTD_CONCAT(FSTD_CONCAT(prefix, name), _unregister)(void);                                         \
    fstd__glob FSTD__RefCountedHandle FSTD_CONCAT(prefix, name);

#define FSTD_SYMBOL_IMPL(prefix, type, name)                                                                           \
    fstd__func_impl const type *FSTD_CONCAT(FSTD_CONCAT(prefix, name), _get)(void) {                                   \
        return (const type *)FSTD_CONCAT(prefix, name).handle;                                                         \
    }                                                                                                                  \
    fstd__func_impl void FSTD_CONCAT(FSTD_CONCAT(fstd___symbol_, name), _register)(const type *ptr) {                  \
        fstd__ref_counted_handle_register(&FSTD_CONCAT(prefix, name), (const void *)ptr);                              \
    }                                                                                                                  \
    fstd__func_impl void FSTD_CONCAT(FSTD_CONCAT(fstd___symbol_, name), _unregister)(void) {                           \
        fstd__ref_counted_handle_unregister(&FSTD_CONCAT(prefix, name));                                               \
    }                                                                                                                  \
    fstd__glob_impl FSTD__RefCountedHandle FSTD_CONCAT(prefix, name);


#define FSTD_SYMBOL_FN_IMPL(prefix, ret, name, ...)                                                                    \
    fstd__func_impl ret (*FSTD_CONCAT(FSTD_CONCAT(prefix, name), _get)(__VA_ARGS__))(void) {                           \
        return (ret (*)(__VA_ARGS__))FSTD_CONCAT(prefix, name).handle;                                                 \
    }                                                                                                                  \
    fstd__func_impl void FSTD_CONCAT(FSTD_CONCAT(fstd___symbol_, name), _register)(ret(*const ptr)(__VA_ARGS__)) {     \
        fstd__ref_counted_handle_register(&FSTD_CONCAT(prefix, name), (const void *)ptr);                              \
    }                                                                                                                  \
    fstd__func_impl void FSTD_CONCAT(FSTD_CONCAT(fstd___symbol_, name), _unregister)(void) {                           \
        fstd__ref_counted_handle_unregister(&FSTD_CONCAT(prefix, name));                                               \
    }                                                                                                                  \
    fstd__glob_impl FSTD__RefCountedHandle FSTD_CONCAT(prefix, name);

#define FSTD_MODULE_EXPORT() FSTD_MODULE_EXPORT_NAMED(FSTD_IDENT(fstd__module_export_))
#define FSTD_MODULE_EXPORT_NAMED(name) FSTD__MODULE_EXPORT(name, FSTD_IDENT(FSTD_CONCAT(fstd__module_export_, name)))
#define FSTD__MODULE_EXPORT(name, private)                                                                             \
    fstd_external const FSTD_ModuleExport private;                                                                     \
    FSTD___MODULE_EXPORT(name, &private);                                                                              \
    const FSTD_ModuleExport private
#ifdef FSTD_PLATFORM_WINDOWS
#define FSTD___MODULE_EXPORT(name, expr)                                                                               \
    __declspec(allocate(FSTD__MODULE_SECTION)) const FSTD_ModuleExport *name = (expr)
#else
#define FSTD___MODULE_EXPORT(name, expr)                                                                               \
    const FSTD_ModuleExport *name __attribute__((retain, used, section(FSTD__MODULE_SECTION))) = (expr)
#endif

typedef bool (*FSTD_ModuleExportIterInspector)(void *FSTD_MAYBE_NULL ctx, const FSTD_ModuleExport *module);
typedef void (*FSTD_ModuleExportIter)(void *FSTD_MAYBE_NULL ctx, FSTD_ModuleExportIterInspector inspector);
fstd_external void fstd__module_export_iter(void *FSTD_MAYBE_NULL ctx, FSTD_ModuleExportIterInspector inspector);

/// Profile of the module subsystem.
///
/// Each profile enables a set of default features.
typedef FSTD_I32 FSTD_ModulesProfile;
enum {
    FSTD_ModulesProfile_Default = (FSTD_ModulesProfile)0,
    FSTD_ModulesProfile_Release = (FSTD_ModulesProfile)1,
    FSTD_ModulesProfile_Dev = (FSTD_ModulesProfile)2,
    FSTD__ModulesProfile_ = FSTD_I32_MAX,
};

/// Optional features recognized by the module subsystem.
///
/// Some features may be mutually exclusive, while other may
/// require additional feature dependencies.
typedef FSTD_U16 FSTD_ModulesFeatureTag;
// NOLINTNEXTLINE
enum {
    FSTD__ModulesFeatureTag_ = FSTD_U16_MAX,
};

typedef FSTD_U16 FSTD_ModulesFeatureRequestFlag;
// NOLINTNEXTLINE
enum {
    FSTD_ModulesFeatureRequestFlag_Required = (FSTD_ModulesFeatureRequestFlag)0,
    FSTD_ModulesFeatureRequestFlag_On = (FSTD_ModulesFeatureRequestFlag)1,
    FSTD_ModulesFeatureRequestFlag_Off = (FSTD_ModulesFeatureRequestFlag)2,
    FSTD__ModulesFeatureRequestFlag_ = FSTD_U16_MAX,
};

/// Request for an optional feature.
typedef struct {
    FSTD_ModulesFeatureTag tag;
    FSTD_ModulesFeatureRequestFlag flag;
} FSTD_ModulesFeatureRequest;

typedef FSTD_U16 FSTD_ModulesFeatureStatusFlag;
// NOLINTNEXTLINE
enum {
    FSTD_ModulesFeatureStatusFlag_On = (FSTD_ModulesFeatureStatusFlag)0,
    FSTD_ModulesFeatureStatusFlag_Off = (FSTD_ModulesFeatureStatusFlag)1,
    FSTD__ModulesFeatureStatusFlag_ = FSTD_U16_MAX,
};

/// Status of an optional feature.
typedef struct {
    FSTD_ModulesFeatureTag tag;
    FSTD_ModulesFeatureStatusFlag flag;
} FSTD_ModulesFeatureStatus;

typedef FSTD_SliceConst(FSTD_ModulesFeatureRequest) FSTD_ModulesFeatureRequests;
typedef FSTD_SliceConst(FSTD_ModulesFeatureStatus) FSTD_ModulesFeatureStatuses;

#ifndef FSTD_DEBUG
#define FSTD_MODULES_DEFAULT_PROFILE FSTD_ModulesProfile_Release
#else
#define FSTD_MODULES_DEFAULT_PROFILE FSTD_ModulesProfile_Dev
#endif

typedef struct {
    FSTD_Cfg id;
    FSTD_ModulesProfile profile;
    FSTD_ModulesFeatureRequests features;
} FSTD_ModulesCfg;

typedef struct {
    FSTD_ModulesProfile (*profile)(void);
    FSTD_ModulesFeatureStatuses (*features)(void);
    FSTD_Status (*root_instance_init)(FSTD_ModuleRootInstance **ctx);
    FSTD_Status (*loader_init)(FSTD_ModuleLoader **loader);
    void (*loader_deinit)(FSTD_ModuleLoader *loader);
    bool (*loader_contains_module)(FSTD_ModuleLoader *loader, FSTD_StrConst module);
    bool (*loader_contains_symbol)(FSTD_ModuleLoader *loader, FSTD_ModuleSymbol symbol);
    bool (*loader_poll_module)(FSTD_ModuleLoader *loader, FSTD_TaskWaker waker, FSTD_StrConst module,
                               FSTD_ModuleLoaderPollModuleResult *result);
    FSTD_Status (*loader_add_module)(FSTD_ModuleLoader *loader, FSTD_ModuleInstance *owner,
                                     const FSTD_ModuleExport *module);
    FSTD_Status (*loader_add_modules_from_path)(FSTD_ModuleLoader *loader, FSTD_Path path,
                                                void *FSTD_MAYBE_NULL filter_data, FSTD_ModuleLoaderFilter filter);
    FSTD_Status (*loader_add_modules_from_iter)(FSTD_ModuleLoader *loader, void *FSTD_MAYBE_NULL filter_data,
                                                FSTD_ModuleLoaderFilter filter, FSTD_ModuleExportIter iterator,
                                                const void *bin_ptr);
    FSTD_ModuleLoaderCommitResult (*loader_commit)(FSTD_ModuleLoader *loader);
    FSTD_Status (*handle_find_by_name)(FSTD_ModuleHandle **handle, FSTD_StrConst module);
    FSTD_Status (*handle_find_by_symbol)(FSTD_ModuleHandle **handle, FSTD_ModuleSymbol symbol);
    bool (*namespace_exists)(FSTD_StrConst ns);
    FSTD_Status (*prune_instances)(void);
    FSTD_Status (*query_parameter)(FSTD_StrConst module, FSTD_StrConst parameter, FSTD_ModuleParamInfo *info);
    FSTD_Status (*read_parameter)(FSTD_ModuleParamTag tag, FSTD_StrConst module, FSTD_StrConst parameter, void *value);
    FSTD_Status (*write_parameter)(FSTD_ModuleParamTag tag, FSTD_StrConst module, FSTD_StrConst parameter,
                                   const void *value);
} FSTD_ModulesVtable;

/// Returns the active profile of the module subsystem.
fstd__func FSTD_ModulesProfile fstd_modules_profile(void);

/// Returns the status of all features known to the subsystem.
fstd__func FSTD_ModulesFeatureStatuses fstd_modules_features(void);

/// Checks for the presence of a namespace in the module subsystem.
///
/// A namespace exists, if at least one loaded module exports one symbol in said namespace.
fstd__func bool fstd_modules_namespace_exists(FSTD_StrConst ns);

/// Marks all instances as unloadable.
///
/// Tries to unload all instances that are not referenced by any other modules. If the instance is
/// still referenced, this will mark the instance as unloadable and enqueue it for unloading.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_modules_prune_instances(void);

/// Queries the info of a module parameter.
///
/// This function can be used to query the datatype, the read access, and the write access of a
/// module parameter. This function fails, if the parameter can not be found.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_modules_query_parameter(FSTD_StrConst module, FSTD_StrConst parameter,
                                                                   FSTD_ModuleParamInfo *info);

/// Reads a module parameter with public read access.
///
/// Reads the value of a module parameter with public read access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow reading with a public access.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_modules_read_parameter_opaque(FSTD_ModuleParamTag tag, FSTD_StrConst module,
                                                                         FSTD_StrConst parameter, void *value);

/// Sets a module parameter with public write access.
///
/// Sets the value of a module parameter with public write access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow writing with a public access.
FSTD_CHECK_USE fstd__func FSTD_Status fstd_modules_write_parameter_opaque(FSTD_ModuleParamTag tag, FSTD_StrConst module,
                                                                          FSTD_StrConst parameter, const void *value);

#define FSTD__MODULES_PARAM(int, int_lower)                                                                            \
    FSTD___MODULES_PARAM(FSTD_CONCAT(FSTD_, int), int_lower, FSTD_CONCAT(FSTD_ModuleParamTag_, int))
#define FSTD___MODULES_PARAM(int, suffix, tag)                                                                         \
    FSTD_CHECK_USE fstd_util FSTD_Status FSTD_CONCAT(fstd_modules_read_parameter_, suffix)(                            \
            FSTD_StrConst module, FSTD_StrConst parameter, int *value) {                                               \
        return fstd_modules_read_parameter_opaque(tag, module, parameter, &value);                                     \
    }                                                                                                                  \
    FSTD_CHECK_USE fstd_util FSTD_Status FSTD_CONCAT(fstd_modules_write_parameter_, suffix)(                           \
            FSTD_StrConst module, FSTD_StrConst parameter, int value) {                                                \
        return fstd_modules_write_parameter_opaque(tag, module, parameter, &value);                                    \
    }

FSTD__MODULES_PARAM(U8, u8)
FSTD__MODULES_PARAM(U16, u16)
FSTD__MODULES_PARAM(U32, u32)
FSTD__MODULES_PARAM(U64, u64)
FSTD__MODULES_PARAM(I8, i8)
FSTD__MODULES_PARAM(I16, i16)
FSTD__MODULES_PARAM(I32, i32)
FSTD__MODULES_PARAM(I64, i64)

// -----------------------------------------
// handle ----------------------------------
// -----------------------------------------

struct FSTD_Ctx {
    FSTD_Version (*get_version)(void);
    FSTD_CoreVtable core_v0;
    FSTD_TracingVtable tracing_v0;
    FSTD_ModulesVtable modules_v0;
    FSTD_TasksVtable tasks_v0;
};

FSTD_PRAGMA_GCC(GCC diagnostic pop)

#ifdef __cplusplus
}
#endif

#endif // FIMO_STD_HEADER

#ifdef FIMO_STD_IMPLEMENTATION

#ifdef __cplusplus
extern "C" {
#endif

// -----------------------------------------
// context api -----------------------------
// -----------------------------------------

fstd__glob_impl FSTD__Ctx fstd__ctx_global;

// NOTE(gabriel): Is sound, as while registered the pointer won't change.
fstd__func_impl FSTD_Ctx *fstd_ctx_get(void) { return fstd__ctx_global.ctx; }

fstd__func_impl void fstd_ctx_register(FSTD_Ctx *ctx) {
    fstd__ref_counted_handle_register(&fstd__ctx_global.handle, ctx);
}

fstd__func_impl void fstd_ctx_unregister(void) { fstd__ref_counted_handle_unregister(&fstd__ctx_global.handle); }

fstd__func_impl void fstd_ctx_deinit(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->core_v0.deinit();
    fstd_ctx_unregister();
}

fstd__func_impl FSTD_Version fstd_ctx_get_version(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->get_version();
}

fstd__func_impl bool fstd_ctx_has_error_result(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->core_v0.has_error_result();
}

fstd__func_impl FSTD_Result fstd_ctx_replace_result(FSTD_Result new_result) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->core_v0.replace_result(new_result);
}

fstd__func_impl FSTD_Result fstd_ctx_take_result(void) { return fstd_ctx_replace_result(FSTD_Result_Ok); }

fstd__func_impl void fstd_ctx_clear_result(void) { fstd_result_deinit(fstd_ctx_take_result()); }

fstd__func_impl void fstd_ctx_set_result(FSTD_Result new_result) {
    fstd_result_deinit(fstd_ctx_replace_result(new_result));
}

// -----------------------------------------
// async subsystem -------------------------
// -----------------------------------------

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_waiter_init(FSTD_TaskWaiter *waiter) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tasks_v0.waiter_init(waiter);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_future_enqueue(const void *data, FSTD_USize data_size,
                                                               FSTD_USize data_alignment, FSTD_USize result_size,
                                                               FSTD_USize result_alignment, FSTD_TaskWaiterPollFn poll,
                                                               FSTD_TaskDeinitFn deinit_data,
                                                               FSTD_TaskDeinitFn deinit_result,
                                                               FSTD_EnqueuedFuture *future) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tasks_v0.future_enqueue(data, data_size, data_alignment, result_size, result_alignment, poll,
                                           deinit_data, deinit_result, future);
}

// -----------------------------------------
// tracing subsystem -----------------------
// -----------------------------------------

fstd__func_impl FSTD_CallStack *fstd_call_stack_init(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tracing_v0.init_call_stack();
}

fstd__func_impl void fstd_call_stack_finish(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.deinit_call_stack(stack, false);
}

fstd__func_impl void fstd_call_stack_abort(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.deinit_call_stack(stack, true);
}

fstd__func_impl FSTD_CallStack *fstd_call_stack_replace_current(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tracing_v0.replace_current_call_stack(stack);
}

fstd__func_impl void fstd_call_stack_unblock(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.unblock_call_stack(stack);
}

fstd__func_impl void fstd_call_stack_suspend_current(bool mark_blocked) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.suspend_current_call_stack(mark_blocked);
}

fstd__func_impl void fstd_call_stack_resume_current(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.resume_current_call_stack();
}

fstd__func_impl void fstd_tracing_is_enabled(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.is_enabled();
}

fstd__func_impl void fstd_tracing_register_thread(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.register_thread();
}

fstd__func_impl void fstd_tracing_unregister_thread(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.unregister_thread();
}

fstd__func_impl void fstd_tracing_enter_span(const FSTD_TracingEventInfo *info, fstd_tracing_fmt_fn fmt,
                                             const void *fmt_data) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.enter_span(info, fmt, fmt_data);
}

fstd__func_impl void fstd_tracing_exit_span(const FSTD_TracingEventInfo *info) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.exit_span(info);
}

fstd__func_impl void fstd_tracing_log_message(const FSTD_TracingEventInfo *info, fstd_tracing_fmt_fn fmt,
                                              const void *fmt_data) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.log_message(info, fmt, fmt_data);
}

// -----------------------------------------
// modules subsystem -----------------------
// -----------------------------------------

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_module_handle_find_by_name(FSTD_ModuleHandle **handle,
                                                                           FSTD_StrConst module) {
    FSTD_Ctx *h = fstd_ctx_get();
    return h->modules_v0.handle_find_by_name(handle, module);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_module_handle_find_by_symbol(FSTD_ModuleHandle **handle,
                                                                             FSTD_ModuleSymbol symbol) {
    FSTD_Ctx *h = fstd_ctx_get();
    return h->modules_v0.handle_find_by_symbol(handle, symbol);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_module_root_instance_init(FSTD_ModuleRootInstance **ctx) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.root_instance_init(ctx);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_module_loader_init(FSTD_ModuleLoader **loader) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_init(loader);
}

fstd__func_impl void fstd_module_loader_deinit(FSTD_ModuleLoader *loader) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->modules_v0.loader_deinit(loader);
}

fstd__func_impl bool fstd_module_loader_contains_module(FSTD_ModuleLoader *loader, FSTD_StrConst module) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_contains_module(loader, module);
}

fstd__func_impl bool fstd_module_loader_contains_symbol(FSTD_ModuleLoader *loader, FSTD_ModuleSymbol symbol) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_contains_symbol(loader, symbol);
}

fstd__func_impl bool fstd_module_loader_poll_module(FSTD_ModuleLoader *loader, FSTD_TaskWaker waker,
                                                    FSTD_StrConst module, FSTD_ModuleLoaderPollModuleResult *result) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_poll_module(loader, waker, module, result);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_module_loader_add_module(FSTD_ModuleLoader *loader,
                                                                         FSTD_ModuleInstance *owner,
                                                                         const FSTD_ModuleExport *module) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_add_module(loader, owner, module);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_module_loader_add_modules_from_path(FSTD_ModuleLoader *loader,
                                                                                    FSTD_Path path, void *filter_data,
                                                                                    FSTD_ModuleLoaderFilter filter) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_add_modules_from_path(loader, path, filter_data, filter);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_module_loader_add_modules_from_iter(FSTD_ModuleLoader *loader,
                                                                                    void *filter_data,
                                                                                    FSTD_ModuleLoaderFilter filter) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_add_modules_from_iter(loader, filter_data, filter, fstd__module_export_iter,
                                                           (const void *)fstd__module_export_iter);
}

FSTD_CHECK_USE fstd__func_impl FSTD_ModuleLoaderCommitResult fstd_module_loader_commit(FSTD_ModuleLoader *loader) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_commit(loader);
}

fstd__func_impl FSTD_ModulesProfile fstd_modules_profile(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.profile();
}

fstd__func_impl FSTD_ModulesFeatureStatuses fstd_modules_features(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.features();
}

fstd__func_impl bool fstd_modules_namespace_exists(FSTD_StrConst ns) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.namespace_exists(ns);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_modules_prune_instances(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.prune_instances();
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_modules_query_parameter(FSTD_StrConst module, FSTD_StrConst parameter,
                                                                        FSTD_ModuleParamInfo *info) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.query_parameter(module, parameter, info);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_modules_read_parameter_opaque(FSTD_ModuleParamTag tag,
                                                                              FSTD_StrConst module,
                                                                              FSTD_StrConst parameter, void *value) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.read_parameter(tag, module, parameter, value);
}

FSTD_CHECK_USE fstd__func_impl FSTD_Status fstd_modules_write_parameter_opaque(FSTD_ModuleParamTag tag,
                                                                               FSTD_StrConst module,
                                                                               FSTD_StrConst parameter,
                                                                               const void *value) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.write_parameter(tag, module, parameter, value);
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_STD_IMPLEMENTATION

/// LICENSE
/// MIT License
///
/// Copyright (c) 2025 Gabriel Borrelli
///
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
///
/// The above copyright notice and this permission notice shall be included in all
/// copies or substantial portions of the Software.
///
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
/// SOFTWARE.
