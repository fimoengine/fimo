const std = @import("std");
const Allocator = std.mem.Allocator;

const c = @import("../c.zig");
const AnyError = @import("../AnyError.zig");
const Path = @import("../path.zig").Path;
const Version = @import("../Version.zig");

const EnqueuedFuture = ProxyAsync.EnqueuedFuture;
const Fallible = ProxyAsync.Fallible;

const InstanceHandle = @import("module/InstanceHandle.zig");
const LoadingSet = @import("module/LoadingSet.zig");
const ModuleHandle = @import("module/ModuleHandle.zig");
const System = @import("module/System.zig");

const Async = @import("async.zig");
const Context = @import("../context.zig");
const ProxyContext = @import("proxy_context.zig");
const ProxyAsync = @import("proxy_context/async.zig");
const ProxyModule = @import("proxy_context/module.zig");

const Self = @This();

sys: System,

pub fn init(ctx: *Context) !Self {
    return Self{ .sys = try System.init(ctx) };
}

pub fn deinit(self: *Self) void {
    self.sys.deinit();
}

pub fn asContext(self: *Self) *Context {
    return @fieldParentPtr("module", self);
}

fn logTrace(self: *Self, comptime fmt: []const u8, args: anytype, location: std.builtin.SourceLocation) void {
    self.asContext().tracing.emitTraceSimple(fmt, args, location);
}

/// Adds a new pseudo instance.
///
/// The pseudo instance provides access to the subsystem for non-instances, and is mainly intended
/// for bootstrapping.
pub fn addPseudoInstance(self: *Self) !*const ProxyModule.PseudoInstance {
    self.logTrace("adding new pseudo instance", .{}, @src());
    self.sys.lock();
    defer self.sys.unlock();

    var name_buf: [32]u8 = undefined;
    var name: []u8 = undefined;
    while (true) {
        var random_bytes: [8]u8 = undefined;
        std.crypto.random.bytes(&random_bytes);
        var suffix: [std.fs.base64_encoder.calcSize(8)]u8 = undefined;
        _ = std.fs.base64_encoder.encode(&suffix, &random_bytes);
        name = std.fmt.bufPrint(&name_buf, "_pseudo_{s}", .{suffix}) catch unreachable;
        if (self.sys.getInstance(name) == null) break;
    }

    const instance = try InstanceHandle.initPseudoInstance(&self.sys, name);
    const handle = InstanceHandle.fromInstancePtr(&instance.instance);
    const inner = handle.lock();
    errdefer inner.deinit();

    var err: ?AnyError = null;
    inner.start(&self.sys, &err) catch unreachable;
    try self.sys.addInstance(inner);
    inner.unlock();
    return instance;
}

/// Initializes a new empty loading set.
pub fn addLoadingSet(self: *Self, err: *?AnyError) !EnqueuedFuture(Fallible(*LoadingSet)) {
    self.logTrace("creating new loading set", .{}, @src());
    var fut = LoadingSet.init(self.asContext()).intoFuture().map(
        Fallible(*LoadingSet),
        Fallible(*LoadingSet).Wrapper(anyerror),
    ).intoFuture();
    errdefer fut.deinit();
    return Async.Task.initFuture(
        @TypeOf(fut),
        &self.asContext().@"async".sys,
        &fut,
        err,
    );
}

/// Searches for an instance by its name.
pub fn findInstanceByName(self: *Self, name: []const u8) System.SystemError!*const ProxyModule.Info {
    self.logTrace("searching for instance, name='{s}'", .{name}, @src());
    self.sys.lock();
    defer self.sys.unlock();

    const instance_ref = self.sys.getInstance(name) orelse return error.NotFound;
    instance_ref.instance.info.ref();
    return instance_ref.instance.info;
}

