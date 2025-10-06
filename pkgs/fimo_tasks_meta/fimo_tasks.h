/// fimo_tasks - v0.2

#ifndef FIMO_TASKS_HEADER
#define FIMO_TASKS_HEADER

#include <fimo_std.h>

#include <stdatomic.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Identifier of a task.
typedef FSTD_USize FTSK_TaskId;

/// Returns the id of the current task.
fstd_func bool ftsk_task_id_current(FTSK_TaskId *id);

/// Identifier of a worker thread in an executor.
typedef FSTD_USize FTSK_Worker;

/// Returns the id of the worker.
fstd_func bool ftsk_worker_id_current(FTSK_Worker *id);

/// A unit of work.
typedef struct FTSK_Task {
    /// Optional label of the task.
    ///
    /// May be used by the runtime for tracing purposes.
    /// If present, the string must live until the task instance is destroyed.
    FSTD_StrConst label;
    /// Number of sub-tasks to start.
    FSTD_USize batch_len;
    /// Entry function of the task.
    void (*run)(struct FTSK_Task *task, FSTD_USize idx);
} FTSK_Task;

/// Yields the current task or thread back to the scheduler.
fstd_func void ftsk_yield(void);

/// Aborts the current task.
fstd_func void ftsk_abort(void);

/// Reports whether a cancellation of the current task has been requested.
fstd_func bool ftsk_cancel_requested(void);

/// Puts the current task or thread to sleep for the specified amount of time.
fstd_func void ftsk_sleep(FSTD_Duration duration);

/// Fetches the arena of the current task.
fstd_func FSTD_Arena *FSTD_MAYBE_NULL ftsk_task_arena(void);

/// A key for a task-specific-storage.
///
/// A new key can be defined by casting from a stable address.
typedef struct FTSK_TssKey FTSK_TssKey;
typedef void (*FTSK_TssKeyDtor)(void *FSTD_MAYBE_NULL value);

/// Associates a value with the key for the current task.
///
/// The current value associated with the key is replaced with the new value without
/// invoking any destructor function. The destructor function is set to `dtor`, and will
/// be invoked upon task exit. May only be called by a task.
fstd_func void ftsk_tss_key_set(const FTSK_TssKey *key, void *FSTD_MAYBE_NULL value,
                                FTSK_TssKeyDtor FSTD_MAYBE_NULL dtor);

/// Returns the value associated to the key for the current task.
///
/// May only be called by a task.
fstd_func void *FSTD_MAYBE_NULL ftsk_tss_key_get(const FTSK_TssKey *key);

/// Clears the value of the current task associated with the key.
///
/// This operation invokes the associated destructor function and sets the value to `null`.
/// May only be called by a task.
fstd_func void ftsk_tss_key_clear(const FTSK_TssKey *key);

typedef FSTD_I32 FTSK_CmdBufCmdTag;
enum {
    FTSK_CmdBufCmdTag_Noop = (FTSK_CmdBufCmdTag)0,
    FTSK_CmdBufCmdTag_SelectWorker = (FTSK_CmdBufCmdTag)1,
    FTSK_CmdBufCmdTag_SelectAnyWorker = (FTSK_CmdBufCmdTag)2,
    FTSK_CmdBufCmdTag_EnqueueTask = (FTSK_CmdBufCmdTag)3,
    FTSK_CmdBufCmdTag_WaitOnBarrier = (FTSK_CmdBufCmdTag)4,
    FTSK_CmdBufCmdTag_WaitOnCmdIndirect = (FTSK_CmdBufCmdTag)5,
    FTSK__CmdBufCmdTag_ = FSTD_I32_MAX,
};

/// An entry of a command buffer.
typedef struct {
    FTSK_CmdBufCmdTag tag;
    union {
        FTSK_Worker select_worker;
        FSTD_U8 select_any_worker;
        FTSK_Task *enqueue_task;
        FSTD_U8 wait_on_barrier;
        FSTD_USize wait_on_cmd_indirect;
    };
} FTSK_CmdBufCmd;

/// A list of commands to process by an executor.
typedef struct FTSK_CmdBuf {
    /// Optional label of the command buffer.
    ///
    /// May be used by the runtime for tracing purposes.
    /// If present, the string must live until the buffer is destroyed.
    FSTD_StrConst label;
    /// List of commands.
    FSTD_SliceConst(FTSK_CmdBufCmd) cmds;
    /// Optional cleanup function of the buffer.
    void (*FSTD_MAYBE_NULL deinit)(struct FTSK_CmdBuf *cmd_buf);
} FTSK_CmdBuf;

/// A handle to an enqueued command buffer.
typedef struct FTSK_CmdBufHandle FTSK_CmdBufHandle;

typedef FSTD_I32 FTSK_CmdBufHandleCompletionStatus;
enum {
    FTSK_CmdBufHandleCompletionStatus_Completed = (FTSK_CmdBufHandleCompletionStatus)0,
    FTSK_CmdBufHandleCompletionStatus_Cancelled = (FTSK_CmdBufHandleCompletionStatus)1,
    FTSK__CmdBufHandleCompletionStatus_ = FSTD_I32_MAX,
};

/// Waits for the command buffer to complete.
///
/// Once called, the handle is consumed.
fstd_func FTSK_CmdBufHandleCompletionStatus ftsk_cmd_buf_handle_join(FTSK_CmdBufHandle *cmd_buf);

/// Release the obligation of the caller to call join and
/// have the handle be cleaned up on completion.
///
/// Once called, the handle is consumed.
fstd_func void ftsk_cmd_buf_handle_detach(FTSK_CmdBufHandle *cmd_buf);

/// Like join, but flags the handle as cancelled.
fstd_func void ftsk_cmd_buf_handle_cancel(FTSK_CmdBufHandle *cmd_buf);

/// Like detach, but flags the handle as cancelled.
fstd_func void ftsk_cmd_buf_handle_cancel_detach(FTSK_CmdBufHandle *cmd_buf);

typedef struct {
    /// Optional label of the executor.
    FSTD_StrConst label;
    /// Maximum number of enqueued cmd buffers.
    ///
    /// A value of `0` indicates to use the default capacity.
    FSTD_USize cmd_buf_capacity;
    /// Number of worker threads owned by the executor.
    ///
    /// A value of `0` indicates to use the default number of workers.
    FSTD_USize worker_count;
    /// Controls the maximum number of spawned tasks.
    ///
    /// The maximum number of spawned tasks is determined as `worker_count * max_load_factor`.
    /// A value of `0` indicates to use the default load factor.
    FSTD_USize max_load_factor;
    /// Minimum stack size in bytes.
    ///
    /// A value of `0` indicates to use the default stack size.
    FSTD_USize stack_size;
    /// Minimum size of the per-task arena.
    ///
    /// A value of `0` indicates to use the default arena size.
    FSTD_USize arena_size;
    /// Number of cached stacks per worker.
    ///
    /// The cache is shared among all workers.
    /// A value of `0` indicates to use the default cache length.
    FSTD_USize worker_stack_cache_len;
    /// Indicates whether to disable the stack cache.
    bool disable_stack_cache;
} FTSK_ExecutorCfg;

typedef struct FTSK_Executor FTSK_Executor;

/// Returns the global executor.
fstd_func FTSK_Executor *ftsk_global_executor();

/// Creates a new executor with the provided configuration.
fstd_func FSTD_Status ftsk_executor_init(FTSK_Executor **exe, const FTSK_ExecutorCfg *cfg);

/// Returns the executor for the current context.
///
/// Is only valid for the duration of the current context (i.e. Task).
fstd_func FTSK_Executor *ftsk_executor_current();

/// Waits until all remaining commands have been executed and consumes the handle.
///
/// New commands can be enqueued to the executor while the call is in process.
fstd_func void ftsk_executor_join(FTSK_Executor *exe);

/// Reports whether the owner of the executor has requested that the executor be joined.
fstd_func bool ftsk_executor_join_requested(FTSK_Executor *exe);

/// Enqueues the commands to the executor.
///
/// The caller will block until the handle could be enqueued.
/// The buffer must outlive the returned handle.
fstd_func FTSK_CmdBufHandle *ftsk_executor_enqueue(FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf);

/// Enqueues the commands to the executor.
///
/// The caller will block until the handle could be enqueued.
/// The buffer must outlive the returned handle.
fstd_func void ftsk_executor_enqueue_detached(FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf);

/// Maximum number of keys allowed for the `waitv` operation.
#define FTSK_FUTEX_MAX_WAITV_KEY_COUNT 128

/// Possible status codes of the futex operations.
typedef FSTD_I32 FTSK_FutexStatus;
enum {
    FTSK_FutexStatus_Ok = (FTSK_FutexStatus)0,
    FTSK_FutexStatus_Invalid = (FTSK_FutexStatus)1,
    FTSK_FutexStatus_Timeout = (FTSK_FutexStatus)2,
    FTSK_FutexStatus_KeyError = (FTSK_FutexStatus)3,
    FTSK__FutexStatus_ = FSTD_I32_MAX,
};

/// Information required for a wait operation.
typedef struct {
    const void *key;
    FSTD_USize key_size;
    FSTD_U64 expect;
    FSTD_USize token;
} FTSK_FutexKeyExpect;

typedef FSTD_USize FTSK_FutexFilterOp;
enum {
    FTSK_FutexFilterOp_Noop = 0,
    FTSK_FutexFilterOp_Deref = 1,
    FTSK__FutexFilterOp_ = FSTD_I32_MAX,
};

typedef FSTD_USize FTSK_FutexFilterTokenType;
enum {
    FTSK_FutexFilterTokenType_U8 = 0,
    FTSK_FutexFilterTokenType_U16 = 1,
    FTSK_FutexFilterTokenType_U32 = 2,
    FTSK_FutexFilterTokenType_U64 = 3,
    FTSK__FutexFilterTokenType_ = FSTD_I32_MAX,
};

typedef FSTD_USize FTSK_FutexFilterCmp;
enum {
    FTSK_FutexFilterCmp_Eq = 0,
    FTSK_FutexFilterCmp_Ne = 1,
    FTSK_FutexFilterCmp_Lt = 2,
    FTSK_FutexFilterCmp_Le = 3,
    FTSK_FutexFilterCmp_Gt = 4,
    FTSK_FutexFilterCmp_Ge = 5,
    FTSK__FutexFilterCmp_ = FSTD_I32_MAX,
};

