#ifndef FIMO_IMPL_MACROS_CONCAT_H
#define FIMO_IMPL_MACROS_CONCAT_H

#define FIMO_CONCAT_(a, b) a##b

/**
 * Concatenates the two identifiers.
 *
 * @param a first identifier
 * @param b second identifier
 */
#define FIMO_CONCAT(a, b) FIMO_CONCAT_(a, b)

#endif // !FIMO_IMPL_MACROS_CONCAT_H
