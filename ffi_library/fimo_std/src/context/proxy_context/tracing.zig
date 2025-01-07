//! Public interface of the tracing subsystem.

const std = @import("std");
const builtin = @import("builtin");

const c = @import("../../c.zig");
const Context = @import("../proxy_context.zig");
const Time = @import("../../time.zig").Time;

context: Context,

const Tracing = @This();

/// Tracing levels.
pub const Level = enum(i32) {
    off,
    err,
    warn,
    info,
    debug,
    trace,
};

/// Metadata for a span and event.
pub const Metadata = extern struct {
    next: ?*const anyopaque = null,
    name: [*:0]const u8,
    target: [*:0]const u8,
    level: Level,
    file_name: ?[*:0]const u8 = null,
    line_number: i32 = -1,
};

/// A period of time, during which events can occur.
pub const Span = extern struct {
    handle: *anyopaque,
    vtable: *const Span.VTable,

    /// VTable of a span.
    ///
    /// Adding fields to the vtable is not a breaking change.
    pub const VTable = extern struct {
        deinit: *const fn (ptr: *anyopaque) callconv(.c) void,
    };

    /// Creates a new span with the default formatter and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message is formatted with the default formatter of the zig standard library. The message
    /// may be cut of, if the length exceeds the internal formatting buffer size.
    pub inline fn init(
        ctx: Tracing,
        name: ?[:0]const u8,
        target: ?[:0]const u8,
        level: Level,
        location: std.builtin.SourceLocation,
        comptime fmt: []const u8,
        args: anytype,
    ) Span {
        const desc = &struct {
            var desc = SpanDesc{
                .metadata = &.{
                    .name = name orelse location.fn_name,
                    .target = target orelse location.module,
                    .level = level,
                    .file_name = location.file,
                    .line_number = @intCast(location.line),
                },
            };
        }.desc;
        return Span.initCustom(
            ctx,
            desc,
            stdFormatter(fmt, @TypeOf(args)),
            &args,
        );
    }

    /// Creates a new span with a custom formatter and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// subsystem may use a formatting buffer of a fixed size. The formatter is expected to cut-of
    /// the message after reaching that specified size. The `desc` must remain valid until the span
    /// is destroyed.
    pub fn initCustom(
        ctx: Tracing,
        desc: *const SpanDesc,
        formatter: *const Formatter,
        data: ?*const anyopaque,
    ) Span {
        return ctx.context.vtable.tracing_v0.span_create(
            ctx.context.data,
            desc,
            formatter,
            data,
        );
    }

    /// Creates a new error span with the default formatter and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message is formatted with the default formatter of the zig standard library. The message
    /// may be cut of, if the length exceeds the internal formatting buffer size.
    pub inline fn initErr(
        ctx: Tracing,
        name: ?[:0]const u8,
        target: ?[:0]const u8,
        location: std.builtin.SourceLocation,
        comptime fmt: []const u8,
        args: anytype,
    ) Span {
        return Span.init(
            ctx,
            name,
            target,
            .err,
            location,
            fmt,
            args,
        );
    }

    /// Creates a new warn span with the default formatter and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message is formatted with the default formatter of the zig standard library. The message
    /// may be cut of, if the length exceeds the internal formatting buffer size.
    pub inline fn initWarn(
        ctx: Tracing,
        name: ?[:0]const u8,
        target: ?[:0]const u8,
        location: std.builtin.SourceLocation,
        comptime fmt: []const u8,
        args: anytype,
    ) Span {
        return Span.init(
            ctx,
            name,
            target,
            .warn,
            location,
            fmt,
            args,
        );
    }

    /// Creates a new info span with the default formatter and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message is formatted with the default formatter of the zig standard library. The message
    /// may be cut of, if the length exceeds the internal formatting buffer size.
    pub inline fn initInfo(
        ctx: Tracing,
        name: ?[:0]const u8,
        target: ?[:0]const u8,
        location: std.builtin.SourceLocation,
        comptime fmt: []const u8,
        args: anytype,
    ) Span {
        return Span.init(
            ctx,
            name,
            target,
            .info,
            location,
            fmt,
            args,
        );
    }

    /// Creates a new debug span with the default formatter and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message is formatted with the default formatter of the zig standard library. The message
    /// may be cut of, if the length exceeds the internal formatting buffer size.
    pub inline fn initDebug(
        ctx: Tracing,
        name: ?[:0]const u8,
        target: ?[:0]const u8,
        location: std.builtin.SourceLocation,
        comptime fmt: []const u8,
        args: anytype,
    ) Span {
        return Span.init(
            ctx,
            name,
            target,
            .debug,
            location,
            fmt,
            args,
        );
    }

    /// Creates a new trace span with the default formatter and enters it.
    ///
    /// If successful, the newly created span is used as the context for succeeding events. The
    /// message is formatted with the default formatter of the zig standard library. The message
    /// may be cut of, if the length exceeds the internal formatting buffer size.
    pub inline fn initTrace(
        ctx: Tracing,
        name: ?[:0]const u8,
        target: ?[:0]const u8,
        location: std.builtin.SourceLocation,
        comptime fmt: []const u8,
        args: anytype,
    ) Span {
        return Span.init(
            ctx,
            name,
            target,
            .trace,
            location,
            fmt,
            args,
        );
    }

    /// Exits and destroys a span.
    ///
    /// The events won't occur inside the context of the exited span anymore. The span must be the
    /// span at the top of the current call stack. The span may not be in use prior to a call to
    /// this function, and may not be used afterwards.
    ///
    /// This function must be called while the owning call stack is bound by the current thread.
    pub fn deinit(self: Span) void {
        self.vtable.deinit(self.handle);
    }
};