/// Filter for a filter operation.
typedef struct {
    FTSK_FutexFilterOp token_op : 1;
    FTSK_FutexFilterTokenType token_type : 2;
    FTSK_FutexFilterCmp cmp_op : 3;
    FTSK_FutexFilterOp cmp_arg_op : 1;
    FSTD_USize token_mask;
    FSTD_USize cmp_arg;
} FTSK_FutexFilter;

/// A filter that accepts everything.
#define FTSK_FUTEX_FILTER_ALL                                                                                          \
    {                                                                                                                  \
            .token_op = FTSK_FutexFilterOp_Noop,                                                                       \
            .token_type = FTSK_FutexFilterTokenType_U8,                                                                \
            .cmp_op = FTSK_FutexFilterCmp_Eq,                                                                          \
            .cmp_arg_op = FTSK_FutexFilterOp_Noop,                                                                     \
            .token_mask = 0,                                                                                           \
            .cmp_arg = 0,                                                                                              \
    }

/// Result of the requeue operation.
typedef struct {
    FSTD_USize wake_count;
    FSTD_USize requeue_count;
} FTSK_FutexRequeueResult;

typedef FSTD_SliceConst(FTSK_FutexKeyExpect) FTSK_FutexKeyExpectSlice;

/// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
///
/// If the value does not match, the function returns imediately with `Invalid`. The `key_size`
/// parameter specifies the size of the value in bytes and must be either of `1`, `2`, `4` or `8`,
/// in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or `u64` respectively, and
/// `expect` is truncated. The `token` is a user definable integer to store additional metadata
/// about the waiter, which can be utilized to controll some wake operations.
///
/// If `timeout` is reached before a wake operation wakes the task, the task will be resumed, and
/// the function returns `Timeout`.
fstd_func FTSK_FutexStatus ftsk_futex_wait(const void *key, FSTD_USize key_size, FSTD_U64 expect, FSTD_USize token,
                                           const FSTD_Instant *FSTD_MAYBE_NULL timeout);

/// Puts the caller to sleep if all keys match their expected values.
///
/// Is a generalization of `wait` for multiple keys. At least `1` key must, and at most
/// `FTSK_FUTEX_MAX_WAITV_KEY_COUNT` may be passed to this function. Otherwise it returns `KeyError`.
fstd_func FTSK_FutexStatus ftsk_futex_waitv(FTSK_FutexKeyExpectSlice keys, const FSTD_Instant *FSTD_MAYBE_NULL timeout,
                                            FSTD_USize *wake_index);

/// Wakes at most `max_waiters` waiting on `key`.
///
/// Uses the token provided by the waiter and the `filter` to determine whether to ignore it from
/// being woken up. Returns the number of woken waiters.
fstd_func FSTD_USize ftsk_futex_wake(const void *key, FSTD_USize max_waiters, FTSK_FutexFilter filter);

/// Requeues waiters from `key_from` to `key_to`.
///
/// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
/// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
/// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
/// the function returns `Invalid`. Uses the token provided by the waiter and the `filter` to
/// determine whether to ignore it from being woken up.
fstd_func FTSK_FutexStatus ftsk_futex_requeue(const void *key_from, const void *key_to, FSTD_USize key_size,
                                              FSTD_U64 expect, FSTD_USize max_wakes, FSTD_USize max_requeues,
                                              FTSK_FutexFilter filter, FTSK_FutexRequeueResult *result);

fstd_func void ftsk__spin_loop_hint() {
#if defined(FSTD_ARCH_X86_64)

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
    __builtin_ia32_pause();
#elif defined(FSTD_COMPILER_MSC)
    _mm_pause();
#else
#error "unsupported compiler"
#endif

#elif defined(FSTD_ARCH_AARCH64)

#if defined(FSTD_COMPILER_GCC_COMPATIBLE)
    asm volatile("yield");
#elif defined(FSTD_COMPILER_MSC)
    __yield();
#else
#error "unsupported compiler"
#endif

#else
#error "unsupported arch"
#endif
}

/// Mutex is a synchronization primitive which enforces atomic access to a
/// shared region of code known as the "critical section".
///
/// It does this by blocking ensuring only one task is in the critical
/// section at any given point in time by blocking the others.
// Taken from https://github.com/rust-lang/rust/blob/master/library/std/src/sys/sync/mutex/futex.rs
typedef struct {
    _Atomic(FSTD_U8) state;
} FTSK_Mutex;
fstd_static_assert(sizeof(FTSK_Mutex) == sizeof(FSTD_U8), "invalid FTSK_Mutex size");

#define FTSK_MUTEX_INIT FSTD_DEFAULT_STRUCT
#define FTSK__MUTEX_UNLOCKED 0
#define FTSK__MUTEX_LOCKED 1
#define FTSK__MUTEX_CONTENDED 2

/// Tries to acquire the mutex without blocking the caller's task.
///
/// Returns `false` if the calling task would have to block to acquire it.
/// Otherwise, returns `true` and the caller should `unlock()` the Mutex to release it.
fstd_util bool ftsk_mutex_try_lock(FTSK_Mutex *mutex) {
    FSTD_U8 expect = FTSK__MUTEX_UNLOCKED;
    return atomic_compare_exchange_strong_explicit(&mutex->state, &expect, FTSK__MUTEX_LOCKED, memory_order_acquire,
                                                   memory_order_relaxed);
}

fstd_util FSTD_U8 ftsk__mutex_spin(FTSK_Mutex *mutex) {
    FSTD_U8 spin = 100;
    for (;;) {
        FSTD_U8 curr = atomic_load_explicit(&mutex->state, memory_order_relaxed);
        if (curr != FTSK__MUTEX_LOCKED || spin == 0) {
            return curr;
        }

        ftsk__spin_loop_hint();
        spin -= 1;
    }
}

FSTD_EXPAND_GCC_COMPATIBLE(__attribute__((cold)))
fstd_util bool ftsk__mutex_lock_contended(FTSK_Mutex *mutex, const FSTD_Instant *timeout) {
    FSTD_U8 curr = ftsk__mutex_spin(mutex);
    if (curr == FTSK__MUTEX_UNLOCKED) {
        if (ftsk_mutex_try_lock(mutex))
            return true;
    }

    for (;;) {
        if (curr != FTSK__MUTEX_CONTENDED) {
            curr = atomic_exchange_explicit(&mutex->state, FTSK__MUTEX_CONTENDED, memory_order_acquire);
            if (curr == FTSK__MUTEX_UNLOCKED)
                return true;
        }

        FTSK_FutexStatus status = ftsk_futex_wait(mutex, sizeof(*mutex), FTSK__MUTEX_CONTENDED, 0, timeout);
        if (status == FTSK_FutexStatus_Timeout)
            return false;
    }
}

/// Acquires the mutex, blocking the caller's task until it can.
///
/// Once acquired, call `unlock()` on the Mutex to release it.
fstd_util void ftsk_mutex_lock(FTSK_Mutex *mutex) {
    if (!ftsk_mutex_try_lock(mutex)) {
        ftsk__mutex_lock_contended(mutex, fstd_nullptr);
    }
}

/// Tries to acquire the mutex, blocking the caller's task until it can or the timeout is reached.
///
/// Returns `true` if the lock could be acquired.
/// Once acquired, call `unlock()` on the Mutex to release it.
fstd_util bool ftsk_mutex_timed_lock(FTSK_Mutex *mutex, FSTD_Duration timeout) {
    if (!ftsk_mutex_try_lock(mutex)) {
        FSTD_Instant t = fstd_instant_add_saturating(fstd_instant_now(), timeout);
        return ftsk__mutex_lock_contended(mutex, &t);
    }
    return true;
}

/// Releases the mutex which was previously acquired.
fstd_util void ftsk_mutex_unlock(FTSK_Mutex *mutex) {
    FSTD_U8 state = atomic_exchange_explicit(&mutex->state, FTSK__MUTEX_UNLOCKED, memory_order_release);
    if (state == FTSK__MUTEX_CONTENDED) {
        ftsk_futex_wake(mutex, 1, FSTD_INIT(FTSK_FutexFilter) FTSK_FUTEX_FILTER_ALL);
    }
}

/// Condition variables are used with a Mutex to efficiently wait for an arbitrary condition to occur.
/// It does this by atomically unlocking the mutex, blocking the thread until notified, and finally re-locking the
/// mutex.
typedef struct {
    _Atomic(FSTD_U32) futex;
} FTSK_Condition;
fstd_static_assert(sizeof(FTSK_Condition) == sizeof(FSTD_U32), "invalid FTSK_Condition size");

#define FTSK_CONDITION_INIT FSTD_DEFAULT_STRUCT

fstd_util bool ftsk__condition_wait(FTSK_Condition *condition, FTSK_Mutex *mutex, const FSTD_Instant *timeout) {
    FSTD_U32 current = atomic_load_explicit(&condition->futex, memory_order_acquire);
    ftsk_mutex_unlock(mutex);
    FTSK_FutexStatus status = ftsk_futex_wait(condition, sizeof(*condition), current, 0, timeout);
    ftsk_mutex_lock(mutex);
    return status != FTSK_FutexStatus_Timeout;
}

/// Atomically releases the Mutex, blocks the caller task, then re-acquires the Mutex on return.
/// "Atomically" here refers to accesses done on the Condition after acquiring the Mutex.
///
/// The Mutex must be locked by the caller's task when this function is called.
/// A Mutex can have multiple Conditions waiting with it concurrently, but not the opposite.
/// It is undefined behavior for multiple tasks to wait with different mutexes using the same Condition concurrently.
/// Once tasks have finished waiting with one Mutex, the Condition can be used to wait with another Mutex.
///
/// A blocking call to wait() is unblocked from one of the following conditions:
/// - a spurious ("at random") wake up occurs
/// - a future call to `signal()` or `broadcast()` which has acquired the Mutex and is sequenced after this `wait()`.
///
/// Given wait() can be interrupted spuriously, the blocking condition should be checked continuously
/// irrespective of any notifications from `signal()` or `broadcast()`.
fstd_util void ftsk_condition_wait(FTSK_Condition *condition, FTSK_Mutex *mutex) {
    ftsk__condition_wait(condition, mutex, fstd_nullptr);
}

