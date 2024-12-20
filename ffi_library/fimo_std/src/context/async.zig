const std = @import("std");
const Allocator = std.mem.Allocator;
const Thread = std.Thread;
const Mutex = Thread.Mutex;
const Condition = Thread.Condition;
const Queue = std.DoublyLinkedList;

const c = @import("../c.zig");
const AnyError = @import("../AnyError.zig");

const RefCount = @import("RefCount.zig");

const Context = @import("../context.zig");
const ProxyAsync = @import("proxy_context/async.zig");
const ProxyTracing = @import("proxy_context/tracing.zig");

const Self = @This();

allocator: Allocator,
mutex: Mutex = .{},
cvar: Condition = .{},
queue: Queue(Task) = .{},
running: bool = false,
should_quit: bool = false,

pub fn init(ctx: *Context) !Self {
    return .{ .allocator = ctx.allocator };
}

pub fn deinit(self: *Self) void {
    self.mutex.lock();
    defer self.mutex.unlock();
    std.debug.assert(self.queue.len == 0);
    std.debug.assert(!self.running);
}

fn asContext(self: *Self) *Context {
    return @fieldParentPtr("async", self);
}

fn startEventLoop(self: *Self, exit_on_completion: bool) !void {
    {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.running) return error.AlreadyRunning;
        self.running = true;
        self.should_quit = exit_on_completion;
    }
    self.executorEventLoop();
}

fn startEventLoopThread(self: *Self) !Thread {
    {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.running) return error.AlreadyRunning;
        self.running = true;
        self.should_quit = false;
    }
    errdefer {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.running = false;
        self.should_quit = false;
    }

    const f = struct {
        fn f(this: *Self) !void {
            this.asContext().ref();
            defer this.asContext().unref();

            var err: ?AnyError = null;
            defer if (err) |e| e.deinit();

            this.asContext().tracing.registerThread(&err) catch |e| @panic(@errorName(e));
            defer this.asContext().tracing.unregisterThread() catch |e| @panic(@errorName(e));
            this.executorEventLoop();
        }
    }.f;
    return Thread.spawn(.{}, f, .{self});
}

fn stopEventLoop(self: *Self) void {
    {
        self.mutex.lock();
        defer self.mutex.unlock();
        std.debug.assert(self.running);
        self.should_quit = true;
    }
    self.cvar.signal();
}

fn executorEventLoop(self: *Self) void {
    while (true) {
        self.mutex.lock();
        defer self.mutex.unlock();
        if (self.queue.len == 0 and self.should_quit) break;
        const task = self.queue.popFirst() orelse {
            self.cvar.wait(&self.mutex);
            continue;
        };

        self.mutex.unlock();
        task.data.poll();
        self.mutex.lock();
    }

    {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.running = false;
    }
}

const EventLoop = struct {
    sys: *Self,
    thread: Thread,

    fn init(sys: *Self) !ProxyAsync.EventLoop {
        sys.asContext().ref();
        errdefer sys.asContext().unref();

        const loop = try sys.allocator.create(EventLoop);
        errdefer sys.allocator.destroy(loop);

        loop.sys = sys;
        loop.thread = try sys.startEventLoopThread();

        const loop_vtable = ProxyAsync.EventLoop.VTable{
            .join = &EventLoop.join,
            .detach = &EventLoop.detach,
        };
        return ProxyAsync.EventLoop{
            .data = loop,
            .vtable = &loop_vtable,
        };
    }

    fn join(ptr: ?*anyopaque) callconv(.c) void {
        const self: *EventLoop = @alignCast(@ptrCast(ptr));
        self.sys.stopEventLoop();
        self.thread.join();

        const sys = self.sys;
        sys.allocator.destroy(self);
        sys.asContext().unref();
    }

    fn detach(ptr: ?*anyopaque) callconv(.c) void {
        const self: *EventLoop = @alignCast(@ptrCast(ptr));
        self.sys.stopEventLoop();
        self.thread.detach();

        const sys = self.sys;
        sys.allocator.destroy(self);
        sys.asContext().unref();
    }
};

