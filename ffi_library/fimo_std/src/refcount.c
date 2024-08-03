// This implementation is a modification of the Arc internals of the
// Rust standard library, which is licensed under "Apache-2.0 OR MIT".

#include <fimo_std/refcount.h>

#define MAX_REFCOUNT (FimoUSize) FIMO_ISIZE_MAX
#define LOCKED_SENTINEL FIMO_USIZE_MAX

#if defined(__x86_64__) || defined(_M_X64) || defined(i386) || defined(__i386__) || defined(__i386) || defined(_M_IX86)
#include <immintrin.h>
#define PAUSE _mm_pause()
#else
#define PAUSE
#endif

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_strong_count(const FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    return count->strong_refs;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_strong_count_atomic(const FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    return atomic_load_explicit(&count->strong_refs, memory_order_acquire);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_weak_count_guarded(const FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    if (count->strong_refs == 0) {
        return 0;
    }
    return count->weak_refs - 1;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_weak_count_unguarded(const FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    return count->weak_refs - 1;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_weak_count_atomic_unguarded(const FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    FimoUSize weak_refs = atomic_load_explicit(&count->weak_refs, memory_order_acquire);
    if (weak_refs == LOCKED_SENTINEL) {
        return 0;
    }
    return weak_refs - 1;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_weak_count_atomic_guarded(const FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    FimoUSize weak_refs = atomic_load_explicit(&count->weak_refs, memory_order_acquire);
    FimoUSize strong_refs = atomic_load_explicit(&count->strong_refs, memory_order_acquire);
    if (strong_refs == 0 || weak_refs == LOCKED_SENTINEL) {
        return 0;
    }
    return weak_refs - 1;
}

FIMO_EXPORT
void fimo_increase_strong_count(FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    FimoUSize old_count = count->strong_refs++;
    FIMO_ASSERT(old_count <= MAX_REFCOUNT)
}

FIMO_EXPORT
void fimo_increase_strong_count_atomic(FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    FimoUSize old_count = atomic_fetch_add_explicit(&count->strong_refs, 1, memory_order_relaxed);
    FIMO_ASSERT(old_count <= MAX_REFCOUNT)
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_decrease_strong_count(FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    FimoUSize old_count = count->strong_refs--;
    return old_count == 1;
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_decrease_strong_count_atomic(FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    // If there are more than one strong references, we can take the fast
    // path and return false.
    if (atomic_fetch_sub_explicit(&count->strong_refs, 1, memory_order_release) != 1) {
        return false;
    }

    // This load operation is needed to prevent reordering of use of the
    // data and deletion of the data guarded by the refcount. Decreasing
    // the refcount synchronizes with this acquire load and ensures that
    // any use of the data happens before decreasing the refcount and
    // before deletion of the data.
    //
    // As explained int the Boost documentation [1]
    // > It is important to enforce any possible access to the object in one
    // > thread (through an existing reference) to *happen before* deleting
    // > the object in a different thread. This is achieved by a "release"
    // > operation after dropping a reference (any access to the object
    // > through this reference must obviously happened before), and an
    // > "acquire" operation before deleting the object.
    //
    // [1]: (www.boost.org/doc/libs/1_55_0/doc/html/atomic/usage_examples.html)
    atomic_load_explicit(&count->strong_refs, memory_order_acquire);
    return true;
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_decrease_weak_count(FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    FimoUSize old_count = count->weak_refs--;
    return old_count == 1;
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_decrease_weak_count_atomic(FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    // The same logic as for the strong count in fimo_decrease_strong_count_atomic
    // applies.
    if (atomic_fetch_sub_explicit(&count->weak_refs, 1, memory_order_release) != 1) {
        return false;
    }
    atomic_load_explicit(&count->weak_refs, memory_order_acquire);
    return true;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_upgrade_refcount(FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    if (count->strong_refs == 0) {
        return FIMO_EINVAL;
    }
    if (count->strong_refs > MAX_REFCOUNT) {
        return FIMO_EOVERFLOW;
    }
    ++count->strong_refs;
    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_upgrade_refcount_atomic(FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    // CAS loop
    FimoUSize expected_count = atomic_load_explicit(&count->strong_refs, memory_order_relaxed);
    for (;;) {
        if (expected_count == 0) {
            return FIMO_EINVAL;
        }
        if (expected_count > MAX_REFCOUNT) {
            return FIMO_EOVERFLOW;
        }
        if (atomic_compare_exchange_weak_explicit(&count->strong_refs, &expected_count, expected_count + 1,
                                                  memory_order_acquire, memory_order_relaxed)) {
            return FIMO_EOK;
        }
    }
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_downgrade_refcount(FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    if (count->weak_refs > MAX_REFCOUNT) {
        return FIMO_EOVERFLOW;
    }
    ++count->weak_refs;
    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_downgrade_refcount_atomic(FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    FimoUSize current = atomic_load_explicit(&count->weak_refs, memory_order_relaxed);
    for (;;) {
        // spin while the weak counter is locked.
        if (current == LOCKED_SENTINEL) {
            PAUSE;
            current = atomic_load_explicit(&count->weak_refs, memory_order_relaxed);
            continue;
        }
        if (current > MAX_REFCOUNT) {
            return FIMO_EOVERFLOW;
        }
        if (atomic_compare_exchange_weak_explicit(&count->weak_refs, &current, current + 1, memory_order_acquire,
                                                  memory_order_relaxed)) {
            return FIMO_EOK;
        }
    }
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_refcount_is_unique(FimoRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    return count->strong_refs == 1;
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_refcount_atomic_is_unique(FimoAtomicRefCount *count) {
    FIMO_DEBUG_ASSERT(count)
    // To check whether our atomic refcount is unique, i.e. both the strong
    // and weak counts are 1, we must resort to locking the weak count.
    // We use LOCKED_SENTINEL as a sentinel for the locked state. The
    // acquire memory order ensures a happens-before relationship, for all
    // writes to the strong count (fimo_upgrade_refcount_atomic) followed
    // by decrements of the weak count (fimo_decrease_weak_count_atomic).
    FimoUSize expected = 1;
    if (atomic_compare_exchange_strong_explicit(&count->weak_refs, &expected, LOCKED_SENTINEL, memory_order_acquire,
                                                memory_order_relaxed)) {
        // Use the acquire memory order to synchronize with a call to
        // fimo_decrease_strong_count_atomic.
        bool is_unique = atomic_load_explicit(&count->strong_refs, memory_order_acquire) == 1;

        // Synchronize with fimo_downgrade_refcount_atomic, by using the
        // release memory order.
        atomic_store_explicit(&count->weak_refs, 1, memory_order_release);
        return is_unique;
    }
    else {
        return false;
    }
}
