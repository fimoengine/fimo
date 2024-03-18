#ifndef FIMO_IMPL_MACROS_ASSERT_H
#define FIMO_IMPL_MACROS_ASSERT_H

#include <stdio.h>
#include <stdlib.h>

#include <fimo_std/impl/macros/stringify.h>

/**
 * Asserts that the passed in `CONDITION` does not equal `0`.
 *
 * If the assertion is not true, the assert prints some info
 * to `stderr` and exits the program.
 */
#define FIMO_ASSERT(CONDITION)                                                                                         \
    if (!(CONDITION)) {                                                                                                \
        perror("assertion error in " __FILE__ ":" FIMO_STRINGIFY(__LINE__) ": FIMO_ASSERT(" #CONDITION ")");           \
        exit(EXIT_FAILURE);                                                                                            \
    }

/**
 * Asserts that the passed in `CONDITION` equals `0`.
 *
 * If the assertion is not true, the assert prints some info
 * to `stderr` and exits the program.
 */
#define FIMO_ASSERT_FALSE(CONDITION)                                                                                   \
    if ((CONDITION)) {                                                                                                 \
        perror("assertion error" __FILE__ ":" FIMO_STRINGIFY(__LINE__) ": FIMO_ASSERT_FALSE(" #CONDITION ")");         \
        exit(EXIT_FAILURE);                                                                                            \
    }

/**
 * Asserts that the passed in `CONDITION` does not equal `0`.
 *
 * If the assertion is not true, the assert prints some info
 * to `stderr` and exits the program.
 *
 * Unlike `FIMO_ASSERT`, `FIMO_DEBUG_ASSERT` is only checked, if
 * `NDEBUG` is defined.
 */
#ifdef NDEBUG
#define FIMO_DEBUG_ASSERT(CONDITION) ((void)0)
#else
#define FIMO_DEBUG_ASSERT(CONDITION)                                                                                   \
    if (!(CONDITION)) {                                                                                                \
        perror("assertion error in " __FILE__ ":" FIMO_STRINGIFY(__LINE__) ": FIMO_DEBUG_ASSERT(" #CONDITION ")");     \
        exit(EXIT_FAILURE);                                                                                            \
    }
#endif

/**
 * Asserts that the passed in `CONDITION` equals `0`.
 *
 * If the assertion is not true, the assert prints some info
 * to `stderr` and exits the program.
 *
 * Unlike `FIMO_ASSERT_FALSE`, `FIMO_DEBUG_ASSERT_FALSE` is
 * only checked, if `NDEBUG` is defined.
 */
#ifdef NDEBUG
#define FIMO_DEBUG_ASSERT_FALSE(CONDITION) ((void)0)
#else
#define FIMO_DEBUG_ASSERT_FALSE(CONDITION)                                                                             \
    if ((CONDITION)) {                                                                                                 \
        perror("assertion error" __FILE__ ":" FIMO_STRINGIFY(__LINE__) ": FIMO_DEBUG_ASSERT_FALSE(" #CONDITION ")");   \
        exit(EXIT_FAILURE);                                                                                            \
    }
#endif

#endif // !FIMO_IMPL_MACROS_ASSERT_H
