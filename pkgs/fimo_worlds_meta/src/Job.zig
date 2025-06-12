//! A concurrent task backed by an executor.

const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
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
    pub fn wait(self: *Fence, provider: anytype) void {
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

            Futex(u8).wait(provider, &self.state, contended);
        }
    }

    /// Wakes all waiters of the fence.
    pub fn signal(self: *Fence, provider: anytype) void {
        const state = self.state.swap(signaled, .release);
        if (state & contended != 0) Futex(u8).wake(provider, &self.state, std.math.maxInt(usize));
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

    pub const has_waiters: usize = 1 << 63;
    const value_mask: usize = ~has_waiters;

    /// Initializes the semaphore with a custom initial value.
    pub fn init(value: u63) TimelineSemaphore {
        return .{ .state = .init(value) };
    }

    /// Returns the current counter of the semaphore.
    pub fn counter(self: *const TimelineSemaphore) u63 {
        const curr_value = self.state.load(.acquire) & value_mask;
        return @truncate(curr_value);
    }

    /// Checks if the semaphore is signaled with a count greater or equal to `value`.
    pub fn isSignaled(self: *const TimelineSemaphore, value: u63) bool {
        const curr_value = self.state.load(.acquire) & value_mask;
        return curr_value >= value;
    }

    /// Blocks the caller until the semaphore reaches a count greater or equal to `value`.
    pub fn wait(self: *TimelineSemaphore, provider: anytype, value: u63) void {
        if (!self.isSignaled(value)) _ = self.waitSlow(provider, value, null);
    }

    /// Blocks the caller until the semaphore reaches a count greater or equal to `value`, or the timeout expires.
    pub fn timedWait(
        self: *TimelineSemaphore,
        provider: anytype,
        value: u63,
        timeout: Duration,
    ) error{Timeout}!void {
        if (!self.isSignaled(value)) {
            const timeout_time = Instant.now().add(timeout) catch null;
            if (!self.waitSlow(provider, value, timeout_time)) return error.Timeout;
        }
    }

    fn waitSlow(self: *TimelineSemaphore, provider: anytype, value: u63, timeout: ?Instant) bool {
        @branchHint(.cold);
        const required: u64 = value;
        var state = self.state.load(.monotonic);
        while (true) {
            if (state & value_mask >= value) {
                _ = self.state.load(.acquire);
                return true;
            }

            if (state & has_waiters == 0) {
                if (self.state.cmpxchgWeak(
                    state,
                    state | has_waiters,
                    .monotonic,
                    .monotonic,
                )) |s| {
                    state = s;
                    continue;
                }
            }

            const Validation = struct {
                ptr: *const atomic.Value(u64),
                value: u64,
                fn f(this: *@This()) bool {
                    return this.ptr.load(.monotonic) <= this.value | has_waiters;
                }
            };
            const BeforeSleep = struct {
                fn f(this: *@This()) void {
                    _ = this;
                }
            };
            const TimedOut = struct {
                ptr: *atomic.Value(u64),
                fn f(this: *@This(), key: *const anyopaque, is_last: bool) void {
                    _ = key;
                    if (is_last) {
                        _ = this.ptr.fetchAnd(~has_waiters, .monotonic);
                    }
                }
            };
            const result = ParkingLot.park(
                provider,
                self,
                Validation{ .ptr = &self.state, .value = value },
                Validation.f,
                BeforeSleep{},
                BeforeSleep.f,
                TimedOut{ .ptr = &self.state },
                TimedOut.f,
                @enumFromInt(@intFromPtr(&required)),
                timeout,
            );
            switch (result.type) {
                // If we were unparked, the count must have been correct.
                .unparked => return true,
                // The state changed, retry.
                .invalid => {},
                .timed_out => return false,
            }

            state = self.state.load(.monotonic);
        }
    }

    /// Sets the internal value of the semaphore, possibly waking waiting tasks.
    ///
    /// `value` must be greater than the current value of the semaphore.
    pub fn signal(self: *TimelineSemaphore, provider: anytype, value: u63) void {
        const state = self.state.load(.monotonic);
        std.debug.assert(state & value_mask < value);
        if (state & has_waiters != 0 or
            self.state.cmpxchgWeak(state, value, .release, .monotonic) != null)
            self.signalSlow(provider, value);
    }

    fn signalSlow(self: *TimelineSemaphore, provider: anytype, value: u63) void {
        @branchHint(.cold);

        var state = self.state.load(.monotonic);
        while (true) {
            // If there are no waiters we can set the value directly.
            std.debug.assert(state & value_mask < value);
            if (state & has_waiters == 0) {
                state = self.state.cmpxchgWeak(
                    state,
                    value,
                    .release,
                    .monotonic,
                ) orelse return;
                continue;
            }

            const Filter = struct {
                count: u64,
                fn f(this: *@This(), token: ParkingLot.ParkToken) ParkingLot.FilterOp {
                    const required: *const u64 = @ptrFromInt(@intFromEnum(token));
                    if (required.* <= this.count) return .unpark;
                    return .skip;
                }
            };
            const Callback = struct {
                ptr: *atomic.Value(u64),
                value: u64,
                fn f(this: *@This(), result: ParkingLot.UnparkResult) ParkingLot.UnparkToken {
                    if (result.has_more_tasks) {
                        this.ptr.store(this.value | has_waiters, .release);
                    } else {
                        this.ptr.store(this.value, .release);
                    }
                    return .default;
                }
            };
            _ = ParkingLot.unparkFilter(
                provider,
                &self.state,
                Filter{ .count = value },
                Filter.f,
                Callback{ .ptr = &self.state, .value = value },
                Callback.f,
            );
        }
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
        counter: u63,
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
    provider: anytype,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    options: SpawnOptions,
) error{SpawnFailed}!void {
    const fences = options.allocator.dupe(
        *Fence,
        options.fences,
    ) catch return error.SpawnFailed;
    const semaphores = options.allocator.dupe(
        SpawnOptions.TimelineSemaphoreInfo,
        options.semaphores,
    ) catch return error.SpawnFailed;
    const Wrapper = struct {
        fn start(
            prov: @TypeOf(provider),
            wait: []*Fence,
            wait_sem: []SpawnOptions.TimelineSemaphoreInfo,
            args_: std.meta.ArgsTuple(@TypeOf(function)),
        ) void {
            for (wait) |f| f.wait(prov);
            for (wait_sem) |i| i.semaphore.wait(prov, i.counter);
            @call(.auto, function, args_);
        }

        fn cleanup(
            prov: @TypeOf(provider),
            allocator: Allocator,
            f: []*Fence,
            sem: []SpawnOptions.TimelineSemaphoreInfo,
            signal: ?SpawnOptions.SignalObject,
        ) void {
            allocator.free(f);
            allocator.free(sem);
            if (signal) |s| switch (s) {
                .fence => |x| x.signal(prov),
                .timeline_semaphore => |x| x.semaphore.signal(prov, x.counter),
                else => unreachable,
            };
        }
    };

    var err: ?AnyError = null;
    errdefer if (err) |e| e.deinit();

    future.goWithCleanup(
        options.executor,
        Wrapper.start,
        .{ provider, fences, semaphores, args },
        Wrapper.cleanup,
        .{ provider, options.allocator, fences, semaphores, options.signal },
        .{
            .allocator = options.allocator,
            .label = options.label,
            .stack_size = options.stack_size,
            .worker = options.worker,
            .dependencies = options.dependencies,
        },
        &err,
    ) catch return error.SpawnFailed;
}
