#ifndef FIMO_TASKS_COMMAND_BUFFER_H
#define FIMO_TASKS_COMMAND_BUFFER_H

#include <stdbool.h>
#include <stddef.h>

#include <fimo_std/error.h>
#include <fimo_std/module.h>
#include <fimo_std/time.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Namespace of the symbols exposed by the bindings.
 */
#define FI_TASKS_SYMBOL_NAMESPACE "fimo_tasks"

/**
 * Name of the `context` symbol.
 */
#define FI_TASKS_SYMBOL_NAME_CONTEXT "context"

/**
 * Major version of the `context` symbol.
 */
#define FI_TASKS_SYMBOL_VERSION_MAJOR_CONTEXT 0

/**
 * Minor version of the `context` symbol.
 */
#define FI_TASKS_SYMBOL_VERSION_MINOR_CONTEXT 1

/**
 * Patch version of the `context` symbol.
 */
#define FI_TASKS_SYMBOL_VERSION_PATCH_CONTEXT 0

/**
 * Constructs a new command.
 *
 * @param TASK pointer to the task to spawn
 */
#define FI_TASKS_SPAWN_TASK_COMMAND(TASK)                                                                              \
    { .type = FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SPAWN_TASK, .data = {.spawn_task = (TASK)}, }

/**
 * Constructs a new command.
 */
#define FI_TASKS_WAIT_BARRIER_COMMAND()                                                                                \
    { .type = FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_BARRIER, .data = {.wait_barrier = 0}, }

/**
 * Constructs a new command.
 *
 * @param BUFFER_HANDLE command buffer to wait for
 */
#define FI_TASKS_WAIT_COMMAND_BUFFER_COMMAND(BUFFER_HANDLE)                                                            \
    {                                                                                                                  \
        .type = FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_COMMAND_BUFFER,                                                \
        .data = {.wait_command_buffer = (BUFFER_HANDLE)},                                                              \
    }

/**
 * Constructs a new command.
 *
 * @param WORKER worker id
 */
#define FI_TASKS_SET_WORKER_COMMAND(WORKER)                                                                            \
    { .type = FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_WORKER, .data = {.set_worker = (WORKER)}, }

/**
 * Constructs a new command.
 */
#define FI_TASKS_ENABLE_ALL_WORKERS_COMMAND()                                                                          \
    { .type = FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_ENABLE_ALL_WORKERS, .data = {.enable_all_workers = 0}, }

/**
 * Constructs a new command.
 *
 * @param STACK_SIZE minimum stack size
 */
#define FI_TASKS_SET_STACK_SIZE_COMMAND(STACK_SIZE)                                                                    \
    { .type = FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_STACK_SIZE, .data = {.set_stack_size = (STACK_SIZE)}, }


/**
 * VTable of a `FiTasksContext`.
 */
typedef struct FiTasksVTable FiTasksVTable;

/**
 * Context of fimo tasks.
 *
 * The context is an opaque object that can only be accessed through
 * the provided vtable.
 */
typedef struct FiTasksContext {
    void *data;
    const FiTasksVTable *vtable;
} FiTasksContext;

/**
 * An entry of a command buffer.
 */
typedef struct FiTasksCommandBufferEntry FiTasksCommandBufferEntry;

/**
 * A command buffer.
 */
typedef struct FiTasksCommandBuffer {
    /**
     * Optional label of the command buffer.
     *
     * May be used by the runtime for tracing purposes.
     * If present, the string must live until the completion
     * of the command buffer, and may be released by the
     * `on_cleanup` function.
     */
    const char *label;
    /**
     * List of commands to process.
     */
    const FiTasksCommandBufferEntry *entries;
    /**
     * Number of commands in the buffer.
     */
    FimoUSize num_entries;
    /**
     * Optional callback function that will be called
     * after the completion of all commands.
     *
     * If present, this function may be invoked on an
     * unspecified thread, outside the scope of a task.
     *
     * @param arg0 optional user data
     * @param arg1 pointer to the command buffer
     */
    void (*on_complete)(void *, struct FiTasksCommandBuffer *);
    /**
     * Optional callback function that will be called
     * if the command buffer could not be executed
     * without an error.
     *
     * If present, this function may be invoked on an
     * unspecified thread, outside the scope of a task.
     *
     * @param arg0 optional user data
     * @param arg1 pointer to the command buffer
     * @param arg2 index of the command that caused the error
     */
    void (*on_abort)(void *, struct FiTasksCommandBuffer *, FimoUSize);
    /**
     * Optional user data to pass to the status callbacks.
     */
    void *status_callback_data;
    /**
     * Optional callback to invoke when cleaning up the
     * command buffer.
     *
     * If present, this function may be invoked on an
     * unspecified thread, outside the scope of a task.
     *
     * @param arg0 optional user data
     * @param arg1 pointer to the command buffer
     */
    void (*on_cleanup)(void *, struct FiTasksCommandBuffer *);
    /**
     * Optional user data to pass to the cleanup callback.
     */
    void *cleanup_data;
} FiTasksCommandBuffer;

