//! Internal inteface of the fimo std library.
const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;
const SinglyLinkedList = std.SinglyLinkedList;

const c = @import("c");

const AnyError = @import("AnyError.zig");
const AnyResult = AnyError.AnyResult;
const Async = @import("context/async.zig");
const Module = @import("context/module.zig");
const RefCount = @import("context/RefCount.zig");
const Tls = @import("context/tls.zig").Tls;
const Tracing = @import("context/tracing.zig");
const pub_ctx = @import("ctx.zig");
const pub_modules = @import("modules.zig");
const pub_tracing = @import("tracing.zig");
const Version = @import("Version.zig");

const GPA = std.heap.GeneralPurposeAllocator(.{});
const Self = @This();

gpa: GPA,
allocator: Allocator,
refcount: RefCount = .{},
result_list: *ResultList,
tracing: Tracing,
module: Module,
async: Async,

var lock: std.Thread.Mutex = .{};
pub var global: Self = undefined;
pub var is_init: bool = false;

const ResultList = struct {
    refcount: RefCount = .{},
    local_result: Tls(LocalResult),

    const LocalResult = struct {
        result: AnyResult = .ok,
        list: *ResultList,
    };

    fn init() !*ResultList {
        const list = try std.heap.c_allocator.create(ResultList);
        list.* = .{ .local_result = undefined };
        errdefer std.heap.c_allocator.destroy(list);

        list.local_result = try Tls(LocalResult).init(&struct {
            fn f(res: *LocalResult) callconv(.c) void {
                const l = res.list;
                res.result.deinit();
                std.heap.c_allocator.destroy(res);
                l.unref();
            }
        }.f);

        return list;
    }

    fn unrefDestroy(self: *ResultList) void {
        if (self.local_result.get()) |l| {
            l.result.deinit();
            std.heap.c_allocator.destroy(l);
            self.local_result.set(null) catch unreachable;
        }
        if (self.refcount.unref() == .noop) return;
        self.local_result.deinit();
        std.heap.c_allocator.destroy(self);
    }

    fn unref(self: *ResultList) void {
        if (self.refcount.unref() == .noop) return;
        self.local_result.deinit();
        std.heap.c_allocator.destroy(self);
    }

    fn replaceResult(self: *ResultList, with: AnyResult) AnyResult {
        const local = if (self.local_result.get()) |l| l else blk: {
            if (with.isOk()) return .ok;
            const l = std.heap.c_allocator.create(LocalResult) catch @panic("oom");
            l.* = .{ .list = self };

            self.refcount.ref();
            self.local_result.set(l) catch |err| @panic(@errorName(err));
            break :blk l;
        };
        const old = local.result;
        local.result = with;
        return old;
    }
};

pub fn init(options: [:null]const ?*const pub_ctx.ConfigHead) !void {
    lock.lock();
    defer lock.unlock();
    if (is_init) return error.AlreadyInitialized;

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

    global = Self{
        .gpa = GPA.init,
        .allocator = undefined,
        .result_list = undefined,
        .tracing = undefined,
        .module = undefined,
        .async = undefined,
    };
    global.allocator = global.gpa.allocator();

    global.result_list = try ResultList.init();
    errdefer global.result_list.unrefDestroy();

    global.tracing = try Tracing.init(global.allocator, tracing_cfg);
    errdefer global.tracing.deinit();

    global.tracing.registerThread();
    defer global.tracing.unregisterThread();

    global.module = try Module.init(&global, modules_cfg orelse &.{});
    errdefer global.module.deinit();

    global.async = try Async.init(&global);
    errdefer global.async.deinit();

    is_init = true;
}

pub fn deinit() void {
    lock.lock();
    defer lock.unlock();
    std.debug.assert(is_init);

    // Might not actually trace anything, since all threads may be unregistered.
    // It's for just in case, that the calling thread did not unregister itself.
    global.tracing.emitTraceSimple("cleaning up context", .{}, @src());

    global.async.deinit();
    global.module.deinit();
    global.tracing.deinit();
    global.result_list.unrefDestroy();
    _ = global.gpa.deinit();
    // if (global.gpa.deinit() == .leak) @panic("memory leak");

    is_init = false;
    global = undefined;
}

pub fn hasErrorResult() bool {
    return if (global.result_list.local_result.get()) |l| l.result.isErr() else false;
}

pub fn replaceResult(with: AnyResult) AnyResult {
    std.debug.assert(is_init);
    return global.result_list.replaceResult(with);
}

pub fn takeResult() AnyResult {
    return replaceResult(.ok);
}

pub fn setResult(result: AnyResult) void {
    replaceResult(result).deinit();
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
    .tracing_v0 = Tracing.vtable,
    .modules_v0 = Module.vtable,
    .tasks_v0 = Async.vtable,
};

test {
    _ = @import("context/graph.zig");
    _ = @import("context/module.zig");
    _ = @import("context/RefCount.zig");
    _ = @import("context/tls.zig");
    _ = @import("context/tmp_path.zig");
    _ = @import("context/tracing.zig");
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
