const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;

const pub_tasks = @import("../../tasks.zig");
const System = @import("System.zig");

const Self = @This();

thread: Thread,

pub fn init() !pub_tasks.EventLoop {
    const loop = try System.allocator.create(Self);
    errdefer System.allocator.destroy(loop);

    loop.thread = try System.startEventLoopThread();

    const loop_vtable = pub_tasks.EventLoop.VTable{
        .join = &Self.join,
        .detach = &Self.detach,
    };
    return pub_tasks.EventLoop{
        .data = loop,
        .vtable = &loop_vtable,
    };
}

fn join(ptr: ?*anyopaque) callconv(.c) void {
    const self: *Self = @alignCast(@ptrCast(ptr));
    System.stopEventLoop();
    self.thread.join();
    System.allocator.destroy(self);
}

fn detach(ptr: ?*anyopaque) callconv(.c) void {
    const self: *Self = @alignCast(@ptrCast(ptr));
    System.stopEventLoop();
    self.thread.detach();
    System.allocator.destroy(self);
}