/**
 * A task.
 */
typedef struct FiTasksTask {
    /**
     * Optional label of the task.
     *
     * May be used by the runtime for tracing purposes.
     * If present, the string must live until the completion
     * of the task, and may be released by the `on_cleanup`
     * function.
     */
    const char *label;
    /**
     * Entry function of the task.
     *
     * @param arg0 optional user data
     * @param arg1 pointer to the task
     * @param arg2 tasks context
     */
    void (*start)(void *, struct FiTasksTask *, FiTasksContext);
    /**
     * Optional user data to pass to the entry function.
     */
    void *user_data;
    /**
     * Optional callback function that will be called
     * after the completion of the task.
     *
     * If present, this function may be invoked on an
     * unspecified thread, outside the scope of a task.
     *
     * @param arg0 optional user data
     * @param arg1 pointer to the task
     */
    void (*on_complete)(void *, struct FiTasksTask *);
    /**
     * Optional callback function that will be called
     * if the task could not be executed without an error.
     *
     * If present, this function may be invoked on an
     * unspecified thread, outside the scope of a task.
     *
     * @param arg0 optional user data
     * @param arg1 pointer to the task
     * @param arg2 optional abort error
     */
    void (*on_abort)(void *, struct FiTasksTask *, void *);
    /**
     * Optional user data to pass to the status callbacks.
     */
    void *status_callback_data;
    /**
     * Optional callback to invoke when cleaning up the task.
     *
     * If present, this function may be invoked on an
     * unspecified thread, outside the scope of a task.
     *
     * @param arg0 optional user data
     * @param arg1 pointer to the task
     */
    void (*on_cleanup)(void *, struct FiTasksTask *);
    /**
     * Optional user data to pass to the cleanup callback.
     */
    void *cleanup_data;
} FiTasksTask;

/**
 * Handle to an enqueued command buffer.
 */
typedef struct FiTasksCommandBufferHandle FiTasksCommandBufferHandle;

/**
 * VTable of a `FiTasksWorkerGroup`.
 */
typedef struct FiTasksWorkerGroupVTable FiTasksWorkerGroupVTable;

/**
 * A reference to a worker group.
 */
typedef struct FiTasksWorkerGroup {
    void *data;
    const FiTasksWorkerGroupVTable *vtable;
} FiTasksWorkerGroup;

/**
 * Core VTable of a `FiTasksWorkerGroup`.
 */
typedef struct FiTasksWorkerGroupVTableV0 {
    FimoUSize (*id)(void *);
    void (*acquire)(void *);
    void (*release)(void *);
    bool (*is_open)(void *);
    bool (*is_worker)(void *);
    const char *(*name)(void *);
    FimoError (*request_close)(void *);
    FimoError (*workers)(void *, FimoUSize **, FimoUSize *);
    FimoError (*stack_sizes)(void *, FimoUSize **, FimoUSize *);
    FimoError (*enqueue_buffer)(void *, FiTasksCommandBuffer *, bool, FiTasksCommandBufferHandle *);
} FiTasksWorkerGroupVTableV0;

struct FiTasksWorkerGroupVTable {
    FiTasksWorkerGroupVTableV0 v0;
};

/**
 * Core VTable of a `FiTasksCommandBufferHandle`.
 */
typedef struct FiTasksCommandBufferHandleVTableV0 {
    void (*acquire)(void *);
    void (*release)(void *);
    FimoError (*worker_group)(void *, FiTasksWorkerGroup *);
    FimoError (*wait_on)(void *, bool *);
} FiTasksCommandBufferHandleVTableV0;

/**
 * VTable of a `FiTasksCommandBufferHandle`.
 */
typedef struct FiTasksCommandBufferHandleVTable {
    FiTasksCommandBufferHandleVTableV0 v0;
} FiTasksCommandBufferHandleVTable;

/**
 * Handle to an enqueued command buffer.
 */
typedef struct FiTasksCommandBufferHandle {
    void *data;
    const FiTasksCommandBufferHandleVTable *vtable;
} FiTasksCommandBufferHandle;

/**
 * Linked list of worker groups.
 */
typedef struct FiTasksWorkerGroupQuery {
    FiTasksWorkerGroup grp;
    struct FiTasksWorkerGroupQuery *next;
} FiTasksWorkerGroupQuery;

/**
 * Configuration structure for stacks of worker groups.
 */
