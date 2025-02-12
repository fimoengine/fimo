#ifndef FIMO_TASKS_META_TASK_LOCAL_H
#define FIMO_TASKS_META_TASK_LOCAL_H

/// A key for a task-specific-storage.
///
/// A new key can be defined by casting from a stable address.
typedef struct FimoTasksMeta_TaskLocalKey FimoTasksMeta_TaskLocalKey;

#ifdef __cplusplus
extern "C" {
#endif

/// Associates a value with the key for the current task.
///
/// The current value associated with the key is replaced with the new value without
/// invoking any destructor function. The destructor function is set to `dtor`, and will
/// be invoked upon task exit. May only be called by a task.
typedef void(*FimoTasksMeta_task_local_set)(
    const FimoTasksMeta_TaskLocalKey *key,
    void *value,
    void(*dtor)(void *value)
);

/// Returns the value associated to the key for the current task.
///
/// May only be called by a task.
typedef void*(*FimoTasksMeta_task_local_get)(const FimoTasksMeta_TaskLocalKey *key);

/// Clears the value of the current task associated with the key.
///
/// This operation invokes the associated destructor function and sets the value to `null`.
/// May only be called by a task.
typedef void(*FimoTasksMeta_task_local_clear)(const FimoTasksMeta_TaskLocalKey *key);

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_META_TASK_LOCAL_H
