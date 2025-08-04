#ifndef FIMO_IMPL_TRACING_H
#define FIMO_IMPL_TRACING_H

#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Argument type for the standard formatter.
typedef struct FimoImplTracingFmtArgs {
    /// `vprintf` format string.
    const char *format;
    /// `vprintf` argument list.
    va_list *vlist;
} FimoImplTracingFmtArgs;

/// Standard formatter.
///
/// This functions acts like a call to `vsnprintf`, where the format string and arguments are
/// stored in `args`. The number of written bytes is returned. `args` must point to an instance of
/// a `FimoInternalTracingFmtArgs`.
static inline FimoUSize fimo_impl_tracing_fmt(char *buffer, FimoUSize buffer_size, const void *args) {
    FIMO_PRAGMA_MSVC(warning(push))
    FIMO_PRAGMA_MSVC(warning(disable : 4996))
    FimoImplTracingFmtArgs *tracing_args = (FimoImplTracingFmtArgs *)args;
    return vsnprintf(buffer, buffer_size, tracing_args->format, *tracing_args->vlist);
    FIMO_PRAGMA_MSVC(warning(pop))
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_IMPL_TRACING_H
