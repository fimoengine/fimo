//! Public interface of async subsystem.
//!
//! The async subsystem presents a simple single threaded event loop.
//! The event loop is mainly designed to handle internal tasks, like
//! handling of module events, but also supports arbitrary user tasks.

const std = @import("std");

const c = @import("../../c.zig");
const Context = @import("../proxy_context.zig");
const AnyError = @import("../../AnyError.zig");

context: Context,

const AsyncExecutor = @This();

/// A handle to an event loop, executing futures.
pub const EventLoop = extern struct {
    data: ?*anyopaque,
    vtable: *const EventLoop.VTable,

    /// VTable of an EventLoop.
    ///
    /// Changing the VTable is **not** a breaking change.
    pub const VTable = extern struct {
        join: *const fn (data: ?*anyopaque) callconv(.c) void,
        detach: *const fn (data: ?*anyopaque) callconv(.c) void,
    };

    /// Initializes a new event loop.
    ///
    /// There can only be one event loop at a time, and it will keep
    /// the context alive until it completes its execution.
    pub fn init(ctx: AsyncExecutor, err: *?AnyError) AnyError.Error!EventLoop {
        var loop: EventLoop = undefined;
        const result = ctx.context.vtable.async_v0.start_event_loop(
            ctx.context.data,
            &loop,
        );
        try AnyError.initChecked(err, result);
        return loop;
    }

    /// Utilize the current thread to complete all tasks in the event loop.
    ///
    /// The intended purpose of this function is to complete all remaining tasks
    /// before cleanup, as the context can not be destroyed until the queue is empty.
    /// Upon the completion of all tasks, the funtion will return to the caller.
    pub fn flushWithCurrentThread(ctx: AsyncExecutor, err: *?AnyError) AnyError.Error!void {
        const result = ctx.context.vtable.async_v0.run_to_completion(
            ctx.context.data,
        );
        try AnyError.initChecked(err, result);
    }

    /// Signals the event loop to complete the remaining jobs and exit afterwards.
    ///
    /// The caller will block until the event loop has completed executing.
    pub fn join(self: EventLoop) void {
        self.vtable.join(self.data);
    }

    /// Signals the event loop to complete the remaining jobs and exit afterwards.
    ///
    /// The caller will exit imediately.
    pub fn detach(self: EventLoop) void {
        self.vtable.detach(self.data);
    }
};

/// Waker of a task.
pub const Waker = extern struct {
    data: ?*anyopaque,
    vtable: *const Waker.VTable,

    /// VTable of a Waker.
    ///
    /// Changing the VTable is a breaking change.
    pub const VTable = extern struct {
        ref: *const fn (data: ?*anyopaque) callconv(.c) void,
        unref: *const fn (data: ?*anyopaque) callconv(.c) void,
        wake_unref: *const fn (data: ?*anyopaque) callconv(.c) void,
        wake: *const fn (data: ?*anyopaque) callconv(.c) void,
        next: ?*anyopaque,
    };

    /// Increases the reference count of the waker.
    pub fn ref(self: Waker) void {
        self.vtable.ref(self.data);
    }

    /// Decreases the reference count of the waker.
    pub fn unref(self: Waker) void {
        self.vtable.ref(self.data);
    }

    /// Wakes the task associated with the current waker and
    /// decreases the wakers reference count.
    pub fn wake_unref(self: Waker) void {
        self.vtable.wake_unref(self.data);
    }

    /// Wakes the task associated with the current waker, without
    /// decreasing the reference count of the waker.
    pub fn wake(self: Waker) void {
        self.vtable.wake(self.data);
    }
};

/// A context that blocks the current thread until it is notified.
pub const BlockingContext = extern struct {
    data: ?*anyopaque,
    vtable: *const BlockingContext.VTable,

    /// VTable of an BlockingContext.
    ///
    /// Changing the VTable is **not** a breaking change.
    pub const VTable = extern struct {
        deinit: *const fn (data: ?*anyopaque) callconv(.c) void,
        waker_ref: *const fn (data: ?*anyopaque) callconv(.c) Waker,
        block_until_notified: *const fn (data: ?*anyopaque) callconv(.c) void,
    };

    /// Initializes a new waker.
    pub fn init(ctx: AsyncExecutor, err: *?AnyError) AnyError.Error!BlockingContext {
        var context: BlockingContext = undefined;
        const result = ctx.context.vtable.async_v0.context_new_blocking(
            ctx.context.data,
            &context,
        );
        try AnyError.initChecked(err, result);
        return context;
    }

    /// Deinitializes the context.
    pub fn deinit(self: BlockingContext) void {
        self.vtable.deinit(self.data);
    }

    /// Returns a reference to the waker for the context.
    ///
    /// The caller does not own the waker.
    pub fn waker(self: BlockingContext) Waker {
        return self.vtable.waker_ref(self.data);
    }

    /// Blocks the current thread until it has been notified.
    ///
    /// The thread can be notified through the waker of the context.
    pub fn block_until_notified(self: BlockingContext) void {
        self.vtable.block_until_notified(self.data);
    }

    /// Blocks the current thread until the future is completed.
    pub fn awaitFuture(
        self: BlockingContext,
        comptime T: type,
        future: anytype,
    ) T {
        const Pollable = @typeInfo(@TypeOf(future)).pointer.child;
        if (!@hasDecl(Pollable, "poll"))
            @compileError("expected a pollable, got " ++ @typeName(Pollable));

        const waker_ref = self.waker();
        while (true) {
            switch (future.poll(waker_ref)) {
                .ready => |v| return v,
                .pending => self.block_until_notified(),
            }
        }
    }
};

