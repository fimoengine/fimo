#include <fimo_std/internal/context.h>

#include <stdlib.h>

#include <fimo_std/memory.h>

static const FimoInternalContextVTable FIMO_INTERNAL_CONTEXT_VTABLE = {
    .check_version = fimo_internal_context_check_version,
    .acquire = fimo_internal_context_acquire,
    .release = fimo_internal_context_release,
    .tracing_call_stack_create = fimo_internal_tracing_call_stack_create,
    .tracing_call_stack_destroy = fimo_internal_tracing_call_stack_destroy,
    .tracing_call_stack_switch = fimo_internal_tracing_call_stack_switch,
    .tracing_call_stack_unblock = fimo_internal_tracing_call_stack_unblock,
    .tracing_call_stack_suspend_current = fimo_internal_tracing_call_stack_suspend_current,
    .tracing_call_stack_resume_current = fimo_internal_tracing_call_stack_resume_current,
    .tracing_span_create = fimo_internal_tracing_span_create_custom,
    .tracing_span_destroy = fimo_internal_tracing_span_destroy,
    .tracing_event_emit = fimo_internal_tracing_event_emit_custom,
    .tracing_is_enabled = fimo_internal_tracing_is_enabled,
    .tracing_register_thread = fimo_internal_tracing_register_thread,
    .tracing_unregister_thread = fimo_internal_tracing_unregister_thread,
    .tracing_flush = fimo_internal_tracing_flush,
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

    // Parse the options. Each option type may occur at most once.
    const FimoTracingCreationConfig* tracing_config = NULL;
    for (const FimoBaseStructIn* opt = options; opt != NULL; opt = opt->next) {
        switch (opt->type) {
        case FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG:
            if (tracing_config) {
                return FIMO_EINVAL;
            }
            tracing_config = (const FimoTracingCreationConfig*)opt;
            break;
        default:
            return FIMO_EINVAL;
        }
    }

    FimoError error = FIMO_EOK;
    FimoInternalContext* ctx = fimo_aligned_alloc(_Alignof(FimoInternalContext),
        sizeof(FimoInternalContext), &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    ctx->ref_count = (FimoAtomicRefCount)FIMO_REFCOUNT_INIT;

    error = fimo_internal_tracing_init(ctx, tracing_config);
    if (FIMO_IS_ERROR(error)) {
        goto cleanup;
    }

    *context = (FimoContext) { .data = ctx, .vtable = &FIMO_INTERNAL_CONTEXT_VTABLE };
    return FIMO_EOK;

cleanup:
    fimo_free_aligned_sized(ctx, _Alignof(FimoInternalContext),
        sizeof(FimoInternalContext));
    return error;
}

FIMO_MUST_USE
{
        return FIMO_EINVAL;
    }

    return FIMO_EOK;
}

void fimo_internal_context_acquire(void* ptr)
{
    FIMO_ASSERT(ptr)
    FimoInternalContext* context = (FimoInternalContext*)ptr;
    fimo_increase_strong_count_atomic(&context->ref_count);
}

void fimo_internal_context_release(void* ptr)
{
    FIMO_ASSERT(ptr)
    FimoInternalContext* context = (FimoInternalContext*)ptr;
    bool can_destroy = fimo_decrease_strong_count_atomic(&context->ref_count);
    if (!can_destroy) {
        return;
    }

    // Destroy all submodules.
    fimo_internal_tracing_destroy(context);

    // Finally deallocate the context.
    fimo_free_aligned_sized(context, _Alignof(FimoInternalContext),
        sizeof(FimoInternalContext));
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
