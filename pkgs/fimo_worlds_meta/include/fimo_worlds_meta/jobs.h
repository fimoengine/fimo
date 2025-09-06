#ifndef FIMO_WORLDS_META_JOBS_H
#define FIMO_WORLDS_META_JOBS_H

#include <stdatomic.h>
#include <stdbool.h>

#include <fimo_std.h>
#include <fimo_tasks.h>

#ifdef __cplusplus
extern "C" {
#endif

/// A fence to synchronize the execution of individual jobs.
typedef struct FimoWorldsMeta_Fence {
    _Atomic(FSTD_U8) state;
} FimoWorldsMeta_Fence;

#define FIMO_WORLDS_META_FENCE_UNSIGNALED (FSTD_U8)0
#define FIMO_WORLDS_META_FENCE_SIGNALED (FSTD_U8)1
#define FIMO_WORLDS_META_FENCE_CONTENDED (FSTD_U8)2

/// Initializes a new unsignaled fence.
static void FimoWorldsMeta_fence_init(FimoWorldsMeta_Fence *fence) {
    atomic_init(&fence->state, FIMO_WORLDS_META_FENCE_UNSIGNALED);
}

/// Checks if the fence is already signaled.
static bool FimoWorldsMeta_fence_is_signaled(const FimoWorldsMeta_Fence *fence) {
    const FSTD_U8 state = atomic_load_explicit(&fence->state, memory_order_acquire);
    return (state & FIMO_WORLDS_META_FENCE_SIGNALED) != 0;
}

/// Blocks the caller until the fence is signaled.
static void FimoWorldsMeta_fence_wait(FimoWorldsMeta_Fence *fence) {
    for (;;) {
        FSTD_U8 state = atomic_load_explicit(&fence->state, memory_order_relaxed);
        if ((state & FIMO_WORLDS_META_FENCE_SIGNALED) != 0) {
            // An atomic fence would probably suffice, but we use an
            // acquire load to support thread sanitizer.
            if (FimoWorldsMeta_fence_is_signaled(&fence->state))
                return;
        }

        if ((state & FIMO_WORLDS_META_FENCE_CONTENDED) == 0) {
            if (!atomic_compare_exchange_weak_explicit(&fence->state, FIMO_WORLDS_META_FENCE_UNSIGNALED,
                                                       FIMO_WORLDS_META_FENCE_CONTENDED, memory_order_relaxed,
                                                       memory_order_relaxed))
                continue;
        }

        ftsk_futex_wait(&fence->state, sizeof(FSTD_U8), FIMO_WORLDS_META_FENCE_CONTENDED, 0, fstd_nullptr);
    }
}

/// Wakes all waiters of the fence.
static void FimoWorldsMeta_fence_signal(FimoWorldsMeta_Fence *fence) {
    const FSTD_U8 state =
            atomic_exchange_explicit(&fence->state, FIMO_WORLDS_META_FENCE_SIGNALED, memory_order_release);
    if ((state & FIMO_WORLDS_META_FENCE_CONTENDED) != 0) {
        FTSK_FutexFilter filter = FTSK_FUTEX_FILTER_ALL;
        ftsk_futex_wake(&fence->state, ~(FSTD_USize)0, filter);
    }
}

/// Resets the state of the fence to be unsignaled.
static void FimoWorldsMeta_fence_reset(FimoWorldsMeta_Fence *fence) {
    const FSTD_U8 state =
            atomic_fetch_and_explicit(&fence->state, ~FIMO_WORLDS_META_FENCE_SIGNALED, memory_order_release);
    fstd_dbg_assert(state != (FIMO_WORLDS_META_FENCE_SIGNALED | FIMO_WORLDS_META_FENCE_CONTENDED));
}

/// A monotonically increasing counter that can be awaited and signaled.
typedef struct FimoWorldsMeta_TimelineSemaphore {
    _Atomic(FSTD_U64) state;
} FimoWorldsMeta_TimelineSemaphore;

/// Initializes the semaphore with a custom initial value.
static void FimoWorldsMeta_timeline_semaphore_init(FimoWorldsMeta_TimelineSemaphore *semaphore, FSTD_U64 value) {
    atomic_init(&semaphore->state, value);
}

/// Returns the current counter of the semaphore.
static FSTD_U64 FimoWorldsMeta_timeline_semaphore_counter(const FimoWorldsMeta_TimelineSemaphore *semaphore) {
    return atomic_load_explicit(&semaphore->state, memory_order_acquire);
}

/// Checks if the semaphore is signaled with a count greater or equal to `value`.
static bool FimoWorldsMeta_timeline_semaphore_is_signaled(const FimoWorldsMeta_TimelineSemaphore *semaphore,
                                                          FSTD_U64 value) {
    return atomic_load_explicit(&semaphore->state, memory_order_acquire) >= value;
}

/// Blocks the caller until the semaphore reaches a count greater or equal to `value`.
static void FimoWorldsMeta_timeline_semaphore_wait(const FimoWorldsMeta_TimelineSemaphore *semaphore, FSTD_U64 value) {
    // Check if the counter already passed the requested value.
    if (FimoWorldsMeta_timeline_semaphore_is_signaled(semaphore, value))
        return;

    for (;;) {
        const FSTD_U64 state = atomic_load_explicit(&semaphore->state, memory_order_relaxed);
        if (state >= value) {
            atomic_load_explicit(&semaphore->state, memory_order_acquire);
            return;
        }

#if FSTD_PTR_64
        ftsk_futex_wait(&semaphore->state, sizeof(FSTD_U64), state, value, fstd_nullptr);
#else
#error "unsupported target"
#endif
    }
}

/// Sets the internal value of the semaphore, possibly waking waiting tasks.
///
/// `value` must be greater than the current value of the semaphore.
static void FimoWorldsMeta_timeline_semaphore_signal(FimoWorldsMeta_TimelineSemaphore *semaphore, FSTD_U64 value) {
    fstd_dbg_assert(atomic_load_explicit(&semaphore->state, memory_order_relaxed) < value);
    atomic_store_explicit(&semaphore->state, value, memory_order_release);

#if FSTD_PTR_64
    FTSK_FutexFilter filter = {
            .token_op = FTSK_FutexFilterOp_Noop,
            .token_type = FTSK_FutexFilterTokenType_U64,
            .cmp_op = FTSK_FutexFilterCmp_Le,
            .cmp_arg_op = FTSK_FutexFilterOp_Noop,
            .token_mask = ~(FSTD_USize)0,
            .cmp_arg = value,
    };
#else
#error "unsupported target"
#endif
    ftsk_futex_wake(&semaphore->state, ~(FSTD_USize)0, filter);
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_WORLDS_META_JOBS_H
