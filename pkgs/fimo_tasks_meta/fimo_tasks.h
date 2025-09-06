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
    FTSK_CmdBufCmdTag_SelectWorker = (FTSK_CmdBufCmdTag)0,
    FTSK_CmdBufCmdTag_SelectAnyWorker = (FTSK_CmdBufCmdTag)1,
    FTSK_CmdBufCmdTag_EnqueueTask = (FTSK_CmdBufCmdTag)2,
    FTSK_CmdBufCmdTag_WaitOnBarrier = (FTSK_CmdBufCmdTag)3,
    FTSK_CmdBufCmdTag_WaitOnCmdIndirect = (FTSK_CmdBufCmdTag)4,
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
fstd_util bool ftsk__mutex_lock_contended(FTSK_Mutex *mutex, FSTD_Instant timeout) {
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

        FTSK_FutexStatus status = ftsk_futex_wait(mutex, sizeof(*mutex), FTSK__MUTEX_CONTENDED, 0, &timeout);
        if (status == FTSK_FutexStatus_Timeout)
            return false;
    }
}

/// Acquires the mutex, blocking the caller's task until it can.
///
/// Once acquired, call `unlock()` on the Mutex to release it.
fstd_util void ftsk_mutex_lock(FTSK_Mutex *mutex) {
    if (!ftsk_mutex_try_lock(mutex)) {
        ftsk__mutex_lock_contended(mutex, FSTD_INIT(FSTD_Instant) FSTD_INSTANT_MAX);
    }
}

