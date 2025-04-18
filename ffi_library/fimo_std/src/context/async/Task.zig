const std = @import("std");
const AtomicValue = std.atomic.Value;
const Mutex = std.Thread.Mutex;
const DoublyLinkedList = std.DoublyLinkedList;

const ProxyAsync = @import("../proxy_context/async.zig");
const ProxyTracing = @import("../proxy_context/tracing.zig");
const RefCount = @import("../RefCount.zig");
const System = @import("System.zig");

const Self = @This();

sys: *System,
refcount: RefCount = .{},
call_stack: ProxyTracing.CallStack,

mutex: Mutex = .{},
state: State = .{},
waiter: ?ProxyAsync.Waker = null,
node: DoublyLinkedList.Node = .{},

result_size: usize,
data: ?*anyopaque,
result: ?*anyopaque,
buffer: []u8,

poll_fn: *const fn (
    data: ?*anyopaque,
    waker: ProxyAsync.Waker,
    result: ?*anyopaque,
) callconv(.c) bool,
cleanup_data_fn: ?*const fn (data: ?*anyopaque) callconv(.c) void,
cleanup_result_fn: ?*const fn (result: ?*anyopaque) callconv(.c) void,

const State = struct {
    state: AtomicValue(u8) = AtomicValue(u8).init(@bitCast(Bits{})),
    waiter: ProxyAsync.Waker = undefined,

    // W(X): Written by X
    // WO(X): Written once by X
    //
    // S(X): Synchronizes with X
    //
    // EL: Event loop
    // EX: External task
    // * : EL + EX
    //
    // `notified`: Resume notification reveiced
    //      W(*)
    //
    // `enqueued`: Task is inserted in the queue
    //      W(*)
    //      enqueued => !completed
    //
    // `completed`: Internal task has run to completion.
    //      WO(EL)
    //      S(data)
    //      completed => !enqueued
    //
    // `consumed`: Result has been consumed
    //      WO(*)
    //      S(result)
    //      consumed => !enqueued
    //      consumed => completed
    //      consumed => !waiting
    //      consumed => !waiter_locked
    //
    // `detached`: Task abort requested
    //      WO(EX)
    //      detached => !waiting
    //      detached => !waiter_locked
    //
    // `waiting`: Waiter is registered
    //      W(*)
    //      waiting: !consumed
    //      waiting: !detached
    //
    // `waiter_locked`
    //      W(EX)
    //      S(waiter)
    //      waiter_locked: !consumed
    //      waiter_locked: !detached
    const Bits = packed struct(u8) {
        notified: bool = false,
        enqueued: bool = false,
        completed: bool = false,
        consumed: bool = false,
        detached: bool = false,
        waiting: bool = false,
        waiter_locked: bool = false,
        padding: u1 = undefined,
    };

    const EnqueueOp = enum { noop, enqueue };

    fn notify(self: *@This()) EnqueueOp {
        _ = self.state.bitSet(
            @bitOffsetOf(Bits, "notified"),
            .monotonic,
        );

        const state: Bits = @bitCast(self.state.load(.monotonic));
        if (state.completed or state.enqueued) return .noop;
        return self.enqueue();
    }

    fn enqueue(self: *@This()) EnqueueOp {
        var expected: Bits = @bitCast(self.state.load(.monotonic));
        while (true) {
            std.debug.assert(!expected.completed);
            if (!expected.notified) return .noop;
            if (expected.enqueued) return .noop;

            var next = expected;
            next.enqueued = true;
            next.notified = false;
            if (self.state.cmpxchgWeak(
                @bitCast(expected),
                @bitCast(next),
                .monotonic,
                .monotonic,
            )) |v| expected = @bitCast(v) else return .enqueue;
        }
    }

    fn dequeue(self: *@This(), completed: bool) union(enum) { noop, cleanup: bool, enqueue } {
        if (completed) {
            // Use a release store to force the result being written before the atomic op.
            const mask = Bits{ .enqueued = true, .completed = true };
            const state: Bits = @bitCast(self.state.fetchXor(@bitCast(mask), .release));
            std.debug.assert(state.enqueued);
            std.debug.assert(!state.completed);
            std.debug.assert(!state.consumed);

            if (!state.detached) return .{ .cleanup = false };
            std.debug.assert(!state.waiting);
            std.debug.assert(!state.waiter_locked);

            // Prevent the cleanup from being reordered before the atomic operation.
            _ = self.state.bitSet(
                @bitOffsetOf(Bits, "consumed"),
                .acquire,
            );
            return .{ .cleanup = true };
        } else {
            const mask = Bits{ .enqueued = true };
            const state: Bits = @bitCast(self.state.fetchXor(@bitCast(mask), .monotonic));
            std.debug.assert(state.enqueued);
            std.debug.assert(!state.completed);
            std.debug.assert(!state.consumed);

            if (state.notified and self.enqueue() == .enqueue) return .enqueue;
            return .noop;
        }
    }

    fn detach(self: *@This()) enum { noop, cleanup } {
        var state: Bits = @bitCast(self.state.load(.monotonic));
        std.debug.assert(!state.detached);
        if (state.consumed) return .noop;

        const locked = self.state.bitSet(
            @bitOffsetOf(Bits, "waiter_locked"),
            .acquire,
        );
        if (locked != 0) @panic("A future may only be polled by one task");
        if (state.waiting) self.waiter.unref();

        // Set `waiter_locked = 0`, `waiting = 0` and `detached = 1`.
        const mask = Bits{
            .detached = true,
            .waiting = state.waiting,
            .waiter_locked = true,
        };
        state = @bitCast(self.state.fetchXor(@bitCast(mask), .release));
        std.debug.assert(!state.detached);
        if (!state.completed) return .noop;

        // Prevent the cleanup from being reordered before the atomic operation.
        const consumed = self.state.bitSet(
            @bitOffsetOf(Bits, "consumed"),
            .acquire,
        );
        if (consumed == 0) return .noop;
        return .cleanup;
    }

    fn lock_waiter(self: *@This()) Bits {
        var locked = self.state.bitSet(
            @bitOffsetOf(Bits, "waiter_locked"),
            .acquire,
        );
        while (locked == 1) {
            std.atomic.spinLoopHint();
            locked = self.state.bitSet(
                @bitOffsetOf(Bits, "waiter_locked"),
                .acquire,
            );
        }
        return @bitCast(self.state.load(.acquire));
    }

    fn unlock_waiter(self: *@This(), toggle_waiting: bool) void {
        const mask = Bits{
            .waiting = toggle_waiting,
            .waiter_locked = true,
        };
        _ = self.state.fetchXor(@bitCast(mask), .release);
    }

    fn wake(self: *@This()) void {
        const state = self.lock_waiter();
        std.debug.assert(state.completed);
        if (!state.waiting) {
            self.unlock_waiter(false);
            return;
        }
        self.waiter.wakeUnref();
        self.unlock_waiter(true);
    }

    fn wait(self: *@This(), waiter: ProxyAsync.Waker) enum { noop, consume } {
        const state: Bits = self.lock_waiter();
        std.debug.assert(!state.consumed);
        std.debug.assert(!state.detached);

        if (state.waiting) self.waiter.unref();
        if (!state.completed) {
            self.waiter = waiter.ref();
            self.unlock_waiter(!state.waiting);
            return .noop;
        }
        self.unlock_waiter(state.waiting);

        // Prevent the read of the result from being reordered before the atomic operation.
        const consumed = self.state.bitSet(
            @bitOffsetOf(Bits, "consumed"),
            .acquire,
        );
        std.debug.assert(consumed == 0);
        return .consume;
    }
};

