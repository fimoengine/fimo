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
#include <stdatomic.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifndef FSTD_NO_STDIO
#include <stdio.h>
#endif

#ifndef FSTD_NO_STDLIB
#include <stdlib.h>
#endif

#if (defined(_M_AMD64) && !defined(_M_ARM64EC)) || defined(__x86_64__)
#define FSTD_ARCH_X86_64
#elif defined(_M_ARM64) || defined(_M_ARM64EC) || defined(__aarch64__)
#define FSTD_ARCH_AARCH64
#else
#error "unknown architecture"
#endif

#if defined(_WIN32)
#define FSTD_PLATFORM_WINDOWS 1
#elif defined(__APPLE__)
#define FSTD_PLATFORM_APPLE 1
#define FSTD_PLATFORM_POSIX 1
#elif defined(__linux__)
#define FSTD_PLATFORM_LINUX 1
#define FSTD_PLATFORM_POSIX 1
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

#ifndef FSTD_COMPILER_GCC_COMPATIBLE
#include <string.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

// -----------------------------------------
// global utilities ------------------------
// -----------------------------------------

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
#define FSTD_EXPAND_GCC_COMPATIBLE(x) x
#else
#define FSTD_EXPAND_GCC_COMPATIBLE(x)
#endif

#if defined(FSTD_COMPILER_MSC_COMPATIBLE)
#define FSTD_EXPAND_MSC_COMPATIBLE(x) x
#else
#define FSTD_EXPAND_MSC_COMPATIBLE(x)
#endif

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

#ifdef __cplusplus
#define FSTD_CONSTEXPR constexpr
#else
#define FSTD_CONSTEXPR const
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
#define fstd_util static inline

#ifdef FSTD_STATIC
#define fstd_glob static
#define fstd_func static
#define fstd_glob_impl static
#define fstd_func_impl static
#else
#define fstd_glob extern
#define fstd_func extern
#define fstd_glob_impl
#define fstd_func_impl
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
#define FSTD_PTR_64 1
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
    v |= v >> 32;
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
    _Atomic(FSTD_USize) count;
} FSTD__RefCountedHandle;
fstd_static_assert(sizeof(FSTD__RefCountedHandle) == 2 * sizeof(FSTD_USize), "invalid FSTD__RefCountedHandle size");
fstd_static_assert(alignof(FSTD__RefCountedHandle) == alignof(FSTD_USize), "invalid FSTD__RefCountedHandle align");

#ifdef FSTD_PTR_64
#define FSTD__REF_COUNTED_HANDLE_LOCKED ((FSTD_USize)1) << 63
#else
#error "unsupported platform"
#endif

fstd_util void fstd__ref_counted_handle_register(FSTD__RefCountedHandle *ref, const void *handle) {
    FSTD_USize locked = FSTD__REF_COUNTED_HANDLE_LOCKED;
    while ((atomic_fetch_or_explicit(&ref->count, locked, memory_order_acquire) & locked) != 0) {
    }

    FSTD_USize count = ref->count & ~locked;
    fstd_dbg_assert(count < locked - 1);
    fstd_dbg_assert(ref->handle == fstd_nullptr || ref->handle == handle);
    fstd_dbg_assert(ref->handle == fstd_nullptr || count > 0);
    ref->handle = handle;
    ref->count += 1;

    atomic_fetch_and_explicit(&ref->count, ~locked, memory_order_release);
}

fstd_util void fstd__ref_counted_handle_unregister(FSTD__RefCountedHandle *ref) {
    FSTD_USize locked = FSTD__REF_COUNTED_HANDLE_LOCKED;
    while ((atomic_fetch_or_explicit(&ref->count, locked, memory_order_acquire) & locked) != 0) {
    }

    FSTD_USize count = ref->count & ~locked;
    fstd_dbg_assert(count > 0);
    fstd_dbg_assert(ref->handle != fstd_nullptr);
    ref->count -= 1;
    if (count == 1)
        ref->handle = fstd_nullptr;

    atomic_fetch_and_explicit(&ref->count, ~locked, memory_order_release);
}

// -----------------------------------------
// memory ----------------------------------
// -----------------------------------------

typedef FSTD_Slice(FSTD_U8) FSTD_MemorySlice;

typedef struct {
    /// Allocates a new buffer.
    void *FSTD_MAYBE_NULL (*alloc)(void *FSTD_MAYBE_NULL data, FSTD_USize len, FSTD_USize align);
    /// Tries to resize the buffer in place.
    bool (*resize)(void *FSTD_MAYBE_NULL data, FSTD_MemorySlice memory, FSTD_USize align, FSTD_USize new_len);
    /// Resizes the buffer, allowing relocation.
    void *FSTD_MAYBE_NULL (*remap)(void *FSTD_MAYBE_NULL data, FSTD_MemorySlice memory, FSTD_USize align,
                                   FSTD_USize new_len);
    /// Frees a previously allocated buffer.
    void (*free)(void *FSTD_MAYBE_NULL data, FSTD_MemorySlice memory, FSTD_USize align);
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
    FSTD_MemorySlice memory = {.ptr = (FSTD_U8 *)ptr, .len = len};
    return alloc.vtable->resize(alloc.ptr, memory, align, new_len);
}

#define fstd_allocator_remap(alloc, ptr, n, new_n)                                                                     \
    (type *)fstd__allocator_remap((alloc), (ptr), sizeof(*ptr) * (n), fstd__alignof(*ptr), sizeof(*ptr) * (new_n))
FSTD_ALLOC fstd_util void *FSTD_MAYBE_NULL fstd__allocator_remap(FSTD_Allocator alloc, void *ptr, FSTD_USize len,
                                                                 FSTD_USize align, FSTD_USize new_len) {
    FSTD_MemorySlice memory = {.ptr = (FSTD_U8 *)ptr, .len = len};
    return alloc.vtable->remap(alloc.ptr, memory, align, new_len);
}

#define fstd_allocator_free(alloc, ptr, n) fstd__allocator_free((alloc), (ptr), sizeof(*ptr) * (n), fstd__alignof(*ptr))
fstd_util void fstd__allocator_free(FSTD_Allocator alloc, void *ptr, FSTD_USize len, FSTD_USize align) {
    FSTD_MemorySlice memory = {.ptr = (FSTD_U8 *)ptr, .len = len};
    alloc.vtable->free(alloc.ptr, memory, align);
}

FSTD_ALLOC fstd_util void *FSTD_MAYBE_NULL fstd__allocator_null_alloc(void *FSTD_MAYBE_NULL arg0, FSTD_USize arg1,
                                                                      FSTD_USize arg2) {
    FSTD_UNUSED(arg0, arg1, arg2);
    return fstd_nullptr;
}
fstd_util bool fstd__allocator_null_resize(void *FSTD_MAYBE_NULL arg0, FSTD_MemorySlice arg1, FSTD_USize arg2,
                                           FSTD_USize arg3) {
    FSTD_UNUSED(arg0, arg1, arg2, arg3);
    return false;
}
FSTD_ALLOC fstd_util FSTD_MAYBE_NULL void *fstd__allocator_null_remap(void *FSTD_MAYBE_NULL arg0, FSTD_MemorySlice arg1,
                                                                      FSTD_USize arg2, FSTD_USize arg3) {
    FSTD_UNUSED(arg0, arg1, arg2, arg3);
    return fstd_nullptr;
}
fstd_util void fstd__allocator_null_free(void *FSTD_MAYBE_NULL arg0, FSTD_MemorySlice arg1, FSTD_USize arg2) {
    FSTD_UNUSED(arg0, arg1, arg2);
}

fstd_internal FSTD_CONSTEXPR FSTD_AllocatorVtable FSTD__AllocatorVtable_Null = {
        .alloc = fstd__allocator_null_alloc,
        .resize = fstd__allocator_null_resize,
        .remap = fstd__allocator_null_remap,
        .free = fstd__allocator_null_free,
};

/// An allocator which does not allocate or free any memory.
fstd_internal FSTD_CONSTEXPR FSTD_Allocator FSTD_Allocator_Null = {
        .ptr = fstd_nullptr,
        .vtable = &FSTD__AllocatorVtable_Null,
};

typedef FSTD_U32 FSTD_ArenaFlags;
enum {
    FSTD_ArenaFlags_LargePages = (1 << 0),
    FSTD__ArenaFlags_ = FSTD_I32_MAX,
};

#define FSTD_ARENA_MIN_ALIGN 16

/// A growable thread-safe memory arena.
typedef struct {
    _Atomic(FSTD_U32) grow_futex;
    FSTD_ArenaFlags flags;
    FSTD_USize page_size;
    FSTD_USize reserve_len;
    _Atomic(FSTD_USize) commit_len;
    void *FSTD_MAYBE_NULL ptr;
    _Atomic(FSTD_USize) pos;
} FSTD_Arena;
fstd_static_assert(sizeof(_Atomic(FSTD_U32)) == sizeof(FSTD_U32), "invalid atomic size");
fstd_static_assert(sizeof(_Atomic(FSTD_USize)) == sizeof(FSTD_USize), "invalid atomic size");
fstd_static_assert(alignof(_Atomic(FSTD_U32)) == alignof(FSTD_U32), "invalid atomic align");
fstd_static_assert(alignof(_Atomic(FSTD_USize)) == alignof(FSTD_USize), "invalid atomic align");

/// Temporary scope of a memory arena.
typedef struct {
    FSTD_Arena *arena;
    FSTD_USize pos;
} FSTD_TmpArena;

/// Allocates a new arena. Returns whether the operation is successfull.
///
/// The `base` argument is reserved and must be `null`.
fstd_external bool fstd_arena_init(FSTD_Arena *arena, void *base, FSTD_ArenaFlags flags, FSTD_USize reserve,
                                   FSTD_USize commit);

/// Frees the resources of the arena.
fstd_external void fstd_arena_deinit(FSTD_Arena *arena);

/// Tries to grow the arena inplace.
fstd_external void fstd_arena_grow(FSTD_Arena *arena, FSTD_USize new_len);

#define fstd_arena_create(arena, type) fstd_arena_push(arena, type, 1)
#define fstd_arena_create_zero(arena, type) fstd_arena_push_zero(arena, type, 1)

/// Pushes the arena at least `len` bytes forwards.
///
/// The contents of the returned range is undefined.
#define fstd_arena_push(arena, type, n) (type *)fstd__arena_push((arena), sizeof(type) * (n), alignof(type))
fstd_util void *fstd__arena_push(FSTD_Arena *arena, FSTD_USize len, FSTD_USize align) {
    align = FSTD__MAX(align, FSTD_ARENA_MIN_ALIGN);
    if (len == 0)
        return fstd_nullptr;

    FSTD_USize pos = atomic_load_explicit(&arena->pos, memory_order_relaxed);
    FSTD_USize start_pos;
    FSTD_USize offset = (FSTD_USize)arena->ptr;
    for (;;) {
        start_pos = fstd_align_forwards_usize(offset + pos, align) - offset;
        FSTD_USize end_pos = start_pos + len;
        if (atomic_load_explicit(&arena->commit_len, memory_order_relaxed) < end_pos) {
            fstd_arena_grow(arena, end_pos);
        }
        if (atomic_compare_exchange_weak_explicit(&arena->pos, &pos, end_pos, memory_order_relaxed,
                                                  memory_order_relaxed))
            break;
    }
    return ((char *)arena->ptr) + start_pos;
}

/// Pushes the arena at least `len` bytes forwards.
///
/// The returned range of [ptr, ptr+len) is zeroed.
#define fstd_arena_push_zero(arena, type, n) (type *)fstd__arena_push_zero((arena), sizeof(type) * (n), alignof(type))
fstd_util void *fstd__arena_push_zero(FSTD_Arena *arena, FSTD_USize len, FSTD_USize align) {
    void *ptr = fstd__arena_push(arena, len, align);
#if FSTD_COMPILER_GCC_COMPATIBLE
    __builtin_memset(ptr, 0, len);
#else
    memset(ptr, 0, len);
#endif
    return ptr;
}

/// Pops `n` elements of type `type` from the end of the arena.
///
/// The freed memory region is not cleared.
/// CAUTION: Use only if you are certain that the arena is not being shared
/// and that you own the memory at the end of the arena.
#define fstd_arena_pop(arena, type, n) fstd__arena_pop((arena), sizeof(type) * (n))
fstd_util void fstd__arena_pop(FSTD_Arena *arena, FSTD_USize bytes) {
    FSTD_USize pos = atomic_fetch_sub_explicit(&arena->pos, bytes, memory_order_relaxed);
    fstd_dbg_assert(pos >= bytes);
}

/// Tries to grow the allocation inplace.
#define fstd_arena_resize(arena, ptr, n, new_n)                                                                        \
    (type *)fstd__arena_resize((arena), (ptr), sizeof(*ptr) * (n), sizeof(*ptr) * (new_n))
fstd_util bool fstd__arena_resize(FSTD_Arena *arena, void *ptr, FSTD_USize len, FSTD_USize new_len) {
    if (len == 0)
        return false;
    if (new_len <= len)
        return true;

    FSTD_USize arena_ptr_int = (FSTD_USize)arena->ptr;
    FSTD_USize ptr_int = (FSTD_USize)ptr;
    fstd_dbg_assert(arena_ptr_int <= ptr_int);

    FSTD_USize start_pos = ptr_int - arena_ptr_int;
    FSTD_USize end_pos = start_pos + len;
    FSTD_USize new_end_pos = start_pos + new_len;
    fstd_dbg_assert(end_pos <= atomic_load_explicit(&arena->pos, memory_order_relaxed));
    return atomic_compare_exchange_strong_explicit(&arena->pos, &end_pos, new_end_pos, memory_order_relaxed,
                                                   memory_order_relaxed);
}

/// Tries to grow the allocation, allocating a new block if it can not be done inplace.
#define fstd_arena_remap(arena, ptr, n, new_n)                                                                         \
    (type *)fstd__arena_remap((arena), (ptr), sizeof(*ptr) * (n), fstd__alignof(*ptr), sizeof(*ptr) * (new_n))
fstd_util void *fstd__arena_remap(FSTD_Arena *arena, void *ptr, FSTD_USize len, FSTD_USize align, FSTD_USize new_len) {
    if (len == 0)
        return fstd__arena_push(arena, new_len, align);
    if (new_len <= len)
        return ptr;

    FSTD_USize arena_ptr_int = (FSTD_USize)arena->ptr;
    FSTD_USize ptr_int = (FSTD_USize)ptr;
    fstd_dbg_assert(arena_ptr_int <= ptr_int);

    FSTD_USize start_pos = ptr_int - arena_ptr_int;
    FSTD_USize end_pos = start_pos + len;
    FSTD_USize new_end_pos = start_pos + new_len;
    fstd_dbg_assert(end_pos <= atomic_load_explicit(&arena->pos, memory_order_relaxed));
    if (atomic_compare_exchange_strong_explicit(&arena->pos, &end_pos, new_end_pos, memory_order_relaxed,
                                                memory_order_relaxed))
        return ptr;

    void *new_ptr = fstd__arena_push(arena, new_len, align);
    __builtin_memcpy(new_ptr, ptr, len);
    return new_ptr;
}

/// Frees the allocated pointer, if it is at the end of the arena.
#define fstd_arena_free(arena, ptr, n) fstd__arena_free((arena), ptr, sizeof(*ptr) * (n))
fstd_util void fstd__arena_free(FSTD_Arena *arena, void *ptr, FSTD_USize len) {
    if (len == 0)
        return;

    FSTD_USize arena_ptr_int = (FSTD_USize)arena->ptr;
    FSTD_USize ptr_int = (FSTD_USize)ptr;
    fstd_dbg_assert(arena_ptr_int <= ptr_int);

    FSTD_USize start_pos = ptr_int - arena_ptr_int;
    FSTD_USize end_pos = start_pos + len;
    fstd_dbg_assert(end_pos <= atomic_load_explicit(&arena->pos, memory_order_relaxed));
    atomic_compare_exchange_strong_explicit(&arena->pos, &end_pos, start_pos, memory_order_relaxed,
                                            memory_order_relaxed);
}

fstd_util void *FSTD_MAYBE_NULL fstd__arena_allocator_alloc(void *FSTD_MAYBE_NULL data, FSTD_USize len,
                                                            FSTD_USize align) {
    FSTD_Arena *arena = (FSTD_Arena *)data;
    return fstd__arena_push(arena, len, align);
}
fstd_util bool fstd__arena_allocator_resize(void *FSTD_MAYBE_NULL data, FSTD_MemorySlice memory, FSTD_USize align,
                                            FSTD_USize new_len) {
    FSTD_UNUSED(align);
    FSTD_Arena *arena = (FSTD_Arena *)data;
    return fstd__arena_resize(arena, memory.ptr, memory.len, new_len);
}
fstd_util void *FSTD_MAYBE_NULL fstd__arena_allocator_remap(void *FSTD_MAYBE_NULL data, FSTD_MemorySlice memory,
                                                            FSTD_USize align, FSTD_USize new_len) {
    FSTD_Arena *arena = (FSTD_Arena *)data;
    return fstd__arena_remap(arena, memory.ptr, memory.len, align, new_len);
}
fstd_util void fstd__arena_allocator_free(void *FSTD_MAYBE_NULL data, FSTD_MemorySlice memory, FSTD_USize align) {
    FSTD_UNUSED(align);
    FSTD_Arena *arena = (FSTD_Arena *)data;
    return fstd__arena_free(arena, memory.ptr, memory.len);
}
fstd_internal FSTD_CONSTEXPR FSTD_AllocatorVtable FSTD__Arena_AllocatorVtable = {
        .alloc = fstd__arena_allocator_alloc,
        .resize = fstd__arena_allocator_resize,
        .remap = fstd__arena_allocator_remap,
        .free = fstd__arena_allocator_free,
};

/// Wraps the arena in an allocator interface.
fstd_util FSTD_Allocator fstd_arena_get_allocator(FSTD_Arena *arena) {
    FSTD_Allocator allocator = {
            .ptr = arena,
            .vtable = &FSTD__Arena_AllocatorVtable,
    };
    return allocator;
}

/// Fetches the current position of the arena.
fstd_util FSTD_USize fstd_arena_get_pos(FSTD_Arena *arena) {
    return atomic_load_explicit(&arena->pos, memory_order_relaxed);
}

/// Resets the position of the arena to a previous position.
fstd_util void fstd_arena_set_pos(FSTD_Arena *arena, FSTD_USize pos) {
    fstd_dbg_assert(atomic_load_explicit(&arena->pos, memory_order_relaxed) >= pos);
    atomic_store_explicit(&arena->pos, pos, memory_order_relaxed);
}

/// Resets the arena position without relinquishing any memory.
fstd_util void fstd_arena_clear(FSTD_Arena *arena) { fstd_arena_set_pos(arena, 0); }

/// Creates a scope for the arena.
fstd_util FSTD_TmpArena fstd_arena_scope(FSTD_Arena *arena) {
    return FSTD_INIT(FSTD_TmpArena){
            .arena = arena,
            .pos = fstd_arena_get_pos(arena),
    };
}

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

fstd_internal FSTD_CONSTEXPR FSTD_Uuid FSTD_ResultCls_Unknown = FSTD_DEFAULT_STRUCT;
fstd_internal FSTD_CONSTEXPR FSTD_Uuid FSTD_ResultCls_Ok = {.qwords = {FSTD_U64_MAX, FSTD_U64_MAX}};
fstd_external const FSTD_ResultVtable FSTD__ResultVTable_PlatformError;

