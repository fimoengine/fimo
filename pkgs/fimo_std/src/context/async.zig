const std = @import("std");

const c = @import("c");

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const Context = @import("../context.zig");
const pub_tasks = @import("../tasks.zig");
pub const BlockingContext = @import("async/BlockingContext.zig");
pub const EventLoop = @import("async/EventLoop.zig");
const System = @import("async/System.zig");
pub const Task = @import("async/Task.zig");

const Self = @This();

sys: System,

pub fn init(ctx: *Context) !Self {
    return Self{ .sys = try System.init(ctx) };
}

pub fn deinit(self: *Self) void {
    self.sys.deinit();
}

pub fn asContext(self: *Self) *Context {
    return @fieldParentPtr("async", self);
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
        std.debug.assert(Context.is_init);
        Context.global.async.sys.startEventLoop(true) catch |err|
            return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    fn startEventLoop(loop: *pub_tasks.EventLoop) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        loop.* = EventLoop.init(&Context.global.async.sys) catch |err|
            return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    fn contextNewBlocking(
        context: *pub_tasks.BlockingContext,
    ) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        context.* = BlockingContext.init(&Context.global.async.sys) catch |err|
            return AnyError.initError(err).intoResult();
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
        std.debug.assert(Context.is_init);
        future.* = Task.init(
            &Context.global.async.sys,
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
