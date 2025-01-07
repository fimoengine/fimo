const std = @import("std");

const c = @import("../c.zig");
const AnyError = @import("../AnyError.zig");
const time = @import("../time.zig");
const tls = @import("tls.zig");

const StackFrame = @import("tracing/StackFrame.zig");

const ProxyTracing = @import("proxy_context/tracing.zig");
const Tracing = @This();

const Allocator = std.mem.Allocator;

allocator: Allocator,
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
pub fn init(allocator: Allocator, config: ?*const ProxyTracing.Config) (TracingError || tls.TlsError)!Tracing {
    errdefer {
        if (config) |cfg| {
            if (cfg.subscribers) |sl| for (sl[0..cfg.subscriber_count]) |s| s.deinit();
        }
    }

    var self = Tracing{
        .allocator = allocator,
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
    self.allocator.free(self.subscribers);
}

/// Creates a new empty call stack.
///
/// If successful, the new call stack is marked as suspended. The
/// new call stack is not set to be the active call stack.
pub fn createCallStack(
    self: *const Tracing,
    err: *?AnyError,
) (TracingError || AnyError.Error)!*ProxyTracing.CallStack {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
    if (!self.isEnabled()) return StackFrame.dummy_span;
    if (!self.isEnabledForCurrentThread()) return error.ThreadNotRegistered;
    const d = self.thread_data.get().?;
    return d.call_stack.pushSpan(desc, formatter, data, err);
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
    var err: ?AnyError = null;
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
) (TracingError || AnyError.Error)!void {
    const event = ProxyTracing.Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = .err,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    var err: ?AnyError = null;
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
    err: *?AnyError,
) (TracingError || AnyError.Error)!void {
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
pub fn registerThread(self: *Tracing, err: *?AnyError) (TracingError || tls.TlsError || AnyError.Error)!void {
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
        err: *?AnyError,
    ) (TracingError || tls.TlsError || AnyError.Error)!*ThreadData {
        const data = try owner.allocator.create(ThreadData);
        errdefer owner.allocator.destroy(data);

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
        owner.allocator.destroy(self);

        // Synchronizes with the acquire on deinit of the context.
        _ = owner.thread_count.fetchSub(1, .release);
    }

    fn deinitAssert(self: *ThreadData) callconv(.C) void {
        self.deinit() catch unreachable;
    }
};

pub const CallStack = struct {
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
        err: *?AnyError,
    ) (TracingError || AnyError.Error)!*CallStack {
        const call_stack = try owner.allocator.create(CallStack);
        errdefer owner.allocator.destroy(call_stack);
        call_stack.* = CallStack{
            .buffer = undefined,
            .max_level = owner.max_level,
            .call_stacks = undefined,
            .owner = owner,
        };

        call_stack.buffer = try owner.allocator.alloc(u8, owner.buffer_size);
        errdefer owner.allocator.free(call_stack.buffer);

        call_stack.call_stacks = try std.ArrayListUnmanaged(*anyopaque).initCapacity(
            owner.allocator,
            owner.subscribers.len,
        );
        errdefer {
            call_stack.call_stacks.deinit(owner.allocator);
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

        self.call_stacks.deinit(self.owner.allocator);
        self.owner.allocator.free(self.buffer);
        self.owner.allocator.destroy(self);
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
        err: *?AnyError,
    ) (TracingError || AnyError.Error)!ProxyTracing.Span {
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
        return frame.asProxySpan();
    }

    fn emitEvent(
        self: *CallStack,
        event: *const ProxyTracing.Event,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
        err: *?AnyError,
    ) (TracingError || AnyError.Error)!void {
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
        try AnyError.initChecked(err, result);
        const message = format_buffer[0..written_characters];

        const now = time.Time.now();
        for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
            subscriber.emitEvent(now, call_stack, event, message);
        }
    }
};

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    const Context = @import("../context.zig");

    fn createCallStack(ptr: *anyopaque, call_stack: **ProxyTracing.CallStack) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        var err: ?AnyError = null;
        if (ctx.tracing.createCallStack(&err)) |cs| {
            call_stack.* = cs;
            return AnyError.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return AnyError.initError(e).err,
        }
    }
    fn destroyCallStack(ptr: *anyopaque, call_stack: *ProxyTracing.CallStack) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.destroyCallStack(
            call_stack,
        ) catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }
    fn replaceCurrentCallStack(
        ptr: *anyopaque,
        call_stack: *ProxyTracing.CallStack,
        old: **ProxyTracing.CallStack,
    ) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        old.* = ctx.tracing.replaceCurrentCallStack(
            call_stack,
        ) catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }
    fn unblockCallStack(ptr: *anyopaque, call_stack: *ProxyTracing.CallStack) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.unblockCallStack(
            call_stack,
        ) catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }
    fn suspendCurrentCallStack(ptr: *anyopaque, mark_blocked: bool) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.suspendCurrentCallStack(
            mark_blocked,
        ) catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }
    fn resumeCurrentCallStack(ptr: *anyopaque) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.resumeCurrentCallStack() catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }
    fn pushSpan(
        ptr: *anyopaque,
        desc: *const ProxyTracing.SpanDesc,
        span: *ProxyTracing.Span,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        var err: ?AnyError = null;
        if (ctx.tracing.pushSpanCustom(desc, formatter, data, &err)) |sp| {
            span.* = sp;
            return AnyError.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return AnyError.initError(e).err,
        }
    }
    fn emitEvent(
        ptr: *anyopaque,
        event: *const ProxyTracing.Event,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        var err: ?AnyError = null;
        if (ctx.tracing.emitEventCustom(event, formatter, data, &err)) |_| {
            return AnyError.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return AnyError.initError(e).err,
        }
    }
    fn isEnabled(ptr: *anyopaque) callconv(.C) bool {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        return ctx.tracing.isEnabled();
    }
    fn registerThread(ptr: *anyopaque) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        var err: ?AnyError = null;
        if (ctx.tracing.registerThread(&err)) |_| {
            return AnyError.intoCResult(null);
        } else |e| switch (e) {
            error.FfiError => return err.?.err,
            else => return AnyError.initError(e).err,
        }
    }
    fn unregisterThread(ptr: *anyopaque) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.unregisterThread() catch |err| return AnyError.initError(err).err;
        return AnyError.intoCResult(null);
    }
    fn flush(ptr: *anyopaque) callconv(.C) c.FimoResult {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.flush();
        return AnyError.intoCResult(null);
    }
};

pub const vtable = ProxyTracing.VTable{
    .call_stack_create = &VTableImpl.createCallStack,
    .call_stack_destroy = &VTableImpl.destroyCallStack,
    .call_stack_switch = &VTableImpl.replaceCurrentCallStack,
    .call_stack_unblock = &VTableImpl.unblockCallStack,
    .call_stack_suspend_current = &VTableImpl.suspendCurrentCallStack,
    .call_stack_resume_current = &VTableImpl.resumeCurrentCallStack,
    .span_create = &VTableImpl.pushSpan,
    .event_emit = &VTableImpl.emitEvent,
    .is_enabled = &VTableImpl.isEnabled,
    .register_thread = &VTableImpl.registerThread,
    .unregister_thread = &VTableImpl.unregisterThread,
    .flush = &VTableImpl.flush,
};
