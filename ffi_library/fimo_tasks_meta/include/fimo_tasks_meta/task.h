#ifndef FIMO_TASKS_META_TASK_H
#define FIMO_TASKS_META_TASK_H

#include <stdbool.h>

#include <fimo_std/error.h>
#include <fimo_std/time.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Identifier of a task.
typedef FimoUSize FimoTasksMeta_TaskId;

/// A unit of work.
typedef struct FimoTasksMeta_Task {
    /// Optional label of the task.
    ///
    /// May be used by the runtime for tracing purposes. If present, the string must live until
    /// the task instance is destroyed. For dynamically allocated labels this may be done in
    /// the `on_deinit` function. Is not null-terminated.
    const char *label;
    /// Length of the label string.
    FimoUSize label_len;
    /// Entry function of the task.
    void(*on_start)(struct FimoTasksMeta_Task *task);
    /// Optional completion handler of the task.
    ///
    /// Will be invoked after successfull completion of the task on an arbitrary thread.
    void(*on_complete)(struct FimoTasksMeta_Task *task);
    /// Optional abortion handler of the task.
    ///
    /// Will be invoked on an arbitrary thread, if the task is aborted.
    void(*on_abort)(struct FimoTasksMeta_Task *task);
    /// Optional deinitialization routine.
    void(*on_deinit)(struct FimoTasksMeta_Task *task);
} FimoTasksMeta_Task;

/// Returns the id of the current task.
typedef bool(*FimoTasksMeta_task_id)(FimoTasksMeta_TaskId *id);

/// Yields the current task or thread back to the scheduler.
typedef void(*FimoTasksMeta_yield)();

/// Aborts the current task.
typedef void(*FimoTasksMeta_abort)();

/// Puts the current task to sleep for the specified amount of time.
typedef void(*FimoTasksMeta_sleep)(FimoDuration duration);

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_META_TASK_H
