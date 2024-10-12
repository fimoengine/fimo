#include <fimo_std/context.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/memory.h>
#include <fimo_std/vtable.h>

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_context_init(const FimoBaseStructIn **options, FimoContext *context) {
    return fimo_internal_context_init(options, context);
}
