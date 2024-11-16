const std = @import("std");

const c = @import("../c.zig");
const heap = @import("../heap.zig");
const Error = @import("../errors.zig").Error;
const time = @import("../time.zig");
const tls = @import("tls.zig");

const ProxyTracing = @import("proxy_context/tracing.zig");
const Tracing = @This();

const Allocator = std.mem.Allocator;
const allocator = heap.fimo_allocator;

subscribers: []ProxyTracing.Subscriber,
buffer_size: usize,
max_level: ProxyTracing.Level,
thread_data: tls.Tls(ThreadData),
thread_count: std.atomic.Value(usize),

const dummy_call_stack_ptr: *ProxyTracing.CallStack = @ptrFromInt(1);
const dummy_span = ProxyTracing.Span{};

/// Errors of the tracing subsystem.
pub const TracingError = error{
    ThreadRegistered,
    ThreadNotRegistered,
    CallStackInUse,
    CallStackEmpty,
    CallStackBound,
    CallStackNotBound,
    CallStackSuspended,
    CallStackNotSuspended,
    CallStackBlocked,
    CallStackNotBlocked,
    CallStackNotEmpty,
    CallStackSpanNotOnTop,
} || Allocator.Error;

/// Initializes the tracing subsystem.
pub fn init(config: ?*const ProxyTracing.Config) (TracingError || tls.TlsError)!Tracing {
    errdefer {
        if (config) |cfg| {
            if (cfg.subscribers) |sl| for (sl[0..cfg.subscriber_count]) |s| s.deinit();
        }
    }

    var self = Tracing{
        .subscribers = undefined,
        .buffer_size = undefined,
        .max_level = undefined,
        .thread_data = undefined,
        .thread_count = undefined,
    };

    if (config) |cfg| {
        const subscribers = if (cfg.subscribers) |s| s[0..cfg.subscriber_count] else @as(
            []ProxyTracing.Subscriber,
            &.{},
        );
        self.subscribers = try allocator.dupe(ProxyTracing.Subscriber, subscribers);
        self.buffer_size = if (cfg.format_buffer_len != 0) cfg.format_buffer_len else 1024;
        self.max_level = cfg.max_level;
    } else {
        self.subscribers = try allocator.dupe(ProxyTracing.Subscriber, &.{});
        self.buffer_size = 0;
        self.max_level = .off;
    }
    errdefer allocator.free(self.subscribers);
    self.thread_data = try tls.Tls(ThreadData).init(ThreadData.deinitAssert);
    errdefer self.thread_data.deinit();
    self.thread_count.store(0, .unordered);

    return self;
}

/// Deinitializes the tracing subsystem.
///
/// May fail if not all threads have been registered.
pub fn deinit(self: *Tracing) void {
    if (self.thread_data.get()) |data| {
        data.deinitAssert();
        self.thread_data.set(null) catch unreachable;
    }

    // Use an acquire load to synchronize with the release in deinit.
    const num_threads = self.thread_count.load(.acquire);
    std.debug.assert(num_threads == 0);

    self.thread_data.deinit();
    for (self.subscribers) |subs| subs.deinit();
    allocator.free(self.subscribers);
}

