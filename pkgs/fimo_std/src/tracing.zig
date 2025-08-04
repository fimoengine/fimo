//! Tracing and structured logging subsystem of the context.
//!
//! Requires an initialized global context.
const std = @import("std");
const builtin = @import("builtin");

const ctx = @import("ctx.zig");
const time = @import("time.zig");

/// Tracing levels.
pub const Level = enum(i32) {
    off,
    err,
    warn,
    info,
    debug,
    trace,
};

/// Basic information regarding a tracing event.
///
/// The subsystem expects instances of this struct to have a static lifetime.
pub const EventInfo = extern struct {
    name: [*:0]const u8,
    target: [*:0]const u8,
    scope: [*:0]const u8,
    file_name: ?[*:0]const u8 = null,
    line_number: i32 = -1,
    level: Level,

    /// Constructs a new info instance.
    pub fn at(
        comptime loc: std.builtin.SourceLocation,
        comptime scope: @Type(.enum_literal),
        comptime lvl: Level,
    ) EventInfo {
        return .{
            .name = loc.fn_name,
            .target = loc.module,
            .scope = if (scope == .default) "" else @tagName(scope),
            .file_name = loc.file,
            .line_number = @intCast(loc.line),
            .level = lvl,
        };
    }
};

/// A period of time, during which events can occur.
pub const Span = struct {
    id: *const EventInfo,

    /// Constructs a new span.
    ///
    /// The event associated with the span is embedded into the callers binary, and is not emitted
    /// to the subsystem.
    pub inline fn at(
        comptime loc: std.builtin.SourceLocation,
        comptime scope: @Type(.enum_literal),
        comptime lvl: Level,
    ) Span {
        const Global = struct {
            const embedded: EventInfo = .at(loc, scope, lvl);
        };
        return .{ .id = &Global.embedded };
    }

    /// Enters the span.
    ///
    /// Once entered, the span is used as the context for succeeding events. Each `enter` operation
    /// must be accompanied with a `exit` operation in reverse entering order. A span may be entered
    /// multiple times. The formatting function may be used to assign a name to the entered span.
    pub fn enter(
        self: Span,
        formatter: *const Formatter,
        formatter_data: *const anyopaque,
    ) void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.enter_span(self.id, formatter, formatter_data);
    }

    /// Exits an entered span.
    ///
    /// The events won't occur inside the context of the exited span anymore. The span must be the
    /// span at the top of the current call stack.
    pub fn exit(self: Span) void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.exit_span(self.id);
    }
};

/// A call stack.
///
/// Each call stack represents a unit of computation, like a thread. A call stack is active on only
/// one thread at any given time. The active call stack of a thread can be swapped, which is useful
/// for tracing where a `M:N` threading model is used. In that case, one would create one stack for
/// each task, and activate it when the task is resumed.
pub const CallStack = opaque {
    /// Creates a new empty call stack.
    ///
    /// The call stack is marked as suspended.
    pub fn init() *CallStack {
        const handle = ctx.Handle.getHandle();
        return handle.tracing_v0.create_call_stack();
    }

    /// Destroys an empty call stack.
    ///
    /// Marks the completion of a task. Before calling this function, the call stack must be empty,
    /// i.e., there must be no active spans on the stack, and must not be active. The call stack may
    /// not be used afterwards. The active call stack of the thread is destroyed automatically, on
    /// thread exit or during destruction of the context.
    pub fn finish(self: *CallStack) void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.destroy_call_stack(self, false);
    }

    /// Unwinds and destroys the call stack.
    ///
    /// Marks that the task was aborted. Before calling this function, the call stack must not be
    /// active. The call stack may not be used afterwards.
    pub fn abort(self: *CallStack) void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.destroy_call_stack(self, true);
    }

    /// Switches the call stack of the current thread.
    ///
    /// This call stack will be used as the active call stack of the calling thread. The old call
    /// stack is returned, enabling the caller to switch back to it afterwards. This call stack
    /// must be in a suspended, but unblocked, state and not be active. The active call stack must
    /// also be in a suspended state, but may also be blocked.
    pub fn swapCurrent(self: *CallStack) *CallStack {
        const handle = ctx.Handle.getHandle();
        return handle.tracing_v0.swap_call_stack(self);
    }

    /// Unblocks the blocked call stack.
    ///
    /// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
    /// marked as blocked.
    pub fn unblock(self: *CallStack) void {
        const handle = ctx.Handle.getHandle();
        return handle.tracing_v0.unblock_call_stack(self);
    }

    /// Marks the current call stack as being suspended.
    ///
    /// While suspended, the call stack can not be utilized for tracing messages. The call stack
    /// optionally also be marked as being blocked. In that case, the call stack must be unblocked
    /// prior to resumption.
    pub fn suspendCurrent(mark_blocked: bool) void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.suspend_current_call_stack(mark_blocked);
    }

    /// Marks the current call stack as being resumed.
    ///
    /// Once resumed, the context can be used to trace messages. To be successful, the current call
    /// stack must be suspended and unblocked.
    pub fn resumeCurrent() void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.resume_current_call_stack();
    }
};

