#ifndef FIMO_IMPL_TRACING_H
#define FIMO_IMPL_TRACING_H

#include <stdio.h>

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
static
inline
FimoResult fimo_impl_tracing_fmt(char *buffer, FimoUSize buffer_size, const void *args, FimoUSize *written_size) {
    if (buffer == NULL || args == NULL || written_size == NULL) {
        return FIMO_EINVAL;
    }
    FIMO_PRAGMA_MSVC(warning(push))
    FIMO_PRAGMA_MSVC(warning(disable : 4996))
    FimoImplTracingFmtArgs *tracing_args = (FimoImplTracingFmtArgs *)args;
    int written = vsnprintf(buffer, buffer_size, tracing_args->format, *tracing_args->vlist);
    *written_size = (FimoUSize)written;
    FIMO_PRAGMA_MSVC(warning(pop))
    return FIMO_EOK;
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_IMPL_TRACING_H