fstd_internal FSTD_CONSTEXPR FSTD_StrConst FSTD__Result_OkDescription = FSTD_STR("ok");
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
fstd_internal FSTD_CONSTEXPR FSTD_ResultVtable FSTD__ResultVtable_Ok = {
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
fstd_internal FSTD_CONSTEXPR FSTD_ResultVtable FSTD__ResultVtable_Error = {
        .cls = FSTD_ResultCls_Unknown,
        .deinit = fstd_nullptr,
        .write = fstd__result_vtable_error_write,
};

/// A type-erased result value.
typedef struct {
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
fstd_internal FSTD_CONSTEXPR FSTD_Result FSTD_Result_Ok = {
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
#define FSTD_VERSION_PB(major_, minor_, patch_, pre_, build_)                                                          \
    {.major = (major_), .minor = (minor_), .patch = (patch_), .pre = FSTD_STR(pre_), .build = FSTD_STR(build_)}

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
    {.secs = (ms) / FSTD_MILLIS_PER_SEC, .nanos = (FSTD_U32)((ms) % FSTD_MILLIS_PER_SEC) * FSTD_NANOS_PER_MILLIS}
#define FSTD_MICROS(us)                                                                                                \
    {.secs = (us) / FSTD_MICROS_PER_SEC, .nanos = (FSTD_U32)((us) % FSTD_MICROS_PER_SEC) * FSTD_NANOS_PER_MICROS}
#define FSTD_NANOS(ns) {.secs = (ns) / FSTD_NANOS_PER_SEC, .nanos = (FSTD_U32)((ns) % FSTD_NANOS_PER_SEC)}

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
FSTD_CHECK_USE fstd_external FSTD_Result fstd_duration_add(FSTD_Duration *out, FSTD_Duration lhs, FSTD_Duration rhs);
fstd_external FSTD_Duration fstd_duration_add_saturating(FSTD_Duration lhs, FSTD_Duration rhs);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_duration_sub(FSTD_Duration *out, FSTD_Duration lhs, FSTD_Duration rhs);
fstd_external FSTD_Duration fstd_duration_sub_saturating(FSTD_Duration lhs, FSTD_Duration rhs);

fstd_external FSTD_Time fstd_time_now(void);
fstd_external FSTD_I32 fstd_time_order(FSTD_Time lhs, FSTD_Time rhs);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_time_elapsed(FSTD_Duration *elapsed, FSTD_Time from);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_time_duration_since(FSTD_Duration *elapsed, FSTD_Time since,
                                                                  FSTD_Time to);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_time_add(FSTD_Time *out, FSTD_Time time, FSTD_Duration duration);
fstd_external FSTD_Time fstd_time_add_saturating(FSTD_Time time, FSTD_Duration duration);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_time_sub(FSTD_Time *out, FSTD_Time time, FSTD_Duration duration);
fstd_external FSTD_Time fstd_time_sub_saturating(FSTD_Time time, FSTD_Duration duration);


fstd_external FSTD_Instant fstd_instant_now(void);
fstd_external FSTD_I32 fstd_instant_order(FSTD_Instant lhs, FSTD_Instant rhs);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_instant_elapsed(FSTD_Duration *elapsed, FSTD_Instant from);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_instant_duration_since(FSTD_Duration *elapsed, FSTD_Instant since,
                                                                     FSTD_Instant to);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_instant_add(FSTD_Instant *out, FSTD_Instant time, FSTD_Duration duration);
fstd_external FSTD_Instant fstd_instant_add_saturating(FSTD_Instant time, FSTD_Duration duration);
FSTD_CHECK_USE fstd_external FSTD_Result fstd_instant_sub(FSTD_Instant *out, FSTD_Instant time, FSTD_Duration duration);
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
#define FSTD_CTX_VERSION_PRE "dev"
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
    FSTD_CfgId_Core = (FSTD_CfgId)1,
    FSTD_CfgId_Tracing = (FSTD_CfgId)2,
    FSTD_CfgId_Modules = (FSTD_CfgId)3,
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
fstd_func FSTD_Ctx *fstd_ctx_get(void);

/// Registers the context as active.
///
/// May panic if a different context is already active.
/// May be called multiple times.
fstd_func void fstd_ctx_register(FSTD_Ctx *ctx);

/// Unregisters the context.
///
/// Must be paired up with a `fstd_ctx_register` call.
fstd_func void fstd_ctx_unregister(void);

/// Core configuration.
typedef struct {
    FSTD_Cfg id;
    FSTD_USize global_arena_reserve_len;
    FSTD_USize global_arena_commit_len;
    FSTD_USize scratch_arena_reserve_len;
    FSTD_USize scratch_arena_commit_len;
} FSTD_CoreCfg;

typedef struct {
    void (*deinit)(void);
    FSTD_Arena *(*get_global_arena)(void);
    FSTD_Arena *(*get_scratch_arena)(FSTD_Arena *FSTD_MAYBE_NULL conflict);
    bool (*has_error_result)(void);
    FSTD_Result (*replace_result)(FSTD_Result new_result);
} FSTD_CoreVtable;

typedef FSTD_SliceConst(FSTD_Cfg *const) FSTD_Cfgs;

/// Initializes a new context with the given options.
///
/// The initialized context is written to `ctx`.
/// Only one context may be initialized at any given moment.
FSTD_CHECK_USE fstd_external FSTD_Result fstd_ctx_init(FSTD_Ctx **ctx, FSTD_Cfgs cfgs);

/// Deinitializes the global context.
///
/// May block until all resources owned by the context are shut down.
fstd_func void fstd_ctx_deinit(void);

/// Returns the version of the initialized context.
///
/// May differ from the one specified during compilation.
fstd_func FSTD_Version fstd_ctx_get_version(void);

/// Returns the global arena shared by all threads.
fstd_func FSTD_Arena *fstd_ctx_get_global_arena(void);

/// Returns the scratch arena for the current thread.
///
/// The scratch arena will be initialized the first time the thread calls this function.
/// The arena is owned by the calling thread and will be invalidated on thread exit
/// or after the context is deinitialized.
fstd_func FSTD_Arena *fstd_ctx_get_scratch_arena(FSTD_Arena *FSTD_MAYBE_NULL conflict);

/// Checks whether the context has an error stored for the current thread.
fstd_func bool fstd_ctx_has_error_result(void);

/// Replaces the thread-local result stored in the context with a new one.
///
/// The old result is returned.
fstd_func FSTD_Result fstd_ctx_replace_result(FSTD_Result new_result);

/// Swaps out the thread-local result with the `ok` result.
fstd_func FSTD_Result fstd_ctx_take_result(void);

/// Clears the thread-local result.
fstd_func void fstd_ctx_clear_result(void);

/// Sets the thread-local result, destroying the old one.
fstd_func void fstd_ctx_set_result(FSTD_Result new_result);

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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_waiter_init(FSTD_TaskWaiter *waiter);

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
    fstd__waiter_await(waiter, &future.data, (FSTD_TaskWaiterPollFn)future.poll, result)
#define fstd_waiter_await_ref(waiter, future, result)                                                                  \
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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_future_enqueue(const void *FSTD_MAYBE_NULL data, FSTD_USize data_size,
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
fstd_func FSTD_CallStack *fstd_call_stack_init(void);

/// Destroys an empty call stack.
///
/// Marks the completion of a task. Before calling this function, the call stack must be empty,
/// i.e., there must be no active spans on the stack, and must not be active. The call stack may
/// not be used afterwards. The active call stack of the thread is destroyed automatically, on
/// thread exit or during destruction of the context.
fstd_func void fstd_call_stack_finish(FSTD_CallStack *stack);

/// Unwinds and destroys the call stack.
///
/// Marks that the task was aborted. Before calling this function, the call stack must not be
/// active. The call stack may not be used afterwards.
fstd_func void fstd_call_stack_abort(FSTD_CallStack *stack);

/// Replaces the call stack of the current thread.
///
/// This call stack will be used as the active call stack of the calling thread. The old call
/// stack is returned, enabling the caller to switch back to it afterwards. This call stack
/// must be in a suspended, but unblocked, state and not be active. The active call stack must
/// also be in a suspended state, but may also be blocked.
fstd_func FSTD_CallStack *fstd_call_stack_replace_current(FSTD_CallStack *stack);

/// Unblocks the blocked call stack.
///
/// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
/// marked as blocked.
fstd_func void fstd_call_stack_unblock(FSTD_CallStack *stack);

/// Marks the current call stack as being suspended.
///
/// While suspended, the call stack can not be utilized for tracing messages. The call stack
/// optionally also be marked as being blocked. In that case, the call stack must be unblocked
/// prior to resumption.
fstd_func void fstd_call_stack_suspend_current(bool mark_blocked);

/// Marks the current call stack as being resumed.
///
/// Once resumed, the context can be used to trace messages. To be successful, the current call
/// stack must be suspended and unblocked.
fstd_func void fstd_call_stack_resume_current(void);

/// Type of a formatter function.
///
/// The formatter function is allowed to format only part of the message, if it would not fit into
/// the buffer. Must return the number of bytes written.
typedef FSTD_USize (*FSTD_TracingFmtFn)(const void *FSTD_MAYBE_NULL data, char *FSTD_MAYBE_NULL buffer,
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

fstd_util FSTD_USize fstd__tracing_fmt_print(const void *data, char *FSTD_MAYBE_NULL buffer, FSTD_USize buffer_len) {
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
    void (*enter_span)(const FSTD_TracingEventInfo *info, FSTD_TracingFmtFn fmt, const void *fmt_data);
    void (*exit_span)(const FSTD_TracingEventInfo *info);
    void (*log_message)(const FSTD_TracingEventInfo *info, FSTD_TracingFmtFn fmt, const void *fmt_data);
} FSTD_TracingVtable;

/// Checks whether the tracing subsystem is enabled.
///
/// This function can be used to check whether to call into the subsystem at all. Calling this
/// function is not necessary, as the remaining functions of the subsystem are guaranteed to return
/// default values, in case the subsystem is disabled.
fstd_func bool fstd_tracing_is_enabled(void);

/// Registers the calling thread with the tracing subsystem.
///
/// The instrumentation is opt-in on a per thread basis, where unregistered threads will
/// behave as if the subsystem was disabled. Once registered, the calling thread gains access to
/// the tracing subsystem and is assigned a new empty call stack. A registered thread must be
/// unregistered from the tracing subsystem before the context is destroyed, by terminating the
/// tread, or by manually unregistering it. A registered thread may not try to register itself.
fstd_func void fstd_tracing_register_thread(void);

/// Unregisters the calling thread from the tracing subsystem.
///
/// Once unregistered, the calling thread looses access to the tracing subsystem until it is
/// registered again. The thread can not be unregistered until the call stack is empty.
fstd_func void fstd_tracing_unregister_thread(void);

/// Enters the span.
///
/// Once entered, the span is used as the context for succeeding events. Each `enter` operation
/// must be accompanied with a `exit` operation in reverse entering order. A span may be entered
/// multiple times. The formatting function may be used to assign a name to the entered span.
fstd_func void fstd_tracing_enter_span(const FSTD_TracingEventInfo *info, FSTD_TracingFmtFn fmt,
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
fstd_func void fstd_tracing_exit_span(const FSTD_TracingEventInfo *info);

/// Logs a message with a custom format function.
fstd_func void fstd_tracing_log_message(const FSTD_TracingEventInfo *info, FSTD_TracingFmtFn fmt,
                                        const void *FSTD_MAYBE_NULL fmt_data);

/// Logs a message with a formatter accepting a `printf` format string.
FSTD_PRINT_F_FMT_ATTR(2, 3)
fstd_func void fstd_tracing_log_message_fmt(const FSTD_TracingEventInfo *info, const char *fmt, ...) {
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
typedef FSTD_I32 FSTD_ModuleAccessGroup;
enum {
    FSTD_ModuleAccessGroup_Public = (FSTD_ModuleAccessGroup)0,
    FSTD_ModuleAccessGroup_Dependency = (FSTD_ModuleAccessGroup)1,
    FSTD_ModuleAccessGroup_Private = (FSTD_ModuleAccessGroup)2,
    FSTD__ModuleAccessGroup_ = FSTD_I32_MAX,
};

/// Data type and access groups of a module parameter.
typedef struct {
    FSTD_ModuleParamTag tag;
    FSTD_ModuleAccessGroup read_group;
    FSTD_ModuleAccessGroup write_group;
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

#define FSTD_MODULE_SYMBOL(name, version) FSTD_MODULE_SYMBOL_NS(name, "", version)
#define FSTD_MODULE_SYMBOL_NS(name_, ns_, version_) {.name = FSTD_STR(name_), .ns = FSTD_STR(ns_), .version = version_}

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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_module_handle_find_by_name(FSTD_ModuleHandle **handle, FSTD_StrConst module);

/// Searches for a module by a symbol it exports.
///
/// Queries the module that exported the specified symbol.
/// The returned handle will have its reference count increased.
FSTD_CHECK_USE fstd_func FSTD_Status fstd_module_handle_find_by_symbol(FSTD_ModuleHandle **handle,
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
    FSTD_ModuleDependency (*query_namespace)(FSTD__ModuleInstance *ctx, FSTD_StrConst ns);
    FSTD_Status (*add_namespace)(FSTD__ModuleInstance *ctx, FSTD_StrConst ns);
    FSTD_Status (*remove_namespace)(FSTD__ModuleInstance *ctx, FSTD_StrConst ns);
    FSTD_ModuleDependency (*query_dependency)(FSTD__ModuleInstance *ctx, FSTD_ModuleHandle *handle);
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
fstd_util FSTD_ModuleDependency fstd_module_instance_query_namespace(FSTD_ModuleInstance *ctx, FSTD_StrConst ns) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->query_namespace(ctx_, ns);
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
fstd_util FSTD_ModuleDependency fstd_module_instance_query_dependency(FSTD_ModuleInstance *ctx,
                                                                      FSTD_ModuleHandle *handle) {
    FSTD__ModuleInstance *ctx_ = (FSTD__ModuleInstance *)ctx;
    return ctx_->vtable->query_dependency(ctx_, handle);
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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_module_root_instance_init(FSTD_ModuleRootInstance **ctx);

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
fstd_util FSTD_ModuleDependency fstd_module_root_instance_query_namespace(FSTD_ModuleRootInstance *ctx,
                                                                          FSTD_StrConst ns) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_query_namespace(ctx_, ns);
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
fstd_util FSTD_ModuleDependency fstd_module_root_instance_query_dependency(FSTD_ModuleRootInstance *ctx,
                                                                           FSTD_ModuleHandle *handle) {
    FSTD_ModuleInstance *ctx_ = (FSTD_ModuleInstance *)ctx;
    return fstd_module_instance_query_dependency(ctx_, handle);
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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_module_loader_init(FSTD_ModuleLoader **loader);

/// Drops the loader.
///
/// Scheduled operations will be completed, but the caller invalidates their reference to the handle.
fstd_func void fstd_module_loader_deinit(FSTD_ModuleLoader *loader);

/// Checks whether the loader contains some module.
fstd_func bool fstd_module_loader_contains_module(FSTD_ModuleLoader *loader, FSTD_StrConst module);

/// Checks whether the loader contains some symbol.
fstd_func bool fstd_module_loader_contains_symbol(FSTD_ModuleLoader *loader, FSTD_ModuleSymbol symbol);

/// Polls the loader for the state of the specified module.
///
/// If the module has not been processed at the time of calling, the waker will be
/// signaled once the function can be polled again.
fstd_func bool fstd_module_loader_poll_module(FSTD_ModuleLoader *loader, FSTD_TaskWaker waker, FSTD_StrConst module,
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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_module_loader_add_module(FSTD_ModuleLoader *loader,
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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_module_loader_add_modules_from_path(FSTD_ModuleLoader *loader, FSTD_Path path,
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
FSTD_CHECK_USE fstd_func FSTD_Status fstd_module_loader_add_modules_from_iter(FSTD_ModuleLoader *loader,
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
FSTD_CHECK_USE fstd_func FSTD_ModuleLoaderCommitResult fstd_module_loader_commit(FSTD_ModuleLoader *loader);

typedef struct {
    FSTD_StrConst name;
    FSTD_ModuleParamTag tag;
    FSTD_ModuleAccessGroup read_group;
    FSTD_ModuleAccessGroup write_group;
    void (*FSTD_MAYBE_NULL read)(FSTD_ModuleParamData data, void *value);
    void (*FSTD_MAYBE_NULL write)(FSTD_ModuleParamData data, const void *value);
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

typedef void (*FSTD_ModuleSymbolExtBind)(const void *symbol);
typedef void (*FSTD_ModuleSymbolExtUnbind)(void);
typedef struct {
    FSTD_ModuleSymbol id;
    FSTD_ModuleSymbolExtBind FSTD_MAYBE_NULL bind;
    FSTD_ModuleSymbolExtUnbind FSTD_MAYBE_NULL unbind;
} FSTD_ModuleSymbolExt;

typedef FSTD_I32 FSTD_ModuleExportSymbolType;
enum {
    FSTD_ModuleExportSymbolType_Static = (FSTD_ModuleExportSymbolType)0,
    FSTD_ModuleExportSymbolType_StateOffset = (FSTD_ModuleExportSymbolType)1,
    FSTD_ModuleExportSymbolType_Dynamic = (FSTD_ModuleExportSymbolType)2,
    FSTD__ModuleExportSymbolType_ = FSTD_I32_MAX,
};

typedef FSTD_I32 FSTD_ModuleExportSymbolLinkage;
enum {
    FSTD_ModuleExportSymbolLinkage_Global = (FSTD_ModuleExportSymbolLinkage)0,
    FSTD__ModuleExportSymbolLinkage_ = FSTD_I32_MAX,
};

typedef FSTD_Fallible(void *) FSTD_ModuleExportDynamicSymbolInitResult;
typedef struct {
    FSTD_ModuleSymbolExt symbol;
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
    FSTD_ModuleExportEventTag_BindCtx = (FSTD_ModuleExportEventTag)0,
    FSTD_ModuleExportEventTag_UnbindCtx = (FSTD_ModuleExportEventTag)1,
    FSTD_ModuleExportEventTag_Init = (FSTD_ModuleExportEventTag)2,
    FSTD_ModuleExportEventTag_Deinit = (FSTD_ModuleExportEventTag)3,
    FSTD_ModuleExportEventTag_Start = (FSTD_ModuleExportEventTag)4,
    FSTD_ModuleExportEventTag_Stop = (FSTD_ModuleExportEventTag)5,
    FSTD_ModuleExportEventTag_DeinitExport = (FSTD_ModuleExportEventTag)6,
    FSTD_ModuleExportEventTag_Dependencies = (FSTD_ModuleExportEventTag)7,
    FSTD__ModuleExportEventTag_ = FSTD_I32_MAX,
};

typedef struct {
    FSTD_ModuleExportEventTag tag;
    void (*FSTD_MAYBE_NULL bind)(FSTD_Ctx *ctx);
} FSTD_ModuleExportEventBindCtx;

typedef struct {
    FSTD_ModuleExportEventTag tag;
    void (*FSTD_MAYBE_NULL unbind)(void);
} FSTD_ModuleExportEventUnbindCtx;

typedef FSTD_Fallible(void *FSTD_MAYBE_NULL) FSTD_ModuleExportEventInitResult;
typedef struct {
    FSTD_ModuleExportEventTag tag;
    bool (*FSTD_MAYBE_NULL poll)(FSTD_ModuleInstance *ctx, FSTD_ModuleLoader *loader, FSTD_TaskWaker waker,
                                 FSTD_ModuleExportEventInitResult *state);
} FSTD_ModuleExportEventInit;

typedef struct {
    FSTD_ModuleExportEventTag tag;
    bool (*FSTD_MAYBE_NULL poll)(FSTD_ModuleInstance *ctx, FSTD_TaskWaker waker, void *FSTD_MAYBE_NULL state);
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
typedef FSTD_SliceConst(FSTD_ModuleSymbolExt) FSTD_ModuleExportSymbolImports;
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

fstd_util FSTD_ModuleExportEventBindCtx fstd_module_export_event_bind_ctx(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventBindCtx ev = {.tag = FSTD_ModuleExportEventTag_BindCtx};
    module->on_event(module, &ev.tag);
    return ev;
}

fstd_util FSTD_ModuleExportEventUnbindCtx fstd_module_export_event_unbind_ctx(const FSTD_ModuleExport *module) {
    FSTD_ModuleExportEventUnbindCtx ev = {.tag = FSTD_ModuleExportEventTag_UnbindCtx};
    module->on_event(module, &ev.tag);
    return ev;
}

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

/// Default event handler for a module export.
///
/// Can be utilized as a fallback in case a custom event handler is provided.
fstd_util void fstd_module_export_default_on_event(const FSTD_ModuleExport *module, FSTD_ModuleExportEventTag *tag) {
    FSTD_UNUSED(module);
    switch (*tag) {
        case FSTD_ModuleExportEventTag_BindCtx: {
            FSTD_ModuleExportEventBindCtx *event = fstd_parent_of(FSTD_ModuleExportEventBindCtx, tag, tag);
            event->bind = fstd_ctx_register;
        } break;
        case FSTD_ModuleExportEventTag_UnbindCtx: {
            FSTD_ModuleExportEventUnbindCtx *event = fstd_parent_of(FSTD_ModuleExportEventUnbindCtx, tag, tag);
            event->unbind = fstd_ctx_unregister;
        } break;
        default:
            break;
    }
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

#ifdef __cplusplus
#define FSTD_SYM(name_, id_, type)                                                                                     \
    fstd_func const type *FSTD_CONCAT(name_, __get)(void);                                                             \
    fstd_func void FSTD_CONCAT(name_, __bind)(const void *ptr);                                                        \
    fstd_func void FSTD_CONCAT(name_, __unbind)(void);                                                                 \
    fstd_internal constexpr auto FSTD_CONCAT(name_, __Cxx) = [] {                                                      \
        FSTD_ModuleSymbol sym_id = id_;                                                                                \
        return ::fstd::modules::Symbol<type>{                                                                          \
                {sym_id.name.ptr, sym_id.name.len},                                                                    \
                {sym_id.ns.ptr, sym_id.ns.len},                                                                        \
                {sym_id.version},                                                                                      \
        };                                                                                                             \
    }();                                                                                                               \
    fstd_internal constexpr FSTD_ModuleSymbol FSTD_CONCAT(name_, _Id) = id_;                                           \
    fstd_internal constexpr FSTD_ModuleSymbolExt name_ = {                                                             \
            .id = id_,                                                                                                 \
            .bind = FSTD_CONCAT(name_, __bind),                                                                        \
            .unbind = FSTD_CONCAT(name_, __unbind),                                                                    \
    };

#define FSTD_SYM_FN(name_, id_, ret, ...)                                                                              \
    fstd_func ret (*FSTD_CONCAT(name_, __get)(void))(__VA_ARGS__);                                                     \
    fstd_func void FSTD_CONCAT(name_, __bind)(const void *ptr);                                                        \
    fstd_func void FSTD_CONCAT(name_, __unbind)(void);                                                                 \
    fstd_internal constexpr auto FSTD_CONCAT(name_, __Cxx) = [] {                                                      \
        FSTD_ModuleSymbol sym_id = id_;                                                                                \
        return ::fstd::modules::Symbol<ret (*)(__VA_ARGS__)>{                                                          \
                {sym_id.name.ptr, sym_id.name.len},                                                                    \
                {sym_id.ns.ptr, sym_id.ns.len},                                                                        \
                {sym_id.version},                                                                                      \
        };                                                                                                             \
    }();                                                                                                               \
    fstd_internal constexpr FSTD_ModuleSymbol FSTD_CONCAT(name_, _Id) = id_;                                           \
    fstd_internal constexpr FSTD_ModuleSymbolExt name_ = {                                                             \
            .id = id_,                                                                                                 \
            .bind = FSTD_CONCAT(name_, __bind),                                                                        \
            .unbind = FSTD_CONCAT(name_, __unbind),                                                                    \
    };

#define FSTD_SYM_IMP(name, type)                                                                                       \
    fstd_func_impl const type *FSTD_CONCAT(name, __get)(void) { return FSTD_CONCAT(name, __Cxx).getPtr(); }            \
    fstd_func_impl void FSTD_CONCAT(name, __bind)(const void *ptr) {                                                   \
        FSTD_CONCAT(name, __Cxx).bind(::fstd::modules::SymbolUtil<type>::refFromPtr(ptr));                             \
    }                                                                                                                  \
    fstd_func_impl void FSTD_CONCAT(name, __unbind)(void) { FSTD_CONCAT(name, __Cxx).unbind(); }

#define FSTD_SYM_FN_IMP(name, ret, ...)                                                                                \
    fstd_func_impl ret (*FSTD_CONCAT(name, __get)(void))(__VA_ARGS__) { return FSTD_CONCAT(name, __Cxx).getPtr(); }    \
    fstd_func_impl void FSTD_CONCAT(name, __bind)(const void *ptr) {                                                   \
        FSTD_CONCAT(name, __Cxx).bind(::fstd::modules::SymbolUtil<ret (*)(__VA_ARGS__)>::refFromPtr(ptr));             \
    }                                                                                                                  \
    fstd_func_impl void FSTD_CONCAT(name, __unbind)(void) { FSTD_CONCAT(name, __Cxx).unbind(); }
#else
#define FSTD_SYM(name, id_, type)                                                                                      \
    fstd_func const type *FSTD_CONCAT(name, __get)(void);                                                              \
    fstd_func void FSTD_CONCAT(name, __bind)(const void *ptr);                                                         \
    fstd_func void FSTD_CONCAT(name, __unbind)(void);                                                                  \
    fstd_glob FSTD__RefCountedHandle FSTD_CONCAT(prefix, name);                                                        \
    fstd_internal const FSTD_ModuleSymbol FSTD_CONCAT(name, _Id) = id_;                                                \
    fstd_internal const FSTD_ModuleSymbolExt name = {                                                                  \
            .id = id_,                                                                                                 \
            .bind = FSTD_CONCAT(name, __bind),                                                                         \
            .unbind = FSTD_CONCAT(name, __unbind),                                                                     \
    };

#define FSTD_SYM_FN(name, id_, ret, ...)                                                                               \
    fstd_func ret (*FSTD_CONCAT(name, __get)(void))(__VA_ARGS__);                                                      \
    fstd_func void FSTD_CONCAT(name, __bind)(const void *ptr);                                                         \
    fstd_func void FSTD_CONCAT(name, __unbind)(void);                                                                  \
    fstd_glob FSTD__RefCountedHandle FSTD_CONCAT(name, __Handle);                                                      \
    fstd_internal const FSTD_ModuleSymbol FSTD_CONCAT(name, _Id) = id_;                                                \
    fstd_internal const FSTD_ModuleSymbolExt name = {                                                                  \
            .id = id_,                                                                                                 \
            .bind = FSTD_CONCAT(name, __bind),                                                                         \
            .unbind = FSTD_CONCAT(name, __unbind),                                                                     \
    };

#define FSTD_SYM_IMP(name, type)                                                                                       \
    fstd_func_impl const type *FSTD_CONCAT(name, __get)(void) {                                                        \
        return (const type *)FSTD_CONCAT(name, __Handle).handle;                                                       \
    }                                                                                                                  \
    fstd_func_impl void FSTD_CONCAT(name, __bind)(const void *ptr) {                                                   \
        fstd__ref_counted_handle_register(&FSTD_CONCAT(name, __Handle), ptr);                                          \
    }                                                                                                                  \
    fstd_func_impl void FSTD_CONCAT(name, __unbind)(void) {                                                            \
        fstd__ref_counted_handle_unregister(&FSTD_CONCAT(name, __Handle));                                             \
    }                                                                                                                  \
    fstd_glob_impl FSTD__RefCountedHandle FSTD_CONCAT(name, __Handle);

#define FSTD_SYM_FN_IMP(name, ret, ...)                                                                                \
    fstd_func_impl ret (*FSTD_CONCAT(name, __get)(void))(__VA_ARGS__) {                                                \
        return (ret (*)(__VA_ARGS__))FSTD_CONCAT(name, __Handle).handle;                                               \
    }                                                                                                                  \
    fstd_func_impl void FSTD_CONCAT(name, __bind)(const void *ptr) {                                                   \
        fstd__ref_counted_handle_register(&FSTD_CONCAT(name, __Handle), ptr);                                          \
    }                                                                                                                  \
    fstd_func_impl void FSTD_CONCAT(name, __unbind)(void) {                                                            \
        fstd__ref_counted_handle_unregister(&FSTD_CONCAT(name, __Handle));                                             \
    }                                                                                                                  \
    fstd_glob_impl FSTD__RefCountedHandle FSTD_CONCAT(name, __Handle);
#endif

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
fstd_func FSTD_ModulesProfile fstd_modules_profile(void);

/// Returns the status of all features known to the subsystem.
fstd_func FSTD_ModulesFeatureStatuses fstd_modules_features(void);

/// Checks for the presence of a namespace in the module subsystem.
///
/// A namespace exists, if at least one loaded module exports one symbol in said namespace.
fstd_func bool fstd_modules_namespace_exists(FSTD_StrConst ns);

/// Marks all instances as unloadable.
///
/// Tries to unload all instances that are not referenced by any other modules. If the instance is
/// still referenced, this will mark the instance as unloadable and enqueue it for unloading.
FSTD_CHECK_USE fstd_func FSTD_Status fstd_modules_prune_instances(void);

/// Queries the info of a module parameter.
///
/// This function can be used to query the datatype, the read access, and the write access of a
/// module parameter. This function fails, if the parameter can not be found.
FSTD_CHECK_USE fstd_func FSTD_Status fstd_modules_query_parameter(FSTD_StrConst module, FSTD_StrConst parameter,
                                                                  FSTD_ModuleParamInfo *info);

/// Reads a module parameter with public read access.
///
/// Reads the value of a module parameter with public read access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow reading with a public access.
FSTD_CHECK_USE fstd_func FSTD_Status fstd_modules_read_parameter_opaque(FSTD_ModuleParamTag tag, FSTD_StrConst module,
                                                                        FSTD_StrConst parameter, void *value);

/// Sets a module parameter with public write access.
///
/// Sets the value of a module parameter with public write access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow writing with a public access.
FSTD_CHECK_USE fstd_func FSTD_Status fstd_modules_write_parameter_opaque(FSTD_ModuleParamTag tag, FSTD_StrConst module,
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

#ifdef __cplusplus
}
#endif

// -----------------------------------------
// c++ -------------------------------------
// -----------------------------------------

#ifdef __cplusplus
#include <algorithm>
#include <array>
#include <atomic>
#include <bit>
#include <compare>
#include <concepts>
#include <expected>
#include <format>
#include <iterator>
#include <memory>
#include <new>
#include <optional>
#include <source_location>
#include <string>
#include <type_traits>
#include <variant>

namespace fstd {

    using i8 = FSTD_I8;
    using i16 = FSTD_I16;
    using i32 = FSTD_I32;
    using i64 = FSTD_I64;
    using isize = FSTD_ISize;

    using u8 = FSTD_U8;
    using u16 = FSTD_U16;
    using u32 = FSTD_U32;
    using u64 = FSTD_U64;
    using usize = FSTD_USize;

    fstd_util auto nextPowOfTwo(auto v) noexcept -> decltype(v) {
        using T = decltype(v);
        if constexpr (std::is_same_v<T, u8>) {
            return fstd_next_power_of_two_u8(v);
        }
        else if constexpr (std::is_same_v<T, u16>) {
            return fstd_next_power_of_two_u16(v);
        }
        else if constexpr (std::is_same_v<T, u32>) {
            return fstd_next_power_of_two_u32(v);
        }
        else if constexpr (std::is_same_v<T, u64>) {
            return fstd_next_power_of_two_u64(v);
        }
        else if constexpr (std::is_same_v<T, usize>) {
            return fstd_next_power_of_two_usize(v);
        }
        else {
            static_assert(false, "invalid unsigned integer");
        }
    }

    fstd_util auto isPowOfTwo(auto v) noexcept -> bool {
        using T = decltype(v);
        if constexpr (std::is_same_v<T, u8>) {
            return fstd_is_power_of_two_u8(v);
        }
        else if constexpr (std::is_same_v<T, u16>) {
            return fstd_is_power_of_two_u16(v);
        }
        else if constexpr (std::is_same_v<T, u32>) {
            return fstd_is_power_of_two_u32(v);
        }
        else if constexpr (std::is_same_v<T, u64>) {
            return fstd_is_power_of_two_u64(v);
        }
        else if constexpr (std::is_same_v<T, usize>) {
            return fstd_is_power_of_two_usize(v);
        }
        else {
            static_assert(false, "invalid unsigned integer");
        }
    }

    template<typename T>
    struct Slice {
        T *ptr;
        usize len;

        using ElementType = T;
        using ValueType = std::remove_cv_t<T>;
        using SizeType = usize;
        using DifferenceType = isize;
        using Pointer = T *;
        using ConstPointer = const T *;
        using Reference = T &;
        using ConstReference = const T &;
        using Iterator = T *;
        using ConstIterator = const T *;
        using ReverseIterator = std::reverse_iterator<Iterator>;
        using ConstReverseIterator = std::reverse_iterator<const T *>;

        constexpr Slice() noexcept = default;
        // constexpr Slice(T *it) noexcept
        //     requires std::is_same_v<std::remove_const_t<T>, char>
        //     : ptr{it}, len{std::char_traits<std::remove_const_t<T>>::length(it)} {}
        template<typename It>
        constexpr Slice(It first, usize count) noexcept : ptr{std::to_address(first)}, len{count} {}
        template<typename It, typename End>
        constexpr Slice(It first, End last) noexcept : ptr{std::to_address(first)}, len{last - first} {}
        template<usize N>
        constexpr Slice(std::type_identity_t<T> (&arr)[N]) noexcept : ptr{arr}, len{N} {}
        template<typename U, usize N>
        constexpr Slice(std::array<U, N> &arr) noexcept : ptr{arr.data()}, len{N} {}
        template<typename U, usize N>
        constexpr Slice(const std::array<U, N> &arr) noexcept : ptr{arr.data()}, len{N} {}
        constexpr Slice(const Slice &) noexcept = default;
        constexpr Slice(Slice &&) noexcept = default;

        constexpr Slice &operator=(const Slice &) noexcept = default;
        constexpr Slice &operator=(Slice &&) noexcept = default;

        constexpr Iterator begin() const noexcept { return ptr; }
        constexpr ConstIterator cbegin() const noexcept { return ptr; }

        constexpr Iterator end() const noexcept { return ptr + len; }
        constexpr ConstIterator cend() const noexcept { return ptr + len; }

        constexpr ReverseIterator rbegin() const noexcept { return ptr + len; }
        constexpr ConstReverseIterator crbegin() const noexcept { return ptr + len; }

        constexpr ReverseIterator rend() const noexcept { return ptr; }
        constexpr ConstReverseIterator crend() const noexcept { return ptr; }

        constexpr Reference front() const noexcept { return ptr[0]; }
        constexpr Reference back() const noexcept { return ptr[len - 1]; }
        constexpr Reference operator[](usize idx) const noexcept { return ptr[idx]; }
        constexpr Pointer data() const noexcept { return ptr; }
        constexpr usize size() const noexcept { return len; }
        constexpr usize sizeBytes() const noexcept { return len * sizeof(*ptr); }
        constexpr bool empty() const noexcept { return len == 0; }

        template<typename U>
        constexpr operator U() const noexcept {
            return {.ptr = this->ptr, .len = this->len};
        }
    };

    struct StrConst : FSTD_StrConst {
        using Type = StrConst;
        using FStd = FSTD_StrConst;
        using ValueType = char;
        using SizeType = usize;
        using DifferenceType = isize;
        using Pointer = char *;
        using ConstPointer = const char *;
        using Reference = char &;
        using ConstReference = const char &;
        using Iterator = const char *;
        using ConstIterator = const char *;
        using ReverseIterator = std::reverse_iterator<Iterator>;
        using ConstReverseIterator = std::reverse_iterator<const char *>;

        constexpr StrConst() noexcept = default;
        constexpr StrConst(const char *it) noexcept : StrConst(it, std::char_traits<char>::length(it)) {};
        template<typename It>
        constexpr StrConst(It first, usize count) noexcept :
            FSTD_StrConst{.ptr = std::to_address(first), .len = count} {}
        template<typename It, typename End>
        constexpr StrConst(It first, End last) noexcept : StrConst(first, last - first) {}
        template<usize N>
        constexpr StrConst(std::type_identity_t<const char> (&arr)[N]) noexcept : StrConst(arr, N) {}
        template<typename T, usize N>
        constexpr StrConst(std::array<T, N> &arr) noexcept : StrConst(arr.data(), N) {}
        template<typename T, usize N>
        constexpr StrConst(const std::array<T, N> &arr) noexcept : StrConst(arr.data(), N) {}
        constexpr StrConst(const Slice<char> &slice) noexcept : StrConst(slice.data(), slice.size()) {}
        constexpr StrConst(const Slice<const char> &slice) noexcept : StrConst(slice.data(), slice.size()) {}
        constexpr StrConst(const FStd &other) noexcept : FStd(other) {};
        constexpr StrConst(const StrConst &) noexcept = default;
        constexpr StrConst(StrConst &&) noexcept = default;

        constexpr StrConst &operator=(const StrConst &) noexcept = default;
        constexpr StrConst &operator=(StrConst &&) noexcept = default;

        friend constexpr auto operator<=>(StrConst lhs, StrConst rhs) noexcept {
            return std::lexicographical_compare_three_way(lhs.begin(), lhs.end(), rhs.begin(), rhs.end());
        }
        friend constexpr bool operator==(StrConst lhs, StrConst rhs) noexcept { return (lhs <=> rhs) == 0; }
        friend constexpr bool operator!=(StrConst lhs, StrConst rhs) noexcept { return (lhs <=> rhs) != 0; }
        friend constexpr bool operator<(StrConst lhs, StrConst rhs) noexcept { return (lhs <=> rhs) < 0; }
        friend constexpr bool operator<=(StrConst lhs, StrConst rhs) noexcept { return (lhs <=> rhs) <= 0; }
        friend constexpr bool operator>(StrConst lhs, StrConst rhs) noexcept { return (lhs <=> rhs) > 0; }
        friend constexpr bool operator>=(StrConst lhs, StrConst rhs) noexcept { return (lhs <=> rhs) >= 0; }

        constexpr Iterator begin() const noexcept { return ptr; }
        constexpr ConstIterator cbegin() const noexcept { return ptr; }

        constexpr Iterator end() const noexcept { return ptr + len; }
        constexpr ConstIterator cend() const noexcept { return ptr + len; }

        constexpr ReverseIterator rbegin() const noexcept { return ReverseIterator{ptr + len}; }
        constexpr ConstReverseIterator crbegin() const noexcept { return ConstReverseIterator{ptr + len}; }

        constexpr ReverseIterator rend() const noexcept { return ReverseIterator{ptr}; }
        constexpr ConstReverseIterator crend() const noexcept { return ConstReverseIterator{ptr}; }

        constexpr ConstReference front() const noexcept { return ptr[0]; }
        constexpr ConstReference back() const noexcept { return ptr[len - 1]; }
        constexpr ConstReference operator[](usize idx) const noexcept { return ptr[idx]; }
        constexpr ConstPointer data() const noexcept { return ptr; }
        constexpr usize size() const noexcept { return len; }
        constexpr usize sizeBytes() const noexcept { return len * sizeof(*ptr); }
        constexpr bool empty() const noexcept { return len == 0; }
    };

    using Uuid = FSTD_Uuid;

    template<auto Value>
    struct ConstexprValue {};

    namespace detail {
        template<auto>
        struct MemberPointerInfo;
        template<typename T, typename U, T U::*Member>
        struct MemberPointerInfo<Member> {
            using Type = T;
            using Base = U;
        };

#pragma pack(push, 1)
        template<auto Member, typename Parent, usize Offset>
            requires std::is_standard_layout_v<Parent>
        union OffsetOfUnion;
        template<typename T, typename Base, T Base::*Member, typename Parent, usize Offset>
        union OffsetOfUnion<Member, Parent, Offset> {
            struct {
                char padding[Offset];
                T member;
            };
            Parent parent;
            Base base;
        };
#pragma pack(pop)

        template<auto Member, typename Base, usize Low, usize High>
        consteval usize offsetOfImpl() {
            constexpr usize Middle = Low + ((High - Low) / 2);
            if constexpr (Low == Middle) {
                return Middle;
            }
            else {
                OffsetOfUnion<Member, Base, Middle> dummy = {};
                if constexpr (&(dummy.parent.*Member) < &dummy.member) {
                    return offsetOfImpl<Member, Base, Low, Middle>();
                }
                else {
                    return offsetOfImpl<Member, Base, Middle, High>();
                }
            }
        }

        template<auto Member>
        consteval usize offsetOf() {
            using Base = typename MemberPointerInfo<Member>::Base;
            return offsetOfImpl<Member, Base, 0, sizeof(Base)>();
        }

        template<typename F, typename Ret, typename... Args>
        concept InvocableWithReturn = std::invocable<F, Args...> && requires(F &&f, Args &&...args) {
            { f(std::forward<Args>(args)...) } -> std::same_as<Ret>;
        };

        template<InvocableWithReturn<void> F>
        struct [[nodiscard]] ScopeGuard final {
            F callback;
            bool active;

            using CallbackType = F;
            ScopeGuard() noexcept = delete;
            explicit ScopeGuard(F &&callback) noexcept(std::is_nothrow_constructible_v<F, F &&>) :
                callback(std::forward<F>(callback)), active(true) {}
            ScopeGuard(const ScopeGuard &other) noexcept = delete;
            ScopeGuard &operator=(const ScopeGuard &other) noexcept = delete;
            ScopeGuard &operator=(ScopeGuard &&other) noexcept = delete;
            ~ScopeGuard() noexcept {
                if (this->active)
                    this->callback();
            }
            void dismiss() noexcept { this->active = false; }
        };

        inline void expectedNullTerminatedArray() {}
        template<std::size_t N>
        struct ConstString {
            char str[N]{};

            static constexpr std::size_t size = N - 1;

            consteval ConstString() {}
            consteval ConstString(const char (&new_str)[N]) {
                if (new_str[N - 1] != '\0')
                    expectedNullTerminatedArray();
                std::copy_n(new_str, size, str);
            }
        };

        template<typename... Args>
        struct FormatString {
            std::format_string<Args...> fmt;
            std::source_location loc = std::source_location::current();

            template<class T>
                requires std::constructible_from<std::format_string<Args...>, T const &>
            consteval FormatString(T const &fmt, std::source_location loc = std::source_location::current()) :
                fmt(fmt), loc(loc){};
        };

        static constexpr int maximum(int a, int b) {
            if (a > b)
                return a;
            return b;
        }
        template<int index, typename A = void, typename... Args>
        struct ArgType {
            using type = typename std::conditional<index == 0, A,
                                                   typename ArgType<maximum(index - 1, 0), Args...>::type>::type;
        };

        template<typename T>
        struct ArgType<0, T> {
            using type = T;
        };

        template<typename... Args>
        struct Tuple {
            struct MembersEnd {};

            template<usize index, typename M = MembersEnd, typename... Membs>
            struct TupleMembers {
                M member;
                typename std::conditional<sizeof...(Membs) == 0, MembersEnd, TupleMembers<index + 1, Membs...>>::type
                        nextMember;
                static constexpr usize getIndex() { return index; };
            };
            TupleMembers<0, Args...> members;

            template<usize index, typename Ret, typename T>
            constexpr Ret &getMemberFromHolder(T &holder) {
                if constexpr (T::getIndex() == index)
                    return holder.member;
                else
                    return getMemberFromHolder<index, Ret>(holder.nextMember);
            };

            template<usize index>
            constexpr ArgType<index, Args...>::type &getMember() {
                return getMemberFromHolder<index, typename ArgType<index, Args...>::type>(members);
            }
        };

        struct ConstUnknownSlice {
            void const *ptr;
            usize len;

            constexpr ConstUnknownSlice() noexcept = default;
            template<typename It>
            constexpr ConstUnknownSlice(It first, usize count) noexcept : ptr{std::to_address(first)}, len{count} {}
            template<typename It, typename End>
            constexpr ConstUnknownSlice(It first, End last) noexcept : ptr{std::to_address(first)}, len{last - first} {}
            template<typename U, usize N>
            constexpr ConstUnknownSlice(std::array<U, N> &arr) noexcept : ptr{arr.data()}, len{N} {}
            template<typename U, usize N>
            constexpr ConstUnknownSlice(const std::array<U, N> &arr) noexcept : ptr{arr.data()}, len{N} {}
            constexpr ConstUnknownSlice(const ConstUnknownSlice &) noexcept = default;
            constexpr ConstUnknownSlice(ConstUnknownSlice &&) noexcept = default;

            constexpr ConstUnknownSlice &operator=(const ConstUnknownSlice &) noexcept = default;
            constexpr ConstUnknownSlice &operator=(ConstUnknownSlice &&) noexcept = default;
        };
    } // namespace detail

    template<detail::InvocableWithReturn<void> Callback>
    inline static detail::ScopeGuard<Callback> makeScopeGuard(Callback &&callback) noexcept(
            std::is_nothrow_constructible_v<detail::ScopeGuard<Callback>, Callback &&>) {
        return detail::ScopeGuard{std::forward<Callback>(callback)};
    }

    template<typename... Args>
    using FormatString = detail::FormatString<std::type_identity_t<Args>...>;

    template<typename T, typename U, T U::*Member>
    inline U *parentOf(T *value, ConstexprValue<Member>) noexcept {
        constexpr static auto offset = detail::offsetOf<Member>();
        char *value_ptr = reinterpret_cast<char *>(value);
        value_ptr -= offset;
        return reinterpret_cast<U *>(value_ptr);
    }

    template<typename T, typename U, T U::*Member>
    inline U const *parentOf(T const *value, ConstexprValue<Member>) noexcept {
        constexpr static auto offset = detail::offsetOf<Member>();
        char const *value_ptr = reinterpret_cast<char *>(value);
        value_ptr -= offset;
        return reinterpret_cast<U const *>(value_ptr);
    }

    // -----------------------------------------
    // memory ----------------------------------
    // -----------------------------------------

    struct Allocator : FSTD_Allocator {
        using Type = Allocator;
        using FStd = FSTD_Allocator;
        constexpr Allocator() noexcept = default;
        constexpr Allocator(const FStd &other) noexcept : FStd(other) {};
        constexpr Allocator(const Allocator &other) noexcept = default;
        constexpr Allocator(Allocator &&other) noexcept = default;
        constexpr Allocator &operator=(const Allocator &other) noexcept = default;
        constexpr Allocator &operator=(Allocator &&other) noexcept = default;

        template<typename T>
        FSTD_ALLOC T *FSTD_MAYBE_NULL alloc(usize n) const noexcept {
            return static_cast<T *>(this->vtable->alloc(this->ptr, sizeof(T) * n, alignof(T)));
        }
        template<typename T>
        bool resize(T *ptr, usize n, usize new_n) const noexcept {
            return this->vtable->resize(this->ptr, {.ptr = (u8 *)ptr, .len = sizeof(T) * n}, alignof(T),
                                        sizeof(T) * new_n);
        }
        template<typename T>
        FSTD_ALLOC T *FSTD_MAYBE_NULL remap(T *ptr, usize n, usize new_n) const noexcept {
            return static_cast<T *>(this->vtable->remap(this->ptr, {.ptr = (u8 *)ptr, .len = sizeof(T) * n}, alignof(T),
                                                        sizeof(T) * new_n));
        }
        template<typename T>
        void free(T *ptr, usize n) const noexcept {
            return this->vtable->free(this->ptr, {.ptr = (u8 *)ptr, .len = sizeof(T) * n}, alignof(T));
        }
        template<typename T>
        FSTD_ALLOC T *FSTD_MAYBE_NULL create() const noexcept {
            return this->alloc<T>(1);
        }
        template<typename T>
        void destroy(T *ptr) const noexcept {
            return this->free(ptr, 1);
        }
    };
    static constexpr Allocator Allocator_Null = {FSTD_Allocator_Null};

    struct Arena : FSTD_Arena {
        using Flags = FSTD_ArenaFlags;
        using Type = Allocator;
        using FStd = FSTD_Arena;
        constexpr Arena() noexcept = default;
        constexpr Arena(const FStd &other) noexcept :
            FStd{.grow_futex = other.grow_futex.load(std::memory_order_relaxed),
                 .flags = other.flags,
                 .page_size = other.page_size,
                 .commit_len = other.commit_len.load(std::memory_order_relaxed),
                 .ptr = other.ptr,
                 .pos = other.pos.load(std::memory_order_relaxed)} {};
        constexpr Arena(const Arena &other) noexcept :
            FStd{.grow_futex = other.grow_futex.load(std::memory_order_relaxed),
                 .flags = other.flags,
                 .page_size = other.page_size,
                 .commit_len = other.commit_len.load(std::memory_order_relaxed),
                 .ptr = other.ptr,
                 .pos = other.pos.load(std::memory_order_relaxed)} {};
        constexpr Arena(Arena &&other) noexcept :
            FStd{.grow_futex = other.grow_futex.load(std::memory_order_relaxed),
                 .flags = other.flags,
                 .page_size = other.page_size,
                 .commit_len = other.commit_len.load(std::memory_order_relaxed),
                 .ptr = other.ptr,
                 .pos = other.pos.load(std::memory_order_relaxed)} {};
        constexpr Arena &operator=(const Arena &other) noexcept {
            if (this != &other) {
                this->grow_futex = other.grow_futex.load(std::memory_order_relaxed);
                this->flags = other.flags;
                this->page_size = other.page_size;
                this->commit_len = other.commit_len.load(std::memory_order_relaxed);
                this->ptr = other.ptr;
                this->pos = other.pos.load(std::memory_order_relaxed);
            }
            return *this;
        }
        constexpr Arena &operator=(Arena &&other) noexcept {
            if (this != &other) {
                this->grow_futex = other.grow_futex.load(std::memory_order_relaxed);
                this->flags = other.flags;
                this->page_size = other.page_size;
                this->commit_len = other.commit_len.load(std::memory_order_relaxed);
                this->ptr = other.ptr;
                this->pos = other.pos.load(std::memory_order_relaxed);
            }
            return *this;
        }

        static std::optional<Arena> init(void *base, Flags flags, usize reserve, usize commit) noexcept {
            Arena arena{};
            if (!fstd_arena_init(&arena, base, flags, reserve, commit))
                return std::nullopt;
            return arena;
        }
        static std::optional<Arena> init(Flags flags, usize reserve, usize commit) noexcept {
            return init(nullptr, flags, reserve, commit);
        }
        static std::optional<Arena> init(usize reserve, usize commit) noexcept {
            return init(nullptr, 0, reserve, commit);
        }
        static std::optional<Arena> init(Flags flags, usize size) noexcept { return init(nullptr, flags, size, size); }
        static std::optional<Arena> init(usize size) noexcept { return init(nullptr, 0, size, size); }
        void deinit() noexcept {
            if (this->ptr)
                fstd_arena_deinit(this);
        }
        void grow(usize new_size) noexcept { fstd_arena_grow(this, new_size); }
        template<typename T>
        T *push(usize n) noexcept {
            return static_cast<T *>(fstd__arena_push(this, sizeof(T) * n, alignof(T)));
        }
        template<typename T>
        T *pushZero(usize n) noexcept {
            return static_cast<T *>(fstd__arena_push_zero(this, sizeof(T) * n, alignof(T)));
        }
        template<typename T>
        void pop(usize n) noexcept {
            return fstd__arena_pop(this, sizeof(T) * n);
        }
        template<typename T>
        bool resize(T FSTD_MAYBE_NULL *ptr, usize n, usize new_n) noexcept {
            return fstd__arena_resize(this, ptr, sizeof(T) * n, sizeof(T) * new_n);
        }
        template<typename T>
        T *FSTD_MAYBE_NULL remap(T FSTD_MAYBE_NULL *ptr, usize n, usize new_n) noexcept {
            return fstd__arena_remap(this, ptr, sizeof(T) * n, alignof(T), sizeof(T) * new_n);
        }
        template<typename T>
        void free(T FSTD_MAYBE_NULL *ptr, usize n) noexcept {
            return fstd__arena_free(this, ptr, sizeof(T) * n);
        }

        Allocator getAllocator() noexcept { return fstd_arena_get_allocator(this); }
        usize getPos() noexcept { return fstd_arena_get_pos(this); }
        void setPos(usize pos) noexcept { return fstd_arena_set_pos(this, pos); }
    };

    // -----------------------------------------
    // errors ----------------------------------
    // -----------------------------------------

    // NOLINTNEXTLINE(performance-enum-size)
    enum class Status : FSTD_Status {
        Ok = FSTD_Status_Ok,
        Failure = FSTD_Status_Failure,
    };

    enum class PlatformError : FSTD_PlatformError {};

    struct Result : FSTD_Result {
        using Type = Result;
        using FStd = FSTD_Result;
        constexpr Result() noexcept : Result(FSTD_Result_Ok) {};
        constexpr Result(const FStd &other) noexcept : FStd(other) {};
        constexpr Result(const Result &other) noexcept = default;
        constexpr Result(Result &&other) noexcept = default;
        constexpr Result &operator=(const Result &other) noexcept = default;
        constexpr Result &operator=(Result &&other) noexcept = default;

        template<typename T>
        constexpr static Result init(T) noexcept;
        static Result initPlatformError(FSTD_PlatformError err) noexcept {
            return fstd_result_init_platform_error(err);
        }
        static Result initPlatformError(PlatformError err) noexcept {
            return initPlatformError(static_cast<FSTD_PlatformError>(err));
        }
        // TODO(gabriel, https://github.com/llvm/llvm-project/issues/82994): Replace with consteval.
        template<detail::ConstString String>
        static constexpr Result initStaticStr() noexcept {
            constexpr static StrConst StringSlice = String.str;
            constexpr static FSTD_ResultVtable VTable = {
                    .cls = {},
                    .deinit = nullptr,
                    .write =
                            [](void *, FSTD_Str dst, usize offset, usize *remaining) {
                                fstd_dbg_assert(offset <= StringSlice.size());
                                Slice<char> destination = {dst.ptr, dst.len};
                                StrConst sub_str = {StringSlice.begin() + offset, StringSlice.size() - offset};
                                usize writable = FSTD__MIN(destination.size(), sub_str.size());
                                std::copy_n(sub_str.begin(), writable, destination.begin());
                                *remaining = sub_str.size() - writable;
                                return writable;
                            },
            };
            return FSTD_Result{
                    .data = nullptr,
                    .vtable = &VTable,
            };
        }
        void deinit() noexcept {
            if (this->vtable)
                fstd_result_deinit(*this);
        }
        bool isOk() const noexcept { return fstd_result_is_ok(*this); }
        bool isErr() const noexcept { return fstd_result_is_err(*this); }
        usize write(Slice<char> dst, usize offset, usize &remaining) const noexcept {
            return fstd_result_write(*this, dst, offset, &remaining);
        }
    };

    template<>
    inline Result Result::init(PlatformError err) noexcept {
        return initPlatformError(err);
    }

    // -----------------------------------------
    // version ---------------------------------
    // -----------------------------------------

    struct Version : FSTD_Version {
        using Type = Version;
        using FStd = FSTD_Version;
        constexpr Version() noexcept = default;
        constexpr Version(const FStd &other) noexcept : FStd(other) {};
        constexpr Version(const Version &other) noexcept = default;
        constexpr Version(Version &&other) noexcept = default;
        constexpr Version &operator=(const Version &other) noexcept = default;
        constexpr Version &operator=(Version &&other) noexcept = default;
        friend constexpr std::partial_ordering operator<=>(Version lhs, Version rhs) {
            auto order = fstd_version_order(&lhs, &rhs);
            if (order < 0)
                return std::partial_ordering::less;
            if (order > 0)
                return std::partial_ordering::greater;
            return std::partial_ordering::equivalent;
        }
        friend constexpr bool operator==(Version lhs, Version rhs) noexcept { return (lhs <=> rhs) == 0; }
        friend constexpr bool operator!=(Version lhs, Version rhs) noexcept { return (lhs <=> rhs) != 0; }
        friend constexpr bool operator<(Version lhs, Version rhs) noexcept { return (lhs <=> rhs) < 0; }
        friend constexpr bool operator<=(Version lhs, Version rhs) noexcept { return (lhs <=> rhs) <= 0; }
        friend constexpr bool operator>(Version lhs, Version rhs) noexcept { return (lhs <=> rhs) > 0; }
        friend constexpr bool operator>=(Version lhs, Version rhs) noexcept { return (lhs <=> rhs) >= 0; }

        bool sattisfies(const Version &required) const noexcept { return fstd_version_sattisfies(this, &required); }
    };

    // -----------------------------------------
    // time ------------------------------------
    // -----------------------------------------

    using TimeInt = FSTD_TimeInt;

    struct Duration : FSTD_Duration {
        using Type = Duration;
        using FStd = FSTD_Duration;
        constexpr Duration() noexcept = default;
        constexpr Duration(const FStd &other) noexcept : FStd(other) {};
        constexpr Duration(const Duration &other) noexcept = default;
        constexpr Duration(Duration &&other) noexcept = default;
        constexpr Duration &operator=(const Duration &other) noexcept = default;
        constexpr Duration &operator=(Duration &&other) noexcept = default;
        Duration operator+(Duration other) const noexcept { return this->addSat(other); }
        Duration &operator+=(Duration other) noexcept {
            *this = this->addSat(other);
            return *this;
        }
        Duration operator-(Duration other) const noexcept { return this->subSat(other); }
        Duration &operator-=(Duration other) noexcept {
            *this = this->subSat(other);
            return *this;
        }
        friend constexpr std::strong_ordering operator<=>(Duration lhs, Duration rhs) {
            if (lhs.secs() < rhs.secs())
                return std::strong_ordering::less;
            if (lhs.secs() > rhs.secs())
                return std::strong_ordering::greater;
            if (lhs.subsecNanos() < rhs.subsecNanos())
                return std::strong_ordering::less;
            if (lhs.subsecNanos() > rhs.subsecNanos())
                return std::strong_ordering::greater;
            return std::strong_ordering::equivalent;
        }
        friend constexpr bool operator==(Duration lhs, Duration rhs) noexcept { return (lhs <=> rhs) == 0; }
        friend constexpr bool operator!=(Duration lhs, Duration rhs) noexcept { return (lhs <=> rhs) != 0; }
        friend constexpr bool operator<(Duration lhs, Duration rhs) noexcept { return (lhs <=> rhs) < 0; }
        friend constexpr bool operator<=(Duration lhs, Duration rhs) noexcept { return (lhs <=> rhs) <= 0; }
        friend constexpr bool operator>(Duration lhs, Duration rhs) noexcept { return (lhs <=> rhs) > 0; }
        friend constexpr bool operator>=(Duration lhs, Duration rhs) noexcept { return (lhs <=> rhs) >= 0; }

        static constexpr auto initSecs(u64 s) -> Duration { return {FSTD_SECONDS(s)}; }
        static constexpr auto initMillis(u64 ms) -> Duration { return {FSTD_MILLIS(ms)}; }
        static constexpr auto initMicros(u64 us) -> Duration { return {FSTD_MICROS(us)}; }
        static constexpr auto initNanos(u64 ns) -> Duration { return {FSTD_NANOS(ns)}; }

        u64 secs() const noexcept { return fstd_duration_secs(*this); }
        u32 subsecMillis() const noexcept { return fstd_duration_subsec_millis(*this); }
        u32 subsecMicros() const noexcept { return fstd_duration_subsec_micros(*this); }
        u32 subsecNanos() const noexcept { return fstd_duration_subsec_nanos(*this); }
        TimeInt millis() const noexcept { return fstd_duration_millis(*this); }
        TimeInt micros() const noexcept { return fstd_duration_micros(*this); }
        TimeInt nanos() const noexcept { return fstd_duration_nanos(*this); }

        std::expected<Duration, Result> add(Duration other) const noexcept {
            Duration result{};
            Result status = fstd_duration_add(&result, *this, other);
            if (status.isErr())
                return std::unexpected(status);
            return result;
        }
        Duration addSat(Duration other) const noexcept { return {fstd_duration_add_saturating(*this, other)}; }

        std::expected<Duration, Result> sub(Duration other) const noexcept {
            Duration result{};
            Result status = fstd_duration_sub(&result, *this, other);
            if (status.isErr())
                return std::unexpected(status);
            return result;
        }
        Duration subSat(Duration other) const noexcept { return {fstd_duration_sub_saturating(*this, other)}; }
    };
    static constexpr Duration DurationZero = {};
    static constexpr Duration DurationMax = {FSTD_DURATION_MAX};

    struct Time : FSTD_Time {
        using Type = Time;
        using FStd = FSTD_Time;
        constexpr Time() noexcept = default;
        constexpr Time(const FStd &other) noexcept : FStd(other) {};
        constexpr Time(const Time &other) noexcept = default;
        constexpr Time(Time &&other) noexcept = default;
        constexpr Time &operator=(const Time &other) noexcept = default;
        constexpr Time &operator=(Time &&other) noexcept = default;
        Time operator+(Duration other) const noexcept { return this->addSat(other); }
        Time &operator+=(Duration other) noexcept {
            *this = this->addSat(other);
            return *this;
        }
        Time operator-(Duration other) const noexcept { return this->subSat(other); }
        Time &operator-=(Duration other) noexcept {
            *this = this->subSat(other);
            return *this;
        }
        friend constexpr std::strong_ordering operator<=>(Time lhs, Time rhs) {
            if (lhs.secs < rhs.secs)
                return std::strong_ordering::less;
            if (lhs.secs > rhs.secs)
                return std::strong_ordering::greater;
            if (lhs.nanos < rhs.nanos)
                return std::strong_ordering::less;
            if (lhs.nanos > rhs.nanos)
                return std::strong_ordering::greater;
            return std::strong_ordering::equivalent;
        }
        friend constexpr bool operator==(Time lhs, Time rhs) noexcept { return (lhs <=> rhs) == 0; }
        friend constexpr bool operator!=(Time lhs, Time rhs) noexcept { return (lhs <=> rhs) != 0; }
        friend constexpr bool operator<(Time lhs, Time rhs) noexcept { return (lhs <=> rhs) < 0; }
        friend constexpr bool operator<=(Time lhs, Time rhs) noexcept { return (lhs <=> rhs) <= 0; }
        friend constexpr bool operator>(Time lhs, Time rhs) noexcept { return (lhs <=> rhs) > 0; }
        friend constexpr bool operator>=(Time lhs, Time rhs) noexcept { return (lhs <=> rhs) >= 0; }

        static Time now() noexcept { return {fstd_time_now()}; }
        static std::expected<Duration, Result> elapsed(Time from) noexcept {
            Duration elapsed{};
            Result result = fstd_time_elapsed(&elapsed, from);
            if (result.isErr())
                return std::unexpected(result);
            return elapsed;
        }
        std::expected<Duration, Result> durationSince(Time since) const noexcept {
            Duration elapsed{};
            Result result = fstd_time_duration_since(&elapsed, since, *this);
            if (result.isErr())
                return std::unexpected(result);
            return elapsed;
        }

        std::expected<Time, Result> add(Duration rhs) const noexcept {
            Time time{};
            Result result = fstd_time_add(&time, *this, rhs);
            if (result.isErr())
                return std::unexpected(result);
            return time;
        }
        Time addSat(Duration rhs) const noexcept { return {fstd_time_add_saturating(*this, rhs)}; }

        std::expected<Time, Result> sub(Duration rhs) const noexcept {
            Time time{};
            Result result = fstd_time_sub(&time, *this, rhs);
            if (result.isErr())
                return std::unexpected(result);
            return time;
        }
        Time subSat(Duration rhs) const noexcept { return {fstd_time_sub_saturating(*this, rhs)}; }
    };
    static constexpr Time TimeEpoch = {};
    static constexpr Time TimeZero = {};
    static constexpr Time TimeMax = {FSTD_TIME_MAX};

    struct Instant : FSTD_Instant {
        using Type = Instant;
        using FStd = FSTD_Instant;
        constexpr Instant() noexcept = default;
        constexpr Instant(const FStd &other) noexcept : FStd(other) {};
        constexpr Instant(const Instant &other) noexcept = default;
        constexpr Instant(Instant &&other) noexcept = default;
        constexpr Instant &operator=(const Instant &other) noexcept = default;
        constexpr Instant &operator=(Instant &&other) noexcept = default;
        Instant operator+(Duration other) const noexcept { return this->addSat(other); }
        Instant &operator+=(Duration other) noexcept {
            *this = this->addSat(other);
            return *this;
        }
        Instant operator-(Duration other) const noexcept { return this->sub_sat(other); }
        Instant &operator-=(Duration other) noexcept {
            *this = this->sub_sat(other);
            return *this;
        }
        friend constexpr std::strong_ordering operator<=>(Instant lhs, Instant rhs) {
            if (lhs.secs < rhs.secs)
                return std::strong_ordering::less;
            if (lhs.secs > rhs.secs)
                return std::strong_ordering::greater;
            if (lhs.nanos < rhs.nanos)
                return std::strong_ordering::less;
            if (lhs.nanos > rhs.nanos)
                return std::strong_ordering::greater;
            return std::strong_ordering::equivalent;
        }
        friend constexpr bool operator==(Instant lhs, Instant rhs) noexcept { return (lhs <=> rhs) == 0; }
        friend constexpr bool operator!=(Instant lhs, Instant rhs) noexcept { return (lhs <=> rhs) != 0; }
        friend constexpr bool operator<(Instant lhs, Instant rhs) noexcept { return (lhs <=> rhs) < 0; }
        friend constexpr bool operator<=(Instant lhs, Instant rhs) noexcept { return (lhs <=> rhs) <= 0; }
        friend constexpr bool operator>(Instant lhs, Instant rhs) noexcept { return (lhs <=> rhs) > 0; }
        friend constexpr bool operator>=(Instant lhs, Instant rhs) noexcept { return (lhs <=> rhs) >= 0; }

        static Instant now() noexcept { return {fstd_instant_now()}; }
        static std::expected<Duration, Result> elapsed(Instant from) noexcept {
            Duration elapsed{};
            Result result = fstd_instant_elapsed(&elapsed, from);
            if (result.isErr())
                return std::unexpected(result);
            return elapsed;
        }
        std::expected<Duration, Result> durationSince(Instant since) const noexcept {
            Duration elapsed{};
            Result result = fstd_instant_duration_since(&elapsed, since, *this);
            if (result.isErr())
                return std::unexpected(result);
            return elapsed;
        }

        std::expected<Instant, Result> add(Duration rhs) const noexcept {
            Instant time{};
            Result result = fstd_instant_add(&time, *this, rhs);
            if (result.isErr())
                return std::unexpected(result);
            return time;
        }
        Instant addSat(Duration rhs) const noexcept { return {fstd_instant_add_saturating(*this, rhs)}; }

        std::expected<Instant, Result> sub(Duration rhs) const noexcept {
            Instant time{};
            Result result = fstd_instant_sub(&time, *this, rhs);
            if (result.isErr())
                return std::unexpected(result);
            return time;
        }
        Instant sub_sat(Duration rhs) const noexcept { return {fstd_instant_sub_saturating(*this, rhs)}; }
    };
    static constexpr Instant InstantZero = {};
    static constexpr Instant InstantMax = {FSTD_INSTANT_MAX};

    // -----------------------------------------
    // paths -----------------------------------
    // -----------------------------------------

    struct Path;

    struct PathBuf : FSTD_PathBuf {
        using Type = PathBuf;
        using FStd = FSTD_PathBuf;
        constexpr PathBuf() noexcept = default;
        constexpr PathBuf(const FStd &other) noexcept : FStd(other) {};
        constexpr PathBuf(const PathBuf &other) noexcept = default;
        constexpr PathBuf(PathBuf &&other) noexcept = default;
        constexpr PathBuf &operator=(const PathBuf &other) noexcept = default;
        constexpr PathBuf &operator=(PathBuf &&other) noexcept = default;
        constexpr operator Path() const noexcept;

        static std::expected<PathBuf, Result> init(const Allocator &alloc, usize capacity) noexcept {
            PathBuf buf{};
            Result status = fstd_path_buf_init_capacity(&buf, alloc, capacity);
            if (status.isErr())
                return std::unexpected(status);
            return buf;
        }
        void deinit(const Allocator &alloc) noexcept {
            if (this->ptr)
                fstd_path_buf_deinit(this, alloc);
        }

        Result push(Path path) noexcept;
        Result push(StrConst path) noexcept;
        Result push(const Allocator &alloc, Path path) noexcept;
        Result push(const Allocator &alloc, StrConst path) noexcept;

        bool pop() noexcept { return fstd_path_buf_pop(this); }

        constexpr Path asPath() const noexcept;
    };

    struct OwnedPath : FSTD_OwnedPath {
        using Type = OwnedPath;
        using FStd = FSTD_OwnedPath;
        constexpr OwnedPath() noexcept = default;
        constexpr OwnedPath(const FStd &other) noexcept : FStd(other) {};
        constexpr OwnedPath(const OwnedPath &other) noexcept = default;
        constexpr OwnedPath(OwnedPath &&other) noexcept = default;
        constexpr OwnedPath &operator=(const OwnedPath &other) noexcept = default;
        constexpr OwnedPath &operator=(OwnedPath &&other) noexcept = default;
    };

    struct Path : FSTD_Path {
        using Type = Path;
        using FStd = FSTD_Path;
        constexpr Path() noexcept = default;
        constexpr Path(const FStd &other) noexcept : FStd(other) {};
        constexpr Path(const Path &other) noexcept = default;
        constexpr Path(Path &&other) noexcept = default;
        constexpr Path &operator=(const Path &other) noexcept = default;
        constexpr Path &operator=(Path &&other) noexcept = default;

        static std::expected<Path, Result> init(StrConst path) noexcept {
            Path p{};
            Result status = fstd_path_init(&p, path);
            if (status.isErr())
                return std::unexpected(status);
            return p;
        }

        bool isAbsolute() const noexcept { return fstd_path_is_absolute(*this); }
        bool isRelative() const noexcept { return fstd_path_is_relative(*this); }
        bool hasRoot() const noexcept { return fstd_path_has_root(*this); }

        std::optional<Path> parent() const noexcept {
            Path parent{};
            bool has_parent = fstd_path_parent(*this, &parent);
            if (!has_parent)
                return std::nullopt;
            return parent;
        }
        std::optional<Path> fileName() const noexcept {
            Path file_name{};
            bool has_file_name = fstd_path_file_name(*this, &file_name);
            if (!has_file_name)
                return std::nullopt;
            return file_name;
        }
    };

    using OsPathChar = FSTD_OsPathChar;

    struct OwnedOsPath : Slice<OsPathChar> {
        using Type = OwnedOsPath;
        using FStd = FSTD_OwnedOsPath;
        constexpr OwnedOsPath() noexcept = default;
        constexpr OwnedOsPath(const FStd &other) noexcept : Slice{other.ptr, other.len} {};
        constexpr OwnedOsPath(const Slice<OsPathChar> &other) noexcept : Slice{other} {};
        constexpr OwnedOsPath(const OwnedOsPath &other) noexcept = default;
        constexpr OwnedOsPath(OwnedOsPath &&other) noexcept = default;
        constexpr OwnedOsPath &operator=(const OwnedOsPath &other) noexcept = default;
        constexpr OwnedOsPath &operator=(OwnedOsPath &&other) noexcept = default;
    };

    struct OsPath : Slice<const FSTD_OsPathChar> {
        using Type = OsPath;
        using FStd = FSTD_OsPath;
        constexpr OsPath() noexcept = default;
        constexpr OsPath(const FStd &other) noexcept : Slice{other.ptr, other.len} {};
        constexpr OsPath(const Slice<const FSTD_OsPathChar> &other) noexcept : Slice{other} {};
        constexpr OsPath(const OsPath &other) noexcept = default;
        constexpr OsPath(OsPath &&other) noexcept = default;
        constexpr OsPath &operator=(const OsPath &other) noexcept = default;
        constexpr OsPath &operator=(OsPath &&other) noexcept = default;
    };

    // NOLINTNEXTLINE(performance-enum-size)
    enum class Win32PathPrefixTag : FSTD_Win32PathPrefixTag {
        Verbatim = FSTD_Win32PathPrefixTag_Verbatim,
        VerbatimUnc = FSTD_Win32PathPrefixTag_VerbatimUnc,
        VerbatimDisk = FSTD_Win32PathPrefixTag_VerbatimDisk,
        DeviceNs = FSTD_Win32PathPrefixTag_DeviceNs,
        Unc = FSTD_Win32PathPrefixTag_Unc,
        Disk = FSTD_Win32PathPrefixTag_Disk,
    };

    struct Win32PathPrefix : FSTD_Win32PathPrefix {
        using Type = Win32PathPrefix;
        using FStd = FSTD_Win32PathPrefix;
        constexpr Win32PathPrefix() noexcept = default;
        constexpr Win32PathPrefix(const FStd &other) noexcept : FStd(other) {};
        constexpr Win32PathPrefix(const Win32PathPrefix &other) noexcept = default;
        constexpr Win32PathPrefix(Win32PathPrefix &&other) noexcept = default;
        constexpr Win32PathPrefix &operator=(const Win32PathPrefix &other) noexcept = default;
        constexpr Win32PathPrefix &operator=(Win32PathPrefix &&other) noexcept = default;
    };

    // NOLINTNEXTLINE(performance-enum-size)
    enum class PathComponentTag : FSTD_PathComponentTag {
        Win32Prefix = FSTD_PathComponentTag_Win32Prefix,
        RootDir = FSTD_PathComponentTag_RootDir,
        CurDir = FSTD_PathComponentTag_CurDir,
        ParentDir = FSTD_PathComponentTag_ParentDir,
        Normal = FSTD_PathComponentTag_Normal,
    };

    struct PathComponent : FSTD_PathComponent {
        using Type = PathComponent;
        using FStd = FSTD_PathComponent;
        constexpr PathComponent() noexcept = default;
        constexpr PathComponent(const FStd &other) noexcept : FStd(other) {};
        constexpr PathComponent(const PathComponent &other) noexcept = default;
        constexpr PathComponent(PathComponent &&other) noexcept = default;
        constexpr PathComponent &operator=(const PathComponent &other) noexcept = default;
        constexpr PathComponent &operator=(PathComponent &&other) noexcept = default;
    };

    struct PathIter : FSTD_PathIter {
        using Type = PathIter;
        using FStd = FSTD_PathIter;
        constexpr PathIter() noexcept = default;
        constexpr PathIter(const FStd &other) noexcept : FStd(other) {};
        constexpr PathIter(const PathIter &other) noexcept = default;
        constexpr PathIter(PathIter &&other) noexcept = default;
        constexpr PathIter &operator=(const PathIter &other) noexcept = default;
        constexpr PathIter &operator=(PathIter &&other) noexcept = default;
    };

    inline constexpr PathBuf::operator Path() const noexcept { return this->asPath(); }
    inline constexpr Path PathBuf::asPath() const noexcept { return {{this->ptr, this->len}}; }

    inline Result PathBuf::push(Path path) noexcept { return fstd_path_buf_push(this, path); }
    inline Result PathBuf::push(StrConst path) noexcept { return fstd_path_buf_push_str(this, path); }
    inline Result PathBuf::push(const Allocator &alloc, Path path) noexcept {
        return fstd_path_buf_push_alloc(this, alloc, path);
    }
    inline Result PathBuf::push(const Allocator &alloc, StrConst path) noexcept {
        return fstd_path_buf_push_str_alloc(this, alloc, path);
    }

    // -----------------------------------------
    // context api -----------------------------
    // -----------------------------------------

    namespace ctx {
        constexpr static Version CurrentVersion = {FSTD_CTX_VERSION};

        // NOLINTNEXTLINE(performance-enum-size)
        enum class CfgId : FSTD_CfgId {
            Core = FSTD_CfgId_Core,
            Tracing = FSTD_CfgId_Tracing,
            Modules = FSTD_CfgId_Modules,
        };

        using Cfg = FSTD_Cfg;

        struct Handle {
            using Type = Handle;
            using FStd = FSTD_Ctx *;
            constexpr Handle() noexcept = default;
            constexpr Handle(const FStd &other) noexcept : handle(other) {};
            constexpr Handle(const Handle &other) noexcept = default;
            constexpr Handle(Handle &&other) noexcept = default;
            constexpr Handle &operator=(const Handle &other) noexcept = default;
            constexpr Handle &operator=(Handle &&other) noexcept = default;
            constexpr operator FSTD_Ctx *() const noexcept { return this->handle; };

            FSTD_Ctx *handle;

            static Handle get() noexcept { return fstd_ctx_get(); }
            static void bind(Handle ctx) noexcept { return fstd_ctx_register(ctx); }
            static void unbind() noexcept { return fstd_ctx_unregister(); }

            static std::expected<Handle, Result> init(Slice<const Cfg *> cfgs) noexcept {
                Handle ctx{};
                Result status = fstd_ctx_init(&ctx.handle, cfgs);
                if (status.isErr())
                    return std::unexpected(status);
                return ctx;
            }
            static void deinit() noexcept { return fstd_ctx_deinit(); }
        };

        inline static Version getVersion() noexcept { return fstd_ctx_get_version(); }
        inline static Arena &getGlobalArena() noexcept { return *static_cast<Arena *>(fstd_ctx_get_global_arena()); }
        inline static Arena &getScratchArena(Arena *conflict) noexcept {
            return *static_cast<Arena *>(fstd_ctx_get_scratch_arena(conflict));
        }
        inline static bool hasErrorResult() noexcept { return fstd_ctx_has_error_result(); }
        inline static Result hasErrorResult(Result new_result) noexcept { return fstd_ctx_replace_result(new_result); }
        inline static Result takeResult() noexcept { return fstd_ctx_take_result(); }
        inline static void clearResult() noexcept { return fstd_ctx_clear_result(); }
        inline static void setResult(Result new_result) noexcept { return fstd_ctx_set_result(new_result); }
    } // namespace ctx

    // -----------------------------------------
    // async subsystem -------------------------
    // -----------------------------------------

    namespace tasks {
        struct Waker : FSTD_TaskWaker {
            using Type = Waker;
            using FStd = FSTD_TaskWaker;
            constexpr Waker() noexcept = default;
            constexpr Waker(const FStd &other) noexcept : FStd(other) {};
            constexpr Waker(const Waker &other) noexcept = default;
            constexpr Waker(Waker &&other) noexcept = default;
            constexpr Waker &operator=(const Waker &other) noexcept = default;
            constexpr Waker &operator=(Waker &&other) noexcept = default;

            Waker ref() const noexcept { return fstd_task_waker_ref(*this); }
            void unref() const noexcept { return fstd_task_waker_unref(*this); }
            void wakeUnref() const noexcept { return fstd_task_waker_wake_unref(*this); }
            void wake() const noexcept { return fstd_task_waker_wake(*this); }
        };

        template<typename T>
        struct PollUtil {
            using ValueType = T;
        };
        template<>
        struct PollUtil<void> {
            using ValueType = std::monostate;
        };

        struct PollPendingType {};
        static constexpr PollPendingType PollPending{};

        template<typename T>
        using PollResult = std::variant<typename PollUtil<T>::ValueType, PollPendingType>;

        template<typename T>
        concept Awaitable = requires(T a, const Waker &waker) {
            typename T::ResultType;
            { a.poll(waker) } -> std::same_as<PollResult<typename T::ResultType>>;
        };

        struct Waiter : FSTD_TaskWaiter {
            using Type = Waiter;
            using FStd = FSTD_TaskWaiter;
            constexpr Waiter() noexcept = default;
            constexpr Waiter(const FStd &other) noexcept : FStd(other) {};
            constexpr Waiter(const Waiter &other) noexcept = default;
            constexpr Waiter(Waiter &&other) noexcept = default;
            constexpr Waiter &operator=(const Waiter &other) noexcept = default;
            constexpr Waiter &operator=(Waiter &&other) noexcept = default;

            static std::expected<Waiter, Status> init() noexcept {
                Waiter waiter{};
                auto status = static_cast<Status>(fstd_waiter_init(&waiter));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return waiter;
            }
            void deinit() const noexcept { return fstd_waiter_deinit(*this); }
            Waker waker() const noexcept { return fstd_waiter_waker(*this); }
            void block() const noexcept { return fstd_waiter_block(*this); }
            auto await(Awaitable auto &fut) const noexcept -> typename std::remove_cvref_t<decltype(fut)>::ResultType {
                Waker waker = this->waker();
                for (;;) {
                    auto status = fut.poll(waker);
                    if (status.index() != 0) {
                        this->block();
                    }
                    else {
                        return std::get<0>(status);
                    }
                }
            }
        };

        template<typename T>
        struct OpaqueFuture {
            using ResultType = T;

            void *data;
            bool (*poll_fn)(void **FSTD_MAYBE_NULL data, FSTD_TaskWaker waker, T *result);
            void (*FSTD_MAYBE_NULL deinit_fn)(void **FSTD_MAYBE_NULL data);

            void deinit() noexcept {
                if (this->deinit_fn)
                    this->deinit_fn(&this->data);
            }
            PollResult<T> poll(Waker waker) noexcept {
                if constexpr (std::is_same_v<T, void>) {
                    bool completed = this->poll_fn(&this->data, waker, nullptr);
                    if (!completed)
                        return PollResult<T>{std::in_place_index<1>};
                    return {};
                }
                else {
                    T result;
                    bool completed = this->poll_fn(&this->data, waker, &result);
                    if (!completed)
                        return PollResult<T>{std::in_place_index<1>};
                    return PollResult<T>{std::in_place_index<0>, std::move(result)};
                }
            }
        };

        template<Awaitable T>
        inline static std::expected<OpaqueFuture<T>, Status> enqueueFuture(T fut) noexcept {
            constexpr static usize fut_size = sizeof(T);
            constexpr static usize fut_align = alignof(T);
            constexpr static usize result_size = sizeof(typename T::ResultType);
            constexpr static usize result_align = alignof(typename T::ResultType);
            constexpr static FSTD_TaskWaiterPollFn poll_fn =
                    +[](void *FSTD_MAYBE_NULL ptr, FSTD_TaskWaker w, void *res) {
                        if constexpr (!std::is_same_v<typename T::ResultType, void>) {
                            T &fut = *static_cast<T *>(ptr);
                            typename T::ResultType *result = static_cast<T::Result *>(res);
                            Waker waker = w;
                            auto status = fut.poll(waker);
                            if (status.index() != 0)
                                return false;
                            std::construct_at(result, std::get<0>(status));
                            return true;
                        }
                        else {
                            T &fut = *static_cast<T *>(ptr);
                            Waker waker = w;
                            auto status = fut.poll(waker);
                            if (status.index() != 0)
                                return false;
                            return true;
                        }
                    };
            constexpr static FSTD_TaskDeinitFn deinit_fut = +[](void *FSTD_MAYBE_NULL ptr) {
                T *fut = static_cast<T *>(ptr);
                std::destroy_at(fut);
            };
            constexpr static FSTD_TaskDeinitFn deinit_result = +[](void *FSTD_MAYBE_NULL ptr) {
                if constexpr (!std::is_same_v<typename T::ResultType, void>) {
                    typename T::ResultType *result = static_cast<T::ResultType *>(ptr);
                    std::destroy_at(result);
                }
            };

            alignas(T) unsigned char buffer[sizeof(T)];
            auto ptr = std::construct_at(reinterpret_cast<T *>(buffer), std::move(fut));

            OpaqueFuture<T> enqueued;
            Status status = static_cast<Status>(
                    fstd_future_enqueue(ptr, fut_size, fut_align, result_size, result_align, poll_fn, deinit_fut,
                                        deinit_result, reinterpret_cast<FSTD_EnqueuedFuture *>(&enqueued)));
            if (status != Status::Ok) {
                std::destroy_at(ptr);
                return std::unexpected(status);
            }
            return enqueued;
        }
    } // namespace tasks

    // -----------------------------------------
    // tracing subsystem -----------------------
    // -----------------------------------------

    namespace tracing {
        // NOLINTNEXTLINE(performance-enum-size)
        enum class Level : FSTD_TracingLevel {
            Off = FSTD_TracingLevel_Off,
            Error = FSTD_TracingLevel_Error,
            Warn = FSTD_TracingLevel_Warn,
            Info = FSTD_TracingLevel_Info,
            Debug = FSTD_TracingLevel_Debug,
            Trace = FSTD_TracingLevel_Trace,
        };
        constexpr static Level DefaultLevel = static_cast<Level>(FSTD_TRACING_DEFAULT_LEVEL);
        constexpr static Level DefaultMaxLevel = static_cast<Level>(FSTD_TRACING_MAX_LEVEL);

        struct EventInfo : FSTD_TracingEventInfo {
            using Type = EventInfo;
            using FStd = FSTD_TracingEventInfo;
            constexpr EventInfo() noexcept = default;
            constexpr EventInfo(const FStd &other) noexcept : FStd(other) {};
            constexpr EventInfo(const EventInfo &other) noexcept = default;
            constexpr EventInfo(EventInfo &&other) noexcept = default;
            constexpr EventInfo &operator=(const EventInfo &other) noexcept = default;
            constexpr EventInfo &operator=(EventInfo &&other) noexcept = default;

            constexpr static EventInfo at(Level lvl,
                                          std::source_location loc = std::source_location::current()) noexcept {
                return at(nullptr, nullptr, lvl, loc);
            }
            constexpr static EventInfo at(const char *FSTD_MAYBE_NULL scope, Level lvl,
                                          std::source_location loc = std::source_location::current()) noexcept {
                return at(nullptr, scope, lvl, loc);
            }
            constexpr static EventInfo at(const char *FSTD_MAYBE_NULL target, const char *FSTD_MAYBE_NULL scope,
                                          Level lvl,
                                          std::source_location loc = std::source_location::current()) noexcept {
                return FSTD_TracingEventInfo{
                        .name = loc.function_name(),
                        .target = target ? target : "",
                        .scope = scope ? scope : "",
                        .file_name = loc.file_name(),
                        .line_number = static_cast<i32>(loc.line()),
                        .level = static_cast<FSTD_TracingLevel>(lvl),
                };
            }
        };

        using Formatter = FSTD_TracingFmtFn;

        struct Span {
            struct Auto {
                constexpr Auto(Span span, Formatter fmt, const void *data) noexcept : id(span.id) {
                    fstd_tracing_enter_span(&this->id, fmt, data);
                }
                constexpr Auto(const Auto &other) noexcept = delete;
                constexpr Auto(Auto &&other) noexcept = delete;
                ~Auto() noexcept { fstd_tracing_exit_span(&this->id); }
                constexpr Auto &operator=(const Auto &other) = delete;
                constexpr Auto &operator=(Auto &&other) = delete;

                const EventInfo &id;
            };

            const EventInfo &id;

            template<typename Unique = decltype([] {})>
            static Span at(Level lvl, std::source_location loc = std::source_location::current()) noexcept {
                return at<Unique>(nullptr, nullptr, lvl, loc);
            }
            template<typename Unique = decltype([] {})>
            static Span at(const char *FSTD_MAYBE_NULL scope, Level lvl,
                           std::source_location loc = std::source_location::current()) noexcept {
                return at<Unique>(nullptr, scope, lvl, loc);
            }
            template<typename Unique = decltype([] {})>
            static Span at(const char *FSTD_MAYBE_NULL target, const char *FSTD_MAYBE_NULL scope, Level lvl,
                           std::source_location loc = std::source_location::current()) noexcept {
                const static EventInfo id = EventInfo::at(target, scope, lvl, loc);
                return {.id = id};
            }

            void enter(Formatter fmt, const void *data) const noexcept {
                fstd_tracing_enter_span(&this->id, fmt, data);
            }
            void exit() const noexcept { fstd_tracing_exit_span(&this->id); }
        };

        constexpr static const char DefaultScope[] = FSTD_TRACING_DEFAULT_SCOPE;
        constexpr static const char DefaultTarget[] = FSTD_TRACING_DEFAULT_TARGET;
        template<detail::ConstString scope = DefaultScope, Level max_level = DefaultMaxLevel>
        struct Scope {
            template<detail::ConstString target = DefaultTarget, Level max_lvl = max_level>
            struct Target {
                constexpr static auto ScopeName = scope;
                constexpr static auto TargetName = target;

                template<typename Unique = decltype([] {}), typename... Args>
                static void logErr(FormatString<Args...> fmt, Args &&...args) noexcept {
                    logStatic<Unique, Level::Error, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static void logWarn(FormatString<Args...> fmt, Args &&...args) noexcept {
                    logStatic<Unique, Level::Warn, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static void logInfo(FormatString<Args...> fmt, Args &&...args) noexcept {
                    logStatic<Unique, Level::Info, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static void logDebug(FormatString<Args...> fmt, Args &&...args) noexcept {
                    logStatic<Unique, Level::Debug, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static void logTrace(FormatString<Args...> fmt, Args &&...args) noexcept {
                    logStatic<Unique, Level::Trace, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static void log(Level lvl, FormatString<Args...> fmt, Args &&...args) noexcept {
                    const auto formatter = [fmt, &args...](Slice<char> buffer) -> usize {
                        std::format_to_n_result result =
                                std::format_to_n(buffer.ptr, buffer.len, fmt.fmt, std::forward<Args>(args)...);
                        return result.size;
                    };
                    using Fmt = decltype(formatter);
                    const auto trampoline = +[](const void *data, char *buffer, usize buffer_len) -> usize {
                        const Fmt &fmt = *static_cast<const Fmt *>(data);
                        return fmt({buffer, buffer_len});
                    };
                    logWithFormatter<Unique>(lvl, trampoline, &formatter, fmt.loc);
                }
                template<typename Unique = decltype([] {}), Level lvl, typename... Args>
                static void logStatic(FormatString<Args...> fmt, Args &&...args) noexcept {
                    const auto formatter = [fmt, &args...](Slice<char> buffer) -> usize {
                        std::format_to_n_result result =
                                std::format_to_n(buffer.ptr, buffer.len, fmt.fmt, std::forward<Args>(args)...);
                        return result.size;
                    };
                    using Fmt = decltype(formatter);
                    const auto trampoline = +[](const void *data, char *buffer, usize buffer_len) -> usize {
                        const Fmt &fmt = *static_cast<const Fmt *>(data);
                        return fmt({buffer, buffer_len});
                    };
                    logWithFormatterStatic<Unique, lvl>(trampoline, &formatter, fmt.loc);
                }
                template<typename Unique = decltype([] {})>
                static void logWithFormatter(Level lvl, Formatter fmt, const void *data,
                                             std::source_location loc = std::source_location::current()) noexcept {
                    if (static_cast<FSTD_TracingLevel>(max_lvl) < static_cast<FSTD_TracingLevel>(lvl))
                        return;
                    const static EventInfo info = EventInfo::at(target.str, scope.str, lvl, loc);
                    fstd_tracing_log_message(&info, fmt, data);
                }
                template<typename Unique = decltype([] {}), Level lvl>
                static void
                logWithFormatterStatic(Formatter fmt, const void *data,
                                       std::source_location loc = std::source_location::current()) noexcept {
                    if constexpr (static_cast<FSTD_TracingLevel>(max_lvl) < static_cast<FSTD_TracingLevel>(lvl))
                        return;
                    const static EventInfo info = EventInfo::at(target.str, scope.str, lvl, loc);
                    fstd_tracing_log_message(&info, fmt, data);
                }
                template<typename Unique = decltype([] {})>
                static auto spanErr() noexcept {
                    return spanStatic<Unique, Level::Error>();
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static auto spanErr(FormatString<Args...> fmt, Args &&...args) noexcept {
                    return spanStatic<Unique, Level::Error, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {})>
                static auto spanWarn() noexcept {
                    return spanStatic<Unique, Level::Warn>();
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static auto spanWarn(FormatString<Args...> fmt, Args &&...args) noexcept {
                    return spanStatic<Unique, Level::Warn, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {})>
                static auto spanInfo() noexcept {
                    return spanStatic<Unique, Level::Info>();
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static auto spanInfo(FormatString<Args...> fmt, Args &&...args) noexcept {
                    return spanStatic<Unique, Level::Info, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {})>
                static auto spanDebug() noexcept {
                    return spanStatic<Unique, Level::Debug>();
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static auto spanDebug(FormatString<Args...> fmt, Args &&...args) noexcept {
                    return spanStatic<Unique, Level::Debug, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {})>
                static auto spanTrace() noexcept {
                    return spanStatic<Unique, Level::Trace>();
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static auto spanTrace(FormatString<Args...> fmt, Args &&...args) noexcept {
                    return spanStatic<Unique, Level::Trace, Args...>(fmt, std::forward<Args>(args)...);
                }
                template<typename Unique = decltype([] {})>
                static std::optional<Span::Auto> span(Level lvl) noexcept {
                    return span<Unique>(lvl, "");
                }
                template<typename Unique = decltype([] {}), Level lvl>
                static auto spanStatic() noexcept {
                    return spanStatic<Unique, lvl>("");
                }
                template<typename Unique = decltype([] {}), typename... Args>
                static std::optional<Span::Auto> span(Level lvl, FormatString<Args...> fmt, Args &&...args) noexcept {
                    const auto formatter = [fmt, &args...](Slice<char> buffer) -> usize {
                        std::format_to_n_result result =
                                std::format_to_n(buffer.ptr, buffer.len, fmt.fmt, std::forward<Args>(args)...);
                        return result.size;
                    };
                    using Fmt = decltype(formatter);
                    const auto trampoline = +[](const void *data, char *buffer, usize buffer_len) -> usize {
                        const Fmt &fmt = *static_cast<const Fmt *>(data);
                        return fmt({buffer, buffer_len});
                    };
                    return spanWithFormatter<Unique>(lvl, trampoline, &formatter, fmt.loc);
                }
                template<typename Unique = decltype([] {}), Level lvl, typename... Args>
                static auto spanStatic(FormatString<Args...> fmt, Args &&...args) noexcept {
                    const auto formatter = [fmt, &args...](Slice<char> buffer) -> usize {
                        std::format_to_n_result result =
                                std::format_to_n(buffer.ptr, buffer.len, fmt.fmt, std::forward<Args>(args)...);
                        return result.size;
                    };
                    using Fmt = decltype(formatter);
                    const auto trampoline = +[](const void *data, char *buffer, usize buffer_len) -> usize {
                        const Fmt &fmt = *static_cast<const Fmt *>(data);
                        return fmt({buffer, buffer_len});
                    };
                    return spanWithFormatterStatic<Unique, lvl>(trampoline, &formatter, fmt.loc);
                }
                template<typename Unique = decltype([] {})>
                static std::optional<Span::Auto>
                spanWithFormatter(Level lvl, Formatter fmt, const void *data,
                                  std::source_location loc = std::source_location::current()) noexcept {
                    if (static_cast<FSTD_TracingLevel>(max_lvl) < static_cast<FSTD_TracingLevel>(lvl))
                        return std::nullopt;
                    Span span = Span::at<Unique>(target.str, scope.str, lvl, loc);
                    return std::optional<Span::Auto>{std::in_place, span, fmt, data};
                }
                template<typename Unique = decltype([] {}), Level lvl>
                static auto
                spanWithFormatterStatic(Formatter fmt, const void *data,
                                        std::source_location loc = std::source_location::current()) noexcept {
                    if constexpr (static_cast<FSTD_TracingLevel>(max_lvl) < static_cast<FSTD_TracingLevel>(lvl)) {
                        struct Dummy {};
                        return Dummy{};
                    }
                    else {
                        Span span = Span::at<Unique>(target.str, scope.str, lvl, loc);
                        return Span::Auto{span, fmt, data};
                    }
                }
            };
            using DefaultCtx = Target<>;
            constexpr static auto ScopeName = DefaultCtx::ScopeName;
            constexpr static auto TargetName = DefaultCtx::TargetName;

            template<typename Unique = decltype([] {}), typename... Args>
            static void logErr(FormatString<Args...> fmt, Args &&...args) noexcept {
                DefaultCtx::template logErr<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static void logWarn(FormatString<Args...> fmt, Args &&...args) noexcept {
                DefaultCtx::template logWarn<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static void logInfo(FormatString<Args...> fmt, Args &&...args) noexcept {
                DefaultCtx::template logInfo<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static void logDebug(FormatString<Args...> fmt, Args &&...args) noexcept {
                DefaultCtx::template logDebug<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static void logTrace(FormatString<Args...> fmt, Args &&...args) noexcept {
                DefaultCtx::template logTrace<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static void log(Level lvl, FormatString<Args...> fmt, Args &&...args) noexcept {
                DefaultCtx::template log<Unique, Args...>(lvl, fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {})>
            static void logWithFormatter(Level lvl, Formatter fmt, const void *data,
                                         std::source_location loc = std::source_location::current()) noexcept {
                DefaultCtx::template logWithFormatter<Unique>(lvl, fmt, data, loc);
            }
            template<typename Unique = decltype([] {})>
            static auto spanErr() noexcept {
                return DefaultCtx::template spanErr<Unique>();
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static auto spanErr(FormatString<Args...> fmt, Args &&...args) noexcept {
                return DefaultCtx::template spanErr<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {})>
            static auto spanWarn() noexcept {
                return DefaultCtx::template spanWarn<Unique>();
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static auto spanWarn(FormatString<Args...> fmt, Args &&...args) noexcept {
                return DefaultCtx::template spanWarn<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {})>
            static auto spanInfo() noexcept {
                return DefaultCtx::template spanInfo<Unique>();
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static auto spanInfo(FormatString<Args...> fmt, Args &&...args) noexcept {
                return DefaultCtx::template spanInfo<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {})>
            static auto spanDebug() noexcept {
                return DefaultCtx::template spanDebug<Unique>();
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static auto spanDebug(FormatString<Args...> fmt, Args &&...args) noexcept {
                return DefaultCtx::template spanDebug<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {})>
            static auto spanTrace() noexcept {
                return DefaultCtx::template spanTrace<Unique>();
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static auto spanTrace(FormatString<Args...> fmt, Args &&...args) noexcept {
                return DefaultCtx::template spanTrace<Unique, Args...>(fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {})>
            static std::optional<Span::Auto> span(Level lvl) noexcept {
                return DefaultCtx::template span<Unique>(lvl);
            }
            template<typename Unique = decltype([] {}), typename... Args>
            static std::optional<Span::Auto> span(Level lvl, FormatString<Args...> fmt, Args &&...args) noexcept {
                return DefaultCtx::template span<Unique, Args...>(lvl, fmt, std::forward<Args>(args)...);
            }
            template<typename Unique = decltype([] {})>
            static std::optional<Span::Auto>
            spanWithFormatter(Level lvl, Formatter fmt, const void *data,
                              std::source_location loc = std::source_location::current()) noexcept {
                return DefaultCtx::template spanWithFormatter<Unique>(lvl, fmt, data, loc);
            }
        };
        using DefaultScopeCtx = Scope<>;
        using DefaultCtx = DefaultScopeCtx::DefaultCtx;

        template<typename Unique = decltype([] {}), typename... Args>
        static void logErr(FormatString<Args...> fmt, Args &&...args) noexcept {
            DefaultScopeCtx::template logErr<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static void logWarn(FormatString<Args...> fmt, Args &&...args) noexcept {
            DefaultScopeCtx::template logWarn<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static void logInfo(FormatString<Args...> fmt, Args &&...args) noexcept {
            DefaultScopeCtx::template logInfo<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static void logDebug(FormatString<Args...> fmt, Args &&...args) noexcept {
            DefaultScopeCtx::template logDebug<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static void logTrace(FormatString<Args...> fmt, Args &&...args) noexcept {
            DefaultScopeCtx::template logTrace<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static void log(Level lvl, FormatString<Args...> fmt, Args &&...args) noexcept {
            DefaultScopeCtx::template log<Unique, Args...>(lvl, fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {})>
        static void logWithFormatter(Level lvl, Formatter fmt, const void *data,
                                     std::source_location loc = std::source_location::current()) noexcept {
            DefaultScopeCtx::template logWithFormatter<Unique>(lvl, fmt, data, loc);
        }
        template<typename Unique = decltype([] {})>
        static auto spanErr() noexcept {
            return DefaultScopeCtx::template spanErr<Unique>();
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static auto spanErr(FormatString<Args...> fmt, Args &&...args) noexcept {
            return DefaultScopeCtx::template spanErr<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {})>
        static auto spanWarn() noexcept {
            return DefaultScopeCtx::template spanWarn<Unique>();
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static auto spanWarn(FormatString<Args...> fmt, Args &&...args) noexcept {
            return DefaultScopeCtx::template spanWarn<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {})>
        static auto spanInfo() noexcept {
            return DefaultScopeCtx::template spanInfo<Unique>();
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static auto spanInfo(FormatString<Args...> fmt, Args &&...args) noexcept {
            return DefaultScopeCtx::template spanInfo<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {})>
        static auto spanDebug() noexcept {
            return DefaultScopeCtx::template spanDebug<Unique>();
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static auto spanDebug(FormatString<Args...> fmt, Args &&...args) noexcept {
            return DefaultScopeCtx::template spanDebug<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {})>
        static auto spanTrace() noexcept {
            return DefaultScopeCtx::template spanTrace<Unique>();
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static auto spanTrace(FormatString<Args...> fmt, Args &&...args) noexcept {
            return DefaultScopeCtx::template spanTrace<Unique, Args...>(fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {})>
        static std::optional<Span::Auto> span(Level lvl) noexcept {
            return DefaultScopeCtx::template span<Unique>(lvl);
        }
        template<typename Unique = decltype([] {}), typename... Args>
        static std::optional<Span::Auto> span(Level lvl, FormatString<Args...> fmt, Args &&...args) noexcept {
            return DefaultScopeCtx::template span<Unique, Args...>(lvl, fmt, std::forward<Args>(args)...);
        }
        template<typename Unique = decltype([] {})>
        static std::optional<Span::Auto>
        spanWithFormatter(Level lvl, Formatter fmt, const void *data,
                          std::source_location loc = std::source_location::current()) noexcept {
            return DefaultScopeCtx::template spanWithFormatter<Unique>(lvl, fmt, data, loc);
        }

        struct Subscriber : FSTD_Subscriber {
            using Type = Subscriber;
            using FStd = FSTD_Subscriber;
            constexpr Subscriber() noexcept = default;
            constexpr Subscriber(const FStd &other) noexcept : FStd(other) {};
            constexpr Subscriber(const Subscriber &other) noexcept = default;
            constexpr Subscriber(Subscriber &&other) noexcept = default;
            constexpr Subscriber &operator=(const Subscriber &other) noexcept = default;
            constexpr Subscriber &operator=(Subscriber &&other) noexcept = default;

            using EventTag = FSTD_TracingEventTag;
            using Start = FSTD_TracingEventStart;
            using Finish = FSTD_TracingEventFinish;
            using RegisterThread = FSTD_TracingEventRegisterThread;
            using UnregisterThread = FSTD_TracingEventUnregisterThread;
            using CreateCallStack = FSTD_TracingEventCreateCallStack;
            using DestroyCallStack = FSTD_TracingEventDestroyCallStack;
            using UnblockCallStack = FSTD_TracingEventUnblockCallStack;
            using SuspendCallStack = FSTD_TracingEventSuspendCallStack;
            using ResumeCallStack = FSTD_TracingEventResumeCallStack;
            using EnterSpan = FSTD_TracingEventEnterSpan;
            using ExitSpan = FSTD_TracingEventExitSpan;
            using LogMessage = FSTD_TracingEventLogMessage;
            using DeclareEventInfo = FSTD_TracingEventDeclareEventInfo;
            using StartThread = FSTD_TracingEventStartThread;
            using StopThread = FSTD_TracingEventStopThread;
            using LoadImage = FSTD_TracingEventLoadImage;
            using UnloadImage = FSTD_TracingEventUnloadImage;
            using ContextSwitch = FSTD_TracingEventContextSwitch;
            using ThreadWakeup = FSTD_TracingEventThreadWakeup;
            using CallStackSample = FSTD_TracingEventCallStackSample;

            template<typename T>
            static Subscriber init(T &ptr) noexcept {
                constexpr static auto on_event = +[](void *data, const EventTag *tag) {
                    T &sub = *static_cast<T *>(data);
                    switch (*tag) {
                        case FSTD_TracingEventTag_Start: {
                            if constexpr (requires { sub.onEvent(std::declval<Start>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&Start::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_Finish: {
                            if constexpr (requires { sub.onEvent(std::declval<Finish>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&Finish::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_RegisterThread: {
                            if constexpr (requires { sub.onEvent(std::declval<RegisterThread>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&RegisterThread::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_UnregisterThread: {
                            if constexpr (requires { sub.onEvent(std::declval<UnregisterThread>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&UnregisterThread::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_CreateCallStack: {
                            if constexpr (requires { sub.onEvent(std::declval<CreateCallStack>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&CreateCallStack::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_DestroyCallStack: {
                            if constexpr (requires { sub.onEvent(std::declval<DestroyCallStack>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&DestroyCallStack::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_UnblockCallStack: {
                            if constexpr (requires { sub.onEvent(std::declval<UnblockCallStack>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&UnblockCallStack::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_SuspendCallStack: {
                            if constexpr (requires { sub.onEvent(std::declval<SuspendCallStack>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&SuspendCallStack::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_ResumeCallStack: {
                            if constexpr (requires { sub.onEvent(std::declval<ResumeCallStack>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&ResumeCallStack::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_EnterSpan: {
                            if constexpr (requires { sub.onEvent(std::declval<EnterSpan>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&EnterSpan::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_ExitSpan: {
                            if constexpr (requires { sub.onEvent(std::declval<ExitSpan>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&ExitSpan::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_LogMessage: {
                            if constexpr (requires { sub.onEvent(std::declval<LogMessage>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&LogMessage::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_DeclareEventInfo: {
                            if constexpr (requires { sub.onEvent(std::declval<DeclareEventInfo>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&DeclareEventInfo::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_StartThread: {
                            if constexpr (requires { sub.onEvent(std::declval<StartThread>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&StartThread::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_StopThread: {
                            if constexpr (requires { sub.onEvent(std::declval<StopThread>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&StopThread::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_LoadImage: {
                            if constexpr (requires { sub.onEvent(std::declval<LoadImage>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&LoadImage::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_UnloadImage: {
                            if constexpr (requires { sub.onEvent(std::declval<UnloadImage>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&UnloadImage::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_ContextSwitch: {
                            if constexpr (requires { sub.onEvent(std::declval<ContextSwitch>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&ContextSwitch::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_ThreadWakeup: {
                            if constexpr (requires { sub.onEvent(std::declval<ThreadWakeup>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&ThreadWakeup::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        case FSTD_TracingEventTag_CallStackSample: {
                            if constexpr (requires { sub.onEvent(std::declval<CallStackSample>()); }) {
                                auto *event = parentOf(tag, ConstexprValue<&CallStackSample::tag>{});
                                sub.onEvent(*event);
                            }
                        } break;
                        default:
                            break;
                    }
                };

                return FSTD_Subscriber{
                        .data = &ptr,
                        .on_event = nullptr,
                };
            }

            void start(Start ev) { fstd_subscriber_start(*this, ev); }
            void finish(Finish ev) { fstd_subscriber_finish(*this, ev); }
            void registerThread(RegisterThread ev) { fstd_subscriber_register_thread(*this, ev); }
            void unregisterThread(UnregisterThread ev) { fstd_subscriber_unregister_thread(*this, ev); }
            void createCallStack(CreateCallStack ev) { fstd_subscriber_create_call_stack(*this, ev); }
            void destroyCallStack(DestroyCallStack ev) { fstd_subscriber_destroy_call_stack(*this, ev); }
            void unblockCallStack(UnblockCallStack ev) { fstd_subscriber_unblock_call_stack(*this, ev); }
            void suspendCallStack(SuspendCallStack ev) { fstd_subscriber_suspend_call_stack(*this, ev); }
            void resumeCallStack(ResumeCallStack ev) { fstd_subscriber_resume_call_stack(*this, ev); }
            void enterSpan(EnterSpan ev) { fstd_subscriber_enter_span(*this, ev); }
            void exitSpan(ExitSpan ev) { fstd_subscriber_exit_span(*this, ev); }
            void logMessage(LogMessage ev) { fstd_subscriber_log_message(*this, ev); }
            void declareEventInfo(DeclareEventInfo ev) { fstd_subscriber_declare_event_info(*this, ev); }
            void startThread(StartThread ev) { fstd_subscriber_start_thread(*this, ev); }
            void stopThread(StopThread ev) { fstd_subscriber_stop_thread(*this, ev); }
            void loadImage(LoadImage ev) { fstd_subscriber_load_image(*this, ev); }
            void unloadImage(UnloadImage ev) { fstd_subscriber_unload_image(*this, ev); }
            void contextSwitch(ContextSwitch ev) { fstd_subscriber_context_switch(*this, ev); }
            void threadWakeup(ThreadWakeup ev) { fstd_subscriber_thread_wakeup(*this, ev); }
            void callStackSample(CallStackSample ev) { fstd_subscriber_call_stack_sample(*this, ev); }
        };

        struct StdErrLogger : Subscriber {
            static StdErrLogger init() noexcept { return {fstd_stderr_logger_init()}; }
            void deinit() const noexcept { return fstd_stderr_logger_deinit(*this); }
        };

        struct CallStack {
            using Type = CallStack;
            using FStd = FSTD_CallStack *;
            constexpr CallStack() noexcept = default;
            constexpr CallStack(const FStd &other) noexcept : handle(other) {};
            constexpr CallStack(const CallStack &other) noexcept = default;
            constexpr CallStack(CallStack &&other) noexcept = default;
            constexpr CallStack &operator=(const CallStack &other) noexcept = default;
            constexpr CallStack &operator=(CallStack &&other) noexcept = default;
            constexpr operator FSTD_CallStack *() const noexcept { return this->handle; };

            FSTD_CallStack *handle;

            static CallStack init() noexcept { return fstd_call_stack_init(); }
            void finish() const noexcept { return fstd_call_stack_finish(this->handle); }
            void abort() const noexcept { return fstd_call_stack_abort(this->handle); }
            CallStack replaceCurrent() const noexcept { return fstd_call_stack_replace_current(this->handle); }
            void unblock() const noexcept { return fstd_call_stack_unblock(this->handle); }
            static void suspendCurrent(bool mark_blocked) noexcept { fstd_call_stack_suspend_current(mark_blocked); }
            static void resumeCurrent() noexcept { fstd_call_stack_resume_current(); }
        };

        struct Cfg : FSTD_TracingCfg {
            using Type = Cfg;
            using FStd = FSTD_TracingCfg;
            constexpr Cfg() noexcept : Cfg(0, DefaultMaxLevel, {}, true, "") {}
            constexpr Cfg(usize format_buffer_len, Level max_level, Slice<const Subscriber> subscribers,
                          bool register_thread, StrConst app_name) noexcept :
                FStd{
                        .id = {.id = static_cast<FSTD_CfgId>(ctx::CfgId::Tracing)},
                        .format_buffer_len = format_buffer_len,
                        .max_level = static_cast<FSTD_TracingLevel>(max_level),
                        .subscribers = subscribers,
                        .register_thread = register_thread,
                        .app_name = app_name,
                } {};
            constexpr Cfg(const FStd &other) noexcept : FStd(other) {};
            constexpr Cfg(const Cfg &other) noexcept = default;
            constexpr Cfg(Cfg &&other) noexcept = default;
            constexpr Cfg &operator=(const Cfg &other) noexcept = default;
            constexpr Cfg &operator=(Cfg &&other) noexcept = default;
        };

        inline static bool isEnabled() noexcept { return fstd_tracing_is_enabled(); }
        inline static void registerThread() noexcept { return fstd_tracing_register_thread(); }
        inline static void unregisterThread() noexcept { return fstd_tracing_unregister_thread(); }
    } // namespace tracing

    // -----------------------------------------
    // modules subsystem -----------------------
    // -----------------------------------------

    namespace modules {
        // NOLINTNEXTLINE(performance-enum-size)
        enum class ParamTag : FSTD_ModuleParamTag {
            U8 = FSTD_ModuleParamTag_U8,
            U16 = FSTD_ModuleParamTag_U16,
            U32 = FSTD_ModuleParamTag_U32,
            U64 = FSTD_ModuleParamTag_U64,
            I8 = FSTD_ModuleParamTag_I8,
            I16 = FSTD_ModuleParamTag_I16,
            I32 = FSTD_ModuleParamTag_I32,
            I64 = FSTD_ModuleParamTag_I64,
        };

        // NOLINTNEXTLINE(performance-enum-size)
        enum class AccessGroup : FSTD_ModuleAccessGroup {
            Public = FSTD_ModuleAccessGroup_Public,
            Dependency = FSTD_ModuleAccessGroup_Dependency,
            Private = FSTD_ModuleAccessGroup_Private,
        };

        struct ParamInfo : FSTD_ModuleParamInfo {
            using Type = ParamInfo;
            using FStd = FSTD_ModuleParamInfo;
            constexpr ParamInfo() noexcept = default;
            constexpr ParamInfo(const FStd &other) noexcept : FStd(other) {};
            constexpr ParamInfo(const ParamInfo &other) noexcept = default;
            constexpr ParamInfo(ParamInfo &&other) noexcept = default;
            constexpr ParamInfo &operator=(const ParamInfo &other) noexcept = default;
            constexpr ParamInfo &operator=(ParamInfo &&other) noexcept = default;
        };

        template<typename T>
        struct ParamUtil;

        template<>
        struct ParamUtil<u8> {
            using BaseType = u8;
            using Param = FSTD_ModuleParamU8;
            static constexpr ParamTag ParamTag = ParamTag::U8;
            static constexpr auto read = fstd_module_param_u8_read;
            static constexpr auto write = fstd_module_param_u8_write;
            static constexpr auto readData = fstd_module_param_data_read_u8;
            static constexpr auto writeData = fstd_module_param_data_write_u8;
        };
        template<>
        struct ParamUtil<u16> {
            using BaseType = u16;
            using Param = FSTD_ModuleParamU16;
            static constexpr ParamTag ParamTag = ParamTag::U16;
            static constexpr auto read = fstd_module_param_u16_read;
            static constexpr auto write = fstd_module_param_u16_write;
            static constexpr auto readData = fstd_module_param_data_read_u16;
            static constexpr auto writeData = fstd_module_param_data_write_u16;
        };
        template<>
        struct ParamUtil<u32> {
            using BaseType = u32;
            using Param = FSTD_ModuleParamU32;
            static constexpr ParamTag ParamTag = ParamTag::U32;
            static constexpr auto read = fstd_module_param_u32_read;
            static constexpr auto write = fstd_module_param_u32_write;
            static constexpr auto readData = fstd_module_param_data_read_u32;
            static constexpr auto writeData = fstd_module_param_data_write_u32;
        };
        template<>
        struct ParamUtil<u64> {
            using BaseType = u64;
            using Param = FSTD_ModuleParamU64;
            static constexpr ParamTag ParamTag = ParamTag::U64;
            static constexpr auto read = fstd_module_param_u64_read;
            static constexpr auto write = fstd_module_param_u64_write;
            static constexpr auto readData = fstd_module_param_data_read_u64;
            static constexpr auto writeData = fstd_module_param_data_write_u64;
        };
        template<>
        struct ParamUtil<i8> {
            using BaseType = i8;
            using Param = FSTD_ModuleParamI8;
            static constexpr ParamTag ParamTag = ParamTag::I8;
            static constexpr auto read = fstd_module_param_i8_read;
            static constexpr auto write = fstd_module_param_i8_write;
            static constexpr auto readData = fstd_module_param_data_read_i8;
            static constexpr auto writeData = fstd_module_param_data_write_i8;
        };
        template<>
        struct ParamUtil<i16> {
            using BaseType = i16;
            using Param = FSTD_ModuleParamI16;
            static constexpr ParamTag ParamTag = ParamTag::I16;
            static constexpr auto read = fstd_module_param_i16_read;
            static constexpr auto write = fstd_module_param_i16_write;
            static constexpr auto readData = fstd_module_param_data_read_i16;
            static constexpr auto writeData = fstd_module_param_data_write_i16;
        };
        template<>
        struct ParamUtil<i32> {
            using BaseType = i32;
            using Param = FSTD_ModuleParamI32;
            static constexpr ParamTag ParamTag = ParamTag::I32;
            static constexpr auto read = fstd_module_param_i32_read;
            static constexpr auto write = fstd_module_param_i32_write;
            static constexpr auto readData = fstd_module_param_data_read_i32;
            static constexpr auto writeData = fstd_module_param_data_write_i32;
        };
        template<>
        struct ParamUtil<i64> {
            using BaseType = i64;
            using Param = FSTD_ModuleParamI64;
            static constexpr ParamTag ParamTag = ParamTag::I64;
            static constexpr auto read = fstd_module_param_i64_read;
            static constexpr auto write = fstd_module_param_i64_write;
            static constexpr auto readData = fstd_module_param_data_read_i64;
            static constexpr auto writeData = fstd_module_param_data_write_i64;
        };

        template<typename T>
        struct Param {
            using Type = Param;
            using FStd = typename ParamUtil<T>::Param *;
            constexpr Param() noexcept = default;
            constexpr Param(const FStd &other) noexcept : handle(other) {};
            constexpr Param(const Param &other) noexcept = default;
            constexpr Param(Param &&other) noexcept = default;
            constexpr Param &operator=(const Param &other) noexcept = default;
            constexpr Param &operator=(Param &&other) noexcept = default;
            constexpr operator typename ParamUtil<T>::Param *() const noexcept { return this->handle; };

            typename ParamUtil<T>::Param *handle;

            ParamTag tag() const noexcept { return ParamUtil<T>::ParamTag; }
            T read() const noexcept { return Param<T>::read(this->handle); }
            void write(T value) const noexcept { return Param<T>::write(this->handle, value); }
        };

        template<>
        struct Param<void> {
            using Type = Param;
            using FStd = FSTD_ModuleParam *;
            constexpr Param() noexcept = default;
            constexpr Param(const FStd &other) noexcept : handle(other) {};
            constexpr Param(const Param &other) noexcept = default;
            constexpr Param(Param &&other) noexcept = default;
            constexpr Param &operator=(const Param &other) noexcept = default;
            constexpr Param &operator=(Param &&other) noexcept = default;
            constexpr operator FSTD_ModuleParam *() const noexcept { return this->handle; };

            FSTD_ModuleParam *handle;

            ParamTag tag() const noexcept { return static_cast<ParamTag>(fstd_module_param_opaque_tag(this->handle)); }
            template<typename T>
            T read() const noexcept {
                fstd_dbg_assert(this->tag() == ParamUtil<T>::ParamTag);
                Param<T> param = reinterpret_cast<typename ParamUtil<T>::Param *>(this->handle);
                return param.read();
            }
            template<typename T>
            void write(T value) const noexcept {
                fstd_dbg_assert(this->tag() == ParamUtil<T>::ParamTag);
                Param<T> param = reinterpret_cast<typename ParamUtil<T>::Param *>(this->handle);
                param.write(value);
            }
        };

        template<typename T>
        struct ParamData : FSTD_ModuleParamData {
            using Type = ParamData;
            using FStd = FSTD_ModuleParamData;
            constexpr ParamData() noexcept = default;
            constexpr ParamData(const FStd &other) noexcept : FStd(other) {};
            constexpr ParamData(const ParamData &other) noexcept = default;
            constexpr ParamData(ParamData &&other) noexcept = default;
            constexpr ParamData &operator=(const ParamData &other) noexcept = default;
            constexpr ParamData &operator=(ParamData &&other) noexcept = default;

            ParamTag tag() const noexcept { return ParamUtil<T>::ParamTag; }
            T read() const noexcept { return Param<T>::readData(*this); }
            void write(T value) const noexcept { return Param<T>::writeData(*this, value); }
        };

        template<>
        struct ParamData<void> : FSTD_ModuleParamData {
            using Type = ParamData;
            using FStd = FSTD_ModuleParamData;
            constexpr ParamData() noexcept = default;
            constexpr ParamData(const FStd &other) noexcept : FStd(other) {};
            constexpr ParamData(const ParamData &other) noexcept = default;
            constexpr ParamData(ParamData &&other) noexcept = default;
            constexpr ParamData &operator=(const ParamData &other) noexcept = default;
            constexpr ParamData &operator=(ParamData &&other) noexcept = default;

            ParamTag tag() const noexcept { return static_cast<ParamTag>(fstd_module_param_data_tag(*this)); }
            template<typename T>
            T read() const noexcept {
                fstd_dbg_assert(this->tag() == ParamUtil<T>::ParamTag);
                ParamData<T> param = *this;
                return param.read();
            }
            template<typename T>
            void write(T value) const noexcept {
                fstd_dbg_assert(this->tag() == ParamUtil<T>::ParamTag);
                ParamData<T> param = *this;
                param.write(value);
            }
        };

        template<typename T>
        struct SymbolUtil {
            using SymbolPtr = T const *;
            using SymbolRef = T const &;

            constexpr static void const *asPtr(SymbolRef ref) { return &ref; }
            constexpr static SymbolPtr ptrFromPtr(void const *ptr) { return static_cast<SymbolPtr>(ptr); }
            constexpr static SymbolRef refFromPtr(void const *ptr) { return *static_cast<SymbolPtr>(ptr); }
        };

        template<>
        struct SymbolUtil<void> {
            using SymbolPtr = void const *;
            using SymbolRef = void const *;

            constexpr static void const *asPtr(SymbolRef ref) { return ref; }
            constexpr static SymbolPtr ptrFromPtr(void const *ptr) { return ptr; }
            constexpr static SymbolRef refFromPtr(void const *ptr) { return ptr; }
        };

        template<typename Ret, typename... Args>
        struct SymbolUtil<Ret (*)(Args...)> {
            using SymbolPtr = Ret (*)(Args...);
            using SymbolRef = Ret (&)(Args...);

            static void const *asPtr(SymbolRef ref) { return reinterpret_cast<const void *>(ref); }
            static SymbolPtr ptrFromPtr(void const *ptr) {
                return reinterpret_cast<SymbolPtr>(const_cast<void *>(ptr));
            }
            static SymbolRef refFromPtr(void const *ptr) {
                return *reinterpret_cast<SymbolPtr>(const_cast<void *>(ptr));
            }
        };

        template<typename T>
        struct SymbolId : FSTD_ModuleSymbol {
            using Type = SymbolId;
            using FStd = FSTD_ModuleSymbol;
            constexpr SymbolId() noexcept = default;
            constexpr SymbolId(StrConst name, Version version) noexcept : SymbolId(name, "", version) {};
            constexpr SymbolId(StrConst name, StrConst ns, Version version) noexcept :
                FStd{.name = name, .ns = ns, .version = version} {};
            constexpr SymbolId(const FStd &other) noexcept : FStd(other) {};
            constexpr SymbolId(const SymbolId &other) noexcept = default;
            constexpr SymbolId(SymbolId &&other) noexcept = default;
            constexpr SymbolId &operator=(const SymbolId &other) noexcept = default;
            constexpr SymbolId &operator=(SymbolId &&other) noexcept = default;

            using SymbolPtr = SymbolUtil<T>::SymbolPtr;
            using SymbolRef = SymbolUtil<T>::SymbolRef;
        };

        template<typename T, typename Unique = decltype([] {})>
        struct Symbol {
            constexpr Symbol(StrConst name, Version version) noexcept : Symbol(name, "", version) {};
            constexpr Symbol(StrConst name, StrConst ns, Version version) noexcept :
                name(name), ns(ns), version(version) {};
            constexpr Symbol(const Symbol &other) noexcept = default;
            constexpr Symbol(Symbol &&other) noexcept = default;
            constexpr Symbol &operator=(const Symbol &other) noexcept = default;
            constexpr Symbol &operator=(Symbol &&other) noexcept = default;
            constexpr operator SymbolId<T>() const noexcept { return {this->name, this->ns, this->version}; }
            constexpr SymbolId<T> asId() const noexcept { return static_cast<SymbolId<T>>(*this); }

            StrConst name;
            StrConst ns;
            Version version;

            using SymbolPtr = typename SymbolUtil<T>::SymbolPtr;
            using SymbolRef = typename SymbolId<T>::SymbolRef;
            static SymbolPtr getPtr() noexcept {
                auto &handle = getHandle();
                return SymbolUtil<T>::ptrFromPtr(handle.handle);
            }
            static SymbolRef get() noexcept {
                auto &handle = getHandle();
                return SymbolUtil<T>::refFromPtr(handle.handle);
            }
            static void bind(SymbolRef symbol) noexcept {
                auto &handle = getHandle();
                fstd__ref_counted_handle_register(&handle, SymbolUtil<T>::asPtr(symbol));
            }
            static void unbind() noexcept {
                auto &handle = getHandle();
                fstd__ref_counted_handle_unregister(&handle);
            }
            static FSTD__RefCountedHandle &getHandle() noexcept {
                static constinit FSTD__RefCountedHandle handle = {};
                return handle;
            }
        };

        struct Handle {
            using Type = Handle;
            using FStd = FSTD_ModuleHandle *;
            constexpr Handle() noexcept = default;
            constexpr Handle(const FStd &other) noexcept : handle(other) {};
            constexpr Handle(const Handle &other) noexcept = default;
            constexpr Handle(Handle &&other) noexcept = default;
            constexpr Handle &operator=(const Handle &other) noexcept = default;
            constexpr Handle &operator=(Handle &&other) noexcept = default;
            constexpr operator FSTD_ModuleHandle *() const noexcept { return this->handle; };

            FSTD_ModuleHandle *handle;

            StrConst name() const noexcept { return fstd_module_handle_name(this->handle); }
            StrConst description() const noexcept { return fstd_module_handle_description(this->handle); }
            StrConst license() const noexcept { return fstd_module_handle_license(this->handle); }
            Path modulePath() const noexcept { return fstd_module_handle_module_path(this->handle); }
            void ref() const noexcept { return fstd_module_handle_ref(this->handle); }
            void unref() const noexcept { return fstd_module_handle_unref(this->handle); }
            void markUnloadable() const noexcept { return fstd_module_handle_mark_unloadable(this->handle); }
            bool isLoaded() const noexcept { return fstd_module_handle_is_loaded(this->handle); }
            bool tryRefInstanceStrong() const noexcept {
                return fstd_module_handle_try_ref_instance_strong(this->handle);
            }
            void unrefInstanceStrong() const noexcept { return fstd_module_handle_unref_instance_strong(this->handle); }
            static std::expected<Handle, Status> findByName(StrConst module) noexcept {
                Handle handle{};
                Status status = static_cast<Status>(fstd_module_handle_find_by_name(&handle.handle, module));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return handle;
            }
            template<typename T>
            static std::expected<Handle, Status> findBySymbol(SymbolId<T> symbol) noexcept {
                Handle handle{};
                Status status = static_cast<Status>(fstd_module_handle_find_by_symbol(&handle.handle, symbol));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return handle;
            }
        };

        // NOLINTNEXTLINE(performance-enum-size)
        enum class Dependency : FSTD_ModuleDependency {
            None = FSTD_ModuleDependency_None,
            Static = FSTD_ModuleDependency_Static,
            Dynamic = FSTD_ModuleDependency_Dynamic,
        };

        struct Instance {
            using Type = Instance;
            using FStd = FSTD_ModuleInstance *;
            constexpr Instance() noexcept = default;
            constexpr Instance(const FStd &other) noexcept : handle(other) {};
            constexpr Instance(const Instance &other) noexcept = default;
            constexpr Instance(Instance &&other) noexcept = default;
            constexpr Instance &operator=(const Instance &other) noexcept = default;
            constexpr Instance &operator=(Instance &&other) noexcept = default;
            constexpr operator FSTD_ModuleInstance *() const noexcept { return this->handle; };

            FSTD_ModuleInstance *handle;

            Param<void> *const *FSTD_MAYBE_NULL parameters() const noexcept {
                return reinterpret_cast<Param<void> *const *>(fstd_module_instance_parameters(this->handle));
            }
            Path const *FSTD_MAYBE_NULL resources() const noexcept {
                return reinterpret_cast<Path const *>(fstd_module_instance_resources(this->handle));
            }
            const void *const *FSTD_MAYBE_NULL imports() const noexcept {
                return fstd_module_instance_imports(this->handle);
            }
            const void *const *FSTD_MAYBE_NULL exports() const noexcept {
                return fstd_module_instance_exports(this->handle);
            }
            Handle moduleHandle() const noexcept { return fstd_module_instance_handle(this->handle); }
            ctx::Handle ctxHandle() const noexcept { return fstd_module_instance_ctx_handle(this->handle); }
            void const *FSTD_MAYBE_NULL state() const noexcept { return fstd_module_instance_state(this->handle); }

            void ref() const noexcept { return fstd_module_instance_ref(this->handle); }
            void unref() const noexcept { return fstd_module_instance_unref(this->handle); }
            Dependency queryNs(StrConst ns) const noexcept {
                return static_cast<Dependency>(fstd_module_instance_query_namespace(this->handle, ns));
            }
            Status addNs(StrConst ns) const noexcept {
                return static_cast<Status>(fstd_module_instance_add_namespace(this->handle, ns));
            }
            Status removeNs(StrConst ns) const noexcept {
                return static_cast<Status>(fstd_module_instance_remove_namespace(this->handle, ns));
            }
            Dependency queryDep(Handle handle) const noexcept {
                return static_cast<Dependency>(fstd_module_instance_query_dependency(this->handle, handle.handle));
            }
            Status addDep(Handle handle) const noexcept {
                return static_cast<Status>(fstd_module_instance_add_dependency(this->handle, handle.handle));
            }
            Status removeDep(Handle handle) const noexcept {
                return static_cast<Status>(fstd_module_instance_remove_dependency(this->handle, handle.handle));
            }
            template<typename T>
            std::expected<typename SymbolId<T>::SymbolPtr, Status> loadSymbol(SymbolId<T> symbol) const noexcept {
                void const *loaded;
                Status status = static_cast<Status>(fstd_module_instance_load_symbol(this->handle, symbol, &loaded));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return SymbolUtil<T>::ptrFromPtr(loaded);
            }
            template<typename T>
            std::expected<T, Status> readParameter(StrConst module, StrConst parameter) const noexcept {
                T value;
                ParamTag tag = ParamUtil<T>::ParamTag;
                Status status = static_cast<Status>(fstd_module_instance_read_parameter_opaque(
                        this->handle, static_cast<FSTD_ModuleParamTag>(tag), module, parameter, &value));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return value;
            }
            template<typename T>
            Status writeParameter(StrConst module, StrConst parameter, const T &value) const noexcept {
                ParamTag tag = ParamUtil<T>::ParamTag;
                Status status = static_cast<Status>(fstd_module_instance_write_parameter_opaque(
                        this->handle, static_cast<FSTD_ModuleParamTag>(tag), module, parameter, &value));
                return status;
            }
        };

        struct RootInstance {
            using Type = RootInstance;
            using FStd = FSTD_ModuleRootInstance *;
            constexpr RootInstance() noexcept = default;
            constexpr RootInstance(const FStd &other) noexcept : handle(other) {};
            constexpr RootInstance(const RootInstance &other) noexcept = default;
            constexpr RootInstance(RootInstance &&other) noexcept = default;
            constexpr RootInstance &operator=(const RootInstance &other) noexcept = default;
            constexpr RootInstance &operator=(RootInstance &&other) noexcept = default;
            constexpr operator FSTD_ModuleRootInstance *() const noexcept { return this->handle; };

            FSTD_ModuleRootInstance *handle;

            static std::expected<RootInstance, Status> init() noexcept {
                RootInstance inst{};
                Status status = static_cast<Status>(fstd_module_root_instance_init(&inst.handle));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return inst;
            }
            void deinit() const noexcept { return fstd_module_root_instance_deinit(this->handle); }
            Dependency queryNs(StrConst ns) const noexcept {
                return static_cast<Dependency>(fstd_module_root_instance_query_namespace(this->handle, ns));
            }
            Status addNs(StrConst ns) const noexcept {
                return static_cast<Status>(fstd_module_root_instance_add_namespace(this->handle, ns));
            }
            Status removeNs(StrConst ns) const noexcept {
                return static_cast<Status>(fstd_module_root_instance_remove_namespace(this->handle, ns));
            }
            Dependency queryDep(Handle handle) const noexcept {
                return static_cast<Dependency>(fstd_module_root_instance_query_dependency(this->handle, handle.handle));
            }
            Status addDep(Handle handle) const noexcept {
                return static_cast<Status>(fstd_module_root_instance_add_dependency(this->handle, handle.handle));
            }
            Status removeDep(Handle handle) const noexcept {
                return static_cast<Status>(fstd_module_root_instance_remove_dependency(this->handle, handle.handle));
            }
            template<typename T>
            std::expected<typename SymbolId<T>::SymbolPtr, Status> loadSymbol(SymbolId<T> symbol) const noexcept {
                void const *loaded;
                Status status =
                        static_cast<Status>(fstd_module_root_instance_load_symbol(this->handle, symbol, &loaded));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return SymbolUtil<T>::ptrFromPtr(loaded);
            }
            template<typename T>
            std::expected<T, Status> readParameter(StrConst module, StrConst parameter) const noexcept {
                T value;
                ParamTag tag = ParamUtil<T>::ParamTag;
                Status status = static_cast<Status>(fstd_module_root_instance_read_parameter_opaque(
                        this->handle, static_cast<FSTD_ModuleParamTag>(tag), module, parameter, &value));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return value;
            }
            template<typename T>
            Status writeParameter(StrConst module, StrConst parameter, const T &value) const noexcept {
                ParamTag tag = ParamUtil<T>::ParamTag;
                Status status = static_cast<Status>(fstd_module_root_instance_write_parameter_opaque(
                        this->handle, static_cast<FSTD_ModuleParamTag>(tag), module, parameter, &value));
                return status;
            }
        };

        // template<typename T>
        // struct ExportParameter {
        //     constexpr ExportParameter(const StrConst &name, T default_value) noexcept :
        //         name(name), read_group(AccessGroup::Private), write_group(AccessGroup::Private), read(nullptr),
        //         write(nullptr), default_value(default_value) {}
        //     constexpr ExportParameter(const StrConst &name, AccessGroup read_group, AccessGroup write_group,
        //                               T default_value) noexcept :
        //         name(name), read_group(read_group), write_group(write_group), read(nullptr), write(nullptr),
        //         default_value(default_value) {}
        //     constexpr ExportParameter(const StrConst &name, AccessGroup read_group, AccessGroup write_group,
        //                               auto &&read, auto &&write, T default_value) noexcept :
        //         name(name), read_group(read_group), write_group(write_group), read(read), write(write),
        //         default_value(default_value) {}
        //     constexpr ExportParameter(const ExportParameter &other) noexcept = default;
        //     constexpr ExportParameter(ExportParameter &&other) noexcept = default;
        //     constexpr ExportParameter &operator=(const ExportParameter &other) noexcept = default;
        //     constexpr ExportParameter &operator=(ExportParameter &&other) noexcept = default;
        //     constexpr operator FSTD_ModuleExportParameter() const noexcept {
        //         FSTD_ModuleExportParameter param;
        //         param.name = this->name;
        //         param.tag = static_cast<FSTD_ModuleParamTag>(this->tag);
        //         param.read_group = static_cast<FSTD_ModuleAccessGroup>(this->read_group);
        //         param.write_group = static_cast<FSTD_ModuleAccessGroup>(this->write_group);
        //         param.read = this->read;
        //         param.write = this->write;

        //         using BaseType = ParamUtil<T>::BaseType;
        //         if constexpr (std::is_same_v<BaseType, u8>) {
        //             param.u8 = this->default_value;
        //         }
        //         else if constexpr (std::is_same_v<BaseType, u16>) {
        //             param.u16 = this->default_value;
        //         }
        //         else if constexpr (std::is_same_v<BaseType, u32>) {
        //             param.u32 = this->default_value;
        //         }
        //         else if constexpr (std::is_same_v<BaseType, u64>) {
        //             param.u64 = this->default_value;
        //         }
        //         else if constexpr (std::is_same_v<BaseType, i8>) {
        //             param.i8 = this->default_value;
        //         }
        //         else if constexpr (std::is_same_v<BaseType, i16>) {
        //             param.i16 = this->default_value;
        //         }
        //         else if constexpr (std::is_same_v<BaseType, i32>) {
        //             param.i32 = this->default_value;
        //         }
        //         else if constexpr (std::is_same_v<BaseType, i64>) {
        //             param.i64 = this->default_value;
        //         }

        //         return param;
        //     }

        //     StrConst name;
        //     constexpr static ParamTag tag = ParamUtil<T>::ParamTag;
        //     AccessGroup read_group;
        //     AccessGroup write_group;
        //     void (*FSTD_MAYBE_NULL read)(ParamData<T> data, void *value);
        //     void (*FSTD_MAYBE_NULL write)(ParamData<T> data, void const *value);
        //     T default_value;
        // };

        template<typename T>
        struct SymbolIdExt : FSTD_ModuleSymbolExt {
            using Type = SymbolIdExt;
            using FStd = FSTD_ModuleSymbolExt;
            constexpr SymbolIdExt() noexcept = default;
            template<typename Unique>
            constexpr SymbolIdExt(const Symbol<T, Unique> &sym) noexcept :
                FSTD_ModuleSymbolExt{
                        .id = sym,
                        .bind = +[](void const *ptr) { Symbol<T, Unique>::bind(SymbolUtil<T>::refFromPtr(ptr)); },
                        .unbind = Symbol<T, Unique>::unbind,
                } {};
            constexpr SymbolIdExt(const FStd &other) noexcept : FStd(other) {};
            constexpr SymbolIdExt(const SymbolIdExt &other) noexcept = default;
            constexpr SymbolIdExt(SymbolIdExt &&other) noexcept = default;
            constexpr SymbolIdExt &operator=(const SymbolIdExt &other) noexcept = default;
            constexpr SymbolIdExt &operator=(SymbolIdExt &&other) noexcept = default;
            constexpr operator SymbolId<T>() const noexcept {
                return {
                        {this->id.name.ptr, this->id.name.len},
                        {this->id.ns.ptr, this->id.ns.len},
                        this->id.version,
                };
            }
        };

        // NOLINTNEXTLINE(performance-enum-size)
        enum class SymbolType : FSTD_ModuleExportSymbolType {
            Static = FSTD_ModuleExportSymbolType_Static,
            StateOffset = FSTD_ModuleExportSymbolType_StateOffset,
            Dynamic = FSTD_ModuleExportSymbolType_Dynamic,
        };

        // NOLINTNEXTLINE(performance-enum-size)
        enum class SymbolLinkage : FSTD_ModuleExportSymbolLinkage {
            Global = FSTD_ModuleExportSymbolLinkage_Global,
        };

        template<typename T>
        struct SymbolExport {
            constexpr SymbolExport() noexcept = default;
            template<typename Unique>
            constexpr SymbolExport(const Symbol<T, Unique> &sym, typename Symbol<T>::SymbolRef value) noexcept :
                SymbolExport(static_cast<SymbolIdExt<T>>(sym), SymbolLinkage::Global, value) {}
            constexpr SymbolExport(const SymbolIdExt<T> &sym, typename Symbol<T>::SymbolRef value) noexcept :
                SymbolExport(sym, SymbolLinkage::Global, value) {}
            template<typename Unique>
            constexpr SymbolExport(const Symbol<T, Unique> &sym, SymbolLinkage linkage,
                                   typename Symbol<T>::SymbolRef value) noexcept :
                SymbolExport(static_cast<SymbolIdExt<T>>(sym), linkage, value) {}
            constexpr SymbolExport(const SymbolIdExt<T> &sym, SymbolLinkage linkage,
                                   typename Symbol<T>::SymbolRef value) noexcept :
                symbol(sym), type(SymbolType::Static), linkage(linkage), static_value(&value) {};

            template<auto Member, typename Unique>
            constexpr SymbolExport(const Symbol<T, Unique> &sym, ConstexprValue<Member> value) noexcept :
                SymbolExport(static_cast<SymbolIdExt<T>>(sym), SymbolLinkage::Global, value) {}
            template<auto Member>
            constexpr SymbolExport(const SymbolIdExt<T> &sym, ConstexprValue<Member> value) noexcept :
                SymbolExport(sym, SymbolLinkage::Global, value) {}
            template<auto Member, typename Unique>
            constexpr SymbolExport(const Symbol<T, Unique> &sym, SymbolLinkage linkage,
                                   ConstexprValue<Member> value) noexcept :
                SymbolExport(static_cast<SymbolIdExt<T>>(sym), linkage, value){};
            template<auto Member>
            constexpr SymbolExport(const SymbolIdExt<T> &sym, SymbolLinkage linkage, ConstexprValue<Member>) noexcept :
                symbol(sym), type(SymbolType::StateOffset), linkage(linkage),
                state_offset(detail::offsetOf<Member>()){};

            constexpr SymbolExport(const SymbolExport &other) noexcept = default;
            constexpr SymbolExport(SymbolExport &&other) noexcept = default;
            constexpr SymbolExport &operator=(const SymbolExport &other) noexcept = default;
            constexpr SymbolExport &operator=(SymbolExport &&other) noexcept = default;

            SymbolIdExt<T> symbol;
            SymbolType type;
            SymbolLinkage linkage;
            union {
                SymbolUtil<T>::SymbolPtr static_value;
                usize state_offset;
                struct {
                    using SymbolPtr = std::remove_const_t<typename SymbolUtil<T>::SymbolPtr>;
                    using InitPollResult = FSTD_Fallible(SymbolPtr);
                    bool (*poll_init)(Instance ctx, tasks::Waker waker, InitPollResult &result);
                    bool (*FSTD_MAYBE_NULL poll_deinit)(Instance ctx, tasks::Waker waker, SymbolPtr value);
                } dynamic_value;
            };
        };

        template<typename... Args>
        static consteval auto makeExport(Args &&...args) {
            return SymbolExport{std::forward<Args>(args)...};
        }

        template<auto Member, typename... Args>
        static consteval SymbolExport<typename detail::MemberPointerInfo<Member>::Type>
        makeMemberExport(Args &&...args) {
            return SymbolExport{std::forward<Args>(args)..., ConstexprValue<Member>{}};
        }


        // template<typename... Ts>
        // struct ParameterList {
        //     constexpr ParameterList(const ExportParameter<Ts> &...args) noexcept : arr{args...} {}
        //     constexpr ParameterList(const std::array<FSTD_ModuleExportParameter, sizeof...(Ts)> &exports) noexcept :
        //         arr{exports} {};
        //     constexpr ParameterList(const ParameterList &other) noexcept = default;
        //     constexpr ParameterList(ParameterList &&other) noexcept = default;
        //     constexpr ParameterList &operator=(const ParameterList &other) noexcept = default;
        //     constexpr ParameterList &operator=(ParameterList &&other) noexcept = default;

        //     constexpr static usize NumExports = sizeof...(Ts);
        //     std::array<FSTD_ModuleExportParameter, NumExports> arr;

        //     template<usize Index>
        //     constexpr std::tuple_element_t<Index, std::tuple<ExportParameter<Ts>...>> get() noexcept {
        //         return this->arr[Index];
        //     }
        //     template<typename T>
        //     constexpr ExportParameter<Ts..., T> withImpl(const ExportParameter<T> &sym) noexcept {
        //         std::array<FSTD_ModuleExportParameter, sizeof...(Ts) + 1> arr{};
        //         for (usize i = 0; i < NumExports; i++) {
        //             arr[i] = this->arr[i];
        //         }
        //         arr[NumExports] = sym;
        //         return {arr};
        //     }
        //     template<typename... Args>
        //     constexpr auto with(Args &&...args) noexcept -> decltype(auto) {
        //         ExportParameter param{std::forward<Args>(args)...};
        //         return this->withImpl(param);
        //     }
        // };

        template<typename... Ts>
        struct SymbolImportList {
            template<typename... Us>
            constexpr SymbolImportList(const Symbol<Ts, Us> &...args) noexcept :
                SymbolImportList(static_cast<SymbolIdExt<Ts>>(args)...) {}
            constexpr SymbolImportList(const SymbolIdExt<Ts> &...args) noexcept : arr{args...} {}
            constexpr SymbolImportList(const std::array<FSTD_ModuleSymbolExt, sizeof...(Ts)> &imports) noexcept :
                arr{imports} {};
            constexpr SymbolImportList(const SymbolImportList &other) noexcept = default;
            constexpr SymbolImportList(SymbolImportList &&other) noexcept = default;
            constexpr SymbolImportList &operator=(const SymbolImportList &other) noexcept = default;
            constexpr SymbolImportList &operator=(SymbolImportList &&other) noexcept = default;

            constexpr static usize NumImports = sizeof...(Ts);
            std::array<FSTD_ModuleSymbolExt, NumImports> arr;

            template<usize Index>
            constexpr std::tuple_element_t<Index, std::tuple<SymbolIdExt<Ts>...>> get() const noexcept {
                return this->arr[Index];
            }
            template<typename F, usize... I>
            constexpr void forEachImpl(F &&f, std::index_sequence<I...>) const {
                ((std::invoke(f, this->template get<I>())), ...);
            }
            template<typename F>
            constexpr void forEach(F &&f) const {
                return this->forEachImpl(std::forward<F>(f), std::index_sequence_for<Ts...>{});
            }

            template<typename... Us>
            constexpr SymbolImportList<Ts..., Us...> with(const SymbolImportList<Us...> &other) const noexcept {
                std::array<FSTD_ModuleSymbolExt, sizeof...(Ts) + sizeof...(Us)> arr{};
                for (usize i = 0; i < NumImports; i++) {
                    arr[i] = this->arr[i];
                }
                for (usize i = 0; i < other.NumImports; i++) {
                    arr[NumImports + i] = other.arr[i];
                }
                return {arr};
            }
            template<typename T>
            constexpr SymbolImportList<Ts..., T> with(const SymbolIdExt<T> &sym) const noexcept {
                std::array<FSTD_ModuleSymbolExt, sizeof...(Ts) + 1> imports{};
                for (usize i = 0; i < NumImports; i++) {
                    imports[i] = this->arr[i];
                }
                imports[NumImports] = sym;
                return {imports};
            }
            template<typename T, typename Unique>
            constexpr SymbolImportList<Ts..., T> with(const Symbol<T, Unique> &sym) const noexcept {
                return this->with(static_cast<SymbolIdExt<T>>(sym));
            }
        };

        template<typename... Ts>
        struct SymbolExportList {
            constexpr SymbolExportList(const SymbolExport<Ts> &...args) noexcept : symbols() {
                copyToTuple(this->symbols, args..., std::make_index_sequence<sizeof...(Ts)>{});
            }
            constexpr SymbolExportList(const SymbolExportList &other) noexcept = default;
            constexpr SymbolExportList(SymbolExportList &&other) noexcept = default;
            constexpr SymbolExportList &operator=(const SymbolExportList &other) noexcept = default;
            constexpr SymbolExportList &operator=(SymbolExportList &&other) noexcept = default;

            constexpr static usize NumExports = sizeof...(Ts);
            detail::Tuple<SymbolExport<Ts>...> symbols;

            template<usize... I>
            static constexpr void copyToTuple(auto &tuple, const SymbolExport<Ts> &...args, std::index_sequence<I...>) {
                ((tuple.template getMember<I>() = args), ...);
            }
            template<typename T, usize... I>
            constexpr SymbolExportList<Ts..., T> withImpl(const SymbolExport<T> &sym,
                                                          std::index_sequence<I...>) noexcept {
                return {this->symbols.template getMember<I>()..., sym};
            }
            template<typename... Args>
            constexpr auto with(Args &&...args) noexcept -> decltype(auto) {
                SymbolExport exp{std::forward<Args>(args)...};
                return this->withImpl(exp, std::make_index_sequence<sizeof...(Ts)>{});
            }
        };

        struct Export;
        struct Loader {
            using Type = Loader;
            using FStd = FSTD_ModuleLoader *;
            constexpr Loader() noexcept = default;
            constexpr Loader(const FStd &other) noexcept : handle(other) {};
            constexpr Loader(const Loader &other) noexcept = default;
            constexpr Loader(Loader &&other) noexcept = default;
            constexpr Loader &operator=(const Loader &other) noexcept = default;
            constexpr Loader &operator=(Loader &&other) noexcept = default;
            constexpr operator FSTD_ModuleLoader *() const noexcept { return this->handle; };

            struct ResolvedModule {
                std::optional<Handle> handle;
                const Export &module;
            };

            // NOLINTNEXTLINE(performance-enum-size)
            enum class FilterRequest : FSTD_ModuleLoaderFilterRequest {
                Skip = FSTD_ModuleLoaderFilterRequest_Skip,
                Load = FSTD_ModuleLoaderFilterRequest_Load,
            };

            FSTD_ModuleLoader *handle;

            static std::expected<Loader, Status> init() noexcept {
                Loader loader;
                Status status = static_cast<Status>(fstd_module_loader_init(&loader.handle));
                if (status != Status::Ok)
                    return std::unexpected(status);
                return loader;
            }
            void deinit() const noexcept { return fstd_module_loader_deinit(this->handle); }
            bool containsModule(StrConst module) const noexcept {
                return fstd_module_loader_contains_module(this->handle, module);
            }
            template<typename T>
            bool containsSymbol(SymbolId<T> symbol) const noexcept {
                return fstd_module_loader_contains_symbol(this->handle, symbol);
            }
            tasks::PollResult<std::expected<ResolvedModule, Result>> pollModule(tasks::Waker waker,
                                                                                StrConst module) const noexcept;
            auto pollModuleFuture(StrConst module) const noexcept -> decltype(auto) {
                struct Future {
                    Loader loader;
                    StrConst module;
                    using ResultType = std::expected<ResolvedModule, Result>;
                    tasks::PollResult<ResultType> poll(tasks::Waker waker) const noexcept {
                        return this->loader.pollModule(waker, this->module);
                    }
                };
                return Future{.loader = *this, .module = module};
            }
            Status addModule(Instance owner, const Export &module) const noexcept;
            Status addModulesFromPath(Path path, auto &&filter) const noexcept;
            Status addModulesFromIter(auto &&filter) const noexcept;
            tasks::OpaqueFuture<Result> commit() const noexcept {
                return std::bit_cast<tasks::OpaqueFuture<Result>>(fstd_module_loader_commit(this->handle));
            }
        };

#if defined(FSTD_PLATFORM_WINDOWS) && !defined(FSTD_COMPILER_CLANG)
#define FSTD_CXX_EXPORT_MODULE(ModuleInfo)                                                                             \
    __declspec(allocate(FSTD__MODULE_SECTION)) constinit const ::fstd::modules::Export *FSTD_IDENT(                    \
            fstd__cxx_module_export_) = &ModuleInfo.moduleExport();
#else
#define FSTD_CXX_EXPORT_MODULE(ModuleInfo)                                                                             \
    constexpr static const ::fstd::modules::Export *FSTD_IDENT(fstd__cxx_module_export_)                               \
            __attribute__((retain, used, section(FSTD__MODULE_SECTION))) = &ModuleInfo.moduleExport();
#endif

        struct Export {
            using Type = Export;
            using FStd = FSTD_ModuleExport;
            constexpr Export() noexcept = default;
            template<typename... Imports, typename... Exports, typename OnEvent>
            constexpr Export(const StrConst &name, const StrConst &description, const StrConst &author,
                             const StrConst &license, const Slice<const Path> &resources,
                             const Slice<const StrConst> &namespaces, const SymbolImportList<Imports...> &imports,
                             const SymbolExportList<Exports...> &exports, OnEvent &&on_event) noexcept {
                this->version = ctx::CurrentVersion;
                this->name = name;
                this->description = description;
                this->author = author;
                this->license = license;
                this->parameters = {};
                this->resources = resources;
                this->namespaces = namespaces;
                this->imports = imports.arr;
                this->exports = {static_cast<const void *>(&exports.symbols), exports.NumExports};
                this->on_event = on_event;
            }
            constexpr Export(const Export &other) noexcept = default;
            constexpr Export(Export &&other) noexcept = default;
            constexpr Export &operator=(const Export &other) noexcept = default;
            constexpr Export &operator=(Export &&other) noexcept = default;

            Version version;
            StrConst name;
            StrConst description;
            StrConst author;
            StrConst license;
            detail::ConstUnknownSlice parameters;
            Slice<const Path> resources;
            Slice<const StrConst> namespaces;
            Slice<const FSTD_ModuleSymbolExt> imports;
            detail::ConstUnknownSlice exports;
            void (*on_event)(const FSTD_ModuleExport *module, FSTD_ModuleExportEventTag *tag);
        };

        // TODO(gabriel, https://github.com/llvm/llvm-project/issues/82994): Replace with consteval.
        template<typename T>
        static constexpr auto makeModule() {
            constexpr static StrConst Name = T::Name;
            constexpr static StrConst Description = []() -> StrConst {
                if constexpr (requires { T::Description; }) {
                    return T::Description;
                }
                else {
                    return "";
                }
            }();
            constexpr static StrConst Author = []() -> StrConst {
                if constexpr (requires { T::Author; }) {
                    return T::Author;
                }
                else {
                    return "";
                }
            }();
            constexpr static StrConst License = []() -> StrConst {
                if constexpr (requires { T::License; }) {
                    return T::License;
                }
                else {
                    return "";
                }
            }();
            constexpr static auto Resources = [] {
                if constexpr (requires { T::Resources; }) {
                    return T::Resources;
                }
                else {
                    return std::array<Path, 0>{};
                }
            }();
            constexpr static auto NumNamespaces = [] -> usize {
                usize count = 0;
                if constexpr (requires { T::Imports; }) {
                    for (usize i = 0; i < T::Imports.NumImports; i++) {
                        StrConst ns = T::Imports.arr[i].id.ns;
                        if (ns.empty())
                            continue;
                        bool found = false;
                        for (usize j = 0; j < i; j++) {
                            StrConst other = T::Imports.arr[j].id.ns;
                            if (ns == other) {
                                found = true;
                                break;
                            }
                        }
                        if (!found)
                            count++;
                    }
                }
                return count;
            }();
            constexpr static auto Namespaces = [] {
                usize next = 0;
                std::array<StrConst, NumNamespaces> namespaces{};
                if constexpr (requires { T::Imports; }) {
                    for (usize i = 0; i < T::Imports.NumImports; i++) {
                        StrConst ns = T::Imports.arr[i].id.ns;
                        if (ns.empty())
                            continue;
                        bool found = false;
                        for (usize j = 0; j < i; j++) {
                            StrConst other = T::Imports.arr[j].id.ns;
                            if (ns == other) {
                                found = true;
                                break;
                            }
                        }
                        if (!found) {
                            namespaces[next] = ns;
                            next++;
                        }
                    }
                }
                return namespaces;
            }();
            constexpr static auto Imports = [] {
                if constexpr (requires { T::Imports; }) {
                    return T::Imports;
                }
                else {
                    return SymbolImportList<>{};
                }
            }();
            constexpr static auto Exports = [] {
                if constexpr (requires { T::Exports; }) {
                    return T::Exports;
                }
                else {
                    return SymbolExportList<>{};
                }
            }();
            using State = decltype([] {
                if constexpr (requires { typename T::State; }) {
                    return std::type_identity<typename T::State>{};
                }
                else {
                    return std::type_identity<T>{};
                }
            }())::type;

            struct GlobalData {
                bool is_init;
                Instance instance;
                State *state;
                alignas(State) unsigned char storage[sizeof(State)];

                static GlobalData &get() noexcept {
                    constinit static GlobalData data = {};
                    return data;
                }
            };
            constexpr static auto OnEvent = [](FSTD_ModuleExport const *module_export, FSTD_ModuleExportEventTag *tag) {
                switch (*tag) {
                    case FSTD_ModuleExportEventTag_BindCtx:
                        if constexpr (requires { T::onBind; }) {
                            auto *event = parentOf(tag, ConstexprValue<&FSTD_ModuleExportEventBindCtx::tag>{});
                            event->bind = [](FSTD_Ctx *cctx) noexcept {
                                ctx::Handle ctx = cctx;
                                std::invoke_r<void>(T::onBind, ctx);
                            };
                            return;
                        };
                        break;
                    case FSTD_ModuleExportEventTag_UnbindCtx:
                        if constexpr (requires { T::onUnbind; }) {
                            auto *event = parentOf(tag, ConstexprValue<&FSTD_ModuleExportEventUnbindCtx::tag>{});
                            event->unbind = []() noexcept { std::invoke_r<void>(T::onUnbind); };
                            return;
                        };
                        break;
                    case FSTD_ModuleExportEventTag_Init: {
                        auto *event = parentOf(tag, ConstexprValue<&FSTD_ModuleExportEventInit::tag>{});
                        if constexpr (requires { T::onInit; }) {
                            event->poll = [](FSTD_ModuleInstance *cinstance, FSTD_ModuleLoader *cloader,
                                             FSTD_TaskWaker cwaker,
                                             FSTD_ModuleExportEventInitResult *cresult) noexcept {
                                auto &global = GlobalData::get();
                                fstd_assert(!global.is_init);
                                global.instance = cinstance;
                                Loader loader = cloader;
                                tasks::Waker waker = cwaker;
                                State *ptr = reinterpret_cast<State *>(global.storage);
                                auto result = std::invoke_r<tasks::PollResult<Result>>(T::onInit, ptr, loader, waker);
                                if (std::holds_alternative<tasks::PollPendingType>(result)) {
                                    return false;
                                }
                                global.is_init = true;
                                global.state = std::launder(ptr);
                                cresult->value = global.state;
                                cresult->result = std::get<Result>(result);
                                return true;
                            };
                        }
                        else {
                            event->poll = [](FSTD_ModuleInstance *instance, FSTD_ModuleLoader *, FSTD_TaskWaker,
                                             FSTD_ModuleExportEventInitResult *result) noexcept {
                                auto &global = GlobalData::get();
                                fstd_assert(!global.is_init);
                                global.is_init = true;
                                global.instance = instance;
                                global.state = std::construct_at(reinterpret_cast<State *>(global.storage));
                                result->value = global.state;
                                result->result = Result{};
                                return true;
                            };
                        };
                        return;
                    }
                    case FSTD_ModuleExportEventTag_Deinit: {
                        auto *event = parentOf(tag, ConstexprValue<&FSTD_ModuleExportEventDeinit::tag>{});
                        if constexpr (requires { T::onDeinit; }) {
                            event->poll = [](FSTD_ModuleInstance *, FSTD_TaskWaker cwaker, void *) noexcept {
                                auto &global = GlobalData::get();
                                fstd_assert(global.is_init);
                                tasks::Waker waker = cwaker;
                                auto result = std::invoke_r<tasks::PollResult<void>>(T::onDeinit, *global.state, waker);
                                if (std::holds_alternative<tasks::PollPendingType>(result)) {
                                    return false;
                                }
                                global.is_init = false;
                                global.state = nullptr;
                                return true;
                            };
                        }
                        else {
                            event->poll = [](FSTD_ModuleInstance *, FSTD_TaskWaker, void *) noexcept {
                                auto &global = GlobalData::get();
                                fstd_assert(global.is_init);
                                global.is_init = false;
                                global.instance = {};
                                std::destroy_at(global.state);
                                global.state = nullptr;
                                return true;
                            };
                        };
                        return;
                    }
                    case FSTD_ModuleExportEventTag_Start: {
                        auto *event = parentOf(tag, ConstexprValue<&FSTD_ModuleExportEventStart::tag>{});
                        if constexpr (requires { T::onStart; }) {
                            event->poll = [](FSTD_ModuleInstance *, FSTD_TaskWaker cwaker,
                                             FSTD_Result *cresult) noexcept {
                                auto &global = GlobalData::get();
                                fstd_assert(global.is_init);
                                tasks::Waker waker = cwaker;
                                auto result = std::invoke_r<tasks::PollResult<Result>>(T::onStart, waker);
                                if (std::holds_alternative<tasks::PollPendingType>(result)) {
                                    return false;
                                }
                                *cresult = std::get<Result>(result);
                                return true;
                            };
                        }
                    } break;
                    case FSTD_ModuleExportEventTag_Stop: {
                        auto *event = parentOf(tag, ConstexprValue<&FSTD_ModuleExportEventStop::tag>{});
                        if constexpr (requires { T::onStop; }) {
                            event->poll = [](FSTD_ModuleInstance *, FSTD_TaskWaker cwaker,
                                             FSTD_Result *cresult) noexcept {
                                auto &global = GlobalData::get();
                                fstd_assert(global.is_init);
                                tasks::Waker waker = cwaker;
                                auto result = std::invoke_r<tasks::PollResult<Result>>(T::onStop, waker);
                                if (std::holds_alternative<tasks::PollPendingType>(result)) {
                                    return false;
                                }
                                *cresult = std::get<Result>(result);
                                return true;
                            };
                        }
                    } break;
                    default:
                        break;
                }
                fstd_module_export_default_on_event(module_export, tag);
            };
            constexpr static Export ModuleExport = {
                    Name, Description, Author, License, Resources, Namespaces, Imports, Exports, OnEvent,
            };

            constexpr static auto loadSymbolImpl = []<typename U>(SymbolId<U> symbol) static noexcept {
                auto &global = GlobalData::get();
                fstd_dbg_assert(global.is_init);
                return global.instance.template loadSymbol<U>(symbol);
            };
            constexpr static auto readParameterImpl = []<typename U>(StrConst module,
                                                                     StrConst parameter) static noexcept {
                auto &global = GlobalData::get();
                fstd_dbg_assert(global.is_init);
                return global.instance.template readParameter<U>(module, parameter);
            };
            constexpr static auto writeParameterImpl = []<typename U>(StrConst module, StrConst parameter,
                                                                      const U &value) static noexcept {
                auto &global = GlobalData::get();
                fstd_dbg_assert(global.is_init);
                return global.instance.template writeParameter<U>(module, parameter, value);
            };
            struct Module {
                static constexpr const Export &moduleExport() noexcept { return ModuleExport; }
                static bool isInit() noexcept { return GlobalData::get().is_init; }
                static Instance instance() noexcept {
                    fstd_dbg_assert(isInit());
                    return GlobalData::get().instance;
                }
                static Handle moduleHandle() noexcept { return instance().moduleHandle(); }
                static ctx::Handle ctxHandle() noexcept { return instance().ctxHandle(); }
                static State &state() noexcept {
                    fstd_dbg_assert(isInit());
                    return *GlobalData::get().state;
                }
                static void ref() noexcept { return instance().ref(); }
                static void unref() noexcept { return instance().unref(); }
                static Dependency queryNs(StrConst ns) noexcept { return instance().queryNs(ns); }
                static Status addNs(StrConst ns) noexcept { return instance().addNs(ns); }
                static Status removeNs(StrConst ns) noexcept { return instance().removeNs(ns); }
                static Dependency queryDep(Handle handle) noexcept { return instance().queryDep(handle); }
                static Status addDep(Handle handle) noexcept { return instance().addDep(handle); }
                static Status removeDep(Handle handle) noexcept { return instance().removeDep(handle); }
                decltype(loadSymbolImpl) loadSymbol = loadSymbolImpl;
                decltype(readParameterImpl) readParameter = readParameterImpl;
                decltype(writeParameterImpl) writeParameter = writeParameterImpl;
            };
            return Module{};
        }

        tasks::PollResult<std::expected<Loader::ResolvedModule, Result>>
        Loader::pollModule(tasks::Waker waker, StrConst module) const noexcept {
            FSTD_ModuleLoaderPollModuleResult cresult;
            if (!fstd_module_loader_poll_module(this->handle, waker, module, &cresult))
                return tasks::PollPending;
            Result result = cresult.result;
            if (result.isErr())
                return std::unexpected(result);

            std::optional<Handle> handle =
                    cresult.value.handle ? std::optional<Handle>{cresult.value.handle} : std::nullopt;
            const Export *module_ptr = reinterpret_cast<const Export *>(cresult.value.module);
            return ResolvedModule{handle, *module_ptr};
        }
        Status Loader::addModule(Instance owner, const Export &module) const noexcept {
            return static_cast<Status>(fstd_module_loader_add_module(
                    this->handle, owner, reinterpret_cast<const FSTD_ModuleExport *>(&module)));
        }
        Status Loader::addModulesFromPath(Path path, auto &&filter) const noexcept {
            using F = decltype(filter);
            auto wrapper = +[](void *data, const FSTD_ModuleExport *module) {
                F &&filter = static_cast<F &&>(*static_cast<std::remove_cvref_t<F> *>(data));
                auto request = std::invoke_r<FilterRequest>(std::forward<F>(filter),
                                                            *reinterpret_cast<const Export *>(module));
                return static_cast<FSTD_ModuleLoaderFilterRequest>(request);
            };
            return static_cast<Status>(fstd_module_loader_add_modules_from_path(this->handle, path, &filter, wrapper));
        }
        Status Loader::addModulesFromIter(auto &&filter) const noexcept {
            using F = decltype(filter);
            auto wrapper = +[](void *data, const FSTD_ModuleExport *module) {
                F &&filter = static_cast<F &&>(*static_cast<std::remove_cvref_t<F> *>(data));
                auto request = std::invoke_r<FilterRequest>(std::forward<F>(filter),
                                                            *reinterpret_cast<const Export *>(module));
                return static_cast<FSTD_ModuleLoaderFilterRequest>(request);
            };
            return static_cast<Status>(fstd_module_loader_add_modules_from_iter(this->handle, &filter, wrapper));
        }

        // NOLINTNEXTLINE(performance-enum-size)
        enum class Profile : FSTD_ModulesProfile {
            Default = FSTD_ModulesProfile_Default,
            Release = FSTD_ModulesProfile_Release,
            Dev = FSTD_ModulesProfile_Dev,
        };

        // NOLINTNEXTLINE(performance-enum-size)
        enum class FeatureTag : FSTD_ModulesFeatureTag {};

        // NOLINTNEXTLINE(performance-enum-size)
        enum class FeatureRequestFlag : FSTD_ModulesFeatureRequestFlag {
            Required = FSTD_ModulesFeatureRequestFlag_Required,
            On = FSTD_ModulesFeatureRequestFlag_On,
            Off = FSTD_ModulesFeatureRequestFlag_Off,
        };

        struct FeatureRequest : FSTD_ModulesFeatureRequest {
            using Type = FeatureRequest;
            using FStd = FSTD_ModulesFeatureRequest;
            constexpr FeatureRequest() noexcept = default;
            constexpr FeatureRequest(FeatureTag tag) noexcept : FeatureRequest(tag, FeatureRequestFlag::Required) {};
            constexpr FeatureRequest(FeatureTag tag, FeatureRequestFlag flag) noexcept :
                FStd{.tag = static_cast<FSTD_ModulesFeatureTag>(tag),
                     .flag = static_cast<FSTD_ModulesFeatureRequestFlag>(flag)} {};
            constexpr FeatureRequest(const FStd &other) noexcept : FStd(other) {};
            constexpr FeatureRequest(const FeatureRequest &other) noexcept = default;
            constexpr FeatureRequest(FeatureRequest &&other) noexcept = default;
            constexpr FeatureRequest &operator=(const FeatureRequest &other) noexcept = default;
            constexpr FeatureRequest &operator=(FeatureRequest &&other) noexcept = default;
        };

        // NOLINTNEXTLINE(performance-enum-size)
        enum class FeatureStatusFlag : FSTD_ModulesFeatureStatusFlag {
            On = FSTD_ModulesFeatureStatusFlag_On,
            Off = FSTD_ModulesFeatureStatusFlag_Off,
        };

        struct FeatureStatus : FSTD_ModulesFeatureStatus {
            using Type = FeatureStatus;
            using FStd = FSTD_ModulesFeatureStatus;
            constexpr FeatureStatus() noexcept = default;
            constexpr FeatureStatus(FeatureTag tag, FeatureStatusFlag flag) noexcept :
                FStd{.tag = static_cast<FSTD_ModulesFeatureTag>(tag),
                     .flag = static_cast<FSTD_ModulesFeatureStatusFlag>(flag)} {};
            constexpr FeatureStatus(const FStd &other) noexcept : FStd(other) {};
            constexpr FeatureStatus(const FeatureStatus &other) noexcept = default;
            constexpr FeatureStatus(FeatureStatus &&other) noexcept = default;
            constexpr FeatureStatus &operator=(const FeatureStatus &other) noexcept = default;
            constexpr FeatureStatus &operator=(FeatureStatus &&other) noexcept = default;
        };
        constexpr static Profile DefaultProfile = static_cast<Profile>(FSTD_MODULES_DEFAULT_PROFILE);

        struct Cfg : FSTD_ModulesCfg {
            using Type = Cfg;
            using FStd = FSTD_ModulesCfg;
            constexpr Cfg() noexcept : Cfg(DefaultProfile, {}) {};
            constexpr Cfg(Profile profile, Slice<const FeatureRequest> features) noexcept :
                FSTD_ModulesCfg{
                        .id = {.id = static_cast<FSTD_CfgId>(ctx::CfgId::Modules)},
                        .profile = static_cast<FSTD_ModulesProfile>(profile),
                        .features = features,
                } {};
            constexpr Cfg(const FStd &other) noexcept : FStd(other) {};
            constexpr Cfg(const Cfg &other) noexcept = default;
            constexpr Cfg(Cfg &&other) noexcept = default;
            constexpr Cfg &operator=(const Cfg &other) noexcept = default;
            constexpr Cfg &operator=(Cfg &&other) noexcept = default;
        };

        inline static Profile profile() noexcept { return static_cast<Profile>(fstd_modules_profile()); }
        inline static Slice<const FeatureStatus> features() noexcept {
            auto features = fstd_modules_features();
            return {static_cast<const FeatureStatus *>(features.ptr), features.len};
        }
        inline static bool nsExists(StrConst ns) noexcept { return fstd_modules_namespace_exists(ns); }
        inline static Status pruneInstances() noexcept { return static_cast<Status>(fstd_modules_prune_instances()); }
        inline static std::expected<ParamInfo, Status> queryParameter(StrConst module, StrConst parameter) noexcept {
            ParamInfo info{};
            Status status = static_cast<Status>(fstd_modules_query_parameter(module, parameter, &info));
            if (status != Status::Ok)
                return std::unexpected(status);
            return info;
        }
        template<typename T>
        inline static std::expected<T, Status> readParameter(StrConst module, StrConst parameter) noexcept {
            T value;
            ParamTag tag = ParamUtil<T>::ParamTag;
            Status status = static_cast<Status>(fstd_modules_read_parameter_opaque(
                    static_cast<FSTD_ModuleParamTag>(tag), module, parameter, &value));
            if (status != Status::Ok)
                return std::unexpected(status);
            return value;
        }
        template<typename T>
        inline static Status writeParameter(StrConst module, StrConst parameter, const T &value) noexcept {
            ParamTag tag = ParamUtil<T>::ParamTag;
            Status status = static_cast<Status>(fstd_modules_write_parameter_opaque(
                    static_cast<FSTD_ModuleParamTag>(tag), module, parameter, &value));
            return status;
        }
    } // namespace modules

} // namespace fstd
#endif

FSTD_PRAGMA_GCC(GCC diagnostic pop)

#ifdef FIMO_STD_IMPLEMENTATION

#ifdef __cplusplus
extern "C" {
#endif

// -----------------------------------------
// context api -----------------------------
// -----------------------------------------

fstd_glob_impl FSTD__Ctx fstd__ctx_global = {};

// NOTE(gabriel): Is sound, as while registered the pointer won't change.
fstd_func_impl FSTD_Ctx *fstd_ctx_get(void) { return fstd__ctx_global.ctx; }

fstd_func_impl void fstd_ctx_register(FSTD_Ctx *ctx) {
    fstd__ref_counted_handle_register(&fstd__ctx_global.handle, ctx);
}

fstd_func_impl void fstd_ctx_unregister(void) { fstd__ref_counted_handle_unregister(&fstd__ctx_global.handle); }

fstd_func_impl void fstd_ctx_deinit(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->core_v0.deinit();
}

fstd_func_impl FSTD_Version fstd_ctx_get_version(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->get_version();
}

fstd_func FSTD_Arena *fstd_ctx_get_global_arena(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->core_v0.get_global_arena();
}

fstd_func FSTD_Arena *fstd_ctx_get_scratch_arena(FSTD_Arena *FSTD_MAYBE_NULL conflict) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->core_v0.get_scratch_arena(conflict);
}

fstd_func_impl bool fstd_ctx_has_error_result(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->core_v0.has_error_result();
}

fstd_func_impl FSTD_Result fstd_ctx_replace_result(FSTD_Result new_result) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->core_v0.replace_result(new_result);
}

fstd_func_impl FSTD_Result fstd_ctx_take_result(void) { return fstd_ctx_replace_result(FSTD_Result_Ok); }

fstd_func_impl void fstd_ctx_clear_result(void) { fstd_result_deinit(fstd_ctx_take_result()); }

fstd_func_impl void fstd_ctx_set_result(FSTD_Result new_result) {
    fstd_result_deinit(fstd_ctx_replace_result(new_result));
}

// -----------------------------------------
// async subsystem -------------------------
// -----------------------------------------

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_waiter_init(FSTD_TaskWaiter *waiter) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tasks_v0.waiter_init(waiter);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_future_enqueue(const void *data, FSTD_USize data_size,
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

fstd_func_impl FSTD_CallStack *fstd_call_stack_init(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tracing_v0.init_call_stack();
}

fstd_func_impl void fstd_call_stack_finish(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.deinit_call_stack(stack, false);
}

fstd_func_impl void fstd_call_stack_abort(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.deinit_call_stack(stack, true);
}

fstd_func_impl FSTD_CallStack *fstd_call_stack_replace_current(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tracing_v0.replace_current_call_stack(stack);
}

fstd_func_impl void fstd_call_stack_unblock(FSTD_CallStack *stack) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.unblock_call_stack(stack);
}

fstd_func_impl void fstd_call_stack_suspend_current(bool mark_blocked) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.suspend_current_call_stack(mark_blocked);
}

fstd_func_impl void fstd_call_stack_resume_current(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.resume_current_call_stack();
}

fstd_func_impl bool fstd_tracing_is_enabled(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->tracing_v0.is_enabled();
}

fstd_func_impl void fstd_tracing_register_thread(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.register_thread();
}

fstd_func_impl void fstd_tracing_unregister_thread(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.unregister_thread();
}

fstd_func_impl void fstd_tracing_enter_span(const FSTD_TracingEventInfo *info, FSTD_TracingFmtFn fmt,
                                            const void *fmt_data) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.enter_span(info, fmt, fmt_data);
}

fstd_func_impl void fstd_tracing_exit_span(const FSTD_TracingEventInfo *info) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.exit_span(info);
}

fstd_func_impl void fstd_tracing_log_message(const FSTD_TracingEventInfo *info, FSTD_TracingFmtFn fmt,
                                             const void *fmt_data) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->tracing_v0.log_message(info, fmt, fmt_data);
}

// -----------------------------------------
// modules subsystem -----------------------
// -----------------------------------------

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_module_handle_find_by_name(FSTD_ModuleHandle **handle,
                                                                          FSTD_StrConst module) {
    FSTD_Ctx *h = fstd_ctx_get();
    return h->modules_v0.handle_find_by_name(handle, module);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_module_handle_find_by_symbol(FSTD_ModuleHandle **handle,
                                                                            FSTD_ModuleSymbol symbol) {
    FSTD_Ctx *h = fstd_ctx_get();
    return h->modules_v0.handle_find_by_symbol(handle, symbol);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_module_root_instance_init(FSTD_ModuleRootInstance **ctx) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.root_instance_init(ctx);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_module_loader_init(FSTD_ModuleLoader **loader) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_init(loader);
}

fstd_func_impl void fstd_module_loader_deinit(FSTD_ModuleLoader *loader) {
    FSTD_Ctx *handle = fstd_ctx_get();
    handle->modules_v0.loader_deinit(loader);
}

fstd_func_impl bool fstd_module_loader_contains_module(FSTD_ModuleLoader *loader, FSTD_StrConst module) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_contains_module(loader, module);
}

fstd_func_impl bool fstd_module_loader_contains_symbol(FSTD_ModuleLoader *loader, FSTD_ModuleSymbol symbol) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_contains_symbol(loader, symbol);
}

fstd_func_impl bool fstd_module_loader_poll_module(FSTD_ModuleLoader *loader, FSTD_TaskWaker waker,
                                                   FSTD_StrConst module, FSTD_ModuleLoaderPollModuleResult *result) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_poll_module(loader, waker, module, result);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_module_loader_add_module(FSTD_ModuleLoader *loader,
                                                                        FSTD_ModuleInstance *owner,
                                                                        const FSTD_ModuleExport *module) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_add_module(loader, owner, module);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_module_loader_add_modules_from_path(FSTD_ModuleLoader *loader,
                                                                                   FSTD_Path path, void *filter_data,
                                                                                   FSTD_ModuleLoaderFilter filter) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_add_modules_from_path(loader, path, filter_data, filter);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_module_loader_add_modules_from_iter(FSTD_ModuleLoader *loader,
                                                                                   void *filter_data,
                                                                                   FSTD_ModuleLoaderFilter filter) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_add_modules_from_iter(loader, filter_data, filter, fstd__module_export_iter,
                                                           (const void *)fstd__module_export_iter);
}

FSTD_CHECK_USE fstd_func_impl FSTD_ModuleLoaderCommitResult fstd_module_loader_commit(FSTD_ModuleLoader *loader) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.loader_commit(loader);
}

fstd_func_impl FSTD_ModulesProfile fstd_modules_profile(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.profile();
}

fstd_func_impl FSTD_ModulesFeatureStatuses fstd_modules_features(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.features();
}

fstd_func_impl bool fstd_modules_namespace_exists(FSTD_StrConst ns) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.namespace_exists(ns);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_modules_prune_instances(void) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.prune_instances();
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_modules_query_parameter(FSTD_StrConst module, FSTD_StrConst parameter,
                                                                       FSTD_ModuleParamInfo *info) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.query_parameter(module, parameter, info);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_modules_read_parameter_opaque(FSTD_ModuleParamTag tag,
                                                                             FSTD_StrConst module,
                                                                             FSTD_StrConst parameter, void *value) {
    FSTD_Ctx *handle = fstd_ctx_get();
    return handle->modules_v0.read_parameter(tag, module, parameter, value);
}

FSTD_CHECK_USE fstd_func_impl FSTD_Status fstd_modules_write_parameter_opaque(FSTD_ModuleParamTag tag,
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

#endif // FIMO_STD_HEADER

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
