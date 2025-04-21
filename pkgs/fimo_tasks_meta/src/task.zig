const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;

const symbols = @import("symbols.zig");
const testing = @import("testing.zig");

/// Identifier of a task.
pub const Id = enum(usize) {
    _,

    /// Returns the id of the current task.
    pub fn current(provider: anytype) ?Id {
        const sym = symbols.task_id.requestFrom(provider);
        var id: Id = undefined;
        if (sym(&id) == false) return null;
        return id;
    }

    test "no task" {
        var ctx = try testing.initTestContext();
        defer ctx.deinit();
        try std.testing.expectEqual(null, Id.current(ctx));
    }

    test "in task" {
        try testing.initTestContextInTask(struct {
            fn f(ctx: *const testing.TestContext, err: *?AnyError) anyerror!void {
                _ = err;
                try std.testing.expect(Id.current(ctx) != null);
            }
        }.f);
    }
};

/// A unit of work.
pub fn Task(comptime T: type) type {
    return extern struct {
        /// Optional label of the task.
        ///
        /// May be used by the runtime for tracing purposes. If present, the string must live until
        /// the task instance is destroyed. For dynamically allocated labels this may be done in
        /// the `on_deinit` function.
        label_: ?[*]const u8 = null,
        /// Length of the label string.
        label_len: usize = 0,
        /// Entry function of the task.
        on_start: *const fn (task: *@This()) callconv(.c) void,
        /// Optional completion handler of the task.
        ///
        /// Will be invoked after successfull completion of the task on an arbitrary thread.
        on_complete: ?*const fn (task: *@This()) callconv(.c) void = null,
        /// Optional abortion handler of the task.
        ///
        /// Will be invoked on an arbitrary thread, if the task is aborted.
        on_abort: ?*const fn (task: *@This()) callconv(.c) void = null,
        /// Optional deinitialization routine.
        on_deinit: ?*const fn (task: *@This()) callconv(.c) void = null,
        /// Task state.
        state: T,

        /// Returns the label of the task.
        pub fn label(self: *const @This()) []const u8 {
            return if (self.label_) |l| l[0..self.label_len] else "<unlabelled>";
        }

        /// Runs the completion and deinit routines of the task.
        pub fn complete(self: *@This()) void {
            if (self.on_complete) |f| f(self);
            if (self.on_deinit) |f| f(self);
        }

        /// Runs the abort and deinit routines of the task.
        pub fn abort(self: *@This()) void {
            if (self.on_abort) |f| f(self);
            if (self.on_deinit) |f| f(self);
        }
    };
}

/// Yields the current task or thread back to the scheduler.
pub fn yield(provider: anytype) void {
    const sym = symbols.yield.requestFrom(provider);
    sym();
}

/// Aborts the current task.
pub fn abort(provider: anytype) noreturn {
    const sym = symbols.abort.requestFrom(provider);
    sym();
    unreachable;
}

test "abort" {
    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) anyerror!void {
            const builder_config = BuilderConfig(*const testing.TestContext){
                .on_start = struct {
                    fn f(t: *Task(*const testing.TestContext)) void {
                        abort(t.state);
                    }
                }.f,
            };
            const builder = Builder(builder_config){
                .label = "abortTask",
                .state = ctx,
            };
            var task = builder.build();

            const Pool = @import("pool.zig").Pool;
            const pool = Pool.current(ctx).?;
            defer pool.unref();

            var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
            defer arena.deinit();
            const allocator = arena.allocator();

            const buffer_config = @import("command_buffer.zig").BuilderConfig(void){};
            var buffer_builder = @import("command_buffer.zig").Builder(buffer_config){ .state = {} };
            try buffer_builder.abortOnError(allocator);
            try buffer_builder.enqueueTask(allocator, @ptrCast(&task));
            var buffer = buffer_builder.build();

            const handle = try pool.enqueueCommandBuffer(&buffer, err);
            defer handle.unref();
            const status = handle.waitOn();
            try std.testing.expect(status == .aborted);
        }
    }.f);
}

/// Puts the current task or thread to sleep for the specified amount of time.
pub fn sleep(provider: anytype, duration: Duration) void {
    const sym = symbols.sleep.requestFrom(provider);
    sym(duration.intoC());
}

test "sleep" {
    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) anyerror!void {
            _ = err;
            const before_sleep = Instant.now();
            const duration = Duration.initSeconds(2);
            sleep(ctx, duration);
            const elapsed = try Instant.elapsed(before_sleep);
            try std.testing.expect(elapsed.order(duration) != .lt);
        }
    }.f);
}

/// A task with an unknown state.
pub const OpaqueTask = Task(void);

/// Configuration of a task builder.
pub fn BuilderConfig(T: type) type {
    return struct {
        on_start: fn (task: *Task(T)) void,
        on_complete: ?fn (task: *Task(T)) void = null,
        on_abort: ?fn (task: *Task(T)) void = null,
        on_deinit: ?fn (task: *Task(T)) void = null,

        pub const State = T;
    };
}

/// Helper type for building tasks.
pub fn Builder(config: anytype) type {
    const State = @TypeOf(config).State;
    const config_: BuilderConfig(State) = config;

    return struct {
        label: ?[]const u8 = null,
        state: State,

        /// Builds a task from the builder.
        pub fn build(self: @This()) Task(State) {
            const Wrapper = struct {
                fn onStart(task: *Task(State)) callconv(.c) void {
                    config_.on_start(task);
                }
                fn onComplete(task: *Task(State)) callconv(.c) void {
                    if (comptime config_.on_complete) |f| f(task);
                }
                fn onAbort(task: *Task(State)) callconv(.c) void {
                    if (comptime config_.on_abort) |f| f(task);
                }
                fn onDeinit(task: *Task(State)) callconv(.c) void {
                    if (comptime config_.on_deinit) |f| f(task);
                }
            };

            const label, const label_len = if (self.label) |l| .{ l.ptr, l.len } else .{ null, 0 };
            return Task(State){
                .label_ = label,
                .label_len = label_len,
                .on_start = &Wrapper.onStart,
                .on_complete = if (config_.on_complete != null) &Wrapper.onComplete else null,
                .on_abort = if (config_.on_abort != null) &Wrapper.onAbort else null,
                .on_deinit = if (config_.on_complete != null) &Wrapper.onDeinit else null,
                .state = self.state,
            };
        }
    };
}