/// Descriptor of a new span.
pub const SpanDesc = extern struct {
    next: ?*const anyopaque = null,
    metadata: *const Metadata,
};

/// An event to be traced.
pub const Event = extern struct {
    next: ?*const anyopaque = null,
    metadata: *const Metadata,
};

/// A call stack.
///
/// Each call stack represents a unit of computation, like a thread. A call stack is active on only
/// one thread at any given time. The active call stack of a thread can be swapped, which is useful
/// for tracing where a `M:N` threading model is used. In that case, one would create one stack for
/// each task, and activate it when the task is resumed.
pub const CallStack = extern struct {
    handle: *anyopaque,
    vtable: *const CallStack.VTable,

    /// VTable of a call stack.
    ///
    /// Adding fields to the vtable is not a breaking change.
    pub const VTable = extern struct {
        deinit: *const fn (handle: *anyopaque) callconv(.c) void,
        replace_active: *const fn (handle: *anyopaque) callconv(.c) CallStack,
        unblock: *const fn (handle: *anyopaque) callconv(.c) void,
    };

    /// Creates a new empty call stack.
    ///
    /// If successful, the new call stack is marked as suspended. The new call stack is not set to
    /// be the active call stack.
    pub fn init(ctx: Tracing) CallStack {
        return ctx.context.vtable.tracing_v0.create_call_stack(ctx.context.data);
    }

    /// Destroys an empty call stack.
    ///
    /// Marks the completion of a task. Before calling this function, the call stack must be empty,
    /// i.e., there must be no active spans on the stack, and must not be active. If successful,
    /// the call stack may not be used afterwards. The active call stack of the thread is destroyed
    /// automatically, on thread exit or during destruction of the context. The caller must own the
    /// call stack uniquely.
    pub fn deinit(self: CallStack) void {
        self.vtable.deinit(self.handle);
    }

    /// Switches the call stack of the current thread.
    ///
    /// If successful, this call stack will be used as the active call stack of the calling thread.
    /// The old call stack is returned, enabling the caller to switch back to it afterwards. This
    /// call stack must be in a suspended, but unblocked, state and not be active. The active call
    /// stack must also be in a suspended state, but may also be blocked.
    pub fn replaceActive(self: CallStack) CallStack {
        return self.vtable.replace_active(self.handle);
    }

    /// Unblocks a blocked call stack.
    ///
    /// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
    /// marked as blocked.
    pub fn unblock(self: CallStack) void {
        self.vtable.unblock(self.handle);
    }

    /// Marks the current call stack as being suspended.
    ///
    /// While suspended, the call stack can not be utilized for tracing messages. The call stack
    /// optionally also be marked as being blocked. In that case, the call stack must be unblocked
    /// prior to resumption.
    pub fn suspendCurrent(ctx: Tracing, mark_blocked: bool) void {
        ctx.context.vtable.tracing_v0.suspend_current_call_stack(
            ctx.context.data,
            mark_blocked,
        );
    }

    /// Marks the current call stack as being resumed.
    ///
    /// Once resumed, the context can be used to trace messages. To be successful, the current call
    /// stack must be suspended and unblocked.
    pub fn resumeCurrent(ctx: Tracing) void {
        ctx.context.vtable.tracing_v0.resume_current_call_stack(ctx.context.data);
    }
};

/// Type of a formatter function.
///
/// The formatter function is allowed to format only part of the message, if it would not fit into
/// the buffer.
pub const Formatter = fn (
    buffer: [*]u8,
    buffer_len: usize,
    data: ?*const anyopaque,
    written: *usize,
) callconv(.c) void;