/// Atomically releases the Mutex, blocks the caller task, then re-acquires the Mutex on return.
/// "Atomically" here refers to accesses done on the Condition after acquiring the Mutex.
///
/// The Mutex must be locked by the caller's task when this function is called.
/// A Mutex can have multiple Conditions waiting with it concurrently, but not the opposite.
/// It is undefined behavior for multiple tasks to wait with different mutexes using the same Condition concurrently.
/// Once tasks have finished waiting with one Mutex, the Condition can be used to wait with another Mutex.
///
/// A blocking call to `timedWait()` is unblocked from one of the following conditions:
/// - a spurious ("at random") wake occurs
/// - the caller was blocked for around `timeout`, in which `error.Timeout` is returned.
/// - a future call to `signal()` or `broadcast()` which has acquired the Mutex and is sequenced after this
/// `timedWait()`.
///
/// Given `timedWait()` can be interrupted spuriously, the blocking condition should be checked continuously
/// irrespective of any notifications from `signal()` or `broadcast()`.
///
/// Returns `true` if the caller was woken up before the timeout elapsed.
fstd_util bool ftsk_condition_timed_wait(FTSK_Condition *condition, FTSK_Mutex *mutex, FSTD_Duration timeout) {
    FSTD_Instant t = fstd_instant_add_saturating(fstd_instant_now(), timeout);
    return ftsk__condition_wait(condition, mutex, &t);
}

/// Unblocks at least one task blocked in a call to `wait()` or `timedWait()` with a given Mutex.
/// The blocked task must be sequenced before this call with respect to acquiring the same Mutex in order to be
/// observable for unblocking. `signal()` can be called with or without the relevant Mutex being acquired and have no
/// "effect" if there's no observable blocked threads.
fstd_util void ftsk_condition_signal(FTSK_Condition *condition) {
    atomic_fetch_add_explicit(&condition->futex, 1, memory_order_relaxed);
    ftsk_futex_wake(condition, 1, FSTD_INIT(FTSK_FutexFilter) FTSK_FUTEX_FILTER_ALL);
}

/// Unblocks all tasks currently blocked in a call to `wait()` or `timedWait()` with a given Mutex.
/// The blocked tasks must be sequenced before this call with respect to acquiring the same Mutex in order to be
/// observable for unblocking. `broadcast()` can be called with or without the relevant Mutex being acquired and have no
/// "effect" if there's no observable blocked threads.
fstd_util void ftsk_condition_broadcast(FTSK_Condition *condition) {
    atomic_fetch_add_explicit(&condition->futex, 1, memory_order_relaxed);
    ftsk_futex_wake(condition, (FSTD_USize)-1, FSTD_INIT(FTSK_FutexFilter) FTSK_FUTEX_FILTER_ALL);
}

/// A thread-safe boolean that can be set and awaited,
/// typically to mark/await the completion of some operation.
typedef struct {
    _Atomic(FSTD_U8) state;
} FTSK_Fence;
fstd_static_assert(sizeof(FTSK_Fence) == sizeof(FSTD_U8), "invalid FTSK_Fence size");

#define FTSK_FENCE_INIT FSTD_DEFAULT_STRUCT
#define FTSK__FENCE_UNSIGNALED 0
#define FTSK__FENCE_SIGNALED 1
#define FTSK__FENCE_CONTENDED 2

/// Checks if the fence is already signaled.
fstd_util bool ftsk_fence_is_signaled(FTSK_Fence *fence) {
    return (atomic_load_explicit(&fence->state, memory_order_acquire) & FTSK__FENCE_SIGNALED) != 0;
}

FSTD_EXPAND_GCC_COMPATIBLE(__attribute__((cold)))
fstd_util bool ftsk__fence_wait(FTSK_Fence *fence, const FSTD_Instant *timeout) {
    FSTD_U8 current = atomic_load_explicit(&fence->state, memory_order_relaxed);
    for (;;) {
        if ((current & FTSK__FENCE_SIGNALED) != 0) {
            (void)atomic_load_explicit(&fence->state, memory_order_acquire);
            return true;
        }
        if ((current & FTSK__FENCE_CONTENDED) == 0) {
            if (!atomic_compare_exchange_weak_explicit(&fence->state, &current, FTSK__FENCE_CONTENDED,
                                                       memory_order_relaxed, memory_order_relaxed)) {
                continue;
            }
        }

        FTSK_FutexStatus status = ftsk_futex_wait(fence, sizeof(*fence), FTSK__FENCE_CONTENDED, 0, timeout);
        if (status == FTSK_FutexStatus_Timeout)
            return false;
    }
}

/// Blocks the caller until the fence is signaled.
fstd_util void ftsk_fence_wait(FTSK_Fence *fence) {
    if (!ftsk_fence_is_signaled(fence)) {
        ftsk__fence_wait(fence, fstd_nullptr);
    }
}

/// Blocks the caller until the fence is signaled, or the timeout expires.
fstd_util bool ftsk_fence_timed_wait(FTSK_Fence *fence, FSTD_Duration timeout) {
    if (!ftsk_fence_is_signaled(fence)) {
        FSTD_Instant t = fstd_instant_add_saturating(fstd_instant_now(), timeout);
        return ftsk__fence_wait(fence, &t);
    }
    return true;
}

/// Wakes all waiters of the fence.
fstd_util void ftsk_fence_signal(FTSK_Fence *fence) {
    FSTD_U8 curr = atomic_exchange_explicit(&fence->state, FTSK__FENCE_SIGNALED, memory_order_release);
    if ((curr & FTSK__FENCE_CONTENDED) != 0) {
        ftsk_futex_wake(fence, ~(FSTD_USize)0, FSTD_INIT(FTSK_FutexFilter) FTSK_FUTEX_FILTER_ALL);
    }
}

/// Resets the state of the fence to be unsignaled.
///
/// May not be called while threads are waiting on the fence.
fstd_util void ftsk_fence_reset(FTSK_Fence *fence) {
    atomic_store_explicit(&fence->state, FTSK__FENCE_UNSIGNALED, memory_order_release);
}

/// A monotonically increasing counter that can be awaited and signaled.
typedef struct {
    _Atomic(FSTD_U64) state;
} FTSK_TimelineSemaphore;
fstd_static_assert(sizeof(FTSK_TimelineSemaphore) == sizeof(FSTD_U64), "invalid FTSK_TimelineSemaphore size");
fstd_static_assert(sizeof(FSTD_USize) <= sizeof(FSTD_U64), "FTSK_TimelineSemaphore supports up to 64 bit pointers");

#define FTSK_FENCE_INIT FSTD_DEFAULT_STRUCT
#define FTSK_FENCE_INIT_WITH(counter) {.state = counter}

/// Returns the current counter of the semaphore.
fstd_util FSTD_U64 ftsk_timeline_semaphore_counter(FTSK_TimelineSemaphore *tsem) {
    return atomic_load_explicit(&tsem->state, memory_order_acquire);
}

/// Checks if the semaphore is signaled with a count greater or equal to `value`.
fstd_util bool ftsk_timeline_semaphore_is_signaled(FTSK_TimelineSemaphore *tsem, FSTD_U64 value) {
    return ftsk_timeline_semaphore_counter(tsem) >= value;
}

FSTD_EXPAND_GCC_COMPATIBLE(__attribute__((cold)))
fstd_util bool ftsk__timeline_semaphore_wait(FTSK_TimelineSemaphore *tsem, FSTD_U64 value,
                                             const FSTD_Instant *timeout) {
    FSTD_U64 curr = atomic_load_explicit(&tsem->state, memory_order_relaxed);
    for (;;) {
        if (curr >= value) {
            (void)atomic_load_explicit(&tsem->state, memory_order_acquire);
            return true;
        }
        FTSK_FutexStatus status = ftsk_futex_wait(tsem, sizeof(*tsem), curr, value, timeout);
        if (status == FTSK_FutexStatus_Timeout)
            return false;
        if (status == FTSK_FutexStatus_Invalid) {
            curr = atomic_load_explicit(&tsem->state, memory_order_relaxed);
            continue;
        }
        return true;
    }
}

/// Blocks the caller until the semaphore reaches a count greater or equal to `value`.
fstd_util void ftsk_timeline_semaphore_wait(FTSK_TimelineSemaphore *tsem, FSTD_U64 value) {
    if (!ftsk_timeline_semaphore_is_signaled(tsem, value)) {
        ftsk__timeline_semaphore_wait(tsem, value, fstd_nullptr);
    }
}

/// Blocks the caller until the semaphore reaches a count greater or equal to `value`, or the timeout expires.
fstd_util bool ftsk_timeline_semaphore_timed_wait(FTSK_TimelineSemaphore *tsem, FSTD_U64 value, FSTD_Duration timeout) {
    if (!ftsk_timeline_semaphore_is_signaled(tsem, value)) {
        FSTD_Instant t = fstd_instant_add_saturating(fstd_instant_now(), timeout);
        return ftsk__timeline_semaphore_wait(tsem, value, &t);
    }
    return true;
}

/// Sets the internal value of the semaphore, possibly waking waiting tasks.
///
/// `value` must be greater than the current value of the semaphore.
fstd_util void ftsk_timeline_semaphore_signal(FTSK_TimelineSemaphore *tsem, FSTD_U64 value) {
    fstd_dbg_assert(atomic_load_explicit(&tsem->state, memory_order_relaxed) < value);
    atomic_store_explicit(&tsem->state, value, memory_order_release);
    // NOTE(gabriel): Wake all where the token is <= `value`.
    ftsk_futex_wake(tsem, ~(FSTD_USize)0,
                    FSTD_INIT(FTSK_FutexFilter){
                            .token_op = FTSK_FutexFilterOp_Noop,
                            .token_type = FTSK_FutexFilterTokenType_U64,
                            .cmp_op = FTSK_FutexFilterCmp_Le,
                            .cmp_arg_op = FTSK_FutexFilterOp_Noop,
                            .token_mask = ~(FSTD_USize)0,
                            .cmp_arg = value,
                    });
}

#define FTSK_SYM_NS "fimo-tasks"
#define FTSK__SYM_ID(name) FSTD_MODULE_SYMBOL_NS(name, FTSK_SYM_NS, FSTD_CTX_VERSION)
#define FTSK_SYM_ALL                                                                                                   \
    FTSK_Sym_TaskId, FTSK_Sym_WorkerId, FTSK_Sym_Yield, FTSK_Sym_Abort, FTSK_Sym_CancelRequested, FTSK_Sym_Sleep,      \
            FTSK_Sym_TaskLocalSet, FTSK_Sym_TaskLocalGet, FTSK_Sym_TaskLocalClear, FTSK_Sym_CmdBufJoin,                \
            FTSK_Sym_CmdBufDetach, FTSK_Sym_CmdBufCancel, FTSK_Sym_CmdBufCancelDetach, FTSK_Sym_ExecutorGlobal,        \
            FTSK_Sym_ExecutorInit, FTSK_Sym_ExecutorCurrent, FTSK_Sym_ExecutorJoin, FTSK_Sym_ExecutorJoinRequested,    \
            FTSK_Sym_ExecutorEnqueue, FTSK_Sym_ExecutorEnqueueDetached, FTSK_Sym_FutexWait, FTSK_Sym_FutexWaitv,       \
            FTSK_Sym_FutexWake, FTSK_Sym_FutexRequeue

