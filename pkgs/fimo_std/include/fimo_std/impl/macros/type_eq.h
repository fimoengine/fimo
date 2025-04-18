#ifndef FIMO_IMPL_MACROS_TYPE_EQ_H
#define FIMO_IMPL_MACROS_TYPE_EQ_H

#ifdef __cplusplus
#include <type_traits>

#define FIMO_TYPE_EQ_HELPER_(EXPR, T) std::is_same_v<decltype(EXPR), T>
#else
#include <assert.h>
#define FIMO_TYPE_EQ_HELPER_(EXPR, T) _Generic((EXPR), T: 1, default: 0)
#endif

/**
 * Returns whether the type of an expression equals some specific type.
 *
 * @param EXPR expression to check the type of
 * @param T type to compare to
 */
#define FIMO_TYPE_EQ(EXPR, T) FIMO_TYPE_EQ_HELPER_(EXPR, T)

/**
 * Asserts at compile time, that the type of an expression equals some specific type.
 *
 * @param EXPR expression to check the type of
 * @param T type to compare to
 */
#define FIMO_ASSERT_TYPE_EQ(EXPR, T) static_assert(FIMO_TYPE_EQ(EXPR, T), "Type mismatch: typeof(" #EXPR ") == " #T);

#endif // !FIMO_IMPL_MACROS_TYPE_EQ_H
