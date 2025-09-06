//! Tracing and structured logging subsystem of the context.
//!
//! Requires an initialized global context.
const std = @import("std");
const builtin = @import("builtin");

const ctx = @import("ctx.zig");
const paths = @import("paths.zig");
const time = @import("time.zig");
pub const db = @import("tracing/db.zig");
pub const net = @import("tracing/net.zig");
pub const StdErrLogger = @import("tracing/StdErrLogger.zig");
const utils = @import("utils.zig");
const SliceConst = utils.SliceConst;

comptime {
    _ = net;
    _ = StdErrLogger;
    if (builtin.is_test) {
        _ = db;
    }
}

/// Tracing levels.
pub const Level = enum(i32) {
    off = 0,
    err = 1,
    warn = 2,
    info = 3,
    debug = 4,
    trace = 5,
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
        return handle.tracing_v0.init_call_stack();
    }

    /// Destroys an empty call stack.
    ///
    /// Marks the completion of a task. Before calling this function, the call stack must be empty,
    /// i.e., there must be no active spans on the stack, and must not be active. The call stack may
    /// not be used afterwards. The active call stack of the thread is destroyed automatically, on
    /// thread exit or during destruction of the context.
    pub fn finish(self: *CallStack) void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.deinit_call_stack(self, false);
    }

    /// Unwinds and destroys the call stack.
    ///
    /// Marks that the task was aborted. Before calling this function, the call stack must not be
    /// active. The call stack may not be used afterwards.
    pub fn abort(self: *CallStack) void {
        const handle = ctx.Handle.getHandle();
        handle.tracing_v0.deinit_call_stack(self, true);
    }

    /// Replaces the call stack of the current thread.
    ///
    /// This call stack will be used as the active call stack of the calling thread. The old call
    /// stack is returned, enabling the caller to switch back to it afterwards. This call stack
    /// must be in a suspended, but unblocked, state and not be active. The active call stack must
    /// also be in a suspended state, but may also be blocked.
    pub fn replaceCurrent(self: *CallStack) *CallStack {
        const handle = ctx.Handle.getHandle();
        return handle.tracing_v0.replace_current_call_stack(self);
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
    /// Common member of all tracing events.
    pub const EventTag = enum(i32) {
        start = 0,
        finish = 1,
        register_thread = 2,
        unregister_thread = 3,
        create_call_stack = 4,
        destroy_call_stack = 5,
        unblock_call_stack = 6,
        suspend_call_stack = 7,
        resume_call_stack = 8,
        enter_span = 9,
        exit_span = 10,
        log_message = 11,
        declare_event_info = 12,
        start_thread = 13,
        stop_thread = 14,
        load_image = 15,
        unload_image = 16,
        context_switch = 17,
        thread_wakeup = 18,
        call_stack_sample = 19,
        _,
    };

    pub const CpuArch = enum(u8) {
        unknown = 0,
        x86_64 = 1,
        aarch64 = 2,
        _,
    };

    pub const Start = extern struct {
        tag: EventTag = .start,
        time: time.compat.Instant,
        epoch: time.compat.Time,
        resolution: time.compat.Duration,
        available_memory: usize,
        process_id: usize,
        num_cores: usize,
        cpu_arch: CpuArch,
        cpu_id: u32,
        cpu_vendor: SliceConst(u8),
        app_name: SliceConst(u8),
        host_info: SliceConst(u8),
    };
    pub const Finish = extern struct {
        tag: EventTag = .finish,
        time: time.compat.Instant,
    };
    pub const RegisterThread = extern struct {
        tag: EventTag = .register_thread,
        time: time.compat.Instant,
        thread_id: usize,
    };
    pub const UnregisterThread = extern struct {
        tag: EventTag = .unregister_thread,
        time: time.compat.Instant,
        thread_id: usize,
    };
    pub const CreateCallStack = extern struct {
        tag: EventTag = .create_call_stack,
        time: time.compat.Instant,
        stack: *anyopaque,
    };
    pub const DestroyCallStack = extern struct {
        tag: EventTag = .destroy_call_stack,
        time: time.compat.Instant,
        stack: *anyopaque,
    };
    pub const UnblockCallStack = extern struct {
        tag: EventTag = .unblock_call_stack,
        time: time.compat.Instant,
        stack: *anyopaque,
    };
    pub const SuspendCallStack = extern struct {
        tag: EventTag = .suspend_call_stack,
        time: time.compat.Instant,
        stack: *anyopaque,
        mark_blocked: bool,
    };
    pub const ResumeCallStack = extern struct {
        tag: EventTag = .resume_call_stack,
        time: time.compat.Instant,
        stack: *anyopaque,
        thread_id: usize,
    };
    pub const EnterSpan = extern struct {
        tag: EventTag = .enter_span,
        time: time.compat.Instant,
        stack: *anyopaque,
        span: *const EventInfo,
        message: SliceConst(u8),
    };
    pub const ExitSpan = extern struct {
        tag: EventTag = .exit_span,
        time: time.compat.Instant,
        stack: *anyopaque,
        is_unwinding: bool,
    };
    pub const LogMessage = extern struct {
        tag: EventTag = .log_message,
        time: time.compat.Instant,
        stack: *anyopaque,
        info: *const EventInfo,
        message: SliceConst(u8),
    };
    pub const DeclareEventInfo = extern struct {
        tag: EventTag = .declare_event_info,
        info: *const EventInfo,
    };
    pub const StartThread = extern struct {
        tag: EventTag = .start_thread,
        time: time.compat.Instant,
        thread_id: usize,
        process_id: usize,
    };
    pub const StopThread = extern struct {
        tag: EventTag = .stop_thread,
        time: time.compat.Instant,
        thread_id: usize,
        process_id: usize,
    };
    pub const LoadImage = extern struct {
        tag: EventTag = .load_image,
        time: time.compat.Instant,
        image_base: usize,
        image_size: usize,
        image_path: paths.compat.Path,
    };
    pub const UnloadImage = extern struct {
        tag: EventTag = .unload_image,
        time: time.compat.Instant,
        image_base: usize,
    };
    pub const ContextSwitch = extern struct {
        tag: EventTag = .context_switch,
        time: time.compat.Instant,
        old_thread_id: usize,
        new_thread_id: usize,
        cpu: u8,
        old_thread_wait_reason: u8,
        old_thread_state: u8,
        previous_cstate: u8,
        new_thread_priority: i8,
        old_thread_priority: i8,
    };
    pub const ThreadWakeup = extern struct {
        tag: EventTag = .thread_wakeup,
        time: time.compat.Instant,
        thread_id: usize,
        cpu: u8,
        adjust_reason: i8,
        adjust_increment: i8,
    };
    pub const CallStackSample = extern struct {
        tag: EventTag = .call_stack_sample,
        time: time.compat.Instant,
        thread_id: usize,
        call_stack: SliceConst(usize),
    };
};

