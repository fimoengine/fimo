#ifndef FIMO_IMPL_MACROS_HAS_BUILTIN_H
#define FIMO_IMPL_MACROS_HAS_BUILTIN_H

/**
 * Checks whether the compiler defines a builtin.
 *
 * @param x builtin
 */
#if defined(__has_builtin)
#define FIMO_HAS_BUILTIN(x) __has_builtin(x)
#else
#define FIMO_HAS_BUILTIN(x) FALSE
#endif

#endif // !FIMO_IMPL_MACROS_HAS_BUILTIN_H