/// Creates a new empty call stack.
///
/// If successful, the new call stack is marked as suspended. The
/// new call stack is not set to be the active call stack.
pub fn createCallStack(
    self: *const Tracing,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.CallStack {
    if (!self.isEnabled()) return dummy_call_stack_ptr;
    const call_stack = try CallStack.init(self, err);
    return @ptrCast(call_stack);
}

/// Destroys an empty call stack.
///
/// Marks the completion of a task. Before calling this function, the
/// call stack must be empty, i.e., there must be no active spans on
/// the stack, and must not be active. If successful, the call stack
/// may not be used afterwards. The active call stack of the thread
/// is destroyed automatically, on thread exit or during destruction
/// of the context. The caller must own the call stack uniquely.
pub fn destroyCallStack(
    self: *const Tracing,
    call_stack: *ProxyTracing.CallStack,
) TracingError!void {
    if (!self.isEnabled()) {
        std.debug.assert(call_stack == dummy_call_stack_ptr);
        return;
    }
    const cs: *CallStack = @alignCast(@ptrCast(call_stack));
    try cs.deinitUnbound();
}

/// Switches the call stack of the current thread.
///
/// If successful, this call stack will be used as the active call
/// stack of the calling thread. The old call stack is returned,
/// enabling the caller to switch back to it afterwards. This call
/// stack must be in a suspended, but unblocked, state and not be
/// active. The active call stack must also be in a suspended state,
/// but may also be blocked.
pub fn replaceCurrentCallStack(
    self: *const Tracing,
    call_stack: *ProxyTracing.CallStack,
) TracingError!*ProxyTracing.CallStack {
    if (!self.isEnabled()) {
        std.debug.assert(call_stack == dummy_call_stack_ptr);
        return dummy_call_stack_ptr;
    }
    if (!self.isEnabledForCurrentThread()) return error.ThreadNotRegistered;
    const data = self.thread_data.get().?;
    const cs: *CallStack = @alignCast(@ptrCast(call_stack));
    if (data.call_stack == cs) return error.CallStackBound;
    try cs.bind();
    data.call_stack.unbind();
    const old = data.call_stack;
    data.call_stack = cs;
    return @ptrCast(old);
}

/// Unblocks a blocked call stack.
///
/// Once unblocked, the call stack may be resumed. The call stack
/// may not be active and must be marked as blocked.
pub fn unblockCallStack(self: *const Tracing, call_stack: *ProxyTracing.CallStack) TracingError!void {
    if (!self.isEnabled()) {
        std.debug.assert(call_stack == dummy_call_stack_ptr);
        return;
    }
    const cs: *CallStack = @alignCast(@ptrCast(call_stack));
    return cs.unblock();
}

/// Marks the current call stack as being suspended.
///
/// While suspended, the call stack can not be utilized for tracing
/// messages. The call stack optionally also be marked as being
/// blocked. In that case, the call stack must be unblocked prior
/// to resumption.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub fn suspendCurrentCallStack(self: *const Tracing, mark_blocked: bool) TracingError!void {
    if (!self.isEnabled()) return;
    if (!self.isEnabledForCurrentThread()) return error.ThreadNotRegistered;
    const data = self.thread_data.get().?;
    return data.call_stack.@"suspend"(mark_blocked);
}

/// Marks the current call stack as being resumed.
///
/// Once resumed, the context can be used to trace messages. To be
/// successful, the current call stack must be suspended and unblocked.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub fn resumeCurrentCallStack(self: *const Tracing) TracingError!void {
    if (!self.isEnabled()) return;
    if (!self.isEnabledForCurrentThread()) return error.ThreadNotRegistered;
    const data = self.thread_data.get().?;
    return data.call_stack.@"resume"();
}

/// Creates a new span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpan(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    level: ProxyTracing.Level,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    const desc = &struct {
        var desc = ProxyTracing.SpanDesc{
            .metadata = &.{
                .name = name orelse location.fn_name,
                .target = target orelse location.module,
                .level = level,
                .file_name = location.file,
                .line_number = @intCast(location.line),
            },
        };
    }.desc;
    return self.pushSpanCustom(
        desc,
        ProxyTracing.stdFormatter(fmt, @TypeOf(args)),
        &args,
        err,
    );
}

/// Creates a new error span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanErr(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .err,
        location,
        err,
    );
}

/// Creates a new warn span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanWarn(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .warn,
        location,
        err,
    );
}

/// Creates a new info span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanInfo(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .info,
        location,
        err,
    );
}

/// Creates a new debug span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanDebug(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .debug,
        location,
        err,
    );
}

/// Creates a new trace span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanTrace(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .trace,
        location,
        err,
    );
}

/// Creates a new error span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanErrSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpanErr(
        fmt,
        args,
        null,
        null,
        location,
        err,
    );
}

/// Creates a new warn span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanWarnSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpanWarn(
        fmt,
        args,
        null,
        null,
        location,
        err,
    );
}

