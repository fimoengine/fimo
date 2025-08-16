const std = @import("std");
const Allocator = std.mem.Allocator;
const ArenaAllocator = std.heap.ArenaAllocator;
const Mutex = std.Thread.Mutex;

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const context = @import("../context.zig");
const pub_context = @import("../ctx.zig");
const pub_modules = @import("../modules.zig");
const Path = @import("../paths.zig").Path;
const pub_tasks = @import("../tasks.zig");
const EnqueuedFuture = pub_tasks.EnqueuedFuture;
const Fallible = pub_tasks.Fallible;
const Version = @import("../Version.zig");
const graph = @import("graph.zig");
const InstanceHandle = @import("modules/InstanceHandle.zig");
const LoadingSet = @import("modules/LoadingSet.zig");
const ModuleHandle = @import("modules/ModuleHandle.zig");
const SymbolRef = @import("modules/SymbolRef.zig");
const ResourceCount = @import("ResourceCount.zig");
const tasks = @import("tasks.zig");
const tmp_path = @import("tmp_path.zig");
const tracing = @import("tracing.zig");

const modules = @This();

pub var mutex: Mutex = .{};
pub var allocator: Allocator = undefined;
var arena: ArenaAllocator = undefined;
var profile: pub_modules.Profile = undefined;
var features: [feature_count]pub_modules.FeatureStatus = undefined;
pub var state: enum { idle, loading_set } = .idle;
pub var instance_count: ResourceCount = .{};
pub var loading_set_count: ResourceCount = .{};
pub var loading_set_waiters: std.ArrayList(LoadingSetWaiter) = .empty;
var dep_graph: graph.GraphUnmanaged(*const pub_modules.OpaqueInstance, void) = undefined;
var string_cache: std.StringArrayHashMapUnmanaged(void) = .empty;
var instances: std.StringArrayHashMapUnmanaged(InstanceRef) = .empty;
var namespaces: std.StringArrayHashMapUnmanaged(NamespaceInfo) = .empty;
var symbols: std.ArrayHashMapUnmanaged(SymbolRef.Id, SymbolRef, SymbolRef.Id.HashContext, false) = .empty;

pub const global_namespace = "";
const feature_count = std.meta.fields(pub_modules.FeatureTag).len;

pub const LoadingSetWaiter = struct {
    waiter: *anyopaque,
    waker: pub_tasks.Waker,
};

const InstanceRef = struct {
    id: graph.NodeId,
    instance: *const pub_modules.OpaqueInstance,
};

const NamespaceInfo = struct {
    num_symbols: usize,
    num_references: usize,
};

pub fn init(config: *const pub_modules.Config) !void {
    allocator = context.allocator;
    arena = .init(context.allocator);
    profile = switch (config.profile) {
        .release, .dev => |x| x,
        else => |x| {
            tracing.logErr(@src(), "unknown profile, profile='{}'", .{@intFromEnum(x)});
            return error.InvalidConfig;
        },
    };

    for (&features, 0..) |*status, i| {
        status.tag = @enumFromInt(i);
        status.flag = .off;
    }
    if (config.features) |cfg_features| for (cfg_features[0..config.feature_count]) |request| {
        switch (request.tag) {
            else => |tag| {
                if (request.flag == .required) {
                    tracing.logErr(@src(), "unknown feature was marked as required, feature='{}'", .{tag});
                    return error.InvaldConfig;
                } else {
                    tracing.logWarn(@src(), "unknown feature...ignoring, feature='{}'", .{tag});
                }
            },
        }
    };
    for (&features) |feature| {
        tracing.logDebug(@src(), "module subsystem feature=`{t}`, status=`{t}`", .{ feature.tag, feature.flag });
    }
    dep_graph = .init(null, null);
}

pub fn deinit() void {
    pruneInstances() catch |err| @panic(@errorName(err));
    instance_count.waitUntilZero();
    loading_set_count.waitUntilZero();
    std.debug.assert(state == .idle);
    std.debug.assert(loading_set_waiters.items.len == 0);

    profile = undefined;
    features = undefined;

    loading_set_waiters.clearAndFree(allocator);
    dep_graph.clear(allocator);

    string_cache = .empty;
    instances = .empty;
    namespaces = .empty;
    symbols = .empty;

    arena.deinit();
    arena = undefined;
    allocator = undefined;
}

