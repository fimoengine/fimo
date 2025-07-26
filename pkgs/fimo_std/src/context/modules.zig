const std = @import("std");
const Allocator = std.mem.Allocator;

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const context = @import("../context.zig");
const pub_modules = @import("../modules.zig");
const Path = @import("../path.zig").Path;
const pub_tasks = @import("../tasks.zig");
const EnqueuedFuture = pub_tasks.EnqueuedFuture;
const Fallible = pub_tasks.Fallible;
const Version = @import("../Version.zig");
const InstanceHandle = @import("module/InstanceHandle.zig");
const LoadingSet = @import("module/LoadingSet.zig");
const ModuleHandle = @import("module/ModuleHandle.zig");
const System = @import("module/System.zig");
const tasks = @import("tasks.zig");
const tracing = @import("tracing.zig");

const modules = @This();

pub var sys: System = undefined;

pub fn init(config: *const pub_modules.Config) !void {
    sys = try System.init(config);
}

pub fn deinit() void {
    sys.deinit();
}

/// Adds a new pseudo instance.
///
/// The pseudo instance provides access to the subsystem for non-instances, and is mainly intended
/// for bootstrapping.
pub fn addPseudoInstance() !*const pub_modules.PseudoInstance {
    tracing.emitTraceSimple("adding new pseudo instance", .{}, @src());
    sys.lock();
    defer sys.unlock();

    var name_buf: [32]u8 = undefined;
    var name: []u8 = undefined;
    while (true) {
        var random_bytes: [8]u8 = undefined;
        std.crypto.random.bytes(&random_bytes);
        var suffix: [std.fs.base64_encoder.calcSize(8)]u8 = undefined;
        _ = std.fs.base64_encoder.encode(&suffix, &random_bytes);
        name = std.fmt.bufPrint(&name_buf, "_pseudo_{s}", .{suffix}) catch unreachable;
        if (sys.getInstance(name) == null) break;
    }

    const instance = try InstanceHandle.initPseudoInstance(&sys, name);
    const handle = InstanceHandle.fromInstancePtr(&instance.instance);
    const inner = handle.lock();
    errdefer {
        inner.stop(&sys);
        inner.deinit();
    }

    try sys.addInstance(inner);
    inner.unlock();
    return instance;
}

/// Initializes a new empty loading set.
pub fn addLoadingSet(err: *?AnyError) !EnqueuedFuture(Fallible(*LoadingSet)) {
    tracing.emitTraceSimple("creating new loading set", .{}, @src());
    var fut = LoadingSet.init(&context.global).intoFuture().map(
        Fallible(*LoadingSet),
        Fallible(*LoadingSet).Wrapper(anyerror),
    ).intoFuture();
    errdefer fut.deinit();
    return tasks.Task.initFuture(
        @TypeOf(fut),
        &context.global.async.sys,
        &fut,
        err,
    );
}

/// Searches for an instance by its name.
pub fn findInstanceByName(name: []const u8) !*const pub_modules.Info {
    tracing.emitTraceSimple("searching for instance, name='{s}'", .{name}, @src());
    sys.lock();
    defer sys.unlock();

    const instance_ref = sys.getInstance(name) orelse return error.NotFound;
    instance_ref.instance.info.ref();
    return instance_ref.instance.info;
}

/// Searches for the instance that exports a specific symbol.
pub fn findInstanceBySymbol(name: []const u8, namespace: []const u8, version: Version) !*const pub_modules.Info {
    tracing.emitTraceSimple(
        "searching for symbol owner, name='{s}', namespace='{s}', version='{f}'",
        .{ name, namespace, version },
        @src(),
    );
    sys.lock();
    defer sys.unlock();

    const symbol_ref = sys.getSymbolCompatible(
        name,
        namespace,
        version,
    ) orelse return error.NotFound;
    const instance_ref = sys.getInstance(symbol_ref.owner) orelse unreachable;
    instance_ref.instance.info.ref();
    return instance_ref.instance.info;
}

/// Queries whether a namespace exists.
///
/// To exist, the namespace must contain at least one symbol.
/// The global namespace always exist.
pub fn queryNamespace(namespace: []const u8) bool {
    tracing.emitTraceSimple("querying namespace, namespace='{s}'", .{namespace}, @src());
    sys.lock();
    defer sys.unlock();
    if (std.mem.eql(u8, namespace, System.global_namespace)) return true;
    return sys.getNamespace(namespace) != null;
}

/// Marks all instances as unloadable.
pub fn pruneInstances() !void {
    tracing.emitTraceSimple("pruning instances", .{}, @src());
    sys.lock();
    defer sys.unlock();
    try sys.pruneInstances();
}

/// Queries the info of a module parameter.
pub fn queryParameter(owner: []const u8, parameter: []const u8) error{NotFound}!struct {
    type: pub_modules.ParameterType,
    read_group: pub_modules.ParameterAccessGroup,
    write_group: pub_modules.ParameterAccessGroup,
} {
    tracing.emitTraceSimple(
        "querying parameter, owner='{s}', parameter='{s}'",
        .{ owner, parameter },
        @src(),
    );
    sys.lock();
    defer sys.unlock();

    const owner_instance = sys.getInstance(owner) orelse return error.NotFound;
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
    value: *anyopaque,
    @"type": pub_modules.ParameterType,
    owner: []const u8,
    parameter: []const u8,
) !void {
    tracing.emitTraceSimple(
        "reading public parameter, value='{*}', type='{s}', owner='{s}', parameter='{s}'",
        .{ value, @tagName(@"type"), owner, parameter },
        @src(),
    );
    sys.lock();
    defer sys.unlock();

    const owner_instance = sys.getInstance(owner) orelse return error.NotFound;
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
    value: *const anyopaque,
    @"type": pub_modules.ParameterType,
    owner: []const u8,
    parameter: []const u8,
) !void {
    tracing.emitTraceSimple(
        "write public parameter, value='{*}', type='{s}', owner='{s}', parameter='{s}'",
        .{ value, @tagName(@"type"), owner, parameter },
        @src(),
    );
    sys.lock();
    defer sys.unlock();

    const owner_instance = sys.getInstance(owner) orelse return error.NotFound;
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
        std.debug.assert(context.is_init);
        return sys.profile;
    }
    fn features(out: *?[*]const pub_modules.FeatureStatus) callconv(.c) usize {
        std.debug.assert(context.is_init);
        out.* = &sys.features;
        return sys.features.len;
    }
    fn addPseudoInstance(instance: **const pub_modules.PseudoInstance) callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        instance.* = modules.addPseudoInstance() catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn addLoadingSet(set: *pub_modules.LoadingSet) callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        set.* = LoadingSet.init() catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn findInstanceByName(
        name: [*:0]const u8,
        info: **const pub_modules.Info,
    ) callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        info.* = modules.findInstanceByName(std.mem.span(name)) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
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
        std.debug.assert(context.is_init);
        info.* = modules.findInstanceBySymbol(
            std.mem.span(name),
            std.mem.span(namespace),
            Version.initC(version),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn queryNamespace(namespace: [*:0]const u8, exists: *bool) callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        exists.* = modules.queryNamespace(std.mem.span(namespace));
        return AnyResult.ok;
    }
    fn pruneInstances() callconv(.c) AnyResult {
        std.debug.assert(context.is_init);
        modules.pruneInstances() catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
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
        std.debug.assert(context.is_init);
        const info = modules.queryParameter(
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
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
        std.debug.assert(context.is_init);
        modules.readParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
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
        std.debug.assert(context.is_init);
        modules.writeParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
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
