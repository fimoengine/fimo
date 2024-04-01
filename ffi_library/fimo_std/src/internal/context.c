#include <fimo_std/internal/context.h>

#include <stdlib.h>

#include <fimo_std/memory.h>
#include <fimo_std/vtable.h>

static const FimoContextVTable FIMO_INTERNAL_CONTEXT_VTABLE = {
        .header =
                {
                        .check_version = fimo_internal_context_check_version,
                },
        .core =
                {
                        .acquire = fimo_internal_context_acquire,
                        .release = fimo_internal_context_release,
                },
        .tracing_v0 =
                {
                        .call_stack_create = fimo_internal_tracing_call_stack_create,
                        .call_stack_destroy = fimo_internal_tracing_call_stack_destroy,
                        .call_stack_switch = fimo_internal_tracing_call_stack_switch,
                        .call_stack_unblock = fimo_internal_tracing_call_stack_unblock,
                        .call_stack_suspend_current = fimo_internal_tracing_call_stack_suspend_current,
                        .call_stack_resume_current = fimo_internal_tracing_call_stack_resume_current,
                        .span_create = fimo_internal_tracing_span_create_custom,
                        .span_destroy = fimo_internal_tracing_span_destroy,
                        .event_emit = fimo_internal_tracing_event_emit_custom,
                        .is_enabled = fimo_internal_tracing_is_enabled,
                        .register_thread = fimo_internal_tracing_register_thread,
                        .unregister_thread = fimo_internal_tracing_unregister_thread,
                        .flush = fimo_internal_tracing_flush,
                },
        .module_v0 =
                {
                        .pseudo_module_new = fimo_internal_trampoline_module_pseudo_module_new,
                        .pseudo_module_destroy = fimo_internal_trampoline_module_pseudo_module_destroy,
                        .set_new = fimo_internal_trampoline_module_set_new,
                        .set_has_module = fimo_internal_trampoline_module_set_has_module,
                        .set_has_symbol = fimo_internal_trampoline_module_set_has_symbol,
                        .set_append_callback = fimo_internal_trampoline_module_set_append_callback,
                        .set_append_modules = fimo_internal_trampoline_module_set_append_modules,
                        .set_dismiss = fimo_internal_trampoline_module_set_dismiss,
                        .set_finish = fimo_internal_trampoline_module_set_finish,
                        .find_by_name = fimo_internal_trampoline_module_find_by_name,
                        .find_by_symbol = fimo_internal_trampoline_module_find_by_symbol,
                        .namespace_exists = fimo_internal_trampoline_module_namespace_exists,
                        .namespace_include = fimo_internal_trampoline_module_namespace_include,
                        .namespace_exclude = fimo_internal_trampoline_module_namespace_exclude,
                        .namespace_included = fimo_internal_trampoline_module_namespace_included,
                        .acquire_dependency = fimo_internal_trampoline_module_acquire_dependency,
                        .relinquish_dependency = fimo_internal_trampoline_module_relinquish_dependency,
                        .has_dependency = fimo_internal_trampoline_module_has_dependency,
                        .load_symbol = fimo_internal_trampoline_module_load_symbol,
                        .unload = fimo_internal_trampoline_module_unload,
                        .param_query = fimo_internal_trampoline_module_param_query,
                        .param_set_public = fimo_internal_trampoline_module_param_set_public,
                        .param_get_public = fimo_internal_trampoline_module_param_get_public,
                        .param_set_dependency = fimo_internal_trampoline_module_param_set_dependency,
                        .param_get_dependency = fimo_internal_trampoline_module_param_get_dependency,
                        .param_set_private = fimo_internal_trampoline_module_param_set_private,
                        .param_get_private = fimo_internal_trampoline_module_param_get_private,
                        .param_set_inner = fimo_internal_trampoline_module_param_set_inner,
                        .param_get_inner = fimo_internal_trampoline_module_get_inner,
                },
};

static FimoVersion FIMO_IMPLEMENTED_VERSION =
        FIMO_VERSION_LONG(FIMO_VERSION_MAJOR, FIMO_VERSION_MINOR, FIMO_VERSION_PATCH, FIMO_VERSION_BUILD_NUMBER);

FIMO_MUST_USE
FimoError fimo_internal_context_init(const FimoBaseStructIn *options, FimoContext *context) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    // Parse the options. Each option type may occur at most once.
    const FimoTracingCreationConfig *tracing_config = NULL;
    for (const FimoBaseStructIn *opt = options; opt != NULL; opt = opt->next) {
        switch (opt->type) {
            case FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG:
                if (tracing_config) {
                    return FIMO_EINVAL;
                }
                tracing_config = (const FimoTracingCreationConfig *)opt;
                break;
            default:
                return FIMO_EINVAL;
        }
    }

    FimoError error = FIMO_EOK;
    FimoInternalContext *ctx = fimo_aligned_alloc(_Alignof(FimoInternalContext), sizeof(FimoInternalContext), &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    ctx->ref_count = (FimoAtomicRefCount)FIMO_REFCOUNT_INIT;

    error = fimo_internal_tracing_init(ctx, tracing_config);
    if (FIMO_IS_ERROR(error)) {
        goto cleanup;
    }

    error = fimo_internal_module_init(&ctx->module);
    if (FIMO_IS_ERROR(error)) {
        goto deinit_tracing;
    }

    *context = (FimoContext){.data = ctx, .vtable = &FIMO_INTERNAL_CONTEXT_VTABLE};
    return FIMO_EOK;

deinit_tracing:
    fimo_internal_tracing_destroy(ctx);
cleanup:
    fimo_free_aligned_sized(ctx, _Alignof(FimoInternalContext), sizeof(FimoInternalContext));
    return error;
}

FIMO_MUST_USE
FimoError fimo_internal_context_to_public_ctx(void *ptr, FimoContext *context) {
    if (ptr == NULL || context == NULL) {
        return FIMO_EINVAL;
    }

    *context = (FimoContext){.data = ptr, .vtable = &FIMO_INTERNAL_CONTEXT_VTABLE};
    return FIMO_EOK;
}

void fimo_internal_context_acquire(void *ptr) {
    FIMO_ASSERT(ptr)
    FimoInternalContext *context = ptr;
    fimo_increase_strong_count_atomic(&context->ref_count);
}

void fimo_internal_context_release(void *ptr) {
    FIMO_ASSERT(ptr)
    FimoInternalContext *context = ptr;
    const bool can_destroy = fimo_decrease_strong_count_atomic(&context->ref_count);
    if (!can_destroy) {
        return;
    }

    // Destroy all submodules.
    fimo_internal_module_destroy(&context->module);
    fimo_internal_tracing_destroy(context);

    // Finally deallocate the context.
    fimo_free_aligned_sized(context, _Alignof(FimoInternalContext), sizeof(FimoInternalContext));
}

FIMO_MUST_USE
FimoError fimo_internal_context_check_version(void *ptr, const FimoVersion *required) {
    // Not strictly required, but we include it, in case we decide to embed
    // the version into the instance in the future.
    FimoInternalContext *context = ptr;
    if (context == NULL || required == NULL) {
        return FIMO_EINVAL;
    }

    if (!fimo_version_compatible(&FIMO_IMPLEMENTED_VERSION, required)) {
        return FIMO_EINVAL;
    }

    return FIMO_EOK;
}
