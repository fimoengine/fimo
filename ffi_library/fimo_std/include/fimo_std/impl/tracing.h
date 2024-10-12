#ifndef FIMO_IMPL_TRACING_H
#define FIMO_IMPL_TRACING_H

#include <fimo_std/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Argument type for the standard formatter.
 */
typedef struct FimoImplTracingFmtArgs {
    /// `vprintf` format string.
    const char *format;
    /// `vprintf` argument list.
    va_list *vlist;
} FimoImplTracingFmtArgs;

/**
 * Standard formatter.
 *
 * This functions acts like a call to `vsnprintf`, where the format string
 * and arguments are stored in `args`. The number of written bytes is
 * written into `written_bytes`. `args` must point to an instance of a
 * `FimoInternalTracingFmtArgs`.
 *
 * @param buffer destination buffer
 * @param buffer_size size of the buffer
 * @param args formatting args
 * @param written_size pointer to the count of written bytes
 *
 * @return Status code.
 */
FimoResult fimo_impl_tracing_fmt(char *buffer, FimoUSize buffer_size, const void *args, FimoUSize *written_size);

#ifdef __cplusplus
}
#endif

#endif // FIMO_IMPL_TRACING_H
