const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayListUnmanaged = std.ArrayListUnmanaged;

const pool = @import("pool.zig");
const StackSize = pool.StackSize;
const Worker = pool.Worker;
const Pool = pool.Pool;
const task = @import("task.zig");
const OpaqueTask = task.OpaqueTask;

/// An entry of a command buffer.
pub const Entry = extern struct {
    tag: enum(i32) {
        abort_on_error,
        set_min_stack_size,
        select_worker,
        select_any_worker,
        enqueue_task,
        enqueue_command_buffer,
        wait_on_barrier,
        wait_on_command_buffer,
        wait_on_command_indirect,
        _,
    },
    payload: extern union {
        abort_on_error: bool,
        set_min_stack_size: StackSize,
        select_worker: Worker,
        select_any_worker: void,
        enqueue_task: *OpaqueTask,
        enqueue_command_buffer: *OpaqueCommandBuffer,
        wait_on_barrier: void,
        wait_on_command_buffer: Handle,
        wait_on_command_indirect: usize,
    },

    /// Deinitializes the entry.
    pub fn deinit(self: Entry) void {
        switch (self.tag) {
            .abort_on_error,
            .set_min_stack_size,
            .select_worker,
            .select_any_worker,
            .wait_on_barrier,
            .wait_on_command_indirect,
            => {},
            .enqueue_task => self.payload.enqueue_task.deinit(),
            .enqueue_command_buffer => self.payload.enqueue_command_buffer.deinit(),
            .wait_on_command_buffer => self.payload.wait_on_command_buffer.unref(),
            else => @panic("unknown entry type"),
        }
    }

    /// Runs the abort routines of the entry.
    pub fn abort(self: Entry) void {
        switch (self.tag) {
            .abort_on_error,
            .set_min_stack_size,
            .select_worker,
            .select_any_worker,
            .wait_on_barrier,
            .wait_on_command_indirect,
            .wait_on_command_buffer,
            => {},
            .enqueue_task => self.payload.enqueue_task.abort(),
            .enqueue_command_buffer => self.payload.enqueue_command_buffer.abort(),
            else => @panic("unknown entry type"),
        }
    }

    comptime {
        const Payload = @FieldType(Entry, "payload");
        std.debug.assert(@sizeOf(Payload) == 2 * @sizeOf(usize));
        std.debug.assert(@alignOf(Payload) <= @alignOf(usize));
    }
};

/// A handle to an enqueued command buffer.
pub const Handle = extern struct {
    data: *anyopaque,
    vtable: *const VTable,

    pub const CompletionStatus = enum(i32) {
        completed,
        aborted,
    };

    /// VTable of a handle.
    pub const VTable = extern struct {
        ref: *const fn (handle: *anyopaque) callconv(.c) void,
        unref: *const fn (handle: *anyopaque) callconv(.c) void,
        owner_pool: *const fn (handle: *anyopaque) callconv(.c) Pool,
        wait_on: *const fn (handle: *anyopaque) callconv(.c) CompletionStatus,
    };

    /// Increases the reference count of the handle.
    pub fn ref(self: Handle) Handle {
        self.vtable.ref(self.data);
        return self;
    }

    /// Releases the reference to the handle.
    pub fn unref(self: Handle) void {
        self.vtable.unref(self.data);
    }

    /// Returns a reference to the worker pool owning the handle.
    pub fn ownerPool(self: Handle) Pool {
        return self.vtable.owner_pool(self.data);
    }

    /// Waits for the completion of the command buffer.
    pub fn waitOn(self: Handle) CompletionStatus {
        return self.vtable.wait_on(self.data);
    }
};

/// A list of commands to process by a worker pool.
pub fn CommandBuffer(T: type) type {
    return extern struct {
        /// Optional label of the command buffer.
        ///
        /// May be used by the runtime for tracing purposes. If present, the string must live until
        /// the task instance is destroyed. For dynamically allocated labels this may be done in
        /// the `on_deinit` function.
        label_: ?[*]const u8 = null,
        /// Length of the label string.
        label_len: usize = 0,
        /// List of commands.
        entries_: ?[*]const Entry = null,
        /// Length of the command list.
        entries_len: usize = 0,
        /// Optional completion handler of the command buffer.
        ///
        /// Will be invoked after successfull completion of the command bufer on an arbitrary
        /// thread.
        on_complete: ?*const fn (buffer: *CommandBuffer(T)) callconv(.c) void = null,
        /// Optional abortion handler of the command buffer..
        ///
        /// Will be invoked on an arbitrary thread, if the command buffer is aborted.
        on_abort: ?*const fn (buffer: *CommandBuffer(T)) callconv(.c) void = null,
        /// Optional deinitialization routine.
        ///
        /// Will be invoked after all references to the task cease to exist.
        on_deinit: ?*const fn (buffer: *CommandBuffer(T)) callconv(.c) void = null,
        /// Command buffer state.
        state: T,

        /// Returns the label of the command buffer.
        pub fn label(self: *const @This()) []const u8 {
            return if (self.label_) |l| l[0..self.label_len] else "<unlabelled>";
        }

        /// Returns the entries of the command buffer.
        pub fn entries(self: *const @This()) []const Entry {
            return if (self.entries_) |l| l[0..self.entries_len] else &.{};
        }

        /// Runs the deinit routine of the command buffer.
        pub fn deinit(self: *@This()) void {
            for (self.entries()) |entry| entry.deinit();
            if (self.on_deinit) |f| f(self);
        }

        /// Runs the completion routine of the command buffer.
        pub fn complete(self: *@This()) void {
            if (self.on_complete) |f| f(self);
        }

        /// Runs the abort routine of the command buffer.
        pub fn abort(self: *@This()) void {
            if (self.on_abort) |f| f(self);
        }
    };
}

