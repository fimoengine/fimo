const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;

const heap = @import("../../heap.zig");
const Path = @import("../../path.zig").Path;
const OwnedPathUnmanaged = @import("../../path.zig").OwnedPathUnmanaged;
const Version = @import("../../Version.zig");

const graph = @import("../graph.zig");

const InstanceHandle = @import("InstanceHandle.zig");
const ModuleHandle = @import("ModuleHandle.zig");
const SymbolRef = @import("SymbolRef.zig");
const System = @import("System.zig");

const Context = @import("../../context.zig");
const ProxyModule = @import("../proxy_context/module.zig");

const allocator = heap.fimo_allocator;
const Self = @This();

mutex: Mutex = .{},
context: *Context,
is_loading: bool = false,
should_recreate_map: bool = false,
module_load_path: OwnedPathUnmanaged,
modules: std.StringArrayHashMapUnmanaged(ModuleInfo) = .{},
symbols: std.ArrayHashMapUnmanaged(
    SymbolRef.Id,
    SymbolRef,
    SymbolRef.Id.HashContext,
    false,
) = .{},

pub const Callback = struct {
    data: ?*anyopaque,
    on_success: *const fn (info: *const ProxyModule.Info, data: ?*anyopaque) callconv(.C) void,
    on_error: *const fn (module: *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) void,
};

const ModuleInfo = struct {
    status: Status,
    handle: *ModuleHandle,
    info: ?*const ProxyModule.Info,
    @"export": *const ProxyModule.Export,
    owner: ?*const ProxyModule.OpaqueInstance,
    callbacks: std.ArrayListUnmanaged(Callback) = .{},

    const Status = enum { unloaded, loaded, err };

    fn init(
        @"export": *const ProxyModule.Export,
        handle: *ModuleHandle,
        owner: ?*const ProxyModule.OpaqueInstance,
    ) !ModuleInfo {
        handle.ref();
        if (owner) |o| {
            const i_handle = InstanceHandle.fromInstancePtr(o);
            const inner = i_handle.lock();
            defer inner.unlock();
            try inner.preventUnload();
        }

        return .{
            .status = .unloaded,
            .handle = handle,
            .info = null,
            .@"export" = @"export",
            .owner = owner,
        };
    }

    fn deinit(self: *ModuleInfo) void {
        for (self.callbacks.items) |callback| {
            std.debug.assert(self.status != .loaded);
            callback.on_error(self.@"export", callback.data);
        }
        self.callbacks.clearAndFree(allocator);
        if (self.status != .loaded) self.@"export".deinit();
        if (self.owner) |owner| {
            const handle = InstanceHandle.fromInstancePtr(owner);
            const inner = handle.lock();
            defer inner.unlock();
            inner.allowUnload();
        }
        self.handle.unref();
    }

    pub fn appendCallback(self: *ModuleInfo, callback: Callback) Allocator.Error!void {
        switch (self.status) {
            .unloaded => try self.callbacks.append(allocator, callback),
            .loaded => callback.on_success(self.info.?, callback.data),
            .err => {
                std.debug.assert(self.info == null);
                callback.on_error(self.@"export", callback.data);
            },
        }
    }

    pub fn signalError(self: *ModuleInfo) void {
        std.debug.assert(self.status == .unloaded);
        self.status = .err;
        while (self.callbacks.popOrNull()) |callback| {
            callback.on_error(self.@"export", callback.data);
        }
    }

    pub fn signalSuccess(self: *ModuleInfo, info: *const ProxyModule.Info) void {
        std.debug.assert(self.status == .unloaded);
        self.status = .loaded;
        while (self.callbacks.popOrNull()) |callback| {
            callback.on_success(info, callback.data);
        }
    }
};

const Queue = std.ArrayListUnmanaged(*ModuleInfo);

pub fn init(context: *Context) Allocator.Error!*Self {
    context.ref();
    errdefer context.unref();

    context.module.sys.lock();
    defer context.module.sys.unlock();
    const module_load_path = try OwnedPathUnmanaged.initPath(
        allocator,
        context.module.sys.tmp_dir.path.asPath(),
    );
    errdefer module_load_path.deinit(allocator);

    const set = try allocator.create(Self);
    set.* = .{
        .context = context,
        .module_load_path = module_load_path,
    };
    return set;
}

