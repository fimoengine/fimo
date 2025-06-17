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
pub const ProxyContext = @import("context/proxy_context.zig");
pub const ProxyModule = @import("context/proxy_context/module.zig");
pub const ProxyTracing = @import("context/proxy_context/tracing.zig");
const RefCount = @import("context/RefCount.zig");
const Tls = @import("context/tls.zig").Tls;
const Tracing = @import("context/tracing.zig");
const Version = @import("Version.zig");

const GPA = std.heap.GeneralPurposeAllocator(.{});
const Self = @This();

gpa: GPA,
allocator: Allocator,
refcount: RefCount = .{},
result_list: *ResultList,
tracing: Tracing,
module: Module,
@"async": Async,

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

pub fn init(options: [:null]const ?*const ProxyContext.TaggedInStruct) !*Self {
    var tracing_cfg: ?*const ProxyTracing.Config = null;
    var module_cfg: ?*const ProxyModule.Config = null;
    for (options) |opt| {
        const o = if (opt) |o| o else return error.InvalidInput;
        switch (o.id) {
            .tracing_config => {
                if (tracing_cfg != null) return error.InvalidInput;
                tracing_cfg = @alignCast(@ptrCast(o));
            },
            .module_config => {
                if (module_cfg != null) return error.InvalidInput;
                module_cfg = @alignCast(@ptrCast(o));
            },
            else => return error.InvalidInput,
        }
    }

    var gpa = GPA.init;
    errdefer if (gpa.deinit() == .leak) @panic("memory leak");

    const self = try gpa.allocator().create(Self);
    errdefer {
        gpa = self.gpa;
        gpa.allocator().destroy(self);
    }
    self.* = Self{
        .gpa = gpa,
        .allocator = undefined,
        .result_list = undefined,
        .tracing = undefined,
        .module = undefined,
        .@"async" = undefined,
    };
    self.allocator = self.gpa.allocator();

    self.result_list = try ResultList.init();
    errdefer self.result_list.unrefDestroy();

    self.tracing = try Tracing.init(self.allocator, tracing_cfg);
    errdefer self.tracing.deinit();

    self.tracing.registerThread();
    defer self.tracing.unregisterThread();

    self.module = try Module.init(self, module_cfg orelse &.{});
    errdefer self.module.deinit();

    self.@"async" = try Async.init(self);
    errdefer self.@"async".deinit();

    return self;
}

pub fn ref(self: *Self) void {
    self.refcount.ref();
}

pub fn unref(self: *Self) void {
    if (self.refcount.unref() == .noop) return;

    // Might not actually trace anything, since all threads may be unregistered.
    // It's for just in case, that the calling thread did not unregister itself.
    self.tracing.emitTraceSimple("cleaning up context, context='{*}'", .{self}, @src());

    self.@"async".deinit();
    self.module.deinit();
    self.tracing.deinit();
    self.result_list.unrefDestroy();

    var gpa = self.gpa;
    gpa.allocator().destroy(self);
    if (gpa.deinit() == .leak) @panic("memory leak");
}

pub fn hasErrorResult(self: *Self) bool {
    return if (self.result_list.local_result.get()) |l| l.result.isErr() else false;
}

pub fn replaceResult(self: *Self, with: AnyResult) AnyResult {
    return self.result_list.replaceResult(with);
}

pub fn takeResult(self: *Self) AnyResult {
    return self.replaceResult(.ok);
}

pub fn setResult(self: *Self, result: AnyResult) void {
    self.replaceResult(result).deinit();
}

pub fn asProxy(self: *Self) ProxyContext {
    return ProxyContext{ .data = self, .vtable = &vtable };
}

pub fn fromProxyPtr(ptr: *anyopaque) *Self {
    return @alignCast(@ptrCast(ptr));
}

pub fn fromProxy(ctx: ProxyContext) *Self {
    std.debug.assert(ctx.vtable == &vtable);
    return @alignCast(@ptrCast(ctx.data));
}

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn isCompatible(ctx: *anyopaque, version: *const c.FimoVersion) callconv(.c) AnyResult {
        _ = ctx;
        const v = Version.initC(version.*);
        if (ProxyContext.context_version.isCompatibleWith(v)) return AnyResult.ok;
        return AnyError.initError(error.NotCompatible).intoResult();
    }
    fn ref(ctx: *anyopaque) callconv(.c) void {
        const self = Self.fromProxyPtr(ctx);
        self.ref();
    }
    fn unref(ctx: *anyopaque) callconv(.c) void {
        const self = Self.fromProxyPtr(ctx);
        self.unref();
    }
    fn hasErrorResult(ctx: *anyopaque) callconv(.c) bool {
        const self = Self.fromProxyPtr(ctx);
        return self.hasErrorResult();
    }
    fn replaceResult(ctx: *anyopaque, with: AnyResult) callconv(.c) AnyResult {
        const self = Self.fromProxyPtr(ctx);
        return self.replaceResult(with);
    }
};

const vtable = ProxyContext.VTable{
    .header = .{
        .check_version = &VTableImpl.isCompatible,
    },
    .core_v0 = .{
        .acquire = &VTableImpl.ref,
        .release = &VTableImpl.unref,
        .has_error_result = &VTableImpl.hasErrorResult,
        .replace_result = &VTableImpl.replaceResult,
    },
    .tracing_v0 = Tracing.vtable,
    .module_v0 = Module.vtable,
    .async_v0 = Async.vtable,
};

test {
    _ = @import("context/graph.zig");
    _ = @import("context/module.zig");
    _ = @import("context/proxy_context.zig");
    _ = @import("context/RefCount.zig");
    _ = @import("context/tls.zig");
    _ = @import("context/tmp_path.zig");
    _ = @import("context/tracing.zig");
}
