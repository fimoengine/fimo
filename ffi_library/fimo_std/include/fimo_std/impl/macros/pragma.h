#ifndef FIMO_IMPL_MACROS_PRAGMA_H
#define FIMO_IMPL_MACROS_PRAGMA_H

/**
 * Wrapper over the `_Pragma` operator.
 *
 * @param X arguments to pass to the operator
 */
#define FIMO_PRAGMA(X) _Pragma(#X)

/**
 * Wrapper over the `FIMO_PRAGMA` macro, that is only enabled,
 * if compiled with the GNU language dialect.
 *
 * @param X arguments to pass to the macro
 */
#ifdef __GNUC__
#define FIMO_PRAGMA_GCC(X) FIMO_PRAGMA(X)
#else
#define FIMO_PRAGMA_GCC(X)
#endif

/**
 * Wrapper over the `FIMO_PRAGMA` macro, that is only enabled,
 * if compiled with the Microsoft Visual C Compiler.
 *
 * @param X arguments to pass to the macro
 */
#ifdef _MSC_VER
#define FIMO_PRAGMA_MSVC(X) FIMO_PRAGMA(X)
#else
#define FIMO_PRAGMA_MSVC(X)
#endif

#endif // !FIMO_IMPL_MACROS_PRAGMA_H
