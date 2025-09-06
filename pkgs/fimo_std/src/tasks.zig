//! Async subsystem of the context.
//!
//! Exposes a simple single threaded event loop for arbitrary tasks.
//! Requires an initialized global context.
const std = @import("std");

const AnyError = @import("AnyError.zig");
const AnyResult = AnyError.AnyResult;
const ctx = @import("ctx.zig");

const SelfTy = @This();

/// Waker of a task.
pub const Waker = extern struct {
    data: ?*anyopaque,
    vtable: *const Waker.VTable,

    pub const VTable = extern struct {
        ref: *const fn (data: ?*anyopaque) callconv(.c) Waker,
        unref: *const fn (data: ?*anyopaque) callconv(.c) void,
        wake_unref: *const fn (data: ?*anyopaque) callconv(.c) void,
        wake: *const fn (data: ?*anyopaque) callconv(.c) void,
    };

    /// Increases the reference count of the waker.
    pub fn ref(self: Waker) Waker {
        return self.vtable.ref(self.data);
    }

    /// Decreases the reference count of the waker.
    pub fn unref(self: Waker) void {
        self.vtable.unref(self.data);
    }

    /// Wakes the task associated with the current waker and decreases the wakers reference count.
    pub fn wakeUnref(self: Waker) void {
        self.vtable.wake_unref(self.data);
    }

    /// Wakes the task associated with the current waker, without decreasing the reference count of
    /// the waker.
    pub fn wake(self: Waker) void {
        self.vtable.wake(self.data);
    }
};