pub fn initFuture(
    comptime T: type,
    sys: *System,
    future: *const T,
) !ProxyAsync.EnqueuedFuture(T.Result) {
    const Wrapper = struct {
        fn poll(
            data: ?*anyopaque,
            waker: ProxyAsync.Waker,
            result: ?*anyopaque,
        ) callconv(.c) bool {
            const this: *T = @alignCast(@ptrCast(data));
            return switch (this.poll(waker)) {
                .ready => |v| {
                    if (@sizeOf(T.Result) != 0) {
                        @as(*T.Result, @alignCast(@ptrCast(result))).* = v;
                    }
                    return true;
                },
                .pending => false,
            };
        }
        fn deinit_data(data: ?*anyopaque) callconv(.c) void {
            const this: *T = @alignCast(@ptrCast(data));
            this.deinit();
        }
        fn deinit_result(result: ?*anyopaque) callconv(.c) void {
            switch (@typeInfo(T.Result)) {
                .@"struct", .@"union", .@"enum" => if (@hasField(T.Result, "deinit")) {
                    const res: *T.Result = if (@sizeOf(T.Result) != 0) @alignCast(@ptrCast(result)) else &.{};
                    res.deinit();
                },
                else => {},
            }
        }
    };
    const f = try init(
        sys,
        std.mem.asBytes(future),
        @sizeOf(T),
        @alignOf(T),
        @sizeOf(T.Result),
        @alignOf(T.Result),
        Wrapper.poll,
        Wrapper.deinit_data,
        Wrapper.deinit_result,
    );
    return @bitCast(f);
}