const BlockingContext = struct {
    sys: *Self,
    refcount: RefCount = .{},

    mutex: Mutex = .{},
    cvar: Condition = .{},
    notified: bool = false,
    waiter: ?Thread.Id = null,

    fn init(sys: *Self) Allocator.Error!ProxyAsync.BlockingContext {
        sys.asContext().ref();
        errdefer sys.asContext().unref();

        const self = try sys.allocator.create(BlockingContext);
        errdefer sys.allocator.destroy(self);
        self.* = .{ .sys = sys };

        const Wrapper = struct {
            fn deinit(ptr: ?*anyopaque) callconv(.c) void {
                const this: *BlockingContext = @alignCast(@ptrCast(ptr));
                this.unref();
            }
            fn waker_ref(ptr: ?*anyopaque) callconv(.c) ProxyAsync.Waker {
                const this: *BlockingContext = @alignCast(@ptrCast(ptr));
                return this.waker_ref();
            }
            fn block_until_notified(ptr: ?*anyopaque) callconv(.c) void {
                const this: *BlockingContext = @alignCast(@ptrCast(ptr));
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

    fn ref(self: *BlockingContext) void {
        self.refcount.ref();
    }

    fn unref(self: *BlockingContext) void {
        if (self.refcount.unref() == .noop) return;
        std.debug.assert(self.waiter == null);

        const sys = self.sys;
        sys.allocator.destroy(self);
        sys.asContext().unref();
    }

    fn waker_ref(self: *BlockingContext) ProxyAsync.Waker {
        const Wrapper = struct {
            fn ref(data: ?*anyopaque) callconv(.c) ProxyAsync.Waker {
                const this: *BlockingContext = @alignCast(@ptrCast(data));
                this.ref();
                return this.waker_ref();
            }
            fn unref(data: ?*anyopaque) callconv(.c) void {
                const this: *BlockingContext = @alignCast(@ptrCast(data));
                this.unref();
            }
            fn wake(data: ?*anyopaque) callconv(.c) void {
                const this: *BlockingContext = @alignCast(@ptrCast(data));
                this.notify();
            }
            fn wakeUnref(data: ?*anyopaque) callconv(.c) void {
                const this: *BlockingContext = @alignCast(@ptrCast(data));
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

    fn notify(self: *BlockingContext) void {
        self.mutex.lock();
        defer self.mutex.unlock();
        self.notified = true;
        self.cvar.signal();
    }

    fn block_until_notified(self: *BlockingContext) void {
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
};

const Task = struct {
    const Node = Queue(Task).Node;

    sys: *Self,
    refcount: RefCount = .{},
    call_stack: *ProxyTracing.CallStack,

    mutex: Mutex = .{},
    state: packed struct {
        notified: bool = true,
        enqueued: bool = false,
        consumed: bool = false,
        detached: bool = false,
        completed: bool = false,
    } = .{},
    waiter: ?ProxyAsync.Waker = null,

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

    pub fn init(
        sys: *Self,
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
        err: *?AnyError,
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

        const call_stack = try sys.asContext().tracing.createCallStack(err);
        errdefer sys.asContext().tracing.destroyCallStack(call_stack) catch unreachable;

        const node = try allocator.create(Node);
        errdefer allocator.destroy(node);

        node.* = .{
            .data = .{
                .sys = sys,
                .call_stack = call_stack,

                .result_size = result_size,
                .data = buffer_data,
                .result = result_data,
                .buffer = buffer,

                .poll_fn = poll_fn,
                .cleanup_data_fn = cleanup_data_fn,
                .cleanup_result_fn = cleanup_result_fn,
            },
        };

        // Increase the ref count for the public future.
        node.data.ref();
        node.data.tryEnqueue();

        const future = ProxyAsync.ExternFuture(*@This(), anyopaque){
            .data = &node.data,
            .poll_fn = &pollPublic,
            .cleanup_fn = &deinitPublic,
        };

        return @bitCast(future);
    }

    fn ref(self: *Task) void {
        self.refcount.ref();
    }

    fn unref(self: *Task) void {
        if (self.refcount.unref() == .noop) return;
        std.debug.assert(!self.state.enqueued);
        std.debug.assert(self.state.completed);
        std.debug.assert(self.state.consumed or self.state.detached);
        std.debug.assert(!(self.state.consumed and self.state.detached));
        const ctx = self.sys.asContext();
        ctx.tracing.destroyCallStack(self.call_stack) catch unreachable;
        const allocator = self.sys.allocator;
        allocator.free(self.buffer);
        const node = self.asNode();
        allocator.destroy(node);
        ctx.unref();
    }

    fn asNode(self: *Task) *Node {
        return @fieldParentPtr("data", self);
    }

    fn asWaker(self: *Task) ProxyAsync.Waker {
        const Wrapper = struct {
            fn ref(data: ?*anyopaque) callconv(.c) ProxyAsync.Waker {
                const this: *Task = @alignCast(@ptrCast(data));
                this.ref();
                return this.asWaker();
            }
            fn unref(data: ?*anyopaque) callconv(.c) void {
                const this: *Task = @alignCast(@ptrCast(data));
                this.unref();
            }
            fn wake(data: ?*anyopaque) callconv(.c) void {
                const this: *Task = @alignCast(@ptrCast(data));
                this.notify();
            }
            fn wakeUnref(data: ?*anyopaque) callconv(.c) void {
                const this: *Task = @alignCast(@ptrCast(data));
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

    fn notify(self: *Task) void {
        const state = blk: {
            self.mutex.lock();
            defer self.mutex.unlock();
            self.state.notified = true;
            break :blk self.state;
        };
        if (state.completed or state.enqueued) return;
        self.tryEnqueue();
    }

    fn pollPublic(
        self_ptr: **Task,
        waker: ProxyAsync.Waker,
        result: ?*anyopaque,
    ) callconv(.c) bool {
        const self = self_ptr.*;
        {
            self.mutex.lock();
            defer self.mutex.unlock();
            std.debug.assert(!self.state.consumed);
            std.debug.assert(!self.state.detached);
            std.debug.assert(self.waiter == null);

            if (!self.state.completed) {
                self.waiter = waker.ref();
                return false;
            }

            self.state.consumed = true;
        }

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

    fn deinitPublic(self_ptr: **Task) callconv(.c) void {
        const self = self_ptr.*;
        const state = blk: {
            self.mutex.lock();
            std.debug.assert(self.waiter == null);
            if (self.state.consumed) {
                self.mutex.unlock();
                self.unref();
                return;
            }
            defer self.mutex.unlock();

            self.state.detached = true;
            break :blk self.state;
        };

        if (state.completed) {
            if (self.cleanup_result_fn) |f| f(self.result);
        }
        self.unref();
    }

    fn tryEnqueue(self: *Task) void {
        {
            self.mutex.lock();
            defer self.mutex.unlock();
            if (self.state.enqueued) return;
            std.debug.assert(self.state.notified);
            std.debug.assert(!self.state.completed);
            self.state.enqueued = true;
            self.state.notified = false;
        }

        self.sys.mutex.lock();
        defer self.sys.mutex.unlock();
        self.sys.queue.append(self.asNode());
    }

    fn poll(self: *Task) void {
        const ctx = self.sys.asContext();
        ctx.tracing.suspendCurrentCallStack(false) catch unreachable;
        const main_stack = ctx.tracing.replaceCurrentCallStack(self.call_stack) catch unreachable;
        ctx.tracing.resumeCurrentCallStack() catch unreachable;

        const completed = self.poll_fn(self.data, self.asWaker(), self.result);

        ctx.tracing.suspendCurrentCallStack(false) catch unreachable;
        self.call_stack = ctx.tracing.replaceCurrentCallStack(main_stack) catch unreachable;
        ctx.tracing.resumeCurrentCallStack() catch unreachable;

        const state = blk: {
            self.mutex.lock();
            defer self.mutex.unlock();
            self.state.enqueued = false;
            self.state.completed = completed;
            break :blk self.state;
        };

        if (completed) {
            if (self.cleanup_data_fn) |f| f(self.data);
            if (state.detached) {
                std.debug.assert(!state.consumed);
                if (self.cleanup_result_fn) |f| f(self.result);
            }

            {
                self.mutex.lock();
                defer self.mutex.unlock();

                if (self.waiter) |w| w.wake_unref();
                self.waiter = null;
            }

            self.unref();
            return;
        }
        if (!state.notified) return;
        self.tryEnqueue();
    }
};

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn runToCompletion(ptr: *anyopaque) callconv(.c) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.@"async".startEventLoop(true) catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }

    fn startEventLoop(ptr: *anyopaque, loop: *ProxyAsync.EventLoop) callconv(.c) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        loop.* = EventLoop.init(&ctx.@"async") catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }

    fn contextNewBlocking(
        ptr: *anyopaque,
        context: *ProxyAsync.BlockingContext,
    ) callconv(.c) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        context.* = BlockingContext.init(&ctx.@"async") catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }

    fn futureEnqueue(
        ptr: *anyopaque,
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
        future: *ProxyAsync.OpaqueFuture,
    ) callconv(.c) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        var err: ?AnyError = null;
        future.* = Task.init(
            &ctx.@"async",
            data,
            data_size,
            data_alignment,
            result_size,
            result_alignment,
            poll_fn,
            cleanup_data_fn,
            cleanup_result_fn,
            &err,
        ) catch |e| switch (e) {
            error.FfiError => return AnyError.intoCResult(err),
            else => return AnyError.initError(e).err,
        };
        return AnyError.intoCResult(null);
    }
};

pub const vtable = ProxyAsync.VTable{
    .run_to_completion = &VTableImpl.runToCompletion,
    .start_event_loop = &VTableImpl.startEventLoop,
    .context_new_blocking = &VTableImpl.contextNewBlocking,
    .future_enqueue = &VTableImpl.futureEnqueue,
};
