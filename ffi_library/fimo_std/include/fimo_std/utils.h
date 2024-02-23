#ifndef FIMO_UTILS_H
#define FIMO_UTILS_H

#include <fimo_std/impl/macros/has_builtin.h>
#include <fimo_std/impl/macros/inline.h>

#include <fimo_std/integers.h>

// Include the sal header on windows.
#ifdef _MSC_VER
#ifdef _USE_ATTRIBUTES_FOR_SAL
#undef _USE_ATTRIBUTES_FOR_SAL
#endif
#define _USE_ATTRIBUTES_FOR_SAL 1
#include <sal.h>
#endif

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
#define FIMO_PRINT_F_FORMAT_ATTR(format_param, dots_param) \
    __attribute__((__format__(__printf__, format_param, dots_param)))
#elif defined(_MSC_VER)
#define FIMO_PRINT_F_FORMAT_ATTR(format_param, dots_param) /**/
#endif

/**
 * Adds a `must_use` attribute to a function return value.
 */
#if defined(__GNUC__) && (__GNUC__ >= 4)
#define FIMO_MUST_USE __attribute__((warn_unused_result))
#elif defined(_MSC_VER) && (_MSC_VER >= 1700)
#define FIMO_MUST_USE _Check_return_
#else
#define FIMO_MUST_USE
#endif

/**
 * Stringifies the identifier, for internal use.
 *
 * @param a identifier
 */
#define FIMO_STRINGIFY_(a) #a

/**
 * Stringifies the identifier.
 *
 * @param a identifier
 */
#define FIMO_STRINGIFY(a) FIMO_STRINGIFY_(a)

/**
 * Concatenates the two identifiers, for internal use.
 *
 * @param a first identifier
 * @param b second identifier
 */
#define FIMO_CONCAT_(a, b) a##b

/**
 * Concatenates the two identifiers.
 *
 * @param a first identifier
 * @param b second identifier
 */
#define FIMO_CONCAT(a, b) FIMO_CONCAT_(a, b)

/**
 * Internal macro for generating unique identifiers.
 */
#ifdef __COUNTER__
#define FIMO_UNIQUE_() __COUNTER__
#else
#define FIMO_UNIQUE_() __LINE__
#endif // __COUNTER__

/**
 * Generates an unique identifier with the given prefix.
 *
 * @param name prefix
 */
#define FIMO_VAR(name) FIMO_CONCAT(name, FIMO_UNIQUE_())

/**
 * Write a pointer to the structure containing the member into a variable,
 * for internal use.
 *
 * @param ptr pointer to the member
 * @param type type of the structure
 * @param member name of the member within the struct
 * @param out identifier of the result
 * @param tmp identifier for the tmp variable
 */
#define FIMO_CONTAINER_OF_(ptr, type, member, out, tmp) \
    do {                                                \
        char* tmp = (char*)(ptr);                       \
        out = ((type*)(tmp - offsetof(type, member)));  \
    } while (0)

/**
 * Write a pointer to the structure containing the member into a variable.
 *
 * @param ptr pointer to the member
 * @param type type of the structure
 * @param member name of the member within the struct
 * @param out identifier of the result
 */
#define FIMO_CONTAINER_OF(ptr, type, member, out) FIMO_CONTAINER_OF_(ptr, type, member, out, FIMO_VAR(_tmp_ptr))

#endif // FIMO_UTILS_H