/// A subscriber for tracing events.
///
/// The main function of the tracing subsystem is managing and routing tracing events to
/// subscribers. Therefore, it does not consume any events on its own, which is the task of the
/// subscribers. Subscribers may utilize the events in any way they deem fit.
pub const Subscriber = extern struct {
    data: *anyopaque,
    on_event: *const fn (data: *anyopaque, event: *const events.EventTag) callconv(.c) void,

    pub fn of(T: type, value: *T) Subscriber {
        if (!@hasDecl(T, "fimo_subscriber")) @compileError("fimo: invalid subscriber, missing `pub const fimo_subscriber = .{...};` declaration: " ++ @typeName(T));
        const info = T.fimo_subscriber;
        const Info = @TypeOf(info);

        inline for (std.meta.fields(Info)) |f| {
            if (!@hasField(events.EventTag, f.name)) @compileError("fimo: invalid subscriber event, got: " ++ f.name);
        }

        const wrapper = struct {
            fn on_event(data: *anyopaque, event: *const events.EventTag) callconv(.c) void {
                const self: *T = @ptrCast(@alignCast(data));
                switch (event.*) {
                    .start => if (comptime @hasField(Info, "start")) {
                        const ev: *const events.Start = @alignCast(@fieldParentPtr("tag", event));
                        info.start(self, ev);
                    },
                    .finish => if (comptime @hasField(Info, "finish")) {
                        const ev: *const events.Finish = @alignCast(@fieldParentPtr("tag", event));
                        info.finish(self, ev);
                    },
                    .register_thread => if (comptime @hasField(Info, "register_thread")) {
                        const ev: *const events.RegisterThread = @alignCast(@fieldParentPtr("tag", event));
                        info.register_thread(self, ev);
                    },
                    .unregister_thread => if (comptime @hasField(Info, "unregister_thread")) {
                        const ev: *const events.UnregisterThread = @alignCast(@fieldParentPtr("tag", event));
                        info.unregister_thread(self, ev);
                    },
                    .create_call_stack => if (comptime @hasField(Info, "create_call_stack")) {
                        const ev: *const events.CreateCallStack = @alignCast(@fieldParentPtr("tag", event));
                        info.create_call_stack(self, ev);
                    },
                    .destroy_call_stack => if (comptime @hasField(Info, "destroy_call_stack")) {
                        const ev: *const events.DestroyCallStack = @alignCast(@fieldParentPtr("tag", event));
                        info.destroy_call_stack(self, ev);
                    },
                    .unblock_call_stack => if (comptime @hasField(Info, "unblock_call_stack")) {
                        const ev: *const events.UnblockCallStack = @alignCast(@fieldParentPtr("tag", event));
                        info.unblock_call_stack(self, ev);
                    },
                    .suspend_call_stack => if (comptime @hasField(Info, "suspend_call_stack")) {
                        const ev: *const events.SuspendCallStack = @alignCast(@fieldParentPtr("tag", event));
                        info.suspend_call_stack(self, ev);
                    },
                    .resume_call_stack => if (comptime @hasField(Info, "resume_call_stack")) {
                        const ev: *const events.ResumeCallStack = @alignCast(@fieldParentPtr("tag", event));
                        info.resume_call_stack(self, ev);
                    },
                    .enter_span => if (comptime @hasField(Info, "enter_span")) {
                        const ev: *const events.EnterSpan = @alignCast(@fieldParentPtr("tag", event));
                        info.enter_span(self, ev);
                    },
                    .exit_span => if (comptime @hasField(Info, "exit_span")) {
                        const ev: *const events.ExitSpan = @alignCast(@fieldParentPtr("tag", event));
                        info.exit_span(self, ev);
                    },
                    .log_message => if (comptime @hasField(Info, "log_message")) {
                        const ev: *const events.LogMessage = @alignCast(@fieldParentPtr("tag", event));
                        info.log_message(self, ev);
                    },
                    .declare_event_info => if (comptime @hasField(Info, "declare_event_info")) {
                        const ev: *const events.DeclareEventInfo = @alignCast(@fieldParentPtr("tag", event));
                        info.declare_event_info(self, ev);
                    },
                    .start_thread => if (comptime @hasField(Info, "start_thread")) {
                        const ev: *const events.StartThread = @alignCast(@fieldParentPtr("tag", event));
                        info.start_thread(self, ev);
                    },
                    .stop_thread => if (comptime @hasField(Info, "stop_thread")) {
                        const ev: *const events.StopThread = @alignCast(@fieldParentPtr("tag", event));
                        info.stop_thread(self, ev);
                    },
                    .load_image => if (comptime @hasField(Info, "load_image")) {
                        const ev: *const events.LoadImage = @alignCast(@fieldParentPtr("tag", event));
                        info.load_image(self, ev);
                    },
                    .unload_image => if (comptime @hasField(Info, "unload_image")) {
                        const ev: *const events.UnloadImage = @alignCast(@fieldParentPtr("tag", event));
                        info.unload_image(self, ev);
                    },
                    .context_switch => if (comptime @hasField(Info, "context_switch")) {
                        const ev: *const events.ContextSwitch = @alignCast(@fieldParentPtr("tag", event));
                        info.context_switch(self, ev);
                    },
                    .thread_wakeup => if (comptime @hasField(Info, "thread_wakeup")) {
                        const ev: *const events.ThreadWakeup = @alignCast(@fieldParentPtr("tag", event));
                        info.thread_wakeup(self, ev);
                    },
                    .call_stack_sample => if (comptime @hasField(Info, "call_stack_sample")) {
                        const ev: *const events.CallStackSample = @alignCast(@fieldParentPtr("tag", event));
                        info.call_stack_sample(self, ev);
                    },
                    else => {},
                }
            }
        };
        return .{ .data = value, .on_event = &wrapper.on_event };
    }

    pub fn start(self: Subscriber, event: events.Start) void {
        std.debug.assert(event.tag == .start);
        self.on_event(self.data, &event.tag);
    }

    pub fn finish(self: Subscriber, event: events.Finish) void {
        std.debug.assert(event.tag == .finish);
        self.on_event(self.data, &event.tag);
    }

    pub fn registerThread(self: Subscriber, event: events.RegisterThread) void {
        std.debug.assert(event.tag == .register_thread);
        self.on_event(self.data, &event.tag);
    }

    pub fn unregisterThread(self: Subscriber, event: events.UnregisterThread) void {
        std.debug.assert(event.tag == .unregister_thread);
        self.on_event(self.data, &event.tag);
    }

    pub fn createCallStack(self: Subscriber, event: events.CreateCallStack) void {
        std.debug.assert(event.tag == .create_call_stack);
        self.on_event(self.data, &event.tag);
    }

    pub fn destroyCallStack(self: Subscriber, event: events.DestroyCallStack) void {
        std.debug.assert(event.tag == .destroy_call_stack);
        self.on_event(self.data, &event.tag);
    }

    pub fn unblockCallStack(self: Subscriber, event: events.UnblockCallStack) void {
        std.debug.assert(event.tag == .unblock_call_stack);
        self.on_event(self.data, &event.tag);
    }

    pub fn suspendCallStack(self: Subscriber, event: events.SuspendCallStack) void {
        std.debug.assert(event.tag == .suspend_call_stack);
        self.on_event(self.data, &event.tag);
    }

    pub fn resumeCallStack(self: Subscriber, event: events.ResumeCallStack) void {
        std.debug.assert(event.tag == .resume_call_stack);
        self.on_event(self.data, &event.tag);
    }

    pub fn enterSpan(self: Subscriber, event: events.EnterSpan) void {
        std.debug.assert(event.tag == .enter_span);
        self.on_event(self.data, &event.tag);
    }

    pub fn exitSpan(self: Subscriber, event: events.ExitSpan) void {
        std.debug.assert(event.tag == .exit_span);
        self.on_event(self.data, &event.tag);
    }

    pub fn logMessage(self: Subscriber, event: events.LogMessage) void {
        std.debug.assert(event.tag == .log_message);
        self.on_event(self.data, &event.tag);
    }

    pub fn declareEventInfo(self: Subscriber, event: events.DeclareEventInfo) void {
        std.debug.assert(event.tag == .declare_event_info);
        self.on_event(self.data, &event.tag);
    }

    pub fn start_thread(self: Subscriber, event: events.StartThread) void {
        std.debug.assert(event.tag == .start_thread);
        self.on_event(self.data, &event.tag);
    }

    pub fn stop_thread(self: Subscriber, event: events.StopThread) void {
        std.debug.assert(event.tag == .stop_thread);
        self.on_event(self.data, &event.tag);
    }

    pub fn load_image(self: Subscriber, event: events.LoadImage) void {
        std.debug.assert(event.tag == .load_image);
        self.on_event(self.data, &event.tag);
    }

    pub fn unload_image(self: Subscriber, event: events.UnloadImage) void {
        std.debug.assert(event.tag == .unload_image);
        self.on_event(self.data, &event.tag);
    }

    pub fn contextSwitch(self: Subscriber, event: events.ContextSwitch) void {
        std.debug.assert(event.tag == .context_switch);
        self.on_event(self.data, &event.tag);
    }

    pub fn threadWakeup(self: Subscriber, event: events.ThreadWakeup) void {
        std.debug.assert(event.tag == .thread_wakeup);
        self.on_event(self.data, &event.tag);
    }

    pub fn callStackSample(self: Subscriber, event: events.CallStackSample) void {
        std.debug.assert(event.tag == .call_stack_sample);
        self.on_event(self.data, &event.tag);
    }
};

