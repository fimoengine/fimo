#ifndef FIMO_IMPL_MACROS_PRINT_FORMAT_H
#define FIMO_IMPL_MACROS_PRINT_FORMAT_H

#include <fimo_std/impl/macros/msvc_sal.h>

/**
 * Marks a parameter as a printf format string.
 */
#if defined(__GNUC__)
#define FIMO_PRINT_F_FORMAT /**/
#elif defined(_MSC_VER)
#define FIMO_PRINT_F_FORMAT _Printf_format_string_
#endif

/**
 * Marks that a function accepts a printf format string and
 * corresponding arguments or a `va_list` that contains these
 * arguments.
 *
 * @param format_param index of the format string parameter
 * @param dots_param index of the arguments parameter
 */
#if defined(__GNUC__)
#define FIMO_PRINT_F_FORMAT_ATTR(format_param, dots_param)                                                             \
    __attribute__((__format__(__printf__, format_param, dots_param)))
#elif defined(_MSC_VER)
#define FIMO_PRINT_F_FORMAT_ATTR(format_param, dots_param) /**/
#endif

#endif // !FIMO_IMPL_MACROS_PRINT_FORMAT_H