/// Tries to acquire the mutex, blocking the caller's task until it can or the timeout is reached.
///
/// Returns `true` if the lock could be acquired.
/// Once acquired, call `unlock()` on the Mutex to release it.
fstd_util bool ftsk_mutex_timed_lock(FTSK_Mutex *mutex, FSTD_Duration timeout) {
    if (!ftsk_mutex_try_lock(mutex)) {
        FSTD_Instant t = fstd_instant_add_saturating(fstd_instant_now(), timeout);
        return ftsk__mutex_lock_contended(mutex, t);
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

fstd_util bool ftsk__condition_wait(FTSK_Condition *condition, FTSK_Mutex *mutex, FSTD_Instant timeout) {
    FSTD_U32 current = atomic_load_explicit(&condition->futex, memory_order_acquire);
    ftsk_mutex_unlock(mutex);
    FTSK_FutexStatus status = ftsk_futex_wait(condition, sizeof(*condition), current, 0, &timeout);
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
    ftsk__condition_wait(condition, mutex, FSTD_INIT(FSTD_Instant) FSTD_INSTANT_MAX);
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
    return ftsk__condition_wait(condition, mutex, t);
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

#define FTSK_CONDITION_INIT FSTD_DEFAULT_STRUCT

#define FTSK_SYM_NS "fimo-tasks"
#define FTSK__SYM_VERSION FSTD_CTX_VERSION

#define FTSK_SYM_TASK_ID FSTD_MODULE_SYMBOL_NS("task_id", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_WORKER_ID FSTD_MODULE_SYMBOL_NS("worker_id", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_YIELD FSTD_MODULE_SYMBOL_NS("yield", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_ABORT FSTD_MODULE_SYMBOL_NS("abort", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_CANCEL_REQUESTED FSTD_MODULE_SYMBOL_NS("cancel_requested", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_SLEEP FSTD_MODULE_SYMBOL_NS("sleep", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_TASK_LOCAL_SET FSTD_MODULE_SYMBOL_NS("task_local_set", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_TASK_LOCAL_GET FSTD_MODULE_SYMBOL_NS("task_local_get", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_TASK_LOCAL_CLEAR FSTD_MODULE_SYMBOL_NS("task_local_clear", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_CMD_BUF_JOIN FSTD_MODULE_SYMBOL_NS("cmd_buf_join", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_CMD_BUF_DETACH FSTD_MODULE_SYMBOL_NS("cmd_buf_detach", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_CMD_BUF_CANCEL FSTD_MODULE_SYMBOL_NS("cmd_buf_cancel", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_CMD_BUF_CANCEL_DETACH FSTD_MODULE_SYMBOL_NS("cmd_buf_cancel_detach", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_EXECUTOR_GLOBAL FSTD_MODULE_SYMBOL_NS("executor_global", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_EXECUTOR_NEW FSTD_MODULE_SYMBOL_NS("executor_new", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_EXECUTOR_CURRENT FSTD_MODULE_SYMBOL_NS("executor_current", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_EXECUTOR_JOIN FSTD_MODULE_SYMBOL_NS("executor_join", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_EXECUTOR_JOIN_REQUESTED                                                                               \
    FSTD_MODULE_SYMBOL_NS("executor_join_requested", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_EXECUTOR_ENQUEUE FSTD_MODULE_SYMBOL_NS("executor_enqueue", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_EXECUTOR_ENQUEUE_DETACHED                                                                             \
    FSTD_MODULE_SYMBOL_NS("executor_enqueue_detached", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_FUTEX_WAIT FSTD_MODULE_SYMBOL_NS("futex_wait", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_FUTEX_WAIT_V FSTD_MODULE_SYMBOL_NS("futex_waitv", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_FUTEX_WAKE FSTD_MODULE_SYMBOL_NS("futex_wake", FTSK_SYM_NS, FTSK__SYM_VERSION)
#define FTSK_SYM_FUTEX_REQUEUE FSTD_MODULE_SYMBOL_NS("futex_requeue", FTSK_SYM_NS, FTSK__SYM_VERSION)

FSTD_SYMBOL_FN(ftsk_sym_, bool, task_id, FTSK_TaskId *id)
FSTD_SYMBOL_FN(ftsk_sym_, bool, worker_id, FTSK_Worker *id)
FSTD_SYMBOL_FN(ftsk_sym_, void, yield, void)
FSTD_SYMBOL_FN(ftsk_sym_, void, abort, void)
FSTD_SYMBOL_FN(ftsk_sym_, bool, cancel_requested, void)
FSTD_SYMBOL_FN(ftsk_sym_, void, sleep, FSTD_Duration duration)
FSTD_SYMBOL_FN(ftsk_sym_, void, task_local_set, const FTSK_TssKey *key, void *FSTD_MAYBE_NULL value,
               FTSK_TssKeyDtor FSTD_MAYBE_NULL dtor)
FSTD_SYMBOL_FN(ftsk_sym_, void *FSTD_MAYBE_NULL, task_local_get, const FTSK_TssKey *key)
FSTD_SYMBOL_FN(ftsk_sym_, void, task_local_clear, const FTSK_TssKey *key)
FSTD_SYMBOL_FN(ftsk_sym_, FTSK_CmdBufHandleCompletionStatus, cmd_buf_join, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL_FN(ftsk_sym_, void, cmd_buf_detach, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL_FN(ftsk_sym_, void, cmd_buf_cancel, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL_FN(ftsk_sym_, void, cmd_buf_cancel_detach, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL(ftsk_sym_, FTSK_Executor, executor_global)
FSTD_SYMBOL_FN(ftsk_sym_, FSTD_Status, executor_new, FTSK_Executor **exe, const FTSK_ExecutorCfg *cfg)
FSTD_SYMBOL_FN(ftsk_sym_, FTSK_Executor *FSTD_MAYBE_NULL, executor_current)
FSTD_SYMBOL_FN(ftsk_sym_, void, executor_join, FTSK_Executor *exe)
FSTD_SYMBOL_FN(ftsk_sym_, bool, executor_join_requested, FTSK_Executor *exe)
FSTD_SYMBOL_FN(ftsk_sym_, FTSK_CmdBufHandle *, executor_enqueue, FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf)
FSTD_SYMBOL_FN(ftsk_sym_, void, executor_enqueue_detached, FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf)
FSTD_SYMBOL_FN(ftsk_sym_, FTSK_FutexStatus, futex_wait, const void *key, FSTD_USize key_size, FSTD_U64 expect,
               FSTD_USize token, const FSTD_Instant *FSTD_MAYBE_NULL timeout)
FSTD_SYMBOL_FN(ftsk_sym_, FTSK_FutexStatus, futex_waitv, FTSK_FutexKeyExpectSlice keys,
               const FSTD_Instant *FSTD_MAYBE_NULL timeout, FSTD_USize *wake_index)
FSTD_SYMBOL_FN(ftsk_sym_, FSTD_USize, futex_wake, const void *key, FSTD_USize max_waiters, FTSK_FutexFilter filter)
FSTD_SYMBOL_FN(ftsk_sym_, FTSK_FutexStatus, futex_requeue, const void *key_from, const void *key_to,
               FSTD_USize key_size, FSTD_U64 expect, FSTD_USize max_wakes, FSTD_USize max_requeues,
               FTSK_FutexFilter filter, FTSK_FutexRequeueResult *result)

#ifdef __cplusplus
}
#endif

#ifdef FIMO_TASKS_IMPLEMENTATION

#ifdef __cplusplus
extern "C" {
#endif

FSTD_SYMBOL_FN_IMPL(ftsk_sym_, bool, task_id, FTSK_TaskId *id)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, bool, worker_id, FTSK_Worker *id)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, yield, void)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, abort, void)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, bool, cancel_requested, void)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, sleep, FSTD_Duration duration)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, task_local_set, const FTSK_TssKey *key, void *FSTD_MAYBE_NULL value,
                    FTSK_TssKeyDtor FSTD_MAYBE_NULL dtor)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void *FSTD_MAYBE_NULL, task_local_get, const FTSK_TssKey *key)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, task_local_clear, const FTSK_TssKey *key)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FTSK_CmdBufHandleCompletionStatus, cmd_buf_join, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, cmd_buf_detach, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, cmd_buf_cancel, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, cmd_buf_cancel_detach, FTSK_CmdBufHandle *cmd_buf)
FSTD_SYMBOL_IMPL(ftsk_sym_, FTSK_Executor, executor_global)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FSTD_Status, executor_new, FTSK_Executor **exe, const FTSK_ExecutorCfg *cfg)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FTSK_Executor *FSTD_MAYBE_NULL, executor_current)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, executor_join, FTSK_Executor *exe)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, bool, executor_join_requested, FTSK_Executor *exe)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FTSK_CmdBufHandle *, executor_enqueue, FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, void, executor_enqueue_detached, FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FTSK_FutexStatus, futex_wait, const void *key, FSTD_USize key_size, FSTD_U64 expect,
                    FSTD_USize token, const FSTD_Instant *FSTD_MAYBE_NULL timeout)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FTSK_FutexStatus, futex_waitv, FTSK_FutexKeyExpectSlice keys,
                    const FSTD_Instant *FSTD_MAYBE_NULL timeout, FSTD_USize *wake_index)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FSTD_USize, futex_wake, const void *key, FSTD_USize max_waiters, FTSK_FutexFilter filter)