pub fn deinit(self: *Self) void {
    std.debug.assert(!self.is_loading);
    while (self.modules.popOrNull()) |*entry| {
        var value = entry.value;
        allocator.free(entry.key);
        value.deinit();
    }
    self.modules.clearAndFree(allocator);
    while (self.symbols.popOrNull()) |*entry| {
        entry.key.deinit();
        entry.value.deinit();
    }
    self.symbols.clearAndFree(allocator);
    self.module_load_path.deinit(allocator);
    self.context.unref();
    allocator.destroy(self);
}

pub fn fromProxySet(set: *ProxyModule.LoadingSet) *Self {
    return @alignCast(@ptrCast(set));
}

pub fn lock(self: *Self) void {
    self.mutex.lock();
}

pub fn unlock(self: *Self) void {
    self.mutex.unlock();
}

fn addModule(self: *Self, module_info: ModuleInfo) Allocator.Error!void {
    const name = try allocator.dupe(u8, module_info.@"export".getName());
    errdefer allocator.free(name);
    try self.modules.put(allocator, name, module_info);
}

pub fn getModule(self: *const Self, name: []const u8) ?*ModuleInfo {
    return self.modules.getPtr(name);
}

fn addSymbol(
    self: *Self,
    name: []const u8,
    namespace: []const u8,
    version: Version,
    owner: []const u8,
) Allocator.Error!void {
    const key = try SymbolRef.Id.init(name, namespace);
    errdefer key.deinit();
    const symbol = try SymbolRef.init(owner, version);
    errdefer symbol.deinit();
    try self.symbols.put(allocator, key, symbol);
    errdefer _ = self.symbols.swapRemove(key);
}

fn removeSymbol(self: *Self, name: []const u8, namespace: []const u8) void {
    var entry = self.symbols.fetchSwapRemove(.{
        .name = @constCast(name),
        .namespace = @constCast(namespace),
    }).?;
    entry.key.deinit();
    entry.value.deinit();
}

pub fn getSymbolAny(self: *const Self, name: []const u8, ns: []const u8) ?*SymbolRef {
    return self.symbols.getPtr(.{ .name = @constCast(name), .namespace = @constCast(ns) });
}

pub fn getSymbol(self: *const Self, name: []const u8, ns: []const u8, version: Version) ?*SymbolRef {
    const symbol = self.getSymbolAny(name, ns) orelse return null;
    if (!symbol.version.isCompatibleWith(version)) return null;
    return symbol;
}

fn addModuleFromExport(
    self: *Self,
    module_handle: *ModuleHandle,
    @"export": *const ProxyModule.Export,
    owner: ?*const ProxyModule.OpaqueInstance,
) !void {
    if (self.getModule(@"export".getName()) != null) return error.Duplicate;
    for (@"export".getSymbolExports()) |exp| {
        const name = std.mem.span(exp.name);
        const namespace = std.mem.span(exp.namespace);
        if (self.getSymbolAny(name, namespace)) |sym| {
            self.context.module.sys.logError(
                "duplicate symbol, owner='{s}', name='{s}', namespace='{s}', version='{long}'",
                .{ sym.owner, name, namespace, sym.version },
                @src(),
            );
            return error.Duplicate;
        }
    }
    for (@"export".getDynamicSymbolExports()) |exp| {
        const name = std.mem.span(exp.name);
        const namespace = std.mem.span(exp.namespace);
        if (self.getSymbolAny(name, namespace)) |sym| {
            self.context.module.sys.logError(
                "duplicate symbol, owner='{s}', name='{s}', namespace='{s}', version='{long}'",
                .{ sym.owner, name, namespace, sym.version },
                @src(),
            );
            return error.Duplicate;
        }
    }
    errdefer {
        for (@"export".getSymbolExports()) |exp| {
            const name = std.mem.span(exp.name);
            const namespace = std.mem.span(exp.namespace);
            if (self.getSymbolAny(name, namespace)) |_| self.removeSymbol(name, namespace);
        }
        for (@"export".getDynamicSymbolExports()) |exp| {
            const name = std.mem.span(exp.name);
            const namespace = std.mem.span(exp.namespace);
            if (self.getSymbolAny(name, namespace)) |_| self.removeSymbol(name, namespace);
        }
    }

    for (@"export".getSymbolExports()) |exp| {
        const name = std.mem.span(exp.name);
        const namespace = std.mem.span(exp.namespace);
        const version = Version.initC(exp.version);
        try self.addSymbol(
            name,
            namespace,
            version,
            @"export".getName(),
        );
    }
    for (@"export".getDynamicSymbolExports()) |exp| {
        const name = std.mem.span(exp.name);
        const namespace = std.mem.span(exp.namespace);
        const version = Version.initC(exp.version);
        try self.addSymbol(
            name,
            namespace,
            version,
            @"export".getName(),
        );
    }

    var module_info = try ModuleInfo.init(@"export", module_handle, owner);
    errdefer module_info.deinit();
    try self.addModule(module_info);
    self.should_recreate_map = true;
}

