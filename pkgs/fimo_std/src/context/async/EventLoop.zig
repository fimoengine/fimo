const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;

const ProxyAsync = @import("../proxy_context/async.zig");
const System = @import("System.zig");

const Self = @This();

sys: *System,
thread: Thread,

pub fn init(sys: *System) !ProxyAsync.EventLoop {
    sys.asContext().ref();
    errdefer sys.asContext().unref();

    const loop = try sys.allocator.create(Self);
    errdefer sys.allocator.destroy(loop);

    loop.sys = sys;
    loop.thread = try sys.startEventLoopThread();

    const loop_vtable = ProxyAsync.EventLoop.VTable{
        .join = &Self.join,
        .detach = &Self.detach,
    };
    return ProxyAsync.EventLoop{
        .data = loop,
        .vtable = &loop_vtable,
    };
}

fn join(ptr: ?*anyopaque) callconv(.c) void {
    const self: *Self = @alignCast(@ptrCast(ptr));
    self.sys.stopEventLoop();
    self.thread.join();

    const sys = self.sys;
    sys.allocator.destroy(self);
    sys.asContext().unref();
}

fn detach(ptr: ?*anyopaque) callconv(.c) void {
    const self: *Self = @alignCast(@ptrCast(ptr));
    self.sys.stopEventLoop();
    self.thread.detach();

    const sys = self.sys;
    sys.allocator.destroy(self);
    sys.asContext().unref();
}
