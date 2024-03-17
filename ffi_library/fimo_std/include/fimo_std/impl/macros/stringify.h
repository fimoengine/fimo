#ifndef FIMO_IMPL_MACROS_STRINGIFY_H
#define FIMO_IMPL_MACROS_STRINGIFY_H

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

#endif // !FIMO_IMPL_MACROS_STRINGIFY_H