/// Searches for the instance that exports a specific symbol.
pub fn findInstanceBySymbol(
    self: *Self,
    name: []const u8,
    namespace: []const u8,
    version: Version,
) System.SystemError!*const ProxyModule.Info {
    self.logTrace(
        "searching for symbol owner, name='{s}', namespace='{s}', version='{long}'",
        .{ name, namespace, version },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const symbol_ref = self.sys.getSymbolCompatible(
        name,
        namespace,
        version,
    ) orelse return error.NotFound;
    const instance_ref = self.sys.getInstance(symbol_ref.owner) orelse unreachable;
    instance_ref.instance.info.ref();
    return instance_ref.instance.info;
}

/// Queries whether a namespace exists.
///
/// To exist, the namespace must contain at least one symbol.
/// The global namespace always exist.
pub fn queryNamespace(self: *Self, namespace: []const u8) bool {
    self.logTrace("querying namespace, namespace='{s}'", .{namespace}, @src());
    self.sys.lock();
    defer self.sys.unlock();
    if (std.mem.eql(u8, namespace, System.global_namespace)) return true;
    return self.sys.getNamespace(namespace) != null;
}

/// Unloads all unreferenced instances.
pub fn pruneInstances(self: *Self) System.SystemError!void {
    self.logTrace("pruning instances", .{}, @src());
    self.sys.lock();
    defer self.sys.unlock();
    try self.sys.cleanupLooseInstances();
}

/// Queries the info of a module parameter.
pub fn queryParameter(
    self: *Self,
    owner: []const u8,
    parameter: []const u8,
) error{NotFound}!struct {
    type: ProxyModule.ParameterType,
    read_group: ProxyModule.ParameterAccessGroup,
    write_group: ProxyModule.ParameterAccessGroup,
} {
    self.logTrace(
        "querying parameter, owner='{s}', parameter='{s}'",
        .{ owner, parameter },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const owner_instance = self.sys.getInstance(owner) orelse return error.NotFound;
    const owner_handle = InstanceHandle.fromInstancePtr(owner_instance.instance);
    const owner_inner = owner_handle.lock();
    defer owner_inner.unlock();

    const param: *InstanceHandle.Parameter = owner_inner.getParameter(parameter) orelse return error.NotFound;
    return .{
        .type = param.type(),
        .read_group = param.read_group,
        .write_group = param.write_group,
    };
}

/// Atomically reads the value and type of a public parameter.
pub fn readParameter(
    self: *Self,
    value: *anyopaque,
    @"type": ProxyModule.ParameterType,
    owner: []const u8,
    parameter: []const u8,
) (InstanceHandle.ParameterError || error{ FfiError, NotFound })!void {
    self.logTrace(
        "reading public parameter, value='{*}', type='{s}', owner='{s}', parameter='{s}'",
        .{ value, @tagName(@"type"), owner, parameter },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const owner_instance = self.sys.getInstance(owner) orelse return error.NotFound;
    const owner_handle = InstanceHandle.fromInstancePtr(owner_instance.instance);
    const owner_inner = owner_handle.lock();
    defer owner_inner.unlock();

    const param: *InstanceHandle.Parameter = owner_inner.getParameter(parameter) orelse return error.NotFound;
    try param.checkType(@"type");
    try param.checkReadPublic();
    param.readTo(value);
}

/// Atomically reads the value and type of a public parameter.
pub fn writeParameter(
    self: *Self,
    value: *const anyopaque,
    @"type": ProxyModule.ParameterType,
    owner: []const u8,
    parameter: []const u8,
) (InstanceHandle.ParameterError || error{ FfiError, NotFound })!void {
    self.logTrace(
        "write public parameter, value='{*}', type='{s}', owner='{s}', parameter='{s}'",
        .{ value, @tagName(@"type"), owner, parameter },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const owner_instance = self.sys.getInstance(owner) orelse return error.NotFound;
    const owner_handle = InstanceHandle.fromInstancePtr(owner_instance.instance);
    const owner_inner = owner_handle.lock();
    defer owner_inner.unlock();

    const param: *InstanceHandle.Parameter = owner_inner.getParameter(parameter) orelse return error.NotFound;
    try param.checkType(@"type");
    try param.checkWritePublic();
    param.writeFrom(value);
}

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn addPseudoInstance(
        ptr: *anyopaque,
        instance: **const ProxyModule.PseudoInstance,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        instance.* = ctx.module.addPseudoInstance() catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn addLoadingSet(
        ptr: *anyopaque,
        set: *ProxyModule.LoadingSet,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        set.* = LoadingSet.init(ctx) catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn findInstanceByName(
        ptr: *anyopaque,
        name: [*:0]const u8,
        info: **const ProxyModule.Info,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        info.* = ctx.module.findInstanceByName(std.mem.span(name)) catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn findInstanceBySymbol(
        ptr: *anyopaque,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: c.FimoVersion,
        info: **const ProxyModule.Info,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        info.* = ctx.module.findInstanceBySymbol(
            std.mem.span(name),
            std.mem.span(namespace),
            Version.initC(version),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn queryNamespace(
        ptr: *anyopaque,
        namespace: [*:0]const u8,
        exists: *bool,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        exists.* = ctx.module.queryNamespace(std.mem.span(namespace));
        return AnyError.intoCResult(null);
    }
    fn pruneInstances(ptr: *anyopaque) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.pruneInstances() catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn queryParameter(
        ptr: *anyopaque,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
        @"type": *ProxyModule.ParameterType,
        read_group: *ProxyModule.ParameterAccessGroup,
        write_group: *ProxyModule.ParameterAccessGroup,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        const info = ctx.module.queryParameter(
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        @"type".* = info.type;
        read_group.* = info.read_group;
        write_group.* = info.write_group;
        return AnyError.intoCResult(null);
    }
    fn readParameter(
        ptr: *anyopaque,
        value: *anyopaque,
        @"type": ProxyModule.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.readParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn writeParameter(
        ptr: *anyopaque,
        value: *const anyopaque,
        @"type": ProxyModule.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.writeParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
};

pub const vtable = ProxyModule.VTable{
    .pseudo_module_new = &VTableImpl.addPseudoInstance,
    .set_new = &VTableImpl.addLoadingSet,
    .find_by_name = &VTableImpl.findInstanceByName,
    .find_by_symbol = &VTableImpl.findInstanceBySymbol,
    .namespace_exists = &VTableImpl.queryNamespace,
    .prune_instances = &VTableImpl.pruneInstances,
    .query_parameter = &VTableImpl.queryParameter,
    .read_parameter = &VTableImpl.readParameter,
    .write_parameter = &VTableImpl.writeParameter,
};
