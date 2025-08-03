const std = @import("std");
const Allocator = std.mem.Allocator;
const ArenaAlloactor = std.heap.ArenaAllocator;
const Mutex = std.Thread.Mutex;

const c = @import("c");

const AnyError = @import("../../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const context = @import("../../context.zig");
const pub_context = @import("../../ctx.zig");
const pub_modules = @import("../../modules.zig");
const Path = @import("../../path.zig").Path;
const OwnedPathUnmanaged = @import("../../path.zig").OwnedPathUnmanaged;
const pub_tasks = @import("../../tasks.zig");
const EnqueuedFuture = pub_tasks.EnqueuedFuture;
const FSMFuture = pub_tasks.FSMFuture;
const Fallible = pub_tasks.Fallible;
const Version = @import("../../Version.zig");
const graph = @import("../graph.zig");
const modules = @import("../modules.zig");
const RefCount = @import("../RefCount.zig");
const tasks = @import("../tasks.zig");
const tracing = @import("../tracing.zig");
const InstanceHandle = @import("InstanceHandle.zig");
const ModuleHandle = @import("ModuleHandle.zig");
const SymbolRef = @import("SymbolRef.zig");

const Self = @This();

mutex: Mutex = .{},
refcount: RefCount = .{},
arena: ArenaAlloactor,
active_commits: usize = 0,
should_recreate_map: bool = false,
active_load_graph: ?*LoadGraph = null,
string_cache: std.StringArrayHashMapUnmanaged(void) = .{},
module_infos: std.StringArrayHashMapUnmanaged(ModuleInfo) = .{},
symbols: std.ArrayHashMapUnmanaged(SymbolRef.Id, SymbolRef, SymbolRef.Id.HashContext, false) = .{},

pub const Callback = struct {
    data: ?*anyopaque,
    on_success: *const fn (info: *const pub_modules.Info, data: ?*anyopaque) callconv(.c) void,
    on_error: *const fn (module: *const pub_modules.Export, data: ?*anyopaque) callconv(.c) void,
};

const ModuleInfo = struct {
    status: Status,
    allocator: Allocator,
    handle: *ModuleHandle,
    callbacks: std.ArrayListUnmanaged(Callback) = .{},

    const Status = union(enum) {
        unloaded: struct {
            @"export": *const pub_modules.Export,
            owner: ?*const pub_modules.OpaqueInstance,
        },
        err: struct {
            @"export": *const pub_modules.Export,
            owner: ?*const pub_modules.OpaqueInstance,
        },
        loaded: struct {
            info: *const pub_modules.Info,
        },
    };

    fn init(
        allocator: Allocator,
        @"export": *const pub_modules.Export,
        handle: *ModuleHandle,
        owner: ?*const pub_modules.OpaqueInstance,
    ) !ModuleInfo {
        handle.ref();
        if (owner) |o| {
            const i_handle = InstanceHandle.fromInstancePtr(o);
            const inner = i_handle.lock();
            defer inner.unlock();
            try inner.refStrong();
        }

        return .{
            .status = .{ .unloaded = .{ .@"export" = @"export", .owner = owner } },
            .allocator = allocator,
            .handle = handle,
        };
    }

    fn deinit(self: *ModuleInfo) void {
        switch (self.status) {
            .unloaded => |v| {
                for (self.callbacks.items) |cb| cb.on_error(v.@"export", cb.data);
                self.callbacks.clearAndFree(self.allocator);
                v.@"export".deinit();
                if (v.owner) |owner| {
                    const handle = InstanceHandle.fromInstancePtr(owner);
                    const inner = handle.lock();
                    defer inner.unlock();
                    inner.unrefStrong();
                }
            },
            .err => |v| {
                for (self.callbacks.items) |cb| cb.on_error(v.@"export", cb.data);
                self.callbacks.clearAndFree(self.allocator);
                v.@"export".deinit();
                if (v.owner) |owner| {
                    const handle = InstanceHandle.fromInstancePtr(owner);
                    const inner = handle.lock();
                    defer inner.unlock();
                    inner.unrefStrong();
                }
            },
            .loaded => |v| {
                std.debug.assert(self.callbacks.items.len == 0);
                self.callbacks.clearAndFree(self.allocator);
                v.info.unref();
            },
        }
        self.handle.unref();
    }

    fn appendCallback(self: *ModuleInfo, callback: Callback) Allocator.Error!void {
        switch (self.status) {
            .unloaded => try self.callbacks.append(self.allocator, callback),
            .loaded => |v| callback.on_success(v.info, callback.data),
            .err => |v| callback.on_error(v.@"export", callback.data),
        }
    }

    pub fn signalError(self: *ModuleInfo) void {
        const status = self.status.unloaded;
        self.status = .{ .err = .{ .@"export" = status.@"export", .owner = status.owner } };
        while (self.callbacks.pop()) |cb| cb.on_error(status.@"export", cb.data);
    }

    pub fn signalSuccess(self: *ModuleInfo, info: *const pub_modules.Info) void {
        const status = self.status.unloaded;
        status.@"export".deinit();
        if (status.owner) |owner| {
            const handle = InstanceHandle.fromInstancePtr(owner);
            const inner = handle.lock();
            defer inner.unlock();
            inner.unrefStrong();
        }

        info.ref();
        self.status = .{ .loaded = .{ .info = info } };
        while (self.callbacks.pop()) |cb| cb.on_success(info, cb.data);
    }
};

const LoadGraph = struct {
    mutex: Mutex = .{},
    set: *Self,
    modules: std.StringArrayHashMapUnmanaged(graph.NodeId) = .{},
    dependency_tree: graph.GraphUnmanaged(Node, void) = graph.GraphUnmanaged(Node, void).init(
        null,
        null,
    ),
    enqueue_count: usize = 0,
    waiter: ?pub_tasks.Waker = null,

    const Node = struct {
        module: []const u8,
        waiter: ?pub_tasks.Waker = null,
        fut: ?pub_tasks.EnqueuedFuture(void) = null,

        fn deinit(self: *Node) void {
            std.debug.assert(self.waiter == null);
            if (self.fut) |*fut| fut.deinit();
        }

        fn signalWaiters(self: *Node, load_graph: *LoadGraph) void {
            const node_id = load_graph.modules.get(self.module).?;
            var it = load_graph.dependency_tree.neighborsIterator(
                node_id,
                .incoming,
            ) catch unreachable;
            while (it.next()) |e| {
                const id = e.node_id;
                const n = load_graph.dependency_tree.nodePtr(id).?;
                if (n.waiter) |waiter| {
                    n.waiter = null;
                    waiter.wakeUnref();
                }
            }
        }

        fn spawn(self: *Node, node_id: graph.NodeId, load_graph: *LoadGraph) !void {
            std.debug.assert(self.fut == null);
            self.fut = try LoadOp.Data.init(node_id, load_graph);
        }
    };

    fn init(set: *Self) !*LoadGraph {
        set.ref();
        errdefer set.unref();

        const allocator = modules.allocator;
        const g = try allocator.create(LoadGraph);
        g.* = .{ .set = set };
        return g;
    }

    fn deinit(self: *LoadGraph) void {
        const set = self.set;
        defer set.unref();
        const allocator = modules.allocator;

        var it = self.dependency_tree.nodesIterator();
        while (it.next()) |node| node.data_ptr.deinit();

        self.modules.deinit(allocator);
        self.dependency_tree.deinit(allocator);
        std.debug.assert(self.enqueue_count == 0);
        std.debug.assert(self.waiter == null);

        allocator.destroy(self);
    }

    fn spawnMissingTaks(self: *LoadGraph) !void {
        const set = self.set;
        if (!set.should_recreate_map) return;
        set.should_recreate_map = false;

        var module_it = set.module_infos.iterator();
        check: while (module_it.next()) |entry| {
            const name = entry.key_ptr.*;
            if (self.modules.contains(name)) continue;

            const info = entry.value_ptr;
            if (info.status != .unloaded) continue;
            errdefer |e| {
                tracing.emitWarnSimple(
                    "internal error loading instance, instance='{s}', error='{s}'",
                    .{ name, @errorName(e) },
                    @src(),
                );
                info.signalError();
            }

            // Check that no other module with the same name is already loaded.
            if (modules.getInstance(name) != null) {
                tracing.emitWarnSimple(
                    "instance with the same name already exists...skipping, instance='{s}'",
                    .{name},
                    @src(),
                );
                info.signalError();
                continue;
            }

            // Check that all imported symbols are already exposed, or will be exposed.
            for (info.status.unloaded.@"export".getSymbolImports()) |imp| {
                const imp_name = std.mem.span(imp.name);
                const imp_ns = std.mem.span(imp.namespace);
                const imp_ver = Version.initC(imp.version);
                // Skip the module if a dependency could not be loaded.
                if (set.getSymbol(imp_name, imp_ns, imp_ver)) |sym| {
                    const owner = set.getModuleInfo(sym.owner).?;
                    if (owner.status == .err) {
                        tracing.emitWarnSimple(
                            "instance can not be loaded due to an error loading one of its dependencies...skipping," ++
                                " instance='{s}', dependency='{s}'",
                            .{ name, sym.owner },
                            @src(),
                        );
                        info.signalError();
                        continue :check;
                    }
                } else if (modules.getSymbolCompatible(imp_name, imp_ns, imp_ver) == null) {
                    tracing.emitWarnSimple(
                        "instance is missing required symbol...skipping," ++
                            " instance='{s}', symbol='{s}', namespace='{s}', version='{f}'",
                        .{ name, imp_name, imp_ns, imp_ver },
                        @src(),
                    );
                    info.signalError();
                    continue :check;
                }
            }

            // Check that no exported symbols are already exposed.
            for (info.status.unloaded.@"export".getSymbolExports()) |e| {
                const e_name = std.mem.span(e.name);
                const e_ns = std.mem.span(e.namespace);
                if (modules.getSymbol(e_name, e_ns) != null) {
                    tracing.emitWarnSimple(
                        "instance exports duplicate symbol...skipping," ++
                            " instance='{s}', symbol='{s}', namespace='{s}'",
                        .{ name, e_name, e_ns },
                        @src(),
                    );
                    info.signalError();
                    continue :check;
                }
            }
            for (info.status.unloaded.@"export".getDynamicSymbolExports()) |e| {
                const e_name = std.mem.span(e.name);
                const e_ns = std.mem.span(e.namespace);
                if (modules.getSymbol(e_name, e_ns) != null) {
                    tracing.emitWarnSimple(
                        "instance exports duplicate symbol...skipping," ++
                            " instance='{s}', symbol='{s}', namespace='{s}'",
                        .{ name, e_name, e_ns },
                        @src(),
                    );
                    info.signalError();
                    continue :check;
                }
            }

            // Create the node such that we can connect them in the next step.
            const id = try self.dependency_tree.addNode(modules.allocator, .{ .module = name });
            try self.modules.put(modules.allocator, name, id);
        }

        // Connect the nodes and spawn the task.
        module_it.reset();
        connect: while (module_it.next()) |entry| {
            const name = entry.key_ptr.*;
            const id = self.modules.get(name).?;
            const node = self.dependency_tree.nodePtr(id).?;
            if (node.fut != null) continue;

            const info = entry.value_ptr;
            if (info.status != .unloaded) continue;
            errdefer |e| {
                tracing.emitWarnSimple(
                    "internal error loading instance, instance='{s}', error='{s}'",
                    .{ name, @errorName(e) },
                    @src(),
                );
                info.signalError();
            }

            for (info.status.unloaded.@"export".getSymbolImports()) |imp| {
                const i_name = std.mem.span(imp.name);
                const i_namespace = std.mem.span(imp.namespace);
                const i_version = Version.initC(imp.version);
                if (set.getSymbol(i_name, i_namespace, i_version)) |sym| {
                    const owner_id = self.modules.get(sym.owner);
                    const owner_info = set.getModuleInfo(sym.owner).?;
                    if (owner_id == null or owner_info.status == .err) {
                        tracing.emitWarnSimple(
                            "instance can not be loaded due to an error loading one of its dependencies...skipping," ++
                                " instance='{s}', dependency='{s}'",
                            .{ name, sym.owner },
                            @src(),
                        );
                        info.signalError();
                        continue :connect;
                    }
                    _ = try self.dependency_tree.addEdge(modules.allocator, {}, id, owner_id.?);
                }
            }

            try node.spawn(id, self);
            self.enqueue_count += 1;
        }
    }

    fn waitForCompletion(self: *LoadGraph, waker: pub_tasks.Waker) enum { done, wait } {
        if (self.enqueue_count == 0) return .done;
        self.notify();
        self.waiter = waker.ref();
        return .wait;
    }

    fn notify(self: *LoadGraph) void {
        if (self.waiter) |waiter| {
            waiter.wakeUnref();
            self.waiter = null;
        }
    }

    fn dequeueModule(self: *LoadGraph) void {
        self.enqueue_count -= 1;
        self.notify();
    }
};

pub fn init() !pub_modules.LoadingSet {
    tracing.emitTraceSimple("creating new loading set", .{}, @src());

    modules.mutex.lock();
    defer modules.mutex.unlock();

    var arena = ArenaAlloactor.init(modules.allocator);
    errdefer arena.deinit();
    const allocator = arena.allocator();

    const set = try allocator.create(Self);
    set.* = .{ .arena = undefined };
    set.arena = arena;
    modules.loading_set_count.increase();

    return set.asProxySet();
}

fn ref(self: *@This()) void {
    self.refcount.ref();
}

fn unref(self: *@This()) void {
    if (self.refcount.unref() == .noop) return;
    std.debug.assert(self.active_commits == 0);
    std.debug.assert(self.active_load_graph == null);

    for (self.module_infos.values()) |*module| module.deinit();
    self.module_infos.clearRetainingCapacity();
    self.symbols.clearRetainingCapacity();

    var arena = self.arena;
    arena.deinit();
    modules.loading_set_count.decrease();
}

pub fn lock(self: *Self) void {
    self.mutex.lock();
}

pub fn unlock(self: *Self) void {
    self.mutex.unlock();
}

pub fn asProxySet(self: *Self) pub_modules.LoadingSet {
    return .{
        .data = self,
        .vtable = &vtable,
    };
}

fn cacheString(self: *Self, value: []const u8) Allocator.Error![]const u8 {
    if (self.string_cache.getKey(value)) |v| return v;
    const alloc = self.arena.allocator();
    const cached = try alloc.dupe(u8, value);
    errdefer alloc.free(cached);
    try self.string_cache.put(alloc, cached, {});
    return cached;
}

fn addModuleInfo(self: *Self, module_info: ModuleInfo) Allocator.Error!void {
    const name = try self.cacheString(module_info.status.unloaded.@"export".getName());
    try self.module_infos.put(self.arena.allocator(), name, module_info);
}

fn getModuleInfo(self: *const Self, name: []const u8) ?*ModuleInfo {
    return self.module_infos.getPtr(name);
}

fn addSymbol(
    self: *Self,
    name: []const u8,
    namespace: []const u8,
    version: Version,
    owner: []const u8,
) Allocator.Error!void {
    const key = SymbolRef.Id{
        .name = try self.cacheString(name),
        .namespace = try self.cacheString(namespace),
    };
    const symbol = SymbolRef{
        .owner = try self.cacheString(owner),
        .version = version,
    };
    try self.symbols.put(self.arena.allocator(), key, symbol);
}

fn removeSymbol(self: *Self, name: []const u8, namespace: []const u8) void {
    if (!self.symbols.swapRemove(.{ .name = name, .namespace = namespace })) unreachable;
}

fn getSymbolAny(self: *const Self, name: []const u8, ns: []const u8) ?*SymbolRef {
    return self.symbols.getPtr(.{ .name = @constCast(name), .namespace = @constCast(ns) });
}

fn getSymbol(self: *const Self, name: []const u8, ns: []const u8, version: Version) ?*SymbolRef {
    const symbol = self.getSymbolAny(name, ns) orelse return null;
    if (!symbol.version.isCompatibleWith(version)) return null;
    return symbol;
}

fn addModuleInner(
    self: *Self,
    module_handle: *ModuleHandle,
    @"export": *const pub_modules.Export,
    owner: ?*const pub_modules.OpaqueInstance,
) !void {
    if (self.getModuleInfo(@"export".getName()) != null) return error.Duplicate;
    for (@"export".getSymbolExports()) |exp| {
        const name = std.mem.span(exp.name);
        const namespace = std.mem.span(exp.namespace);
        if (self.getSymbolAny(name, namespace)) |sym| {
            tracing.emitErrSimple(
                "duplicate symbol, owner='{s}', name='{s}', namespace='{s}', version='{f}'",
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
            tracing.emitErrSimple(
                "duplicate symbol, owner='{s}', name='{s}', namespace='{s}', version='{f}'",
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

    var module_info = try ModuleInfo.init(
        context.allocator,
        @"export",
        module_handle,
        owner,
    );
    errdefer module_info.deinit();
    try self.addModuleInfo(module_info);
    self.should_recreate_map = true;
    if (self.active_load_graph) |g| g.notify();
}

fn validate_export(@"export": *const pub_modules.Export) error{InvalidExport}!void {
    if (@"export".next != null) {
        tracing.emitWarnSimple("the next field is reserved for future use", .{}, @src());
        return error.InvalidExport;
    }
    if (!pub_context.context_version.isCompatibleWith(@"export".getVersion())) {
        tracing.emitWarnSimple(
            "incompatible context version, got='{f}', required='{f}'",
            .{ pub_context.context_version, @"export".getVersion() },
            @src(),
        );
        return error.InvalidExport;
    }

    var has_error = false;
    if (std.mem.startsWith(u8, @"export".getName(), "__")) {
        tracing.emitWarnSimple(
            "export uses reserved name, export='{s}'",
            .{@"export".name},
            @src(),
        );
        return error.InvalidExport;
    }

    const namespaces = @"export".getNamespaceImports();
    for (namespaces, 0..) |ns, i| {
        if (std.mem.eql(u8, std.mem.span(ns.name), "")) {
            tracing.emitWarnSimple(
                "can not import global namespace, export='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), ns.name, i },
                @src(),
            );
            has_error = true;
        }

        var count: usize = 0;
        for (namespaces[0..i]) |x| {
            if (std.mem.eql(u8, std.mem.span(ns.name), std.mem.span(x.name))) count += 1;
        }
        if (count > 1) {
            tracing.emitWarnSimple(
                "duplicate namespace, export='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), ns.name, i },
                @src(),
            );
            has_error = true;
        }
    }

    const imports = @"export".getSymbolImports();
    for (imports, 0..) |imp, i| {
        var ns_found = std.mem.eql(u8, std.mem.span(imp.namespace), "");
        for (namespaces) |ns| {
            if (ns_found) break;
            if (std.mem.eql(u8, std.mem.span(imp.namespace), std.mem.span(ns.name))) {
                ns_found = true;
            }
        }
        if (!ns_found) {
            tracing.emitWarnSimple(
                "required namespace not imported, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), imp.name, imp.namespace, i },
                @src(),
            );
            has_error = true;
        }
    }

    const exports = @"export".getSymbolExports();
    for (exports, 0..) |exp, i| {
        const name = std.mem.span(exp.name);
        const namespace = std.mem.span(exp.namespace);
        if (std.mem.startsWith(u8, name, "__")) {
            tracing.emitWarnSimple(
                "can not export a symbol with a reserved name, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), name, namespace, i },
                @src(),
            );
            has_error = true;
        }
        if (std.mem.startsWith(u8, name, "__")) {
            tracing.emitWarnSimple(
                "can not export a symbol in a reserved namespace, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), name, namespace, i },
                @src(),
            );
            has_error = true;
        }
        if (exp.linkage != .global) {
            tracing.emitWarnSimple(
                "unknown symbol linkage specified, export='{s}', symbol='{s}', ns='{s}', linkage='{}', index='{}'",
                .{ @"export".getName(), name, namespace, @intFromEnum(exp.linkage), i },
                @src(),
            );
            has_error = true;
        }

        for (imports) |imp| {
            const imp_name = std.mem.span(imp.name);
            const imp_namespace = std.mem.span(imp.namespace);
            if (std.mem.eql(u8, name, imp_name) and
                std.mem.eql(u8, namespace, imp_namespace))
            {
                tracing.emitWarnSimple(
                    "can not import and export the same symbol, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                    .{ @"export".getName(), name, namespace, i },
                    @src(),
                );
                has_error = true;
                break;
            }
        }

        var count: usize = 0;
        for (exports[0..i]) |x| {
            const exp_name = std.mem.span(x.name);
            const exp_namespace = std.mem.span(x.namespace);
            if (std.mem.eql(u8, name, exp_name) and
                std.mem.eql(u8, namespace, exp_namespace)) count += 1;
        }
        if (count > 1) {
            tracing.emitWarnSimple(
                "duplicate export, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), name, namespace, i },
                @src(),
            );
            has_error = true;
        }
    }

    const dynamic_exports = @"export".getDynamicSymbolExports();
    for (dynamic_exports, 0..) |exp, i| {
        const name = std.mem.span(exp.name);
        const namespace = std.mem.span(exp.namespace);
        if (std.mem.startsWith(u8, name, "__")) {
            tracing.emitWarnSimple(
                "can not export a symbol with a reserved name, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), name, namespace, i },
                @src(),
            );
            has_error = true;
        }
        if (std.mem.startsWith(u8, name, "__")) {
            tracing.emitWarnSimple(
                "can not export a symbol in a reserved namespace, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), name, namespace, i },
                @src(),
            );
            has_error = true;
        }
        if (exp.linkage != .global) {
            tracing.emitWarnSimple(
                "unknown symbol linkage specified, export='{s}', symbol='{s}', ns='{s}', linkage='{}', index='{}'",
                .{ @"export".getName(), name, namespace, @intFromEnum(exp.linkage), i },
                @src(),
            );
            has_error = true;
        }

        for (imports) |imp| {
            const imp_name = std.mem.span(imp.name);
            const imp_namespace = std.mem.span(imp.namespace);
            if (std.mem.eql(u8, name, imp_name) and
                std.mem.eql(u8, namespace, imp_namespace))
            {
                tracing.emitWarnSimple(
                    "can not import and export the same symbol, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                    .{ @"export".getName(), name, namespace, i },
                    @src(),
                );
                has_error = true;
                break;
            }
        }

        var count: usize = 0;
        for (exports) |x| {
            const exp_name = std.mem.span(x.name);
            const exp_namespace = std.mem.span(x.namespace);
            if (std.mem.eql(u8, name, exp_name) and
                std.mem.eql(u8, namespace, exp_namespace)) count += 1;
        }
        for (dynamic_exports[0..i]) |x| {
            const exp_name = std.mem.span(x.name);
            const exp_namespace = std.mem.span(x.namespace);
            if (std.mem.eql(u8, name, exp_name) and
                std.mem.eql(u8, namespace, exp_namespace)) count += 1;
        }
        if (count > 1) {
            tracing.emitWarnSimple(
                "duplicate export, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), name, namespace, i },
                @src(),
            );
            has_error = true;
        }
    }

    const modifiers = @"export".getModifiers();
    for (modifiers, 0..) |mod, i| {
        switch (mod.tag) {
            .destructor, .dependency => {},
            .debug_info, .instance_state, .start_event, .stop_event => {
                for (modifiers[0..i]) |x| {
                    if (x.tag == mod.tag) {
                        tracing.emitWarnSimple(
                            "the modifier may only appear once, export='{s}', modifier=`{s}`, index='{}'",
                            .{ @"export".getName(), @tagName(mod.tag), i },
                            @src(),
                        );
                        has_error = true;
                    }
                }
            },
            else => {
                tracing.emitWarnSimple(
                    "unknown modifier, export='{s}', modifier='{}', index='{}'",
                    .{ @"export".getName(), @intFromEnum(mod.tag), i },
                    @src(),
                );
                has_error = true;
            },
        }
    }

    if (has_error) return error.InvalidExport;
}

const AppendModulesData = struct {
    err: ?Allocator.Error = null,
    filter_data: ?*anyopaque,
    filter_fn: *const fn (
        @"export": *const pub_modules.Export,
        data: ?*anyopaque,
    ) callconv(.c) pub_modules.LoadingSet.FilterRequest,
    exports: std.ArrayListUnmanaged(*const pub_modules.Export) = .{},
};

fn appendModules(@"export": *const pub_modules.Export, o_data: ?*anyopaque) callconv(.c) bool {
    const data: *AppendModulesData = @alignCast(@ptrCast(o_data));
    validate_export(@"export") catch {
        tracing.emitWarnSimple("skipping export", .{}, @src());
        return true;
    };

    if (data.filter_fn(@"export", data.filter_data) == .load) {
        data.exports.append(modules.allocator, @"export") catch |err| {
            @"export".deinit();
            data.err = err;
            return false;
        };
    }

    return true;
}

fn addModule(
    self: *Self,
    owner_inner: *InstanceHandle.Inner,
    @"export": *const pub_modules.Export,
) !void {
    try validate_export(@"export");
    try self.addModuleInner(owner_inner.handle.?, @"export", owner_inner.instance.?);
}

fn addModulesFromHandle(
    self: *Self,
    module_handle: *ModuleHandle,
    filter_fn: *const fn (
        @"export": *const pub_modules.Export,
        data: ?*anyopaque,
    ) callconv(.c) pub_modules.LoadingSet.FilterRequest,
    filter_data: ?*anyopaque,
) !void {
    var append_data = AppendModulesData{
        .filter_fn = filter_fn,
        .filter_data = filter_data,
    };
    defer append_data.exports.deinit(modules.allocator);
    errdefer for (append_data.exports.items) |exp| exp.deinit();
    module_handle.iterator(&appendModules, &append_data);
    if (append_data.err) |err| return err;

    for (append_data.exports.items) |exp| {
        try self.addModuleInner(module_handle, exp, null);
    }
}

fn addModulesFromPath(
    self: *Self,
    path: Path,
    filter_fn: *const fn (
        @"export": *const pub_modules.Export,
        data: ?*anyopaque,
    ) callconv(.c) pub_modules.LoadingSet.FilterRequest,
    filter_data: ?*anyopaque,
) !void {
    const module_handle = try ModuleHandle.initPath(modules.allocator, path);
    defer module_handle.unref();
    try self.addModulesFromHandle(module_handle, filter_fn, filter_data);
}

fn addModulesFromLocal(
    self: *Self,
    iterator_fn: ModuleHandle.IteratorFn,
    filter_fn: *const fn (
        @"export": *const pub_modules.Export,
        data: ?*anyopaque,
    ) callconv(.c) pub_modules.LoadingSet.FilterRequest,
    filter_data: ?*anyopaque,
    bin_ptr: *const anyopaque,
) !void {
    const module_handle = try ModuleHandle.initLocal(
        modules.allocator,
        iterator_fn,
        bin_ptr,
    );
    defer module_handle.unref();
    try self.addModulesFromHandle(module_handle, filter_fn, filter_data);
}

// ----------------------------------------------------
// Futures
// ----------------------------------------------------

const LoadOp = FSMFuture(struct {
    node_id: graph.NodeId,
    load_graph: *LoadGraph,
    name: [:0]const u8 = undefined,
    instance_future: InstanceHandle.InitExportedOp = undefined,
    instance: *pub_modules.OpaqueInstance = undefined,
    start_instance_future: InstanceHandle.StartInstanceOp = undefined,
    ret: void = {},

    pub const __no_abort = true;

    fn init(node_id: graph.NodeId, load_graph: *LoadGraph) !EnqueuedFuture(void) {
        const data = @This(){
            .node_id = node_id,
            .load_graph = load_graph,
        };
        var f = LoadOp.init(data).intoFuture();
        return tasks.Task.initFuture(@TypeOf(f), &f);
    }

    pub fn __set_err(self: *@This(), trace: ?*std.builtin.StackTrace, err: anyerror) void {
        if (trace) |tr| tracing.emitStackTraceSimple(tr.*, @src());
        self.ret = err;
    }

    pub fn __ret(self: *@This()) void {
        return self.ret;
    }

    pub fn __unwind0(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = reason;

        self.load_graph.set.lock();
        defer self.load_graph.set.unlock();

        self.load_graph.mutex.lock();
        defer self.load_graph.mutex.unlock();

        const node = self.load_graph.dependency_tree.nodePtr(self.node_id).?;
        node.signalWaiters(self.load_graph);
        self.load_graph.dequeueModule();
    }

    pub fn __state0(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        const set = self.load_graph.set;
        set.lock();
        defer set.unlock();

        self.load_graph.mutex.lock();
        defer self.load_graph.mutex.unlock();

        const node = self.load_graph.dependency_tree.nodePtr(self.node_id).?;
        const info = set.getModuleInfo(node.module).?;

        // Check that it is not part of a cycle.
        const is_cyclic = self.load_graph.dependency_tree.pathExists(
            modules.allocator,
            self.node_id,
            self.node_id,
        ) catch |err| {
            tracing.emitWarnSimple(
                "internal error while verifying module dependencies...skipping," ++
                    " instance='{s}', error='{s}'",
                .{ node.module, @errorName(err) },
                @src(),
            );
            info.signalError();
            return .ret;
        };
        if (is_cyclic) {
            tracing.emitWarnSimple(
                "module has a cyclic dependency...skipping, instance='{s}'",
                .{node.module},
                @src(),
            );
            info.signalError();
            return .ret;
        }

        // Check all dependencies.
        var it = self.load_graph.dependency_tree.neighborsIterator(
            self.node_id,
            .outgoing,
        ) catch unreachable;
        while (it.next()) |dep| {
            const dep_id = dep.node_id;
            const dep_node = self.load_graph.dependency_tree.nodePtr(dep_id) orelse continue;
            const dep_name = dep_node.module;
            const dep_info = set.getModuleInfo(dep_name).?;
            var status = dep_info.status;
            if (dep_node.fut == null) {
                const unloaded = status.unloaded;
                status = .{ .err = .{ .@"export" = unloaded.@"export", .owner = unloaded.owner } };
            }

            switch (status) {
                .err => {
                    tracing.emitWarnSimple(
                        "instance can not be loaded due to an error loading one of its dependencies...skipping," ++
                            " instance='{s}', dependency='{s}'",
                        .{ node.module, dep_name },
                        @src(),
                    );
                    info.signalError();
                    return .ret;
                },
                .unloaded => {
                    // todo: fix, the waiter is for the commit op, not for the individual modules
                    self.load_graph.notify();
                    self.load_graph.waiter = waker.ref();
                    return .yield;
                },
                .loaded => {},
            }
        }

        return .next;
    }

    pub fn __state1(self: *@This(), waker: pub_tasks.Waker) void {
        _ = waker;

        modules.mutex.lock();
        errdefer modules.mutex.unlock();

        const set = self.load_graph.set;
        set.lock();
        errdefer set.unlock();

        const node = blk: {
            self.load_graph.mutex.lock();
            defer self.load_graph.mutex.unlock();
            break :blk self.load_graph.dependency_tree.nodePtr(self.node_id).?;
        };
        const info = set.getModuleInfo(node.module).?;
        self.name = std.mem.span(info.status.unloaded.@"export".name);
        tracing.emitTraceSimple("loading instance, instance='{s}'", .{self.name}, @src());

        // Recheck that all dependencies could be loaded.
        for (info.status.unloaded.@"export".getSymbolImports()) |i| {
            const i_name = std.mem.span(i.name);
            const i_namespace = std.mem.span(i.namespace);
            const i_version = Version.initC(i.version);
            if (set.getSymbol(i_name, i_namespace, i_version)) |sym| {
                const owner = set.getModuleInfo(sym.owner).?;
                std.debug.assert(owner.status != .unloaded);
            }
        }

        // Initialize the instance future.
        set.unlock();
        self.instance_future = InstanceHandle.InitExportedOp.init(.{
            .set = set.asProxySet(),
            .@"export" = info.status.unloaded.@"export",
            .handle = info.handle,
        });
    }

    pub fn __unwind2(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = reason;
        const set = self.load_graph.set;

        self.instance_future.deinit();
        set.unlock();
        modules.mutex.unlock();
    }

    pub fn __state2(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        const set = self.load_graph.set;
        switch (self.instance_future.poll(waker)) {
            .ready => |result| {
                self.instance = result catch |err| {
                    if (context.hasErrorResult()) {
                        const e = context.takeResult().unwrapErr();
                        defer e.deinit();
                        tracing.emitWarnSimple(
                            "instance construction error...skipping," ++
                                " instance='{s}', error='{f}:{f}'",
                            .{ self.name, std.fmt.alt(e, .formatName), e },
                            @src(),
                        );
                    } else tracing.emitWarnSimple(
                        "instance construction error...skipping," ++
                            " instance='{s}', error='{s}'",
                        .{ self.name, @errorName(err) },
                        @src(),
                    );
                    set.lock();
                    set.getModuleInfo(self.name).?.signalError();
                    return .ret;
                };
                set.lock();
                return .next;
            },
            .pending => return .yield,
        }
    }

    pub fn __state3(self: *@This(), waker: pub_tasks.Waker) void {
        _ = waker;
        const set = self.load_graph.set;
        const instance = self.instance;

        const instance_handle = InstanceHandle.fromInstancePtr(instance);
        const inner = instance_handle.lock();

        set.unlock();
        self.start_instance_future = inner.start();
    }

    pub fn __unwind4(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = reason;
        self.start_instance_future.deinit();
    }

    pub fn __state4(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        const set = self.load_graph.set;
        const instance = self.instance;
        const instance_handle = InstanceHandle.fromInstancePtr(instance);
        const inner = @constCast(&instance_handle.inner);

        switch (self.start_instance_future.poll(waker)) {
            .ready => |result| {
                result catch |e_| {
                    if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
                    if (context.hasErrorResult()) {
                        const e = context.takeResult().unwrapErr();
                        defer e.deinit();
                        tracing.emitWarnSimple(
                            "instance `on_start` error...skipping," ++
                                " instance='{s}', error='{f}:{f}'",
                            .{ self.name, std.fmt.alt(e, .formatName), e },
                            @src(),
                        );
                    } else tracing.emitWarnSimple(
                        "instance `on_start` error...skipping," ++
                            " instance='{s}', error='{s}'",
                        .{ self.name, @errorName(e_) },
                        @src(),
                    );
                    inner.unrefStrong();
                    inner.deinit();
                    set.lock();
                    set.getModuleInfo(self.name).?.signalError();
                    return .ret;
                };
                set.lock();
            },
            .pending => return .yield,
        }

        modules.addInstance(inner) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            tracing.emitWarnSimple(
                "internal error while adding instance...skipping," ++
                    " instance='{s}', error='{s}'",
                .{ self.name, @errorName(e) },
                @src(),
            );
            inner.stop();
            inner.unrefStrong();
            inner.deinit();
            return .ret;
        };
        defer inner.unlock();
        defer inner.unrefStrong();

        set.getModuleInfo(self.name).?.signalSuccess(self.instance.info);
        tracing.emitTraceSimple("instance loaded, instance='{s}'", .{self.name}, @src());
        return .ret;
    }
});

const CommitOp = FSMFuture(struct {
    set: *Self,
    load_graph: *LoadGraph = undefined,
    ret: anyerror!void = undefined,

    pub const __no_abort = true;

    fn init(set: *Self) EnqueuedFuture(Fallible(void)) {
        tracing.emitTraceSimple("commiting loading set, set='{*}'", .{set}, @src());
        set.ref();

        const data = @This(){
            .set = set,
        };
        var f = CommitOp.init(data).intoFuture().map(
            Fallible(void),
            Fallible(void).Wrapper(anyerror),
        );

        return tasks.Task.initFuture(@TypeOf(f), &f) catch |e| {
            set.unref();
            return tasks.initErrorFuture(void, e);
        };
    }

    pub fn __set_err(self: *@This(), trace: ?*std.builtin.StackTrace, err: anyerror) void {
        if (trace) |tr| tracing.emitStackTraceSimple(tr.*, @src());
        self.ret = err;
    }

    pub fn __ret(self: *@This()) !void {
        return self.ret;
    }

    pub fn __unwind0(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = reason;
        self.set.unref();
    }

    pub fn __state0(self: *@This(), waker: pub_tasks.Waker) !pub_tasks.FSMOp {
        modules.mutex.lock();
        defer modules.mutex.unlock();

        self.set.lock();
        defer self.set.unlock();

        // Ensure that no two commit operations are running in parallel.
        if (modules.state == .loading_set) {
            try modules.loading_set_waiters.append(
                modules.allocator,
                .{ .waiter = self, .waker = waker.ref() },
            );
            return .yield;
        }
        modules.state = .loading_set;
        errdefer modules.state = .idle;

        self.load_graph = try LoadGraph.init(self.set);
        std.debug.assert(self.set.active_load_graph == null);
        self.set.active_load_graph = self.load_graph;
        self.set.active_commits += 1;
        return .next;
    }

    pub fn __unwind1(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = reason;
        modules.mutex.lock();
        defer modules.mutex.unlock();

        self.set.lock();
        defer self.set.unlock();

        self.set.active_load_graph = null;
        self.set.active_commits -= 1;

        self.load_graph.deinit();
        modules.state = .idle;
        if (modules.loading_set_waiters.pop()) |waiter| {
            waiter.waker.wakeUnref();
        }
    }

    pub fn __state1(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        waker.wake();

        self.set.lock();
        defer self.set.unlock();

        self.load_graph.mutex.lock();
        defer self.load_graph.mutex.unlock();

        // Spawn all tasks and wait.
        self.load_graph.spawnMissingTaks() catch |err| {
            self.ret = err;
            return .next;
        };
        switch (self.load_graph.waitForCompletion(waker)) {
            .done => return .ret,
            .wait => return .yield,
        }
    }

    pub fn __state2(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        self.set.lock();
        defer self.set.unlock();

        self.load_graph.mutex.lock();
        defer self.load_graph.mutex.unlock();

        const err: anyerror = if (self.ret) |_| unreachable else |e| e;

        // Abort all not-spawned tasks and wait.
        var it = self.set.module_infos.iterator();
        while (it.next()) |entry| {
            const name = entry.key_ptr.*;
            if (!self.load_graph.modules.contains(name)) {
                tracing.emitWarnSimple(
                    "aborting load of instance due to internal error, instance='{s}', error='{s}'",
                    .{ name, @errorName(err) },
                    @src(),
                );
                entry.value_ptr.signalError();
            }
        }
        switch (self.load_graph.waitForCompletion(waker)) {
            .done => return .ret,
            .wait => return .yield,
        }
    }
});

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn ref(this: *anyopaque) callconv(.c) void {
        const self: *Self = @alignCast(@ptrCast(this));
        self.ref();
    }
    fn unref(this: *anyopaque) callconv(.c) void {
        const self: *Self = @alignCast(@ptrCast(this));
        self.unref();
    }
    fn queryModule(
        this: *anyopaque,
        module: [*:0]const u8,
    ) callconv(.c) bool {
        const self: *Self = @alignCast(@ptrCast(this));
        const module_ = std.mem.span(module);

        tracing.emitTraceSimple(
            "querying loading set module, set='{*}', name='{s}'",
            .{ self, module_ },
            @src(),
        );

        self.lock();
        defer self.unlock();
        return self.getModuleInfo(module_) != null;
    }
    fn querySymbol(
        this: *anyopaque,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: Version.CVersion,
    ) callconv(.c) bool {
        const self: *Self = @alignCast(@ptrCast(this));
        const name_ = std.mem.span(name);
        const namespace_ = std.mem.span(namespace);
        const version_ = Version.initC(version);

        tracing.emitTraceSimple(
            "querying loading set symbol, set='{*}', name='{s}', namespace='{s}', version='{f}'",
            .{ self, name_, namespace_, version_ },
            @src(),
        );

        self.lock();
        defer self.unlock();
        return self.getSymbol(name_, namespace_, version_) != null;
    }
    fn addCallback(
        this: *anyopaque,
        module: [*:0]const u8,
        on_success: *const fn (info: *const pub_modules.Info, data: ?*anyopaque) callconv(.c) void,
        on_error: *const fn (module: *const pub_modules.Export, data: ?*anyopaque) callconv(.c) void,
        on_abort: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        data: ?*anyopaque,
    ) callconv(.c) pub_context.Status {
        const self: *Self = @alignCast(@ptrCast(this));
        const module_ = std.mem.span(module);
        const callback = Callback{
            .data = data,
            .on_success = on_success,
            .on_error = on_error,
        };

        tracing.emitTraceSimple(
            "adding callback to the loading set, set='{*}', module='{s}', callback='{}'",
            .{ self, module_, callback },
            @src(),
        );

        self.lock();
        defer self.unlock();

        _ = blk: {
            const module_info = self.getModuleInfo(module_) orelse break :blk error.NotFound;
            module_info.appendCallback(callback) catch |err| break :blk err;
        } catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            if (on_abort) |f| f(data);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn addModule(
        this: *anyopaque,
        owner: *const pub_modules.OpaqueInstance,
        @"export": *const pub_modules.Export,
    ) callconv(.c) pub_context.Status {
        const self: *Self = @alignCast(@ptrCast(this));

        tracing.emitTraceSimple(
            "adding module to the loading set, set='{*}', module='{s}'",
            .{ self, @"export".getName() },
            @src(),
        );

        self.lock();
        defer self.unlock();

        _ = blk: {
            const owner_handle = InstanceHandle.fromInstancePtr(owner);
            const owner_inner = owner_handle.lock();
            defer owner_inner.unlock();
            self.addModule(owner_inner, @"export") catch |err| break :blk err;
        } catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn addModulesFromPath(
        this: *anyopaque,
        path: c.FimoUTF8Path,
        filter_fn: *const fn (
            module: *const pub_modules.Export,
            data: ?*anyopaque,
        ) callconv(.c) pub_modules.LoadingSet.FilterRequest,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        filter_data: ?*anyopaque,
    ) callconv(.c) pub_context.Status {
        const self: *Self = @alignCast(@ptrCast(this));
        const path_ = Path.initC(path);

        tracing.emitTraceSimple(
            "adding modules to loading set, set='{*}', path='{f}'",
            .{ self, path_ },
            @src(),
        );

        self.lock();
        defer self.unlock();
        defer if (filter_deinit) |f| f(filter_data);

        self.addModulesFromPath(path_, filter_fn, filter_data) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn addModulesFromLocal(
        this: *anyopaque,
        filter_fn: *const fn (
            module: *const pub_modules.Export,
            data: ?*anyopaque,
        ) callconv(.c) pub_modules.LoadingSet.FilterRequest,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        filter_data: ?*anyopaque,
        iterator_fn: *const fn (
            f: *const fn (module: *const pub_modules.Export, data: ?*anyopaque) callconv(.c) bool,
            data: ?*anyopaque,
        ) callconv(.c) void,
        bin_ptr: *const anyopaque,
    ) callconv(.c) pub_context.Status {
        const self: *Self = @alignCast(@ptrCast(this));

        tracing.emitTraceSimple(
            "adding local modules to loading set, set='{*}'",
            .{self},
            @src(),
        );

        self.lock();
        defer self.unlock();
        defer if (filter_deinit) |f| f(filter_data);

        self.addModulesFromLocal(
            iterator_fn,
            filter_fn,
            filter_data,
            bin_ptr,
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.emitStackTraceSimple(tr.*, @src());
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn commit(this: *anyopaque) callconv(.c) EnqueuedFuture(Fallible(void)) {
        const self: *Self = @alignCast(@ptrCast(this));
        return CommitOp.Data.init(self);
    }
};

const vtable = pub_modules.LoadingSet.VTable{
    .ref = &VTableImpl.ref,
    .unref = &VTableImpl.unref,
    .query_module = &VTableImpl.queryModule,
    .query_symbol = &VTableImpl.querySymbol,
    .add_callback = &VTableImpl.addCallback,
    .add_module = &VTableImpl.addModule,
    .add_modules_from_path = &VTableImpl.addModulesFromPath,
    .add_modules_from_local = &VTableImpl.addModulesFromLocal,
    .commit = &VTableImpl.commit,
};