/// Formatter of the zig standard library.
pub fn stdFormatter(comptime fmt: []const u8, ARGS: type) Formatter {
    return struct {
        fn format(
            buffer: [*]u8,
            buffer_len: usize,
            data: ?*const anyopaque,
            written: *usize,
        ) callconv(.c) void {
            const b = buffer[0..buffer_len];
            const args: *const ARGS = @alignCast(@ptrCast(data));
            if (std.fmt.bufPrint(b, fmt, args.*)) |out| {
                written.* = out.len;
            } else |_| written.* = buffer_len;
        }
    }.format;
}

/// Formatter for a zig stack trace.
pub fn stackTraceFormatter(
    buffer: [*]u8,
    buffer_len: usize,
    data: ?*const anyopaque,
    written: *usize,
) callconv(.c) void {
    const buf = buffer[0..buffer_len];
    const stack_trace: *const std.builtin.StackTrace = @alignCast(@ptrCast(data));
    if (builtin.strip_debug_info) {
        if (std.fmt.bufPrint(buf, "Unable to dump stack trace: debug info stripped", .{})) |out| {
            written.* = out.len;
        } else |_| written.* = buffer_len;
    }

    const debug_info = std.debug.getSelfDebugInfo() catch |err| {
        if (std.fmt.bufPrint(
            buf,
            "Unable to dump stack trace: Unable to open debug info: {s}",
            .{@errorName(err)},
        )) |out| {
            written.* = out.len;
        } else |_| written.* = buffer_len;
        return;
    };
    var stream = std.io.fixedBufferStream(buf);
    var writer = std.io.countingWriter(stream.writer());
    std.debug.writeStackTrace(
        stack_trace.*,
        writer.writer(),
        debug_info,
        .no_color,
    ) catch |err| switch (err) {
        error.NoSpaceLeft => {},
        else => {
            if (std.fmt.bufPrint(
                buf,
                "Unable to dump stack trace: {s}",
                .{@errorName(err)},
            )) |out| {
                written.* = out.len;
            } else |_| written.* = buffer_len;
        },
    };
    written.* = @intCast(writer.bytes_written);
}

