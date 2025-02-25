//! Portable user-space implementation of the linux futex API.
//!
//! The API was first detailed by the WebKit developers [[1][1],[2][2], [3][3]].
//!
//! [1]: https://webkit.org/blog/6161/locking-in-webkit/
//! [2]: https://trac.webkit.org/browser/webkit/trunk/Source/WTF/wtf/ParkingLot.h
//! [3]: https://docs.rs/parking_lot_core/latest/parking_lot_core/
const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;

const symbols = @import("../symbols.zig");

/// A value associated with a parked task for filter purposes.
pub const ParkToken = enum(usize) {
    _,

    pub const default: ParkToken = @enumFromInt(0);
};

/// A value which is passed from an unparker to a parked task.
pub const UnparkToken = enum(usize) {
    _,

    pub const default: UnparkToken = @enumFromInt(0);
};

/// Result of a park operation.
pub const ParkResult = extern struct {
    type: enum(i32) { unparked, invalid, timed_out } = .invalid,
    token: UnparkToken = .default,
};

/// Result of a park multiple operation.
pub const ParkMultipleResult = extern struct {
    type: enum(i32) { unparked, invalid, timed_out, keys_invalid } = .invalid,
    index: u32 = 0,
    token: UnparkToken = .default,
};

/// Maximum number of keys allowed for the park multiple operation.
pub const max_park_multiple_key_count = 128;

/// Result of an unpark operation.
pub const UnparkResult = extern struct {
    /// Number of tasks that were unparked.
    unparked_tasks: usize = 0,
    /// Number of tasks that were requeued.
    requeued_tasks: usize = 0,
    /// Whether there are any tasks remaining in the queue.
    /// This only returns true if a task was unparked.
    has_more_tasks: bool = false,
    /// This is set to true on average once every 0.5ms for any given key.
    /// It should be used to switch to a fair unlocking mechanism for a particular unlock.
    be_fair: bool = false,
};

/// Operation that `unparkRequeue` should perform.
pub const RequeueOp = extern struct {
    /// Maximum number of tasks to unpark from the source queue.
    num_tasks_to_unpark: usize = 0,
    /// Maxinum number of tasks to requeue to the destination queue.
    num_tasks_to_requeue: usize = 0,
};

/// Operation that `unparkFilter` should perform for each task.
pub const FilterOp = enum(i32) {
    /// Unpark the task and continue scanning the list of parked tasks.
    unpark,
    /// Don't unpark the task and continue scanning the list of parked tasks.
    skip,
    /// Don't unpark the task and stop scanning the list of parked tasks.
    stop,
};

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
pub fn park(
    provider: anytype,
    key: *const anyopaque,
    validation_data: anytype,
    validation: fn (data: *@TypeOf(validation_data)) bool,
    before_sleep_data: anytype,
    before_sleep: fn (data: *@TypeOf(before_sleep_data)) void,
    timed_out_data: anytype,
    timed_out: fn (data: *@TypeOf(timed_out_data), key: *const anyopaque, is_last: bool) void,
    token: ParkToken,
    timeout: ?Instant,
) ParkResult {
    const Validation = struct {
        fn f(data: *anyopaque) callconv(.c) bool {
            return validation(@ptrCast(@alignCast(data)));
        }
    };
    const BeforeSleep = struct {
        fn f(data: *anyopaque) callconv(.c) void {
            return before_sleep(@ptrCast(@alignCast(data)));
        }
    };
    const TimedOut = struct {
        fn f(data: *anyopaque, k: *const anyopaque, is_last: bool) callconv(.c) void {
            return timed_out(@ptrCast(@alignCast(data)), k, is_last);
        }
    };
    var validation_data_ = validation_data;
    var before_sleep_data_ = before_sleep_data;
    var timed_out_data_ = timed_out_data;
    const timeout_ = if (timeout) |t| t.intoC() else null;
    const timeout_ptr = if (timeout_) |t| &t else null;
    const sym = symbols.parking_lot_park.requestFrom(provider);
    return sym(
        key,
        &validation_data_,
        &Validation.f,
        &before_sleep_data_,
        &BeforeSleep.f,
        &timed_out_data_,
        &TimedOut.f,
        token,
        timeout_ptr,
    );
}

/// Parks the current task in the queues associated with the given keys.
///
/// A maximum of `max_park_multiple_key_count` keys may be provided.
///
/// The `validation` function is called while the queue managing the key is locked and can abort
/// the operation by returning false. If `validation` returns true then the current task is
/// appended to the queue and the queue is unlocked.
///
/// The `before_sleep` function is called after the queues are unlocked but before the task is put
/// to sleep. The task will then sleep until it is unparked or the given timeout is reached. Since
/// it is called while the queue is unlocked, it can be used to perform additional operations, as
/// long as `park` or `parkMultiple` is not called recursively.
pub fn parkMultiple(
    provider: anytype,
    keys: []*const anyopaque,
    validation_data: anytype,
    validation: fn (data: *@TypeOf(validation_data), key_index: usize) bool,
    before_sleep_data: anytype,
    before_sleep: fn (data: *@TypeOf(before_sleep_data)) void,
    token: ParkToken,
    timeout: ?Instant,
) ParkMultipleResult {
    const Validation = struct {
        fn f(data: *anyopaque, key_index: usize) callconv(.c) bool {
            return validation(@ptrCast(@alignCast(data)), key_index);
        }
    };
    const BeforeSleep = struct {
        fn f(data: *anyopaque) callconv(.c) void {
            return before_sleep(@ptrCast(@alignCast(data)));
        }
    };
    var validation_data_ = validation_data;
    var before_sleep_data_ = before_sleep_data;
    const timeout_ = if (timeout) |t| t.intoC() else null;
    const timeout_ptr = if (timeout_) |t| &t else null;
    const sym = symbols.parking_lot_park_multiple.requestFrom(provider);
    return sym(
        keys,
        &validation_data_,
        &Validation.f,
        &before_sleep_data_,
        &BeforeSleep.f,
        token,
        timeout_ptr,
    );
}

