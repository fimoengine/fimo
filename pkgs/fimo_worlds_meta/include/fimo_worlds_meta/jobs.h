#ifndef FIMO_WORLDS_META_JOBS_H
#define FIMO_WORLDS_META_JOBS_H

#include <stdatomic.h>
#include <stdbool.h>

#include <fimo_std.h>
#include <fimo_tasks_meta/package.h>

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
static void FimoWorldsMeta_fence_init(volatile FimoWorldsMeta_Fence *fence) {
    atomic_init(&fence->state, FIMO_WORLDS_META_FENCE_UNSIGNALED);
}

/// Checks if the fence is already signaled.
static bool FimoWorldsMeta_fence_is_signaled(const volatile FimoWorldsMeta_Fence *fence) {
    const FSTD_U8 state = atomic_load_explicit(&fence->state, memory_order_acquire);
    return (state & FIMO_WORLDS_META_FENCE_SIGNALED) != 0;
}

/// Blocks the caller until the fence is signaled.
static void FimoWorldsMeta_fence_wait(volatile FimoWorldsMeta_Fence *fence, FimoTasksMeta_futex_wait futex_wait) {
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

        futex_wait(&fence->state, sizeof(FSTD_U8), FIMO_WORLDS_META_FENCE_CONTENDED, 0, NULL);
    }
}

/// Wakes all waiters of the fence.
static void FimoWorldsMeta_fence_signal(volatile FimoWorldsMeta_Fence *fence, FimoTasksMeta_futex_wake futex_wake) {
    const FSTD_U8 state =
            atomic_exchange_explicit(&fence->state, FIMO_WORLDS_META_FENCE_SIGNALED, memory_order_release);
    if ((state & FIMO_WORLDS_META_FENCE_CONTENDED) != 0) {
        futex_wake(&fence->state, ~(FSTD_USize)0, FIMO_TASKS_META_FUTEX_FILTER_ALL);
    }
}

/// Resets the state of the fence to be unsignaled.
static void FimoWorldsMeta_fence_reset(volatile FimoWorldsMeta_Fence *fence) {
    const FSTD_U8 state =
            atomic_fetch_and_explicit(&fence->state, ~FIMO_WORLDS_META_FENCE_SIGNALED, memory_order_release);
    fstd_dbg_assert(state != (FIMO_WORLDS_META_FENCE_SIGNALED | FIMO_WORLDS_META_FENCE_CONTENDED));
}

/// A monotonically increasing counter that can be awaited and signaled.
typedef struct FimoWorldsMeta_TimelineSemaphore {
    _Atomic(FSTD_U64) state;
} FimoWorldsMeta_TimelineSemaphore;

/// Initializes the semaphore with a custom initial value.
static void FimoWorldsMeta_timeline_semaphore_init(volatile FimoWorldsMeta_TimelineSemaphore *semaphore,
                                                   FSTD_U64 value) {
    atomic_init(&semaphore->state, value);
}

/// Returns the current counter of the semaphore.
static FSTD_U64 FimoWorldsMeta_timeline_semaphore_counter(const volatile FimoWorldsMeta_TimelineSemaphore *semaphore) {
    return atomic_load_explicit(&semaphore->state, memory_order_acquire);
}

/// Checks if the semaphore is signaled with a count greater or equal to `value`.
static bool FimoWorldsMeta_timeline_semaphore_is_signaled(const volatile FimoWorldsMeta_TimelineSemaphore *semaphore,
                                                          FSTD_U64 value) {
    return atomic_load_explicit(&semaphore->state, memory_order_acquire) >= value;
}

/// Blocks the caller until the semaphore reaches a count greater or equal to `value`.
static void FimoWorldsMeta_timeline_semaphore_wait(const volatile FimoWorldsMeta_TimelineSemaphore *semaphore,
                                                   FSTD_U64 value, FimoTasksMeta_futex_wait futex_wait) {
    // Check if the counter already passed the requested value.
    if (FimoWorldsMeta_timeline_semaphore_is_signaled(semaphore, value))
        return;

    for (;;) {
        const FSTD_U64 state = atomic_load_explicit(&semaphore->state, memory_order_relaxed);
        if (state >= value) {
            atomic_load_explicit(&semaphore->state, memory_order_acquire);
            return;
        }

#if FIMO_USIZE_WIDTH >= 64
        futex_wait(&semaphore->state, sizeof(FSTD_U64), state, value, NULL);
#else
        futex_wait(&semaphore->state, sizeof(FSTD_U64), state, (FSTD_USize)&value, NULL);
#endif
    }
}

/// Sets the internal value of the semaphore, possibly waking waiting tasks.
///
/// `value` must be greater than the current value of the semaphore.
static void FimoWorldsMeta_timeline_semaphore_signal(volatile FimoWorldsMeta_TimelineSemaphore *semaphore,
                                                     FSTD_U64 value, FimoTasksMeta_futex_wake futex_wake) {
    fstd_dbg_assert(atomic_load_explicit(&semaphore->state, memory_order_relaxed) < value);
    atomic_store_explicit(&semaphore->state, value, memory_order_release);

    FimoTasksMeta_FutexFilter filter;
    filter.token_mask = ~(FSTD_USize)0;
#if FSTD_USIZE_WIDTH >= 64
    filter.op = FimoTasksMeta_futex_filter_op_init(
            FIMO_TASKS_META_FUTEX_FILTER_TOKEN_OP_NOOP, FIMO_TASKS_META_FUTEX_FILTER_TOKEN_TYPE_U64,
            FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_LE, FIMO_TASKS_META_FUTEX_FILTER_CMP_ARG_OP_NOOP);
    filter.cmp_arg = value;
#else
    filter.op = FimoTasksMeta_futex_filter_op_init(
            FIMO_TASKS_META_FUTEX_FILTER_TOKEN_OP_DEREF, FIMO_TASKS_META_FUTEX_FILTER_TOKEN_TYPE_U64,
            FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_LE, FIMO_TASKS_META_FUTEX_FILTER_CMP_ARG_OP_DEREF);
    filter.cmp_arg = (FSTD_USize)&value;
#endif

    futex_wake(&semaphore->state, ~(FSTD_USize)0, filter);
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_WORLDS_META_JOBS_H
