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
 * The abi of the context is unstable. From outside of the internals
 * of fimo_std a `FimoInternalContext` may only be accessed through
 * the provided vtable.
 */
typedef struct FimoInternalContext {
    FimoAtomicRefCount ref_count;
    FimoInternalTracingContext tracing;
    FimoInternalModuleContext module;
} FimoInternalContext;

/**
 * Initializes a new context with the given options.
 *
 * If `options` is `NULL`, the context is initialized with the default options,
 * otherwise `options` must be an array terminated with a `NULL` element. The
 * initialized context is written to `context`. In case of an error, this function
 * cleans up the configuration options.
 *
 * @param options init options
 * @param context pointer to the context (not `NULL`)
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_context_init(const FimoBaseStructIn **options, FimoContext *context);

/**
 * Returns the public context for the internal context.
 *
 * This function does not modify the reference count.
 *
 * @param ptr pointer to the context.
 * @param context public context.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_context_to_public_ctx(void *ptr, FimoContext *context);

/**
 * Acquires a reference to the context by increasing the reference count.
 *
 * @param ptr pointer to the context.
 */
void fimo_internal_context_acquire(void *ptr);

/**
 * Releases a reference to the context by decreasing the reference count.
 *
 * @param ptr pointer to the context.
 */
void fimo_internal_context_release(void *ptr);

/**
 * Checks that the implemented context version is compatible with `version`.
 *
 * @param ptr pointer to the context
 * @param required required version
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_context_check_version(void *ptr, const FimoVersion *required);

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_INTERNAL_CONTEXT_H
