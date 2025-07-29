const std = @import("std");
const Allocator = std.mem.Allocator;

const context = @import("../context.zig");
const time = @import("../time.zig");
const pub_tracing = @import("../tracing.zig");
const ResourceCount = @import("ResourceCount.zig");
const CallStack = @import("tracing/CallStack.zig");
const StackFrame = @import("tracing/StackFrame.zig");

const tracing = @This();

pub var allocator: Allocator = undefined;
pub var subscribers: []pub_tracing.Subscriber = undefined;
pub var buffer_size: usize = undefined;
pub var max_level: pub_tracing.Level = undefined;
pub var thread_count: ResourceCount = .{};
pub var call_stack_count: ResourceCount = .{};

/// Initializes the tracing subsystem.
pub fn init(config: *const pub_tracing.Config) !void {
    allocator = context.allocator;
    const subs = if (config.subscribers) |s| s[0..config.subscriber_count] else @as(
        []pub_tracing.Subscriber,
        &.{},
    );
    subscribers = try allocator.dupe(pub_tracing.Subscriber, subs);
    for (subscribers) |sub| sub.ref();
    buffer_size = if (config.format_buffer_len != 0) config.format_buffer_len else 1024;
    max_level = config.max_level;
    errdefer {
        for (subscribers) |sub| sub.unref();
        allocator.free(subscribers);
    }
}

/// Deinitializes the tracing subsystem.
///
/// May fail if not all threads have been registered.
pub fn deinit() void {
    // Wait until all threads have been deregistered.
    if (ThreadData.get() != null) ThreadData.cleanup();
    thread_count.waitUntilZero();
    call_stack_count.waitUntilZero();

    for (subscribers) |subs| subs.unref();
    allocator.free(subscribers);
}

/// Creates a new empty call stack.
///
/// If successful, the new call stack is marked as suspended. The new call stack is not set to be
/// the active call stack.
pub fn createCallStack() pub_tracing.CallStack {
    if (!isEnabled()) return CallStack.dummy_call_stack;
    const call_stack = CallStack.init();
    return call_stack.asProxy();
}

/// Marks the current call stack as being suspended.
///
/// While suspended, the call stack can not be utilized for tracing messages. The call stack
/// optionally also be marked as being blocked. In that case, the call stack must be unblocked
/// prior to resumption.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub fn suspendCurrentCallStack(mark_blocked: bool) void {
    if (!isEnabled()) return;
    if (!isEnabledForCurrentThread()) @panic(@errorName(error.ThreadNotRegistered));
    const data = ThreadData.get().?;
    return data.call_stack.@"suspend"(mark_blocked);
}

/// Marks the current call stack as being resumed.
///
/// Once resumed, the context can be used to trace messages. To be successful, the current call
/// stack must be suspended and unblocked.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub fn resumeCurrentCallStack() void {
    if (!isEnabled()) return;
    if (!isEnabledForCurrentThread()) @panic(@errorName(error.ThreadNotRegistered));
    const data = ThreadData.get().?;
    return data.call_stack.@"resume"();
}

/// Creates a new span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpan(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    level: pub_tracing.Level,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    const desc = &struct {
        var desc = pub_tracing.SpanDesc{
            .metadata = &.{
                .name = name orelse location.fn_name,
                .target = target orelse location.module,
                .level = level,
                .file_name = location.file,
                .line_number = @intCast(location.line),
            },
        };
    }.desc;
    return pushSpanCustom(desc, pub_tracing.stdFormatter(fmt, @TypeOf(args)), &args);
}

/// Creates a new error span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanErr(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpan(fmt, args, name, target, .err, location);
}

/// Creates a new warn span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanWarn(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpan(fmt, args, name, target, .warn, location);
}

/// Creates a new info span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanInfo(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpan(fmt, args, name, target, .info, location);
}

/// Creates a new debug span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanDebug(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpan(fmt, args, name, target, .debug, location);
}

/// Creates a new trace span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanTrace(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpan(fmt, args, name, target, .trace, location);
}

/// Creates a new error span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanErrSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpanErr(fmt, args, null, null, location);
}

