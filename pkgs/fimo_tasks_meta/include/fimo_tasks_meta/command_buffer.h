#ifndef FIMO_TASKS_META_COMMAND_BUFFER_H
#define FIMO_TASKS_META_COMMAND_BUFFER_H

#include <assert.h>
#include <stdalign.h>
#include <stdbool.h>

#include <fimo_std.h>

#include <fimo_tasks_meta/pool.h>
#include <fimo_tasks_meta/tasks.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FimoTasksMeta_CommandBuffer FimoTasksMeta_CommandBuffer;

/// Completion status of a command buffer handle.
typedef enum FimoTasksMeta_CommandBufferHandleCompletionStatus {
    FIMO_TASKS_META_COMMAND_BUFFER_HANDLE_COMPLETION_STATUS_COMPLETED,
    FIMO_TASKS_META_COMMAND_BUFFER_HANDLE_COMPLETION_STATUS_ABORTED,
} FimoTasksMeta_CommandBufferHandleCompletionStatus;

/// VTable of a command buffer handle.
typedef struct FimoTasksMeta_CommandBufferHandleVTable {
    /// Acquires a reference to the handle.
    void (*acquire)(void *handle);
    /// Releases the reference to the handle.
    void (*release)(void *handle);
    /// Returns a reference to the worker pool owning the handle.
    FimoTasksMeta_Pool (*owner_pool)(void *handle);
    /// Waits for the completion of the command buffer.
    FimoTasksMeta_CommandBufferHandleCompletionStatus (*wait_on)(void *handle);
} FimoTasksMeta_CommandBufferHandleVTable;

/// A handle to an enqueued command buffer.
typedef struct FimoTasksMeta_CommandBufferHandle {
    void *data;
    const FimoTasksMeta_CommandBufferHandleVTable *vtable;
} FimoTasksMeta_CommandBufferHandle;

/// Type of an entry of a command buffer.
typedef enum FimoTasksMeta_CommandBufferEntryType : FSTD_I32 {
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_ABORT_ON_ERROR,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_SET_MIN_STACK_SIZE,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_SELECT_WORKER,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_SELECT_ANY_WORKER,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_ENQUEUE_TASK,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_ENQUEUE_COMMAND_BUFFER,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_WAIT_ON_BARRIER,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_WAIT_ON_COMMAND_BUFFER,
    FIMO_TASKS_META_COMMAND_BUFFER_ENTRY_TYPE_WAIT_ON_COMMAND_INDIRECT,
} FimoTasksMeta_CommandBufferEntryType;

/// Payload of an entry of a command buffer.
typedef union FimoTasksMeta_CommandBufferEntryPayload {
    /// Configures whether to abort the following commands if any of them errors.
    bool abort_on_error;
    /// Specifies the minimum stack size for the following tasks.
    FimoTasksMeta_PoolStackSize set_min_stack_size;
    /// Specifies that the following tasks may only be enqueued on the provided worker.
    FimoTasksMeta_PoolWorker select_worker;
    /// Specifies that the following tasks may be enqueued on any worker of the pool.
    ///
    /// The value is ignored.
    FSTD_U8 select_any_worker;
    /// Enqueues a task.
    ///
    /// The command will complete when the task is completed.
    FimoTasksMeta_Task *enqueue_task;
    /// Enqueues a sub command buffer.
    ///
    /// The command will complete when the sub command buffer is completed.
    FimoTasksMeta_CommandBuffer *enqueue_command_buffer;
    /// Waits for the completion of all preceding commands.
    ///
    /// The value is ignored.
    FSTD_U8 wait_on_barrier;
    /// Waits for the completion of the command buffer handle.
    FimoTasksMeta_CommandBufferHandle wait_on_command_buffer;
    /// Waits for the completion of some specific command contained in the buffer.
    ///
    /// Waits for the completion of the command at index `this_command - value`.
    FSTD_USize wait_on_command_indirect;
} FimoTasksMeta_CommandBufferEntryPayload;

static_assert(sizeof(FimoTasksMeta_CommandBufferEntryPayload) == sizeof(FSTD_USize[2]), "");
static_assert(alignof(FimoTasksMeta_CommandBufferEntryPayload) <= alignof(FSTD_USize), "");

/// An entry of a command buffer.
typedef struct FimoTasksMeta_CommandBufferEntry {
    FimoTasksMeta_CommandBufferEntryType type;
    FimoTasksMeta_CommandBufferEntryPayload payload;
} FimoTasksMeta_CommandBufferEntry;

/// A list of commands to process by a worker pool.
typedef struct FimoTasksMeta_CommandBuffer {
    /// Optional label of the command buffer.
    ///
    /// May be used by the runtime for tracing purposes. If present, the string must live until
    /// the task instance is destroyed. For dynamically allocated labels this may be done in
    /// the `on_deinit` function. Is not null-terminated.
    const char *label;
    /// Length of the label string.
    FSTD_USize label_len;
    /// List of commands.
    const FimoTasksMeta_CommandBufferEntry *entries;
    /// Length of the command list.
    FSTD_USize entries_len;
    /// Optional completion handler of the command buffer.
    ///
    /// Will be invoked after successfull completion of the command bufer on an arbitrary
    /// thread.
    void (*on_complete)(FimoTasksMeta_CommandBuffer *buffer);
    /// Optional abortion handler of the command buffer..
    ///
    /// Will be invoked on an arbitrary thread, if the command buffer is aborted.
    void (*on_abort)(FimoTasksMeta_CommandBuffer *buffer);
    /// Optional deinitialization routine.
    void (*on_deinit)(FimoTasksMeta_CommandBuffer *buffer);
} FimoTasksMeta_CommandBuffer;

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_META_COMMAND_BUFFER_H
