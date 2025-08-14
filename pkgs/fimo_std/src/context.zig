//! Internal inteface of the fimo std library.
const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;
const SinglyLinkedList = std.SinglyLinkedList;
const builtin = @import("builtin");

const AnyError = @import("AnyError.zig");
const AnyResult = AnyError.AnyResult;
const modules = @import("context/modules.zig");
const ResourceCount = @import("context/ResourceCount.zig");
const tasks = @import("context/tasks.zig");
const tracing = @import("context/tracing.zig");
const pub_ctx = @import("ctx.zig");
const pub_modules = @import("modules.zig");
const pub_tracing = @import("tracing.zig");
const Version = @import("Version.zig");

const Self = @This();

var lock: std.Thread.Mutex = .{};
pub var is_init: bool = false;

var result_count: ResourceCount = .{};

var debug_allocator = switch (builtin.mode) {
    .Debug, .ReleaseSafe => std.heap.DebugAllocator(.{}).init,
    else => {},
};
pub var allocator = switch (builtin.mode) {
    .Debug, .ReleaseSafe => debug_allocator.allocator(),
    else => std.heap.smp_allocator,
};

pub const ThreadData = struct {
    result: AnyResult = .ok,
    tracing: ?tracing.ThreadData = null,
    node: std.SinglyLinkedList.Node = .{},

    pub fn get() ?*ThreadData {
        return Impl.get();
    }

    pub fn getOrInit() *ThreadData {
        return Impl.getOrInit();
    }

    fn replaceResult(self: *ThreadData, with: AnyResult) AnyResult {
        const old = self.result;
        if (old.isOk() and with.isErr())
            result_count.increase()
        else if (old.isErr() and with.isOk())
            result_count.decrease();
        self.result = with;
        return old;
    }

    fn onThreadExit(self: *ThreadData) void {
        self.replaceResult(.ok).deinit();
        if (self.tracing) |*tr| {
            tr.onThreadExit();
            self.tracing = null;
        }
    }

    const WindowsImpl = struct {
        threadlocal var data: ?ThreadData = null;
        export var thread_data_on_exit: std.os.windows.PIMAGE_TLS_CALLBACK linksection(".CRT$XLB") = @ptrCast(&tss_callback);
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
            if (dwReason == DLL_PROCESS_DETACH or dwReason == DLL_THREAD_DETACH) if (data) |*d| ThreadData.onThreadExit(d);
        }
        fn get() ?*ThreadData {
            if (data) |*d| return d;
            return null;
        }
        fn getOrInit() *ThreadData {
            if (data == null) data = .{};
            return if (data) |*d| d else unreachable;
        }
    };

    const PosixImpl = struct {
        var key_is_init: bool = false;
        var key: std.c.pthread_key_t = undefined;
        var nodes: [512]ThreadData = [_]ThreadData{.{}} ** 512;
        var linked_list: std.SinglyLinkedList = .{};
        var list_lock: std.Thread.Mutex = .{};
        threadlocal var cache: ?*ThreadData = null;

        fn init() !void {
            if (key_is_init) return;
            switch (std.c.pthread_key_create(&key, &dtor)) {
                .SUCCESS => {},
                .AGAIN => return error.TlsSlotsQuotaExceeded,
                .NOMEM => return error.SystemResources,
                else => |err| return std.posix.unexpectedErrno(err),
            }
            for (nodes[0..]) |*node| linked_list.prepend(&node.node);
            key_is_init = true;
        }
        fn dtor(ptr: *anyopaque) callconv(.c) void {
            const data: *ThreadData = @ptrCast(@alignCast(ptr));
            ThreadData.onThreadExit(data);
            list_lock.lock();
            defer list_lock.unlock();
            linked_list.prepend(&data.node);
        }
        fn get() ?*ThreadData {
            return cache;
        }
        fn getOrInit() *ThreadData {
            if (cache) |data| return data;
            list_lock.lock();
            defer list_lock.unlock();
            const node = linked_list.popFirst() orelse @panic("thread count exceeded");
            const data: *ThreadData = @fieldParentPtr("node", node);
            const status: std.c.E = @enumFromInt(std.c.pthread_setspecific(key, data));
            switch (status) {
                .SUCCESS => cache = data,
                .INVAL => unreachable,
                .NOMEM => @panic("oom"),
                else => |err| @panic(@errorName(std.posix.unexpectedErrno(err))),
            }
            return data;
        }
    };

    const Impl = if (builtin.os.tag == .windows)
        WindowsImpl
    else if (builtin.link_libc)
        PosixImpl
    else
        @compileError("unsupported target");
};

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
    try ThreadData.Impl.init();

    var tracing_cfg: ?*const pub_tracing.Config = null;
    var modules_cfg: ?*const pub_modules.Config = null;
    for (options) |opt| {
        const o = if (opt) |o| o else return error.InvalidInput;
        switch (o.id) {
            .tracing => {
                if (tracing_cfg != null) return error.InvalidInput;
                tracing_cfg = @ptrCast(@alignCast(o));
            },
            .modules => {
                if (modules_cfg != null) return error.InvalidInput;
                modules_cfg = @ptrCast(@alignCast(o));
            },
            else => return error.InvalidInput,
        }
    }

    try tracing.init(tracing_cfg orelse &.{});
    errdefer tracing.deinit();

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
    tracing.logTrace(@src(), "cleaning up context", .{});

    modules.deinit();
    tasks.deinit();
    tracing.deinit();

    clearResult();
    result_count.waitUntilZero();

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
    if (ThreadData.get()) |data| return data.result.isErr();
    return false;
}

pub fn replaceResult(with: AnyResult) AnyResult {
    std.debug.assert(is_init);
    if (with.isOk()) {
        const data = ThreadData.get() orelse return .ok;
        return data.replaceResult(with);
    } else {
        const data = ThreadData.getOrInit();
        return data.replaceResult(with);
    }
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