/// Creates a new warn span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanWarnSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpanWarn(fmt, args, null, null, location);
}

/// Creates a new info span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanInfoSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpanInfo(fmt, args, null, null, location);
}

/// Creates a new debug span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanDebugSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpanDebug(fmt, args, null, null, location);
}

/// Creates a new trace span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanTraceSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) pub_tracing.Span {
    return pushSpanTrace(fmt, args, null, null, location);
}

/// Creates a new span with a custom formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The
/// subsystem may use a formatting buffer of a fixed size. The formatter is expected to cut-of the
/// message after reaching that specified size. The `desc` must remain valid until the span is
/// destroyed.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub fn pushSpanCustom(
    desc: *const pub_tracing.SpanDesc,
    formatter: *const pub_tracing.Formatter,
    data: ?*const anyopaque,
) pub_tracing.Span {
    if (!isEnabled()) return StackFrame.dummy_span;
    if (!isEnabledForCurrentThread()) @panic(@errorName(error.ThreadNotRegistered));
    const d = ThreadData.get().?;
    return d.call_stack.pushSpan(desc, formatter, data);
}

/// Emits a new event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitEvent(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    level: pub_tracing.Level,
    location: std.builtin.SourceLocation,
) void {
    const event = pub_tracing.Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = level,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    emitEventCustom(&event, pub_tracing.stdFormatter(fmt, @TypeOf(args)), &args);
}

/// Emits a new error event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitErr(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
    return emitEvent(fmt, args, name, target, .err, location);
}

/// Emits a new warn event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitWarn(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
    return emitEvent(fmt, args, name, target, .warn, location);
}

/// Emits a new info event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitInfo(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
    return emitEvent(fmt, args, name, target, .info, location);
}

/// Emits a new debug event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitDebug(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
    return emitEvent(fmt, args, name, target, .debug, location);
}

/// Emits a new trace event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitTrace(
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
    return emitEvent(fmt, args, name, target, .trace, location);
}

/// Emits a new error event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitErrSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return emitErr(fmt, args, null, null, location);
}

/// Emits a new warn event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitWarnSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return emitWarn(fmt, args, null, null, location);
}

/// Emits a new info event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitInfoSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return emitInfo(fmt, args, null, null, location);
}

/// Emits a new debug event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitDebugSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return emitDebug(fmt, args, null, null, location);
}

