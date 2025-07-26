//! Internal inteface of the fimo std library.
const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;
const SinglyLinkedList = std.SinglyLinkedList;
const builtin = @import("builtin");

const AnyError = @import("AnyError.zig");
const AnyResult = AnyError.AnyResult;
const modules = @import("context/modules.zig");
const tasks = @import("context/tasks.zig");
const tracing = @import("context/tracing.zig");
const pub_ctx = @import("ctx.zig");
const pub_modules = @import("modules.zig");
const pub_tracing = @import("tracing.zig");
const Version = @import("Version.zig");

const Self = @This();

var lock: std.Thread.Mutex = .{};
pub var is_init: bool = false;

var result_count: std.atomic.Value(u32) = .init(0);
threadlocal var result: AnyResult = .ok;

const waiting_on_result_cleanup: u32 = 1 << 31;
const result_count_mask: u32 = ~waiting_on_result_cleanup;

var debug_allocator = switch (builtin.mode) {
    .Debug, .ReleaseSafe => std.heap.DebugAllocator(.{}).init,
    else => {},
};
pub var allocator = switch (builtin.mode) {
    .Debug, .ReleaseSafe => debug_allocator.allocator(),
    else => std.heap.smp_allocator,
};

const WindowsThreadExitHandler = struct {
    threadlocal var registered: bool = false;
    export var thread_exit_callback: std.os.windows.PIMAGE_TLS_CALLBACK linksection(".CRT$XLB") = @ptrCast(&tss_callback);

    fn init() !void {}
    fn tss_callback(
        h: ?std.os.windows.PVOID,
        dwReason: std.os.windows.DWORD,
        pv: ?std.os.windows.PVOID,
    ) callconv(.winapi) void {
        _ = h;
        _ = pv;

        const DLL_PROCESS_DETACH = 0;
        const DLL_THREAD_DETACH = 3;
        if (registered and (dwReason == DLL_PROCESS_DETACH or dwReason == DLL_THREAD_DETACH)) {
            onThreadExit();
        }
    }
    pub fn ensureRegistered() void {
        registered = true;
    }
};

const PosixThreadExitHandler = struct {
    var key_is_init: bool = false;
    var key: std.c.pthread_key_t = undefined;

    fn init() !void {
        if (key_is_init) return;
        switch (std.c.pthread_key_create(&key, &dtor)) {
            .SUCCESS => {
                key_is_init = true;
                return;
            },
            .AGAIN => return error.TlsSlotsQuotaExceeded,
            .NOMEM => return error.SystemResources,
            else => |err| return std.posix.unexpectedErrno(err),
        }
    }
    fn dtor(ptr: *anyopaque) callconv(.c) void {
        _ = ptr;
        onThreadExit();
    }
    pub fn ensureRegistered() void {
        const status: std.c.E = @enumFromInt(std.c.pthread_setspecific(key, @ptrFromInt(1)));
        switch (status) {
            .SUCCESS => return,
            .INVAL => unreachable,
            .NOMEM => @panic(@errorName(error.SystemResources)),
            else => |err| @panic(@errorName(std.posix.unexpectedErrno(err))),
        }
    }
};

const ThreadExitHandler = if (builtin.os.tag == .windows)
    WindowsThreadExitHandler
else if (builtin.link_libc)
    PosixThreadExitHandler
else
    @compileError("unsupported target");

pub fn init(options: [:null]const ?*const pub_ctx.ConfigHead) !void {
    lock.lock();
    defer lock.unlock();
    if (is_init) return error.AlreadyInitialized;

    errdefer switch (builtin.mode) {
        .Debug, .ReleaseSafe => _ = {
            if (debug_allocator.deinit() == .leak) @panic("memory leak");
            debug_allocator = .init;
            allocator = debug_allocator.allocator();
        },
        else => {},
    };
    try ThreadExitHandler.init();

    var tracing_cfg: ?*const pub_tracing.Config = null;
    var modules_cfg: ?*const pub_modules.Config = null;
    for (options) |opt| {
        const o = if (opt) |o| o else return error.InvalidInput;
        switch (o.id) {
            .tracing => {
                if (tracing_cfg != null) return error.InvalidInput;
                tracing_cfg = @alignCast(@ptrCast(o));
            },
            .modules => {
                if (modules_cfg != null) return error.InvalidInput;
                modules_cfg = @alignCast(@ptrCast(o));
            },
            else => return error.InvalidInput,
        }
    }

    try tracing.init(tracing_cfg orelse &.{});
    errdefer tracing.deinit();

    tracing.registerThread();
    defer tracing.unregisterThread();

    try tasks.init();
    errdefer tasks.deinit();

    try modules.init(modules_cfg orelse &.{});
    errdefer modules.deinit();

    is_init = true;
}

