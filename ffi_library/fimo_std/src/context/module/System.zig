const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;

const AnyError = @import("../../AnyError.zig");
const Context = @import("../../context.zig");
const Version = @import("../../Version.zig");
const Async = @import("../async.zig");
const graph = @import("../graph.zig");
const Module = @import("../module.zig");
const ProxyAsync = @import("../proxy_context/async.zig");
const ProxyModule = @import("../proxy_context/module.zig");
const tmp_path = @import("../tmp_path.zig");
const InstanceHandle = @import("InstanceHandle.zig");
const LoadingSet = @import("LoadingSet.zig");
const SymbolRef = @import("SymbolRef.zig");

pub const global_namespace = "";
const Self = @This();

mutex: Mutex = .{},
allocator: Allocator,
tmp_dir: tmp_path.TmpDirUnmanaged,
state: enum { idle, loading_set } = .idle,
loading_set_waiters: std.ArrayListUnmanaged(LoadingSetWaiter) = .{},
dep_graph: graph.GraphUnmanaged(*const ProxyModule.OpaqueInstance, void),
instances: std.StringArrayHashMapUnmanaged(InstanceRef) = .{},
namespaces: std.StringArrayHashMapUnmanaged(NamespaceInfo) = .{},
symbols: std.ArrayHashMapUnmanaged(
    SymbolRef.Id,
    SymbolRef,
    SymbolRef.Id.HashContext,
    false,
) = .{},

pub const SystemError = error{
    InUse,
    Duplicate,
    NotFound,
    NotPermitted,
    NotADependency,
    CyclicDependency,
    LoadingInProcess,
} || Allocator.Error;

pub const LoadingSetWaiter = struct {
    waiter: *anyopaque,
    waker: ProxyAsync.Waker,
};

const InstanceRef = struct {
    id: graph.NodeId,
    instance: *const ProxyModule.OpaqueInstance,
};

const NamespaceInfo = struct {
    num_symbols: usize,
    num_references: usize,
};

pub fn init(ctx: *const Context) (SystemError || tmp_path.TmpDirError)!Self {
    var module = Self{
        .allocator = ctx.allocator,
        .tmp_dir = undefined,
        .dep_graph = graph.GraphUnmanaged(
            *const ProxyModule.OpaqueInstance,
            void,
        ).init(null, null),
    };

    module.tmp_dir = try tmp_path.TmpDirUnmanaged.init(module.allocator, "fimo_modules_");
    ctx.tracing.emitTraceSimple(
        "module subsystem tmp dir: {s}",
        .{module.tmp_dir.path.raw},
        @src(),
    );

    return module;
}

pub fn deinit(self: *Self) void {
    std.debug.assert(self.state == .idle);
    std.debug.assert(self.loading_set_waiters.items.len == 0);

    self.tmp_dir.deinit(self.allocator);
    self.loading_set_waiters.deinit(self.allocator);
    self.dep_graph.deinit(self.allocator);
    while (self.instances.popOrNull()) |entry| self.allocator.free(entry.key);
    while (self.namespaces.popOrNull()) |entry| self.allocator.free(entry.key);
    while (self.symbols.popOrNull()) |*entry| {
        entry.key.deinit(self.allocator);
        entry.value.deinit(self.allocator);
    }

    self.instances.clearAndFree(self.allocator);
    self.namespaces.clearAndFree(self.allocator);
    self.symbols.clearAndFree(self.allocator);
}

pub fn lock(self: *Self) void {
    self.mutex.lock();
}

pub fn unlock(self: *Self) void {
    self.mutex.unlock();
}

pub fn asContext(self: *Self) *Context {
    const module: *Module = @fieldParentPtr("sys", self);
    return module.asContext();
}

pub fn logTrace(self: *Self, comptime fmt: []const u8, args: anytype, location: std.builtin.SourceLocation) void {
    self.asContext().tracing.emitTraceSimple(fmt, args, location);
}

pub fn logWarn(self: *Self, comptime fmt: []const u8, args: anytype, location: std.builtin.SourceLocation) void {
    self.asContext().tracing.emitWarnSimple(fmt, args, location);
}

pub fn logError(self: *Self, comptime fmt: []const u8, args: anytype, location: std.builtin.SourceLocation) void {
    self.asContext().tracing.emitErrSimple(fmt, args, location);
}

pub fn getInstance(self: *Self, name: []const u8) ?*InstanceRef {
    return self.instances.getPtr(name);
}

