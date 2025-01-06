const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;

const AnyError = @import("../../AnyError.zig");
const c = @import("../../c.zig");
const heap = @import("../../heap.zig");
const Path = @import("../../path.zig").Path;
const OwnedPathUnmanaged = @import("../../path.zig").OwnedPathUnmanaged;
const Version = @import("../../Version.zig");

const graph = @import("../graph.zig");
const RefCount = @import("../RefCount.zig");

const InstanceHandle = @import("InstanceHandle.zig");
const ModuleHandle = @import("ModuleHandle.zig");
const SymbolRef = @import("SymbolRef.zig");
const System = @import("System.zig");

const Context = @import("../../context.zig");
const Async = @import("../async.zig");
const ProxyAsync = @import("../proxy_context/async.zig");
const ProxyModule = @import("../proxy_context/module.zig");

const EnqueuedFuture = ProxyAsync.EnqueuedFuture;
const FSMFuture = ProxyAsync.FSMFuture;
const Fallible = ProxyAsync.Fallible;

const Self = @This();

mutex: Mutex = .{},
refcount: RefCount = .{},
context: *Context,
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
    allocator: Allocator,
    handle: *ModuleHandle,
    info: ?*const ProxyModule.Info,
    @"export": *const ProxyModule.Export,
    owner: ?*const ProxyModule.OpaqueInstance,
    callbacks: std.ArrayListUnmanaged(Callback) = .{},

    const Status = enum { unloaded, loaded, err };

    fn init(
        allocator: Allocator,
        @"export": *const ProxyModule.Export,
        handle: *ModuleHandle,
        owner: ?*const ProxyModule.OpaqueInstance,
    ) !ModuleInfo {
        handle.ref();
        if (owner) |o| {
            const i_handle = InstanceHandle.fromInstancePtr(o);
            const inner = i_handle.lock();
            defer inner.unlock();
            try inner.refStrong();
        }

        return .{
            .status = .unloaded,
            .allocator = allocator,
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
        self.callbacks.clearAndFree(self.allocator);
        if (self.status != .loaded) self.@"export".deinit();
        if (self.owner) |owner| {
            const handle = InstanceHandle.fromInstancePtr(owner);
            const inner = handle.lock();
            defer inner.unlock();
            inner.unrefStrong();
        }
        self.handle.unref();
    }

    fn appendCallback(self: *ModuleInfo, callback: Callback) Allocator.Error!void {
        switch (self.status) {
            .unloaded => try self.callbacks.append(self.allocator, callback),
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

pub fn init(context: *Context) !ProxyModule.LoadingSet {
    context.module.sys.logTrace("creating new loading set", .{}, @src());

    const sys = &context.module.sys;
    sys.lock();
    defer sys.unlock();

    context.ref();
    errdefer context.unref();

    const allocator = sys.allocator;
    const module_load_path = try OwnedPathUnmanaged.initPath(
        allocator,
        sys.tmp_dir.path.asPath(),
    );
    errdefer module_load_path.deinit(allocator);

    const set = try allocator.create(Self);
    set.* = .{
        .context = context,
        .module_load_path = module_load_path,
    };
    return set.asProxySet();
}

fn ref(self: *@This()) void {
    self.refcount.ref();
}

fn unref(self: *@This()) void {
    if (self.refcount.unref() == .noop) return;
    while (self.modules.popOrNull()) |*entry| {
        var value = entry.value;
        self.context.allocator.free(entry.key);
        value.deinit();
    }
    self.modules.clearAndFree(self.context.allocator);
    while (self.symbols.popOrNull()) |*entry| {
        entry.key.deinit(self.context.allocator);
        entry.value.deinit(self.context.allocator);
    }
    self.symbols.clearAndFree(self.context.allocator);
    self.module_load_path.deinit(self.context.allocator);
    const context = self.context;
    self.context.allocator.destroy(self);
    context.unref();
}

pub fn lock(self: *Self) void {
    self.mutex.lock();
}

pub fn unlock(self: *Self) void {
    self.mutex.unlock();
}

pub fn asProxySet(self: *Self) ProxyModule.LoadingSet {
    return .{
        .data = self,
        .vtable = &vtable,
    };
}

fn asSys(self: *Self) *System {
    return &self.context.module.sys;
}

fn logTrace(self: *Self, comptime fmt: []const u8, args: anytype, location: std.builtin.SourceLocation) void {
    self.asSys().logTrace(fmt, args, location);
}

fn addModuleInfo(self: *Self, module_info: ModuleInfo) Allocator.Error!void {
    const name = try self.context.allocator.dupe(u8, module_info.@"export".getName());
    errdefer self.context.allocator.free(name);
    try self.modules.put(self.context.allocator, name, module_info);
}

fn getModuleInfo(self: *const Self, name: []const u8) ?*ModuleInfo {
    return self.modules.getPtr(name);
}

fn addSymbol(
    self: *Self,
    name: []const u8,
    namespace: []const u8,
    version: Version,
    owner: []const u8,
) Allocator.Error!void {
    const key = try SymbolRef.Id.init(self.context.allocator, name, namespace);
    errdefer key.deinit(self.context.allocator);
    const symbol = try SymbolRef.init(self.context.allocator, owner, version);
    errdefer symbol.deinit(self.context.allocator);
    try self.symbols.put(self.context.allocator, key, symbol);
    errdefer _ = self.symbols.swapRemove(key);
}

fn removeSymbol(self: *Self, name: []const u8, namespace: []const u8) void {
    var entry = self.symbols.fetchSwapRemove(.{
        .name = @constCast(name),
        .namespace = @constCast(namespace),
    }).?;
    entry.key.deinit(self.context.allocator);
    entry.value.deinit(self.context.allocator);
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
    @"export": *const ProxyModule.Export,
    owner: ?*const ProxyModule.OpaqueInstance,
) !void {
    if (self.getModuleInfo(@"export".getName()) != null) return error.Duplicate;
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

    var module_info = try ModuleInfo.init(
        self.context.allocator,
        @"export",
        module_handle,
        owner,
    );
    errdefer module_info.deinit();
    try self.addModuleInfo(module_info);
    self.should_recreate_map = true;
}

fn validate_export(sys: *System, @"export": *const ProxyModule.Export) error{InvalidExport}!void {
    if (@"export".id != .module_export) {
        sys.logWarn("invalid struct id, id='{}'", .{@"export".id}, @src());
        return error.InvalidExport;
    }
    if (@"export".next != null) {
        sys.logWarn("the next field is reserved for future use", .{}, @src());
        return error.InvalidExport;
    }
    if (!Context.ProxyContext.context_version.isCompatibleWith(@"export".getVersion())) {
        sys.logWarn(
            "incompatible context version, got='{long}', required='{long}'",
            .{ Context.ProxyContext.context_version, @"export".getVersion() },
            @src(),
        );
        return error.InvalidExport;
    }

    var has_error = false;
    if (std.mem.startsWith(u8, @"export".getName(), "__")) {
        sys.logWarn(
            "export uses reserved name, export='{s}'",
            .{@"export".name},
            @src(),
        );
        return error.InvalidExport;
    }

    const namespaces = @"export".getNamespaceImports();
    for (namespaces, 0..) |ns, i| {
        if (std.mem.eql(u8, std.mem.span(ns.name), "")) {
            sys.logWarn(
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
            sys.logWarn(
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
            sys.logWarn(
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
        for (imports) |imp| {
            const imp_name = std.mem.span(imp.name);
            const imp_namespace = std.mem.span(imp.namespace);
            if (std.mem.eql(u8, name, imp_name) and
                std.mem.eql(u8, namespace, imp_namespace))
            {
                sys.logWarn(
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
            sys.logWarn(
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
        for (imports) |imp| {
            const imp_name = std.mem.span(imp.name);
            const imp_namespace = std.mem.span(imp.namespace);
            if (std.mem.eql(u8, name, imp_name) and
                std.mem.eql(u8, namespace, imp_namespace))
            {
                sys.logWarn(
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
            sys.logWarn(
                "duplicate export, export='{s}', symbol='{s}', ns='{s}', index='{}'",
                .{ @"export".getName(), name, namespace, i },
                @src(),
            );
            has_error = true;
        }
    }

    const modifiers = @"export".getModifiers();
    for (modifiers, 0..) |mod, i| {
        if (@intFromEnum(mod.tag) >= c.FIMO_MODULE_EXPORT_MODIFIER_KEY_LAST) {
            sys.logWarn(
                "unknown modifier, export='{s}', modifier='{}', index='{}'",
                .{ @"export".getName(), @intFromEnum(mod.tag), i },
                @src(),
            );
            has_error = true;
        }

        switch (mod.tag) {
            .debug_info => {
                for (modifiers[0..i]) |x| {
                    if (x.tag == .debug_info) {
                        sys.logWarn(
                            "debug info modifier may only appear once, export='{s}', index='{}'",
                            .{ @"export".getName(), i },
                            @src(),
                        );
                        has_error = true;
                    }
                }
            },
            else => {},
        }
    }

    if (has_error) return error.InvalidExport;
}

const AppendModulesData = struct {
    sys: *System,
    err: ?Allocator.Error = null,
    filter_data: ?*anyopaque,
    filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
    exports: std.ArrayListUnmanaged(*const ProxyModule.Export) = .{},
};

fn appendModules(@"export": *const ProxyModule.Export, o_data: ?*anyopaque) callconv(.C) bool {
    const data: *AppendModulesData = @alignCast(@ptrCast(o_data));
    validate_export(data.sys, @"export") catch {
        data.sys.logWarn("skipping export", .{}, @src());
        return true;
    };

    if (data.filter_fn == null or data.filter_fn.?(@"export", data.filter_data)) {
        data.exports.append(data.sys.allocator, @"export") catch |err| {
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
    @"export": *const ProxyModule.Export,
) !void {
    try validate_export(&self.context.module.sys, @"export");
    try self.addModuleInner(owner_inner.handle.?, @"export", owner_inner.instance.?);
}

fn addModulesFromHandle(
    self: *Self,
    module_handle: *ModuleHandle,
    filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
    filter_data: ?*anyopaque,
) !void {
    var append_data = AppendModulesData{
        .sys = &self.context.module.sys,
        .filter_fn = filter_fn,
        .filter_data = filter_data,
    };
    defer append_data.exports.deinit(self.context.allocator);
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
    filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
    filter_data: ?*anyopaque,
) !void {
    const module_handle = try ModuleHandle.initPath(
        self.context.allocator,
        path,
        self.module_load_path.asPath(),
    );
    defer module_handle.unref();
    try self.addModulesFromHandle(module_handle, filter_fn, filter_data);
}

fn addModulesFromLocal(
    self: *Self,
    iterator_fn: ModuleHandle.IteratorFn,
    filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
    filter_data: ?*anyopaque,
    bin_ptr: *const anyopaque,
) !void {
    const module_handle = try ModuleHandle.initLocal(
        self.context.allocator,
        iterator_fn,
        bin_ptr,
    );
    defer module_handle.unref();
    try self.addModulesFromHandle(module_handle, filter_fn, filter_data);
}

fn createQueue(self: *const Self, sys: *System) System.SystemError!Queue {
    var instances = graph.GraphUnmanaged(*ModuleInfo, void).init(null, null);
    defer instances.deinit(self.context.allocator);
    var instance_map = std.StringArrayHashMapUnmanaged(graph.NodeId){};
    defer instance_map.deinit(self.context.allocator);

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
                const owner = self.getModuleInfo(sym.owner).?;
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
        const node = try instances.addNode(self.context.allocator, instance);
        try instance_map.put(self.context.allocator, name.*, node);
    }

    // Connect all nodes in the graph.
    var instance_it = instance_map.iterator();
    connect_graph: while (instance_it.next()) |entry| {
        const name = entry.key_ptr.*;
        const node = entry.value_ptr.*;
        const instance = self.getModuleInfo(name).?;

        for (instance.@"export".getSymbolImports()) |imp| {
            const i_name = std.mem.span(imp.name);
            const i_namespace = std.mem.span(imp.namespace);
            const i_version = Version.initC(imp.version);
            if (self.getSymbol(i_name, i_namespace, i_version)) |sym| {
                const owner_entry = instance_map.get(sym.owner);
                const owner = self.getModuleInfo(sym.owner).?;
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
                    self.context.allocator,
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

    if (try instances.isCyclic(self.context.allocator)) return error.CyclicDependency;
    const order = instances.sortTopological(self.context.allocator, .outgoing) catch |err|
        switch (err) {
        Allocator.Error.OutOfMemory => return Allocator.Error.OutOfMemory,
        else => unreachable,
    };
    defer self.context.allocator.free(order);

    var queue = Queue{};
    errdefer queue.deinit(self.context.allocator);
    for (order) |node| {
        const instance = instances.nodePtr(node).?.*;
        try queue.append(self.context.allocator, instance);
    }

    return queue;
}

// ----------------------------------------------------
// Futures
// ----------------------------------------------------

const CommitOp = FSMFuture(struct {
    set: *Self,
    lock_fut: Async.Mutex.LockOp = undefined,
    ret: anyerror!void = undefined,

    pub const __no_abort = true;

    fn init(set: *Self) EnqueuedFuture(Fallible(void)) {
        set.asSys().logTrace("commiting loading set, set='{*}'", .{set}, @src());
        set.ref();
        errdefer set.unref();

        const data = @This(){
            .set = set,
        };
        var f = CommitOp.init(data).intoFuture().map(
            Fallible(void),
            Fallible(void).Wrapper(anyerror),
        );

        var err: ?AnyError = null;
        defer if (err) |e| e.deinit();
        return Async.Task.initFuture(
            @TypeOf(f),
            &set.context.@"async".sys,
            &f,
            &err,
        ) catch |e| Async.initErrorFuture(void, e);
    }

    pub fn __set_err(self: *@This(), trace: ?*std.builtin.StackTrace, err: anyerror) void {
        if (trace) |tr| self.set.context.tracing.emitStackTraceSimple(tr.*, @src());
        self.ret = err;
    }

    pub fn __ret(self: *@This()) !void {
        return self.ret;
    }

    pub fn __unwind0(self: *@This(), reason: ProxyAsync.FSMUnwindReason) void {
        _ = reason;
        self.set.unref();
    }

    pub fn __state0(self: *@This(), waker: ProxyAsync.Waker) void {
        _ = waker;
        self.lock_fut = self.set.asSys().lockAsync();
    }

    pub fn __unwind1(self: *@This(), reason: ProxyAsync.FSMUnwindReason) void {
        _ = reason;
        self.lock_fut.deinit();
    }

    pub fn __state1(self: *@This(), waker: ProxyAsync.Waker) !ProxyAsync.FSMOp {
        switch (self.lock_fut.poll(waker)) {
            .pending => return .yield,
            .ready => |v| {
                try v;
            },
        }
        errdefer self.set.asSys().mutex.unlock();
        self.set.lock();

        if (self.set.asSys().state == .loading_set) {
            try self.set.asSys().loading_set_waiters.append(
                self.set.asSys().allocator,
                .{ .waiter = self, .waker = waker.ref() },
            );
            self.lock_fut.deinit();
            self.lock_fut = self.set.asSys().lockAsync();
            self.set.asSys().mutex.unlock();
            return .yield;
        }
        self.set.asSys().state = .loading_set;
        return .next;
    }

    pub fn __unwind2(self: *@This(), reason: ProxyAsync.FSMUnwindReason) void {
        _ = reason;
        self.set.unlock();
        self.set.asSys().state = .idle;
        if (self.set.asSys().loading_set_waiters.popOrNull()) |waiter| {
            waiter.waker.wakeUnref();
        }
        self.set.asSys().mutex.unlock();
    }

    pub fn __state2(self: *@This(), waker: ProxyAsync.Waker) !void {
        _ = waker;
        const set = self.set;
        const sys = set.asSys();

        set.should_recreate_map = false;
        var queue = try set.createQueue(sys);
        defer queue.deinit(sys.allocator);
        outer: while (queue.items.len > 0 or set.should_recreate_map) {
            if (set.should_recreate_map) {
                set.should_recreate_map = false;
                queue.deinit(sys.allocator);
                queue = .{};
                queue = try set.createQueue(sys);
                continue;
            }
            const instance_info = queue.pop();
            const instance_name = instance_info.@"export".name;

            // Recheck that all dependencies could be loaded.
            for (instance_info.@"export".getSymbolImports()) |i| {
                const i_name = std.mem.span(i.name);
                const i_namespace = std.mem.span(i.namespace);
                const i_version = Version.initC(i.version);
                if (set.getSymbol(i_name, i_namespace, i_version)) |sym| {
                    const owner = set.getModuleInfo(sym.owner).?;
                    if (owner.status == .err) {
                        sys.logWarn(
                            "instance can not be loaded due to an error loading one of its dependencies...skipping," ++
                                " instance='{s}', dependency='{s}'",
                            .{ instance_name, sym.owner },
                            @src(),
                        );
                        instance_info.signalError();
                        continue :outer;
                    }
                }
            }

            // Check that the explicit dependencies exist.
            for (instance_info.@"export".getModifiers()) |mod| {
                if (mod.tag != .dependency) continue;
                const dependency = mod.value.dependency;
                if (sys.getInstance(std.mem.span(dependency.name)) == null) {
                    sys.logWarn(
                        "instance can not be loaded due to a missing dependency...skipping," ++
                            " instance='{s}', dependency='{s}'",
                        .{ instance_name, dependency.name },
                        @src(),
                    );
                    instance_info.signalError();
                    continue :outer;
                }
            }

            // Construct the instance.
            var err: ?AnyError = null;
            const instance = InstanceHandle.initExportedInstance(
                sys,
                set,
                instance_info.@"export",
                instance_info.handle,
                &err,
            ) catch |e| switch (e) {
                error.FfiError => {
                    sys.logWarn(
                        "instance construction error...skipping," ++
                            " instance='{s}', error='{dbg}:{}'",
                        .{ instance_name, err.?, err.? },
                        @src(),
                    );
                    err.?.deinit();
                    instance_info.signalError();
                    continue :outer;
                },
                else => return e,
            };

            const instance_handle = InstanceHandle.fromInstancePtr(instance);
            const inner = instance_handle.lock();
            errdefer inner.deinit();

            inner.start(sys, &err) catch {
                sys.logWarn(
                    "instance `on_start` error...skipping," ++
                        " instance='{s}', error='{dbg}:{}'",
                    .{ instance_name, err.?, err.? },
                    @src(),
                );
                err.?.deinit();
                instance_info.signalError();
                continue :outer;
            };
            errdefer inner.stop(sys);

            try sys.addInstance(inner);
            defer inner.unlock();
            instance_info.signalSuccess(instance.info);
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

        self.logTrace(
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
        version: c.FimoVersion,
    ) callconv(.c) bool {
        const self: *Self = @alignCast(@ptrCast(this));
        const name_ = std.mem.span(name);
        const namespace_ = std.mem.span(namespace);
        const version_ = Version.initC(version);

        self.logTrace(
            "querying loading set symbol, set='{*}', name='{s}', namespace='{s}', version='{long}'",
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
        on_success: *const fn (info: *const ProxyModule.Info, data: ?*anyopaque) callconv(.C) void,
        on_error: *const fn (module: *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) void,
        on_abort: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        data: ?*anyopaque,
    ) callconv(.c) c.FimoResult {
        const self: *Self = @alignCast(@ptrCast(this));
        const module_ = std.mem.span(module);
        const callback = Callback{
            .data = data,
            .on_success = on_success,
            .on_error = on_error,
        };

        self.logTrace(
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
            if (@errorReturnTrace()) |tr|
                self.context.tracing.emitStackTraceSimple(tr.*, @src());
            if (on_abort) |f| f(data);
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn addModule(
        this: *anyopaque,
        owner: *const ProxyModule.OpaqueInstance,
        @"export": *const ProxyModule.Export,
    ) callconv(.c) c.FimoResult {
        const self: *Self = @alignCast(@ptrCast(this));

        self.logTrace(
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
            if (@errorReturnTrace()) |tr|
                self.context.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn addModulesFromPath(
        this: *anyopaque,
        path: c.FimoUTF8Path,
        filter_fn: *const fn (module: *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        filter_data: ?*anyopaque,
    ) callconv(.c) c.FimoResult {
        const self: *Self = @alignCast(@ptrCast(this));
        const path_ = Path.initC(path);

        self.logTrace(
            "adding modules to loading set, set='{*}', path='{}'",
            .{ self, path_ },
            @src(),
        );

        self.lock();
        defer self.unlock();
        defer if (filter_deinit) |f| f(filter_data);

        self.addModulesFromPath(path_, filter_fn, filter_data) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.context.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn addModulesFromLocal(
        this: *anyopaque,
        filter_fn: *const fn (module: *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        filter_data: ?*anyopaque,
        iterator_fn: *const fn (
            f: *const fn (module: *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
            data: ?*anyopaque,
        ) callconv(.C) void,
        bin_ptr: *const anyopaque,
    ) callconv(.c) c.FimoResult {
        const self: *Self = @alignCast(@ptrCast(this));

        self.logTrace(
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
            if (@errorReturnTrace()) |tr|
                self.context.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn commit(this: *anyopaque) callconv(.c) EnqueuedFuture(Fallible(void)) {
        const self: *Self = @alignCast(@ptrCast(this));
        return CommitOp.Data.init(self);
    }
};

const vtable = ProxyModule.LoadingSet.VTable{
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
