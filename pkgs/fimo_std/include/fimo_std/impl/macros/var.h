#ifndef FIMO_IMPL_MACROS_VAR_H
#define FIMO_IMPL_MACROS_VAR_H

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

#endif // !FIMO_IMPL_MACROS_VAR_H
