const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayListUnmanaged = std.ArrayListUnmanaged;

const fimo_std = @import("fimo_std");
const Waker = fimo_std.tasks.Waker;
const Poll = fimo_std.tasks.Poll;
const BlockingContext = fimo_std.tasks.BlockingContext;
const EnqueuedFuture = fimo_std.tasks.EnqueuedFuture;

const command_buffer = @import("command_buffer.zig");
const Entry = command_buffer.Entry;
const Handle = command_buffer.Handle;
const CommandBuffer = command_buffer.CommandBuffer;
const OpaqueCommandBuffer = command_buffer.OpaqueCommandBuffer;
const pool = @import("pool.zig");
const StackSize = pool.StackSize;
const Worker = pool.Worker;
const Pool = pool.Pool;
const task = @import("task.zig");
const Task = task.Task;
const testing = @import("testing.zig");

pub const SpawnError = Allocator.Error || fimo_std.ctx.Error;

/// Options for spawning new futures.
pub const SpawnFutureOptions = struct {
    /// Allocator used to spawn to future.
    ///
    /// Must outlive the future handle.
    allocator: Allocator,
    /// Label of the underlying future command buffer.
    label: ?[]const u8 = null,
    /// Minimum stack size of the future.
    stack_size: ?StackSize = null,
    /// Worker to assign the future to.
    worker: ?Worker = null,
    /// List of dependencies to wait for before starting the future.
    ///
    /// The future will be aborted if any one dependency fails.
    /// Each handle must belong to the same pool as the one the future will be spawned in.
    dependencies: []const *const Handle = &.{},
};

/// An utility type to spawn single task command buffers.
pub fn Future(Result: type) type {
    return struct {
        handle: Handle,
        result: *const Result,

        const AwaitResult = switch (@typeInfo(Result)) {
            .error_set => error{Aborted} || Result,
            .error_union => |x| anyerror!x.payload,
            else => error{Aborted}!Result,
        };

        /// Deinitializes the future.
        ///
        /// If the future has not yet completed running, it will be detached.
        /// The result of the future is not cleaned up.
        pub fn deinit(self: *const @This()) void {
            self.handle.unref();
        }

        /// Awaits for the completion of the future and returns the result.
        pub fn await(self: *const @This()) AwaitResult {
            return switch (self.handle.waitOn()) {
                .completed => self.result.*,
                .aborted => error.Aborted,
            };
        }
    };
}

/// Spawns a new detached future in the provided pool.
pub fn go(
    executor: Pool,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    options: SpawnFutureOptions,
) SpawnError!void {
    const cleanup = struct {
        fn f() void {}
    }.f;
    return goWithCleanup(executor, function, args, cleanup, .{}, options);
}