fn cacheString(value: []const u8) Allocator.Error![]const u8 {
    if (string_cache.getKey(value)) |v| return v;
    const alloc = arena.allocator();
    const cached = try alloc.dupe(u8, value);
    errdefer alloc.free(cached);
    try string_cache.put(alloc, cached, {});
    return cached;
}

pub fn getInstance(name: []const u8) ?*InstanceRef {
    return instances.getPtr(name);
}

pub fn addInstance(inner: *InstanceHandle.Inner) !void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    if (instances.contains(std.mem.span(handle.info.name))) return error.Duplicate;

    const instance = inner.instance.?;
    const node = try dep_graph.addNode(allocator, instance);
    errdefer _ = dep_graph.removeNode(allocator, node) catch unreachable;

    // Validate symbols and namespaces.
    for (inner.symbols.keys()) |key| {
        if (getSymbol(key.name, key.namespace) != null) return error.Duplicate;
    }
    for (inner.namespaces.keys()) |ns| if (getNamespace(ns) == null) return error.NotFound;

    // Acquire all imported namespaces.
    for (inner.namespaces.keys()) |ns| refNamespace(ns) catch unreachable;
    errdefer for (inner.namespaces.keys()) |ns| unrefNamespace(ns);

    // Insert all dependencies in the dependency graph.
    var dep_it = inner.dependencies.iterator();
    while (dep_it.next()) |entry| {
        const data = getInstance(entry.key_ptr.*) orelse return error.NotFound;
        if (&entry.value_ptr.instance.info != data.instance.info) @panic("unexpected instance info");
        _ = dep_graph.addEdge(
            allocator,
            {},
            node,
            data.id,
        ) catch |err| switch (err) {
            Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
            else => unreachable,
        };
    }
    if (inner.@"export") |exp| {
        for (exp.getModifiers()) |mod| {
            if (mod.tag != .dependency) continue;
            const dependency = mod.value.dependency;
            const dep_instance = getInstance(
                std.mem.span(dependency.name),
            ) orelse return error.NotFound;
            if (dep_instance.instance.info != dependency) @panic("unexpected instance info");
            _ = dep_graph.addEdge(
                allocator,
                {},
                node,
                dep_instance.id,
            ) catch |err| switch (err) {
                Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
                else => unreachable,
            };
        }
    }
    if (try dep_graph.isCyclic(allocator)) return error.CyclicDependency;

    // Allocate all exported namespaces.
    errdefer for (inner.symbols.keys()) |key| cleanupUnusedNamespace(key.namespace);
    for (inner.symbols.keys()) |key| try ensureInitNamespace(key.namespace);

    // Export all symbols.
    errdefer for (inner.symbols.keys()) |key| {
        if (getSymbol(key.name, key.namespace) != null)
            removeSymbol(key.name, key.namespace);
    };
    var sym_it = inner.symbols.iterator();
    while (sym_it.next()) |entry| {
        try addSymbol(
            entry.key_ptr.name,
            entry.key_ptr.namespace,
            entry.value_ptr.version,
            std.mem.span(instance.info.name),
        );
    }

    const data = InstanceRef{ .id = node, .instance = instance };
    const key = try cacheString(std.mem.span(instance.info.name));
    try instances.put(arena.allocator(), key, data);
}

/// Adds a new root instance.
///
/// The root instance provides access to the subsystem for non-instances, and is mainly intended
/// for bootstrapping.
pub fn addRootInstance() !*const pub_modules.RootInstance {
    tracing.logTrace(@src(), "adding new root instance", .{});
    mutex.lock();
    defer mutex.unlock();

    var name_buf: [32]u8 = undefined;
    var name: []u8 = undefined;
    while (true) {
        var random_bytes: [8]u8 = undefined;
        std.crypto.random.bytes(&random_bytes);
        var suffix: [std.fs.base64_encoder.calcSize(8)]u8 = undefined;
        _ = std.fs.base64_encoder.encode(&suffix, &random_bytes);
        name = std.fmt.bufPrint(&name_buf, "_root_{s}", .{suffix}) catch unreachable;
        if (getInstance(name) == null) break;
    }

    const instance = try InstanceHandle.initRootInstance(name);
    const handle = InstanceHandle.fromInstancePtr(&instance.instance);
    const inner = handle.lock();
    errdefer {
        inner.stop();
        inner.deinit();
    }

    try addInstance(inner);
    inner.unlock();
    return instance;
}

