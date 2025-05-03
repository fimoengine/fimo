#ifndef FIMO_WORLDS_META_JOBS_H
#define FIMO_WORLDS_META_JOBS_H

#include <stdatomic.h>
#include <stdbool.h>

#include <fimo_std/fimo.h>
#include <fimo_tasks_meta/package.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Definition of a fence.
typedef _Atomic(FimoU8) FimoWorldsMeta_Fence;

#define FIMO_WORLDS_META_FENCE_UNSIGNALED (FimoU8)0
#define FIMO_WORLDS_META_FENCE_SIGNALED (FimoU8)1
#define FIMO_WORLDS_META_FENCE_CONTENDED (FimoU8)2

/// Initializes a new unsignaled fence.
static FIMO_INLINE_ALWAYS
void FimoWorldsMeta_fence_init(volatile FimoWorldsMeta_Fence *fence) {
    atomic_init(fence, FIMO_WORLDS_META_FENCE_UNSIGNALED);
}

/// Checks if the fence is already signaled.
static FIMO_INLINE_ALWAYS
bool FimoWorldsMeta_fence_is_signaled(const volatile FimoWorldsMeta_Fence *fence) {
    const FimoU8 state = atomic_load_explicit(fence, memory_order_acquire);
    return (state & FIMO_WORLDS_META_FENCE_SIGNALED) != 0;
}

static bool FimoWorldsMeta_Impl_fence_wait_validate(void *ptr) {
    const FimoWorldsMeta_Fence *fence = (const FimoWorldsMeta_Fence *)ptr;
    return atomic_load_explicit(fence, memory_order_relaxed) == FIMO_WORLDS_META_FENCE_CONTENDED;
}

static void FimoWorldsMeta_Impl_fence_wait_before_sleep(void*) {}
static void FimoWorldsMeta_Impl_fence_wait_timeout(void*, const void*, bool) {}

/// Blocks the caller until the fence is signaled.
static FIMO_INLINE_ALWAYS
void FimoWorldsMeta_fence_wait(
    volatile FimoWorldsMeta_Fence *fence,
    FimoTasksMeta_parking_lot_park park
) {
    for (;;) {
        FimoU8 state = atomic_load_explicit(fence, memory_order_relaxed);
        if ((state & FIMO_WORLDS_META_FENCE_SIGNALED) != 0) {
            // An atomic fence would probably suffice, but we use an
            // acquire load to support thread sanitizer.
            if (FimoWorldsMeta_fence_is_signaled(fence)) return;
        }

        if ((state & FIMO_WORLDS_META_FENCE_CONTENDED) == 0) {
            if (!atomic_compare_exchange_weak_explicit(
                fence,
                FIMO_WORLDS_META_FENCE_UNSIGNALED,
                FIMO_WORLDS_META_FENCE_CONTENDED,
                memory_order_relaxed,
                memory_order_relaxed
            )) continue;
        }

        park(
            fence,
            fence,
            FimoWorldsMeta_Impl_fence_wait_validate,
            NULL,
            FimoWorldsMeta_Impl_fence_wait_before_sleep,
            NULL,
            FimoWorldsMeta_Impl_fence_wait_timeout,
            FIMO_TASKS_META_PARKING_LOT_PARK_TOKEN_DEFAULT,
            NULL
        );
    }
}

/// Wakes all waiters of the fence.
static FIMO_INLINE_ALWAYS
void FimoWorldsMeta_fence_signal(
    volatile FimoWorldsMeta_Fence *fence,
    FimoTasksMeta_parking_lot_unpark_all unpark_all
) {
    const FimoU8 state = atomic_exchange_explicit(
        fence,
        FIMO_WORLDS_META_FENCE_SIGNALED,
        memory_order_release
    );
    if ((state & FIMO_WORLDS_META_FENCE_CONTENDED) != 0) {
        unpark_all(fence, FIMO_TASKS_META_PARKING_LOT_UNPARK_TOKEN_DEFAULT);
    }
}

/// Resets the state of the fence to be unsignaled.
static FIMO_INLINE_ALWAYS
void FimoWorldsMeta_fence_reset(volatile FimoWorldsMeta_Fence *fence) {
    const FimoU8 state = atomic_fetch_and_explicit(
        fence,
        ~FIMO_WORLDS_META_FENCE_SIGNALED,
        memory_order_release
    );
    FIMO_DEBUG_ASSERT(state != (FIMO_WORLDS_META_FENCE_SIGNALED | FIMO_WORLDS_META_FENCE_CONTENDED));
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_WORLDS_META_JOBS_H
