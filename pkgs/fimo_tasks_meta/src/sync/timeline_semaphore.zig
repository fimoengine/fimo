const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;

const Futex = @import("Futex.zig");

/// A monotonically increasing counter that can be awaited and signaled.
pub const TimelineSemaphore = extern struct {
    state: atomic.Value(u64) = .init(0),

    /// Initializes the semaphore with a custom initial value.
    pub fn init(value: u64) TimelineSemaphore {
        return .{ .state = .init(value) };
    }

    /// Returns the current counter of the semaphore.
    pub fn counter(self: *const TimelineSemaphore) u64 {
        return self.state.load(.acquire);
    }

    /// Checks if the semaphore is signaled with a count greater or equal to `value`.
    pub fn isSignaled(self: *const TimelineSemaphore, value: u64) bool {
        return self.state.load(.acquire) >= value;
    }

    /// Blocks the caller until the semaphore reaches a count greater or equal to `value`.
    pub fn wait(self: *TimelineSemaphore, value: u64) void {
        if (!self.isSignaled(value)) _ = self.waitSlow(value, null);
    }

    /// Blocks the caller until the semaphore reaches a count greater or equal to `value`, or the timeout expires.
    pub fn timedWait(
        self: *TimelineSemaphore,
        value: u64,
        timeout: Duration,
    ) error{Timeout}!void {
        if (!self.isSignaled(value)) {
            const timeout_time = Instant.now().add(timeout) catch null;
            if (!self.waitSlow(value, timeout_time)) return error.Timeout;
        }
    }

    fn waitSlow(self: *TimelineSemaphore, value: u64, timeout: ?Instant) bool {
        @branchHint(.cold);
        var state = self.state.load(.monotonic);
        while (true) {
            if (state >= value) {
                _ = self.state.load(.acquire);
                return true;
            }

            if (timeout) |t| {
                Futex.TypedHelper(u64).timedWait(
                    &self.state,
                    state,
                    if (comptime @sizeOf(u64) <= @sizeOf(usize)) value else @intFromPtr(&value),
                    t,
                ) catch |err| switch (err) {
                    error.Invalid => {
                        state = self.state.load(.monotonic);
                        continue;
                    },
                    error.Timeout => return false,
                };
            } else {
                Futex.TypedHelper(u64).wait(
                    &self.state,
                    state,
                    if (comptime @sizeOf(u64) <= @sizeOf(usize)) value else @intFromPtr(&value),
                ) catch |err| switch (err) {
                    error.Invalid => {
                        state = self.state.load(.monotonic);
                        continue;
                    },
                };
            }

            return true;
        }
    }

    /// Sets the internal value of the semaphore, possibly waking waiting tasks.
    ///
    /// `value` must be greater than the current value of the semaphore.
    pub fn signal(self: *TimelineSemaphore, value: u64) void {
        std.debug.assert(self.state.load(.monotonic) < value);
        self.state.store(value, .release);

        const filter = if (comptime @sizeOf(u64) <= @sizeOf(usize))
            // @as(u64, token) <= @as(u64, value)
            Futex.Filter{
                .op = .{
                    .token_op = .noop,
                    .token_type = .u64,
                    .cmp_op = .le,
                    .cmp_arg_op = .noop,
                },
                .token_mask = ~@as(usize, 0),
                .cmp_arg = value,
            }
        else
            // @as(*const u64, token).* <= @as(*const u64, value).*
            Futex.Filter{
                .op = .{
                    .token_op = .deref,
                    .token_type = .u64,
                    .cmp_op = .le,
                    .cmp_arg_op = .deref,
                },
                .token_mask = ~@as(usize, 0),
                .cmp_arg = @intFromPtr(&value),
            };
        _ = Futex.wakeFilter(&self.state, std.math.maxInt(usize), filter);
    }
};