/// Returns a scoped tracing namespace that emits all events using the scope provided here.
pub fn scoped(scope: @Type(.enum_literal)) type {
    return struct {
        /// Logs a error message.
        pub inline fn logErr(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) void {
            @This().log(loc, .err, fmt, args);
        }

        /// Logs a warning message.
        pub inline fn logWarn(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) void {
            @This().log(loc, .warn, fmt, args);
        }

        /// Logs an info message.
        pub inline fn logInfo(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) void {
            @This().log(loc, .info, fmt, args);
        }

        /// Logs a debug message.
        pub inline fn logDebug(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) void {
            @This().log(loc, .debug, fmt, args);
        }

        /// Logs a trace message.
        pub inline fn logTrace(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) void {
            @This().log(loc, .trace, fmt, args);
        }

        /// Logs the current stack trace as an error.
        pub inline fn logStackTrace(
            comptime loc: std.builtin.SourceLocation,
            stack_trace: std.builtin.StackTrace,
        ) void {
            @This().logWithFormatter(loc, .err, stackTraceFormatter, &stack_trace);
        }

        /// Logs a message using to the zig formatting logic.
        pub inline fn log(
            comptime loc: std.builtin.SourceLocation,
            comptime lvl: Level,
            comptime fmt: []const u8,
            args: anytype,
        ) void {
            @This().logWithFormatter(loc, lvl, stdFormatter(fmt, @TypeOf(args)), &args);
        }

        /// Logs a message with a custom format function.
        pub inline fn logWithFormatter(
            comptime loc: std.builtin.SourceLocation,
            comptime lvl: Level,
            formatter: *const Formatter,
            formatter_data: *const anyopaque,
        ) void {
            const Global = struct {
                const embedded: EventInfo = .at(loc, scope, lvl);
            };
            const handle = ctx.Handle.getHandle();
            handle.tracing_v0.log_message(&Global.embedded, formatter, formatter_data);
        }

        /// Creates and enters an error span.
        pub inline fn spanErr(comptime loc: std.builtin.SourceLocation) Span {
            return @This().span(loc, .err);
        }

        /// Creates and enters an error span.
        pub inline fn spanErrNamed(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) Span {
            return @This().spanNamed(loc, .err, fmt, args);
        }

        /// Creates and enters a warning span.
        pub inline fn spanWarn(comptime loc: std.builtin.SourceLocation) Span {
            return @This().span(loc, .warn);
        }

        /// Creates and enters a warning span.
        pub inline fn spanWarnNamed(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) Span {
            return @This().spanNamed(loc, .warn, fmt, args);
        }

        /// Creates and enters an info span.
        pub inline fn spanInfo(comptime loc: std.builtin.SourceLocation) Span {
            return @This().span(loc, .info);
        }

        /// Creates and enters an info span.
        pub inline fn spanInfoNamed(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) Span {
            return @This().spanNamed(loc, .info, fmt, args);
        }

        /// Creates and enters a debug span.
        pub inline fn spanDebug(comptime loc: std.builtin.SourceLocation) Span {
            return @This().span(loc, .debug);
        }

        /// Creates and enters a debug span.
        pub inline fn spanDebugNamed(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) Span {
            return @This().spanNamed(loc, .debug, fmt, args);
        }

        /// Creates and enters a trace span.
        pub inline fn spanTrace(comptime loc: std.builtin.SourceLocation) Span {
            return @This().span(loc, .trace);
        }

        /// Creates and enters a trace span.
        pub inline fn spanTraceNamed(
            comptime loc: std.builtin.SourceLocation,
            comptime fmt: []const u8,
            args: anytype,
        ) Span {
            return @This().spanNamed(loc, .trace, fmt, args);
        }

        /// Creates and enters a span.
        pub inline fn span(
            comptime loc: std.builtin.SourceLocation,
            comptime lvl: Level,
        ) Span {
            return @This().spanNamed(loc, lvl, "", .{});
        }

        /// Creates and enters a span using to the zig formatting logic.
        pub inline fn spanNamed(
            comptime loc: std.builtin.SourceLocation,
            comptime lvl: Level,
            comptime fmt: []const u8,
            args: anytype,
        ) Span {
            return @This().spanNamedWithFormatter(loc, lvl, stdFormatter(fmt, @TypeOf(args)), &args);
        }

        /// Creates and enters a span with a custom format function.
        pub inline fn spanNamedWithFormatter(
            comptime loc: std.builtin.SourceLocation,
            comptime lvl: Level,
            formatter: *const Formatter,
            formatter_data: *const anyopaque,
        ) Span {
            const sp: Span = .at(loc, scope, lvl);
            sp.enter(formatter, formatter_data);
            return sp;
        }
    };
}

