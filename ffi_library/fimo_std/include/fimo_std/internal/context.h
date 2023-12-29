#ifndef FIMO_INTERNAL_CONTEXT_H
#define FIMO_INTERNAL_CONTEXT_H

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/refcount.h>
#include <fimo_std/version.h>

#include <fimo_std/internal/module.h>
#include <fimo_std/internal/tracing.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Internal representation of the context.
 *
 * The abi of the context is unstable, except fot the first member
 * being the atomic reference count. From outside of the internals
 * of fimo_std a `FimoInternalContext` may only be accessed through
 * the provided vtable.
 */
typedef struct FimoInternalContext {
    FimoAtomicRefCount ref_count;
    FimoInternalContextTracing tracing;
} FimoInternalContext;

/**
 * Internal vtable of a context.
 *
 * The abi of this type is semi-stable, where given two compatible
 * versions `v1` and `v2` with `v1 <= v2`, a pointer to the vtable
 * in `v2`, i.e., `FimoInternalContextVTable_v2*` can be cast to a
 * pointer to the vtable in version `v1`, or
 * `FimoInternalContextVTable_v1*`. To that end, we are allowed to
 * add new fields to this struct and restricting the alignment.
 * Further, to detect a version mismatch, we require that the member
 * `check_version`, is always defined as the first member.
 */
typedef struct FimoInternalContextVTable {
    FimoError (*check_version)(void*, const FimoVersion*);
    FimoError (*destroy)(void*);
    FimoError (*dealloc)(void*);
    FimoError (*tracing_call_stack_create)(void*, FimoTracingCallStack*);
    FimoError (*tracing_call_stack_destroy)(void*, FimoTracingCallStack);
    FimoError (*tracing_call_stack_switch)(void*, FimoTracingCallStack,
        FimoTracingCallStack*);
    FimoError (*tracing_call_stack_unblock)(void*, FimoTracingCallStack);
    FimoError (*tracing_call_stack_suspend_current)(void*, bool);
    FimoError (*tracing_call_stack_resume_current)(void*);
    FimoError (*tracing_span_create)(void*, const FimoTracingSpanDesc*,
        FimoTracingSpan*, FimoTracingFormat, const void*);
    FimoError (*tracing_span_destroy)(void*, FimoTracingSpan*);
    FimoError (*tracing_event_emit)(void*, const FimoTracingEvent*,
        FimoTracingFormat, const void*);
    bool (*tracing_is_enabled)(void*);
    FimoError (*tracing_register_thread)(void*);
    FimoError (*tracing_unregister_thread)(void*);
    FimoError (*tracing_flush)(void*);
    FimoError (*module_lock)(void*);
    FimoError (*module_unlock)(void*);
    FimoError (*module_pseudo_module_new)(void*, const FimoModule**);
    FimoError (*module_pseudo_module_destroy)(void*, const FimoModule*);
    FimoError (*module_set_new)(void*, FimoModuleLoadingSet*);
    FimoError (*module_set_has_module)(void*, FimoModuleLoadingSet,
        const char*, bool*);
    FimoError (*module_set_has_symbol)(void*, FimoModuleLoadingSet,
        const char*, const char*, FimoVersion, bool*);
    FimoError (*module_set_append)(void*, FimoModuleLoadingSet,
        const char*, FimoModuleLoadingFilter, void*,
        FimoModuleLoadingSuccessCallback, FimoModuleLoadingErrorCallback,
        FimoModuleLoadingCleanupCallback, void*,
        void (*)(bool (*)(const FimoModuleExport*, void*), void*));
    FimoError (*module_set_dismiss)(void*, FimoModuleLoadingSet);
    FimoError (*module_set_finish)(void*, FimoModuleLoadingSet);
    FimoError (*module_find_by_name)(void*, const char*,
        const FimoModuleInfo**);
    FimoError (*module_find_by_symbol)(void*, const char*, const char*,
        FimoVersion, const FimoModuleInfo**);
    FimoError (*module_namespace_exists)(void*, const char*, bool*);
    FimoError (*module_namespace_include)(void*, const FimoModule*,
        const char*);
    FimoError (*module_namespace_exclude)(void*, const FimoModule*,
        const char*);
    FimoError (*module_namespace_included)(void*, const FimoModule*,
        const char*, bool*, bool*);
    FimoError (*module_acquire_dependency)(void*, const FimoModule*,
        const FimoModuleInfo*);
    FimoError (*module_relinquish_dependency)(void*, const FimoModule*,
        const FimoModuleInfo*);
    FimoError (*module_has_dependency)(void*, const FimoModule*,
        const FimoModuleInfo*, bool*, bool*);
    FimoError (*module_load_symbol)(void*, const FimoModule*, const char*,
        const char*, FimoVersion, const void**);
    FimoError (*module_unload)(void*, const FimoModuleInfo*);
    FimoError (*module_param_query)(void*, const char*, const char*,
        FimoModuleParamType*, FimoModuleParamAccess*, FimoModuleParamAccess*);
    FimoError (*module_param_set_public)(void*, const void*, const char*,
        const char*);
    FimoError (*module_param_get_public)(void*, void*, const char*,
        const char*);
    FimoError (*module_param_set_dependency)(void*, const FimoModule*,
        const void*, const char*, const char*);
    FimoError (*module_param_get_dependency)(void*, const FimoModule*,
        void*, const char*, const char*);
    FimoError (*module_param_set_private)(void*, const FimoModule*,
        const void*, FimoModuleParam*);
    FimoError (*module_param_get_private)(void*, const FimoModule*,
        void*, const FimoModuleParam*);
} FimoInternalContextVTable;

/**
 * Initializes a new context.
 *
 * If `options` is `NULL`, the context is initialized with the default options.
 * A pointer to the initialized context is written to `context`.
 *
 * @param options init options
 * @param context pointer to the context (not `NULL`)
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_context_init(const FimoBaseStructIn* options,
    FimoContext* context);

/**
 * Destroys the context.
 *
 * @param ptr pointer to the context
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_context_destroy(void* ptr);

/**
 * Deallocates the context.
 *
 * @param ptr pointer to the context
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_context_dealloc(void* ptr);

/**
 * Checks that the implemented context version is compatible with `version`.
 *
 * @param ptr pointer to the context
 * @param required required version
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_context_check_version(void* ptr,
    const FimoVersion* required);

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_INTERNAL_CONTEXT_H
