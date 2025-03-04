/// Portable user-space implementation of the linux futex API.
///
/// The API was first detailed by the WebKit developers [[1][1],[2][2], [3][3]].
///
/// [1]: https://webkit.org/blog/6161/locking-in-webkit/
/// [2]: https://trac.webkit.org/browser/webkit/trunk/Source/WTF/wtf/ParkingLot.h
/// [3]: https://docs.rs/parking_lot_core/latest/parking_lot_core/

#ifndef FIMO_TASKS_META_PARKING_LOT_H
#define FIMO_TASKS_META_PARKING_LOT_H

#include <stdbool.h>

#include <fimo_std/error.h>
#include <fimo_std/time.h>

#ifdef __cplusplus
extern "C" {
#endif

/// A value associated with a parked task for filter purposes.
typedef FimoUSize FimoTasksMeta_ParkingLotParkToken;

/// A value which is passed from an unparker to a parked task.
typedef FimoUSize FimoTasksMeta_ParkingLotUnparkToken;

typedef enum FimoTasksMeta_ParkingLotParkResultType : FimoI32 {
    FIMO_TASKS_META_PARKING_LOT_PARK_RESULT_TYPE_UNPARKED,
    FIMO_TASKS_META_PARKING_LOT_PARK_RESULT_TYPE_INVALID,
    FIMO_TASKS_META_PARKING_LOT_PARK_RESULT_TYPE_TIMED_OUT,
} FimoTasksMeta_ParkingLotParkResultType;

/// Result of a park operation.
typedef struct FimoTasksMeta_ParkingLotParkResult {
    FimoTasksMeta_ParkingLotParkResultType type;
    FimoTasksMeta_ParkingLotUnparkToken token;
} FimoTasksMeta_ParkingLotParkResult;

/// Result of a park multiple operation.
typedef struct FimoTasksMeta_ParkingLotParkMultipleResult {
    FimoTasksMeta_ParkingLotParkResultType type;
    FimoI32 index;
    FimoTasksMeta_ParkingLotUnparkToken token;
} FimoTasksMeta_ParkingLotParkMultipleResult;

/// Maximum number of keys allowed for the park multiple operation.
#define FIMO_TASKS_META_PARKING_LOT_MAX_PARK_MULTIPLE_KEY_COUNT 128

/// Result of an unpark operation.
typedef struct FimoTasksMeta_ParkingLotUnparkResult {
    /// Number of tasks that were unparked.
    FimoUSize unparked_tasks;
    /// Number of tasks that were requeued.
    FimoUSize requeued_tasks;
    /// Whether there are any tasks remaining in the queue.
    /// This only returns true if a task was unparked.
    bool has_more_tasks;
    /// This is set to true on average once every 0.5ms for any given key.
    /// It should be used to switch to a fair unlocking mechanism for a particular unlock.
    bool be_fair;
} FimoTasksMeta_ParkingLotUnparkResult;

/// Operation to perform during a requeue.
typedef struct FimoTasksMeta_ParkingLotRequeueOp {
    /// Maximum number of tasks to unpark from the source queue.
    FimoUSize num_tasks_to_unpark;
    /// Maxinum number of tasks to requeue to the destination queue.
    FimoUSize num_tasks_to_requeue;
} FimoTasksMeta_ParkingLotRequeueOp;

/// Operation to perform for a task during filtering.
typedef enum FimoTasksMeta_ParkingLotFilterOp : FimoI32 {
    /// Unpark the task and continue scanning the list of parked tasks.
    FIMO_TASKS_META_PARKING_LOT_FILTER_OP_UNPARK,
    /// Don't unpark the task and continue scanning the list of parked tasks.
    FIMO_TASKS_META_PARKING_LOT_FILTER_OP_SKIP,
    /// Don't unpark the task and stop scanning the list of parked tasks.
    FIMO_TASKS_META_PARKING_LOT_FILTER_OP_STOP,
} FimoTasksMeta_ParkingLotFilterOp;

/// Parks the current task in the queue associated with the given key.
///
/// The `validation` function is called while the queue is locked and can abort the operation by
/// returning false. If `validation` returns true then the current task is appended to the queue
/// and the queue is unlocked.
///
/// The `before_sleep` function is called after the queue is unlocked but before the task is put to
/// sleep. The task will then sleep until it is unparked or the given timeout is reached. Since it
/// is called while the queue is unlocked, it can be used to perform additional operations, as long
/// as `park` is not called recursively.
///
/// The `timed_out` function is also called while the queue is locked, but only if the timeout was
/// reached. It is passed the key of the queue it was in when it timed out, which may be different
/// from the original key if the task was requeued. It is also passed a bool which indicates whether
/// it was the last task in the queue.
typedef FimoTasksMeta_ParkingLotParkResult(*FimoTasksMeta_parking_lot_park)(
    const void *key,
    void *validation_data,
    bool(*validation)(void *data),
    void *before_sleep_data,
    void(*before_sleep)(void *data),
    void *timed_out_data,
    void(*timed_out)(void *data, const void *key, bool is_last),
    FimoTasksMeta_ParkingLotParkToken token,
    const FimoInstant *timeout
);

/// Parks the current task in the queues associated with the given keys.
///
/// A maximum of `FIMO_TASKS_META_PARKING_LOT_MAX_PARK_MULTIPLE_KEY_COUNT` keys may be provided.
///
/// The `validation` function is called while the queue managing the key is locked and can abort
/// the operation by returning false. If `validation` returns true then the current task is
/// appended to the queue and the queue is unlocked.
///
/// The `before_sleep` function is called after the queues are unlocked but before the task is put
/// to sleep. The task will then sleep until it is unparked or the given timeout is reached. Since
/// it is called while the queue is unlocked, it can be used to perform additional operations, as
/// long as `park` or `parkMultiple` is not called recursively.
typedef FimoTasksMeta_ParkingLotParkMultipleResult(*FimoTasksMeta_parking_lot_park_multiple)(
    const void * const *keys,
    FimoUSize key_count,
    void *validation_data,
    bool(*validation)(void *data, FimoUSize key_index),
    void *before_sleep_data,
    void(*before_sleep)(void *data),
    FimoTasksMeta_ParkingLotParkToken token,
    const FimoInstant *timeout
);

/// Unparks one task from the queue associated with the given key.
///
/// The `callback` function is called while the queue is locked and before the target task is woken
/// up. The `result` argument to the function indicates whether a task was found in the queue and
/// whether this was the last task in the queue. This value is also returned by the function.
typedef FimoTasksMeta_ParkingLotUnparkResult(*FimoTasksMeta_parking_lot_unpark_one)(
    const void *key,
    void *callback_data,
    FimoTasksMeta_ParkingLotUnparkToken(*callback)(
        void *data,
        FimoTasksMeta_ParkingLotUnparkResult result
    )
);

/// Unparks all tasks in the queue associated with the given key.
///
/// The given unpark token is passed to all unparked tasks. This function returns the number of
/// tasks that were unparked.
typedef FimoUSize(*FimoTasksMeta_parking_lot_unpark_all)(
    const void *key,
    FimoTasksMeta_ParkingLotUnparkToken token
);

/// Unparks a number of tasks from the front of the queue associated with `key` depending on the
/// results of a filter function which inspects the park token associated with each task.
///
/// The `filter` function is called for each task in the queue or until `.stop` is returned. This
/// function is passed the park token associated with a particular task, which is unparked if
/// `.unpark` is returned.
///
/// The `callback` function is also called while both queues are locked. It is passed a result
/// indicating the number of tasks that were unparked and whether there are still parked tasks in
/// the queue. This result value is also returned by the function.
///
/// The `callback` function should return an unpark token value which will be passed to all tasks
/// that are unparked. If no task is unparked then the returned value is ignored.
typedef FimoTasksMeta_ParkingLotUnparkResult(*FimoTasksMeta_parking_lot_unpark_filter)(
    const void *key,
    void *filter_data,
    FimoTasksMeta_ParkingLotFilterOp(*filter)(
        void *data,
        FimoTasksMeta_ParkingLotParkToken token
    ),
    void *callback_data,
    FimoTasksMeta_ParkingLotUnparkToken(*callback)(
        void *data,
        FimoTasksMeta_ParkingLotUnparkResult result
    )
);

/// Removes tasks from the queue associated with `key_from`, and requeues them onto the queue
/// associated with `key_to`.
///
/// The `validate` function is called while both queues are locked. Its return value will determine
/// the maximum number or tasks to unpark, and the maximum number of tasks to requeue onto the
/// target queue.
///
/// The `callback` function is also called while both queues are locked. It is passed the result of
/// the `validate` function, and a `result`, indicating the number of unparked and requeued tasks.
/// The result will also be returned as the result of the function. The resulting unpark token will
/// be passed to the unparked task, or will be ignored if no task was unparked.
typedef FimoTasksMeta_ParkingLotUnparkResult(*FimoTasksMeta_parking_lot_unpark_requeue)(
    const void *key_from,
    const void *key_to,
    void *validate_data,
    FimoTasksMeta_ParkingLotRequeueOp(*validate)(void *data),
    void *callback_data,
    FimoTasksMeta_ParkingLotUnparkToken(*callback)(
        void *data,
        FimoTasksMeta_ParkingLotRequeueOp op,
        FimoTasksMeta_ParkingLotUnparkResult result
    )
);

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_META_PARKING_LOT_H