/// A waiter that blocks the current thread until it is notified.
///
/// The waiter is intended to be used by threads other than the event loop thread, as they are not
/// bound to a waker. Using this waiter inside the event loop will result in a deadlock.
pub const Waiter = extern struct {
    data: ?*anyopaque,
    vtable: *const Waiter.VTable,

    pub const VTable = extern struct {
        deinit: *const fn (data: ?*anyopaque) callconv(.c) void,
        waker_ref: *const fn (data: ?*anyopaque) callconv(.c) Waker,
        block: *const fn (data: ?*anyopaque) callconv(.c) void,
    };

    /// Initializes a new waiter.
    pub fn init() ctx.Error!Waiter {
        var context: Waiter = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.tasks_v0.waiter_init(&context).intoErrorUnion();
        return context;
    }

    /// Deinitializes the waiter.
    pub fn deinit(self: Waiter) void {
        self.vtable.deinit(self.data);
    }

    /// Returns a reference to the waker for the waiter.
    ///
    /// The caller does not own the waker.
    pub fn waker(self: Waiter) Waker {
        return self.vtable.waker_ref(self.data);
    }

    /// Blocks the current thread until it has been notified.
    ///
    /// The thread can be notified through the waker of the waiter.
    pub fn block(self: Waiter) void {
        self.vtable.block(self.data);
    }

    /// Blocks the current thread until the future is completed.
    pub fn awaitFuture(
        self: Waiter,
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
                .pending => self.block(),
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
pub fn Future(
    comptime T: type,
    comptime U: type,
    poll_fn: fn (*T, Waker) Poll(U),
    deinit_fn: ?fn (*T) void,
) type {
    return struct {
        data: T,

        pub const Result = U;
        pub const Future = @This();

        /// Initializes the future.
        pub fn init(data: T) @This() {
            return .{ .data = data };
        }

        /// Deinitializes the future.
        ///
        /// Can be called at any time to abort the future.
        pub fn deinit(self: *@This()) void {
            if (deinit_fn) |f| f(&self.data);
        }

        /// Constructs a future from the current instance.
        pub fn intoFuture(self: @This()) @This() {
            return self;
        }

        /// Constructs an extern future from the current instance.
        pub fn intoExternFuture(self: @This()) ExternFuture(@This(), U) {
            return ExternFuture(@This(), U).init(
                self,
                poll,
                if (deinit_fn) deinit else null,
            );
        }

        /// Polls the future.
        ///
        /// The object may not be moved once it is polled. Once the future returns `ready` it may
        /// not be polled again.
        ///
        /// The waker is not owned by the callee, and it may not decrease it's reference count
        /// without increasing it first.
        pub fn poll(self: *@This(), waker: Waker) Poll(U) {
            return poll_fn(&self.data, waker);
        }

        /// Awaits for the completion of the future using the specified context.
        ///
        /// The context must provide a generic method called `awaitFuture`, that takes the return
        /// type as the first parameter and a pointer to the future as the second parameter, and
        /// blocks the current task until the future polls as ready.
        pub fn awaitBlockingBorrow(self: *@This(), context: anytype) U {
            return context.awaitFuture(U, self);
        }

        /// Awaits for the completion of the future using the specified context.
        ///
        /// Like `awaitBlockingBorrow`, but this method takes ownership of the future.
        pub fn awaitBlocking(self: @This(), context: anytype) U {
            var this = self;
            const result = this.awaitBlockingBorrow(context);
            this.deinit();
            return result;
        }

        /// Maps the result of the future to another type.
        pub fn map(self: @This(), comptime V: type, map_fn: anytype) MapFuture(@This(), V, map_fn) {
            return MapFuture(@This(), V, map_fn).init(self);
        }

        /// Moves the future on the async executor.
        ///
        /// Polling the new future will block the current task.
        pub fn enqueue(self: @This(), comptime deinit_result_fn: ?fn (*U) void) ctx.Error!OpaqueFuture(U) {
            const This = @This();
            const Wrapper = struct {
                fn poll(data: ?*anyopaque, waker: Waker, result: ?*anyopaque) callconv(.c) bool {
                    const this: *This = @ptrCast(@alignCast(data));
                    const res: *U = if (@sizeOf(U) != 0) @ptrCast(@alignCast(result)) else &.{};
                    switch (this.poll(waker)) {
                        .ready => |v| {
                            res.* = v;
                            return true;
                        },
                        .pending => return false,
                    }
                }
                fn deinit_data(data: ?*anyopaque) callconv(.c) void {
                    const this: *This = @ptrCast(@alignCast(data));
                    this.deinit();
                }
                fn deinit_result(result: ?*anyopaque) callconv(.c) void {
                    const res: *U = if (@sizeOf(U) != 0) @ptrCast(@alignCast(result)) else &.{};
                    if (deinit_result_fn) |f| f(res);
                }
            };

            var enqueued: EnqueuedFuture = undefined;
            const handle = ctx.Handle.getHandle();
            try handle.tasks_v0.future_enqueue(
                std.mem.asBytes(&self),
                @sizeOf(@This()),
                @alignOf(@This()),
                @sizeOf(U),
                @alignOf(U),
                &Wrapper.poll,
                &Wrapper.deinit_data,
                &Wrapper.deinit_result,
                &enqueued,
            ).intoErrorUnion();
            return @bitCast(enqueued);
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
        deinit_fn: ?*const fn (data: OptT) callconv(.c) void,

        pub const Result = U;
        pub const Future = SelfTy.Future(@This(), U, poll, deinit);

        /// Initializes a new future.
        pub fn init(data: T, poll_fn: fn (*T, Waker) Poll(U), deinit_fn: ?fn (*T) void) @This() {
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
                    if (deinit_fn) |cl| cl(d);
                }
            };

            return .{
                .data = data,
                .poll_fn = &Wrapper.poll,
                .deinit_fn = if (deinit_fn != null) &Wrapper.cleanup else null,
            };
        }

        /// Deinitializes the future.
        ///
        /// Can be called at any time to abort the future.
        pub fn deinit(self: *@This()) void {
            if (self.deinit_fn) |cl| cl(&self.data);
        }

        /// Constructs a future from the current instance.
        pub fn intoFuture(self: @This()) @This().Future {
            return @This().Future.init(self);
        }

        /// Constructs an extern future from the current instance.
        pub fn intoExternFuture(self: @This()) @This() {
            return self;
        }

        /// Polls the future.
        ///
        /// The object may not be moved once it is polled. Once the future returns `ready` it may
        /// not be polled again.
        ///
        /// The waker is not owned by the callee, and it may not decrease it's reference count
        /// without increasing it first.
        pub fn poll(self: *@This(), waker: Waker) Poll(U) {
            var result: U = undefined;
            if (self.poll_fn(&self.data, waker, &result)) return .{ .ready = result };
            return .pending;
        }
    };
}

/// A future with an opaque handle and specified return type.
pub fn OpaqueFuture(comptime T: type) type {
    return ExternFuture(*anyopaque, T);
}

/// Type of an enqueued future.
pub const EnqueuedFuture = OpaqueFuture(anyopaque);

/// A future that returns immediately.
pub fn ReadyFuture(comptime T: type, deinit_fn: ?fn (*T) void) type {
    return struct {
        data: ?T,

        pub const Result = T;
        pub const Future = SelfTy.Future(
            @This(),
            T,
            poll,
            if (deinit_fn) deinit else null,
        );

        pub fn init(data: T) @This() {
            return .{ .data = data };
        }

        pub fn deinit(self: *@This()) void {
            if (deinit_fn) |f| {
                if (self.data) |*data| f(data);
                self.data = null;
            }
        }

        pub fn intoFuture(self: @This()) @This().Future {
            return @This().Future.init(self);
        }

        pub fn poll(self: *@This(), waker: Waker) Poll(T) {
            _ = waker;
            const x = self.data.?;
            self.data = null;
            return .{ .ready = x };
        }
    };
}

/// A future that transforms the output type of another future.
pub fn MapFuture(comptime T: type, comptime U: type, map_fn: anytype) type {
    const MapFn = @TypeOf(map_fn);
    std.debug.assert(@typeInfo(MapFn).@"fn".params.len == 1);
    std.debug.assert(@typeInfo(MapFn).@"fn".return_type.? == U);

    return struct {
        data: T,

        pub const Result = U;
        pub const Future = SelfTy.Future(@This(), U, poll, deinit);

        pub fn init(data: T) @This() {
            return .{ .data = data };
        }

        pub fn deinit(self: *@This()) void {
            self.data.deinit();
        }

        pub fn intoFuture(self: @This()) @This().Future {
            return @This().Future.init(self);
        }

        pub fn poll(self: *@This(), waker: Waker) Poll(U) {
            return switch (self.data.poll(waker)) {
                .ready => |v| .{ .ready = map_fn(v) },
                .pending => .pending,
            };
        }
    };
}

/// An integer able to represent every state of a finite state machine.
pub fn FSMState(comptime T: type) type {
    var num_states = 0;
    for (std.meta.declarations(T)) |decl| {
        if (std.mem.startsWith(u8, decl.name, "__state")) num_states += 1;
    }

    return std.math.IntFittingRange(0, num_states);
}

/// An operation of the finite state machine.
pub const FSMOp = enum {
    next,
    yield,
    ret,
};

/// An operation of the finite state machine.
pub fn FSMOpExt(comptime T: type) type {
    return union(enum) {
        transition: FSMState(T),
        next,
        yield,
        ret,
    };
}

/// Operations permitted while unwinding the finite state machine.
pub const FSMUnwindOp = enum {
    unwind,
    ret,
};

/// Operations permitted while unwinding the finite state machine.
pub fn FSMUnwindOpExt(comptime T: type) type {
    return union(enum) {
        transition: FSMState(T),
        unwind,
        ret,
    };
}

/// Reason for the unwind operation.
pub const FSMUnwindReason = enum {
    abort,
    completed,
    err,
};

/// A future from a finite state machine.
pub fn FSMFuture(comptime T: type) type {
    comptime var num_states = 0;
    for (std.meta.declarations(T)) |decl| {
        if (std.mem.startsWith(u8, decl.name, "__state")) num_states += 1;
    }

    const ret_f = @field(T, "__ret");
    const U = @typeInfo(@TypeOf(ret_f)).@"fn".return_type.?;

    const no_unwind: bool = if (@hasDecl(T, "__no_unwind"))
        @field(T, "__no_unwind")
    else
        false;

    const no_abort: bool = if (@hasDecl(T, "__no_abort"))
        @field(T, "__no_abort")
    else
        false;

    return struct {
        state: FSMState(T) = 0,
        data: T,

        pub const Data = T;
        pub const Result = U;
        pub const Future = SelfTy.Future(
            @This(),
            U,
            poll,
            if (no_unwind and !@hasDecl(T, "deinit")) null else deinit,
        );

        pub fn init(data: T) @This() {
            return .{ .data = data };
        }

        pub fn deinit(self: *@This()) void {
            self.unwind(.abort);
            if (@hasDecl(T, "deinit")) {
                self.data.deinit();
            }
        }

        pub fn intoFuture(self: @This()) @This().Future {
            return @This().Future.init(self);
        }

        fn unwind(self: *@This(), comptime reason: FSMUnwindReason) void {
            if (no_abort and reason == .abort) {
                if (self.state != 0 and self.state != num_states)
                    @panic("abort not supported by the future");
            }

            if (!no_unwind and num_states != 0) {
                sm: switch (self.state) {
                    inline 0...num_states - 1 => |i| {
                        const unwind_func = std.fmt.comptimePrint("__unwind{}", .{i});
                        if (!@hasDecl(T, unwind_func)) {
                            const next_state = if (i != 0) i - 1 else num_states;
                            continue :sm next_state;
                        }

                        const f = @field(T, unwind_func);
                        switch (@typeInfo(@TypeOf(f)).@"fn".return_type.?) {
                            void => {
                                f(&self.data, reason);
                                const next_state = if (i != 0) i - 1 else num_states;
                                continue :sm next_state;
                            },
                            FSMUnwindOp => {
                                switch (f(&self.data, reason)) {
                                    .unwind => {
                                        const next_state = if (i != 0) i - 1 else num_states;
                                        continue :sm next_state;
                                    },
                                    .ret => {
                                        continue :sm num_states;
                                    },
                                }
                            },
                            FSMUnwindOpExt(T) => {
                                switch (f(&self.data, reason)) {
                                    .transition => |next| {
                                        std.debug.assert(next < num_states);
                                        continue :sm next;
                                    },
                                    .unwind => {
                                        const next_state = if (i != 0) i - 1 else num_states;
                                        continue :sm next_state;
                                    },
                                    .ret => {
                                        continue :sm num_states;
                                    },
                                }
                            },
                            else => |t| @compileError("invalid unwind return type " ++ @typeName(t)),
                        }
                    },
                    else => self.state = num_states,
                }
            } else {
                self.state = num_states;
            }
        }

        pub fn poll(self: *@This(), waker: Waker) Poll(U) {
            if (num_states != 0) {
                sm: switch (self.state) {
                    inline 0...num_states - 1 => |i| {
                        const state_func = std.fmt.comptimePrint("__state{}", .{i});
                        const f = @field(T, state_func);
                        const result = f(&self.data, waker);

                        const op = if (@typeInfo(@TypeOf(result)) == .error_union)
                            result catch |err| {
                                const tr = @errorReturnTrace();
                                const set_err = @field(T, "__set_err");
                                if (@typeInfo(@TypeOf(set_err)).@"fn".params.len == 2)
                                    set_err(&self.data, err)
                                else if (@typeInfo(@TypeOf(set_err)).@"fn".params.len == 3)
                                    set_err(&self.data, tr, err);
                                self.state = i;
                                self.unwind(.err);
                                break :sm;
                            }
                        else
                            result;

                        switch (@TypeOf(op)) {
                            void => {
                                const next_state: comptime_int = i + 1;
                                if (next_state == num_states) self.state = i;
                                continue :sm next_state;
                            },
                            FSMOp => {
                                const x: FSMOp = op;
                                switch (x) {
                                    .next => {
                                        const next_state: comptime_int = i + 1;
                                        if (next_state == num_states) self.state = i;
                                        continue :sm next_state;
                                    },
                                    .yield => {
                                        self.state = i;
                                        return .pending;
                                    },
                                    .ret => {
                                        self.state = i;
                                        continue :sm num_states;
                                    },
                                }
                            },
                            FSMOpExt(T) => {
                                const x: FSMOpExt(T) = op;
                                switch (x) {
                                    .transition => |next| {
                                        std.debug.assert(next < num_states);
                                        continue :sm next;
                                    },
                                    .next => {
                                        const next_state: comptime_int = i + 1;
                                        if (next_state == num_states) self.state = i;
                                        continue :sm next_state;
                                    },
                                    .yield => {
                                        self.state = i;
                                        return .pending;
                                    },
                                    .ret => {
                                        self.state = i;
                                        continue :sm num_states;
                                    },
                                }
                            },
                            else => |t| @compileError("invalid state return type " ++ @typeName(t)),
                        }
                    },
                    else => self.unwind(.completed),
                }
            }

            return .{ .ready = ret_f(&self.data) };
        }
    };
}

/// A fallible result.
pub fn Fallible(comptime T: type) type {
    return extern struct {
        result: AnyResult,
        value: T,

        const Self = @This();

        /// A wrapper function.
        pub fn Wrapper(comptime E: type) fn (E!T) Self {
            return struct {
                fn f(value: E!T) Self {
                    return Self.wrap(value);
                }
            }.f;
        }

        /// Extracts the contained result.
        pub fn unwrap(self: Self) ctx.Error!T {
            if (self.result.isErr()) {
                ctx.setResult(self.result);
                return error.OperationFailed;
            }
            return self.value;
        }

        /// Wraps an error union into a fallible result.
        pub fn wrap(value: anyerror!T) Self {
            const x = value catch |err| {
                return .{
                    .result = AnyError.initError(err).intoResult(),
                    .value = undefined,
                };
            };
            return .{
                .result = AnyResult.ok,
                .value = x,
            };
        }
    };
}

/// base VTable of the async subsystem.
///
/// Changing this definition is a breaking change.
pub const VTable = extern struct {
    waiter_init: *const fn (context: *Waiter) callconv(.c) ctx.Status,
    future_enqueue: *const fn (
        data: ?[*]const u8,
        data_size: usize,
        data_alignment: usize,
        result_size: usize,
        result_alignment: usize,
        poll: *const fn (data: ?*anyopaque, waker: Waker, result: ?*anyopaque) callconv(.c) bool,
        cleanup_data: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        cleanup_result: ?*const fn (result: ?*anyopaque) callconv(.c) void,
        future: *EnqueuedFuture,
    ) callconv(.c) ctx.Status,
};