typedef struct FiTasksWorkerGroupConfigStack {
    /**
     * Reserved for future use.
     * Must be `null`.
     */
    void *next;
    /**
     * Size of the stack allocation.
     *
     * The size may be rounded up to a multiple of the page size,
     * and may also include an additional guard page. A value of
     * `0` indicates to use the default stack size for the system.
     */
    FimoUSize size;
    /**
     * Number of stacks to preallocate.
     */
    FimoUSize starting_residency;
    /**
     * Number of resident stacks to target at any given point
     * in time. If the number of resident stacks is lower than
     * the indicated target, the worker group may cache unassigned
     * stacks. A value of `0` indicates that no residency number
     * should be targeted.
     */
    FimoUSize residency_target;
    /**
     * Maximum number of stacks to have allocated at any given
     * point in time. If more stacks are required, the tasks
     * will be put on hold, until they can be acquired. This,
     * may lead to a deadlock under some circumstances. A value
     * of `0` indicates no upper limit to the number of resident
     * stacks.
     */
    FimoUSize max_residency;
    /**
     * Enables the overflow protection of the stack. May require
     * the allocation of an additional guard page for each stack.
     * Enabling this option may marginally increase the allocation
     * time for each stack, but is advised, as a task may otherwise
     * overwrite foreign memory.
     */
    bool enable_stack_overflow_protection;
} FiTasksWorkerGroupConfigStack;

/**
 * Configuration structure for the creation of worker groups.
 */
typedef struct FiTasksWorkerGroupConfig {
    /**
     * Reserved for future use.
     * Must be `null`.
     */
    void *next;
    /**
     * Non-unique name of the worker group.
     * Must not be `null`.
     */
    const char *name;
    /**
     * Array of configuration for the stacks available to the
     * worker group. Each worker group must provide at least
     * one stack.
     */
    const FiTasksWorkerGroupConfigStack *stacks;
    /**
     * Number of stacks specified in the configuration.
     */
    FimoUSize num_stacks;
    /**
     * Index to the default stack configuration for each task
     * that does not specify otherwise.
     */
    FimoUSize default_stack_index;
    /**
     * Number of worker threads to start in the new worker group.
     * A value of `0` starts one worker for each core thread
     * present in the system.
     */
    FimoUSize number_of_workers;
    /**
     * Indicates whether to make a reference to the new worker
     * group queryable through the task context. Specifying this
     * does not stop others to acquire a reference to the worker
     * group through its id.
     */
    bool is_queryable;
} FiTasksWorkerGroupConfig;

/**
 * Type for an entry in a command buffer.
 */
typedef enum FiTasksCommandBufferEntryType {
    /**
     * Specifies that the runtime should spawn a new task.
     */
    FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SPAWN_TASK = 0,
    /**
     * A barrier command.
     *
     * Barriers synchronize the execution of a command buffer,
     * ensuring that the preceding commands have been completed.
     */
    FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_BARRIER = 1,
    /**
     * A command buffer synchronization command.
     *
     * Synchronizes the following commands with the completion
     * of another command buffer.
     */
    FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_WAIT_COMMAND_BUFFER = 2,
    /**
     * Specifies the worker thread on which the following
     * must be spawned.
     */
    FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_WORKER = 3,
    /**
     * Enables the following tasks to be spawned on all
     * available workers of the worker group.
     */
    FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_ENABLE_ALL_WORKERS = 4,
    /**
     * Requests a minimum stack size for the following
     * tasks.
     */
    FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_SET_STACK_SIZE = 5,
    FI_TASKS_COMMAND_BUFFER_ENTRY_TYPE_FORCE32 = 0x7FFFFFFF
} FiTasksCommandBufferEntryType;

/**
 * Data for an entry in a command buffer.
 */
typedef union FiTasksCommandBufferEntryData {
    /**
     * Pointer to the task to spawn.
     */
    FiTasksTask *spawn_task;
    /**
     * Placeholder.
     */
    FimoU8 wait_barrier;
    /**
     * Strong reference to a command buffer handle.
     */
    FiTasksCommandBufferHandle wait_command_buffer;
    /**
     * Worker.
     */
    FimoUSize set_worker;
    /**
     * Placeholder.
     */
    FimoU8 enable_all_workers;
    /**
     * Stack size.
     */
    FimoUSize set_stack_size;
} FiTasksCommandBufferEntryData;

struct FiTasksCommandBufferEntry {
    /**
     * Command identifier.
     */
    FiTasksCommandBufferEntryType type;
    /**
     * Data associated with the command.
     */
    FiTasksCommandBufferEntryData data;
};

/**
 * A key for a task-specific-storage.
 */
typedef const struct FiTasksTssKey_ *FiTasksTssKey;

/**
 * Destructor for a task-specific-storage.
 */
typedef void (*FiTasksTssDtor)(void *);

/**
 * Types of results of a `FiTasksParkResult`.
 */