/// The default scoped tracing namespace.
pub const default = scoped(default_trace_scope);
pub const default_trace_scope = .default;

pub const logErr = default.logErr;
pub const logWarn = default.logWarn;
pub const logInfo = default.logInfo;
pub const logDebug = default.logDebug;
pub const logTrace = default.logTrace;
pub const logStackTrace = default.logStackTrace;
pub const log = default.log;
pub const logWithFormatter = default.logWithFormatter;
pub const spanErr = default.spanErr;
pub const spanErrNamed = default.spanErrNamed;
pub const spanWarn = default.spanWarn;
pub const spanWarnNamed = default.spanWarnNamed;
pub const spanInfo = default.spanInfo;
pub const spanInfoNamed = default.spanInfoNamed;
pub const spanDebug = default.spanDebug;
pub const spanDebugNamed = default.spanDebugNamed;
pub const spanTrace = default.spanTrace;
pub const spanTraceNamed = default.spanTraceNamed;
pub const span = default.span;
pub const spanNamed = default.spanNamed;
pub const spanNamedWithFormatter = default.spanNamedWithFormatter;

/// Subscriber events.
pub const events = struct {
    /// Common header of all events.
    pub const Event = enum(u32) {
        register_thread,
        unregister_thread,
        create_call_stack,
        destroy_call_stack,
        unblock_call_stack,
        suspend_call_stack,
        resume_call_stack,
        enter_span,
        exit_span,
        log_message,
        _,
    };

    pub const RegisterThread = extern struct {
        event: Event = .register_thread,
        time: time.compat.Instant,
    };
    pub const UnregisterThread = extern struct {
        event: Event = .unregister_thread,
        time: time.compat.Instant,
    };
    pub const CreateCallStack = extern struct {
        event: Event = .create_call_stack,
        time: time.compat.Instant,
    };
    pub const DestroyCallStack = extern struct {
        event: Event = .destroy_call_stack,
        stack: *anyopaque,
        time: time.compat.Instant,
    };
    pub const UnblockCallStack = extern struct {
        event: Event = .unblock_call_stack,
        stack: *anyopaque,
        time: time.compat.Instant,
    };
    pub const SuspendCallStack = extern struct {
        event: Event = .suspend_call_stack,
        stack: *anyopaque,
        time: time.compat.Instant,
        mark_blocked: bool,
    };
    pub const ResumeCallStack = extern struct {
        event: Event = .resume_call_stack,
        stack: *anyopaque,
        time: time.compat.Instant,
    };
    pub const EnterSpan = extern struct {
        event: Event = .enter_span,
        stack: *anyopaque,
        time: time.compat.Instant,
        span: *const EventInfo,
        message: [*]const u8,
        message_length: usize,
    };
    pub const ExitSpan = extern struct {
        event: Event = .exit_span,
        stack: *anyopaque,
        time: time.compat.Instant,
        span: *const EventInfo,
        is_unwinding: bool,
    };
    pub const LogMessage = extern struct {
        event: Event = .log_message,
        stack: *anyopaque,
        time: time.compat.Instant,
        info: *const EventInfo,
        message: [*]const u8,
        message_length: usize,
    };
};

