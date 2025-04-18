const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;

const ProxyAsync = @import("../proxy_context/async.zig");
const RefCount = @import("../RefCount.zig");
const System = @import("System.zig");

const Self = @This();

sys: *System,
refcount: RefCount = .{},

mutex: Mutex = .{},
cvar: Condition = .{},
notified: bool = false,
waiter: ?Thread.Id = null,

pub fn init(sys: *System) Allocator.Error!ProxyAsync.BlockingContext {
    sys.asContext().ref();
    errdefer sys.asContext().unref();

    const self = try sys.allocator.create(Self);
    errdefer sys.allocator.destroy(self);
    self.* = .{ .sys = sys };

    const Wrapper = struct {
        fn deinit(ptr: ?*anyopaque) callconv(.c) void {
            const this: *Self = @alignCast(@ptrCast(ptr));
            this.unref();
        }
        fn waker_ref(ptr: ?*anyopaque) callconv(.c) ProxyAsync.Waker {
            const this: *Self = @alignCast(@ptrCast(ptr));
            return this.asWaker();
        }
        fn block_until_notified(ptr: ?*anyopaque) callconv(.c) void {
            const this: *Self = @alignCast(@ptrCast(ptr));
            this.block_until_notified();
        }
    };

    const context_vtable = ProxyAsync.BlockingContext.VTable{
        .deinit = &Wrapper.deinit,
        .waker_ref = &Wrapper.waker_ref,
        .block_until_notified = &Wrapper.block_until_notified,
    };

    return ProxyAsync.BlockingContext{
        .data = self,
        .vtable = &context_vtable,
    };
}

fn ref(self: *Self) void {
    self.refcount.ref();
}

fn unref(self: *Self) void {
    if (self.refcount.unref() == .noop) return;
    std.debug.assert(self.waiter == null);

    const sys = self.sys;
    sys.allocator.destroy(self);
    sys.asContext().unref();
}

fn asWaker(self: *Self) ProxyAsync.Waker {
    const Wrapper = struct {
        fn ref(data: ?*anyopaque) callconv(.c) ProxyAsync.Waker {
            const this: *Self = @alignCast(@ptrCast(data));
            this.ref();
            return this.asWaker();
        }
        fn unref(data: ?*anyopaque) callconv(.c) void {
            const this: *Self = @alignCast(@ptrCast(data));
            this.unref();
        }
        fn wake(data: ?*anyopaque) callconv(.c) void {
            const this: *Self = @alignCast(@ptrCast(data));
            this.notify();
        }
        fn wakeUnref(data: ?*anyopaque) callconv(.c) void {
            const this: *Self = @alignCast(@ptrCast(data));
            this.notify();
            this.unref();
        }
    };

    const waker_vtable = ProxyAsync.Waker.VTable{
        .ref = &Wrapper.ref,
        .unref = &Wrapper.unref,
        .wake = &Wrapper.wake,
        .wake_unref = &Wrapper.wakeUnref,
        .next = null,
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

fn block_until_notified(self: *Self) void {
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