pub fn deinit() void {
    lock.lock();
    defer lock.unlock();
    std.debug.assert(is_init);

    // Might not actually trace anything, since all threads may be unregistered.
    // It's for just in case, that the calling thread did not unregister itself.
    tracing.emitTraceSimple("cleaning up context", .{}, @src());

    modules.deinit();
    tasks.deinit();
    tracing.deinit();

    clearResult();
    while (true) {
        const count = result_count.load(.acquire);
        if (count & result_count_mask == 0) break;
        if (count & waiting_on_result_cleanup == 0) {
            _ = result_count.cmpxchgWeak(
                count,
                count | waiting_on_result_cleanup,
                .monotonic,
                .monotonic,
            ) orelse continue;
        }
        lock.unlock();
        std.Thread.Futex.wait(&result_count, count | waiting_on_result_cleanup);
        lock.lock();
    }
    result_count.store(0, .monotonic);

    switch (builtin.mode) {
        .Debug, .ReleaseSafe => _ = {
            // if (debug_allocator.deinit() == .leak) @panic("memory leak");
            if (debug_allocator.deinit() == .leak) {}
            debug_allocator = .init;
            allocator = debug_allocator.allocator();
        },
        else => {},
    }
    is_init = false;
}

pub fn hasErrorResult() bool {
    std.debug.assert(is_init);
    return result.isErr();
}

pub fn replaceResult(with: AnyResult) AnyResult {
    std.debug.assert(is_init);
    const old = result;
    if (old.isOk() and with.isErr()) {
        ThreadExitHandler.ensureRegistered();
        _ = result_count.fetchAdd(1, .monotonic);
    } else if (old.isErr() and with.isOk()) {
        const count = result_count.fetchSub(1, .release) - 1;
        if (count == waiting_on_result_cleanup) std.Thread.Futex.wake(&result_count, std.math.maxInt(u32));
    }
    result = with;
    return old;
}

pub fn takeResult() AnyResult {
    return replaceResult(.ok);
}

pub fn clearResult() void {
    takeResult().deinit();
}

pub fn setResult(res: AnyResult) void {
    replaceResult(res).deinit();
}

fn onThreadExit() void {
    lock.lock();
    defer lock.unlock();
    if (!is_init) return;
    clearResult();
    tracing.onThreadExit();
}

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const HandleImpl = struct {
    fn getVersion() callconv(.c) Version.CVersion {
        return pub_ctx.context_version.intoC();
    }
    fn deinit() callconv(.c) void {
        Self.deinit();
    }
    fn hasErrorResult() callconv(.c) bool {
        return Self.hasErrorResult();
    }
    fn replaceResult(with: AnyResult) callconv(.c) AnyResult {
        return Self.replaceResult(with);
    }
};

pub const handle = pub_ctx.Handle{
    .get_version = &HandleImpl.getVersion,
    .core_v0 = .{
        .deinit = &HandleImpl.deinit,
        .has_error_result = &HandleImpl.hasErrorResult,
        .replace_result = &HandleImpl.replaceResult,
    },
    .tracing_v0 = tracing.vtable,
    .modules_v0 = modules.vtable,
    .tasks_v0 = tasks.vtable,
};

comptime {
    if (builtin.is_test) {
        _ = @import("context/graph.zig");
        _ = @import("context/modules.zig");
        _ = @import("context/RefCount.zig");
        _ = @import("context/tmp_path.zig");
        _ = @import("context/tracing.zig");
    }
}

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    export fn fimo_context_init(options: [*:null]const ?*const pub_ctx.ConfigHead, h: **const pub_ctx.Handle) AnyResult {
        init(std.mem.span(options)) catch |err| return AnyError.initError(err).intoResult();
        h.* = &handle;
        return AnyResult.ok;
    }
};

comptime {
    _ = ffi;
}