/// Result of poll operation.
pub fn Poll(comptime T: type) type {
    return union(enum) {
        ready: T,
        pending,
    };
}

/// An asynchronous operation.
pub fn Future(comptime T: type, comptime U: type) type {
    const OptPointer = struct {
        fn ty(comptime V: type) type {
            return if (V == anyopaque or @sizeOf(V) == 0) ?*V else *V;
        }
        fn unwrap(comptime V: type, x: ty(V)) *V {
            return if (V == anyopaque or @sizeOf(V) == 0) &.{} else x;
        }
    };
    const OptT = OptPointer.ty(T);
    const OptU = OptPointer.ty(U);

    return struct {
        data: T,
        poll_fn: *const fn (data: OptT, waker: Waker, result: OptU) bool,
        cleanup_fn: ?*const fn (data: OptT) void,

        /// Initializes a new future.
        pub fn init(data: T, poll_fn: fn (*T, Waker) Poll(U), cleanup_fn: ?fn (*T) void) @This() {
            const Wrapper = struct {
                fn poll(dat: OptT, waker: Waker, result: OptU) bool {
                    const d = OptPointer.unwrap(T, dat);
                    const r = OptPointer.unwrap(U, result);
                    r.* = switch (poll_fn(d, waker)) {
                        .ready => |v| v,
                        .pending => return false,
                    };
                    return true;
                }
                fn cleanup(dat: OptT) void {
                    const d = OptPointer.unwrap(T, dat);
                    if (cleanup_fn) |cl| cl(d);
                }
            };

            return .{
                .data = data,
                .poll_fn = &Wrapper.poll,
                .cleanup_fn = if (cleanup_fn != null) &Wrapper.cleanup else null,
            };
        }

        /// Deinitializes the future.
        ///
        /// Can be called at any time to abort the future.
        pub fn deinit(self: *@This()) void {
            if (self.cleanup_fn) |cl| cl(&self.data);
        }

        /// Polls the future.
        ///
        /// The object may not be moved once it is polled.
        /// Once the future returns `ready` it may not be polled again.
        ///
        /// The waker is not owned by the callee, and it may not decrease
        /// it's reference count without increasing it first.
        pub fn poll(self: *@This(), waker: Waker) Poll(U) {
            var result: U = undefined;
            if (self.poll_fn(&self.data, waker, &result)) return .{ .ready = result };
            return .pending;
        }

        /// Moves the future on the async executor.
        ///
        /// Polling the new future will block the current task.
        pub fn enqueue(
            self: @This(),
            ctx: AsyncExecutor,
            comptime deinit_result_fn: ?fn (*U) void,
            err: *?AnyError,
        ) AnyError.Error!*EnqueuedFuture(U) {
            const This = @This();
            const Wrapper = struct {
                fn poll(data: ?*anyopaque, waker: Waker, result: ?*anyopaque) callconv(.c) bool {
                    const this: *This = @alignCast(@ptrCast(data));
                    const res: *U = if (@sizeOf(U) != 0) @alignCast(@ptrCast(result)) else &.{};
                    return this.poll_fn(&this.data, waker, res);
                }
                fn deinit_data(data: ?*anyopaque) callconv(.c) void {
                    const this: *This = @alignCast(@ptrCast(data));
                    this.deinit();
                }
                fn deinit_result(result: ?*anyopaque) callconv(.c) void {
                    const res: *U = if (@sizeOf(U) != 0) @alignCast(@ptrCast(result)) else &.{};
                    if (deinit_result_fn) |f| f(res);
                }
            };

            var enqueued: *OpaqueFuture = undefined;
            const result = ctx.context.vtable.async_v0.future_enqueue(
                ctx.context.data,
                std.mem.asBytes(&self),
                @sizeOf(@This()),
                @alignOf(@This()),
                @sizeOf(U),
                @alignOf(U),
                &Wrapper.poll,
                &Wrapper.deinit_data,
                &Wrapper.deinit_result,
                &enqueued,
            );
            try AnyError.initChecked(err, result);
            return @alignCast(@ptrCast(enqueued));
        }
    };
}