/// A subscriber for tracing events.
///
/// The main function of the tracing subsystem is managing and routing tracing events to
/// subscribers. Therefore it does not consume any events on its own, which is the task of the
/// subscribers. Subscribers may utilize the events in any way they deem fit.
pub const Subscriber = extern struct {
    next: ?*const anyopaque = null,
    data: ?*anyopaque,
    vtable: *const Subscriber.VTable,

    /// VTable of a tracing subscriber.
    ///
    /// Adding/removing functionality to a subscriber through this table is a breaking change, as a
    /// subscriber may be implemented from outside the library.
    pub const VTable = extern struct {
        /// Destroys the subscriber.
        destroy: *const fn (ctx: ?*anyopaque) callconv(.c) void,
        /// Creates a new stack.
        call_stack_create: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
        ) callconv(.c) *anyopaque,
        /// Drops an empty call stack.
        ///
        /// Calling this function reverts the creation of the call stack.
        call_stack_drop: *const fn (
            ctx: ?*anyopaque,
            call_stack: *anyopaque,
        ) callconv(.c) void,
        /// Destroys a stack.
        call_stack_destroy: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
            call_stack: *anyopaque,
        ) callconv(.c) void,
        /// Marks the stack as unblocked.
        call_stack_unblock: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
            call_stack: *anyopaque,
        ) callconv(.c) void,
        /// Marks the stack as suspended/blocked.
        call_stack_suspend: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
            call_stack: *anyopaque,
            mark_blocked: bool,
        ) callconv(.c) void,
        /// Marks the stack as resumed.
        call_stack_resume: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
            call_stack: *anyopaque,
        ) callconv(.c) void,
        /// Creates a new span.
        span_push: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
            span_desc: *const SpanDesc,
            msg: [*]const u8,
            msg_len: usize,
            call_stack: *anyopaque,
        ) callconv(.c) void,
        /// Drops a newly created span.
        ///
        /// Calling this function reverts the creation of the span.
        span_drop: *const fn (ctx: ?*anyopaque, call_stack: *anyopaque) callconv(.c) void,
        /// Exits and destroys a span.
        span_pop: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
            call_stack: *anyopaque,
        ) callconv(.c) void,
        /// Emits an event.
        event_emit: *const fn (
            ctx: ?*anyopaque,
            time: *const c.FimoTime,
            call_stack: *anyopaque,
            event: *const Event,
            msg: [*]const u8,
            msg_len: usize,
        ) callconv(.c) void,
        /// Flushes the messages of the subscriber.
        flush: *const fn (ctx: ?*anyopaque) callconv(.c) void,
    };

    /// Initializes the subscriber interface from an existing object.
    ///
    /// The object must be kept alive for as long as the subscriber is still in use.
    pub fn init(
        comptime CallStackT: type,
        obj: anytype,
        comptime destroy_fn: fn (ctx: @TypeOf(obj)) void,
        comptime call_stack_create_fn: fn (ctx: @TypeOf(obj), time: Time) *CallStackT,
        comptime call_stack_drop_fn: fn (ctx: @TypeOf(obj), call_stack: *CallStackT) void,
        comptime call_stack_destroy_fn: fn (
            ctx: @TypeOf(obj),
            time: Time,
            call_stack: *CallStackT,
        ) void,
        comptime call_stack_unblock_fn: fn (
            ctx: @TypeOf(obj),
            time: Time,
            call_stack: *CallStackT,
        ) void,
        comptime call_stack_suspend_fn: fn (
            ctx: @TypeOf(obj),
            time: Time,
            call_stack: *CallStackT,
            mark_blocked: bool,
        ) void,
        comptime call_stack_resume_fn: fn (
            ctx: @TypeOf(obj),
            time: Time,
            call_stack: *CallStackT,
        ) void,
        comptime span_push_fn: fn (
            ctx: @TypeOf(obj),
            time: Time,
            span_desc: *const SpanDesc,
            msg: []const u8,
            call_stack: *CallStackT,
        ) void,
        comptime span_drop_fn: fn (ctx: @TypeOf(obj), call_stack: *CallStackT) void,
        comptime span_pop_fn: fn (ctx: @TypeOf(obj), time: Time, call_stack: *CallStackT) void,
        comptime event_emit_fn: fn (
            ctx: @TypeOf(obj),
            time: Time,
            call_stack: *CallStackT,
            event: *const Event,
            msg: []const u8,
        ) void,
        comptime flush_fn: fn (ctx: @TypeOf(obj)) void,
    ) Subscriber {
        const Ptr = @TypeOf(obj);
        std.debug.assert(@typeInfo(Ptr) == .pointer);
        // std.debug.assert(@typeInfo(Ptr).pointer.is_const == false);
        std.debug.assert(@typeInfo(Ptr).pointer.size == .One);
        std.debug.assert(@typeInfo(@typeInfo(Ptr).pointer.child) == .@"struct");

        const impl = struct {
            fn destroy(ptr: ?*anyopaque) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                destroy_fn(self);
            }
            fn callStackCreate(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
            ) callconv(.c) *anyopaque {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const t = Time.initC(time_c.*);
                return call_stack_create_fn(self, t);
            }
            fn callStackDrop(
                ptr: ?*anyopaque,
                call_stack: *anyopaque,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                call_stack_drop_fn(self, cs);
            }
            fn callStackDestroy(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
                call_stack: *anyopaque,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const t = Time.initC(time_c.*);
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                call_stack_destroy_fn(self, t, cs);
            }
            fn callStackUnblock(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
                call_stack: *anyopaque,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const t = Time.initC(time_c.*);
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                call_stack_unblock_fn(self, t, cs);
            }
            fn callStackSuspend(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
                call_stack: *anyopaque,
                mark_blocked: bool,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const t = Time.initC(time_c.*);
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                call_stack_suspend_fn(self, t, cs, mark_blocked);
            }
            fn callStackResume(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
                call_stack: *anyopaque,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const t = Time.initC(time_c.*);
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                call_stack_resume_fn(self, t, cs);
            }
            fn spanPush(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
                span_desc: *const SpanDesc,
                msg: [*]const u8,
                msg_len: usize,
                call_stack: *anyopaque,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const t = Time.initC(time_c.*);
                const m = msg[0..msg_len];
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                span_push_fn(
                    self,
                    t,
                    span_desc,
                    m,
                    cs,
                );
            }
            fn spanDrop(
                ptr: ?*anyopaque,
                call_stack: *anyopaque,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                span_drop_fn(self, cs);
            }
            fn spanPop(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
                call_stack: *anyopaque,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                const t = Time.initC(time_c.*);
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                span_pop_fn(self, t, cs);
            }
            fn eventEmit(
                ptr: ?*anyopaque,
                time_c: *const c.FimoTime,
                call_stack: *anyopaque,
                event: *const Event,
                msg: [*]const u8,
                msg_len: usize,
            ) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(ptr.?));
                const t = Time.initC(time_c.*);
                const cs: *CallStackT = @alignCast(@ptrCast(call_stack));
                const m = msg[0..msg_len];
                event_emit_fn(self, t, cs, event, m);
            }
            fn flush(ptr: ?*anyopaque) callconv(.c) void {
                const self: Ptr = @alignCast(@ptrCast(@constCast(ptr.?)));
                flush_fn(self);
            }
        };

        return Subscriber{
            .data = @constCast(obj),
            .vtable = &.{
                .destroy = impl.destroy,
                .call_stack_create = impl.callStackCreate,
                .call_stack_drop = impl.callStackDrop,
                .call_stack_destroy = impl.callStackDestroy,
                .call_stack_unblock = impl.callStackUnblock,
                .call_stack_suspend = impl.callStackSuspend,
                .call_stack_resume = impl.callStackResume,
                .span_push = impl.spanPush,
                .span_drop = impl.spanDrop,
                .span_pop = impl.spanPop,
                .event_emit = impl.eventEmit,
                .flush = impl.flush,
            },
        };
    }

    pub fn deinit(self: Subscriber) void {
        self.vtable.destroy(self.data);
    }

    pub fn createCallStack(
        self: Subscriber,
        timepoint: Time,
    ) *anyopaque {
        return self.vtable.call_stack_create(self.data, &timepoint.intoC());
    }

    pub fn dropCallStack(self: Subscriber, call_stack: *anyopaque) void {
        self.vtable.call_stack_drop(
            self.data,
            call_stack,
        );
    }

    pub fn destroyCallStack(self: Subscriber, timepoint: Time, call_stack: *anyopaque) void {
        self.vtable.call_stack_destroy(
            self.data,
            &timepoint.intoC(),
            call_stack,
        );
    }

    pub fn unblockCallStack(self: Subscriber, timepoint: Time, call_stack: *anyopaque) void {
        self.vtable.call_stack_unblock(
            self.data,
            &timepoint.intoC(),
            call_stack,
        );
    }

    pub fn suspendCallStack(
        self: Subscriber,
        timepoint: Time,
        call_stack: *anyopaque,
        mark_blocked: bool,
    ) void {
        self.vtable.call_stack_suspend(
            self.data,
            &timepoint.intoC(),
            call_stack,
            mark_blocked,
        );
    }

    pub fn resumeCallStack(self: Subscriber, timepoint: Time, call_stack: *anyopaque) void {
        self.vtable.call_stack_resume(self.data, &timepoint.intoC(), call_stack);
    }

    pub fn createSpan(
        self: Subscriber,
        timepoint: Time,
        span_desc: *const SpanDesc,
        message: []const u8,
        call_stack: *anyopaque,
    ) void {
        self.vtable.span_push(
            self.data,
            &timepoint.intoC(),
            @ptrCast(span_desc),
            message.ptr,
            message.len,
            call_stack,
        );
    }

    pub fn dropSpan(self: Subscriber, call_stack: *anyopaque) void {
        self.vtable.span_drop(self.data, call_stack);
    }

    pub fn destroySpan(self: Subscriber, timepoint: Time, call_stack: *anyopaque) void {
        self.vtable.span_pop(self.data, &timepoint.intoC(), call_stack);
    }

    pub fn emitEvent(
        self: Subscriber,
        timepoint: Time,
        call_stack: *anyopaque,
        event: *const Event,
        message: []const u8,
    ) void {
        self.vtable.event_emit(
            self.data,
            &timepoint.intoC(),
            call_stack,
            @ptrCast(event),
            message.ptr,
            message.len,
        );
    }

    pub fn flush(self: Subscriber) void {
        self.vtable.flush(self.data);
    }
};

