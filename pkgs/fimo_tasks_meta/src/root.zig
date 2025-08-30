const std = @import("std");
const Allocator = std.mem.Allocator;

pub const c = @import("c");
const fimo_std = @import("fimo_std");
const Error = fimo_std.ctx.Error;
const tracing = fimo_std.tracing;
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;

pub const symbols = @import("symbols.zig");
pub const sync = @import("sync.zig");
const testing = @import("testing.zig");

/// A string label.
pub const Label = extern struct {
    ptr: ?[*]const u8 = null,
    len: usize = 0,

    pub fn init(label: ?[]const u8) Label {
        const l = label orelse return .{};
        return .{ .ptr = l.ptr, .len = l.len };
    }

    pub fn get(self: Label) ?[]const u8 {
        const ptr = self.ptr orelse return null;
        return ptr[0..self.len];
    }
};

/// Identifier of a task.
pub const TaskId = enum(usize) {
    _,

    /// Returns the id of the current task.
    pub fn current() ?TaskId {
        const sym = symbols.task_id.getGlobal().get();
        var id: TaskId = undefined;
        if (sym(&id) == false) return null;
        return id;
    }

    test "no task" {
        var ctx = try testing.initTestContext();
        defer ctx.deinit();
        try std.testing.expectEqual(null, TaskId.current());
    }

    test "in task" {
        try testing.initTestContextInTask(struct {
            fn f() anyerror!void {
                try std.testing.expect(TaskId.current() != null);
            }
        }.f);
    }
};

/// Identifier of a worker thread in an `Executor`.
pub const Worker = enum(usize) {
    _,

    /// Returns the id of the current worker.
    pub fn current() ?Worker {
        const sym = symbols.worker_id.getGlobal().get();
        var id: Worker = undefined;
        if (sym(&id) == false) return null;
        return id;
    }

    test "no task" {
        var ctx = try testing.initTestContext();
        defer ctx.deinit();
        try std.testing.expectEqual(null, Worker.current());
    }

    test "in task" {
        try testing.initTestContextInTask(struct {
            fn f() anyerror!void {
                try std.testing.expect(Worker.current() != null);
            }
        }.f);
    }
};

/// A unit of work.
pub const Task = extern struct {
    /// Optional label of the task.
    ///
    /// May be used by the runtime for tracing purposes.
    /// If present, the string must live until the task instance is destroyed.
    label: Label = .{},
    /// Number of sub-tasks to start.
    batch_len: usize = 1,
    /// Entry function of the task.
    run: *const fn (task: *Task, idx: usize) callconv(.c) void,
};

/// Yields the current task or thread back to the scheduler.
pub fn yield() void {
    const sym = symbols.yield.getGlobal().get();
    sym();
}

test "yield" {
    try testing.initTestContextInTask(struct {
        fn f() anyerror!void {
            for (0..100000) |_| yield();
        }
    }.f);
}

/// Aborts the current task.
pub fn abort() noreturn {
    const sym = symbols.abort.getGlobal().get();
    sym();
    unreachable;
}

test "abort" {
    try testing.initTestContextInTask(struct {
        fn f() anyerror!void {
            const ex = Executor.current().?;
            const Runner = struct {
                fn run(done: *bool) void {
                    abort();
                    done.* = true;
                }
            };
            var done: bool = false;
            var fut = Future(Runner.run){};
            fut.spawn(ex, .{&done});
            fut.join();
            try std.testing.expect(done == false);
        }
    }.f);
}

/// Reports whether a cancellation of the current task has been requested.
pub fn cancelRequested() bool {
    const sym = symbols.cancel_requested.getGlobal().get();
    return sym();
}

/// Puts the current task or thread to sleep for the specified amount of time.
pub fn sleep(duration: Duration) void {
    const sym = symbols.sleep.getGlobal().get();
    sym(duration.intoC());
}

test "sleep" {
    try testing.initTestContextInTask(struct {
        fn f() anyerror!void {
            const before_sleep = Instant.now();
            const duration = Duration.initSeconds(2);
            sleep(duration);
            const elapsed = try Instant.elapsed(before_sleep);
            try std.testing.expect(elapsed.order(duration) != .lt);
        }
    }.f);
}

test "short sleep" {
    try testing.initTestContextInTask(struct {
        fn f() anyerror!void {
            const duration = Duration.initMillis(1);
            for (0..10) |_| {
                const before_sleep = Instant.now();
                sleep(duration);
                const elapsed = try Instant.elapsed(before_sleep);
                try std.testing.expect(elapsed.order(duration) != .lt);
                tracing.logInfo(@src(), "slept for {}ms", .{elapsed.millis()});
            }
        }
    }.f);
}

