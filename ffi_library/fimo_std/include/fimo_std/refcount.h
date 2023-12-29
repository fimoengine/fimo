#ifndef FIMO_STD_REFCOUNT_H
#define FIMO_STD_REFCOUNT_H

#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * A strong and weak counter pair for reference counts.
 *
 * The counter aborts the program, if either the strong or the weak count
 * reaches INTPTR_MAX, to safeguard against overflow.
 */
typedef struct FimoRefCount {
    uintptr_t strong_refs;
    uintptr_t weak_refs;
} FimoRefCount;

/**
 * A strong and weak counter pair for atomic
 * reference counts.
 *
 * The counter aborts the program, if either the strong or the weak count
 * reaches INTPTR_MAX, to safeguard against overflow.
 */
typedef struct FimoAtomicRefCount {
    atomic_uintptr_t strong_refs;
    atomic_uintptr_t weak_refs;
} FimoAtomicRefCount;

#define FIMO_REFCOUNT_INIT                \
    {                                     \
        .strong_refs = 1, .weak_refs = 1, \
    }

#ifdef FIMO_MACRO_HELPER_FUNCTIONS
/**
 * Initializes a new refcount.
 *
 * @return The refcount
 */
FimoRefCount fimo_refcount_init(void);

/**
 * Initializes a new atomic refcount.
 *
 * @return The refcount.
 */
FimoAtomicRefCount fimo_refcount_atomic_init(void);
#endif // FIMO_MACRO_HELPER_FUNCTIONS

/**
 * Get the refcount's strong value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return The refcount's strong value.
 */
FIMO_MUST_USE
uintptr_t fimo_strong_count(const FimoRefCount* count);

/**
 * Get the atomic refcount's strong value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return The refcount's strong value.
 */
FIMO_MUST_USE
uintptr_t fimo_strong_count_atomic(const FimoAtomicRefCount* count);

/**
 * Get the refcount's weak value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return The refcount's weak value.
 */
FIMO_MUST_USE
uintptr_t fimo_weak_count(const FimoRefCount* count);

/**
 * Get the atomic refcount's weak value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * The function does not ensure, that the value of the strong count
 * is greater than zero.
 *
 * @return The refcount's weak value.
 */
FIMO_MUST_USE
uintptr_t fimo_weak_count_atomic_unguarded(const FimoAtomicRefCount* count);

/**
 * Get the atomic refcount's weak value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * The function ensures that there is at least one strong reference,
 * otherwise it returns `0`. The returned value may be off by one in
 * either direction.
 *
 * @return The refcount's weak value.
 */
FIMO_MUST_USE
uintptr_t fimo_weak_count_atomic_guarded(const FimoAtomicRefCount* count);

/**
 * Increase the refcount's strong value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * This function may abort the program, if the strong value is saturated.
 */
void fimo_increase_strong_count(FimoRefCount* count);

/**
 * Increase the atomic refcount's strong value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * This function may abort the program, if the strong value is saturated.
 */
void fimo_increase_strong_count_atomic(FimoAtomicRefCount* count);

/**
 * Increase the refcount's weak value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * This function may abort the program, if the weak value is saturated.
 */
void fimo_increase_weak_count(FimoRefCount* count);

/**
 * Increase the atomic refcount's weak value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * This function may abort the program, if the weak value is saturated.
 */
void fimo_increase_weak_count_atomic(FimoAtomicRefCount* count);

/**
 * Decreases the refcount's strong value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return Whether the data guarded by the refcount can be destroyed.
 */
FIMO_MUST_USE
bool fimo_decrease_strong_count(FimoRefCount* count);

/**
 * Decreases the atomic refcount's strong value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return Whether the data guarded by the refcount can be destroyed.
 */
FIMO_MUST_USE
bool fimo_decrease_strong_count_atomic(FimoAtomicRefCount* count);

/**
 * Decreases the refcount's weak value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return Whether the refcount can be destroyed.
 */
FIMO_MUST_USE
bool fimo_decrease_weak_count(FimoRefCount* count);

/**
 * Decreases the atomic refcount's weak value.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return Whether the refcount can be destroyed.
 */
FIMO_MUST_USE
bool fimo_decrease_weak_count_atomic(FimoAtomicRefCount* count);

/**
 * Upgrades a weak reference to a strong reference.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The operation was successful.
 * @error `FIMO_EINVAL`: The strong count reached `0`.
 * @error `FIMO_EOVERFLOW`: Strong count saturated.
 */
FIMO_MUST_USE
FimoError fimo_upgrade_refcount(FimoRefCount* count);

/**
 * Upgrades a weak reference to a strong reference.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The operation was successful.
 * @error `FIMO_EINVAL`: The strong count reached `0`.
 * @error `FIMO_EOVERFLOW`: Strong count saturated.
 */
FIMO_MUST_USE
FimoError fimo_upgrade_refcount_atomic(FimoAtomicRefCount* count);

/**
 * Downgrades a strong reference to a weak reference.
 *
 * @param count: the refcount (not `NULL`)
 *
 * This function does not decrease the strong count, therefore it creates a new
 * weak reference.
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The operation was successful.
 * @error `FIMO_EOVERFLOW`: Weak count saturated.
 */
FIMO_MUST_USE
FimoError fimo_downgrade_refcount(FimoRefCount* count);

/**
 * Downgrades a strong reference to a weak reference.
 *
 * @param count: the refcount (not `NULL`)
 *
 * This function does not decrease the strong count, therefore it creates a new
 * weak reference.
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The operation was successful.
 * @error `FIMO_EOVERFLOW`: Weak count saturated.
 */
FIMO_MUST_USE
FimoError fimo_downgrade_refcount_atomic(FimoAtomicRefCount* count);

/**
 * Checks whether there is only one reference.
 *
 * @param count: the refcount (not `NULL`)
 *
 * @return Whether the reference is unique.
 */
FIMO_MUST_USE
bool fimo_refcount_is_unique(FimoRefCount* count);

/**
 * Checks whether there is only one reference.
 *
 * @param count: the refcount (not `NULL`)
 *
 * Whether the reference is unique.
 */
FIMO_MUST_USE
bool fimo_refcount_atomic_is_unique(FimoAtomicRefCount* count);

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_STD_REFCOUNT_H