/// Spawns a new detached future in the provided pool.
///
/// The cleanup function is invoked after all references to the future cease to exist.
pub fn goWithCleanup(
    executor: Pool,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    cleanup: anytype,
    cleanup_args: std.meta.ArgsTuple(@TypeOf(cleanup)),
    options: SpawnFutureOptions,
) SpawnError!void {
    if (@typeInfo(@TypeOf(function)).@"fn".return_type.? != void) {
        @compileError("expected function with a `void` return type");
    }
    const FutureState = struct {
        allocator: Allocator,
        buffer_len: usize,
        args: @TypeOf(args),
        cleanup_args: @TypeOf(cleanup_args),
        task: Task(void),
        command_buffer: CommandBuffer(void),
    };

    const label_len = if (options.label) |l| l.len else 0;
    const num_entries = blk: {
        var num: usize = 2 + options.dependencies.len;
        if (options.stack_size != null) num += 1;
        if (options.worker != null) num += 1;
        break :blk num;
    };

    const label_start = @sizeOf(FutureState);
    const entries_start = std.mem.alignForward(usize, label_start + label_len, @alignOf(Entry));
    const full_bytes_len = entries_start + (num_entries * @sizeOf(Entry));
    const alloc = options.allocator.alignedAlloc(u8, .of(FutureState), full_bytes_len) catch |e| {
        @call(.auto, cleanup, cleanup_args);
        return e;
    };

    const future: *FutureState = std.mem.bytesAsValue(FutureState, alloc[0..@sizeOf(FutureState)]);
    const label: []u8 = alloc[label_start .. label_start + label_len];
    const entries: []Entry = @alignCast(std.mem.bytesAsSlice(Entry, alloc[entries_start..]));
    std.mem.copyForwards(u8, label, options.label orelse "");

    future.* = .{
        .allocator = options.allocator,
        .buffer_len = full_bytes_len,
        .args = args,
        .cleanup_args = cleanup_args,
        .task = .{
            .on_start = &struct {
                fn f(t: *Task(void)) callconv(.c) void {
                    const fut: *FutureState = @fieldParentPtr("task", t);
                    @call(.auto, function, fut.args);
                }
            }.f,
            .state = {},
        },
        .command_buffer = .{
            .label_ = label.ptr,
            .label_len = label.len,
            .entries_ = entries.ptr,
            .entries_len = entries.len,
            .on_deinit = &struct {
                fn f(b: *CommandBuffer(void)) callconv(.c) void {
                    const fut: *FutureState = @fieldParentPtr("command_buffer", b);
                    const cl_args = fut.cleanup_args;
                    const allocator = fut.allocator;
                    const bytes = std.mem.asBytes(fut).ptr[0..fut.buffer_len];
                    allocator.free(bytes);
                    @call(.auto, cleanup, cl_args);
                }
            }.f,
            .state = {},
        },
    };

    var entries_al = ArrayListUnmanaged(Entry).initBuffer(entries);
    entries_al.appendAssumeCapacity(.{
        .tag = .abort_on_error,
        .payload = .{ .abort_on_error = true },
    });
    if (options.stack_size) |stack_size| entries_al.appendAssumeCapacity(.{
        .tag = .set_min_stack_size,
        .payload = .{ .set_min_stack_size = stack_size },
    });
    if (options.worker) |worker| entries_al.appendAssumeCapacity(.{
        .tag = .select_worker,
        .payload = .{ .select_worker = worker },
    });
    for (options.dependencies) |handle| entries_al.appendAssumeCapacity(.{
        .tag = .wait_on_command_buffer,
        .payload = .{ .wait_on_command_buffer = handle.ref() },
    });
    entries_al.appendAssumeCapacity(.{
        .tag = .enqueue_task,
        .payload = .{ .enqueue_task = @ptrCast(&future.task) },
    });

    try executor.enqueueCommandBufferDetached(&future.command_buffer);
}

/// Spawns a new future in the provided pool.
pub fn init(
    executor: Pool,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    options: SpawnFutureOptions,
) SpawnError!Future(@typeInfo(@TypeOf(function)).@"fn".return_type.?) {
    const cleanup = struct {
        fn f() void {}
    }.f;
    return initWithCleanup(executor, function, args, cleanup, .{}, options);
}

