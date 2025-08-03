//! Condition variables are used with a Mutex to efficiently wait for an arbitrary condition to occur.
//! It does this by atomically unlocking the mutex, blocking the thread until notified, and finally re-locking the mutex.
//! Condition can be statically initialized and is at most `@sizeOf(u32)` large.
const std = @import("std");
const atomic = std.atomic;
const math = std.math;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;

const Futex = @import("Futex.zig");
const Mutex = @import("Mutex.zig");

const Condition = @This();

futex: atomic.Value(u32) = .init(0),

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
pub fn wait(self: *Condition, mutex: *Mutex) void {
    self.waitInternal(mutex, null) catch unreachable;
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
    mutex: *Mutex,
    timeout: Duration,
) error{Timeout}!void {
    const timeout_time = Instant.now().addSaturating(timeout);
    try self.waitInternal(mutex, timeout_time);
}

/// Unblocks at least one task blocked in a call to `wait()` or `timedWait()` with a given Mutex.
/// The blocked task must be sequenced before this call with respect to acquiring the same Mutex in order to be observable for unblocking.
/// `signal()` can be called with or without the relevant Mutex being acquired and have no "effect" if there's no observable blocked threads.
pub fn signal(self: *Condition) void {
    _ = self.futex.fetchAdd(1, .monotonic);
    _ = Futex.wake(&self.futex, 1);
}

/// Unblocks all tasks currently blocked in a call to `wait()` or `timedWait()` with a given Mutex.
/// The blocked tasks must be sequenced before this call with respect to acquiring the same Mutex in order to be observable for unblocking.
/// `broadcast()` can be called with or without the relevant Mutex being acquired and have no "effect" if there's no observable blocked threads.
pub fn broadcast(self: *Condition) void {
    _ = self.futex.fetchAdd(1, .monotonic);
    _ = Futex.wake(&self.futex, std.math.maxInt(usize));
}

fn waitInternal(
    self: *Condition,
    mutex: *Mutex,
    timeout: ?Instant,
) error{Timeout}!void {
    // Examine the notification counter _before_ we unlock the mutex.
    const futex_value = self.futex.load(.monotonic);

    // Unlock the mutex before going to sleep.
    mutex.unlock();
    defer mutex.lock();

    // Wait, but only if there hasn't been any
    // notification since we unlocked the mutex.
    if (timeout) |t|
        Futex.TypedHelper(u32).timedWait(&self.futex, futex_value, 0, t) catch |err| switch (err) {
            error.Timeout => return error.Timeout,
            error.Invalid => {},
        }
    else
        Futex.TypedHelper(u32).wait(&self.futex, futex_value, 0) catch {};
}
