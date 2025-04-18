const std = @import("std");
const Allocator = std.mem.Allocator;

const c = @import("../c.zig");
const Context = @import("../context.zig");
const time = @import("../time.zig");
const ProxyTracing = @import("proxy_context/tracing.zig");
const tls = @import("tls.zig");
const CallStack = @import("tracing/CallStack.zig");
const StackFrame = @import("tracing/StackFrame.zig");

const Tracing = @This();

allocator: Allocator,
subscribers: []ProxyTracing.Subscriber,
buffer_size: usize,
max_level: ProxyTracing.Level,
thread_data: tls.Tls(ThreadData),
thread_count: std.atomic.Value(usize),

/// Initializes the tracing subsystem.
pub fn init(allocator: Allocator, config: ?*const ProxyTracing.Config) (Allocator.Error || tls.TlsError)!Tracing {
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
        for (self.subscribers) |sub| sub.ref();
        self.buffer_size = if (cfg.format_buffer_len != 0) cfg.format_buffer_len else 1024;
        self.max_level = cfg.max_level;
    } else {
        self.subscribers = try allocator.dupe(ProxyTracing.Subscriber, &.{});
        self.buffer_size = 0;
        self.max_level = .off;
    }
    errdefer {
        for (self.subscribers) |sub| sub.unref();
        allocator.free(self.subscribers);
    }
    self.thread_data = try tls.Tls(ThreadData).init(ThreadData.deinit);
    errdefer self.thread_data.deinit();
    self.thread_count.store(0, .unordered);

    return self;
}

/// Deinitializes the tracing subsystem.
///
/// May fail if not all threads have been registered.
pub fn deinit(self: *Tracing) void {
    if (self.thread_data.get()) |data| {
        data.deinit();
        self.thread_data.set(null) catch unreachable;
    }

    // Use an acquire load to synchronize with the release in deinit.
    const num_threads = self.thread_count.load(.acquire);
    std.debug.assert(num_threads == 0);

    self.thread_data.deinit();
    for (self.subscribers) |subs| subs.unref();
    self.allocator.free(self.subscribers);
}

pub fn asContext(self: *Tracing) *Context {
    return @fieldParentPtr("tracing", self);
}

/// Creates a new empty call stack.
///
/// If successful, the new call stack is marked as suspended. The new call stack is not set to be
/// the active call stack.
pub fn createCallStack(self: *Tracing) ProxyTracing.CallStack {
    if (!self.isEnabled()) return CallStack.dummy_call_stack;
    const call_stack = CallStack.init(self);
    return call_stack.asProxy();
}

/// Marks the current call stack as being suspended.
///
/// While suspended, the call stack can not be utilized for tracing messages. The call stack
/// optionally also be marked as being blocked. In that case, the call stack must be unblocked
/// prior to resumption.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub fn suspendCurrentCallStack(self: *const Tracing, mark_blocked: bool) void {
    if (!self.isEnabled()) return;
    if (!self.isEnabledForCurrentThread()) @panic(@errorName(error.ThreadNotRegistered));
    const data = self.thread_data.get().?;
    return data.call_stack.@"suspend"(mark_blocked);
}

/// Marks the current call stack as being resumed.
///
/// Once resumed, the context can be used to trace messages. To be successful, the current call
/// stack must be suspended and unblocked.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub fn resumeCurrentCallStack(self: *const Tracing) void {
    if (!self.isEnabled()) return;
    if (!self.isEnabledForCurrentThread()) @panic(@errorName(error.ThreadNotRegistered));
    const data = self.thread_data.get().?;
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
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    level: ProxyTracing.Level,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
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
    );
}

/// Creates a new error span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanErr(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .err,
        location,
    );
}

/// Creates a new warn span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanWarn(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .warn,
        location,
    );
}

/// Creates a new info span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanInfo(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .info,
        location,
    );
}

/// Creates a new debug span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanDebug(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .debug,
        location,
    );
}

/// Creates a new trace span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanTrace(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpan(
        fmt,
        args,
        name,
        target,
        .trace,
        location,
    );
}

/// Creates a new error span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanErrSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpanErr(
        fmt,
        args,
        null,
        null,
        location,
    );
}

/// Creates a new warn span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanWarnSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpanWarn(
        fmt,
        args,
        null,
        null,
        location,
    );
}

/// Creates a new info span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanInfoSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpanInfo(
        fmt,
        args,
        null,
        null,
        location,
    );
}

/// Creates a new debug span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanDebugSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpanDebug(
        fmt,
        args,
        null,
        null,
        location,
    );
}

/// Creates a new trace span with the default formatter and enters it.
///
/// If successful, the newly created span is used as the context for succeeding events. The message
/// is formatted with the default formatter of the zig standard library. The message may be cut of,
/// if the length exceeds the internal formatting buffer size.
///
/// This function may panic, if the current thread is not registered with the subsystem.
pub inline fn pushSpanTraceSimple(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) ProxyTracing.Span {
    return self.pushSpanTrace(
        fmt,
        args,
        null,
        null,
        location,
    );
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
    self: *const Tracing,
    desc: *const ProxyTracing.SpanDesc,
    formatter: *const ProxyTracing.Formatter,
    data: ?*const anyopaque,
) ProxyTracing.Span {
    if (!self.isEnabled()) return StackFrame.dummy_span;
    if (!self.isEnabledForCurrentThread()) @panic(@errorName(error.ThreadNotRegistered));
    const d = self.thread_data.get().?;
    return d.call_stack.pushSpan(desc, formatter, data);
}

