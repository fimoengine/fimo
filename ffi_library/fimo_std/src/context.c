#include <fimo_std/context.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/memory.h>
#include <fimo_std/vtable.h>

static const FimoVersion FIMO_REQUIRED_VERSION =
        FIMO_VERSION_LONG(FIMO_VERSION_MAJOR, FIMO_VERSION_MINOR, FIMO_VERSION_PATCH, FIMO_VERSION_BUILD_NUMBER);

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_context_init(const FimoBaseStructIn **options, FimoContext *context) {
    return fimo_internal_context_init(options, context);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_context_check_version(const FimoContext context) {
    const FimoContextVTableHeader *vtable = context.vtable;
    return vtable->check_version(context.data, &FIMO_REQUIRED_VERSION);
}

FIMO_EXPORT
void fimo_context_acquire(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    vtable->core.acquire(context.data);
}

FIMO_EXPORT
void fimo_context_release(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    vtable->core.release(context.data);
}