/// Unparks one task from the queue associated with the given key.
///
/// The `callback` function is called while the queue is locked and before the target task is woken
/// up. The `result` argument to the function indicates whether a task was found in the queue and
/// whether this was the last task in the queue. This value is also returned by `unparkOne`.
pub fn unparkOne(
    provider: anytype,
    key: *const anyopaque,
    callback_data: anytype,
    callback: fn (data: *@TypeOf(callback_data), result: UnparkResult) UnparkToken,
) UnparkResult {
    const Callback = struct {
        fn f(data: *anyopaque, result: UnparkResult) callconv(.c) UnparkToken {
            return callback(@ptrCast(@alignCast(data)), result);
        }
    };
    var callback_data_ = callback_data;
    const sym = symbols.parking_lot_unpark_one.requestFrom(provider);
    return sym(key, &callback_data_, &Callback.f);
}

/// Unparks all tasks in the queue associated with the given key.
///
/// The given unpark token is passed to all unparked tasks. This function returns the number of
/// tasks that were unparked.
pub fn unparkAll(provider: anytype, key: *const anyopaque, token: UnparkToken) usize {
    const sym = symbols.parking_lot_unpark_all.requestFrom(provider);
    return sym(key, token);
}

/// Unparks a number of tasks from the front of the queue associated with `key` depending on the
/// results of a filter function which inspects the park token associated with each task.
///
/// The `filter` function is called for each task in the queue or until `.stop` is returned. This
/// function is passed the park token associated with a particular task, which is unparked if
/// `.unpark` is returned.
///
/// The `callback` function is also called while both queues are locked. It is passed a result
/// indicating the number of tasks that were unparked and whether there are still parked tasks in
/// the queue. This result value is also returned by `unparkFilter`.
///
/// The `callback` function should return an unpark token value which will be passed to all tasks
/// that are unparked. If no task is unparked then the returned value is ignored.
pub fn unparkFilter(
    provider: anytype,
    key: *const anyopaque,
    filter_data: anytype,
    filter: fn (data: *@TypeOf(filter_data), token: ParkToken) FilterOp,
    callback_data: anytype,
    callback: fn (data: *@TypeOf(callback_data), result: UnparkResult) UnparkToken,
) UnparkResult {
    const Filter = struct {
        fn f(data: *anyopaque, token: ParkToken) callconv(.c) FilterOp {
            return filter(@ptrCast(@alignCast(data)), token);
        }
    };
    const Callback = struct {
        fn f(data: *anyopaque, result: UnparkResult) callconv(.c) UnparkToken {
            return callback(@ptrCast(@alignCast(data)), result);
        }
    };
    var filter_data_ = filter_data;
    var callback_data_ = callback_data;
    const sym = symbols.parking_lot_unpark_filter.requestFrom(provider);
    return sym(key, &filter_data_, &Filter.f, &callback_data_, &Callback.f);
}

/// Removes tasks from the queue associated with `key_from`, and requeues them onto the queue
/// associated with `key_to`.
///
/// The `validate` function is called while both queues are locked. Its return value will determine
/// the maximum number or tasks to unpark, and the maximum number of tasks to requeue onto the
/// target queue.
///
/// The `callback` function is also called while both queues are locked. It is passed the result of
/// the `validate` function, and a `result`, indicating the number of unparked and requeued tasks.
/// The result will also be returned as the result of the `unparkRequeue` function. The resulting
/// unpark token will be passed to the unparked task, or will be ignored if no task was unparked.
pub fn unparkRequeue(
    provider: anytype,
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    validate_data: anytype,
    validate: fn (data: *@TypeOf(validate_data)) RequeueOp,
    callback_data: anytype,
    callback: fn (data: *@TypeOf(callback_data), op: RequeueOp, result: UnparkResult) UnparkToken,
) UnparkResult {
    const Validate = struct {
        fn f(data: *anyopaque) callconv(.c) RequeueOp {
            return validate(@ptrCast(@alignCast(data)));
        }
    };
    const Callback = struct {
        fn f(data: *anyopaque, op: RequeueOp, result: UnparkResult) callconv(.c) UnparkToken {
            return callback(@ptrCast(@alignCast(data)), op, result);
        }
    };
    var validate_data_ = validate_data;
    var callback_data_ = callback_data;
    const sym = symbols.parking_lot_unpark_requeue.requestFrom(provider);
    return sym(key_from, key_to, &validate_data_, &Validate.f, &callback_data_, &Callback.f);
}
