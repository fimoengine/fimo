const std = @import("std");
const Allocator = std.mem.Allocator;
const builtin = @import("builtin");

const context = @import("../context.zig");
const time = @import("../time.zig");
const Duration = time.Duration;
const Instant = time.Instant;
const Time = time.Time;
const pub_tracing = @import("../tracing.zig");
pub const Level = pub_tracing.Level;
pub const EventInfo = pub_tracing.EventInfo;
pub const Formatter = pub_tracing.Formatter;
pub const stdFormatter = pub_tracing.stdFormatter;
pub const stackTraceFormatter = pub_tracing.stackTraceFormatter;
pub const events = pub_tracing.events;
const ResourceCount = @import("ResourceCount.zig");
pub const CallStack = @import("tracing/CallStack.zig");
const Sampler = @import("tracing/Sampler.zig");

const tracing = @This();

pub var allocator: Allocator = undefined;
pub var subscribers: []pub_tracing.Subscriber = undefined;
pub var buffer_size: usize = undefined;
pub var max_level: pub_tracing.Level = undefined;
pub var event_info_cache: EventInfoCache = undefined;
pub var thread_count: ResourceCount = .{};
pub var call_stack_count: ResourceCount = .{};

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
        if (!isEnabled()) return;
        if (!isEnabledForCurrentThread()) @panic("thread not registered");
        const d = ThreadData.get().?;
        d.call_stack.enterFrame(self.id, formatter, formatter_data);
    }

    /// Exits an entered span.
    ///
    /// The events won't occur inside the context of the exited span anymore. The span must be the
    /// span at the top of the current call stack.
    pub fn exit(self: Span) void {
        if (!isEnabled()) return;
        if (!isEnabledForCurrentThread()) @panic("thread not registered");
        const d = ThreadData.get().?;
        d.call_stack.exitFrame(self.id);
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
            logMessage(&Global.embedded, formatter, formatter_data);
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

const MEMORYSTATUSEX = extern struct {
    dwLength: u32,
    dwMemoryLoad: u32,
    ullTotalPhys: u64,
    ullAvailPhys: u64,
    ullTotalPageFile: u64,
    ullAvailPageFile: u64,
    ullTotalVirtual: u64,
    ullAvailVirtual: u64,
    ullAvailExtendedVirtual: u64,
};
extern "kernel32" fn GlobalMemoryStatusEx(*MEMORYSTATUSEX) callconv(.winapi) std.os.windows.BOOL;

/// Initializes the tracing subsystem.
pub fn init(config: *const pub_tracing.Cfg) !void {
    allocator = context.allocator;
    const subs = config.subscribers.intoSliceOrEmpty();
    subscribers = try allocator.dupe(pub_tracing.Subscriber, subs);
    buffer_size = if (config.format_buffer_len != 0) config.format_buffer_len else 1024;
    max_level = config.max_level;
    event_info_cache = .{};
    errdefer allocator.free(subscribers);

    const resolution = blk: {
        var nanos: usize = std.math.maxInt(usize);
        for (0..500000) |_| {
            const t1 = Instant.now();
            const elapsed = t1.elapsed() catch unreachable;
            const elapsed_ns = elapsed.nanos();
            if (elapsed_ns > 0) nanos = @min(nanos, @as(usize, @intCast(elapsed_ns)));
        }
        break :blk Duration.initNanos(nanos).intoC();
    };
    const available_memory: usize = switch (comptime builtin.target.os.tag) {
        .windows => blk: {
            var status: MEMORYSTATUSEX = undefined;
            status.dwLength = @intCast(@sizeOf(MEMORYSTATUSEX));
            if (GlobalMemoryStatusEx(&status) == 0) break :blk 0;
            break :blk status.ullTotalPhys;
        },
        .ios, .macos, .tvos, .visionos, .watchos => blk: {
            var memsize: usize = undefined;
            var size: usize = @sizeOf(usize);
            if (std.c.sysctlbyname("hw.memsize", &memsize, &size, null, 0) != 0) break :blk 0;
            break :blk memsize;
        },
        .linux => blk: {
            var info: std.os.linux.Sysinfo = undefined;
            if (std.os.linux.sysinfo(&info) == -1) break :blk 0;
            break :blk info.totalram * info.mem_unit;
        },
        else => 0,
    };
    const process_id: usize = if (comptime builtin.target.os.tag == .windows)
        std.os.windows.GetCurrentProcessId()
    else
        @as(u32, @bitCast(std.c.getpid()));
    const num_cores = std.Thread.getCpuCount() catch 0;
    const cpu_arch: events.CpuArch = switch (comptime builtin.target.cpu.arch) {
        .x86_64 => .x86_64,
        .aarch64 => .aarch64,
        else => .unknown,
    };
    const cpu_id: u32, const cpu_vendor: [12]u8 = if (comptime builtin.target.cpu.arch == .x86_64) blk: {
        var eax: u32, var ebx: u32, var ecx: u32, var edx: u32 = .{ 0, 0, 0, 0 };
        asm volatile ("cpuid"
            : [ret1] "={eax}" (eax),
              [ret2] "={ebx}" (ebx),
              [ret3] "={ecx}" (ecx),
              [ret4] "={edx}" (edx),
            : [id] "{eax}" (0),
            : .{});
        const vendor: [12]u8 = @bitCast([_]u32{ ebx, edx, ecx });

        asm volatile ("cpuid"
            : [ret1] "={eax}" (eax),
            : [id] "{eax}" (1),
            : .{});
        const id = (eax & 0xFFF) | ((eax & 0xFFF0000) >> 4);
        break :blk .{ id, vendor };
    } else .{ 0, [_]u8{0} ** 12 };
    _ = .{ cpu_id, cpu_vendor };

    const buffer = struct {
        var buffer = [_]u8{0} ** 1024;
    }.buffer[0..];
    var writer = std.Io.Writer.fixed(buffer);

    // OS
    switch (comptime builtin.target.os.tag) {
        .windows => {
            var version: std.os.windows.OSVERSIONINFOW = undefined;
            version.dwOSVersionInfoSize = @sizeOf(std.os.windows.OSVERSIONINFOW);
            if (std.os.windows.ntdll.RtlGetVersion(&version) != .SUCCESS) {
                writer.writeAll("OS: Windows;") catch unreachable;
            } else {
                writer.print("OS: Windows {}.{}.{};", .{
                    version.dwMajorVersion,
                    version.dwMinorVersion,
                    version.dwBuildNumber,
                }) catch unreachable;
            }
        },
        .linux => {
            var utsname: std.os.linux.utsname = undefined;
            if (std.os.linux.uname(&utsname) != 0) unreachable;
            writer.print("OS: Linux {s};", .{utsname.release}) catch unreachable;
        },
        .ios => writer.writeAll("OS: Darwin (iOS);") catch unreachable,
        .macos => writer.writeAll("OS: Darwin (macOS);") catch unreachable,
        .tvos => writer.writeAll("OS: Darwin (tvOS);") catch unreachable,
        .visionos => writer.writeAll("OS: Darwin (visionOS);") catch unreachable,
        .watchos => writer.writeAll("OS: Darwin (watchOS);") catch unreachable,
        else => writer.writeAll("OS: unknown;") catch unreachable,
    }

    writer.print("Compiler: Zig {s};", .{builtin.zig_version_string}) catch unreachable;
    writer.print("Backend: {t};", .{builtin.zig_backend}) catch unreachable;
    writer.print("ABI: {t};", .{builtin.target.abi}) catch unreachable;
    writer.print("Arch: {t};", .{builtin.cpu.arch}) catch unreachable;
    writer.print("CPU cores: {};", .{num_cores}) catch unreachable;
    writer.print("RAM: {} MB", .{available_memory / 1024 / 1024}) catch unreachable;

    const now = Instant.now().intoC();
    const epoch = Time.now().intoC();
    for (subscribers) |subscriber| subscriber.start(.{
        .time = now,
        .epoch = epoch,
        .resolution = resolution,
        .available_memory = available_memory,
        .process_id = process_id,
        .num_cores = num_cores,
        .cpu_arch = cpu_arch,
        .cpu_id = cpu_id,
        .cpu_vendor = .fromSlice(&cpu_vendor),
        .app_name = config.app_name,
        .host_info = .fromSlice(writer.buffered()),
    });
    if (config.register_thread) registerThread();
    Sampler.start() catch |err| logWarn(@src(), "could not start tracing sampler: {t}", .{err});
}

/// Deinitializes the tracing subsystem.
///
/// May fail if not all threads have been registered.
pub fn deinit() void {
    // Wait until all threads have been deregistered.
    if (ThreadData.get() != null) ThreadData.cleanup();
    thread_count.waitUntilZero();
    call_stack_count.waitUntilZero();

    Sampler.stop();
    const now = Instant.now().intoC();
    for (subscribers) |subscriber| subscriber.finish(.{ .time = now });
    allocator.free(subscribers);
}

/// Returns whether the subsystem was configured to enable tracing.
///
/// This is the case if there are any subscribers and the trace level is not `off`.
pub fn isEnabled() bool {
    return !(max_level == .off or subscribers.len == 0);
}

/// Returns whether the subsystem is configured to trace the current thread.
///
/// In addition to requiring the correct configuration of the subsystem, this also requires that
/// the current thread be registered.
pub fn isEnabledForCurrentThread() bool {
    return isEnabled() and ThreadData.get() != null;
}

/// Checks whether an event with the provided info would lead to a tracing operation.
pub fn wouldTrace(info: *const EventInfo) bool {
    if (!isEnabledForCurrentThread()) return false;
    return @intFromEnum(max_level) >= @intFromEnum(info.level);
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

fn logMessage(
    info: *const EventInfo,
    formatter: *const Formatter,
    formatter_data: *const anyopaque,
) void {
    if (!wouldTrace(info)) return;
    const d = ThreadData.get().?;
    d.call_stack.logMessage(info, formatter, formatter_data);
}

pub const EventInfoCache = struct {
    cache: [cache_len]std.atomic.Value(?*const EventInfo) = @splat(.init(null)),
    const cache_len = 4096;

    /// Caches the element and returns whether it was inserted into the cache;
    pub fn cacheInfo(self: *EventInfoCache, info: *const EventInfo) bool {
        var hasher: std.hash.Wyhash = .init(0);
        std.hash.autoHash(&hasher, info);
        const hash = hasher.final();
        const idx = hash & (cache_len - 1);
        const old = self.cache[idx].swap(info, .monotonic);
        return old != info;
    }
};

pub const ThreadData = struct {
    call_stack: *CallStack,
    fmt_buffer: []u8,

    fn init() void {
        const thread_data = context.ThreadData.getOrInit();
        if (thread_data.tracing != null) @panic("thread already registered");

        const now = Instant.now().intoC();
        const thread_id = std.Thread.getCurrentId();
        for (subscribers) |subscriber| {
            subscriber.registerThread(.{
                .time = now,
                .thread_id = thread_id,
            });
        }

        const fmt_buffer = tracing.allocator.alloc(
            u8,
            tracing.buffer_size,
        ) catch |err| @panic(@errorName(err));
        const this = ThreadData{
            .call_stack = CallStack.initBound(fmt_buffer),
            .fmt_buffer = fmt_buffer,
        };
        thread_data.tracing = this;
        thread_count.increase();
    }

    fn cleanup() void {
        const thread_data = context.ThreadData.get() orelse @panic("thread not registered");
        const this = &(thread_data.tracing orelse @panic(@panic("thread not registered")));

        this.call_stack.finishBound();

        const now = Instant.now().intoC();
        const thread_id = std.Thread.getCurrentId();
        for (subscribers) |subscriber| {
            subscriber.unregisterThread(.{
                .time = now,
                .thread_id = thread_id,
            });
        }

        tracing.allocator.free(this.fmt_buffer);
        thread_data.tracing = null;
        thread_count.decrease();
    }

    pub fn get() ?*ThreadData {
        const thread_data = context.ThreadData.get() orelse return null;
        if (thread_data.tracing) |*tr| return tr;
        return null;
    }

    pub fn onThreadExit(self: *ThreadData) void {
        self.call_stack.finishBound();
        thread_count.decrease();
    }
};

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
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
    fn initCallStack() callconv(.c) *pub_tracing.CallStack {
        std.debug.assert(context.is_init);
        const stack = CallStack.init();
        return @ptrCast(stack);
    }
    fn deinitCallStack(stack: *pub_tracing.CallStack, abort: bool) callconv(.c) void {
        std.debug.assert(context.is_init);
        const stack_: *CallStack = @ptrCast(@alignCast(stack));
        if (abort) stack_.abort() else stack_.finish();
    }
    fn replaceCurrentCallStack(stack: *pub_tracing.CallStack) callconv(.c) *pub_tracing.CallStack {
        std.debug.assert(context.is_init);
        const stack_: *CallStack = @ptrCast(@alignCast(stack));
        const old = stack_.swapCurrent();
        return @ptrCast(old);
    }
    fn unblockCallStack(stack: *pub_tracing.CallStack) callconv(.c) void {
        std.debug.assert(context.is_init);
        const stack_: *CallStack = @ptrCast(@alignCast(stack));
        stack_.unblock();
    }
    fn suspendCurrentCallStack(mark_blocked: bool) callconv(.c) void {
        std.debug.assert(context.is_init);
        CallStack.suspendCurrent(mark_blocked);
    }
    fn resumeCurrentCallStack() callconv(.c) void {
        std.debug.assert(context.is_init);
        CallStack.resumeCurrent();
    }
    fn enterSpan(
        info: *const EventInfo,
        formatter: *const Formatter,
        formatter_data: *const anyopaque,
    ) callconv(.c) void {
        std.debug.assert(context.is_init);
        (Span{ .id = info }).enter(formatter, formatter_data);
    }
    fn exitSpan(info: *const EventInfo) callconv(.c) void {
        std.debug.assert(context.is_init);
        (Span{ .id = info }).exit();
    }
    fn logMessage(
        info: *const EventInfo,
        formatter: *const Formatter,
        formatter_data: *const anyopaque,
    ) callconv(.c) void {
        std.debug.assert(context.is_init);
        tracing.logMessage(info, formatter, formatter_data);
    }
};

pub const vtable = pub_tracing.VTable{
    .is_enabled = &VTableImpl.isEnabled,
    .register_thread = &VTableImpl.registerThread,
    .unregister_thread = &VTableImpl.unregisterThread,
    .init_call_stack = &VTableImpl.initCallStack,
    .deinit_call_stack = &VTableImpl.deinitCallStack,
    .replace_current_call_stack = &VTableImpl.replaceCurrentCallStack,
    .unblock_call_stack = &VTableImpl.unblockCallStack,
    .suspend_current_call_stack = &VTableImpl.suspendCurrentCallStack,
    .resume_current_call_stack = &VTableImpl.resumeCurrentCallStack,
    .enter_span = &VTableImpl.enterSpan,
    .exit_span = &VTableImpl.exitSpan,
    .log_message = &VTableImpl.logMessage,
};