/// Creates a new info span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanInfoSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpanInfo(
        fmt,
        args,
        null,
        null,
        location,
        err,
    );
}

/// Creates a new debug span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanDebugSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpanDebug(
        fmt,
        args,
        null,
        null,
        location,
        err,
    );
}

/// Creates a new trace span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The message is formatted with the default
/// formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub inline fn pushSpanTraceSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    return self.pushSpanTrace(
        fmt,
        args,
        null,
        null,
        location,
        err,
    );
}

/// Creates a new span with a custom formatter and enters it.
///
/// If successful, the newly created span is used as the context for
/// succeeding events. The subsystem may use a formatting buffer of a
/// fixed size. The formatter is expected to cut-of the message after
/// reaching that specified size. The `desc` must remain valid until
/// the span is destroyed.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub fn pushSpanCustom(
    self: *const Tracing,
    desc: *const ProxyTracing.SpanDesc,
    formatter: *const ProxyTracing.Formatter,
    data: ?*const anyopaque,
    err: *?Error,
) (TracingError || error{FfiError})!*ProxyTracing.Span {
    if (!self.isEnabled()) return @constCast(&dummy_span);
    if (!self.isEnabledForCurrentThread()) return error.ThreadNotRegistered;
    const d = self.thread_data.get().?;
    return d.call_stack.pushSpan(desc, formatter, data, err);
}

/// Removes the span from the top of the current call stack.
///
/// If successful, succeeding events won't occur inside the context of the
/// exited span anymore. The span must be the span at the top of the current
/// call stack. The span may not be in use prior to a call to this function,
/// and may not be used afterwards.
///
/// This function may return an error, if the current thread is not
/// registered with the subsystem.
pub fn popSpan(self: *const Tracing, span: *ProxyTracing.Span) TracingError!void {
    if (!self.isEnabled()) {
        std.debug.assert(span == &dummy_span);
        return;
    }
    if (!self.isEnabledForCurrentThread()) return error.ThreadNotRegistered;
    const data = self.thread_data.get().?;
    return data.call_stack.popSpan(span);
}

/// Emits a new event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitEvent(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    level: ProxyTracing.Level,
    location: std.builtin.SourceLocation,
) TracingError!void {
    const event = ProxyTracing.Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = level,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    var err: ?Error = null;
    self.emitEventCustom(
        &event,
        ProxyTracing.stdFormatter(fmt, @TypeOf(args)),
        &args,
        &err,
    ) catch |e| switch (e) {
        error.FfiError => err.?.deinit(),
        else => return @as(TracingError, @errorCast(e)),
    };
}

/// Emits a new error event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitErr(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) TracingError!void {
    return self.emitEvent(
        fmt,
        args,
        name,
        target,
        .err,
        location,
    );
}

/// Emits a new warn event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitWarn(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) TracingError!void {
    return self.emitEvent(
        fmt,
        args,
        name,
        target,
        .warn,
        location,
    );
}

/// Emits a new info event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitInfo(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) TracingError!void {
    return self.emitEvent(
        fmt,
        args,
        name,
        target,
        .info,
        location,
    );
}

/// Emits a new debug event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitDebug(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) TracingError!void {
    return self.emitEvent(
        fmt,
        args,
        name,
        target,
        .debug,
        location,
    );
}

/// Emits a new trace event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitTrace(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) TracingError!void {
    return self.emitEvent(
        fmt,
        args,
        name,
        target,
        .trace,
        location,
    );
}

/// Emits a new error event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitErrSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitErr(
        fmt,
        args,
        null,
        null,
        location,
    ) catch @panic("Trace failed");
}

/// Emits a new warn event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitWarnSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitWarn(
        fmt,
        args,
        null,
        null,
        location,
    ) catch @panic("Trace failed");
}

/// Emits a new info event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitInfoSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitInfo(
        fmt,
        args,
        null,
        null,
        location,
    ) catch @panic("Trace failed");
}

/// Emits a new debug event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitDebugSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitDebug(
        fmt,
        args,
        null,
        null,
        location,
    ) catch @panic("Trace failed");
}