/// Spawns a new future in the provided pool.
///
/// The cleanup function is invoked after all references to the future cease to exist.
pub fn initWithCleanup(
    executor: Pool,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    cleanup: anytype,
    cleanup_args: std.meta.ArgsTuple(@TypeOf(cleanup)),
    options: SpawnFutureOptions,
) SpawnError!Future(@typeInfo(@TypeOf(function)).@"fn".return_type.?) {
    const Result = @typeInfo(@TypeOf(function)).@"fn".return_type.?;
    const FutureState = struct {
        allocator: Allocator,
        buffer_len: usize,
        result: Result = undefined,
        args: @TypeOf(args),
        cleanup_args: @TypeOf(cleanup_args),
        task: Task(void),
        command_buffer: CommandBuffer(void),
    };

    const label_len = if (options.label) |l| l.len else 0;
    const num_entries = blk: {
        var num: usize = 2 + options.dependencies.len;
        if (options.stack_size != null) num += 1;
        if (options.worker != null) num += 1;
        break :blk num;
    };

    const label_start = @sizeOf(FutureState);
    const entries_start = std.mem.alignForward(usize, label_start + label_len, @alignOf(Entry));
    const full_bytes_len = entries_start + (num_entries * @sizeOf(Entry));
    const alloc = options.allocator.alignedAlloc(u8, .of(FutureState), full_bytes_len) catch |e| {
        @call(.auto, cleanup, cleanup_args);
        return e;
    };

    const future: *FutureState = std.mem.bytesAsValue(FutureState, alloc[0..@sizeOf(FutureState)]);
    const label: []u8 = alloc[label_start .. label_start + label_len];
    const entries: []Entry = @alignCast(std.mem.bytesAsSlice(Entry, alloc[entries_start..]));
    std.mem.copyForwards(u8, label, options.label orelse "");

    future.* = .{
        .allocator = options.allocator,
        .buffer_len = full_bytes_len,
        .args = args,
        .cleanup_args = cleanup_args,
        .task = .{
            .on_start = &struct {
                fn f(t: *Task(void)) callconv(.c) void {
                    const fut: *FutureState = @fieldParentPtr("task", t);
                    fut.result = @call(.auto, function, fut.args);
                }
            }.f,
            .state = {},
        },
        .command_buffer = .{
            .label_ = label.ptr,
            .label_len = label.len,
            .entries_ = entries.ptr,
            .entries_len = entries.len,
            .on_deinit = &struct {
                fn f(b: *CommandBuffer(void)) callconv(.c) void {
                    const fut: *FutureState = @fieldParentPtr("command_buffer", b);
                    const cl_args = fut.cleanup_args;
                    const allocator = fut.allocator;
                    const bytes = std.mem.asBytes(fut).ptr[0..fut.buffer_len];
                    allocator.free(bytes);
                    @call(.auto, cleanup, cl_args);
                }
            }.f,
            .state = {},
        },
    };

    var entries_al = ArrayListUnmanaged(Entry).initBuffer(entries);
    entries_al.appendAssumeCapacity(.{
        .tag = .abort_on_error,
        .payload = .{ .abort_on_error = true },
    });
    if (options.stack_size) |stack_size| entries_al.appendAssumeCapacity(.{
        .tag = .set_min_stack_size,
        .payload = .{ .set_min_stack_size = stack_size },
    });
    if (options.worker) |worker| entries_al.appendAssumeCapacity(.{
        .tag = .select_worker,
        .payload = .{ .select_worker = worker },
    });
    for (options.dependencies) |handle| entries_al.appendAssumeCapacity(.{
        .tag = .wait_on_command_buffer,
        .payload = .{ .wait_on_command_buffer = handle.ref() },
    });
    entries_al.appendAssumeCapacity(.{
        .tag = .enqueue_task,
        .payload = .{ .enqueue_task = @ptrCast(&future.task) },
    });

    const handle = try executor.enqueueCommandBuffer(&future.command_buffer);
    return .{ .handle = handle, .result = &future.result };
}

/// Spawns a new future that can be polled by stackless futures.
pub fn initPollable(
    executor: Pool,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    options: SpawnFutureOptions,
) SpawnError!EnqueuedFuture(@typeInfo(@TypeOf(function)).@"fn".return_type.?) {
    const cleanup = struct {
        fn f() void {}
    }.f;
    return initPollableWithCleanup(executor, function, args, cleanup, .{}, options);
}