typedef enum FiTasksParkResultType {
    /**
     * The wait operation was skipped by the runtime.
     */
    FI_TASKS_PARK_RESULT_TYPE_INVALID = 0,
    /**
     * The wait operation timed out.
     */
    FI_TASKS_PARK_RESULT_TYPE_TIMED_OUT = 1,
    /**
     * Task was unparked by another task with the given token.
     */
    FI_TASKS_PARK_RESULT_TYPE_UNPARKED = 2,
    FI_TASKS_PARK_RESULT_TYPE_FORCE32 = 0x7FFFFFFF
} FiTasksParkResultType;

/**
 * Data passed to a task upon wakeup.
 */
typedef struct FiTasksParkResult {
    FiTasksParkResultType type;
    const void *data;
} FiTasksParkResult;

/**
 * Result of an `unpark_*` operation.
 */
typedef struct FiTasksUnparkResult {
    /**
     * Number of tasks notified by the operation.
     */
    FimoUSize unparked_tasks;
    /**
     * The number of tasks that were requeued.
     */
    FimoUSize requeued_tasks;
    /**
     * Whether there are any tasks remaining in the queue.
     * This only returns true if a task was unparked.
     */
    bool has_more_tasks;
    /**
     * Indicates whether a fair unlocking mechanism should be used.
     */
    bool be_fair;
} FiTasksUnparkResult;

/**
 * Operation that `unpark_filter` should perform for each task.
 */
typedef enum FiTasksUnparkFilterOp {
    /**
     * Unparks the task and continues the filter operation.
     */
    FI_TASKS_UNPARK_FILTER_OP_UNPARK = 0,
    /**
     * Stops the filter operation without notifying the task.
     */
    FI_TASKS_UNPARK_FILTER_OP_STOP = 1,
    /**
     * Continues the filter operation without notifying the task.
     */
    FI_TASKS_UNPARK_FILTER_OP_SKIP = 2,
    FI_TASKS_UNPARK_FILTER_OP_FORCE32 = 0x7FFFFFFF
} FiTasksUnparkFilterOp;

/**
 * Operation that `unpark_requeue` should perform.
 */
typedef enum FiTasksRequeueOp {
    /**
     * Abort the operation without doing anything.
     */
    FI_TASKS_REQUEUE_OP_ABORT = 0,
    /**
     * Unpark one task and requeue the rest onto the target queue.
     */
    FI_TASKS_REQUEUE_OP_UNPARK_ONE_REQUEUE_REST = 1,
    /**
     * Requeue all tasks onto the target queue.
     */
    FI_TASKS_REQUEUE_OP_REQUEUE_ALL = 2,
    /**
     * Unpark one task and leave the rest parked. No requeueing is done.
     */
    FI_TASKS_REQUEUE_OP_UNPARK_ONE = 3,
    /**
     * Requeue one task and leave the rest parked on the original queue.
     */
    FI_TASKS_REQUEUE_OP_REQUEUE_ONE = 4,
    FI_TASKS_REQUEUE_OP_FORCE32 = 0x7FFFFFFF
} FiTasksRequeueOp;

/**
 * Core VTable of a `FiTasksContext`.
 */
typedef struct FiTasksVTableV0 {
    bool (*is_worker)(void *);
    FimoError (*task_id)(void *, FimoUSize *);
    FimoError (*worker_id)(void *, FimoUSize *);
    FimoError (*worker_group)(void *, FiTasksWorkerGroup *);
    FimoError (*worker_group_by_id)(void *, FimoUSize, FiTasksWorkerGroup *);
    FimoError (*query_worker_groups)(void *, FiTasksWorkerGroupQuery **);
    FimoError (*release_worker_group_query)(void *, FiTasksWorkerGroupQuery *);
    FimoError (*create_worker_group)(void *, FiTasksWorkerGroupConfig, FiTasksWorkerGroup *);
    FimoError (*yield)(void *);
    FimoError (*abort)(void *, void *);
    FimoError (*sleep)(void *, FimoDuration);
    FimoError (*tss_set)(void *, FiTasksTssKey, void *, FiTasksTssDtor);
    FimoError (*tss_get)(void *, FiTasksTssKey, void **);
    FimoError (*tss_clear)(void *, FiTasksTssKey);
    FimoError (*park_conditionally)(void *, const void *, bool (*)(void *), void *, void (*)(void *), void *,
                                    void (*)(void *, const void *, bool), void *, const void *, const FimoDuration *,
                                    FiTasksParkResult *);
    FimoError (*unpark_one)(void *, const void *, const void *(*)(void *, FiTasksUnparkResult), void *,
                            FiTasksUnparkResult *);
    FimoError (*unpark_all)(void *, const void *, const void *, FimoUSize *);
    FimoError (*unpark_requeue)(void *, const void *, const void *, FiTasksRequeueOp (*)(void *), void *,
                                const void *(*)(void *, FiTasksRequeueOp, FiTasksUnparkResult), void *,
                                FiTasksUnparkResult *);
    FimoError (*unpark_filter)(void *, const void *, FiTasksUnparkFilterOp (*)(void *, const void *), void *,
                               const void *(*)(void *, FiTasksUnparkResult), void *, FiTasksUnparkResult *);
} FiTasksVTableV0;