const AppendModulesData = struct {
    sys: *System,
    err: ?Allocator.Error = null,
    filter_data: ?*anyopaque,
    filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
    exports: std.ArrayListUnmanaged(*const ProxyModule.Export) = .{},
};

fn appendModules(@"export": *const ProxyModule.Export, o_data: ?*anyopaque) callconv(.C) bool {
    // TODO: Validate export.

    const data: *AppendModulesData = @alignCast(@ptrCast(o_data));
    if (data.filter_fn == null or data.filter_fn.?(@"export", data.filter_data)) {
        data.exports.append(allocator, @"export") catch |err| {
            @"export".deinit();
            data.err = err;
            return false;
        };
    }

    return true;
}

pub fn addModuleDynamic(
    self: *Self,
    owner_inner: *InstanceHandle.Inner,
    @"export": *const ProxyModule.Export,
) !void {
    // TODO: Validate export.

    try self.addModuleFromExport(owner_inner.handle.?, @"export", owner_inner.instance.?);
}

pub fn addModulesFromPath(
    self: *Self,
    module_path: ?Path,
    iterator_fn: ModuleHandle.IteratorFn,
    filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
    filter_data: ?*anyopaque,
    bin_ptr: *const anyopaque,
) !void {
    const module_handle = if (module_path) |p|
        try ModuleHandle.initPath(p, self.module_load_path.asPath())
    else
        try ModuleHandle.initLocal(iterator_fn, bin_ptr);
    defer module_handle.unref();

    var append_data = AppendModulesData{
        .sys = &self.context.module.sys,
        .filter_fn = filter_fn,
        .filter_data = filter_data,
    };
    defer append_data.exports.deinit(allocator);
    errdefer for (append_data.exports.items) |exp| exp.deinit();
    module_handle.iterator(&appendModules, &append_data);
    if (append_data.err) |err| return err;

    for (append_data.exports.items) |exp| {
        try self.addModuleFromExport(module_handle, exp, null);
    }
}