FSTD_SYM_FN(FTSK_Sym_TaskId, FTSK__SYM_ID("task_id"), bool, FTSK_TaskId *id)
FSTD_SYM_FN(FTSK_Sym_WorkerId, FTSK__SYM_ID("worker_id"), bool, FTSK_Worker *id)
FSTD_SYM_FN(FTSK_Sym_Yield, FTSK__SYM_ID("yield"), void, void)
FSTD_SYM_FN(FTSK_Sym_Abort, FTSK__SYM_ID("abort"), void, void)
FSTD_SYM_FN(FTSK_Sym_CancelRequested, FTSK__SYM_ID("cancel_requested"), bool, void)
FSTD_SYM_FN(FTSK_Sym_Sleep, FTSK__SYM_ID("sleep"), void, FSTD_Duration duration)
FSTD_SYM_FN(FTSK_Sym_TaskLocalSet, FTSK__SYM_ID("task_local_set"), void, const FTSK_TssKey *key,
            void *FSTD_MAYBE_NULL value, FTSK_TssKeyDtor FSTD_MAYBE_NULL dtor)
FSTD_SYM_FN(FTSK_Sym_TaskLocalGet, FTSK__SYM_ID("task_local_get"), void *FSTD_MAYBE_NULL, const FTSK_TssKey *key)
FSTD_SYM_FN(FTSK_Sym_TaskLocalClear, FTSK__SYM_ID("task_local_clear"), void, const FTSK_TssKey *key)
FSTD_SYM_FN(FTSK_Sym_CmdBufJoin, FTSK__SYM_ID("cmd_buf_join"), FTSK_CmdBufHandleCompletionStatus,
            FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM_FN(FTSK_Sym_CmdBufDetach, FTSK__SYM_ID("cmd_buf_detach"), void, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM_FN(FTSK_Sym_CmdBufCancel, FTSK__SYM_ID("cmd_buf_cancel"), void, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM_FN(FTSK_Sym_CmdBufCancelDetach, FTSK__SYM_ID("cmd_buf_cancel_detach"), void, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM(FTSK_Sym_ExecutorGlobal, FTSK__SYM_ID("executor_global"), FTSK_Executor)
FSTD_SYM_FN(FTSK_Sym_ExecutorInit, FTSK__SYM_ID("executor_init"), FSTD_Status, FTSK_Executor **exe,
            const FTSK_ExecutorCfg *cfg)
FSTD_SYM_FN(FTSK_Sym_ExecutorCurrent, FTSK__SYM_ID("executor_current"), FTSK_Executor *FSTD_MAYBE_NULL, void)
FSTD_SYM_FN(FTSK_Sym_ExecutorJoin, FTSK__SYM_ID("executor_join"), void, FTSK_Executor *exe)
FSTD_SYM_FN(FTSK_Sym_ExecutorJoinRequested, FTSK__SYM_ID("executor_join_requested"), bool, FTSK_Executor *exe)
FSTD_SYM_FN(FTSK_Sym_ExecutorEnqueue, FTSK__SYM_ID("executor_enqueue"), FTSK_CmdBufHandle *, FTSK_Executor *exe,
            FTSK_CmdBuf *cmd_buf)
FSTD_SYM_FN(FTSK_Sym_ExecutorEnqueueDetached, FTSK__SYM_ID("executor_enqueue_detached"), void, FTSK_Executor *exe,
            FTSK_CmdBuf *cmd_buf)
FSTD_SYM_FN(FTSK_Sym_FutexWait, FTSK__SYM_ID("futex_wait"), FTSK_FutexStatus, const void *key, FSTD_USize key_size,
            FSTD_U64 expect, FSTD_USize token, const FSTD_Instant *FSTD_MAYBE_NULL timeout)
FSTD_SYM_FN(FTSK_Sym_FutexWaitv, FTSK__SYM_ID("futex_waitv"), FTSK_FutexStatus, FTSK_FutexKeyExpectSlice keys,
            const FSTD_Instant *FSTD_MAYBE_NULL timeout, FSTD_USize *wake_index)
FSTD_SYM_FN(FTSK_Sym_FutexWake, FTSK__SYM_ID("futex_wake"), FSTD_USize, const void *key, FSTD_USize max_waiters,
            FTSK_FutexFilter filter)
FSTD_SYM_FN(FTSK_Sym_FutexRequeue, FTSK__SYM_ID("futex_requeue"), FTSK_FutexStatus, const void *key_from,
            const void *key_to, FSTD_USize key_size, FSTD_U64 expect, FSTD_USize max_wakes, FSTD_USize max_requeues,
            FTSK_FutexFilter filter, FTSK_FutexRequeueResult *result)

#ifdef __cplusplus
}
#endif

#ifdef __cplusplus
namespace ftasks {
    namespace sym {
        constexpr static auto Namespace = FTSK_SYM_NS;
        constexpr static fstd::Version SymVersion = {FSTD_CTX_VERSION};
        constexpr static auto TaskId = FTSK_Sym_TaskId__Cxx;
        constexpr static auto WorkerId = FTSK_Sym_WorkerId__Cxx;
        constexpr static auto Yield = FTSK_Sym_Yield__Cxx;
        constexpr static auto Abort = FTSK_Sym_Abort__Cxx;
        constexpr static auto CancelRequested = FTSK_Sym_CancelRequested__Cxx;
        constexpr static auto Sleep = FTSK_Sym_Sleep__Cxx;
        constexpr static auto TaskLocalSet = FTSK_Sym_TaskLocalSet__Cxx;
        constexpr static auto TaskLocalGet = FTSK_Sym_TaskLocalGet__Cxx;
        constexpr static auto TaskLocalClear = FTSK_Sym_TaskLocalClear__Cxx;
        constexpr static auto CmdBufJoin = FTSK_Sym_CmdBufJoin__Cxx;
        constexpr static auto CmdBufDetach = FTSK_Sym_CmdBufDetach__Cxx;
        constexpr static auto CmdBufCancel = FTSK_Sym_CmdBufCancel__Cxx;
        constexpr static auto CmdBufCancelDetach = FTSK_Sym_CmdBufCancelDetach__Cxx;
        constexpr static auto ExecutorGlobal = FTSK_Sym_ExecutorGlobal__Cxx;
        constexpr static auto ExecutorInit = FTSK_Sym_ExecutorInit__Cxx;
        constexpr static auto ExecutorCurrent = FTSK_Sym_ExecutorCurrent__Cxx;
        constexpr static auto ExecutorJoin = FTSK_Sym_ExecutorJoin__Cxx;
        constexpr static auto ExecutorJoinRequested = FTSK_Sym_ExecutorJoinRequested__Cxx;
        constexpr static auto ExecutorEnqueue = FTSK_Sym_ExecutorEnqueue__Cxx;
        constexpr static auto ExecutorEnqueueDetached = FTSK_Sym_ExecutorEnqueueDetached__Cxx;
        constexpr static auto FutexWait = FTSK_Sym_FutexWait__Cxx;
        constexpr static auto FutexWaitv = FTSK_Sym_FutexWaitv__Cxx;
        constexpr static auto FutexWake = FTSK_Sym_FutexWake__Cxx;
        constexpr static auto FutexRequeue = FTSK_Sym_FutexRequeue__Cxx;

        constexpr static auto AllSymbols = fstd::modules::SymbolImportList{
                TaskId,
                WorkerId,
                Yield,
                Abort,
                CancelRequested,
                Sleep,
                TaskLocalSet,
                TaskLocalGet,
                TaskLocalClear,
                CmdBufJoin,
                CmdBufDetach,
                CmdBufCancel,
                CmdBufCancelDetach,
                ExecutorGlobal,
                ExecutorInit,
                ExecutorCurrent,
                ExecutorJoin,
                ExecutorJoinRequested,
                ExecutorEnqueue,
                ExecutorEnqueueDetached,
                FutexWait,
                FutexWaitv,
                FutexWake,
                FutexRequeue,
        };
    } // namespace sym

    /// Identifier of a task.
    struct TaskId {
        enum class Value : FTSK_TaskId {};

        Value value;

        std::optional<TaskId> current() noexcept {
            FTSK_TaskId id;
            if (!sym::TaskId.get()(&id))
                return std::nullopt;
            return TaskId{static_cast<Value>(id)};
        }
    };

    /// A unit of work.
    struct Worker {
        enum class Value : FTSK_Worker {};

        Value value;

        std::optional<Worker> current() noexcept {
            FTSK_Worker id;
            if (!sym::WorkerId.get()(&id))
                return std::nullopt;
            return Worker{static_cast<Value>(id)};
        }
    };

    struct Task : FTSK_Task {
        constexpr Task() noexcept = default;
        constexpr Task(void (*run)(FTSK_Task *, fstd::usize)) noexcept : Task({}, 1, run) {}
        constexpr Task(fstd::StrConst label, void (*run)(FTSK_Task *, fstd::usize)) noexcept : Task(label, 1, run) {}
        constexpr Task(fstd::usize batch_len, void (*run)(FTSK_Task *, fstd::usize)) noexcept :
            Task({}, batch_len, run) {}
        constexpr Task(fstd::StrConst label, fstd::usize batch_len, void (*run)(FTSK_Task *, fstd::usize)) noexcept :
            FTSK_Task{.label = label, .batch_len = batch_len, .run = run} {}
        constexpr explicit Task(const FTSK_Task &other) noexcept : FTSK_Task(other) {}
        constexpr Task(const Task &other) noexcept = default;
        constexpr Task(Task &&other) noexcept = default;
        constexpr Task &operator=(const Task &other) noexcept = default;
        constexpr Task &operator=(Task &&other) noexcept = default;
    };

    /// Yields the current task or thread back to the scheduler.
    inline static void yield() noexcept { return sym::Yield.get()(); }

    /// Aborts the current task.
    inline static void abort() noexcept { return sym::Abort.get()(); }

    /// Reports whether a cancellation of the current task has been requested.
    inline static bool cancelRequested() noexcept { return sym::CancelRequested.get()(); }

    /// Puts the current task or thread to sleep for the specified amount of time.
    inline static void sleep(fstd::Duration duration) noexcept { return sym::Sleep.get()(duration); }

    /// A key for a task-specific-storage.
    template<typename T>
    struct TssKey {
        constexpr TssKey() noexcept = default;
        constexpr TssKey(const TssKey &other) noexcept = delete;
        constexpr TssKey(TssKey &&other) noexcept = delete;
        constexpr TssKey &operator=(const TssKey &other) noexcept = delete;
        constexpr TssKey &operator=(TssKey &&other) noexcept = delete;

        /// Returns the value associated to the key for the current task.
        ///
        /// May only be called by a task.
        T *get() const noexcept {
            return static_cast<T *>(sym::TaskLocalGet.get()(reinterpret_cast<FTSK_TssKey *>(this)));
        }

        /// Associates a value with the key for the current task.
        ///
        /// The current value associated with the key is replaced with the new value without
        /// invoking any destructor function. The destructor function is set to `dtor`, and will
        /// be invoked upon task exit. May only be called by a task.
        void set(T *value, void (*dtor)(T *)) const noexcept {
            return sym::TaskLocalSet.get()(reinterpret_cast<FTSK_TssKey *>(this), value, dtor);
        }

        /// Clears the value of the current task associated with the key.
        ///
        /// This operation invokes the associated destructor function and sets the value to `null`.
        /// May only be called by a task.
        void clear() const noexcept { return sym::TaskLocalClear.get()(reinterpret_cast<FTSK_TssKey *>(this)); }
    };

    /// A list of commands to process by an executor.
    struct CmdBuf : FTSK_CmdBuf {
        /// An entry of a command buffer.
        struct Cmd : FTSK_CmdBufCmd {
            /// NOLINTNEXTLINE(performance-enum-size)
            enum class Tag : FTSK_CmdBufCmdTag {
                Noop = FTSK_CmdBufCmdTag_Noop,
                SelectWorker = FTSK_CmdBufCmdTag_SelectWorker,
                SelectAnyWorker = FTSK_CmdBufCmdTag_SelectAnyWorker,
                EnqueueTask = FTSK_CmdBufCmdTag_EnqueueTask,
                WaitOnBarrier = FTSK_CmdBufCmdTag_WaitOnBarrier,
                WaitOnCmdIndirect = FTSK_CmdBufCmdTag_WaitOnCmdIndirect,
            };

            struct NoopT {};
            constexpr static NoopT Noop{};

            struct SelectAnyWorkerT {};
            constexpr static SelectAnyWorkerT SelectAnyWorker{};

            struct WaitOnBarrierT {};
            constexpr static WaitOnBarrierT WaitOnBarrier{};

            struct WaitOnCmdIndirect {
                fstd::usize offset;
            };

            constexpr Cmd() noexcept = default;
            constexpr Cmd(NoopT) noexcept : Cmd() {}
            constexpr Cmd(Worker worker) noexcept :
                FTSK_CmdBufCmd{.tag = static_cast<FTSK_CmdBufCmdTag>(Tag::SelectWorker),
                               .select_worker = static_cast<FTSK_Worker>(worker.value)} {}
            constexpr Cmd(SelectAnyWorkerT) noexcept :
                FTSK_CmdBufCmd{.tag = static_cast<FTSK_CmdBufCmdTag>(Tag::SelectAnyWorker), .select_any_worker = 0} {}
            constexpr Cmd(Task &task) noexcept :
                FTSK_CmdBufCmd{.tag = static_cast<FTSK_CmdBufCmdTag>(Tag::EnqueueTask), .enqueue_task = &task} {}
            constexpr Cmd(WaitOnBarrierT) noexcept :
                FTSK_CmdBufCmd{.tag = static_cast<FTSK_CmdBufCmdTag>(Tag::WaitOnBarrier), .wait_on_barrier = 0} {}
            constexpr Cmd(WaitOnCmdIndirect cmd) noexcept :
                FTSK_CmdBufCmd{.tag = static_cast<FTSK_CmdBufCmdTag>(Tag::WaitOnCmdIndirect),
                               .wait_on_cmd_indirect = cmd.offset} {}
            constexpr explicit Cmd(const FTSK_CmdBufCmd &other) noexcept : FTSK_CmdBufCmd(other) {}
            constexpr Cmd(const Cmd &other) noexcept = default;
            constexpr Cmd(Cmd &&other) noexcept = default;
            constexpr Cmd &operator=(const Cmd &other) noexcept = default;
            constexpr Cmd &operator=(Cmd &&other) noexcept = default;
        };

        constexpr CmdBuf() noexcept = default;
        constexpr CmdBuf(fstd::Slice<Cmd> cmds) noexcept : CmdBuf({}, cmds, nullptr) {};
        constexpr CmdBuf(fstd::StrConst label, fstd::Slice<Cmd> cmds) noexcept : CmdBuf(label, cmds, nullptr) {};
        constexpr CmdBuf(fstd::Slice<Cmd> cmds, void (*dtor)(FTSK_CmdBuf *)) noexcept : CmdBuf({}, cmds, dtor) {};
        constexpr CmdBuf(fstd::StrConst label, fstd::Slice<Cmd> cmds, void (*dtor)(FTSK_CmdBuf *)) noexcept :
            FTSK_CmdBuf{.label = label, .cmds = cmds, .deinit = dtor} {};
        constexpr explicit CmdBuf(const FTSK_CmdBuf &other) noexcept : FTSK_CmdBuf(other) {}
        constexpr CmdBuf(const CmdBuf &other) noexcept = default;
        constexpr CmdBuf(CmdBuf &&other) noexcept = default;
        constexpr CmdBuf &operator=(const CmdBuf &other) noexcept = default;
        constexpr CmdBuf &operator=(CmdBuf &&other) noexcept = default;
    };

    /// A handle to an enqueued command buffer.
    struct CmdBufHandle {
        FTSK_CmdBufHandle *handle;

        constexpr CmdBufHandle() noexcept = default;
        constexpr explicit CmdBufHandle(FTSK_CmdBufHandle *other) noexcept : handle(other) {}
        constexpr CmdBufHandle(const CmdBufHandle &other) noexcept = default;
        constexpr CmdBufHandle(CmdBufHandle &&other) noexcept = default;
        constexpr CmdBufHandle &operator=(const CmdBufHandle &other) noexcept = default;
        constexpr CmdBufHandle &operator=(CmdBufHandle &&other) noexcept = default;

        // NOLINTNEXTLINE(performance-enum-size)
        enum class CompletionStatus : FTSK_CmdBufHandleCompletionStatus {
            Completed = FTSK_CmdBufHandleCompletionStatus_Completed,
            Cancelled = FTSK_CmdBufHandleCompletionStatus_Cancelled,
        };

        /// Waits for the command buffer to complete.
        ///
        /// Once called, the handle is consumed.
        CompletionStatus join() const noexcept { return static_cast<CompletionStatus>(sym::CmdBufJoin.get()(handle)); }

        /// Release the obligation of the caller to call join and
        /// have the handle be cleaned up on completion.
        ///
        /// Once called, the handle is consumed.
        void detach() const noexcept { return sym::CmdBufDetach.get()(handle); }

        /// Like `join`, but flags the handle as cancelled.
        void cancel() const noexcept { return sym::CmdBufCancel.get()(handle); }

        /// Like `detach`, but flags the handle as cancelled.
        void cancelDetach() const noexcept { return sym::CmdBufCancelDetach.get()(handle); }
    };

    /// A handle to an executor.
    struct Executor {
        FTSK_Executor *handle;

        constexpr Executor() noexcept = default;
        constexpr explicit Executor(FTSK_Executor *other) noexcept : handle(other) {}
        constexpr Executor(const Executor &other) noexcept = default;
        constexpr Executor(Executor &&other) noexcept = default;
        constexpr Executor &operator=(const Executor &other) noexcept = default;
        constexpr Executor &operator=(Executor &&other) noexcept = default;

        struct Cfg {
            /// Optional label of the executor.
            fstd::StrConst label;
            /// Maximum number of enqueued cmd buffers.
            ///
            /// A value of `0` indicates to use the default capacity.
            fstd::usize cmd_buf_capacity;
            /// Number of worker threads owned by the executor.
            ///
            /// A value of `0` indicates to use the default number of workers.
            fstd::usize worker_count;
            /// Controls the maximum number of spawned tasks.
            ///
            /// The maximum number of spawned tasks is determined as `worker_count * max_load_factor`.
            /// A value of `0` indicates to use the default load factor.
            fstd::usize max_load_factor;
            /// Minimum stack size in bytes.
            ///
            /// A value of `0` indicates to use the default stack size.
            fstd::usize stack_size;
            /// Minimum size of the per-task arena.
            ///
            /// A value of `0` indicates to use the default arena size.
            fstd::usize arena_size;
            /// Number of cached stacks per worker.
            ///
            /// The cache is shared among all workers.
            /// A value of `0` indicates to use the default cache length.
            fstd::usize worker_stack_cache_len;
            /// Indicates whether to disable the stack cache.
            bool disable_stack_cache;
        };

        /// Returns the global executor.
        static Executor global() noexcept { return Executor{const_cast<FTSK_Executor *>(&sym::ExecutorGlobal.get())}; }

        /// Creates a new executor with the provided configuration.
        static std::expected<Executor, fstd::Status> init(const Cfg &cfg) noexcept {
            FTSK_ExecutorCfg ccfg = {
                    .label = cfg.label,
                    .cmd_buf_capacity = cfg.cmd_buf_capacity,
                    .worker_count = cfg.worker_count,
                    .max_load_factor = cfg.max_load_factor,
                    .stack_size = cfg.stack_size,
                    .arena_size = cfg.arena_size,
                    .worker_stack_cache_len = cfg.worker_stack_cache_len,
                    .disable_stack_cache = cfg.disable_stack_cache,
            };
            Executor exe{};
            fstd::Status status = static_cast<fstd::Status>(sym::ExecutorInit.get()(&exe.handle, &ccfg));
            if (status != fstd::Status::Ok)
                return std::unexpected(status);
            return exe;
        }

        /// Returns the executor for the current context.
        ///
        /// Is only valid for the duration of the current context (i.e. Task).
        static std::optional<Executor> current() noexcept {
            Executor exe{sym::ExecutorCurrent.get()()};
            if (!exe.handle)
                return std::nullopt;
            return exe;
        }

        /// Waits until all remaining commands have been executed and consumes the handle.
        ///
        /// New commands can be enqueued to the executor while the call is in process.
        void join() const noexcept { return sym::ExecutorJoin.get()(handle); }

        /// Reports whether the owner of the executor has requested that the executor be joined.
        bool joinRequested() const noexcept { return sym::ExecutorJoinRequested.get()(handle); }

        /// Enqueues the commands to the executor.
        ///
        /// The caller will block until the handle could be enqueued.
        /// The buffer must outlive the returned handle.
        CmdBufHandle enqueue(CmdBuf &cmd_buf) const noexcept {
            return CmdBufHandle{sym::ExecutorEnqueue.get()(handle, &cmd_buf)};
        }

        /// Enqueues the commands to the executor.
        ///
        /// The caller will block until the handle could be enqueued.
        /// The buffer must outlive the returned handle.
        void enqueueDetached(CmdBuf &cmd_buf) const noexcept {
            return sym::ExecutorEnqueueDetached.get()(handle, &cmd_buf);
        }

        /// Invokes the callable on each element of the slice.
        ///
        /// The slice is split up in chunks of `batch_size` length, which are possibly
        /// processed in parallel. The `threshold` specifies the minimum number of elements
        /// required to switch over to parallel processing. It is recommended that the
        /// `threshold` be set to a multiple of the `batch_size`.
        template<typename T, fstd::InvocableWithReturn<void, T &> Callable>
        void forEach(fstd::Slice<T> elements, fstd::usize batch_size, fstd::usize threshold,
                     Callable &&f) const noexcept {
            if (elements.size() < threshold) {
                for (auto &element: elements) {
                    std::invoke(f, element);
                }
                return;
            }

            fstd_dbg_assert(batch_size != 0);
            struct Context {
                fstd::usize batch_size;
                fstd::Slice<T> elements;
                Callable f;
                Task task;

                static void run(FTSK_Task *task, fstd::usize index) noexcept {
                    Context &ctx = *fstd::parentOf(static_cast<Task *>(task), fstd::ConstexprValue<&Context::task>{});
                    fstd::usize start_idx = index * ctx.batch_size;
                    fstd::usize end_idx = start_idx + ctx.batch_size;
                    for (fstd::usize i = start_idx; i < end_idx; i++) {
                        std::invoke(ctx.f, ctx.elements[i]);
                    }
                }
            };
            fstd::usize num_batches = 1 + ((elements.size() - 1) / batch_size);
            Context ctx = {
                    .batch_size = batch_size,
                    .elements = elements,
                    .f = std::forward<Callable>(f),
                    .task =
                            {
                                    num_batches,
                                    Context::run,
                            },
            };

            std::array<CmdBuf::Cmd, 1> cmd = {{ctx.task}};
            CmdBuf cmd_buf = {cmd};
            enqueue(cmd_buf).join();
        }

        /// Invokes the callable on each element of the slice.
        ///
        /// The slice is split up in chunks of `batch_size` length, which are possibly
        /// processed in parallel.
        template<typename T, fstd::InvocableWithReturn<void, T &> Callable>
        void forEach(fstd::Slice<T> elements, fstd::usize batch_size, Callable &&f) const noexcept {
            return forEach(elements, batch_size, batch_size, std::forward<Callable>(f));
        }
    };

    namespace futex {
        // NOLINTNEXTLINE(performance-enum-size)
        enum class Status : FTSK_FutexStatus {
            Ok = FTSK_FutexStatus_Ok,
            Invalid = FTSK_FutexStatus_Invalid,
            Timeout = FTSK_FutexStatus_Timeout,
            KeyError = FTSK_FutexStatus_KeyError,
        };

        using KeyExpect = FTSK_FutexKeyExpect;
        using Filter = FTSK_FutexFilter;
        constexpr static Filter All = FTSK_FUTEX_FILTER_ALL;

        using RequeueResult = FTSK_FutexRequeueResult;

        static_assert(std::atomic<fstd::u8>::is_always_lock_free);
        static_assert(std::atomic<fstd::u16>::is_always_lock_free);
        static_assert(std::atomic<fstd::u32>::is_always_lock_free);
        static_assert(std::atomic<fstd::u64>::is_always_lock_free);
        static_assert(std::atomic<fstd::i8>::is_always_lock_free);
        static_assert(std::atomic<fstd::i16>::is_always_lock_free);
        static_assert(std::atomic<fstd::i32>::is_always_lock_free);
        static_assert(std::atomic<fstd::i64>::is_always_lock_free);

        template<typename T, typename U>
        concept Awaitable = (sizeof(T) <= sizeof(fstd::u64)) and std::integral<U> and
                            (std::same_as<T, U> or std::same_as<T, std::atomic<U>>);

        /// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
        ///
        /// If the value does not match, the function returns imediately with `Status::Invalid`. The
        /// `key_size` parameter specifies the size of the value in bytes and must be either of `1`, `2`,
        /// `4` or `8`, in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or
        /// `u64` respectively, and `expect` is truncated. The `token` is a user definable integer to store
        /// additional metadata about the waiter, which can be utilized to controll some wake operations.
        static inline Status wait(const void *key, fstd::usize key_size, fstd::u64 expect, fstd::usize token,
                                  const fstd::Instant &timeout) noexcept {
            return static_cast<Status>(sym::FutexWait.get()(key, key_size, expect, token, &timeout));
        }

        /// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
        ///
        /// If the value does not match, the function returns imediately with `Status::Invalid`. The
        /// `key_size` parameter specifies the size of the value in bytes and must be either of `1`, `2`,
        /// `4` or `8`, in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or
        /// `u64` respectively, and `expect` is truncated.
        static inline Status wait(const void *key, fstd::usize key_size, fstd::u64 expect,
                                  const fstd::Instant &timeout) noexcept {
            return wait(key, key_size, expect, 0, timeout);
        }

        template<std::integral T, Awaitable<T> A>
        static inline Status wait(const A &value, T expect, fstd::usize token, const fstd::Instant &timeout) noexcept {
            return wait(&value, sizeof(T), std::bit_cast<std::make_unsigned_t<T>>(expect), token, timeout);
        }

        template<std::integral T, Awaitable<T> A>
        static inline Status wait(const A &value, T expect, const fstd::Instant &timeout) noexcept {
            return wait(&value, sizeof(T), std::bit_cast<std::make_unsigned_t<T>>(expect), timeout);
        }

        /// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
        ///
        /// If the value does not match, the function returns imediately with `Status::Invalid`. The
        /// `key_size` parameter specifies the size of the value in bytes and must be either of `1`, `2`,
        /// `4` or `8`, in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or
        /// `u64` respectively, and `expect` is truncated. The `token` is a user definable integer to store
        /// additional metadata about the waiter, which can be utilized to controll some wake operations.
        static inline Status wait(const void *key, fstd::usize key_size, fstd::u64 expect, fstd::usize token) noexcept {
            return static_cast<Status>(sym::FutexWait.get()(key, key_size, expect, token, nullptr));
        }

        /// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
        ///
        /// If the value does not match, the function returns imediately with `Status::Invalid`. The
        /// `key_size` parameter specifies the size of the value in bytes and must be either of `1`, `2`,
        /// `4` or `8`, in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or
        /// `u64` respectively, and `expect` is truncated.
        static inline Status wait(const void *key, fstd::usize key_size, fstd::u64 expect) noexcept {
            return wait(key, key_size, expect, 0);
        }

        template<std::integral T, Awaitable<T> A>
        static inline Status wait(const A &value, T expect, fstd::usize token) noexcept {
            return wait(&value, sizeof(T), std::bit_cast<std::make_unsigned_t<T>>(expect), token);
        }

        template<std::integral T, Awaitable<T> A>
        static inline Status wait(const A &value, T expect) noexcept {
            return wait(&value, sizeof(T), std::bit_cast<std::make_unsigned_t<T>>(expect));
        }

        /// Puts the caller to sleep if all keys match their expected values.
        ///
        /// Is a generalization of `wait` for multiple keys. At least `1` key must, and at most
        /// `max_waitv_key_count` may be passed to this function. Otherwise it returns `Status::KeyError`.
        static inline std::expected<fstd::usize, Status> waitv(fstd::Slice<const KeyExpect> keys,
                                                               const fstd::Instant &timeout) noexcept {
            fstd::usize wake_idx;
            Status status = static_cast<Status>(sym::FutexWaitv.get()(keys, &timeout, &wake_idx));
            if (status != Status::Ok)
                return std::unexpected(status);
            return wake_idx;
        }

        /// Puts the caller to sleep if all keys match their expected values.
        ///
        /// Is a generalization of `wait` for multiple keys. At least `1` key must, and at most
        /// `max_waitv_key_count` may be passed to this function. Otherwise it returns `Status::KeyError`.
        static inline std::expected<fstd::usize, Status> waitv(fstd::Slice<const KeyExpect> keys) noexcept {
            fstd::usize wake_idx;
            Status status = static_cast<Status>(sym::FutexWaitv.get()(keys, nullptr, &wake_idx));
            if (status != Status::Ok)
                return std::unexpected(status);
            return wake_idx;
        }

        /// Wakes at most `max_waiters` waiting on `key`.
        ///
        /// Uses the token provided by the waiter and the `filter` to determine whether to ignore it from
        /// being woken up. Returns the number of woken waiters.
        static inline fstd::usize wake(const void *key, fstd::usize max_waiters, Filter filter) noexcept {
            return sym::FutexWake.get()(key, max_waiters, filter);
        }

        /// Wakes at most `max_waiters` waiting on `key`.
        ///
        /// Returns the number of woken waiters.
        static inline fstd::usize wake(const void *key, fstd::usize max_waiters) noexcept {
            return wake(key, max_waiters, All);
        }

        /// Wakes all waiters waiting on `key`.
        ///
        /// Uses the token provided by the waiter and the `filter` to determine whether to ignore it from
        /// being woken up. Returns the number of woken waiters.
        static inline fstd::usize wake(const void *key, Filter filter) noexcept {
            return wake(key, std::numeric_limits<fstd::usize>::max(), filter);
        }

        /// Wakes all waiters waiting on `key`.
        ///
        /// Returns the number of woken waiters.
        static inline fstd::usize wake(const void *key) noexcept {
            return wake(key, std::numeric_limits<fstd::usize>::max());
        }

        /// Requeues waiters from `key_from` to `key_to`.
        ///
        /// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
        /// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
        /// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
        /// the function returns `Status::Invalid`. Uses the token provided by the waiter and the `filter`
        /// to determine whether to ignore it from being woken up.
        static inline std::expected<RequeueResult, Status> requeue(const void *key_from, const void *key_to,
                                                                   fstd::usize key_size, fstd::u64 expect,
                                                                   fstd::usize max_wakes, fstd::usize max_requeues,
                                                                   Filter filter) noexcept {
            RequeueResult result;
            Status status = static_cast<Status>(sym::FutexRequeue.get()(key_from, key_to, key_size, expect, max_wakes,
                                                                        max_requeues, filter, &result));
            if (status != Status::Ok)
                return std::unexpected(status);
            return result;
        }

        /// Requeues waiters from `key_from` to `key_to`.
        ///
        /// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
        /// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
        /// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
        /// the function returns `Status::Invalid`.
        static inline std::expected<RequeueResult, Status> requeue(const void *key_from, const void *key_to,
                                                                   fstd::usize key_size, fstd::u64 expect,
                                                                   fstd::usize max_wakes,
                                                                   fstd::usize max_requeues) noexcept {
            return requeue(key_from, key_to, key_size, expect, max_wakes, max_requeues, All);
        }

        template<std::integral T, Awaitable<T> A>
        static inline std::expected<RequeueResult, Status> requeue(const A &from, const A &to, T expect,
                                                                   fstd::usize max_wakes, fstd::usize max_requeues,
                                                                   Filter filter) noexcept {
            return requeue(&from, &to, std::bit_cast<std::make_unsigned_t<T>>(expect), max_wakes, max_requeues, filter);
        }

        template<std::integral T, Awaitable<T> A>
        static inline std::expected<RequeueResult, Status>
        requeue(const A &from, const A &to, T expect, fstd::usize max_wakes, fstd::usize max_requeues) noexcept {
            return requeue<T, A>(&from, &to, expect, max_wakes, max_requeues, All);
        }
    } // namespace futex

    /// Mutex is a synchronization primitive which enforces atomic access to a
    /// shared region of code known as the "critical section".
    ///
    /// It does this by blocking ensuring only one task is in the critical
    /// section at any given point in time by blocking the others.
    struct Mutex : FTSK_Mutex {
        constexpr Mutex() noexcept = default;
        constexpr Mutex(const FTSK_Mutex &other) noexcept :
            FTSK_Mutex{.state = other.state.load(std::memory_order_relaxed)} {}
        constexpr Mutex(const Mutex &other) noexcept : Mutex(static_cast<const FTSK_Mutex &>(other)) {}
        constexpr Mutex(Mutex &&other) noexcept : Mutex(static_cast<const FTSK_Mutex &>(other)) {}
        constexpr Mutex &operator=(const Mutex &other) noexcept {
            if (this != &other) {
                this->state = other.state.load(std::memory_order_relaxed);
            }
            return *this;
        }
        constexpr Mutex &operator=(Mutex &&other) noexcept {
            if (this != &other) {
                this->state = other.state.load(std::memory_order_relaxed);
            }
            return *this;
        }

        /// Tries to acquire the mutex without blocking the caller's task.
        ///
        /// Returns `false` if the calling task would have to block to acquire it.
        /// Otherwise, returns `true` and the caller should `unlock()` the Mutex to release it.
        bool tryLock() noexcept { return ftsk_mutex_try_lock(this); }

        /// Acquires the mutex, blocking the caller's task until it can.
        ///
        /// Once acquired, call `unlock()` on the Mutex to release it.
        void lock() noexcept { return ftsk_mutex_lock(this); }

        /// Tries to acquire the mutex, blocking the caller's task until it can or the timeout is reached.
        ///
        /// Returns `true` if the lock could be acquired.
        /// Once acquired, call `unlock()` on the Mutex to release it.
        bool tryLockFor(const fstd::Duration &timeout) noexcept { return ftsk_mutex_timed_lock(this, timeout); }

        /// Releases the mutex which was previously acquired.
        void unlock() noexcept { return ftsk_mutex_unlock(this); }
    };

    /// Condition variables are used with a Mutex to efficiently wait for an arbitrary condition to occur.
    /// It does this by atomically unlocking the mutex, blocking the thread until notified, and finally re-locking the
    /// mutex.
    struct Condition : FTSK_Condition {
        constexpr Condition() noexcept = default;
        constexpr Condition(const FTSK_Condition &other) noexcept :
            FTSK_Condition{.futex = other.futex.load(std::memory_order_relaxed)} {}
        constexpr Condition(const Condition &other) noexcept : Condition(static_cast<const FTSK_Condition &>(other)) {}
        constexpr Condition(Condition &&other) noexcept : Condition(static_cast<const FTSK_Condition &>(other)) {}
        constexpr Condition &operator=(const Condition &other) noexcept {
            if (this != &other) {
                this->futex = other.futex.load(std::memory_order_relaxed);
            }
            return *this;
        }
        constexpr Condition &operator=(Condition &&other) noexcept {
            if (this != &other) {
                this->futex = other.futex.load(std::memory_order_relaxed);
            }
            return *this;
        }

        /// Atomically releases the Mutex, blocks the caller task, then re-acquires the Mutex on return.
        /// "Atomically" here refers to accesses done on the Condition after acquiring the Mutex.
        ///
        /// The Mutex must be locked by the caller's task when this function is called.
        /// A Mutex can have multiple Conditions waiting with it concurrently, but not the opposite.
        /// It is undefined behavior for multiple tasks to wait with different mutexes using the same Condition
        /// concurrently. Once tasks have finished waiting with one Mutex, the Condition can be used to wait with
        /// another Mutex.
        ///
        /// A blocking call to wait() is unblocked from one of the following conditions:
        /// - a spurious ("at random") wake up occurs
        /// - a future call to `signal()` or `broadcast()` which has acquired the Mutex and is sequenced after this
        /// `wait()`.
        ///
        /// Given wait() can be interrupted spuriously, the blocking condition should be checked continuously
        /// irrespective of any notifications from `signal()` or `broadcast()`.
        void wait(Mutex &mutex) noexcept { return ftsk_condition_wait(this, &mutex); }

        /// Atomically releases the Mutex, blocks the caller task, then re-acquires the Mutex on return.
        /// "Atomically" here refers to accesses done on the Condition after acquiring the Mutex.
        ///
        /// The Mutex must be locked by the caller's task when this function is called.
        /// A Mutex can have multiple Conditions waiting with it concurrently, but not the opposite.
        /// It is undefined behavior for multiple tasks to wait with different mutexes using the same Condition
        /// concurrently. Once tasks have finished waiting with one Mutex, the Condition can be used to wait with
        /// another Mutex.
        ///
        /// A blocking call to `waitFor()` is unblocked from one of the following conditions:
        /// - a spurious ("at random") wake occurs
        /// - the caller was blocked for around `timeout`, in which `error.Timeout` is returned.
        /// - a future call to `signal()` or `broadcast()` which has acquired the Mutex and is sequenced after this
        /// `waitFor()`.
        ///
        /// Given `waitFor()` can be interrupted spuriously, the blocking condition should be checked continuously
        /// irrespective of any notifications from `signal()` or `broadcast()`.
        ///
        /// Returns `true` if the caller was woken up before the timeout elapsed.
        bool waitFor(Mutex &mutex, const fstd::Duration &timeout) noexcept {
            return ftsk_condition_timed_wait(this, &mutex, timeout);
        }

        /// Unblocks at least one task blocked in a call to `wait()` or `waitFor()` with a given Mutex.
        /// The blocked task must be sequenced before this call with respect to acquiring the same Mutex in order to be
        /// observable for unblocking. `signal()` can be called with or without the relevant Mutex being acquired and
        /// have no "effect" if there's no observable blocked threads.
        void signal() noexcept { return ftsk_condition_signal(this); }

        /// Unblocks all tasks currently blocked in a call to `wait()` or `waitFor()` with a given Mutex.
        /// The blocked tasks must be sequenced before this call with respect to acquiring the same Mutex in order to be
        /// observable for unblocking. `broadcast()` can be called with or without the relevant Mutex being acquired and
        /// have no "effect" if there's no observable blocked threads.
        void broadcast() noexcept { return ftsk_condition_broadcast(this); }
    };

    /// A thread-safe boolean that can be set and awaited,
    /// typically to mark/await the completion of some operation.
    struct Fence : FTSK_Fence {
        constexpr Fence() noexcept = default;
        constexpr Fence(const FTSK_Fence &other) noexcept :
            FTSK_Fence{.state = other.state.load(std::memory_order_relaxed)} {}
        constexpr Fence(const Fence &other) noexcept : Fence(static_cast<const FTSK_Fence &>(other)) {}
        constexpr Fence(Fence &&other) noexcept : Fence(static_cast<const FTSK_Fence &>(other)) {}
        constexpr Fence &operator=(const Fence &other) noexcept {
            if (this != &other) {
                this->state = other.state.load(std::memory_order_relaxed);
            }
            return *this;
        }
        constexpr Fence &operator=(Fence &&other) noexcept {
            if (this != &other) {
                this->state = other.state.load(std::memory_order_relaxed);
            }
            return *this;
        }

        /// Checks if the fence is already signaled.
        bool isSignaled() noexcept { return ftsk_fence_is_signaled(this); }

        /// Blocks the caller until the fence is signaled.
        void wait() noexcept { return ftsk_fence_wait(this); }

        /// Blocks the caller until the fence is signaled, or the timeout expires.
        bool waitFor(const fstd::Duration &timeout) noexcept { return ftsk_fence_timed_wait(this, timeout); }

        /// Wakes all waiters of the fence.
        void signal() noexcept { return ftsk_fence_signal(this); }

        /// Resets the state of the fence to be unsignaled.
        ///
        /// May not be called while threads are waiting on the fence.
        void reset() noexcept { return ftsk_fence_reset(this); }
    };

    /// A monotonically increasing counter that can be awaited and signaled.
    struct TimelineSemaphore : FTSK_TimelineSemaphore {
        constexpr TimelineSemaphore() noexcept = default;
        constexpr TimelineSemaphore(fstd::u64 counter) noexcept : FTSK_TimelineSemaphore{.state = counter} {};
        constexpr TimelineSemaphore(const FTSK_TimelineSemaphore &other) noexcept :
            FTSK_TimelineSemaphore{.state = other.state.load(std::memory_order_relaxed)} {}
        constexpr TimelineSemaphore(const TimelineSemaphore &other) noexcept :
            TimelineSemaphore(static_cast<const FTSK_TimelineSemaphore &>(other)) {}
        constexpr TimelineSemaphore(TimelineSemaphore &&other) noexcept :
            TimelineSemaphore(static_cast<const FTSK_TimelineSemaphore &>(other)) {}
        constexpr TimelineSemaphore &operator=(const TimelineSemaphore &other) noexcept {
            if (this != &other) {
                this->state = other.state.load(std::memory_order_relaxed);
            }
            return *this;
        }
        constexpr TimelineSemaphore &operator=(TimelineSemaphore &&other) noexcept {
            if (this != &other) {
                this->state = other.state.load(std::memory_order_relaxed);
            }
            return *this;
        }

        /// Returns the current counter of the semaphore.
        fstd::u64 counter() noexcept { return ftsk_timeline_semaphore_counter(this); }

        /// Checks if the semaphore is signaled with a count greater or equal to `value`.
        bool isSignaled(fstd::u64 value) noexcept { return ftsk_timeline_semaphore_is_signaled(this, value); }

        /// Blocks the caller until the semaphore reaches a count greater or equal to `value`.
        void wait(fstd::u64 value) noexcept { return ftsk_timeline_semaphore_wait(this, value); }

        /// Blocks the caller until the semaphore reaches a count greater or equal to `value`, or the timeout expires.
        bool waitFor(fstd::u64 value, const fstd::Duration &timeout) noexcept {
            return ftsk_timeline_semaphore_timed_wait(this, value, timeout);
        }

        /// Sets the internal value of the semaphore, possibly waking waiting tasks.
        ///
        /// `value` must be greater than the current value of the semaphore.
        void signal(fstd::u64 value) noexcept { return ftsk_timeline_semaphore_signal(this, value); }
    };
} // namespace ftasks

#endif

#ifdef FIMO_TASKS_IMPLEMENTATION

#ifdef __cplusplus
extern "C" {
#endif

FSTD_SYM_FN_IMP(FTSK_Sym_TaskId, bool, FTSK_TaskId *id)
FSTD_SYM_FN_IMP(FTSK_Sym_WorkerId, bool, FTSK_Worker *id)
FSTD_SYM_FN_IMP(FTSK_Sym_Yield, void, void)
FSTD_SYM_FN_IMP(FTSK_Sym_Abort, void, void)
FSTD_SYM_FN_IMP(FTSK_Sym_CancelRequested, bool, void)
FSTD_SYM_FN_IMP(FTSK_Sym_Sleep, void, FSTD_Duration duration)
FSTD_SYM_FN_IMP(FTSK_Sym_TaskLocalSet, void, const FTSK_TssKey *key, void *FSTD_MAYBE_NULL value,
                FTSK_TssKeyDtor FSTD_MAYBE_NULL dtor)
FSTD_SYM_FN_IMP(FTSK_Sym_TaskLocalGet, void *FSTD_MAYBE_NULL, const FTSK_TssKey *key)
FSTD_SYM_FN_IMP(FTSK_Sym_TaskLocalClear, void, const FTSK_TssKey *key)
FSTD_SYM_FN_IMP(FTSK_Sym_CmdBufJoin, FTSK_CmdBufHandleCompletionStatus, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM_FN_IMP(FTSK_Sym_CmdBufDetach, void, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM_FN_IMP(FTSK_Sym_CmdBufCancel, void, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM_FN_IMP(FTSK_Sym_CmdBufCancelDetach, void, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYM_IMP(FTSK_Sym_ExecutorGlobal, FTSK_Executor)
FSTD_SYM_FN_IMP(FTSK_Sym_ExecutorInit, FSTD_Status, FTSK_Executor **exe, const FTSK_ExecutorCfg *cfg)
FSTD_SYM_FN_IMP(FTSK_Sym_ExecutorCurrent, FTSK_Executor *FSTD_MAYBE_NULL, void)
FSTD_SYM_FN_IMP(FTSK_Sym_ExecutorJoin, void, FTSK_Executor *exe)
FSTD_SYM_FN_IMP(FTSK_Sym_ExecutorJoinRequested, bool, FTSK_Executor *exe)
FSTD_SYM_FN_IMP(FTSK_Sym_ExecutorEnqueue, FTSK_CmdBufHandle *, FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf)
FSTD_SYM_FN_IMP(FTSK_Sym_ExecutorEnqueueDetached, void, FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf)
FSTD_SYM_FN_IMP(FTSK_Sym_FutexWait, FTSK_FutexStatus, const void *key, FSTD_USize key_size, FSTD_U64 expect,
                FSTD_USize token, const FSTD_Instant *FSTD_MAYBE_NULL timeout)
FSTD_SYM_FN_IMP(FTSK_Sym_FutexWaitv, FTSK_FutexStatus, FTSK_FutexKeyExpectSlice keys,
                const FSTD_Instant *FSTD_MAYBE_NULL timeout, FSTD_USize *wake_index)
FSTD_SYM_FN_IMP(FTSK_Sym_FutexWake, FSTD_USize, const void *key, FSTD_USize max_waiters, FTSK_FutexFilter filter)
FSTD_SYM_FN_IMP(FTSK_Sym_FutexRequeue, FTSK_FutexStatus, const void *key_from, const void *key_to, FSTD_USize key_size,
                FSTD_U64 expect, FSTD_USize max_wakes, FSTD_USize max_requeues, FTSK_FutexFilter filter,
                FTSK_FutexRequeueResult *result)

fstd_func_impl bool ftsk_task_id_current(FTSK_TaskId *id) { return FTSK_Sym_TaskId__get()(id); }

fstd_func_impl bool ftsk_worker_id_current(FTSK_Worker *id) { return FTSK_Sym_WorkerId__get()(id); }

fstd_func_impl void ftsk_yield(void) { FTSK_Sym_Yield__get()(); }

fstd_func_impl void ftsk_abort(void) { FTSK_Sym_Abort__get()(); }

fstd_func_impl bool ftsk_cancel_requested(void) { return FTSK_Sym_CancelRequested__get()(); }

fstd_func_impl void ftsk_sleep(FSTD_Duration duration) { FTSK_Sym_Sleep__get()(duration); }

fstd_func_impl void ftsk_tss_key_set(const FTSK_TssKey *key, void *FSTD_MAYBE_NULL value,
                                     FTSK_TssKeyDtor FSTD_MAYBE_NULL dtor) {
    FTSK_Sym_TaskLocalSet__get()(key, value, dtor);
}

fstd_func_impl void *FSTD_MAYBE_NULL ftsk_tss_key_get(const FTSK_TssKey *key) {
    return FTSK_Sym_TaskLocalGet__get()(key);
}

fstd_func_impl void ftsk_tss_key_clear(const FTSK_TssKey *key) { FTSK_Sym_TaskLocalClear__get()(key); }

fstd_func_impl FTSK_CmdBufHandleCompletionStatus ftsk_cmd_buf_handle_join(FTSK_CmdBufHandle *cmd_buf) {
    return FTSK_Sym_CmdBufJoin__get()(cmd_buf);
}

fstd_func_impl void ftsk_cmd_buf_handle_detach(FTSK_CmdBufHandle *cmd_buf) { FTSK_Sym_CmdBufDetach__get()(cmd_buf); }

fstd_func_impl void ftsk_cmd_buf_handle_cancel(FTSK_CmdBufHandle *cmd_buf) { FTSK_Sym_CmdBufCancel__get()(cmd_buf); }

fstd_func_impl void ftsk_cmd_buf_handle_cancel_detach(FTSK_CmdBufHandle *cmd_buf) {
    FTSK_Sym_CmdBufCancelDetach__get()(cmd_buf);
}

fstd_func FTSK_Executor *ftsk_global_executor() { return (FTSK_Executor *)FTSK_Sym_ExecutorGlobal__get(); }

fstd_func FSTD_Status ftsk_executor_init(FTSK_Executor **exe, const FTSK_ExecutorCfg *cfg) {
    return FTSK_Sym_ExecutorInit__get()(exe, cfg);
}

fstd_func FTSK_Executor *ftsk_executor_current() { return FTSK_Sym_ExecutorCurrent__get()(); }

fstd_func void ftsk_executor_join(FTSK_Executor *exe) { FTSK_Sym_ExecutorJoin__get()(exe); }

fstd_func bool ftsk_executor_join_requested(FTSK_Executor *exe) { return FTSK_Sym_ExecutorJoinRequested__get()(exe); }

fstd_func FTSK_CmdBufHandle *ftsk_executor_enqueue(FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf) {
    return FTSK_Sym_ExecutorEnqueue__get()(exe, cmd_buf);
}

fstd_func void ftsk_executor_enqueue_detached(FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf) {
    FTSK_Sym_ExecutorEnqueueDetached__get()(exe, cmd_buf);
}

fstd_func FTSK_FutexStatus ftsk_futex_wait(const void *key, FSTD_USize key_size, FSTD_U64 expect, FSTD_USize token,
                                           const FSTD_Instant *FSTD_MAYBE_NULL timeout) {
    return FTSK_Sym_FutexWait__get()(key, key_size, expect, token, timeout);
}

fstd_func FTSK_FutexStatus ftsk_futex_waitv(FTSK_FutexKeyExpectSlice keys, const FSTD_Instant *FSTD_MAYBE_NULL timeout,
                                            FSTD_USize *wake_index) {
    return FTSK_Sym_FutexWaitv__get()(keys, timeout, wake_index);
}

fstd_func FSTD_USize ftsk_futex_wake(const void *key, FSTD_USize max_waiters, FTSK_FutexFilter filter) {
    return FTSK_Sym_FutexWake__get()(key, max_waiters, filter);
}

fstd_func FTSK_FutexStatus ftsk_futex_requeue(const void *key_from, const void *key_to, FSTD_USize key_size,
                                              FSTD_U64 expect, FSTD_USize max_wakes, FSTD_USize max_requeues,
                                              FTSK_FutexFilter filter, FTSK_FutexRequeueResult *result) {
    return FTSK_Sym_FutexRequeue__get()(key_from, key_to, key_size, expect, max_wakes, max_requeues, filter, result);
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_IMPLEMENTATION

#endif // FIMO_TASKS_HEADER

/// LICENSE
/// MIT License
///
/// Copyright (c) 2025 Gabriel Borrelli
///
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
///
/// The above copyright notice and this permission notice shall be included in all
/// copies or substantial portions of the Software.
///
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
/// SOFTWARE.