struct FiTasksVTable {
    FiTasksVTableV0 v0;
};

/**
 * Returns the unique id of the worker group.
 *
 * @param grp worker group
 *
 * @return Worker group id
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoUSize fi_tasks_worker_group_id(FiTasksWorkerGroup grp) {
    return grp.vtable->v0.id(grp.data);
}

/**
 * Acquires a strong reference to the worker group.
 *
 * @param grp worker group
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS void fi_tasks_worker_group_acquire(FiTasksWorkerGroup grp) {
    grp.vtable->v0.acquire(grp.data);
}

/**
 * Releases a strong reference to the worker group.
 *
 * @param grp worker group
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS void fi_tasks_worker_group_release(FiTasksWorkerGroup grp) {
    grp.vtable->v0.release(grp.data);
}

/**
 * Checks whether the worker group is open to receive new commands.
 *
 * @param grp worker group
 *
 * @return `true` if new commands can be enqueued to the worker group.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS bool fi_tasks_worker_group_is_open(FiTasksWorkerGroup grp) {
    return grp.vtable->v0.is_open(grp.data);
}

/**
 * Checks whether the current thread is a worker thread of the group.
 *
 * @param grp worker group
 *
 * @return `true` if the current thread is a worker thread of the group.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS bool fi_tasks_worker_group_is_worker(FiTasksWorkerGroup grp) {
    return grp.vtable->v0.is_worker(grp.data);
}

/**
 * Returns the name of the worker.
 *
 * The returned reference is guaranteed to be valid for as long as the
 * caller owns a strong reference to the worker group.
 *
 * @param grp worker group
 *
 * @return Worker group name
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS const char *fi_tasks_worker_group_name(FiTasksWorkerGroup grp) {
    return grp.vtable->v0.name(grp.data);
}

/**
 * Requests that the worker group stops accepting new commands.
 *
 * If successful, the worker group will close its side of the
 * channel and stop accepting new commands. The currently enqueued
 * commands will be run to completion.
 *
 * @param grp worker group
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_worker_group_request_close(FiTasksWorkerGroup grp) {
    return grp.vtable->v0.request_close(grp.data);
}

/**
 * Fetches a list of worker ids available in the worker group.
 *
 * If the `workers` parameter is `null`, this function will only
 * return the number of workers in the group without allocating
 * any memory. Otherwise, `workers` will be set to point to an
 * array allocated by `fimo_malloc` and must be deallocated by
 * the caller.
 *
 * @param grp worker group
 * @param workers array of workers ids
 * @param count worker count
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_worker_group_workers(FiTasksWorkerGroup grp, FimoUSize **workers,
                                                                  FimoUSize *count) {
    return grp.vtable->v0.workers(grp.data, workers, count);
}

/**
 * Fetches a list of stack sizes available in the worker group.
 *
 * When spawning new tasks, they will be assigned one stack which
 * matches the requirements specified in the commands.
 *
 * If the `stack_sizes` parameter is `null`, this function will
 * only return the number of stack sizes in the group without
 * allocating any memory. Otherwise, `workers` will be set to
 * point to an array allocated by `fimo_malloc` and must be
 * deallocated by the caller.
 *
 * @param grp worker group
 * @param stack_sizes array of stack sizes
 * @param count worker count
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_worker_group_stack_sizes(FiTasksWorkerGroup grp, FimoUSize **stack_sizes,
                                                                      FimoUSize *count) {
    return grp.vtable->v0.stack_sizes(grp.data, stack_sizes, count);
}

/**
 * Enqueues a command buffer to the scheduler of the worker group.
 *
 * On success, the command buffer `buffer` will be owned by the
 * worker group, which will start executing the contained commands
 * in parallel. The buffer must remain alive until the worker group
 * relinquishes its ownership, which happens after the completion
 * and detaching of the buffer. The caller may specify whether to
 * enqueue the buffer in a detached state by setting the `detached`
 * parameter to `true`, in which case `handle` will be set to `null`.
 *
 * @param grp worker group
 * @param buffer command buffer to enqueue
 * @param detached whether to enqueue the buffer in a detached state
 * @param handle resulting handle to the enqueued buffer
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_worker_group_enqueue_buffer(FiTasksWorkerGroup grp,
                                                                         FiTasksCommandBuffer *buffer, bool detached,
                                                                         FiTasksCommandBufferHandle *handle) {
    return grp.vtable->v0.enqueue_buffer(grp.data, buffer, detached, handle);
}

/**
 * Acquires a strong reference to the handle.
 *
 * @param handle buffer handle
 */
static FIMO_INLINE_ALWAYS void fi_tasks_command_buffer_handle_acquire(FiTasksCommandBufferHandle handle) {
    handle.vtable->v0.acquire(handle.data);
}