pub fn init(
    sys: *System,
    data: ?[*]const u8,
    data_size: usize,
    data_alignment: usize,
    result_size: usize,
    result_alignment: usize,
    poll_fn: *const fn (
        data: ?*anyopaque,
        waker: ProxyAsync.Waker,
        result: ?*anyopaque,
    ) callconv(.c) bool,
    cleanup_data_fn: ?*const fn (data: ?*anyopaque) callconv(.c) void,
    cleanup_result_fn: ?*const fn (result: ?*anyopaque) callconv(.c) void,
) !ProxyAsync.OpaqueFuture {
    sys.asContext().ref();
    errdefer sys.asContext().unref();
    const allocator = sys.allocator;

    const buffer_align: usize = @max(data_alignment, result_alignment);
    const buffer_size = std.mem.alignForward(usize, data_size, result_alignment) +
        result_size + (buffer_align - 1);
    const buffer = try allocator.alloc(u8, buffer_size);
    errdefer allocator.free(buffer);

    const data_offset: usize = if (data_size != 0)
        std.mem.alignPointerOffset(buffer.ptr, data_alignment).?
    else
        0;

    const result_offset: usize = if (result_size != 0)
        data_offset + data_size + std.mem.alignPointerOffset(
            buffer[data_offset + data_size ..].ptr,
            result_alignment,
        ).?
    else
        0;

    const buffer_data: ?*anyopaque = if (data) |d| blk: {
        std.debug.assert(data_size != 0);
        const src_bytes = d[0..data_size];
        const dst_bytes = buffer[data_offset .. data_offset + data_size];
        @memcpy(dst_bytes, src_bytes);
        break :blk dst_bytes.ptr;
    } else blk: {
        std.debug.assert(data_size == 0);
        break :blk null;
    };
    const result_data: ?*anyopaque = if (result_size != 0) &buffer[result_offset] else null;
    std.debug.assert(std.mem.isAligned(@intFromPtr(buffer_data), data_alignment));
    std.debug.assert(std.mem.isAligned(@intFromPtr(result_data), result_alignment));

    const call_stack = sys.asContext().tracing.createCallStack();
    errdefer call_stack.deinit();

    const self = try allocator.create(Self);
    errdefer allocator.destroy(self);

    self.* = .{
        .sys = sys,
        .call_stack = call_stack,

        .result_size = result_size,
        .data = buffer_data,
        .result = result_data,
        .buffer = buffer,

        .poll_fn = poll_fn,
        .cleanup_data_fn = cleanup_data_fn,
        .cleanup_result_fn = cleanup_result_fn,
    };

    // Increase the ref count for the public future.
    self.ref();

    const op = self.state.notify();
    std.debug.assert(op == .enqueue);
    self.enqueueAndIncreaseCount();

    const future = ProxyAsync.ExternFuture(*@This(), anyopaque){
        .data = self,
        .poll_fn = &pollPublic,
        .cleanup_fn = &deinitPublic,
    };

    return @bitCast(future);
}