pub fn removeInstance(inner: *InstanceHandle.Inner) !void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    if (!inner.canUnload()) return error.NotPermitted;
    if (!instances.contains(std.mem.span(handle.info.name))) return error.NotFound;

    for (inner.symbols.keys()) |key| removeSymbol(key.name, key.namespace);
    for (inner.namespaces.keys()) |ns| unrefNamespace(ns);
    errdefer {
        for (inner.namespaces.keys()) |ns| refNamespace(ns) catch |e| @panic(@errorName(e));
        var symbols_it = inner.symbols.iterator();
        while (symbols_it.next()) |entry| {
            addSymbol(
                entry.key_ptr.name,
                entry.key_ptr.namespace,
                entry.value_ptr.version,
                std.mem.span(handle.info.name),
            ) catch |e| @panic(@errorName(e));
        }
    }

    for (inner.symbols.keys()) |key| {
        if (std.mem.eql(u8, key.namespace, global_namespace)) continue;
        if (namespaces.getPtr(key.namespace)) |ns| {
            if (ns.num_references != 0 and ns.num_symbols == 0) return error.InUse;
        }
    }
    inner.clearDependencies();

    const instance_id = instances.fetchSwapRemove(std.mem.span(handle.info.name)).?.value.id;
    _ = dep_graph.removeNode(allocator, instance_id) catch |err| @panic(@errorName(err));
}

pub fn linkInstances(inner: *InstanceHandle.Inner, other: *InstanceHandle.Inner) !void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    const other_handle = InstanceHandle.fromInnerPtr(other);
    if (inner.isDetached() or other.isDetached()) return error.NotFound;
    if (inner.getDependency(std.mem.span(other_handle.info.name)) != null) return error.Duplicate;
    if (other_handle.type == .root) return error.NotPermitted;

    const instance_ref = getInstance(std.mem.span(handle.info.name)).?;
    const other_instance_ref = getInstance(std.mem.span(other_handle.info.name)).?;

    const would_cycle = dep_graph.pathExists(
        allocator,
        other_instance_ref.id,
        instance_ref.id,
    ) catch |err| switch (err) {
        Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
        else => unreachable,
    };
    if (would_cycle) return error.CyclicDependency;

    const edge = dep_graph.addEdge(
        allocator,
        {},
        instance_ref.id,
        other_instance_ref.id,
    ) catch |err| switch (err) {
        Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
        else => unreachable,
    };
    errdefer _ = dep_graph.removeEdge(allocator, edge.id) catch unreachable;
    try inner.addDependency(other, .dynamic);
}

pub fn unlinkInstances(inner: *InstanceHandle.Inner, other: *InstanceHandle.Inner) !void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    const other_handle = InstanceHandle.fromInnerPtr(other);

    const dependency_info = inner.getDependency(
        std.mem.span(other_handle.info.name),
    ) orelse return error.NotADependency;
    if (dependency_info.type == .static) return error.NotPermitted;

    const instance_ref = getInstance(std.mem.span(handle.info.name)).?;
    const other_instance_ref = getInstance(std.mem.span(other_handle.info.name)).?;

    const edge = (dep_graph.findEdge(
        instance_ref.id,
        other_instance_ref.id,
    ) catch unreachable).?;
    _ = dep_graph.removeEdge(allocator, edge) catch unreachable;
    inner.removeDependency(other) catch unreachable;
}

/// Marks all instances as unloadable.
pub fn pruneInstances() !void {
    tracing.logTrace(@src(), "pruning instances", .{});
    mutex.lock();
    defer mutex.unlock();

    const nodes = dep_graph.sortTopological(allocator, .incoming) catch |err| switch (err) {
        Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
        else => unreachable,
    };
    defer allocator.free(nodes);

    for (nodes) |node| {
        const instance = dep_graph.nodePtr(node).?.*;
        const handle = InstanceHandle.fromInstancePtr(instance);
        if (handle.type != .regular) continue;

        const inner = handle.lock();
        var unlock_inner = true;
        defer if (unlock_inner) inner.unlock();

        if (inner.isUnloading()) continue;
        if (!inner.canUnload()) {
            inner.enqueueUnload() catch return error.EnqueueError;
            continue;
        }
        tracing.logTrace(@src(), "unloading unused instance, instance='{s}'", .{instance.info.name});
        try removeInstance(inner);
        inner.stop();
        inner.deinit();
        unlock_inner = false;
    }
}