/// A subscriber for tracing events.
///
/// The main function of the tracing subsystem is managing and routing tracing events to
/// subscribers. Therefore, it does not consume any events on its own, which is the task of the
/// subscribers. Subscribers may utilize the events in any way they deem fit.
pub const Subscriber = extern struct {
    data: *anyopaque,
    on_event: *const fn (data: *anyopaque, event: *const events.Event) callconv(.c) *anyopaque,

    pub fn of(T: type, value: *T) Subscriber {
        if (!@hasDecl(T, "fimo_subscriber")) @compileError("fimo: invalid module, missing `pub const fimo_subscriber = .{...};` declaration: " ++ @typeName(T));
        const info = T.fimo_subscriber;
        const Info = @TypeOf(info);

        const wrapper = struct {
            fn on_event(data: *anyopaque, event: *const events.Event) callconv(.c) *anyopaque {
                const self: *T = @ptrCast(@alignCast(data));
                switch (event.*) {
                    .register_thread => if (comptime @hasField(Info, "register_thread")) {
                        const ev: *const events.RegisterThread = @alignCast(@fieldParentPtr("event", event));
                        info.register_thread(self, ev);
                        return @constCast(&{});
                    },
                    .unregister_thread => if (comptime @hasField(Info, "unregister_thread")) {
                        const ev: *const events.UnregisterThread = @alignCast(@fieldParentPtr("event", event));
                        info.unregister_thread(self, ev);
                        return @constCast(&{});
                    },
                    .create_call_stack => if (comptime @hasField(Info, "create_call_stack")) {
                        const ev: *const events.CreateCallStack = @alignCast(@fieldParentPtr("event", event));
                        return info.create_call_stack(self, ev);
                    },
                    .destroy_call_stack => if (comptime @hasField(Info, "destroy_call_stack")) {
                        const ev: *const events.DestroyCallStack = @alignCast(@fieldParentPtr("event", event));
                        info.destroy_call_stack(self, ev);
                        return @constCast(&{});
                    },
                    .unblock_call_stack => if (comptime @hasField(Info, "unblock_call_stack")) {
                        const ev: *const events.UnblockCallStack = @alignCast(@fieldParentPtr("event", event));
                        info.unblock_call_stack(self, ev);
                        return @constCast(&{});
                    },
                    .suspend_call_stack => if (comptime @hasField(Info, "suspend_call_stack")) {
                        const ev: *const events.SuspendCallStack = @alignCast(@fieldParentPtr("event", event));
                        info.suspend_call_stack(self, ev);
                        return @constCast(&{});
                    },
                    .resume_call_stack => if (comptime @hasField(Info, "resume_call_stack")) {
                        const ev: *const events.ResumeCallStack = @alignCast(@fieldParentPtr("event", event));
                        info.resume_call_stack(self, ev);
                        return @constCast(&{});
                    },
                    .enter_span => if (comptime @hasField(Info, "enter_span")) {
                        const ev: *const events.EnterSpan = @alignCast(@fieldParentPtr("event", event));
                        info.enter_span(self, ev);
                        return @constCast(&{});
                    },
                    .exit_span => if (comptime @hasField(Info, "exit_span")) {
                        const ev: *const events.ExitSpan = @alignCast(@fieldParentPtr("event", event));
                        info.exit_span(self, ev);
                        return @constCast(&{});
                    },
                    .log_message => if (comptime @hasField(Info, "log_message")) {
                        const ev: *const events.LogMessage = @alignCast(@fieldParentPtr("event", event));
                        info.log_message(self, ev);
                        return @constCast(&{});
                    },
                    else => return @constCast(&{}),
                }
                return @constCast(&{});
            }
        };
        return .{ .data = value, .on_event = &wrapper.on_event };
    }

    pub fn registerThread(self: Subscriber, event: events.RegisterThread) void {
        std.debug.assert(event.event == .register_thread);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn unregisterThread(self: Subscriber, event: events.UnregisterThread) void {
        std.debug.assert(event.event == .unregister_thread);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn createCallStack(self: Subscriber, event: events.CreateCallStack) *anyopaque {
        std.debug.assert(event.event == .create_call_stack);
        return self.on_event(self.data, &event.event);
    }

    pub fn dropCallStack(self: Subscriber, event: events.DropCallStack) void {
        std.debug.assert(event.event == .drop_call_stack);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn destroyCallStack(self: Subscriber, event: events.DestroyCallStack) void {
        std.debug.assert(event.event == .destroy_call_stack);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn unblockCallStack(self: Subscriber, event: events.UnblockCallStack) void {
        std.debug.assert(event.event == .unblock_call_stack);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn suspendCallStack(self: Subscriber, event: events.SuspendCallStack) void {
        std.debug.assert(event.event == .suspend_call_stack);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn resumeCallStack(self: Subscriber, event: events.ResumeCallStack) void {
        std.debug.assert(event.event == .resume_call_stack);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn enterSpan(self: Subscriber, event: events.EnterSpan) void {
        std.debug.assert(event.event == .enter_span);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn exitSpan(self: Subscriber, event: events.ExitSpan) void {
        std.debug.assert(event.event == .exit_span);
        _ = self.on_event(self.data, &event.event);
    }

    pub fn logMessage(self: Subscriber, event: events.LogMessage) void {
        std.debug.assert(event.event == .log_message);
        _ = self.on_event(self.data, &event.event);
    }
};

/// Type of a formatter function.
///
/// The formatter function is allowed to format only part of the message, if it would not fit into
/// the buffer.
pub const Formatter = fn (
    buffer: [*]u8,
    buffer_len: usize,
    data: *const anyopaque,
) callconv(.c) usize;

/// Formatter of the zig standard library.
pub fn stdFormatter(comptime fmt: []const u8, ARGS: type) Formatter {
    return struct {
        fn format(
            buffer: [*]u8,
            buffer_len: usize,
            data: *const anyopaque,
        ) callconv(.c) usize {
            const b = buffer[0..buffer_len];
            const args: *const ARGS = @alignCast(@ptrCast(data));
            return if (std.fmt.bufPrint(b, fmt, args.*)) |out|
                out.len
            else |_|
                buffer_len;
        }
    }.format;
}

/// Formatter for a zig stack trace.
pub fn stackTraceFormatter(
    buffer: [*]u8,
    buffer_len: usize,
    data: *const anyopaque,
) callconv(.c) usize {
    const buf = buffer[0..buffer_len];
    const stack_trace: *const std.builtin.StackTrace = @alignCast(@ptrCast(data));
    if (builtin.strip_debug_info) return if (std.fmt.bufPrint(buf, "Unable to dump stack trace: debug info stripped", .{})) |out|
        out.len
    else |_|
        buffer_len;

    const debug_info = std.debug.getSelfDebugInfo() catch |err| return if (std.fmt.bufPrint(
        buf,
        "Unable to dump stack trace: Unable to open debug info: {s}",
        .{@errorName(err)},
    )) |out|
        out.len
    else |_|
        buffer_len;
    var writer: std.Io.Writer = .fixed(buf);
    std.debug.writeStackTrace(stack_trace.*, &writer, debug_info, .no_color) catch |err| switch (err) {
        error.WriteFailed => {},
        else => return if (std.fmt.bufPrint(buf, "Unable to dump stack trace: {s}", .{@errorName(err)})) |out|
            out.len
        else |_|
            buffer_len,
    };
    return writer.buffered().len;
}

/// Configuration for the tracing subsystem.
pub const Config = extern struct {
    id: ctx.ConfigId = .tracing,
    /// Length in characters of the per-call-stack buffer used when formatting mesasges.
    format_buffer_len: usize = 0,
    /// Maximum level for which to consume tracing events.
    max_level: Level = switch (builtin.mode) {
        .Debug => .debug,
        .ReleaseSafe => .info,
        .ReleaseFast, .ReleaseSmall => .err,
    },
    /// Array of subscribers to register with the tracing subsystem.
    subscribers: ?[*]const Subscriber = null,
    /// Number of subscribers to register with the tracing subsystem.
    subscriber_count: usize = 0,
};

/// Base VTable of the tracing subsystem.
///
/// Changing this definition is a breaking change.
pub const VTable = extern struct {
    is_enabled: *const fn () callconv(.c) bool,
    register_thread: *const fn () callconv(.c) void,
    unregister_thread: *const fn () callconv(.c) void,
    create_call_stack: *const fn () callconv(.c) *CallStack,
    destroy_call_stack: *const fn (stack: *CallStack, abort: bool) callconv(.c) void,
    swap_call_stack: *const fn (stack: *CallStack) callconv(.c) *CallStack,
    unblock_call_stack: *const fn (stack: *CallStack) callconv(.c) void,
    suspend_current_call_stack: *const fn (mark_blocked: bool) callconv(.c) void,
    resume_current_call_stack: *const fn () callconv(.c) void,
    enter_span: *const fn (
        id: *const EventInfo,
        formatter: *const Formatter,
        formatter_data: *const anyopaque,
    ) callconv(.c) void,
    exit_span: *const fn (id: *const EventInfo) callconv(.c) void,
    log_message: *const fn (
        info: *const EventInfo,
        formatter: *const Formatter,
        formatter_data: *const anyopaque,
    ) callconv(.c) void,
};

/// Checks whether the tracing subsystem is enabled.
///
/// This function can be used to check whether to call into the subsystem at all. Calling this
/// function is not necessary, as the remaining functions of the subsystem are guaranteed to return
/// default values, in case the subsystem is disabled.
pub fn isEnabled() bool {
    const handle = ctx.Handle.getHandle();
    return handle.tracing_v0.is_enabled();
}

/// Registers the calling thread with the tracing subsystem.
///
/// The instrumentation is opt-in on a per thread basis, where unregistered threads will
/// behave as if the subsystem was disabled. Once registered, the calling thread gains access to
/// the tracing subsystem and is assigned a new empty call stack. A registered thread must be
/// unregistered from the tracing subsystem before the context is destroyed, by terminating the
/// tread, or by manually unregistering it. A registered thread may not try to register itself.
pub fn registerThread() void {
    const handle = ctx.Handle.getHandle();
    handle.tracing_v0.register_thread();
}

/// Unregisters the calling thread from the tracing subsystem.
///
/// Once unregistered, the calling thread looses access to the tracing subsystem until it is
/// registered again. The thread can not be unregistered until the call stack is empty.
pub fn unregisterThread() void {
    const handle = ctx.Handle.getHandle();
    handle.tracing_v0.unregister_thread();
}

const LoggingSubscriber = struct {
    const Span = struct {
        id: *const EventInfo,
        message: []const u8,
    };
    const Stack = struct {
        arena: std.heap.ArenaAllocator,
        spans: std.ArrayListUnmanaged(LoggingSubscriber.Span) = .empty,
    };

    pub const fimo_subscriber = .{
        .create_call_stack = createCallStack,
        .destroy_call_stack = destroyCallStack,
        .enter_span = enterSpan,
        .exit_span = exitSpan,
        .log_message = logMessage,
    };

    const allocator = std.heap.c_allocator;
    threadlocal var print_buffer = std.mem.zeroes([print_buffer_len + overlength_correction.len:0]u8);
    const print_buffer_len = 1024;

    var config: std.Io.tty.Config = undefined;
    var init_config = std.once(detectConfig);

    fn detectConfig() void {
        config = .detect(std.fs.File.stderr());
    }

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

    fn createCallStack(self: *LoggingSubscriber, event: *const events.CreateCallStack) *Stack {
        _ = self;
        _ = event;
        const stack = allocator.create(Stack) catch |err| @panic(@errorName(err));
        stack.* = .{ .arena = .init(allocator) };
        return stack;
    }

    fn destroyCallStack(self: *LoggingSubscriber, event: *const events.DestroyCallStack) void {
        _ = self;
        const stack: *Stack = @ptrCast(@alignCast(event.stack));
        std.debug.assert(stack.spans.items.len == 0);
        stack.arena.deinit();
        allocator.destroy(stack);
    }

    fn enterSpan(self: *LoggingSubscriber, event: *const events.EnterSpan) void {
        _ = self;
        const stack: *Stack = @ptrCast(@alignCast(event.stack));
        const message = stack.arena.allocator().dupe(
            u8,
            event.message[0..event.message_length],
        ) catch |err| @panic(@errorName(err));
        errdefer stack.arena.allocator().free(message);
        stack.spans.append(
            allocator,
            .{ .id = event.span, .message = message },
        ) catch |err| @panic(@errorName(err));
    }

    fn exitSpan(self: *LoggingSubscriber, event: *const events.ExitSpan) void {
        _ = self;
        const stack: *Stack = @ptrCast(@alignCast(event.stack));
        std.debug.assert(stack.spans.getLast().id == event.span);
        const sp = stack.spans.pop() orelse unreachable;
        const message = sp.message;
        stack.arena.allocator().free(message);
    }

    fn logMessage(self: *LoggingSubscriber, event: *const events.LogMessage) void {
        _ = self;
        init_config.call();

        const stack: *Stack = @ptrCast(@alignCast(event.stack));
        const message = event.message[0..event.message_length];
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
        cursor += switch (event.info.level) {
            .off => 0,
            .err => format(cursor, error_fmt, .{ event.info.name, message }),
            .warn => format(cursor, warn_fmt, .{ event.info.name, message }),
            .info => format(cursor, info_fmt, .{ event.info.name, message }),
            .debug => format(cursor, debug_fmt, .{ event.info.name, message }),
            .trace => format(cursor, trace_fmt, .{ event.info.name, message }),
        };

        // Write out the file location.
        if (event.info.file_name) |file_name| {
            cursor += format(cursor, file_path_fmt, .{
                file_name,
                event.info.line_number,
            });
        } else {
            cursor += format(cursor, unknown_file_path_fmt, .{});
        }

        // Write out the call stack.
        {
            var i: usize = stack.spans.items.len;
            while (i > 0) {
                i -= 1;
                const sp = stack.spans.items[i];
                cursor += format(cursor, backtrace_fmt, .{ sp.id.name, sp.message });
            }
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

        var buffer: [64]u8 = undefined;
        const stderr = std.debug.lockStderrWriter(&buffer);
        defer std.debug.unlockStderrWriter();
        stderr.writeAll(print_buffer[0..cursor]) catch {};
    }
};

pub const default_subscriber = Subscriber.of(LoggingSubscriber, @constCast(&LoggingSubscriber{}));
comptime {
    @export(&default_subscriber, .{ .name = "FIMO_TRACING_DEFAULT_SUBSCRIBER" });
}