/// Emits a new trace event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitTraceSimple(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    return emitTrace(fmt, args, null, null, location);
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitStackTrace(
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    stack_trace: std.builtin.StackTrace,
    location: std.builtin.SourceLocation,
) void {
    const event = pub_tracing.Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = .err,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    emitEventCustom(&event, pub_tracing.stackTraceFormatter, &stack_trace);
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitStackTraceSimple(
    stack_trace: std.builtin.StackTrace,
    location: std.builtin.SourceLocation,
) void {
    return emitStackTrace(null, null, stack_trace, location);
}

/// Emits a new event with a custom formatter.
///
/// The subsystem may use a formatting buffer of a fixed size. The formatter is expected to cut-of
/// the message after reaching that specified size.
pub fn emitEventCustom(
    event: *const pub_tracing.Event,
    formatter: *const pub_tracing.Formatter,
    data: ?*const anyopaque,
) void {
    if (!wouldTrace(event.metadata)) return;
    const d = ThreadData.get().?;
    d.call_stack.emitEvent(event, formatter, data);
}

/// Returns whether the subsystem was configured to enable tracing.
///
/// This is the case if there are any subscribers and the trace level is not `off`.
pub fn isEnabled() bool {
    return !(max_level == .off or subscribers.len == 0);
}

/// Returns whether the subsystem is configured to trace the current thread.
///
/// In addition to requiring the correctt configuration of the subsystem, this also requires that
/// the current thread be registered.
pub fn isEnabledForCurrentThread() bool {
    return isEnabled() and ThreadData.get() != null;
}

/// Checks whether an event or span with the provided metadata would lead to a tracing operation.
pub fn wouldTrace(metadata: *const pub_tracing.Metadata) bool {
    if (!isEnabledForCurrentThread()) return false;
    return @intFromEnum(max_level) >= @intFromEnum(metadata.level);
}

/// Tries to register the current thread with the subsystem.
///
/// Upon registration, the current thread is assigned a new tracing call stack.
pub fn registerThread() void {
    if (!isEnabled()) return;
    ThreadData.init();
}

/// Tries to unregister the current thread from the subsystem.
///
/// May fail if the call stack of the thread is not empty.
pub fn unregisterThread() void {
    if (!isEnabled()) return;
    ThreadData.cleanup();
}

/// Flushes all tracing messages from the subscribers.
pub fn flush() void {
    if (!isEnabled()) return;
    for (subscribers) |sub| {
        sub.flush();
    }
}

pub const ThreadData = struct {
    call_stack: *CallStack,
    ref_count: usize = 1,

    fn init() void {
        const thread_data = context.ThreadData.getOrInit();
        if (thread_data.tracing) |*this| {
            this.ref_count += 1;
            return;
        }

        const this = ThreadData{
            .call_stack = CallStack.init(),
        };
        this.call_stack.bind();
        this.call_stack.@"resume"();
        thread_data.tracing = this;
        thread_count.increase();
    }

    fn cleanup() void {
        const thread_data = context.ThreadData.get() orelse @panic("thread not registered");
        const this = &(thread_data.tracing orelse @panic(@panic("thread not registered")));
        this.ref_count -= 1;
        if (this.ref_count > 0) return;

        this.call_stack.deinit();
        thread_data.tracing = null;
        thread_count.decrease();
    }

    pub fn get() ?*ThreadData {
        const thread_data = context.ThreadData.get() orelse return null;
        if (thread_data.tracing) |*tr| return tr;
        return null;
    }

    pub fn onThreadExit(self: *ThreadData) void {
        self.ref_count -= 1;
        std.debug.assert(self.ref_count == 0);
        self.call_stack.deinit();
        thread_count.decrease();
    }
};

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn createCallStack() callconv(.c) pub_tracing.CallStack {
        std.debug.assert(context.is_init);
        return tracing.createCallStack();
    }
    fn suspendCurrentCallStack(mark_blocked: bool) callconv(.c) void {
        std.debug.assert(context.is_init);
        tracing.suspendCurrentCallStack(mark_blocked);
    }
    fn resumeCurrentCallStack() callconv(.c) void {
        std.debug.assert(context.is_init);
        tracing.resumeCurrentCallStack();
    }
    fn pushSpan(
        desc: *const pub_tracing.SpanDesc,
        formatter: *const pub_tracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.c) pub_tracing.Span {
        std.debug.assert(context.is_init);
        return tracing.pushSpanCustom(desc, formatter, data);
    }
    fn emitEvent(
        event: *const pub_tracing.Event,
        formatter: *const pub_tracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.c) void {
        std.debug.assert(context.is_init);
        tracing.emitEventCustom(event, formatter, data);
    }
    fn isEnabled() callconv(.c) bool {
        std.debug.assert(context.is_init);
        return tracing.isEnabled();
    }
    fn registerThread() callconv(.c) void {
        std.debug.assert(context.is_init);
        tracing.registerThread();
    }
    fn unregisterThread() callconv(.c) void {
        std.debug.assert(context.is_init);
        tracing.unregisterThread();
    }
    fn flush() callconv(.c) void {
        std.debug.assert(context.is_init);
        tracing.flush();
    }
};

pub const vtable = pub_tracing.VTable{
    .create_call_stack = &VTableImpl.createCallStack,
    .suspend_current_call_stack = &VTableImpl.suspendCurrentCallStack,
    .resume_current_call_stack = &VTableImpl.resumeCurrentCallStack,
    .span_create = &VTableImpl.pushSpan,
    .event_emit = &VTableImpl.emitEvent,
    .is_enabled = &VTableImpl.isEnabled,
    .register_thread = &VTableImpl.registerThread,
    .unregister_thread = &VTableImpl.unregisterThread,
    .flush = &VTableImpl.flush,
};
