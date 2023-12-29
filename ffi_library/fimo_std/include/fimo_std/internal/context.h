#ifndef FIMO_INTERNAL_CONTEXT_H
#define FIMO_INTERNAL_CONTEXT_H

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/refcount.h>
#include <fimo_std/version.h>

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