/// A key for a task-specific-storage.
///
/// A new key can be defined by casting from a stable address.
pub fn TssKey(comptime T: type) type {
    return opaque {
        /// Associates a value with the key for the current task.
        ///
        /// The current value associated with the key is replaced with the new value without
        /// invoking any destructor function. The destructor function is set to `dtor`, and will
        /// be invoked upon task exit. May only be called by a task.
        pub fn set(
            self: *const @This(),
            value: ?*T,
            comptime dtor: ?fn (value: ?*T) void,
        ) void {
            const Wrapper = struct {
                fn dtorWrapper(v: ?*anyopaque) callconv(.c) void {
                    if (comptime dtor) |f| f(@ptrCast(@alignCast(v)));
                }
            };
            const sym = symbols.task_local_set.getGlobal().get();
            sym(
                @ptrCast(self),
                @ptrCast(value),
                if (comptime dtor != null) &Wrapper.dtorWrapper else null,
            );
        }

        /// Returns the value associated to the key for the current task.
        ///
        /// May only be called by a task.
        pub fn get(self: *const @This()) ?*T {
            const sym = symbols.task_local_get.getGlobal().get();
            return @ptrCast(@alignCast(sym(@ptrCast(self))));
        }

        /// Clears the value of the current task associated with the key.
        ///
        /// This operation invokes the associated destructor function and sets the value to `null`.
        /// May only be called by a task.
        pub fn clear(self: *const @This()) void {
            const sym = symbols.task_local_clear.getGlobal().get();
            sym(@ptrCast(self));
        }
    };
}

/// A key with an unknown value type.
pub const AnyTssKey = TssKey(anyopaque);

/// An entry of a command buffer.
pub const CmdBufCmd = extern struct {
    tag: enum(i32) {
        select_worker,
        select_any_worker,
        enqueue_task,
        wait_on_barrier,
        wait_on_cmd_indirect,
        _,
    },
    payload: extern union {
        select_worker: Worker,
        select_any_worker: void,
        enqueue_task: *Task,
        wait_on_barrier: void,
        wait_on_cmd_indirect: usize,
    },
};

/// A slice of command buffer commands.
pub const CmdBufCmdList = extern struct {
    ptr: ?[*]const CmdBufCmd = null,
    len: usize = 0,

    pub fn init(entries: []const CmdBufCmd) CmdBufCmdList {
        return .{ .ptr = entries.ptr, .len = entries.len };
    }

    pub fn get(self: CmdBufCmdList) ?[]const CmdBufCmd {
        const ptr = self.ptr orelse return null;
        return ptr[0..self.len];
    }
};

/// A list of commands to process by an executor.
pub const CmdBuf = extern struct {
    /// Optional label of the command buffer.
    ///
    /// May be used by the runtime for tracing purposes.
    /// If present, the string must live until the buffer is destroyed.
    label: Label = .{},
    /// List of commands.
    cmds: CmdBufCmdList = .{},
    /// Optional cleanup function of the buffer.
    deinit: ?*const fn (cmd_buf: *CmdBuf) callconv(.c) void = null,
};

/// A handle to an enqueued command buffer.
pub const CmdBufHandle = opaque {
    pub const CompletionStatus = enum(i32) {
        completed,
        cancelled,
    };

    /// Waits for the command buffer to complete.
    ///
    /// Once called, the handle is consumed.
    pub fn join(self: *CmdBufHandle) CompletionStatus {
        const sym = symbols.cmd_buf_join.getGlobal().get();
        return sym(self);
    }

    /// Release the obligation of the caller to call join and
    /// have the handle be cleaned up on completion.
    ///
    /// Once called, the handle is consumed.
    pub fn detach(self: *CmdBufHandle) void {
        const sym = symbols.cmd_buf_detach.getGlobal().get();
        sym(self);
    }

    /// Like `join`, but flags the handle as cancelled.
    pub fn cancel(self: *CmdBufHandle) void {
        const sym = symbols.cmd_buf_cancel.getGlobal().get();
        sym(self);
    }

    /// Like `detach`, but flags the handle as cancelled.
    pub fn cancelDetach(self: *CmdBufHandle) void {
        const sym = symbols.cmd_buf_cancel_detach.getGlobal().get();
        sym(self);
    }
};

pub const ExecutorCfg = extern struct {
    /// Optional label of the executor.
    label: Label = .{},
    /// Maximum number of enqueued cmd buffers.
    ///
    /// A value of `0` indicates to use the default capacity.
    cmd_buf_capacity: usize = 0,
    /// Number of worker threads owned by the executor.
    ///
    /// A value of `0` indicates to use the default number of workers.
    worker_count: usize = 0,
    /// Controls the maximum number of spawned tasks.
    ///
    /// The maximum number of spawned tasks is determined as `worker_count * max_load_factor`.
    /// A value of `0` indicates to use the default load factor.
    max_load_factor: usize = 0,
    /// Minimum stack size in bytes.
    ///
    /// A value of `0` indicates to use the default stack size.
    stack_size: usize = 0,
    /// Number of cached stacks per worker.
    ///
    /// The cache is shared among all workers.
    /// A value of `0` indicates to use the default cache length.
    worker_stack_cache_len: usize = 0,
    /// Indicates whether to disable the stack cache.
    disable_stack_cache: bool = false,
};