/// Searches for an instance by its name.
pub fn findInstanceByName(name: []const u8) !*const pub_modules.Info {
    tracing.logTrace(@src(), "searching for instance, name='{s}'", .{name});
    mutex.lock();
    defer mutex.unlock();

    const instance_ref = getInstance(name) orelse return error.NotFound;
    instance_ref.instance.info.ref();
    return instance_ref.instance.info;
}

/// Searches for the instance that exports a specific symbol.
pub fn findInstanceBySymbol(name: []const u8, namespace: []const u8, version: Version) !*const pub_modules.Info {
    tracing.logTrace(@src(), "searching for symbol owner, name='{s}', namespace='{s}', version='{f}'", .{ name, namespace, version });
    mutex.lock();
    defer mutex.unlock();

    const symbol_ref = getSymbolCompatible(
        name,
        namespace,
        version,
    ) orelse return error.NotFound;
    const instance_ref = getInstance(symbol_ref.owner) orelse unreachable;
    instance_ref.instance.info.ref();
    return instance_ref.instance.info;
}

/// Queries whether a namespace exists.
///
/// To exist, the namespace must contain at least one symbol.
/// The global namespace always exist.
pub fn queryNamespace(namespace: []const u8) bool {
    tracing.logTrace(@src(), "querying namespace, namespace='{s}'", .{namespace});
    mutex.lock();
    defer mutex.unlock();

    if (std.mem.eql(u8, namespace, global_namespace)) return true;
    return getNamespace(namespace) != null;
}

pub fn getNamespace(name: []const u8) ?*NamespaceInfo {
    return namespaces.getPtr(name);
}

fn ensureInitNamespace(name: []const u8) !void {
    if (std.mem.eql(u8, name, global_namespace)) return;
    if (namespaces.contains(name)) return;

    const key = try cacheString(name);
    const ns = NamespaceInfo{ .num_symbols = 0, .num_references = 0 };
    try namespaces.put(arena.allocator(), key, ns);
}

fn cleanupUnusedNamespace(namespace: []const u8) void {
    if (std.mem.eql(u8, namespace, global_namespace)) return;
    if (getNamespace(namespace)) |ns| {
        if (ns.num_symbols == 0 and ns.num_references == 0) {
            if (!namespaces.swapRemove(namespace)) unreachable;
        }
    }
}

pub fn refNamespace(name: []const u8) !void {
    if (std.mem.eql(u8, name, global_namespace)) return;
    const ns = getNamespace(name) orelse return error.NotFound;
    ns.num_references += 1;
}

pub fn unrefNamespace(name: []const u8) void {
    if (std.mem.eql(u8, name, global_namespace)) return;
    const ns: *NamespaceInfo = getNamespace(name) orelse @panic(@errorName(error.NotFound));
    ns.num_references -= 1;
    cleanupUnusedNamespace(name);
}

pub fn getSymbol(name: []const u8, namespace: []const u8) ?*SymbolRef {
    return symbols.getPtr(.{ .name = name, .namespace = namespace });
}

pub fn getSymbolCompatible(name: []const u8, namespace: []const u8, version: Version) ?*SymbolRef {
    const symbol = getSymbol(name, namespace) orelse return null;
    if (!symbol.version.isCompatibleWith(version)) return null;
    return symbol;
}

fn addSymbol(
    name: []const u8,
    namespace: []const u8,
    version: Version,
    owner: []const u8,
) !void {
    if (getSymbol(name, namespace) != null) return error.Duplicate;
    const key = SymbolRef.Id{
        .name = try cacheString(name),
        .namespace = try cacheString(namespace),
    };
    const symbol = SymbolRef{
        .owner = try cacheString(owner),
        .version = version,
    };
    try symbols.put(arena.allocator(), key, symbol);
    errdefer _ = symbols.swapRemove(key);

    if (std.mem.eql(u8, namespace, global_namespace)) return;
    const ns = getNamespace(namespace) orelse return error.NotFound;
    ns.num_symbols += 1;
}

