#include <fimo_std/impl/tracing.h>

#include <stdio.h>

FimoError fimo_impl_tracing_fmt(char *buffer, FimoUSize buffer_size, const void *args, FimoUSize *written_size) {
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
