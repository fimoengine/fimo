#ifndef FIMO_TASKS_META_TASK_H
#define FIMO_TASKS_META_TASK_H

#include <stdbool.h>

#include <fimo_std.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Identifier of a task.
typedef FSTD_USize FimoTasksMeta_TaskId;

/// A unit of work.
typedef struct FimoTasksMeta_Task {
    /// Optional label of the task.
    ///
    /// May be used by the runtime for tracing purposes.
    /// If present, the string must live until the task instance is destroyed.
    /// Is not null-terminated.
    const char *label;
    /// Length of the label string.
    FSTD_USize label_len;
    /// Entry function of the task.
    void (*run)(struct FimoTasksMeta_Task *task);
} FimoTasksMeta_Task;

/// Returns the id of the current task.
typedef bool (*FimoTasksMeta_task_id)(FimoTasksMeta_TaskId *id);

/// Yields the current task or thread back to the scheduler.
typedef void (*FimoTasksMeta_yield)();

/// Aborts the current task.
typedef void (*FimoTasksMeta_abort)();

/// Reports whether a cancellation of the current task has been requested.
typedef bool (*FimoTasksMeta_cancel_requested)();

/// Puts the current task to sleep for the specified amount of time.
typedef void (*FimoTasksMeta_sleep)(FSTD_Duration duration);


/// A key for a task-specific-storage.
///
/// A new key can be defined by casting from a stable address.
typedef struct FimoTasksMeta_TaskLocalKey FimoTasksMeta_TaskLocalKey;

/// Associates a value with the key for the current task.
///
/// The current value associated with the key is replaced with the new value without
/// invoking any destructor function. The destructor function is set to `dtor`, and will
/// be invoked upon task exit. May only be called by a task.
typedef void (*FimoTasksMeta_task_local_set)(const FimoTasksMeta_TaskLocalKey *key, void *value,
                                             void (*dtor)(void *value));

/// Returns the value associated to the key for the current task.
///
/// May only be called by a task.
typedef void *(*FimoTasksMeta_task_local_get)(const FimoTasksMeta_TaskLocalKey *key);

/// Clears the value of the current task associated with the key.
///
/// This operation invokes the associated destructor function and sets the value to `null`.
/// May only be called by a task.
typedef void (*FimoTasksMeta_task_local_clear)(const FimoTasksMeta_TaskLocalKey *key);

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_META_TASK_H