pub fn addInstance(self: *Self, inner: *InstanceHandle.Inner) SystemError!void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    if (self.instances.contains(std.mem.span(handle.info.name))) return error.Duplicate;

    const instance = inner.instance.?;
    const node = try self.dep_graph.addNode(self.allocator, instance);
    errdefer _ = self.dep_graph.removeNode(self.allocator, node) catch unreachable;

    // Validate symbols and namespaces.
    for (inner.symbols.keys()) |key| {
        if (self.getSymbol(key.name, key.namespace) != null) return error.Duplicate;
    }
    for (inner.namespaces.keys()) |ns| if (self.getNamespace(ns) == null) return error.NotFound;

    // Acquire all imported namespaces.
    for (inner.namespaces.keys()) |ns| self.refNamespace(ns) catch unreachable;
    errdefer for (inner.namespaces.keys()) |ns| self.unrefNamespace(ns);

    // Insert all dependencies in the dependency graph.
    var dep_it = inner.dependencies.iterator();
    while (dep_it.next()) |entry| {
        const data = self.getInstance(entry.key_ptr.*) orelse return error.NotFound;
        if (&entry.value_ptr.instance.info != data.instance.info) @panic("unexpected instance info");
        _ = self.dep_graph.addEdge(
            self.allocator,
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
            const dep_instance = self.getInstance(
                std.mem.span(dependency.name),
            ) orelse return error.NotFound;
            if (dep_instance.instance.info != dependency) @panic("unexpected instance info");
            _ = self.dep_graph.addEdge(
                self.allocator,
                {},
                node,
                dep_instance.id,
            ) catch |err| switch (err) {
                Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
                else => unreachable,
            };
        }
    }
    if (try self.dep_graph.isCyclic(self.allocator)) return error.CyclicDependency;

    // Allocate all exported namespaces.
    errdefer for (inner.symbols.keys()) |key| self.cleanupUnusedNamespace(key.namespace);
    for (inner.symbols.keys()) |key| try self.ensureInitNamespace(key.namespace);

    // Export all symbols.
    errdefer for (inner.symbols.keys()) |key| {
        if (self.getSymbol(key.name, key.namespace) != null)
            self.removeSymbol(key.name, key.namespace);
    };
    var sym_it = inner.symbols.iterator();
    while (sym_it.next()) |entry| {
        try self.addSymbol(
            entry.key_ptr.name,
            entry.key_ptr.namespace,
            entry.value_ptr.version,
            std.mem.span(instance.info.name),
        );
    }

    const data = InstanceRef{ .id = node, .instance = instance };
    const key = try self.allocator.dupe(u8, std.mem.span(instance.info.name));
    errdefer self.allocator.free(key);
    try self.instances.put(self.allocator, key, data);
}

pub fn removeInstance(self: *Self, inner: *InstanceHandle.Inner) SystemError!void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    if (!inner.canUnload()) return error.NotPermitted;
    if (!self.instances.contains(std.mem.span(handle.info.name))) return error.NotFound;

    for (inner.symbols.keys()) |key| self.removeSymbol(key.name, key.namespace);
    for (inner.namespaces.keys()) |ns| self.unrefNamespace(ns);
    errdefer {
        for (inner.namespaces.keys()) |ns| self.refNamespace(ns) catch |e| @panic(@errorName(e));
        var symbols_it = inner.symbols.iterator();
        while (symbols_it.next()) |entry| {
            self.addSymbol(
                entry.key_ptr.name,
                entry.key_ptr.namespace,
                entry.value_ptr.version,
                std.mem.span(handle.info.name),
            ) catch |e| @panic(@errorName(e));
        }
    }

    for (inner.symbols.keys()) |key| {
        if (std.mem.eql(u8, key.namespace, global_namespace)) continue;
        if (self.namespaces.getPtr(key.namespace)) |ns| {
            if (ns.num_references != 0 and ns.num_symbols == 0) return error.InUse;
        }
    }
    inner.clearDependencies();

    const instance = self.instances.fetchSwapRemove(std.mem.span(handle.info.name)).?;
    self.allocator.free(instance.key);
    _ = self.dep_graph.removeNode(self.allocator, instance.value.id) catch |err|
        @panic(@errorName(err));
}

pub fn linkInstances(
    self: *Self,
    inner: *InstanceHandle.Inner,
    other: *InstanceHandle.Inner,
) (SystemError || InstanceHandle.InstanceHandleError)!void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    const other_handle = InstanceHandle.fromInnerPtr(other);
    if (inner.isDetached() or other.isDetached()) return error.NotFound;
    if (inner.getDependency(std.mem.span(other_handle.info.name)) != null) return error.Duplicate;
    if (other_handle.type == .pseudo) return error.NotPermitted;

    const instance_ref = self.getInstance(std.mem.span(handle.info.name)).?;
    const other_instance_ref = self.getInstance(std.mem.span(other_handle.info.name)).?;

    const would_cycle = self.dep_graph.pathExists(
        self.allocator,
        other_instance_ref.id,
        instance_ref.id,
    ) catch |err| switch (err) {
        Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
        else => unreachable,
    };
    if (would_cycle) return error.CyclicDependency;

    const edge = self.dep_graph.addEdge(
        self.allocator,
        {},
        instance_ref.id,
        other_instance_ref.id,
    ) catch |err| switch (err) {
        Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
        else => unreachable,
    };
    errdefer _ = self.dep_graph.removeEdge(self.allocator, edge.id) catch unreachable;
    try inner.addDependency(other, .dynamic);
}

