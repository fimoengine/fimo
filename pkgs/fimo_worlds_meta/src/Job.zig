//! A concurrent task backed by an executor.

const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const Duration = fimo_std.time.Duration;
const Instant = fimo_std.time.Instant;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const future = fimo_tasks_meta.future;
const Handle = fimo_tasks_meta.command_buffer.Handle;
const StackSize = fimo_tasks_meta.pool.StackSize;
const Worker = fimo_tasks_meta.pool.Worker;
const Pool = fimo_tasks_meta.pool.Pool;
const Futex = fimo_tasks_meta.sync.Futex;
const ParkingLot = fimo_tasks_meta.sync.ParkingLot;

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

/// Options for spawning a new job.
pub const SpawnOptions = struct {
    /// Executor of the job.
    executor: Pool,
    /// Allocator used to spawn the job.
    ///
    /// Must outlive the job.
    allocator: Allocator,
    /// Label of the underlying job command buffer.
    label: ?[]const u8 = null,
    /// Minimum stack size of the job.
    stack_size: ?StackSize = null,
    /// Worker to assign the job to.
    worker: ?Worker = null,
    /// List of dependencies to wait for before starting the job.
    ///
    /// The job will be aborted if any one dependency fails.
    /// Each handle must belong to the same pool as the one the job will be spawned in.
    dependencies: []const *const Handle = &.{},
    /// List of fences to wait for before starting the job.
    fences: []const *Fence = &.{},
    /// List of semaphores to wait for before starting the job.
    semaphores: []const TimelineSemaphoreInfo = &.{},
    /// Optional object to signal upon completion of the job.
    signal: ?SignalObject = null,

    /// Information required for a wait/signal operation on a semaphore.
    pub const TimelineSemaphoreInfo = struct {
        semaphore: *TimelineSemaphore,
        counter: u64,
    };

    /// Object that can be signaled at the end of a job.
    pub const SignalObject = union(enum) {
        fence: *Fence,
        timeline_semaphore: TimelineSemaphoreInfo,
        _,
    };
};

/// Spawns a new job.
pub fn go(
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    options: SpawnOptions,
) future.SpawnError!void {
    const fences = try options.allocator.dupe(*Fence, options.fences);
    const semaphores = try options.allocator.dupe(SpawnOptions.TimelineSemaphoreInfo, options.semaphores);
    const Wrapper = struct {
        fn start(
            wait: []*Fence,
            wait_sem: []SpawnOptions.TimelineSemaphoreInfo,
            args_: std.meta.ArgsTuple(@TypeOf(function)),
        ) void {
            for (wait) |f| f.wait();
            for (wait_sem) |i| i.semaphore.wait(i.counter);
            @call(.auto, function, args_);
        }

        fn cleanup(
            allocator: Allocator,
            f: []*Fence,
            sem: []SpawnOptions.TimelineSemaphoreInfo,
            signal: ?SpawnOptions.SignalObject,
        ) void {
            allocator.free(f);
            allocator.free(sem);
            if (signal) |s| switch (s) {
                .fence => |x| x.signal(),
                .timeline_semaphore => |x| x.semaphore.signal(x.counter),
                else => unreachable,
            };
        }
    };

    try future.goWithCleanup(
        options.executor,
        Wrapper.start,
        .{ fences, semaphores, args },
        Wrapper.cleanup,
        .{ options.allocator, fences, semaphores, options.signal },
        .{
            .allocator = options.allocator,
            .label = options.label,
            .stack_size = options.stack_size,
            .worker = options.worker,
            .dependencies = options.dependencies,
        },
    );
}
