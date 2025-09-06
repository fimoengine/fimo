const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;

const pub_tasks = @import("../../tasks.zig");
const RefCount = @import("../RefCount.zig");
const tasks = @import("../tasks.zig");

const Self = @This();

refcount: RefCount = .{},

mutex: Mutex = .{},
cvar: Condition = .{},
notified: bool = false,
waiter: ?Thread.Id = null,

pub fn init() Allocator.Error!pub_tasks.Waiter {
    const self = try tasks.allocator.create(Self);
    errdefer tasks.allocator.destroy(self);
    self.* = .{};
    tasks.context_count.increase();

    const Wrapper = struct {
        fn deinit(ptr: ?*anyopaque) callconv(.c) void {
            const this: *Self = @ptrCast(@alignCast(ptr));
            this.unref();
        }
        fn waker_ref(ptr: ?*anyopaque) callconv(.c) pub_tasks.Waker {
            const this: *Self = @ptrCast(@alignCast(ptr));
            return this.asWaker();
        }
        fn block(ptr: ?*anyopaque) callconv(.c) void {
            const this: *Self = @ptrCast(@alignCast(ptr));
            this.block();
        }
    };

    const waiter_vtable = pub_tasks.Waiter.VTable{
        .deinit = &Wrapper.deinit,
        .waker_ref = &Wrapper.waker_ref,
        .block = &Wrapper.block,
    };

    return .{
        .data = self,
        .vtable = &waiter_vtable,
    };
}

fn ref(self: *Self) void {
    self.refcount.ref();
}

fn unref(self: *Self) void {
    if (self.refcount.unref() == .noop) return;
    std.debug.assert(self.waiter == null);
    tasks.allocator.destroy(self);
    tasks.context_count.decrease();
}

fn asWaker(self: *Self) pub_tasks.Waker {
    const Wrapper = struct {
        fn ref(data: ?*anyopaque) callconv(.c) pub_tasks.Waker {
            const this: *Self = @ptrCast(@alignCast(data));
            this.ref();
            return this.asWaker();
        }
        fn unref(data: ?*anyopaque) callconv(.c) void {
            const this: *Self = @ptrCast(@alignCast(data));
            this.unref();
        }
        fn wake(data: ?*anyopaque) callconv(.c) void {
            const this: *Self = @ptrCast(@alignCast(data));
            this.notify();
        }
        fn wakeUnref(data: ?*anyopaque) callconv(.c) void {
            const this: *Self = @ptrCast(@alignCast(data));
            this.notify();
            this.unref();
        }
    };

    const waker_vtable = pub_tasks.Waker.VTable{
        .ref = &Wrapper.ref,
        .unref = &Wrapper.unref,
        .wake = &Wrapper.wake,
        .wake_unref = &Wrapper.wakeUnref,
    };

    return .{
        .data = self,
        .vtable = &waker_vtable,
    };
}

fn notify(self: *Self) void {
    self.mutex.lock();
    defer self.mutex.unlock();
    self.notified = true;
    self.cvar.signal();
}

fn block(self: *Self) void {
    self.mutex.lock();
    defer self.mutex.unlock();
    const id = Thread.getCurrentId();
    while (!self.notified) {
        if (self.waiter != null) @panic("a context may only be used by one thread");
        self.waiter = id;
        self.cvar.wait(&self.mutex);
        self.waiter = null;
    }
    self.notified = false;
}
