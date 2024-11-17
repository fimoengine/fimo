//! Internal inteface of the fimo std library.
const std = @import("std");
const Allocator = std.mem.Allocator;

const c = @import("c.zig");
const heap = @import("heap.zig");
const Error = @import("errors.zig").Error;
const Version = @import("Version.zig");

const RefCount = @import("context/RefCount.zig");
const Tracing = @import("context/tracing.zig");
const Module = @import("context/module.zig");

pub const ProxyTracing = @import("context/proxy_context/tracing.zig");
pub const ProxyModule = @import("context/proxy_context/module.zig");
pub const ProxyContext = @import("context/proxy_context.zig");

const allocator = heap.fimo_allocator;
const Self = @This();

refcount: RefCount = .{},
tracing: Tracing,
module: Module,

pub fn init(options: [:null]const ?*const ProxyContext.TaggedInStruct) !*Self {
    var cleanup_options: bool = true;
    errdefer if (cleanup_options) for (options) |opt| {
        const o = if (opt) |o| o else continue;
        switch (o.id) {
            .tracing_creation_config => {
                const cfg: *const ProxyTracing.Config = @alignCast(@ptrCast(o));
                cfg.deinit();
            },
            else => {},
        }
    };

    var tracing_cfg: ?*const ProxyTracing.Config = null;
    for (options) |opt| {
        const o = if (opt) |o| o else return error.InvalidInput;
        switch (o.id) {
            .tracing_creation_config => {
                if (tracing_cfg != null) return error.InvalidInput;
                tracing_cfg = @alignCast(@ptrCast(o));
            },
            else => return error.InvalidInput,
        }
    }
    cleanup_options = false;
    errdefer if (tracing_cfg) |cfg| cfg.deinit();

    const self = try allocator.create(Self);
    errdefer allocator.destroy(self);
    self.* = Self{
        .tracing = undefined,
        .module = undefined,
    };

    self.tracing = try Tracing.init(tracing_cfg);
    errdefer self.tracing.deinit();
    tracing_cfg = null;

    self.module = try Module.init(self);
    errdefer self.module.deinit();

    return self;
}

pub fn ref(self: *Self) void {
    self.refcount.ref();
}

pub fn unref(self: *Self) void {
    if (self.refcount.unref() == .noop) return;
    self.module.deinit();
    self.tracing.deinit();
    allocator.destroy(self);
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
    fn isCompatible(ctx: *anyopaque, version: *const c.FimoVersion) callconv(.C) c.FimoResult {
        _ = ctx;
        const v = Version.initC(version.*);
        if (ProxyContext.context_version.isCompatibleWith(v)) return Error.intoCResult(null);
        return Error.initError(error.NotCompatible).err;
    }
    fn ref(ctx: *anyopaque) callconv(.C) void {
        const self = Self.fromProxyPtr(ctx);
        self.ref();
    }
    fn unref(ctx: *anyopaque) callconv(.C) void {
        const self = Self.fromProxyPtr(ctx);
        self.unref();
    }
};

const vtable = ProxyContext.VTable{
    .header = .{
        .check_version = &VTableImpl.isCompatible,
    },
    .core_v0 = .{
        .acquire = &VTableImpl.ref,
        .release = &VTableImpl.unref,
    },
    .tracing_v0 = Tracing.vtable,
    .module_v0 = Module.vtable,
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