/// Emits a new trace event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig
/// standard library. The message may be cut of, if the length exceeds
/// the internal formatting buffer size.
pub fn emitTraceSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return self.emitTrace(
        fmt,
        args,
        null,
        null,
        location,
    ) catch @panic("Trace failed");
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal
/// formatting buffer size.
pub fn emitStackTrace(
    self: *const Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    stack_trace: std.builtin.StackTrace,
    location: std.builtin.SourceLocation,
) (TracingError || error{FfiError})!void {
    const event = ProxyTracing.Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = .err,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    var err: ?Error = null;
    self.emitEventCustom(
        &event,
        ProxyTracing.stackTraceFormatter,
        &stack_trace,
        &err,
    ) catch |e| switch (e) {
        error.FfiError => err.?.deinit(),
        else => return e,
    };
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal
/// formatting buffer size.
pub fn emitStackTraceSimple(
    self: *const Tracing,
    stack_trace: std.builtin.StackTrace,
    location: std.builtin.SourceLocation,
) void {
    return self.emitStackTrace(
        null,
        null,
        stack_trace,
        location,
    ) catch @panic("Trace failed");
}

/// Emits a new event with a custom formatter.
///
/// The subsystem may use a formatting buffer of a fixed size. The formatter is
/// expected to cut-of the message after reaching that specified size.
pub fn emitEventCustom(
    self: *const Tracing,
    event: *const ProxyTracing.Event,
    formatter: *const ProxyTracing.Formatter,
    data: ?*const anyopaque,
    err: *?Error,
) (TracingError || error{FfiError})!void {
    if (!self.wouldTrace(event.metadata)) return;
    const d = self.thread_data.get().?;
    try d.call_stack.emitEvent(event, formatter, data, err);
}

/// Returns whether the subsystem was configured to enable tracing.
///
/// This is the case if there are any subscribers and the trace level is not `off`.
pub fn isEnabled(self: *const Tracing) bool {
    return !(self.max_level == .off or self.subscribers.len == 0);
}

/// Returns whether the subsystem is configured to trace the current thread.
///
/// In addition to requiring the correctt configuration of the subsystem,
/// this also requires that the current thread be registered.
pub fn isEnabledForCurrentThread(self: *const Tracing) bool {
    return self.isEnabled() and self.thread_data.get() != null;
}

/// Checks whether an event or span with the provided metadata would lead to a tracing operation.
pub fn wouldTrace(self: *const Tracing, metadata: *const ProxyTracing.Metadata) bool {
    if (!self.isEnabledForCurrentThread()) return false;
    return @intFromEnum(self.max_level) >= @intFromEnum(metadata.level);
}

/// Tries to register the current thread with the subsystem.
///
/// Upon registration, the current thread is assigned a new tracing call stack.
pub fn registerThread(self: *Tracing, err: *?Error) (TracingError || tls.TlsError || error{FfiError})!void {
    if (!self.isEnabled()) return;
    if (self.thread_data.get() != null) return error.ThreadRegistered;

    const data = try ThreadData.init(self, err);
    errdefer data.deinitAssert();
    try self.thread_data.set(data);
}

/// Tries to unregister the current thread from the subsystem.
///
/// May fail if the call stack of the thread is not empty.
pub fn unregisterThread(self: *Tracing) TracingError!void {
    if (!self.isEnabled()) return;

    const data = self.thread_data.get();
    if (data) |d| {
        try d.deinit();
        self.thread_data.set(null) catch unreachable;
    } else return error.ThreadNotRegistered;
}

/// Flushes all tracing messages from the subscribers.
pub fn flush(self: *const Tracing) void {
    if (!self.isEnabled()) return;
    for (self.subscribers) |sub| {
        sub.flush();
    }
}

const ThreadData = struct {
    call_stack: *CallStack,
    owner: *Tracing,

    fn init(
        owner: *Tracing,
        err: *?Error,
    ) (TracingError || tls.TlsError || error{FfiError})!*ThreadData {
        const data = try allocator.create(ThreadData);
        errdefer allocator.destroy(data);

        data.* = ThreadData{
            .call_stack = try CallStack.init(owner, err),
            .owner = owner,
        };
        errdefer data.call_stack.deinit() catch @panic("Unwind error");
        try data.call_stack.bind();
        errdefer data.call_stack.unbind();
        try data.call_stack.@"resume"();

        // Is a counter so it does not need any synchronization.
        _ = owner.thread_count.fetchAdd(1, .monotonic);
        return data;
    }

    fn deinit(self: *ThreadData) TracingError!void {
        const owner = self.owner;
        try self.call_stack.deinit();
        allocator.destroy(self);

        // Synchronizes with the acquire on deinit of the context.
        _ = owner.thread_count.fetchSub(1, .release);
    }

    fn deinitAssert(self: *ThreadData) callconv(.C) void {
        self.deinit() catch unreachable;
    }
};

const CallStack = struct {
    mutex: std.Thread.Mutex.Recursive = std.Thread.Mutex.Recursive.init,
    state: packed struct(u8) {
        suspended: bool = true,
        blocked: bool = false,
        _padding: u6 = 0,
    } = .{},
    buffer: []u8,
    cursor: usize = 0,
    max_level: ProxyTracing.Level,
    call_stacks: std.ArrayListUnmanaged(*anyopaque),
    start_frame: ?*StackFrame = null,
    end_frame: ?*StackFrame = null,
    owner: *const Tracing,

    fn init(
        owner: *const Tracing,
        err: *?Error,
    ) (TracingError || error{FfiError})!*CallStack {
        const call_stack = try allocator.create(CallStack);
        errdefer allocator.destroy(call_stack);
        call_stack.* = CallStack{
            .buffer = undefined,
            .max_level = owner.max_level,
            .call_stacks = undefined,
            .owner = owner,
        };

        call_stack.buffer = try allocator.alloc(u8, owner.buffer_size);
        errdefer allocator.free(call_stack.buffer);

        call_stack.call_stacks = try std.ArrayListUnmanaged(*anyopaque).initCapacity(
            allocator,
            owner.subscribers.len,
        );
        errdefer {
            call_stack.call_stacks.deinit(allocator);
            for (call_stack.call_stacks.items, owner.subscribers) |cs, subscriber| {
                subscriber.dropCallStack(cs);
            }
        }

        const now = time.Time.now();
        for (owner.subscribers) |subscriber| {
            const cs = try subscriber.createCallStack(now, err);
            call_stack.call_stacks.appendAssumeCapacity(cs);
        }

        return call_stack;
    }

    fn deinit(self: *CallStack) TracingError!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        errdefer self.mutex.unlock();
        if (self.state.blocked) return error.CallStackBlocked;
        if (self.end_frame != null) return error.CallStackNotEmpty;

        const now = time.Time.now();
        for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
            subscriber.destroyCallStack(now, call_stack);
        }

        self.call_stacks.deinit(allocator);
        allocator.free(self.buffer);
        allocator.destroy(self);
    }

    fn deinitUnbound(self: *CallStack) TracingError!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        errdefer self.mutex.unlock();
        if (self.mutex.lock_count != 1) return error.CallStackBound;
        try self.deinit();
    }

    fn bind(self: *CallStack) TracingError!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        errdefer self.mutex.unlock();
        if (self.mutex.lock_count != 1) return error.CallStackBound;
        if (self.state.blocked) return error.CallStackBlocked;
        if (!self.state.suspended) return error.CallStackNotSuspended;
    }

    fn unbind(self: *CallStack) void {
        std.debug.assert(self.mutex.lock_count == 1);
        self.mutex.unlock();
    }

    fn unblock(self: *CallStack) TracingError!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        defer self.mutex.unlock();
        if (self.mutex.lock_count != 1) return error.CallStackBound;
        if (!self.state.blocked) return error.CallStackNotBlocked;
        std.debug.assert(self.state.suspended);

        const now = time.Time.now();
        for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
            subscriber.unblockCallStack(now, call_stack);
        }

        self.state.blocked = false;
    }

    fn @"suspend"(self: *CallStack, mark_blocked: bool) TracingError!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        defer self.mutex.unlock();
        if (self.mutex.lock_count == 1) return error.CallStackNotBound;
        if (self.state.suspended) return error.CallStackSuspended;

        const now = time.Time.now();
        for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
            subscriber.suspendCallStack(now, call_stack, mark_blocked);
        }

        self.state.suspended = true;
        self.state.blocked = mark_blocked;
    }

    fn @"resume"(self: *CallStack) TracingError!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        defer self.mutex.unlock();
        if (self.mutex.lock_count == 1) return error.CallStackNotBound;
        if (self.state.blocked) return error.CallStackBlocked;
        if (!self.state.suspended) return error.CallStackNotSuspended;

        const now = time.Time.now();
        for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
            subscriber.resumeCallStack(now, call_stack);
        }

        self.state.suspended = false;
    }

    fn pushSpan(
        self: *CallStack,
        desc: *const ProxyTracing.SpanDesc,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
        err: *?Error,
    ) (TracingError || error{FfiError})!*ProxyTracing.Span {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        defer self.mutex.unlock();
        if (self.mutex.lock_count == 1) return error.CallStackNotBound;
        if (self.state.blocked) return error.CallStackBlocked;
        if (self.state.suspended) return error.CallStackSuspended;

        const frame = try StackFrame.init(
            self,
            desc,
            formatter,
            data,
            err,
        );
        return &frame.span;
    }

    fn popSpan(
        self: *CallStack,
        span: *ProxyTracing.Span,
    ) TracingError!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        defer self.mutex.unlock();
        if (self.mutex.lock_count == 1) return error.CallStackNotBound;
        if (self.state.blocked) return error.CallStackBlocked;
        if (self.state.suspended) return error.CallStackSuspended;
        if (self.end_frame == null) return error.CallStackEmpty;

        const frame = self.end_frame.?;
        if (&frame.span != span) return error.CallStackSpanNotOnTop;
        frame.deinit();
    }

    fn emitEvent(
        self: *CallStack,
        event: *const ProxyTracing.Event,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
        err: *?Error,
    ) (TracingError || error{FfiError})!void {
        if (!self.mutex.tryLock()) return error.CallStackInUse;
        defer self.mutex.unlock();
        if (self.mutex.lock_count == 1) return error.CallStackNotBound;
        if (self.state.blocked) return error.CallStackBlocked;
        if (self.state.suspended) return error.CallStackSuspended;

        if (@intFromEnum(event.metadata.level) > @intFromEnum(self.max_level)) {
            return;
        }

        var written_characters: usize = undefined;
        const format_buffer = self.buffer[self.cursor..];
        const result = formatter(
            format_buffer.ptr,
            format_buffer.len,
            data,
            &written_characters,
        );
        try Error.initChecked(err, result);
        const message = format_buffer[0..written_characters];

        const now = time.Time.now();
        for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
            subscriber.emitEvent(now, call_stack, event, message);
        }
    }

    const StackFrame = struct {
        span: ProxyTracing.Span,
        metadata: *const ProxyTracing.Metadata,
        parent_cursor: usize,
        parent_max_level: ProxyTracing.Level,
        next: ?*StackFrame = null,
        previous: ?*StackFrame,
        owner: *CallStack,

        fn init(
            owner: *CallStack,
            desc: *const ProxyTracing.SpanDesc,
            formatter: *const ProxyTracing.Formatter,
            data: ?*const anyopaque,
            err: *?Error,
        ) (TracingError || error{FfiError})!*StackFrame {
            const rest_buffer = owner.buffer[owner.cursor..];

            var written_characters: usize = undefined;
            const result = formatter(
                rest_buffer.ptr,
                rest_buffer.len,
                data,
                &written_characters,
            );
            try Error.initChecked(err, result);
            const message = rest_buffer[0..written_characters];

            var num_created_spans: usize = 0;
            errdefer {
                const call_stacks = owner.call_stacks.items[0..num_created_spans];
                const subscribers = owner.owner.subscribers[0..num_created_spans];
                for (call_stacks, subscribers) |call_stack, subscriber| {
                    subscriber.dropSpan(call_stack);
                }
            }

            const now = time.Time.now();
            for (owner.call_stacks.items, owner.owner.subscribers) |call_stack, subscriber| {
                try subscriber.createSpan(
                    now,
                    desc,
                    message,
                    call_stack,
                    err,
                );
                num_created_spans += 1;
            }

            const frame = try allocator.create(StackFrame);
            errdefer allocator.destroy(frame);
            frame.* = .{
                .span = .{},
                .metadata = desc.metadata,
                .parent_cursor = owner.cursor,
                .parent_max_level = owner.max_level,
                .previous = owner.end_frame,
                .owner = owner,
            };

            owner.cursor += written_characters;
            owner.max_level = @enumFromInt(@min(@intFromEnum(desc.metadata.level), @intFromEnum(owner.max_level)));

            if (owner.end_frame) |end_frame| {
                end_frame.next = frame;
                owner.end_frame = frame;
            } else {
                owner.start_frame = frame;
                owner.end_frame = frame;
            }

            return frame;
        }

        fn deinit(self: *StackFrame) void {
            const now = time.Time.now();
            for (self.owner.call_stacks.items, self.owner.owner.subscribers) |call_stack, subscriber| {
                subscriber.destroySpan(now, call_stack);
            }

            self.owner.cursor = self.parent_cursor;
            self.owner.max_level = self.parent_max_level;
            if (self.previous) |previous| {
                previous.next = null;
                self.owner.end_frame = previous;
            } else {
                self.owner.start_frame = null;
                self.owner.end_frame = null;
            }

            allocator.destroy(self);
        }
    };
};

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