/// Type of a formatter function.
///
/// The formatter function is allowed to format only part of the message, if it would not fit into
/// the buffer.
pub const Formatter = fn (
    data: *const anyopaque,
    buffer: [*]u8,
    buffer_len: usize,
) callconv(.c) usize;

/// Formatter of the zig standard library.
pub fn stdFormatter(comptime fmt: []const u8, ARGS: type) Formatter {
    return struct {
        fn format(
            data: *const anyopaque,
            buffer: [*]u8,
            buffer_len: usize,
        ) callconv(.c) usize {
            const b = buffer[0..buffer_len];
            const args: *const ARGS = @ptrCast(@alignCast(data));
            return if (std.fmt.bufPrint(b, fmt, args.*)) |out|
                out.len
            else |_|
                buffer_len;
        }
    }.format;
}

/// Formatter for a zig stack trace.
pub fn stackTraceFormatter(
    data: *const anyopaque,
    buffer: [*]u8,
    buffer_len: usize,
) callconv(.c) usize {
    const buf = buffer[0..buffer_len];
    const stack_trace: *const std.builtin.StackTrace = @ptrCast(@alignCast(data));
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
pub const Cfg = extern struct {
    cfg: ctx.Cfg = .{ .id = .tracing },
    /// Length in bytes of the per-call-stack buffer used when formatting mesasges.
    format_buffer_len: usize = 0,
    /// Maximum level for which to consume tracing events.
    max_level: Level = switch (builtin.mode) {
        .Debug => .trace,
        .ReleaseSafe => .warn,
        .ReleaseFast, .ReleaseSmall => .off,
    },
    /// Slice of subscribers to register with the tracing subsystem.
    subscribers: SliceConst(Subscriber) = .fromSlice(null),
    /// Register the calling thread.
    register_thread: bool = true,
    /// Name of the application.
    app_name: SliceConst(u8) = .fromSlice(""),
};

/// Base VTable of the tracing subsystem.
///
/// Changing this definition is a breaking change.
pub const VTable = extern struct {
    is_enabled: *const fn () callconv(.c) bool,
    register_thread: *const fn () callconv(.c) void,
    unregister_thread: *const fn () callconv(.c) void,
    init_call_stack: *const fn () callconv(.c) *CallStack,
    deinit_call_stack: *const fn (stack: *CallStack, abort: bool) callconv(.c) void,
    replace_current_call_stack: *const fn (stack: *CallStack) callconv(.c) *CallStack,
    unblock_call_stack: *const fn (stack: *CallStack) callconv(.c) void,
    suspend_current_call_stack: *const fn (mark_blocked: bool) callconv(.c) void,
    resume_current_call_stack: *const fn () callconv(.c) void,
    enter_span: *const fn (
        info: *const EventInfo,
        formatter: *const Formatter,
        formatter_data: *const anyopaque,
    ) callconv(.c) void,
    exit_span: *const fn (info: *const EventInfo) callconv(.c) void,
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