fn removeSymbol(name: []const u8, namespace: []const u8) void {
    if (!symbols.swapRemove(.{ .name = name, .namespace = namespace })) unreachable;
    if (std.mem.eql(u8, namespace, global_namespace)) return;
    const ns = getNamespace(namespace) orelse unreachable;
    ns.num_symbols -= 1;
    cleanupUnusedNamespace(namespace);
}

/// Initializes a new empty loading set.
pub fn addLoadingSet(err: *?AnyError) !EnqueuedFuture(Fallible(*LoadingSet)) {
    tracing.logTrace(@src(), "creating new loading set", .{});
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

/// Queries the info of a module parameter.
pub fn queryParameter(owner: []const u8, parameter: []const u8) error{NotFound}!struct {
    type: pub_modules.ParameterType,
    read_group: pub_modules.ParameterAccessGroup,
    write_group: pub_modules.ParameterAccessGroup,
} {
    tracing.logTrace(@src(), "querying parameter, owner='{s}', parameter='{s}'", .{ owner, parameter });
    mutex.lock();
    defer mutex.unlock();

    const owner_instance = getInstance(owner) orelse return error.NotFound;
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
    tracing.logTrace(
        @src(),
        "reading public parameter, value='{*}', type='{s}', owner='{s}', parameter='{s}'",
        .{ value, @tagName(@"type"), owner, parameter },
    );
    mutex.lock();
    defer mutex.unlock();

    const owner_instance = getInstance(owner) orelse return error.NotFound;
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
    tracing.logTrace(
        @src(),
        "write public parameter, value='{*}', type='{s}', owner='{s}', parameter='{s}'",
        .{ value, @tagName(@"type"), owner, parameter },
    );
    mutex.lock();
    defer mutex.unlock();

    const owner_instance = getInstance(owner) orelse return error.NotFound;
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
        return modules.profile;
    }
    fn features(out: *?[*]const pub_modules.FeatureStatus) callconv(.c) usize {
        std.debug.assert(context.is_init);
        out.* = &modules.features;
        return modules.features.len;
    }
    fn addRootInstance(instance: **const pub_modules.RootInstance) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        instance.* = modules.addRootInstance() catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn addLoadingSet(set: *pub_modules.LoadingSet) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        set.* = LoadingSet.init() catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn findInstanceByName(
        name: [*:0]const u8,
        info: **const pub_modules.Info,
    ) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        info.* = modules.findInstanceByName(std.mem.span(name)) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn findInstanceBySymbol(
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: Version.CVersion,
        info: **const pub_modules.Info,
    ) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        info.* = modules.findInstanceBySymbol(
            std.mem.span(name),
            std.mem.span(namespace),
            Version.initC(version),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn queryNamespace(namespace: [*:0]const u8, exists: *bool) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        exists.* = modules.queryNamespace(std.mem.span(namespace));
        return .ok;
    }
    fn pruneInstances() callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        modules.pruneInstances() catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn queryParameter(
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
        @"type": *pub_modules.ParameterType,
        read_group: *pub_modules.ParameterAccessGroup,
        write_group: *pub_modules.ParameterAccessGroup,
    ) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        const info = modules.queryParameter(
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        @"type".* = info.type;
        read_group.* = info.read_group;
        write_group.* = info.write_group;
        return .ok;
    }
    fn readParameter(
        value: *anyopaque,
        @"type": pub_modules.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        modules.readParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn writeParameter(
        value: *const anyopaque,
        @"type": pub_modules.ParameterType,
        owner: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) pub_context.Status {
        std.debug.assert(context.is_init);
        modules.writeParameter(
            value,
            @"type",
            std.mem.span(owner),
            std.mem.span(parameter),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
};

pub const vtable = pub_modules.VTable{
    .profile = &VTableImpl.profile,
    .features = &VTableImpl.features,
    .root_module_new = &VTableImpl.addRootInstance,
    .set_new = &VTableImpl.addLoadingSet,
    .find_by_name = &VTableImpl.findInstanceByName,
    .find_by_symbol = &VTableImpl.findInstanceBySymbol,
    .namespace_exists = &VTableImpl.queryNamespace,
    .prune_instances = &VTableImpl.pruneInstances,
    .query_parameter = &VTableImpl.queryParameter,
    .read_parameter = &VTableImpl.readParameter,
    .write_parameter = &VTableImpl.writeParameter,
};
