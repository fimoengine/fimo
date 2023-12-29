#include <fimo_std/internal/context.h>

#include <stdlib.h>

#include <fimo_std/memory.h>

static const FimoInternalContextVTable FIMO_INTERNAL_CONTEXT_VTABLE = {
    .check_version = fimo_internal_context_check_version,
    .destroy = fimo_internal_context_destroy,
    .dealloc = fimo_internal_context_dealloc,
};

static FimoVersion FIMO_IMPLEMENTED_VERSION
    = FIMO_VERSION_LONG(FIMO_VERSION_MAJOR,
        FIMO_VERSION_MINOR,
        FIMO_VERSION_PATCH,
        FIMO_VERSION_BUILD_NUMBER);

FIMO_MUST_USE
FimoError fimo_internal_context_init(const FimoBaseStructIn* options,
    FimoContext* context)
{
    if (!context) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoInternalContext* ctx = fimo_aligned_alloc(_Alignof(FimoInternalContext),
        sizeof(FimoInternalContext), &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    ctx->ref_count = (FimoAtomicRefCount)FIMO_REFCOUNT_INIT;

    *context = (FimoContext) { .data = ctx, .vtable = &FIMO_INTERNAL_CONTEXT_VTABLE };
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_context_destroy(void* ptr)
{
    FimoInternalContext* context = (FimoInternalContext*)ptr;
    if (!context || fimo_strong_count_atomic(&context->ref_count) != 0) {
        return FIMO_EINVAL;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_context_dealloc(void* ptr)
{
    FimoInternalContext* context = (FimoInternalContext*)ptr;
    if (!context || fimo_weak_count_atomic_unguarded(&context->ref_count) != 0) {
        return FIMO_EINVAL;
    }

    fimo_free_aligned_sized(context, _Alignof(FimoInternalContext), sizeof(FimoInternalContext));
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_context_check_version(void* ptr,
    const FimoVersion* required)
{
    // Not strictly required, but we include it, in case we decide to embed
    // the version into the instance in the future.
    FimoInternalContext* context = (FimoInternalContext*)ptr;
    if (!context || !required) {
        return FIMO_EINVAL;
    }

    if (!fimo_version_compatible(&FIMO_IMPLEMENTED_VERSION, required)) {
        return FIMO_EINVAL;
    }

    return FIMO_EOK;
}