fn ref(self: *Self) void {
    self.refcount.ref();
}

fn unref(self: *Self) void {
    if (self.refcount.unref() == .noop) return;
    const state: State.Bits = @bitCast(self.state.state.load(.monotonic));
    std.debug.assert(!state.enqueued);
    std.debug.assert(state.completed);
    std.debug.assert(state.consumed);
    std.debug.assert(!state.waiting);
    std.debug.assert(!state.waiter_locked);
    const ctx = self.sys.asContext();
    self.call_stack.deinit();
    const allocator = self.sys.allocator;
    allocator.free(self.buffer);
    allocator.destroy(self);
    ctx.unref();
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
    switch (self.state.notify()) {
        .noop => {},
        .enqueue => self.enqueue(),
    }
}

fn pollPublic(
    self_ptr: **Self,
    waker: ProxyAsync.Waker,
    result: ?*anyopaque,
) callconv(.c) bool {
    const self = self_ptr.*;
    if (self.state.wait(waker) == .noop) return false;

    if (result) |res| {
        std.debug.assert(self.result_size != 0);
        const src_bytes = @as([*]u8, @ptrCast(self.result.?))[0..self.result_size];
        const dst_bytes = @as([*]u8, @ptrCast(res))[0..self.result_size];
        @memcpy(dst_bytes, src_bytes);
    } else {
        std.debug.assert(self.result_size == 0);
    }

    return true;
}

fn deinitPublic(self_ptr: **Self) callconv(.c) void {
    const self = self_ptr.*;
    if (self.state.detach() == .cleanup)
        if (self.cleanup_result_fn) |f| f(self.result);
    self.unref();
}

fn enqueueAndIncreaseCount(self: *Self) void {
    self.sys.mutex.lock();
    defer self.sys.mutex.unlock();
    self.sys.enqueued_tasks += 1;
    self.sys.queue.append(&self.node);
    self.sys.cvar.signal();
}

fn decreaseCount(self: *Self) void {
    self.sys.mutex.lock();
    defer self.sys.mutex.unlock();
    self.sys.enqueued_tasks -= 1;
    // No signal is necessary, as it is only called by the event loop.
}

fn enqueue(self: *Self) void {
    self.sys.mutex.lock();
    defer self.sys.mutex.unlock();
    self.sys.queue.append(&self.node);
    self.sys.cvar.signal();
}

pub fn poll(self: *Self) void {
    const ctx = self.sys.asContext();
    ctx.tracing.suspendCurrentCallStack(false);
    const main_stack = self.call_stack.replaceActive();
    ctx.tracing.resumeCurrentCallStack();

    const completed = self.poll_fn(self.data, self.asWaker(), self.result);

    ctx.tracing.suspendCurrentCallStack(false);
    self.call_stack = main_stack.replaceActive();
    ctx.tracing.resumeCurrentCallStack();

    switch (self.state.dequeue(completed)) {
        .noop => {},
        .cleanup => |detached| {
            if (self.cleanup_data_fn) |f| f(self.data);
            if (detached) if (self.cleanup_result_fn) |f| f(self.result);
            self.state.wake();
            self.decreaseCount();
            self.unref();
        },
        .enqueue => self.enqueue(),
    }
}
