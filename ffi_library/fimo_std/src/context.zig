//! Internal inteface of the fimo std library.
const std = @import("std");
const Allocator = std.mem.Allocator;

const c = @import("c.zig");
const AnyError = @import("AnyError.zig");
const Version = @import("Version.zig");

const RefCount = @import("context/RefCount.zig");

const Async = @import("context/async.zig");
const Tracing = @import("context/tracing.zig");
const Module = @import("context/module.zig");

pub const ProxyTracing = @import("context/proxy_context/tracing.zig");
pub const ProxyModule = @import("context/proxy_context/module.zig");
pub const ProxyContext = @import("context/proxy_context.zig");

const GPA = std.heap.GeneralPurposeAllocator(.{});
const Self = @This();

gpa: GPA,
allocator: Allocator,
refcount: RefCount = .{},
tracing: Tracing,
module: Module,
@"async": Async = undefined,

pub fn init(options: [:null]const ?*const ProxyContext.TaggedInStruct) !*Self {
    var tracing_cfg: ?*const ProxyTracing.Config = null;
    for (options) |opt| {
        const o = if (opt) |o| o else return error.InvalidInput;
        switch (o.id) {
            .tracing_config => {
                if (tracing_cfg != null) return error.InvalidInput;
                tracing_cfg = @alignCast(@ptrCast(o));
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
        .tracing = undefined,
        .module = undefined,
    };
    self.allocator = self.gpa.allocator();

    self.tracing = try Tracing.init(self.allocator, tracing_cfg);
    errdefer self.tracing.deinit();
    tracing_cfg = null;

    self.module = try Module.init(self);
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

    var gpa = self.gpa;
    gpa.allocator().destroy(self);
    if (gpa.deinit() == .leak) @panic("memory leak");
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
        if (ProxyContext.context_version.isCompatibleWith(v)) return AnyError.intoCResult(null);
        return AnyError.initError(error.NotCompatible).err;
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
