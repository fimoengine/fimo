//! Condition variables are used with a Mutex to efficiently wait for an arbitrary condition to occur.
//! It does this by atomically unlocking the mutex, blocking the thread until notified, and finally re-locking the mutex.
//! Condition can be statically initialized and is at most `@sizeOf(*anyopaque)` large.
const std = @import("std");
const atomic = std.atomic;
const math = std.math;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Time = time.Time;

const Mutex = @import("Mutex.zig");
const ParkingLot = @import("ParkingLot.zig");

const Condition = @This();
state: atomic.Value(?*Mutex) = .init(null),

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
///
/// May only be called from within a task.
pub fn wait(self: *Condition, provider: anytype, mutex: *Mutex) void {
    self.waitInternal(provider, mutex, null) catch unreachable;
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
/// - a future call to `signal()` or `broadcast()` which has acquired the Mutex and is sequenced after this `timedWait()`.
///
/// Given `timedWait()` can be interrupted spuriously, the blocking condition should be checked continuously
/// irrespective of any notifications from `signal()` or `broadcast()`.
///
/// May only be called from within a task.
pub fn timedWait(
    self: *Condition,
    provider: anytype,
    mutex: *Mutex,
    timeout: Duration,
) error{Timeout}!void {
    const timeout_time = Time.now().addSaturating(timeout);
    try self.waitInternal(provider, mutex, timeout_time);
}

/// Unblocks at least one task blocked in a call to `wait()` or `timedWait()` with a given Mutex.
/// The blocked task must be sequenced before this call with respect to acquiring the same Mutex in order to be observable for unblocking.
/// `signal()` can be called with or without the relevant Mutex being acquired and have no "effect" if there's no observable blocked threads.
pub fn signal(self: *Condition, provider: anytype) void {
    const mutex = self.state.load(.monotonic) orelse return;
    self.signalSlow(provider, mutex);
}

/// Unblocks all tasks currently blocked in a call to `wait()` or `timedWait()` with a given Mutex.
/// The blocked tasks must be sequenced before this call with respect to acquiring the same Mutex in order to be observable for unblocking.
/// `broadcast()` can be called with or without the relevant Mutex being acquired and have no "effect" if there's no observable blocked threads.
pub fn broadcast(self: *Condition, provider: anytype) void {
    const mutex = self.state.load(.monotonic) orelse return;
    self.broadcastSlow(provider, mutex);
}

fn waitInternal(
    self: *Condition,
    provider: anytype,
    mutex: *Mutex,
    timeout: ?Time,
) error{Timeout}!void {
    const Validation = struct {
        condition: *Condition,
        mutex: *Mutex,
        fn f(this: *@This()) bool {
            if (this.condition.state.load(.monotonic)) |state| {
                if (state != this.mutex) {
                    @panic("attempted to use a condition variable with more than one mutex");
                }
            } else {
                this.condition.state.store(this.mutex, .monotonic);
            }
            return true;
        }
    };
    const BeforeSleep = struct {
        provider: @TypeOf(provider),
        mutex: *Mutex,
        fn f(this: *@This()) void {
            this.mutex.unlock(this.provider);
        }
    };
    const TimedOut = struct {
        condition: *Condition,
        was_requeued: *bool,
        fn f(this: *@This(), key: *const anyopaque, is_last: bool) void {
            this.was_requeued.* = key != this.condition;
            if (!this.was_requeued.* and is_last) this.condition.state.store(null, .monotonic);
        }
    };

    var was_requeued: bool = false;
    const result = ParkingLot.park(
        provider,
        self,
        Validation{ .condition = self, .mutex = mutex },
        Validation.f,
        BeforeSleep{ .provider = provider, .mutex = mutex },
        BeforeSleep.f,
        TimedOut{ .condition = self, .was_requeued = &was_requeued },
        TimedOut.f,
        .default,
        timeout,
    );

    // Relock the mutex.
    if (result.token != @FieldType(Mutex, "impl").handoff_token) mutex.lock(provider);
    if (!(result.type == .unparked or was_requeued)) return error.Timeout;
}

fn signalSlow(self: *Condition, provider: anytype, mutex: *Mutex) void {
    @branchHint(.cold);

    const Validate = struct {
        condition: *Condition,
        mutex: *Mutex,
        fn f(this: *@This()) ParkingLot.RequeueOp {
            if (this.condition.state.load(.monotonic) != this.mutex) return ParkingLot.RequeueOp{};

            // If the mutex is unlocked we unpark one task, otherwise we requeue it onto the mutex.
            return if (this.mutex.impl.markParkedILocked())
                ParkingLot.RequeueOp{ .num_tasks_to_requeue = 1 }
            else
                ParkingLot.RequeueOp{ .num_tasks_to_unpark = 1 };
        }
    };
    const Callback = struct {
        condition: *Condition,
        fn f(
            this: *@This(),
            op: ParkingLot.RequeueOp,
            result: ParkingLot.UnparkResult,
        ) ParkingLot.UnparkToken {
            _ = op;
            // If there aren't any waiters left we clear the state of the condition.
            if (!result.has_more_tasks) this.condition.state.store(null, .monotonic);
            return .default;
        }
    };

    _ = ParkingLot.unparkRequeue(
        provider,
        self,
        mutex,
        Validate{ .condition = self, .mutex = mutex },
        Validate.f,
        Callback{ .condition = self },
        Callback.f,
    );
}

fn broadcastSlow(self: *Condition, provider: anytype, mutex: *Mutex) void {
    @branchHint(.cold);

    const Validate = struct {
        condition: *Condition,
        mutex: *Mutex,
        fn f(this: *@This()) ParkingLot.RequeueOp {
            if (this.condition.state.load(.monotonic) != this.mutex) return ParkingLot.RequeueOp{};
            this.condition.state.store(null, .monotonic);

            // If the mutex is unlocked we unpark one task otherwise we requeue the remaining tasks
            // onto the mutex.
            return if (this.mutex.impl.markParkedILocked())
                ParkingLot.RequeueOp{ .num_tasks_to_requeue = math.maxInt(usize) }
            else
                ParkingLot.RequeueOp{
                    .num_tasks_to_unpark = 1,
                    .num_tasks_to_requeue = math.maxInt(usize),
                };
        }
    };
    const Callback = struct {
        mutex: *Mutex,
        fn f(
            this: *@This(),
            op: ParkingLot.RequeueOp,
            result: ParkingLot.UnparkResult,
        ) ParkingLot.UnparkToken {
            // Mark the mutex as parked, if we requeued one task onto the mutex.
            if (op.num_tasks_to_unpark == 1 and result.requeued_tasks != 0) {
                this.mutex.impl.markParked();
            }
            return .default;
        }
    };

    _ = ParkingLot.unparkRequeue(
        provider,
        self,
        mutex,
        Validate{ .condition = self, .mutex = mutex },
        Validate.f,
        Callback{ .mutex = mutex },
        Callback.f,
    );
}