/// A command buffer with an unknown state.
pub const OpaqueCommandBuffer = CommandBuffer(void);

/// Configuration of a command buffer builder.
pub fn BuilderConfig(T: type) type {
    return struct {
        on_complete: ?fn (buffer: *CommandBuffer(T)) void = null,
        on_abort: ?fn (buffer: *CommandBuffer(T)) void = null,
        on_deinit: ?fn (buffer: *CommandBuffer(T)) void = null,

        pub const State = T;
    };
}

/// A builder for a `CommandBuffer`.
pub fn Builder(config: anytype) type {
    const State = @TypeOf(config).State;
    const config_: BuilderConfig(State) = config;

    return struct {
        label: ?[]const u8 = null,
        entries: ArrayListUnmanaged(Entry) = .{},
        state: State,

        /// Aborts all commands of the command buffer.
        pub fn abortAndFreeCommands(self: *@This(), allocator: Allocator) void {
            for (self.entries.items) |entry| {
                entry.abort();
                entry.deinit();
            }
            self.entries.clearAndFree(allocator);
        }

        /// Configures the command buffer to abort the following commands if any of them errors.
        pub fn abortOnError(self: *@This(), allocator: Allocator) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .abort_on_error,
                .payload = .{ .abort_on_error = true },
            });
        }

        /// Configures the command buffer to ignore the errors of the following commands.
        pub fn continueOnError(self: *@This(), allocator: Allocator) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .abort_on_error,
                .payload = .{ .abort_on_error = false },
            });
        }

        /// Specifies the minimum stack size for the following tasks.
        pub fn setMinStackSize(
            self: *@This(),
            allocator: Allocator,
            size: StackSize,
        ) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .set_min_stack_size,
                .payload = .{ .set_min_stack_size = size },
            });
        }

        /// Specifies that the following tasks may only be enqueued on the provided worker.
        pub fn selectWorker(
            self: *@This(),
            allocator: Allocator,
            worker: Worker,
        ) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .select_worker,
                .payload = .{ .select_worker = worker },
            });
        }

        /// Specifies that the following tasks may be enqueued on any worker of the pool.
        pub fn selectAnyWorker(self: *@This(), allocator: Allocator) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .select_any_worker,
                .payload = .select_any_worker,
            });
        }

        /// Enqueues a task.
        ///
        /// The command will complete when the task is completed.
        pub fn enqueueTask(
            self: *@This(),
            allocator: Allocator,
            t: *OpaqueTask,
        ) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .enqueue_task,
                .payload = .{ .enqueue_task = t },
            });
        }

        /// Enqueues a sub command buffer.
        ///
        /// The command will complete when the sub command buffer is completed.
        pub fn enqueueCommandBuffer(
            self: *@This(),
            allocator: Allocator,
            buffer: *OpaqueCommandBuffer,
        ) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .enqueue_command_buffer,
                .payload = .{ .enqueue_command_buffer = buffer },
            });
        }

        /// Waits for the completion of all preceding commands.
        pub fn waitOnBarrier(self: *@This(), allocator: Allocator) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .wait_on_barrier,
                .payload = .wait_on_barrier,
            });
        }

        /// Waits for the completion of the command buffer handle.
        pub fn waitOnCommandBuffer(
            self: *@This(),
            allocator: Allocator,
            handle: Handle,
        ) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .wait_on_command_buffer,
                .payload = .{ .wait_on_command_buffer = handle },
            });
        }

        /// Waits for the completion of some specific command contained in the buffer.
        ///
        /// Waits for the completion of the command at index `this_command - offset`.
        pub fn waitOnCommandIndirect(
            self: *@This(),
            allocator: Allocator,
            offset: usize,
        ) Allocator.Error!void {
            try self.entries.append(allocator, .{
                .tag = .wait_on_command_indirect,
                .payload = .{ .wait_on_command_indirect = offset },
            });
        }

        /// Builds a command buffer from the builder.
        pub fn build(self: @This()) CommandBuffer(State) {
            const Wrapper = struct {
                fn onComplete(buffer: *CommandBuffer(State)) callconv(.c) void {
                    if (comptime config_.on_complete) |f| f(buffer);
                }
                fn onAbort(buffer: *CommandBuffer(State)) callconv(.c) void {
                    if (comptime config_.on_abort) |f| f(buffer);
                }
                fn onDeinit(buffer: *CommandBuffer(State)) callconv(.c) void {
                    if (comptime config_.on_deinit) |f| f(buffer);
                }
            };

            const label, const label_len = if (self.label) |l| .{ l.ptr, l.len } else .{ null, 0 };
            return CommandBuffer(State){
                .label_ = label,
                .label_len = label_len,
                .entries_ = self.entries.items.ptr,
                .entries_len = self.entries.items.len,
                .on_complete = if (config_.on_complete != null) &Wrapper.onComplete else null,
                .on_abort = if (config_.on_abort != null) &Wrapper.onAbort else null,
                .on_deinit = if (config_.on_complete != null) &Wrapper.onDeinit else null,
                .state = self.state,
            };
        }
    };
}