/**
 * Releases a strong reference to the handle.
 *
 * @param handle buffer handle
 */
static FIMO_INLINE_ALWAYS void fi_tasks_command_buffer_handle_release(FiTasksCommandBufferHandle handle) {
    handle.vtable->v0.release(handle.data);
}

/**
 * Returns a strong reference to the worker group executing the buffer handle.
 *
 * @param handle buffer handle
 * @param grp worker group
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_command_buffer_worker_group(FiTasksCommandBufferHandle handle,
                                                                         FiTasksWorkerGroup *grp) {
    return handle.vtable->v0.worker_group(handle.data, grp);
}

/**
 * Waits on the completion of all commands in the command buffer.
 *
 * Calling this function will halt the execution of the current
 * task, until the completion of the specified command buffer.
 * The handle reference is invalidated by calling this function.
 * If provided, `error` is set to whether the command buffer could
 * be executed completely.
 *
 * May only be called in a task running in the same worker group.
 *
 * @param handle handle to the command buffer
 * @param error optional success status of the command buffer
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_command_buffer_wait_on(FiTasksCommandBufferHandle handle, bool *error) {
    return handle.vtable->v0.wait_on(handle.data, error);
}

/**
 * Returns whether the current thread is a worker thread,
 * managed by some worker group the context owns.
 *
 * @param ctx context
 *
 * @return `true` if the current thread is a worker thread
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS bool fi_tasks_ctx_is_worker(FiTasksContext ctx) { return ctx.vtable->v0.is_worker(ctx.data); }

/**
 * Returns the unique id of the current task.
 *
 * The id will be unique for as long as the task is being
 * executed, but may be reused upon completion.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param id task id
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_task_id(FiTasksContext ctx, FimoUSize *id) {
    return ctx.vtable->v0.task_id(ctx.data, id);
}

/**
 * Returns the id of the current worker.
 *
 * The id is guaranteed to be unique inside the current
 * worker group.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param id worker id
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_worker_id(FiTasksContext ctx, FimoUSize *id) {
    return ctx.vtable->v0.worker_id(ctx.data, id);
}

/**
 * Acquires a reference to the worker group owning the current task.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param grp worker group
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_worker_group(FiTasksContext ctx, FiTasksWorkerGroup *grp) {
    return ctx.vtable->v0.worker_group(ctx.data, grp);
}

/**
 * Acquires a reference to the worker group assigned to the specified id.
 *
 * The reference must be released explicitly.
 *
 * @param ctx context
 * @param id worker group id
 * @param grp worker group
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_worker_group_by_id(FiTasksContext ctx, FimoUSize id,
                                                                    FiTasksWorkerGroup *grp) {
    return ctx.vtable->v0.worker_group_by_id(ctx.data, id, grp);
}

/**
 * Queries a list of worker groups available in the context.
 *
 * The queried references are valid until they are released through
 * a call to `fi_tasks_release_worker_group_query`. The caller does
 * not own the returned references, and may therefore not release
 * any of them, but may use them otherwise.
 *
 * @param ctx context
 * @param query queried list of worker groups
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_query_worker_groups(FiTasksContext ctx,
                                                                     FiTasksWorkerGroupQuery **query) {
    return ctx.vtable->v0.query_worker_groups(ctx.data, query);
}

/**
 * Releases the queried list of worker groups.
 *
 * The caller may not use the list after calling this function.
 *
 * @param ctx context
 * @param query queried list of worker groups
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_release_worker_group_query(FiTasksContext ctx,
                                                                            FiTasksWorkerGroupQuery *query) {
    return ctx.vtable->v0.release_worker_group_query(ctx.data, query);
}

/**
 * Creates a new worker group.
 *
 * The new worker group is created according to the provided configuration.
 * The resulting worker group is returned in `grp`.
 *
 * @param ctx context
 * @param cfg creation config
 * @param grp worker group.
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_create_worker_group(FiTasksContext ctx, FiTasksWorkerGroupConfig cfg,
                                                                     FiTasksWorkerGroup *grp) {
    return ctx.vtable->v0.create_worker_group(ctx.data, cfg, grp);
}

/**
 * Yields the execution of the current task back to the scheduler.
 *
 * Yielding a task may allow other tasks to be scheduled.
 * May only be called in a task.
 *
 * @param ctx context
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_yield(FiTasksContext ctx) { return ctx.vtable->v0.yield(ctx.data); }

/**
 * Aborts the current task.
 *
 * This function should be used with care, as it stops the execution
 * of the current task immediately, without performing any stack
 * unwinding and cleanup of private task data. The task may provide
 * some optional error data that will be passed to the task `on_abort`
 * callback. Note that the task is responsible for cleaning up the
 * error data.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param error error data
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_abort(FiTasksContext ctx, void *error) {
    return ctx.vtable->v0.abort(ctx.data, error);
}

/**
 * Sets the value of the task-specific storage key.
 *
 * The current value of the task-specific storage key for the current
 * task is replaced with `data`, without calling any associated
 * destructor function.
 *
 * The value is optionally also assigned a destructor function that is
 * called upon the completion of the task. It will be invoked even if
 * the value associated with the task-specific storage key is `null`.
 * The destructor may not call into the context.
 *
 * The task-specific storage key must be a unique pointer in the control
 * of the caller, for as long as the task is running, or until the task-
 * specific storage key has been cleared through `fi_tasks_ctx_tss_clear`.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key task-specific storage key
 * @param value new value for the current task
 * @param dtor optional destructor function
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_tss_set(FiTasksContext ctx, FiTasksTssKey key, void *value,
                                                         FiTasksTssDtor dtor) {
    return ctx.vtable->v0.tss_set(ctx.data, key, value, dtor);
}

/**
 * Returns the current value of the task-specific storage key.
 *
 * The value of the task-specific storage key is initially set to
 * `null`, but may be changed through `fi_tasks_ctx_tss_set`.
 * Different tasks may get different values associated with the same
 * task-specific storage key.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key task-specific storage key
 * @param value associated value
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_tss_get(FiTasksContext ctx, FiTasksTssKey key, void **value) {
    return ctx.vtable->v0.tss_get(ctx.data, key, value);
}

/**
 * Clears the value of the current task associated with the
 * task-specific storage key.
 *
 * This operation clears the value of the current task associated
 * with the task-specific storage key, by setting it to `null` and
 * optionally invoking the included destructor function. If neither
 * a value, nor a destructor function are associated with the current
 * task, this function results in a no-op.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key task-specific storage key
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_tss_clear(FiTasksContext ctx, FiTasksTssKey key) {
    return ctx.vtable->v0.tss_clear(ctx.data, key);
}

/**
 * Parks the current task in the queue associated with the given key.
 *
 * The `validate` function is called while the queue is locked and can abort
 * the operation by returning false. If `validate` returns true then the
 * current task is appended to the queue and the queue is unlocked.
 *
 * The `before_sleep` function is called after the queue is unlocked but before
 * the task is put to sleep. The task will then sleep until it is unparked
 * or the given timeout is reached.
 *
 * The `timed_out` function is also called while the queue is locked, but only
 * if the timeout was reached. It is passed the key of the queue it was in when
 * it timed out, which may be different from the original key if
 * `fi_tasks_ctx_unpark_requeue` was called. It is also passed a bool which
 * indicates whether it was the last task in the queue.
 *
 * You should only call this function with an address that you control, since
 * you could otherwise interfere with the operation of other synchronization
 * primitives.
 *
 * The `validate` and `timed_out` functions are called while the queue is
 * locked and must not panic or call into any function in the context.
 *
 * The `before_sleep` function is called outside the queue lock and is allowed
 * to call `fi_tasks_ctx_unpark_one`, `fi_tasks_ctx_unpark_all`,
 * `fi_tasks_ctx_unpark_requeue` or `fi_tasks_ctx_unpark_filter`, but it is not
 * allowed to call `fi_tasks_ctx_park_conditionally`.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key queue key
 * @param validate validation function
 * @param validate_data validation data
 * @param before_sleep before sleep function
 * @param before_sleep_data before sleep function data
 * @param timed_out timed-out function
 * @param timed_out_data timed-out function data
 * @param park_token park token
 * @param timeout optional timeout
 * @param result park result
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_park_conditionally(
        FiTasksContext ctx, const void *key, bool (*validate)(void *), void *validate_data,
        void (*before_sleep)(void *), void *before_sleep_data, void (*timed_out)(void *, const void *, bool),
        void *timed_out_data, const void *park_token, const FimoDuration *timeout, FiTasksParkResult *result) {
    return ctx.vtable->v0.park_conditionally(ctx.data, key, validate, validate_data, before_sleep, before_sleep_data,
                                             timed_out, timed_out_data, park_token, timeout, result);
}

/**
 * Unparks one task from the queue associated with the given key.
 *
 * The `callback` function is called while the queue is locked and before the
 * target task is woken up. The `FiTasksUnparkResult` argument to the function
 * indicates whether a task was found in the queue and whether this was the
 * last task in the queue. This value is also returned by `fi_tasks_ctx_unpark_one`.
 *
 * The `callback` function should return an unpark token value which will be
 * passed to the task that is unparked. If no task is unparked then the
 * returned value is ignored.
 *
 * You should only call this function with an address that you control, since
 * you could otherwise interfere with the operation of other synchronization
 * primitives.
 *
 * The `callback` function is called while the queue is locked and must not
 * panic or call into any function in the context.
 *
 * The context functions are not re-entrant and calling this method from the
 * context of an asynchronous signal handler may result in undefined behavior,
 * including corruption of internal state and/or deadlocks.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key queue key
 * @param callback callback function
 * @param callback_data callback function data
 * @param result unpark result
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_unpark_one(FiTasksContext ctx, const void *key,
                                                            const void *(*callback)(void *, FiTasksUnparkResult),
                                                            void *callback_data, FiTasksUnparkResult *result) {
    return ctx.vtable->v0.unpark_one(ctx.data, key, callback, callback_data, result);
}

/**
 * Unparks all tasks in the queue associated with the given key.
 *
 * The given unpark token is passed to all unparked tasks.
 *
 * This function returns the number of tasks that were unparked.
 *
 * You should only call this function with an address that you control, since
 * you could otherwise interfere with the operation of other synchronization
 * primitives.
 *
 * The context functions are not re-entrant and calling this method from the
 * context of an asynchronous signal handler may result in undefined behavior,
 * including corruption of internal state and/or deadlocks.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key queue key
 * @param unpark_token unpark token
 * @param unparked_tasks number of unparked tasks
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_unpark_all(FiTasksContext ctx, const void *key,
                                                            const void *unpark_token, FimoUSize *unparked_tasks) {
    return ctx.vtable->v0.unpark_all(ctx.data, key, unpark_token, unparked_tasks);
}

/**
 * Removes all tasks from the queue associated with `key_from`, optionally
 * unparks the first one and requeues the rest onto the queue associated with
 * `key_to`.
 *
 * The `validate` function is called while both queues are locked. Its return
 * value will determine which operation is performed, or whether the operation
 * should be aborted. See `FiTasksRequeueOp` for details about the different
 * possible return values.
 *
 * The `callback` function is also called while both queues are locked. It is
 * passed the `FiTasksRequeueOp` returned by `validate` and a `FiTasksUnparkResult`.
 * indicating whether a task was unparked and whether there are tasks still
 * parked in the new queue. This `FiTasksUnparkResult` value is also returned
 * by `fi_tasks_ctx_unpark_requeue`.
 *
 * The `callback` function should return an unpark token value which will be
 * passed to the task that is unparked. If no task is unparked then the
 * returned value is ignored.
 *
 * You should only call this function with an address that you control, since
 * you could otherwise interfere with the operation of other synchronization
 * primitives.
 *
 * The `validate` and `callback` functions are called while the queue is locked
 * and must not panic or call into any function in the context.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key_from source queue key
 * @param key_to destination queue key
 * @param validate validation function
 * @param validate_data validation function data
 * @param callback callback function
 * @param callback_data callback function data
 * @param result unpark result
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_unpark_requeue(
        FiTasksContext ctx, const void *key_from, const void *key_to, FiTasksRequeueOp (*validate)(void *),
        void *validate_data, const void *(*callback)(void *, FiTasksRequeueOp, FiTasksUnparkResult),
        void *callback_data, FiTasksUnparkResult *result) {
    return ctx.vtable->v0.unpark_requeue(ctx.data, key_from, key_to, validate, validate_data, callback, callback_data,
                                         result);
}

/**
 * Unparks a number of tasks from the front of the queue associated with
 * `key` depending on the results of a filter function which inspects the
 * park token associated with each task.
 *
 * The `filter` function is called for each task in the queue or until
 * `FI_TASKS_UNPARK_FILTER_OP_STOP` is returned. This function is passed
 * the park token associated with a particular task, which is unparked
 * if `FI_TASKS_UNPARK_FILTER_OP_UNPARK` is returned.
 *
 * The `callback` function is also called while both queues are locked.
 * It is passed a `FiTasksUnparkResult` indicating the number of tasks
 * that were unparked and whether there are still parked tasks in the
 * queue. This `FiTasksUnparkResult` value is also returned by
 * `fi_tasks_unpark_filter`.
 *
 * The `callback` function should return an unpark token value which
 * will be passed to all tasks that are unparked. If no task is unparked
 * then the returned value is ignored.
 *
 * You should only call this function with an address that you control,
 * since you could otherwise interfere with the operation of other
 * synchronization primitives.
 *
 * The `filter` and `callback` functions are called while the queue is
 * locked and must not panic or call into any function in which may call
 * into the context.
 *
 * May only be called in a task.
 *
 * @param ctx context
 * @param key queue key
 * @param filter filter function
 * @param filter_data optional filter data
 * @param callback callback function
 * @param callback_data optional callback data
 * @param result unpark result
 *
 * @return Status code.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoError fi_tasks_ctx_unpark_filter(FiTasksContext ctx, const void *key,
                                                               FiTasksUnparkFilterOp (*filter)(void *, const void *),
                                                               void *filter_data,
                                                               const void *(*callback)(void *, FiTasksUnparkResult),
                                                               void *callback_data, FiTasksUnparkResult *result) {
    return ctx.vtable->v0.unpark_filter(ctx.data, key, filter, filter_data, callback, callback_data, result);
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_COMMAND_BUFFER_H