/// Emits a new event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitEvent(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    level: ProxyTracing.Level,
    location: std.builtin.SourceLocation,
) void {
    const event = ProxyTracing.Event{
        .metadata = &.{
            .name = name orelse location.fn_name,
            .target = target orelse location.module,
            .level = level,
            .file_name = location.file,
            .line_number = @intCast(location.line),
        },
    };
    self.emitEventCustom(
        &event,
        ProxyTracing.stdFormatter(fmt, @TypeOf(args)),
        &args,
    );
}

/// Emits a new error event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitErr(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
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
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitWarn(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
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
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitInfo(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
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
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitDebug(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
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
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitTrace(
    self: *const Tracing,
    comptime fmt: []const u8,
    args: anytype,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    location: std.builtin.SourceLocation,
) void {
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
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
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
    );
}

/// Emits a new warn event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
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
    );
}

/// Emits a new info event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
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
    );
}

/// Emits a new debug event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
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
    );
}

/// Emits a new trace event with the standard formatter.
///
/// The message is formatted using the default formatter of the zig standard library. The message
/// may be cut of, if the length exceeds the internal formatting buffer size.
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
    );
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal formatting buffer size.
pub fn emitStackTrace(
    self: *const Tracing,
    name: ?[:0]const u8,
    target: ?[:0]const u8,
    stack_trace: std.builtin.StackTrace,
    location: std.builtin.SourceLocation,
) void {
    const event = ProxyTracing.Event{
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
        ProxyTracing.stackTraceFormatter,
        &stack_trace,
    );
}

/// Emits a new error event dumping the stack trace.
///
/// The stack trace may be cut of, if the length exceeds the internal formatting buffer size.
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
    );
}

/// Emits a new event with a custom formatter.
///
/// The subsystem may use a formatting buffer of a fixed size. The formatter is expected to cut-of
/// the message after reaching that specified size.
pub fn emitEventCustom(
    self: *const Tracing,
    event: *const ProxyTracing.Event,
    formatter: *const ProxyTracing.Formatter,
    data: ?*const anyopaque,
) void {
    if (!self.wouldTrace(event.metadata)) return;
    const d = self.thread_data.get().?;
    d.call_stack.emitEvent(event, formatter, data);
}

/// Returns whether the subsystem was configured to enable tracing.
///
/// This is the case if there are any subscribers and the trace level is not `off`.
pub fn isEnabled(self: *const Tracing) bool {
    return !(self.max_level == .off or self.subscribers.len == 0);
}

/// Returns whether the subsystem is configured to trace the current thread.
///
/// In addition to requiring the correctt configuration of the subsystem, this also requires that
/// the current thread be registered.
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
pub fn registerThread(self: *Tracing) void {
    if (!self.isEnabled()) return;

    if (self.thread_data.get()) |data| {
        data.ref_count += 1;
        return;
    } else {
        const data = ThreadData.init(self);
        self.thread_data.set(data) catch |err| @panic(@errorName(err));
    }
}

/// Tries to unregister the current thread from the subsystem.
///
/// May fail if the call stack of the thread is not empty.
pub fn unregisterThread(self: *Tracing) void {
    if (!self.isEnabled()) return;

    const data = self.thread_data.get();
    if (data) |d| {
        d.ref_count -= 1;
        if (d.ref_count > 0) return;

        d.deinit();
        self.thread_data.set(null) catch |err| @panic(@errorName(err));
    } else @panic(@errorName(error.ThreadNotRegistered));
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
    ref_count: usize = 1,

    fn init(owner: *Tracing) *ThreadData {
        const data = owner.allocator.create(
            ThreadData,
        ) catch |err| @panic(@errorName(err));
        data.* = ThreadData{
            .call_stack = CallStack.init(owner),
            .owner = owner,
        };
        data.call_stack.bind();
        data.call_stack.@"resume"();

        // Is a counter so it does not need any synchronization.
        _ = owner.thread_count.fetchAdd(1, .monotonic);
        return data;
    }

    fn deinit(self: *ThreadData) callconv(.c) void {
        const owner = self.owner;
        self.call_stack.deinit();
        owner.allocator.destroy(self);

        // Synchronizes with the acquire on deinit of the context.
        _ = owner.thread_count.fetchSub(1, .release);
    }
};

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn createCallStack(ptr: *anyopaque) callconv(.c) ProxyTracing.CallStack {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        return ctx.tracing.createCallStack();
    }
    fn suspendCurrentCallStack(ptr: *anyopaque, mark_blocked: bool) callconv(.c) void {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.suspendCurrentCallStack(mark_blocked);
    }
    fn resumeCurrentCallStack(ptr: *anyopaque) callconv(.c) void {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.resumeCurrentCallStack();
    }
    fn pushSpan(
        ptr: *anyopaque,
        desc: *const ProxyTracing.SpanDesc,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.c) ProxyTracing.Span {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        return ctx.tracing.pushSpanCustom(desc, formatter, data);
    }
    fn emitEvent(
        ptr: *anyopaque,
        event: *const ProxyTracing.Event,
        formatter: *const ProxyTracing.Formatter,
        data: ?*const anyopaque,
    ) callconv(.c) void {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.emitEventCustom(event, formatter, data);
    }
    fn isEnabled(ptr: *anyopaque) callconv(.c) bool {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        return ctx.tracing.isEnabled();
    }
    fn registerThread(ptr: *anyopaque) callconv(.c) void {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.registerThread();
    }
    fn unregisterThread(ptr: *anyopaque) callconv(.c) void {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.unregisterThread();
    }
    fn flush(ptr: *anyopaque) callconv(.c) void {
        const ctx: *Context = @alignCast(@ptrCast(ptr));
        ctx.tracing.flush();
    }
};

pub const vtable = ProxyTracing.VTable{
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
