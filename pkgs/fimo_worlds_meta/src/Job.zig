//! A concurrent task backed by an executor.

const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const future = fimo_tasks_meta.future;
const Handle = fimo_tasks_meta.command_buffer.Handle;
const StackSize = fimo_tasks_meta.pool.StackSize;
const Worker = fimo_tasks_meta.pool.Worker;
const Pool = fimo_tasks_meta.pool.Pool;
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
    /// Optional fence to signal upon completion of the job.
    signal: ?*Fence = null,
};

/// Spawns a new job.
pub fn go(
    provider: anytype,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    options: SpawnOptions,
) error{SpawnFailed}!void {
    const fences = options.allocator.dupe(*Fence, options.fences) catch return error.SpawnFailed;
    const Wrapper = struct {
        fn start(
            prov: @TypeOf(provider),
            wait: []*Fence,
            args_: std.meta.ArgsTuple(@TypeOf(function)),
        ) void {
            for (wait) |f| f.wait(prov);
            @call(.auto, function, args_);
        }

        fn cleanup(prov: @TypeOf(provider), allocator: Allocator, f: []*Fence, signal: ?*Fence) void {
            allocator.free(f);
            if (signal) |s| s.signal(prov);
        }
    };

    var err: ?AnyResult = null;
    errdefer if (err) |e| e.deinit();

    future.goWithCleanup(
        options.executor,
        Wrapper.start,
        .{ provider, fences, args },
        Wrapper.cleanup,
        .{ provider, options.allocator, fences, options.signal },
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
