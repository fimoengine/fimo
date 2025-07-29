const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;
const DoublyLinkedList = std.DoublyLinkedList;

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const context = @import("../context.zig");
const pub_tasks = @import("../tasks.zig");
const ResourceCount = @import("ResourceCount.zig");
pub const BlockingContext = @import("tasks/BlockingContext.zig");
pub const Task = @import("tasks/Task.zig");
const tracing = @import("tracing.zig");

const tasks = @This();

pub var allocator: Allocator = undefined;
var thread: Thread = undefined;
pub var mutex: Mutex = .{};
pub var cvar: Condition = .{};
pub var running_tasks: usize = 0;
pub var task_count: ResourceCount = .{};
pub var context_count: ResourceCount = .{};
pub var queue: DoublyLinkedList = .{};
pub var should_quit: bool = false;

pub fn init() !void {
    allocator = context.allocator;
    thread = try Thread.spawn(.{ .allocator = allocator }, runEventLoop, .{});
    thread.setName("fimo event loop") catch {};
}

pub fn deinit() void {
    task_count.waitUntilZero();
    context_count.waitUntilZero();
    {
        mutex.lock();
        defer mutex.unlock();
        should_quit = true;
        cvar.signal();
    }
    thread.join();
    thread = undefined;
    should_quit = false;
    std.debug.assert(running_tasks == 0);
    std.debug.assert(queue.len() == 0);
    allocator = undefined;
}

fn runEventLoop() void {
    tracing.registerThread();
    defer tracing.unregisterThread();

    loop: while (true) {
        const node = blk: {
            mutex.lock();
            defer mutex.unlock();
            break :blk queue.popFirst() orelse {
                if (running_tasks == 0 and should_quit) break :loop;
                cvar.wait(&mutex);
                continue;
            };
        };
        const task: *Task = @fieldParentPtr("node", node);
        task.poll();
    }
}

pub fn initErrorFuture(comptime T: type, e: anyerror) pub_tasks.EnqueuedFuture(pub_tasks.Fallible(T)) {
    const Wrapper = struct {
        fn poll(data: **anyopaque, waker: pub_tasks.Waker) pub_tasks.Poll(pub_tasks.Fallible(T)) {
            _ = waker;
            const err_int: std.meta.Int(.unsigned, @bitSizeOf(anyerror)) = @intCast(@intFromPtr(data.*));
            const err = @errorFromInt(err_int);
            return .{ .ready = .{
                .result = AnyError.initError(err).intoResult(),
                .value = undefined,
            } };
        }
    };

    const e_ptr: *anyopaque = @ptrFromInt(@intFromError(e));
    return pub_tasks.EnqueuedFuture(pub_tasks.Fallible(T)).init(
        e_ptr,
        Wrapper.poll,
        null,
    );
}

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn contextNewBlocking(
        blk_ctx: *pub_tasks.BlockingContext,
    ) callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        blk_ctx.* = BlockingContext.init() catch |err| return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    fn futureEnqueue(
        data: ?[*]const u8,
        data_size: usize,
        data_alignment: usize,
        result_size: usize,
        result_alignment: usize,
        poll_fn: *const fn (
            data: ?*anyopaque,
            waker: pub_tasks.Waker,
            result: ?*anyopaque,
        ) callconv(.c) bool,
        cleanup_data_fn: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        cleanup_result_fn: ?*const fn (result: ?*anyopaque) callconv(.c) void,
        future: *pub_tasks.OpaqueFuture,
    ) callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        future.* = Task.init(
            data,
            data_size,
            data_alignment,
            result_size,
            result_alignment,
            poll_fn,
            cleanup_data_fn,
            cleanup_result_fn,
        ) catch |e| return AnyError.initError(e).intoResult();
        return AnyResult.ok;
    }
};

pub const vtable = pub_tasks.VTable{
    .context_new_blocking = &VTableImpl.contextNewBlocking,
    .future_enqueue = &VTableImpl.futureEnqueue,
};