/// Configuration for the tracing subsystem.
pub const Config = extern struct {
    id: Context.TypeId = .tracing_config,
    next: ?*const void = null,
    /// Length in characters of the per-call-stack buffer used when formatting mesasges.
    format_buffer_len: usize = 0,
    /// Maximum level for which to consume tracing events.
    max_level: Level = switch (builtin.mode) {
        .Debug => .debug,
        .ReleaseSafe => .info,
        .ReleaseFast, .ReleaseSmall => .err,
    },
    /// Array of subscribers to register with the tracing subsystem.
    ///
    /// The ownership of the subscribers is transferred to the context.
    subscribers: ?[*]const Subscriber = null,
    /// Number of subscribers to register with the tracing subsystem.
    subscriber_count: usize = 0,

    pub fn deinit(self: *const Config) void {
        if (self.subscribers) |subscribers| {
            for (subscribers[0..self.subscriber_count]) |s| s.deinit();
        }
    }
};

/// VTable of the tracing subsystem.
///
/// Changing the VTable is a breaking change.
pub const VTable = extern struct {
    create_call_stack: *const fn (ctx: *anyopaque) callconv(.c) CallStack,
    suspend_current_call_stack: *const fn (
        ctx: *anyopaque,
        mark_blocked: bool,
    ) callconv(.c) void,
    resume_current_call_stack: *const fn (ctx: *anyopaque) callconv(.c) void,
    span_create: *const fn (
        ctx: *anyopaque,
        span_desc: *const SpanDesc,
        formatter: *const Formatter,
        data: ?*const anyopaque,
    ) callconv(.c) Span,
    event_emit: *const fn (
        ctx: *anyopaque,
        event: *const Event,
        formatter: *const Formatter,
        data: ?*const anyopaque,
    ) callconv(.c) void,
    is_enabled: *const fn (ctx: *anyopaque) callconv(.c) bool,
    register_thread: *const fn (ctx: *anyopaque) callconv(.c) void,
    unregister_thread: *const fn (ctx: *anyopaque) callconv(.c) void,
    flush: *const fn (ctx: *anyopaque) callconv(.c) void,
};

