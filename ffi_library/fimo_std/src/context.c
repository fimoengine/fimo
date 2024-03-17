#include <fimo_std/context.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/memory.h>

static const FimoVersion FIMO_REQUIRED_VERSION
    = FIMO_VERSION_LONG(FIMO_VERSION_MAJOR,
        FIMO_VERSION_MINOR,
        FIMO_VERSION_PATCH,
        FIMO_VERSION_BUILD_NUMBER);

struct FimoInternalContextMinCompatVTable {
    FimoError (*check_version)(void*, const FimoVersion*);
};

FIMO_MUST_USE
FimoError fimo_context_init(const FimoBaseStructIn* options,
    FimoContext* context)
{
    return fimo_internal_context_init(options, context);
}

FIMO_MUST_USE
FimoError fimo_context_check_version(FimoContext context)
{
    const struct FimoInternalContextMinCompatVTable* vtable = context.vtable;
    return vtable->check_version(context.data, &FIMO_REQUIRED_VERSION);
}

void fimo_context_acquire(FimoContext context)
{
    const struct FimoInternalContextVTable* vtable = context.vtable;
    vtable->acquire(context.data);
}

void fimo_context_release(FimoContext context)
{
    const struct FimoInternalContextVTable* vtable = context.vtable;
    vtable->release(context.data);
}