pub fn unlinkInstances(self: *Self, inner: *InstanceHandle.Inner, other: *InstanceHandle.Inner) SystemError!void {
    const handle = InstanceHandle.fromInnerPtr(inner);
    const other_handle = InstanceHandle.fromInnerPtr(other);

    const dependency_info = inner.getDependency(
        std.mem.span(other_handle.info.name),
    ) orelse return error.NotADependency;
    if (dependency_info.type == .static) return error.NotPermitted;

    const instance_ref = self.getInstance(std.mem.span(handle.info.name)).?;
    const other_instance_ref = self.getInstance(std.mem.span(other_handle.info.name)).?;

    const edge = (self.dep_graph.findEdge(
        instance_ref.id,
        other_instance_ref.id,
    ) catch unreachable).?;
    _ = self.dep_graph.removeEdge(self.allocator, edge) catch unreachable;
    inner.removeDependency(other) catch unreachable;
}

pub fn cleanupLooseInstances(self: *Self) SystemError!void {
    var it = self.dep_graph.externalsIterator(.incoming);
    while (it.next()) |entry| {
        const instance = entry.data_ptr.*;
        const handle = InstanceHandle.fromInstancePtr(instance);
        if (handle.type != .regular) continue;

        const inner = handle.lock();
        var unlock_inner = true;
        defer if (unlock_inner) inner.unlock();

        if (!inner.canUnload()) continue;
        self.logTrace(
            "unloading unused instance, instance='{s}'",
            .{instance.info.name},
            @src(),
        );
        try self.removeInstance(inner);
        inner.stop(self);
        inner.deinit();
        unlock_inner = false;

        // Rebuild the iterator.
        it = self.dep_graph.externalsIterator(.incoming);
    }
}

pub fn getNamespace(self: *Self, name: []const u8) ?*NamespaceInfo {
    return self.namespaces.getPtr(name);
}

fn ensureInitNamespace(self: *Self, name: []const u8) SystemError!void {
    if (std.mem.eql(u8, name, global_namespace)) return;
    if (self.namespaces.contains(name)) return;

    const key = try self.allocator.dupe(u8, name);
    const ns = NamespaceInfo{ .num_symbols = 0, .num_references = 0 };
    try self.namespaces.put(self.allocator, key, ns);
}

fn cleanupUnusedNamespace(self: *Self, namespace: []const u8) void {
    if (std.mem.eql(u8, namespace, global_namespace)) return;
    if (self.getNamespace(namespace)) |ns| {
        if (ns.num_symbols == 0 and ns.num_references == 0) {
            const kv = self.namespaces.fetchSwapRemove(namespace).?;
            self.allocator.free(kv.key);
        }
    }
}

pub fn refNamespace(self: *Self, name: []const u8) SystemError!void {
    if (std.mem.eql(u8, name, global_namespace)) return;
    const ns = self.getNamespace(name) orelse return error.NotFound;
    ns.num_references += 1;
}

pub fn unrefNamespace(self: *Self, name: []const u8) void {
    if (std.mem.eql(u8, name, global_namespace)) return;
    const ns: *NamespaceInfo = self.getNamespace(name) orelse @panic(@errorName(error.NotFound));
    ns.num_references -= 1;
    self.cleanupUnusedNamespace(name);
}

pub fn getSymbol(self: *Self, name: []const u8, namespace: []const u8) ?*SymbolRef {
    return self.symbols.getPtr(.{ .name = @constCast(name), .namespace = @constCast(namespace) });
}

pub fn getSymbolCompatible(self: *Self, name: []const u8, namespace: []const u8, version: Version) ?*SymbolRef {
    const symbol = self.getSymbol(name, namespace) orelse return null;
    if (!symbol.version.isCompatibleWith(version)) return null;
    return symbol;
}

fn addSymbol(
    self: *Self,
    name: []const u8,
    namespace: []const u8,
    version: Version,
    owner: []const u8,
) SystemError!void {
    if (self.getSymbol(name, namespace) != null) return error.Duplicate;
    const key = try SymbolRef.Id.init(self.allocator, name, namespace);
    errdefer key.deinit(self.allocator);
    const symbol = try SymbolRef.init(self.allocator, owner, version);
    errdefer symbol.deinit(self.allocator);
    try self.symbols.put(self.allocator, key, symbol);
    errdefer _ = self.symbols.swapRemove(key);

    if (std.mem.eql(u8, namespace, global_namespace)) return;
    const ns = self.getNamespace(namespace) orelse return error.NotFound;
    ns.num_symbols += 1;
}

fn removeSymbol(self: *Self, name: []const u8, namespace: []const u8) void {
    var entry = self.symbols.fetchSwapRemove(.{
        .name = @constCast(name),
        .namespace = @constCast(namespace),
    }).?;
    entry.key.deinit(self.allocator);
    entry.value.deinit(self.allocator);

    if (std.mem.eql(u8, namespace, global_namespace)) return;
    const ns = self.getNamespace(namespace) orelse unreachable;
    ns.num_symbols -= 1;
    self.cleanupUnusedNamespace(namespace);
}