/// Emits a new event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitEvent(
    self: Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    level: Level,
    location: std.builtin.SourceLocation,
    comptime fmt: []const u8,
    args: anytype,
) void {
    const event = Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = level,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    return self.emitEventCustom(
        &event,
        stdFormatter(fmt, @TypeOf(args)),
        &args,
    );
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitStackTrace(
    self: Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    stack_trace: std.builtin.StackTrace,
    location: std.builtin.SourceLocation,
) void {
    const event = Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = .err,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    self.emitEventCustom(
        &event,
        stackTraceFormatter,
        &stack_trace,
    );
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitStackTraceSimple(
    self: Tracing,
    stack_trace: std.builtin.StackTrace,
    location: std.builtin.SourceLocation,
) void {
    return self.emitStackTrace(
        null,
        null,
        stack_trace,
        location,
    );
}

/// Emits a new event with a custom formatter.
///
/// The subsystem may use a formatting buffer of a fixed size. The formatter is expected to cut-of
/// the message after reaching that specified size.
pub fn emitEventCustom(
    self: Tracing,
    event: *const Event,
    formatter: *const Formatter,
    data: ?*const anyopaque,
) void {
    return self.context.vtable.tracing_v0.event_emit(
        self.context.data,
        event,
        formatter,
        data,
    );
}

/// Emits a new error event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitErr(
    self: Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    comptime fmt: []const u8,
    args: anytype,
) void {
    return self.emitEvent(
        name,
        target,
        .err,
        location,
        fmt,
        args,
    );
}

/// Emits a new warn event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitWarn(
    self: Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    comptime fmt: []const u8,
    args: anytype,
) void {
    return self.emitEvent(
        name,
        target,
        .warn,
        location,
        fmt,
        args,
    );
}

/// Emits a new info event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitInfo(
    self: Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    comptime fmt: []const u8,
    args: anytype,
) void {
    return self.emitEvent(
        name,
        target,
        .info,
        location,
        fmt,
        args,
    );
}

/// Emits a new debug event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitDebug(
    self: Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    comptime fmt: []const u8,
    args: anytype,
) void {
    return self.emitEvent(
        name,
        target,
        .debug,
        location,
        fmt,
        args,
    );
}

/// Emits a new trace event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitTrace(
    self: Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    comptime fmt: []const u8,
    args: anytype,
) void {
    return self.emitEvent(
        name,
        target,
        .trace,
        location,
        fmt,
        args,
    );
}

/// Emits a new error event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitErrSimple(
    self: Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitErr(
        null,
        null,
        location,
        fmt,
        args,
    );
}

/// Emits a new warn event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitWarnSimple(
    self: Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitWarn(
        null,
        null,
        location,
        fmt,
        args,
    );
}

/// Emits a new info event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitInfoSimple(
    self: Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitInfo(
        null,
        null,
        location,
        fmt,
        args,
    );
}

/// Emits a new debug event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitDebugSimple(
    self: Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitDebug(
        null,
        null,
        location,
        fmt,
        args,
    );
}

/// Emits a new trace event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitTraceSimple(
    self: Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitTrace(
        null,
        null,
        location,
        fmt,
        args,
    );
}

/// Checks whether the tracing subsystem is enabled.
///
/// This function can be used to check whether to call into the subsystem at all. Calling this
/// function is not necessary, as the remaining functions of the subsystem are guaranteed to return
/// default values, in case the subsystem is disabled.
pub fn isEnabled(self: Tracing) bool {
    return self.context.vtable.tracing_v0.is_enabled(self.context.data);
}

