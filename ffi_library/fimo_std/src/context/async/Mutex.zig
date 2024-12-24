const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const DoublyLinkedList = std.DoublyLinkedList;

const ProxyAsync = @import("../proxy_context/async.zig");

const Self = @This();

mutex: Mutex = .{},
waiters: WaitQueue = .{},

pub fn tryLock(self: *Self) bool {
    return self.mutex.tryLock();
}

pub fn lock(self: *Self) void {
    self.mutex.lock();
}

pub fn lockAsync(self: *Self, allocator: Allocator) LockOp {
    return LockOp.init(.{ .mutex = self, .allocator = allocator });
}

pub fn unlock(self: *Self) void {
    self.mutex.unlock();
    self.waiters.wakeOne();
}

pub fn assertEmpty(self: *Self) void {
    self.waiters.assertEmpty();
}

const Waiter = struct {
    waiter: *const anyopaque,
    waker: ProxyAsync.Waker,
    allocator: Allocator,
};

const WaitQueue = struct {
    mutex: Mutex = .{},
    queue: DoublyLinkedList(Waiter) = .{},

    fn wait(
        allocator: Allocator,
        mutex: *Self,
        waiter: *const anyopaque,
        waker: ProxyAsync.Waker,
    ) Allocator.Error!ProxyAsync.Poll(void) {
        const self = &mutex.waiters;
        self.mutex.lock();
        defer self.mutex.unlock();

        if (mutex.tryLock()) return .ready;

        const node = try allocator.create(DoublyLinkedList(Waiter).Node);
        node.* = .{
            .data = .{
                .waiter = waiter,
                .waker = waker.ref(),
                .allocator = allocator,
            },
        };

        self.queue.append(node);
        return .pending;
    }

    fn wakeOne(self: *WaitQueue) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.queue.popFirst()) |node| {
            const waker = node.data.waker;
            const allocator = node.data.allocator;
            allocator.destroy(node);
            waker.wakeUnref();
        }
    }

    fn removeFromList(self: *WaitQueue, waiter: *const anyopaque) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        var current = self.queue.first;
        while (current) |node| {
            if (node.data.waiter == waiter) {
                self.queue.remove(node);

                const waker = node.data.waker;
                const allocator = node.data.allocator;
                allocator.destroy(node);
                waker.unref();
                return;
            }
            current = node.next;
        }

        // If we were not in the list, we have been woken up and
        // need to wake up someone else to not incurr a deadlock.
        if (self.queue.popFirst()) |node| {
            const waker = node.data.waker;
            const allocator = node.data.allocator;
            allocator.destroy(node);
            waker.wakeUnref();
        }
    }

    fn assertEmpty(self: *WaitQueue) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        std.debug.assert(self.queue.len == 0);
    }
};

pub const LockOp = ProxyAsync.FSMFuture(struct {
    mutex: *Self,
    allocator: Allocator,
    ret: __result = undefined,

    pub const __result = Allocator.Error!void;
    const __op = ProxyAsync.FSMOp;
    const __poll_op = Allocator.Error!__op;

    pub fn __set_err(self: *@This(), err: Allocator.Error) void {
        self.ret = err;
    }

    pub fn __ret(self: *@This()) __result {
        return self.ret;
    }

    pub fn __unwind0(self: *@This(), reason: ProxyAsync.FSMUnwindReason) void {
        if (reason == .abort) self.mutex.waiters.removeFromList(self);
    }

    pub fn __state0(self: *@This(), waker: ProxyAsync.Waker) __poll_op {
        if (self.mutex.tryLock()) return .ret;

        return switch (try WaitQueue.wait(
            self.allocator,
            self.mutex,
            self,
            waker,
        )) {
            .pending => .yield,
            .ready => .ret,
        };
    }
});
