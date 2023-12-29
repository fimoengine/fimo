#ifndef FIMO_CONTEXT_H
#define FIMO_CONTEXT_H

#include <fimo_std/error.h>
#include <fimo_std/refcount.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Context of the fimo std.
 *
 * The context is an opaque object that can only be accessed through
 * the provided vtable, which is also opaque. The data pointer can be
 * cast to a pointer to an `FimoAtomicRefCount`.
 */
typedef struct FimoContext {
    void* data;
    const void* vtable;
} FimoContext;

/**
 * Returns a pointer to the atomic reference counter from a context.
 */
#define FIMO_CONTEXT_REF_COUNT(context) (FimoAtomicRefCount*)((context).data)

/**
 * Fimo std structure types.
 */
typedef enum FimoStructType {
    FIMO_STRUCT_TYPE_FORCE32 = 0x7FFFFFFF
} FimoStructType;

/**
 * Base structure for a read-only pointer chain.
 */
typedef struct FimoBaseStructIn {
    FimoStructType type;
    const struct FimoBaseStructIn* next;
} FimoBaseStructIn;

/**
 * Base structure for a pointer chain.
 */
typedef struct FimoBaseStructOut {
    FimoStructType type;
    struct FimoBaseStructOut* next;
} FimoBaseStructOut;

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
FimoError fimo_context_init(const FimoBaseStructIn* options,
    FimoContext* context);

/**
 * Destroys the context.
 *
 * This function destroys the `FimoContext`, without deallocating it.
 * This function must be called when the strong reference count of the
 * `FimoContext` instance reaches `0`.
 *
 * @param context the context
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_context_destroy_strong(FimoContext context);

/**
 * Deallocates a context.
 *
 * This function deallocates the `FimoContext`, without destroying it.
 * This function must be called when the weak reference count of the
 * `FimoContext` instance reaches `0`.
 *
 * @param context the context
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_context_destroy_weak(FimoContext context);

/**
 * Checks the compatibility of the context version.
 *
 * This function must be called upon the acquisition of a context, that
 * was not created locally, e.g., when being passed a context from
 * another shared library. Failure of doing so, may cause undefined
 * behavior, if the context is later utilized.
 *
 * @param context the context
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_context_check_version(FimoContext context);

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_CONTEXT_H
