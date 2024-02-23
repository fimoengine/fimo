#ifndef FIMO_IMPL_MACROS_INLINE_H
#define FIMO_IMPL_MACROS_INLINE_H

/**
 * Requests that a function be always inlined.
 */
#if defined(__GNUC__)
#define FIMO_INLINE_ALWAYS inline __attribute__((always_inline))
#elif defined(_MSC_VER)
#define FIMO_INLINE_ALWAYS __forceinline
#else
#define FIMO_INLINE_ALWAYS inline
#endif

#endif // !FIMO_IMPL_MACROS_INLINE_H