// Remove once the c sources have been removed.
const ffi = struct {
    export fn fimo_internal_tracing_alloc() *Tracing {
        return allocator.create(Tracing) catch @panic("OOM");
    }
    export fn fimo_internal_tracing_dealloc(tracing: *Tracing) void {
        allocator.destroy(tracing);
    }
    export fn fimo_internal_tracing_init(tracing: *Tracing, cfg: ?*const ProxyTracing.Config) c.FimoResult {
        tracing.* = Tracing.init(cfg) catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    export fn fimo_internal_tracing_destroy(tracing: *Tracing) void {
        tracing.deinit();
    }
    export fn fimo_internal_tracing_event_emit_custom(
        tracing: *Tracing,
        event: *const ProxyTracing.Event,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.C) c.FimoResult {
        var err: ?Error = null;
        if (tracing.emitEventCustom(event, formatter, data, &err)) |_| {
            return Error.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return Error.initError(e).err,
        }
    }
    export fn fimo_internal_tracing_cleanup_options(cfg: *const ProxyTracing.Config) void {
        if (cfg.subscribers) |subscribers| {
            for (subscribers[0..cfg.subscriber_count]) |s| s.deinit();
        }
    }
};

comptime {
    _ = ffi;
}

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    const Context = @import("../context.zig").Context;

    fn createCallStack(ctx: *Context, call_stack: **ProxyTracing.CallStack) callconv(.C) c.FimoResult {
        var err: ?Error = null;
        if (ctx.tracing.createCallStack(&err)) |cs| {
            call_stack.* = cs;
            return Error.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return Error.initError(e).err,
        }
    }
    fn destroyCallStack(ctx: *Context, call_stack: *ProxyTracing.CallStack) callconv(.C) c.FimoResult {
        ctx.tracing.destroyCallStack(
            call_stack,
        ) catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    fn replaceCurrentCallStack(
        ctx: *Context,
        call_stack: *ProxyTracing.CallStack,
        old: **ProxyTracing.CallStack,
    ) callconv(.C) c.FimoResult {
        old.* = ctx.tracing.replaceCurrentCallStack(
            call_stack,
        ) catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    fn unblockCallStack(ctx: *Context, call_stack: *ProxyTracing.CallStack) callconv(.C) c.FimoResult {
        ctx.tracing.unblockCallStack(
            call_stack,
        ) catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    fn suspendCurrentCallStack(ctx: *Context, mark_blocked: bool) callconv(.C) c.FimoResult {
        ctx.tracing.suspendCurrentCallStack(
            mark_blocked,
        ) catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    fn resumeCurrentCallStack(ctx: *Context) callconv(.C) c.FimoResult {
        ctx.tracing.resumeCurrentCallStack() catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    fn pushSpan(
        ctx: *Context,
        desc: *const ProxyTracing.SpanDesc,
        span: **ProxyTracing.Span,
        formatter: *const ProxyTracing.Formatter,
        data: ?*anyopaque,
    ) callconv(.C) c.FimoResult {
        var err: ?Error = null;
        if (ctx.tracing.pushSpanCustom(desc, formatter, data, &err)) |sp| {
            span.* = sp;
            return Error.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return Error.initError(e).err,
        }
    }
    fn popSpan(
        ctx: *Context,
        span: *ProxyTracing.Span,
    ) callconv(.C) c.FimoResult {
        ctx.tracing.popSpan(span) catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    fn emitEvent(
        ctx: *Context,
        event: *const ProxyTracing.Event,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.C) c.FimoResult {
        var err: ?Error = null;
        if (ctx.tracing.emitEventCustom(event, formatter, data, &err)) |_| {
            return Error.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return Error.initError(e).err,
        }
    }
    fn isEnabled(ctx: *Context) callconv(.C) bool {
        return ctx.tracing.isEnabled();
    }
    fn registerThread(ctx: *Context) callconv(.C) c.FimoResult {
        var err: ?Error = null;
        if (ctx.tracing.registerThread(&err)) |_| {
            return Error.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return Error.initError(e).err,
        }
    }
    fn unregisterThread(ctx: *Context) callconv(.C) c.FimoResult {
        ctx.tracing.unregisterThread() catch |err| return Error.initError(err).err;
        return Error.intoCResult(null);
    }
    fn flush(ctx: *Context) callconv(.C) c.FimoResult {
        ctx.tracing.flush();
        return Error.intoCResult(null);
    }
};

comptime {
    @export(&VTableImpl.createCallStack, .{
        .name = "fimo_internal_trampoline_tracing_call_stack_create",
    });
    @export(&VTableImpl.destroyCallStack, .{
        .name = "fimo_internal_trampoline_tracing_call_stack_destroy",
    });
    @export(&VTableImpl.replaceCurrentCallStack, .{
        .name = "fimo_internal_trampoline_tracing_call_stack_switch",
    });
    @export(&VTableImpl.unblockCallStack, .{
        .name = "fimo_internal_trampoline_tracing_call_stack_unblock",
    });
    @export(&VTableImpl.suspendCurrentCallStack, .{
        .name = "fimo_internal_trampoline_tracing_call_stack_suspend_current",
    });
    @export(&VTableImpl.resumeCurrentCallStack, .{
        .name = "fimo_internal_trampoline_tracing_call_stack_resume_current",
    });
    @export(&VTableImpl.pushSpan, .{
        .name = "fimo_internal_trampoline_tracing_span_create",
    });
    @export(&VTableImpl.popSpan, .{
        .name = "fimo_internal_trampoline_tracing_span_destroy",
    });
    @export(&VTableImpl.emitEvent, .{
        .name = "fimo_internal_trampoline_tracing_event_emit",
    });
    @export(&VTableImpl.isEnabled, .{
        .name = "fimo_internal_trampoline_tracing_is_enabled",
    });
    @export(&VTableImpl.registerThread, .{
        .name = "fimo_internal_trampoline_tracing_register_thread",
    });
    @export(&VTableImpl.unregisterThread, .{
        .name = "fimo_internal_trampoline_tracing_unregister_thread",
    });
    @export(&VTableImpl.flush, .{
        .name = "fimo_internal_trampoline_tracing_flush",
    });
}