FSTD_SYMBOL_FN_IMPL(ftsk_sym_, FTSK_FutexStatus, futex_requeue, const void *key_from, const void *key_to,
                    FSTD_USize key_size, FSTD_U64 expect, FSTD_USize max_wakes, FSTD_USize max_requeues,
                    FTSK_FutexFilter filter, FTSK_FutexRequeueResult *result)

fstd_func_impl bool ftsk_task_id_current(FTSK_TaskId *id) { return ftsk_sym_task_id_get()(id); }

fstd_func_impl bool ftsk_worker_id_current(FTSK_Worker *id) { return ftsk_sym_worker_id_get()(id); }

fstd_func_impl void ftsk_yield(void) { ftsk_sym_yield_get()(); }

fstd_func_impl void ftsk_abort(void) { ftsk_sym_abort_get()(); }

fstd_func_impl bool ftsk_cancel_requested(void) { return ftsk_sym_cancel_requested_get()(); }

fstd_func_impl void ftsk_sleep(FSTD_Duration duration) { ftsk_sym_sleep_get()(duration); }

fstd_func_impl void ftsk_tss_key_set(const FTSK_TssKey *key, void *FSTD_MAYBE_NULL value,
                                     FTSK_TssKeyDtor FSTD_MAYBE_NULL dtor) {
    ftsk_sym_task_local_set_get()(key, value, dtor);
}