/// Registers the calling thread with the tracing subsystem.
///
/// The tracing of the subsystem is opt-in on a per thread basis, where unregistered threads will
/// behave as if the subsystem was disabled. Once registered, the calling thread gains access to
/// the tracing subsystem and is assigned a new empty call stack. A registered thread must be
/// unregistered from the tracing subsystem before the context is destroyed, by terminating the
/// tread, or by manually calling `unregisterThread()`.
pub fn registerThread(self: Tracing) void {
    self.context.vtable.tracing_v0.register_thread(self.context.data);
}

/// Unregisters the calling thread from the tracing subsystem.
///
/// Once unregistered, the calling thread looses access to the tracing subsystem until it is
/// registered again. The thread can not be unregistered until the call stack is empty.
pub fn unregisterThread(self: Tracing) void {
    self.context.vtable.tracing_v0.unregister_thread(self.context.data);
}

/// Flushes the streams used for tracing.
///
/// If successful, any unwritten data is written out by the individual subscribers.
pub fn flush(self: Tracing) void {
    self.context.vtable.tracing_v0.flush(self.context.data);
}

const DefaultSubscriber = struct {
    const Self = @This();
    const Span = struct {
        previous: ?*Self.Span = null,
        next: ?*Self.Span = null,
        desc: *const SpanDesc,
        message: []const u8,
    };
    const CallStack = struct {
        tail: ?*Self.Span = null,
    };
    const allocator = @import("../../heap.zig").fimo_allocator;
    threadlocal var print_buffer = std.mem.zeroes([Self.print_buffer_len + overlength_correction.len:0]u8);
    const print_buffer_len = 1024;
    var mutex: std.Thread.Mutex = .{};

    const ansi_color_red: []const u8 = "\x1b[31m";
    const ansi_color_green: []const u8 = "\x1b[32m";
    const ansi_color_yellow: []const u8 = "\x1b[33m";
    const ansi_color_blue: []const u8 = "\x1b[34m";
    const ansi_color_magenta: []const u8 = "\x1b[35m";
    const ansi_color_reset: []const u8 = "\x1b[0m";

    const ansi_sgr_italic: []const u8 = "\x1b[3m";
    const ansi_sgr_reset: []const u8 = "\x1b[0m";

    const error_fmt: []const u8 = ansi_color_red ++ "ERROR {s}: {s}" ++ ansi_color_reset ++ "\n";
    const warn_fmt: []const u8 = ansi_color_yellow ++ "WARN {s}: {s}" ++ ansi_color_reset ++ "\n";
    const info_fmt: []const u8 = ansi_color_green ++ "INFO {s}: {s}" ++ ansi_color_reset ++ "\n";
    const debug_fmt: []const u8 = ansi_color_blue ++ "DEBUG {s}: {s}" ++ ansi_color_reset ++ "\n";
    const trace_fmt: []const u8 = ansi_color_magenta ++ "TRACE {s}: {s}" ++ ansi_color_reset ++ "\n";

    const file_path_fmt: []const u8 = "\t" ++ ansi_sgr_italic ++ "at" ++ ansi_sgr_reset ++ " {s}:{d}\n";
    const unknown_file_path_fmt: []const u8 = "\t" ++ ansi_sgr_italic ++ "at" ++ ansi_sgr_reset ++ " unknown\n";
    const backtrace_fmt: []const u8 = "\t" ++ ansi_sgr_italic ++ "in" ++ ansi_sgr_reset ++ " {s}" ++ ansi_sgr_italic ++ " with" ++ ansi_sgr_reset ++ " {s}\n";
    const overlength_correction: []const u8 = "\t..." ++ ansi_color_reset ++ "\n";

    fn deinit(self: *const Self) void {
        _ = self;
    }

    fn createCallStack(self: *const Self, time: Time) *Self.CallStack {
        _ = self;
        _ = time;
        const cs = Self.allocator.create(
            Self.CallStack,
        ) catch |err| @panic(@errorName(err));
        cs.tail = null;
        return cs;
    }

    fn dropCallStack(self: *const Self, call_stack: *Self.CallStack) void {
        _ = self;
        std.debug.assert(call_stack.tail == null);
        Self.allocator.destroy(call_stack);
    }

    fn destroyCallStack(self: *const Self, time: Time, call_stack: *Self.CallStack) void {
        _ = self;
        _ = time;
        std.debug.assert(call_stack.tail == null);
        Self.allocator.destroy(call_stack);
    }

    fn unblockCallStack(self: *const Self, time: Time, call_stack: *Self.CallStack) void {
        _ = self;
        _ = time;
        _ = call_stack;
    }

    fn suspendCallStack(self: *const Self, time: Time, call_stack: *Self.CallStack, mark_blocked: bool) void {
        _ = self;
        _ = time;
        _ = call_stack;
        _ = mark_blocked;
    }

    fn resumeCallStack(self: *const Self, time: Time, call_stack: *Self.CallStack) void {
        _ = self;
        _ = time;
        _ = call_stack;
    }

    fn createSpan(
        self: *const Self,
        time: Time,
        desc: *const SpanDesc,
        message: []const u8,
        call_stack: *Self.CallStack,
    ) void {
        _ = self;
        _ = time;
        const span = Self.allocator.create(
            Self.Span,
        ) catch |err| @panic(@errorName(err));
        span.* = Self.Span{
            .previous = call_stack.tail,
            .desc = desc,
            .message = message,
        };
        call_stack.tail = span;
    }

    fn dropSpan(self: *const Self, call_stack: *Self.CallStack) void {
        _ = self;
        const tail = call_stack.tail.?;
        const previous = tail.previous;
        Self.allocator.destroy(tail);
        call_stack.tail = previous;
    }

    fn destroySpan(self: *const Self, time: Time, call_stack: *Self.CallStack) void {
        _ = self;
        _ = time;
        const tail = call_stack.tail.?;
        const previous = tail.previous;
        Self.allocator.destroy(tail);
        call_stack.tail = previous;
    }

    fn emitEvent(
        self: *const Self,
        time: Time,
        call_stack: *Self.CallStack,
        event: *const Event,
        message: []const u8,
    ) void {
        _ = self;
        _ = time;

        const format = struct {
            fn f(cursor: usize, comptime fmt: []const u8, args: anytype) usize {
                const buffer = std.fmt.bufPrint(
                    print_buffer[cursor..print_buffer_len],
                    fmt,
                    args,
                ) catch return 0;
                return buffer.len;
            }
        }.f;

        // Write the event message.
        var cursor: usize = 0;
        cursor += switch (event.metadata.level) {
            .off => 0,
            .err => format(cursor, error_fmt, .{ event.metadata.name, message }),
            .warn => format(cursor, warn_fmt, .{ event.metadata.name, message }),
            .info => format(cursor, info_fmt, .{ event.metadata.name, message }),
            .debug => format(cursor, debug_fmt, .{ event.metadata.name, message }),
            .trace => format(cursor, trace_fmt, .{ event.metadata.name, message }),
        };

        // Write out the file location.
        if (event.metadata.file_name) |file_name| {
            cursor += format(cursor, file_path_fmt, .{
                file_name,
                event.metadata.line_number,
            });
        } else {
            cursor += format(cursor, unknown_file_path_fmt, .{});
        }

        // Write out the call stack.
        var span = call_stack.tail;
        while (span) |s| {
            cursor += format(cursor, backtrace_fmt, .{ s.desc.metadata.name, s.message });
            span = s.previous;
        }

        // Correct overlong messages.
        if (cursor >= print_buffer_len) {
            // Check if we have an incomplete ANSI escape sequence.
            // Our longest escape sequence consists of 5 bytes.
            for (0..5) |i| {
                if (print_buffer[cursor - i - 1] == 'm') break;
                if (print_buffer[cursor - i - 1] == '\x1b') {
                    cursor = cursor - i - 1;
                    break;
                }
            }

            const rest_buffer = print_buffer[cursor..];
            const correction_start: usize = if (print_buffer[cursor - 1] == 'n') 0 else 1;
            std.mem.copyForwards(u8, rest_buffer, overlength_correction[correction_start..]);
        }

        Self.mutex.lock();
        defer Self.mutex.unlock();

        const writer = std.io.getStdErr().writer();
        writer.print("{s}", .{Self.print_buffer[0..cursor]}) catch {};
    }

    fn flush(self: *const Self) void {
        _ = self;
        // zig Files are not buffered.
    }
};

pub const default_subscriber = Subscriber.init(
    DefaultSubscriber.CallStack,
    &DefaultSubscriber{},
    DefaultSubscriber.deinit,
    DefaultSubscriber.createCallStack,
    DefaultSubscriber.dropCallStack,
    DefaultSubscriber.destroyCallStack,
    DefaultSubscriber.unblockCallStack,
    DefaultSubscriber.suspendCallStack,
    DefaultSubscriber.resumeCallStack,
    DefaultSubscriber.createSpan,
    DefaultSubscriber.dropSpan,
    DefaultSubscriber.destroySpan,
    DefaultSubscriber.emitEvent,
    DefaultSubscriber.flush,
);

comptime {
    @export(&default_subscriber, .{ .name = "FIMO_TRACING_DEFAULT_SUBSCRIBER" });
}