pub fn createQueue(self: *const Self, sys: *System) System.SystemError!Queue {
    var instances = graph.GraphUnmanaged(*ModuleInfo, void).init(null, null);
    defer instances.deinit(allocator);
    var instance_map = std.StringArrayHashMapUnmanaged(graph.NodeId){};
    defer instance_map.deinit(allocator);

    // Allocate a node for each loadable module.
    var module_it = self.modules.iterator();
    instance_check: while (module_it.next()) |entry| {
        const name = entry.key_ptr;
        const instance = entry.value_ptr;
        if (instance.status != .unloaded) continue;

        // Check that no other module with the same name is already loaded.
        if (sys.getInstance(name.*) != null) {
            sys.logWarn(
                "instance with the same name already exists...skipping, instance='{s}'",
                .{name.*},
                @src(),
            );
            instance.signalError();
            continue;
        }

        // Check that all imported symbols are already exposed, or will be exposed.
        for (instance.@"export".getSymbolImports()) |imp| {
            const imp_name = std.mem.span(imp.name);
            const imp_ns = std.mem.span(imp.namespace);
            const imp_ver = Version.initC(imp.version);
            // Skip the module if a dependency could not be loaded.
            if (self.getSymbol(imp_name, imp_ns, imp_ver)) |sym| {
                const owner = self.getModule(sym.owner).?;
                if (owner.status == .err) {
                    sys.logWarn(
                        "instance can not be loaded due to an error loading one of its dependencies...skipping," ++
                            " instance='{s}', dependency='{s}'",
                        .{ name.*, sym.owner },
                        @src(),
                    );
                    instance.signalError();
                    continue :instance_check;
                }
            } else if (sys.getSymbolCompatible(imp_name, imp_ns, imp_ver) == null) {
                sys.logWarn(
                    "instance is missing required symbol...skipping," ++
                        " instance='{s}', symbol='{s}', namespace='{s}, version='{long}'",
                    .{ name.*, imp_name, imp_ns, imp_ver },
                    @src(),
                );
                instance.signalError();
                continue :instance_check;
            }
        }

        // Check that no exported symbols are already exposed.
        for (instance.@"export".getSymbolExports()) |e| {
            const e_name = std.mem.span(e.name);
            const e_ns = std.mem.span(e.namespace);
            if (sys.getSymbol(e_name, e_ns) != null) {
                sys.logWarn(
                    "instance exports duplicate symbol...skipping," ++
                        " instance='{s}', symbol='{s}', namespace='{s}",
                    .{ name.*, e_name, e_ns },
                    @src(),
                );
                instance.signalError();
                continue :instance_check;
            }
        }
        for (instance.@"export".getDynamicSymbolExports()) |e| {
            const e_name = std.mem.span(e.name);
            const e_ns = std.mem.span(e.namespace);
            if (sys.getSymbol(e_name, e_ns) != null) {
                sys.logWarn(
                    "instance exports duplicate symbol...skipping," ++
                        " instance='{s}', symbol='{s}', namespace='{s}",
                    .{ name.*, e_name, e_ns },
                    @src(),
                );
                instance.signalError();
                continue :instance_check;
            }
        }

        // Create a new node and insert it into the hashmap.
        const node = try instances.addNode(allocator, instance);
        try instance_map.put(allocator, name.*, node);
    }

    // Connect all nodes in the graph.
    var instance_it = instance_map.iterator();
    connect_graph: while (instance_it.next()) |entry| {
        const name = entry.key_ptr.*;
        const node = entry.value_ptr.*;
        const instance = self.getModule(name).?;

        for (instance.@"export".getSymbolImports()) |imp| {
            const i_name = std.mem.span(imp.name);
            const i_namespace = std.mem.span(imp.namespace);
            const i_version = Version.initC(imp.version);
            if (self.getSymbol(i_name, i_namespace, i_version)) |sym| {
                const owner_entry = instance_map.get(sym.owner);
                const owner = self.getModule(sym.owner).?;
                if (owner_entry == null or owner.status == .err) {
                    sys.asContext().tracing.emitWarnSimple(
                        "instance can not be loaded due to an error loading one of its dependencies...skipping," ++
                            " instance='{s}', dependency='{s}'",
                        .{ name, sym.owner },
                        @src(),
                    );
                    instance.signalError();
                    continue :connect_graph;
                }

                const to_node = owner_entry.?;
                _ = instances.addEdge(
                    allocator,
                    {},
                    node,
                    to_node,
                ) catch |err| switch (err) {
                    Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
                    else => unreachable,
                };
            }
        }
    }

    if (try instances.isCyclic(allocator)) return error.CyclicDependency;
    const order = instances.sortTopological(allocator, .outgoing) catch |err| switch (err) {
        Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
        else => unreachable,
    };
    defer allocator.free(order);

    var queue = Queue{};
    errdefer queue.deinit(allocator);
    for (order) |node| {
        const instance = instances.nodePtr(node).?.*;
        try queue.append(allocator, instance);
    }

    return queue;
}