fstd_func_impl void *FSTD_MAYBE_NULL ftsk_tss_key_get(const FTSK_TssKey *key) {
    return ftsk_sym_task_local_get_get()(key);
}

fstd_func_impl void ftsk_tss_key_clear(const FTSK_TssKey *key) { ftsk_sym_task_local_clear_get()(key); }

fstd_func_impl FTSK_CmdBufHandleCompletionStatus ftsk_cmd_buf_handle_join(FTSK_CmdBufHandle *cmd_buf) {
    return ftsk_sym_cmd_buf_join_get()(cmd_buf);
}

fstd_func_impl void ftsk_cmd_buf_handle_detach(FTSK_CmdBufHandle *cmd_buf) { ftsk_sym_cmd_buf_detach_get()(cmd_buf); }

fstd_func_impl void ftsk_cmd_buf_handle_cancel(FTSK_CmdBufHandle *cmd_buf) { ftsk_sym_cmd_buf_cancel_get()(cmd_buf); }

fstd_func_impl void ftsk_cmd_buf_handle_cancel_detach(FTSK_CmdBufHandle *cmd_buf) {
    ftsk_sym_cmd_buf_cancel_detach_get()(cmd_buf);
}

fstd_func FTSK_Executor *ftsk_global_executor() { return (FTSK_Executor *)ftsk_sym_executor_global_get(); }

fstd_func FSTD_Status ftsk_executor_init(FTSK_Executor **exe, const FTSK_ExecutorCfg *cfg) {
    return ftsk_sym_executor_new_get()(exe, cfg);
}

fstd_func FTSK_Executor *ftsk_executor_current() { return ftsk_sym_executor_current_get()(); }

fstd_func void ftsk_executor_join(FTSK_Executor *exe) { ftsk_sym_executor_join_get()(exe); }

fstd_func bool ftsk_executor_join_requested(FTSK_Executor *exe) { return ftsk_sym_executor_join_requested_get()(exe); }

fstd_func FTSK_CmdBufHandle *ftsk_executor_enqueue(FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf) {
    return ftsk_sym_executor_enqueue_get()(exe, cmd_buf);
}

fstd_func void ftsk_executor_enqueue_detached(FTSK_Executor *exe, FTSK_CmdBuf *cmd_buf) {
    ftsk_sym_executor_enqueue_detached_get()(exe, cmd_buf);
}

fstd_func FTSK_FutexStatus ftsk_futex_wait(const void *key, FSTD_USize key_size, FSTD_U64 expect, FSTD_USize token,
                                           const FSTD_Instant *FSTD_MAYBE_NULL timeout) {
    return ftsk_sym_futex_wait_get()(key, key_size, expect, token, timeout);
}

fstd_func FTSK_FutexStatus ftsk_futex_waitv(FTSK_FutexKeyExpectSlice keys, const FSTD_Instant *FSTD_MAYBE_NULL timeout,
                                            FSTD_USize *wake_index) {
    return ftsk_sym_futex_waitv_get()(keys, timeout, wake_index);
}

fstd_func FSTD_USize ftsk_futex_wake(const void *key, FSTD_USize max_waiters, FTSK_FutexFilter filter) {
    return ftsk_sym_futex_wake_get()(key, max_waiters, filter);
}

fstd_func FTSK_FutexStatus ftsk_futex_requeue(const void *key_from, const void *key_to, FSTD_USize key_size,
                                              FSTD_U64 expect, FSTD_USize max_wakes, FSTD_USize max_requeues,
                                              FTSK_FutexFilter filter, FTSK_FutexRequeueResult *result) {
    return ftsk_sym_futex_requeue_get()(key_from, key_to, key_size, expect, max_wakes, max_requeues, filter, result);
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
