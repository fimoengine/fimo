//! A concurrent task backed by an executor.

const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const Duration = fimo_std.time.Duration;
const Instant = fimo_std.time.Instant;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Futex = fimo_tasks_meta.sync.Futex;

/// A fence to synchronize the execution of individual jobs.
pub const Fence = extern struct {
    state: atomic.Value(u8) = .init(unsignaled),

    pub const unsignaled: u8 = 0b00;
    pub const signaled: u8 = 0b01;
    pub const contended: u8 = 0b10;

    /// Checks if the fence is already signaled.
    pub fn isSignaled(self: *const Fence) bool {
        return self.state.load(.acquire) & signaled != 0;
    }

    /// Blocks the caller until the fence is signaled.
    pub fn wait(self: *Fence) void {
        while (true) {
            const state = self.state.load(.monotonic);
            if (state & signaled != 0)
                if (self.state.load(.acquire) & signaled != 0) return;

            if (state & contended == 0) {
                if (self.state.cmpxchgWeak(
                    unsignaled,
                    contended,
                    .monotonic,
                    .monotonic,
                )) |_| continue;
            }

            Futex.TypedHelper(u8).wait(&self.state, contended, 0) catch {};
        }
    }

    /// Wakes all waiters of the fence.
    pub fn signal(self: *Fence) void {
        const state = self.state.swap(signaled, .release);
        if (state & contended != 0) _ = Futex.wake(&self.state, std.math.maxInt(usize));
    }

    /// Resets the state of the fence to be unsignaled.
    pub fn reset(self: *Fence) void {
        const state = self.state.fetchAnd(~signaled, .release);
        std.debug.assert(state != (signaled | contended));
    }
};

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