pub fn initPollableWithCleanup(
    executor: Pool,
    function: anytype,
    args: std.meta.ArgsTuple(@TypeOf(function)),
    cleanup: anytype,
    cleanup_args: std.meta.ArgsTuple(@TypeOf(cleanup)),
    options: SpawnFutureOptions,
) SpawnError!EnqueuedFuture(@typeInfo(@TypeOf(function)).@"fn".return_type.?) {
    const Result = @typeInfo(@TypeOf(function)).@"fn".return_type.?;
    const FutureState = struct {
        waker: ?Waker = null,
        mutex: std.Thread.Mutex = .{},
        allocator: Allocator,
        buffer_len: usize,
        handle: Handle = undefined,
        result: Result = undefined,
        ready: std.atomic.Value(bool) = .init(false),
        args: @TypeOf(args),
        cleanup_args: @TypeOf(cleanup_args),
        task: Task(void),
        command_buffer: CommandBuffer(void),

        fn poll(this: **anyopaque, waker: Waker) Poll(Result) {
            const self: *@This() = @ptrCast(@alignCast(this.*));
            if (self.ready.load(.acquire)) return .{ .ready = self.result };
            self.mutex.lock();
            defer self.mutex.unlock();
            if (self.ready.load(.monotonic)) return .{ .ready = self.result };
            if (self.waker) |w| w.unref();
            self.waker = waker.ref();
            return .pending;
        }

        fn onCleanup(this: **anyopaque) void {
            const self: *@This() = @ptrCast(@alignCast(this.*));
            self.handle.unref();
        }
    };

    const label_len = if (options.label) |l| l.len else 0;
    const num_entries = blk: {
        var num: usize = 2 + options.dependencies.len;
        if (options.stack_size != null) num += 1;
        if (options.worker != null) num += 1;
        break :blk num;
    };

    const label_start = @sizeOf(FutureState);
    const entries_start = std.mem.alignForward(usize, label_start + label_len, @alignOf(Entry));
    const full_bytes_len = entries_start + (num_entries * @sizeOf(Entry));
    const alloc = options.allocator.alignedAlloc(u8, .of(FutureState), full_bytes_len) catch |e| {
        @call(.auto, cleanup, cleanup_args);
        return e;
    };

    const future: *FutureState = std.mem.bytesAsValue(FutureState, alloc[0..@sizeOf(FutureState)]);
    const label: []u8 = alloc[label_start .. label_start + label_len];
    const entries: []Entry = @alignCast(std.mem.bytesAsSlice(Entry, alloc[entries_start..]));
    std.mem.copyForwards(u8, label, options.label orelse "");

    future.* = .{
        .allocator = options.allocator,
        .buffer_len = full_bytes_len,
        .args = args,
        .cleanup_args = cleanup_args,
        .task = .{
            .on_start = &struct {
                fn f(t: *Task(void)) callconv(.c) void {
                    const fut: *FutureState = @fieldParentPtr("task", t);
                    fut.result = @call(.auto, function, fut.args);
                    fut.ready.store(true, .release);
                    fut.mutex.lock();
                    defer fut.mutex.unlock();
                    if (fut.waker) |w| w.wakeUnref();
                    fut.waker = null;
                }
            }.f,
            .state = {},
        },
        .command_buffer = .{
            .label_ = label.ptr,
            .label_len = label.len,
            .entries_ = entries.ptr,
            .entries_len = entries.len,
            .on_deinit = &struct {
                fn f(b: *CommandBuffer(void)) callconv(.c) void {
                    const fut: *FutureState = @fieldParentPtr("command_buffer", b);
                    const cl_args = fut.cleanup_args;
                    const allocator = fut.allocator;
                    if (fut.waker) |w| w.wakeUnref();
                    const buffer = std.mem.asBytes(fut).ptr[0..fut.buffer_len];
                    allocator.free(buffer);
                    @call(.auto, cleanup, cl_args);
                }
            }.f,
            .state = {},
        },
    };

    var entries_al = ArrayListUnmanaged(Entry).initBuffer(entries);
    entries_al.appendAssumeCapacity(.{
        .tag = .abort_on_error,
        .payload = .{ .abort_on_error = true },
    });
    if (options.stack_size) |stack_size| entries_al.appendAssumeCapacity(.{
        .tag = .set_min_stack_size,
        .payload = .{ .set_min_stack_size = stack_size },
    });
    if (options.worker) |worker| entries_al.appendAssumeCapacity(.{
        .tag = .select_worker,
        .payload = .{ .select_worker = worker },
    });
    for (options.dependencies) |handle| entries_al.appendAssumeCapacity(.{
        .tag = .wait_on_command_buffer,
        .payload = .{ .wait_on_command_buffer = handle.ref() },
    });
    entries_al.appendAssumeCapacity(.{
        .tag = .enqueue_task,
        .payload = .{ .enqueue_task = &future.task },
    });

    future.handle = try executor.enqueueCommandBuffer(&future.command_buffer);
    return EnqueuedFuture(Result).init(future, FutureState.poll, FutureState.onCleanup);
}

test "pollable future" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const p = try Pool.init(&.{ .worker_count = 4, .label_ = "test", .label_len = 4 });
    defer {
        p.requestClose();
        p.unref();
    }

    const start = struct {
        fn f(a: usize, b: usize) usize {
            task.sleep(.initMillis(200));
            return a + b;
        }
    }.f;
    var future = try initPollable(
        p,
        start,
        .{ 5, 10 },
        .{ .label = "pollable future", .allocator = std.testing.allocator },
    );
    defer future.deinit();
    var fut = future.intoFuture();

    const awaiter = try fimo_std.tasks.BlockingContext.init();
    defer awaiter.deinit();
    try std.testing.expectEqual(15, fut.awaitBlockingBorrow(awaiter));
}