/// A future with a defined layout.
pub fn ExternFuture(comptime T: type, comptime U: type) type {
    const OptPointer = struct {
        fn ty(comptime V: type) type {
            return if (V == anyopaque or @sizeOf(V) == 0) ?*V else *V;
        }
        fn unwrap(comptime V: type, x: ty(V)) *V {
            return if (V == anyopaque or @sizeOf(V) == 0) &.{} else x;
        }
    };
    const OptT = OptPointer.ty(T);
    const OptU = OptPointer.ty(U);

    return extern struct {
        data: T,
        poll_fn: *const fn (data: OptT, waker: Waker, result: OptU) callconv(.c) bool,
        cleanup_fn: ?*const fn (data: OptT) callconv(.c) void,

        /// Initializes a new future.
        pub fn init(data: T, poll_fn: fn (*T, Waker) Poll(U), cleanup_fn: ?fn (*T) void) @This() {
            const Wrapper = struct {
                fn poll(dat: OptT, waker: Waker, result: OptU) callconv(.c) bool {
                    const d = OptPointer.unwrap(T, dat);
                    const r = OptPointer.unwrap(U, result);
                    r.* = switch (poll_fn(d, waker)) {
                        .ready => |v| v,
                        .pending => return false,
                    };
                    return true;
                }
                fn cleanup(dat: OptT) callconv(.c) void {
                    const d = OptPointer.unwrap(T, dat);
                    if (cleanup_fn) |cl| cl(d);
                }
            };

            return .{
                .data = data,
                .poll_fn = &Wrapper.poll,
                .cleanup_fn = if (cleanup_fn != null) &Wrapper.cleanup else null,
            };
        }

        /// Deinitializes the future.
        ///
        /// Can be called at any time to abort the future.
        pub fn deinit(self: *@This()) void {
            if (self.cleanup_fn) |cl| cl(&self.data);
        }

        /// Polls the future.
        ///
        /// The object may not be moved once it is polled.
        /// Once the future returns `ready` it may not be polled again.
        ///
        /// The waker is not owned by the callee, and it may not decrease
        /// it's reference count without increasing it first.
        pub fn poll(self: *@This(), waker: Waker) Poll(U) {
            var result: U = undefined;
            if (self.poll_fn(&self.data, waker, &result)) return .{ .ready = result };
            return .pending;
        }

        /// Moves the future on the async executor.
        ///
        /// Polling the new future will block the current task.
        pub fn enqueue(
            self: @This(),
            ctx: AsyncExecutor,
            comptime deinit_result_fn: ?fn (*U) void,
            err: *?AnyError,
        ) AnyError.Error!*EnqueuedFuture(U) {
            const This = @This();
            const Wrapper = struct {
                fn poll(data: ?*anyopaque, waker: Waker, result: ?*anyopaque) callconv(.c) bool {
                    const this: *This = @alignCast(@ptrCast(data));
                    const res: *U = if (@sizeOf(U) != 0) @alignCast(@ptrCast(result)) else &.{};
                    return this.poll_fn(&this.data, waker, res);
                }
                fn deinit_data(data: ?*anyopaque) callconv(.c) void {
                    const this: *This = @alignCast(@ptrCast(data));
                    this.deinit();
                }
                fn deinit_result(result: ?*anyopaque) callconv(.c) void {
                    const res: *U = if (@sizeOf(U) != 0) @alignCast(@ptrCast(result)) else &.{};
                    if (deinit_result_fn) |f| f(res);
                }
            };

            var enqueued: *OpaqueFuture = undefined;
            const result = ctx.context.vtable.async_v0.future_enqueue(
                ctx.context.data,
                std.mem.asBytes(&self),
                @sizeOf(@This()),
                @alignOf(@This()),
                @sizeOf(U),
                @alignOf(U),
                &Wrapper.poll,
                &Wrapper.deinit_data,
                &Wrapper.deinit_result,
                &enqueued,
            );
            try AnyError.initChecked(err, result);
            return @alignCast(@ptrCast(enqueued));
        }
    };
}

/// An enqueued future.
pub fn EnqueuedFuture(comptime T: type) type {
    return ExternFuture(*anyopaque, T);
}

/// An enqueued future with an unknown result type.
pub const OpaqueFuture = EnqueuedFuture(anyopaque);

/// VTable of the async subsystem.
///
/// Changing the VTable is a breaking change.
pub const VTable = extern struct {
    run_to_completion: *const fn (ctx: *anyopaque) callconv(.c) c.FimoResult,
    start_event_loop: *const fn (ctx: *anyopaque, loop: *EventLoop) callconv(.c) c.FimoResult,
    context_new_blocking: *const fn (ctx: *anyopaque, context: *BlockingContext) callconv(.c) c.FimoResult,
    future_enqueue: *const fn (
        ctx: *anyopaque,
        data: ?[*]const u8,
        data_size: usize,
        data_alignment: usize,
        result_size: usize,
        result_alignment: usize,
        poll: *const fn (data: ?*anyopaque, waker: Waker, result: ?*anyopaque) callconv(.c) bool,
        cleanup_data: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        cleanup_result: ?*const fn (result: ?*anyopaque) callconv(.c) void,
        future: **OpaqueFuture,
    ) callconv(.c) c.FimoResult,
};
