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

/// Removes a pseudo instance.
pub fn removePseudoInstance(
    self: *Self,
    instance: *const ProxyModule.PseudoInstance,
) System.SystemError!void {
    self.logTrace(
        "removing pseudo instance, instance='{s}'",
        .{instance.instance.info.name},
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const handle = InstanceHandle.fromInstancePtr(&instance.instance);
    if (handle.type != .pseudo) return error.NotPermitted;
    const inner = handle.lock();
    var inner_destroyed = false;
    errdefer if (!inner_destroyed) inner.unlock();

    try self.sys.removeInstance(inner);
    inner.stop(&self.sys);
    inner.deinit();
    inner_destroyed = true;

    try self.sys.cleanupLooseInstances();
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

/// Unloads an instance.
///
/// To succeed, the instance may not be in use by any other instance.
/// After unloading the instance, this function cleans up unreferenced instances, except pseudo instances.
pub fn unloadInstance(self: *Self, instance_info: *const ProxyModule.Info) System.SystemError!void {
    self.logTrace(
        "unloading instance, instance='{s}'",
        .{instance_info.name},
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const handle = InstanceHandle.fromInfoPtr(instance_info);
    if (handle.type != .regular) return error.NotPermitted;
    const inner = handle.lock();
    var inner_destroyed = false;
    errdefer if (!inner_destroyed) inner.unlock();
    try self.sys.removeInstance(inner);
    inner.stop(&self.sys);
    inner.deinit();
    inner_destroyed = true;
    try self.sys.cleanupLooseInstances();
}

/// Unloads all unreferenced instances.
pub fn unloadUnusedInstances(self: *Self) System.SystemError!void {
    self.logTrace("unloading unused instances", .{}, @src());
    self.sys.lock();
    defer self.sys.unlock();
    try self.sys.cleanupLooseInstances();
}

/// Queries the info of a module parameter.
pub fn queryParameterInfo(
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
        .type = param.data.getType(),
        .read_group = param.read_group,
        .write_group = param.write_group,
    };
}

/// Atomically reads the value and type of a public parameter.
pub fn readPublicParameterTo(
    self: *Self,
    value: *anyopaque,
    @"type": *ProxyModule.ParameterType,
    owner: []const u8,
    parameter: []const u8,
    err: *?AnyError,
) (InstanceHandle.ParameterError || error{ FfiError, NotFound })!void {
    self.logTrace(
        "reading public parameter, value='{*}', type='{*}', owner='{s}', parameter='{s}'",
        .{ value, @"type", owner, parameter },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const owner_instance = self.sys.getInstance(owner) orelse return error.NotFound;
    const owner_handle = InstanceHandle.fromInstancePtr(owner_instance.instance);
    const owner_inner = owner_handle.lock();
    defer owner_inner.unlock();

    const param: *InstanceHandle.Parameter = owner_inner.getParameter(parameter) orelse return error.NotFound;
    try param.checkReadPublic();
    try param.readTo(value, @"type", err);
}

/// Atomically reads the value and type of a public parameter.
pub fn writePublicParameterFrom(
    self: *Self,
    value: *const anyopaque,
    @"type": ProxyModule.ParameterType,
    owner: []const u8,
    parameter: []const u8,
    err: *?AnyError,
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
    try param.checkWritePublic();
    try param.writeFrom(value, @"type", err);
}

/// Atomically reads the value and type of a dependency parameter.
pub fn readDependencyParameterTo(
    self: *Self,
    reader: *const ProxyModule.OpaqueInstance,
    value: *anyopaque,
    @"type": *ProxyModule.ParameterType,
    owner: []const u8,
    parameter: []const u8,
    err: *?AnyError,
) (InstanceHandle.ParameterError || error{ FfiError, NotFound })!void {
    self.logTrace(
        "reading dependency parameter, reader='{s}', value='{*}', type='{*}', owner='{s}', parameter='{s}'",
        .{ reader.info.name, value, @"type", owner, parameter },
        @src(),
    );
    const handle = InstanceHandle.fromInstancePtr(reader);
    const inner = handle.lock();
    defer inner.unlock();

    const owner_handle = inner.getDependency(owner) orelse return error.NotADependency;
    const owner_inner = owner_handle.instance.lock();
    defer owner_inner.unlock();

    const param: *InstanceHandle.Parameter = owner_inner.getParameter(parameter) orelse return error.NotFound;
    try param.checkReadDependency(inner);
    try param.readTo(value, @"type", err);
}

/// Atomically reads the value and type of a dependency parameter.
pub fn writeDependencyParameterFrom(
    self: *Self,
    writer: *const ProxyModule.OpaqueInstance,
    value: *const anyopaque,
    @"type": ProxyModule.ParameterType,
    owner: []const u8,
    parameter: []const u8,
    err: *?AnyError,
) (InstanceHandle.ParameterError || error{ FfiError, NotFound })!void {
    self.logTrace(
        "writing dependency parameter, reader='{s}', value='{*}', type='{s}', owner='{s}', parameter='{s}'",
        .{ writer.info.name, value, @tagName(@"type"), owner, parameter },
        @src(),
    );
    const handle = InstanceHandle.fromInstancePtr(writer);
    const inner = handle.lock();
    defer inner.unlock();

    const owner_handle = inner.getDependency(owner) orelse return error.NotADependency;
    const owner_inner = owner_handle.instance.lock();
    defer owner_inner.unlock();

    const param: *InstanceHandle.Parameter = owner_inner.getParameter(parameter) orelse return error.NotFound;
    try param.checkWriteDependency(inner);
    try param.writeFrom(value, @"type", err);
}

/// Atomically reads the value and type of a private parameter.
pub fn readPrivateParameterTo(
    self: *Self,
    reader: *const ProxyModule.OpaqueInstance,
    value: *anyopaque,
    @"type": *ProxyModule.ParameterType,
    o_param: *const ProxyModule.OpaqueParameter,
    err: *?AnyError,
) (InstanceHandle.ParameterError || AnyError.Error)!void {
    self.logTrace(
        "reading private parameter, reader='{s}', value='{*}', type='{*}', parameter='{*}'",
        .{ reader.info.name, value, @"type", o_param },
        @src(),
    );
    const handle = InstanceHandle.fromInstancePtr(reader);
    const inner = handle.lock();
    defer inner.unlock();

    const param: *const InstanceHandle.Parameter = @alignCast(@ptrCast(o_param));
    try param.checkReadPrivate(reader);
    try param.readTo(value, @"type", err);
}

/// Atomically writes the value of a private parameter.
pub fn writePrivateParameterFrom(
    self: *Self,
    writer: *const ProxyModule.OpaqueInstance,
    value: *const anyopaque,
    @"type": ProxyModule.ParameterType,
    o_param: *ProxyModule.OpaqueParameter,
    err: *?AnyError,
) (InstanceHandle.ParameterError || AnyError.Error)!void {
    self.logTrace(
        "writing private parameter, writer='{s}', value='{*}', type='{s}', parameter='{*}'",
        .{ writer.info.name, value, @tagName(@"type"), o_param },
        @src(),
    );
    const handle = InstanceHandle.fromInstancePtr(writer);
    const inner = handle.lock();
    defer inner.unlock();

    const param: *InstanceHandle.Parameter = @alignCast(@ptrCast(o_param));
    try param.checkReadPrivate(writer);
    try param.writeFrom(value, @"type", err);
}

/// Atomically reads the value and type of a parameter data.
pub fn readParameterDataTo(
    self: *Self,
    reader: *const ProxyModule.OpaqueInstance,
    value: *anyopaque,
    @"type": *ProxyModule.ParameterType,
    o_param: *const ProxyModule.OpaqueParameterData,
) InstanceHandle.ParameterError!void {
    const param: *const InstanceHandle.Parameter.Data = @alignCast(@ptrCast(o_param));
    self.logTrace(
        "reading parameter data, reader='{s}', value='{*}', type='{*}', parameter='{*}'",
        .{ reader.info.name, value, @"type", param },
        @src(),
    );
    try param.checkOwner(reader);
    param.readTo(value, @"type");
}

/// Atomically writes the value a parameter data.
pub fn writeParameterDataFrom(
    self: *Self,
    writer: *const ProxyModule.OpaqueInstance,
    value: *const anyopaque,
    @"type": ProxyModule.ParameterType,
    o_param: *ProxyModule.OpaqueParameterData,
) InstanceHandle.ParameterError!void {
    const param: *InstanceHandle.Parameter.Data = @alignCast(@ptrCast(o_param));
    self.logTrace(
        "writing parameter data, writer='{s}', value='{*}', type='{s}', parameter='{*}'",
        .{ writer.info.name, value, @tagName(@"type"), param },
        @src(),
    );
    try param.checkOwner(writer);
    try param.checkType(@"type");
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
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn removePseudoInstance(
        ptr: *anyopaque,
        instance: *const ProxyModule.PseudoInstance,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.removePseudoInstance(instance) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn addLoadingSet(
        ptr: *anyopaque,
        fut: *EnqueuedFuture(Fallible(ProxyModule.LoadingSet)),
    ) callconv(.C) c.FimoResult {
        var err: ?AnyError = null;
        const ctx = Context.fromProxyPtr(ptr);

        fut.* = LoadingSet.init(ctx, &err) catch |e| {
            return switch (e) {
                AnyError.Error.FfiError => AnyError.intoCResult(err),
                else => AnyError.initError(e).err,
            };
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
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
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
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
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
    fn unloadInstance(ptr: *anyopaque, instance_info: ?*const ProxyModule.Info) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        if (instance_info) |info|
            ctx.module.unloadInstance(info) catch |e| {
                if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
                return AnyError.initError(e).err;
            }
        else
            ctx.module.unloadUnusedInstances() catch |e| {
                if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
                return AnyError.initError(e).err;
            };
        return AnyError.intoCResult(null);
    }
    fn queryParameterInfo(
        ptr: *anyopaque,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
        @"type": *ProxyModule.ParameterType,
        read_group: *ProxyModule.ParameterAccessGroup,
        write_group: *ProxyModule.ParameterAccessGroup,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        const info = ctx.module.queryParameterInfo(
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        @"type".* = info.type;
        read_group.* = info.read_group;
        write_group.* = info.write_group;
        return AnyError.intoCResult(null);
    }
    fn readPublicParameterTo(
        ptr: *anyopaque,
        value: *anyopaque,
        @"type": *ProxyModule.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        var err: ?AnyError = null;
        ctx.module.readPublicParameterTo(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
            &err,
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            if (err) |x| return x.err;
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn writePublicParameterFrom(
        ptr: *anyopaque,
        value: *const anyopaque,
        @"type": ProxyModule.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        var err: ?AnyError = null;
        ctx.module.writePublicParameterFrom(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
            &err,
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            if (err) |x| return x.err;
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn readDependencyParameterTo(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        value: *anyopaque,
        @"type": *ProxyModule.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        var err: ?AnyError = null;
        ctx.module.readDependencyParameterTo(
            instance,
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
            &err,
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            if (err) |x| return x.err;
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn writeDependencyParameterFrom(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        value: *const anyopaque,
        @"type": ProxyModule.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        var err: ?AnyError = null;
        ctx.module.writeDependencyParameterFrom(
            instance,
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
            &err,
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            if (err) |x| return x.err;
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn readPrivateParameterTo(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        value: *anyopaque,
        @"type": *ProxyModule.ParameterType,
        param: *const ProxyModule.OpaqueParameter,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        var err: ?AnyError = null;
        ctx.module.readPrivateParameterTo(instance, value, @"type", param, &err) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            if (err) |x| return x.err;
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn writePrivateParameterFrom(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        value: *const anyopaque,
        @"type": ProxyModule.ParameterType,
        param: *ProxyModule.OpaqueParameter,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        var err: ?AnyError = null;
        ctx.module.writePrivateParameterFrom(instance, value, @"type", param, &err) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            if (err) |x| return x.err;
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn readParameterDataTo(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        value: *anyopaque,
        @"type": *ProxyModule.ParameterType,
        param: *const ProxyModule.OpaqueParameterData,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.readParameterDataTo(instance, value, @"type", param) catch |err| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(err).err;
        };
        return AnyError.intoCResult(null);
    }
    fn writeParameterDataFrom(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        value: *const anyopaque,
        @"type": ProxyModule.ParameterType,
        param: *ProxyModule.OpaqueParameterData,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.writeParameterDataFrom(instance, value, @"type", param) catch |err| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(err).err;
        };
        return AnyError.intoCResult(null);
    }
};

pub const vtable = ProxyModule.VTable{
    .pseudo_module_new = &VTableImpl.addPseudoInstance,
    .pseudo_module_destroy = &VTableImpl.removePseudoInstance,
    .set_new = &VTableImpl.addLoadingSet,
    .find_by_name = &VTableImpl.findInstanceByName,
    .find_by_symbol = &VTableImpl.findInstanceBySymbol,
    .namespace_exists = &VTableImpl.queryNamespace,
    .unload = &VTableImpl.unloadInstance,
    .param_query = &VTableImpl.queryParameterInfo,
    .param_set_public = &VTableImpl.writePublicParameterFrom,
    .param_get_public = &VTableImpl.readPublicParameterTo,
    .param_set_dependency = &VTableImpl.writeDependencyParameterFrom,
    .param_get_dependency = &VTableImpl.readDependencyParameterTo,
    .param_set_private = &VTableImpl.writePrivateParameterFrom,
    .param_get_private = &VTableImpl.readPrivateParameterTo,
    .param_set_inner = &VTableImpl.writeParameterDataFrom,
    .param_get_inner = &VTableImpl.readParameterDataTo,
};