/// A handle to an executor.
pub const Executor = opaque {
    pub fn globalExecutor() *Executor {
        const sym = symbols.executor_global.getGlobal().get();
        return @constCast(sym);
    }

    /// Creates a new executor with the provided configuration.
    pub fn init(cfg: ExecutorCfg) Error!*Executor {
        var ex: *Executor = undefined;
        const sym = symbols.executor_new.getGlobal().get();
        try sym(&ex, &cfg).intoErrorUnion();
        return ex;
    }

    /// Returns the executor for the current context.
    ///
    /// Is only valid for the duration of the current context (i.e. Task).
    pub fn current() ?*Executor {
        const sym = symbols.executor_current.getGlobal().get();
        return sym();
    }

    test "current, no task" {
        var ctx = try testing.initTestContext();
        defer ctx.deinit();
        try std.testing.expectEqual(null, Executor.current());
    }

    test "current, in task" {
        try testing.initTestContextInTask(struct {
            fn f() anyerror!void {
                const exe = Executor.current();
                try std.testing.expect(exe != null);
            }
        }.f);
    }

    /// Waits until all remaining commands have been executed and consumes the handle.
    ///
    /// New commands can be enqueued to the executor while the call is in process.
    pub fn join(self: *Executor) void {
        const sym = symbols.executor_join.getGlobal().get();
        sym(self);
    }

    /// Reports whether the owner of the executor has requested that the executor be joined.
    pub fn joinRequested(self: *Executor) bool {
        const sym = symbols.executor_join_requested.getGlobal().get();
        return sym(self);
    }

    /// Enqueues the commands to the executor.
    ///
    /// The caller will block until the handle could be enqueued.
    /// The buffer must outlive the returned handle.
    pub fn enqueue(self: *Executor, cmds: *CmdBuf) *CmdBufHandle {
        const sym = symbols.executor_enqueue.getGlobal().get();
        return sym(self, cmds);
    }

    /// Enqueues the commands to the executor.
    ///
    /// The caller will block until the handle could be enqueued.
    /// The buffer must outlive the returned handle.
    pub fn enqueueDetached(self: *Executor, cmds: *CmdBuf) void {
        const sym = symbols.executor_enqueue_detached.getGlobal().get();
        sym(self, cmds);
    }
};

pub fn Future(function: anytype) type {
    const F = @TypeOf(function);
    const Args = std.meta.ArgsTuple(F);
    const Result = @typeInfo(F).@"fn".return_type.?;

    return struct {
        args: Args = undefined,
        result: Result = undefined,
        task: Task = .{ .run = run },
        cmd: CmdBufCmd = undefined,
        cmd_buf: CmdBuf = undefined,
        handle: *CmdBufHandle = undefined,

        const Self = @This();
        const AllocFuture = struct {
            inner: Self,
            allocator: Allocator,
            dealloc: std.atomic.Value(bool) = .init(false),

            pub fn join(self: *AllocFuture) Result {
                const result = self.inner.join();
                const dealloc = self.dealloc.swap(true, .acquire);
                const allocator = self.allocator;
                if (dealloc) allocator.destroy(self);
                return result;
            }
        };

        fn run(task: *Task, idx: usize) callconv(.c) void {
            _ = idx;
            const self: *Self = @alignCast(@fieldParentPtr("task", task));
            self.result = @call(.auto, function, self.args);
        }

        pub fn join(self: *Self) Result {
            _ = self.handle.join();
            return self.result;
        }

        pub fn spawn(self: *Self, executor: *Executor, args: Args) void {
            self.args = args;
            self.cmd = .{ .tag = .enqueue_task, .payload = .{ .enqueue_task = &self.task } };
            self.cmd_buf.cmds = .init(@ptrCast(&self.cmd));
            self.handle = executor.enqueue(&self.cmd_buf);
        }

        pub fn spawnAlloc(
            gpa: Allocator,
            executor: *Executor,
            args: Args,
        ) error{OutOfMemory}!*AllocFuture {
            const fut = try gpa.create(AllocFuture);
            fut.* = .{ .inner = .{}, .allocator = gpa };
            fut.inner.cmd_buf.deinit = struct {
                fn deinit(cmd_buf: *CmdBuf) callconv(.c) void {
                    const inner: *Self = @alignCast(@fieldParentPtr("cmd_buf", cmd_buf));
                    const outer: *AllocFuture = @alignCast(@fieldParentPtr("inner", inner));
                    const dealloc = outer.dealloc.swap(true, .acquire);
                    const allocator = outer.allocator;
                    if (dealloc) allocator.destroy(outer);
                }
            }.deinit;
            fut.inner.spawn(executor, args);
            return fut;
        }
    };
}

test {
    std.testing.refAllDeclsRecursive(symbols);
    std.testing.refAllDeclsRecursive(sync);
}
