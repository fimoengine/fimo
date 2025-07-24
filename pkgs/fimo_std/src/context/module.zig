const std = @import("std");
const Allocator = std.mem.Allocator;

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const Context = @import("../context.zig");
const pub_modules = @import("../modules.zig");
const Path = @import("../path.zig").Path;
const pub_tasks = @import("../tasks.zig");
const EnqueuedFuture = pub_tasks.EnqueuedFuture;
const Fallible = pub_tasks.Fallible;
const Version = @import("../Version.zig");
const Async = @import("async.zig");
const InstanceHandle = @import("module/InstanceHandle.zig");
const LoadingSet = @import("module/LoadingSet.zig");
const ModuleHandle = @import("module/ModuleHandle.zig");
const System = @import("module/System.zig");

const Self = @This();

sys: System,

pub fn init(ctx: *Context, config: *const pub_modules.Config) !Self {
    return Self{ .sys = try System.init(ctx, config) };
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
pub fn addPseudoInstance(self: *Self) !*const pub_modules.PseudoInstance {
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
    errdefer {
        inner.stop(&self.sys);
        inner.deinit();
    }

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
        &self.asContext().async.sys,
        &fut,
        err,
    );
}

/// Searches for an instance by its name.
pub fn findInstanceByName(self: *Self, name: []const u8) System.SystemError!*const pub_modules.Info {
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
) System.SystemError!*const pub_modules.Info {
    self.logTrace(
        "searching for symbol owner, name='{s}', namespace='{s}', version='{f}'",
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

/// Marks all instances as unloadable.
pub fn pruneInstances(self: *Self) System.SystemError!void {
    self.logTrace("pruning instances", .{}, @src());
    self.sys.lock();
    defer self.sys.unlock();
    try self.sys.pruneInstances();
}

/// Queries the info of a module parameter.
pub fn queryParameter(
    self: *Self,
    owner: []const u8,
    parameter: []const u8,
) error{NotFound}!struct {
    type: pub_modules.ParameterType,
    read_group: pub_modules.ParameterAccessGroup,
    write_group: pub_modules.ParameterAccessGroup,
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
    @"type": pub_modules.ParameterType,
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
    @"type": pub_modules.ParameterType,
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
    fn profile() callconv(.c) pub_modules.Profile {
        std.debug.assert(Context.is_init);
        return Context.global.module.sys.profile;
    }
    fn features(out: *?[*]const pub_modules.FeatureStatus) callconv(.c) usize {
        std.debug.assert(Context.is_init);
        out.* = &Context.global.module.sys.features;
        return Context.global.module.sys.features.len;
    }
    fn addPseudoInstance(instance: **const pub_modules.PseudoInstance) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        instance.* = Context.global.module.addPseudoInstance() catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn addLoadingSet(set: *pub_modules.LoadingSet) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        set.* = LoadingSet.init(&Context.global) catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn findInstanceByName(
        name: [*:0]const u8,
        info: **const pub_modules.Info,
    ) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        info.* = Context.global.module.findInstanceByName(std.mem.span(name)) catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn findInstanceBySymbol(
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: Version.CVersion,
        info: **const pub_modules.Info,
    ) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        info.* = Context.global.module.findInstanceBySymbol(
            std.mem.span(name),
            std.mem.span(namespace),
            Version.initC(version),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn queryNamespace(namespace: [*:0]const u8, exists: *bool) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        exists.* = Context.global.module.queryNamespace(std.mem.span(namespace));
        return AnyResult.ok;
    }
    fn pruneInstances() callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        Context.global.module.pruneInstances() catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn queryParameter(
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
        @"type": *pub_modules.ParameterType,
        read_group: *pub_modules.ParameterAccessGroup,
        write_group: *pub_modules.ParameterAccessGroup,
    ) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        const info = Context.global.module.queryParameter(
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        @"type".* = info.type;
        read_group.* = info.read_group;
        write_group.* = info.write_group;
        return AnyResult.ok;
    }
    fn readParameter(
        value: *anyopaque,
        @"type": pub_modules.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        Context.global.module.readParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn writeParameter(
        value: *const anyopaque,
        @"type": pub_modules.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) AnyResult {
        std.debug.assert(Context.is_init);
        Context.global.module.writeParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr|
                Context.global.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
};

pub const vtable = pub_modules.VTable{
    .profile = &VTableImpl.profile,
    .features = &VTableImpl.features,
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
