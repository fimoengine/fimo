const std = @import("std");

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const context = @import("../context.zig");
const pub_tasks = @import("../tasks.zig");
pub const BlockingContext = @import("tasks/BlockingContext.zig");
pub const EventLoop = @import("tasks/EventLoop.zig");
const System = @import("tasks/System.zig");
pub const Task = @import("tasks/Task.zig");

const tasks = @This();

pub fn init() !void {
    try System.init();
}

pub fn deinit() void {
    System.deinit();
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
    fn runToCompletion() callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        System.startEventLoop(true) catch |err| return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    fn startEventLoop(loop: *pub_tasks.EventLoop) callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        loop.* = EventLoop.init() catch |err| return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

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
    .run_to_completion = &VTableImpl.runToCompletion,
    .start_event_loop = &VTableImpl.startEventLoop,
    .context_new_blocking = &VTableImpl.contextNewBlocking,
    .future_enqueue = &VTableImpl.futureEnqueue,
};
