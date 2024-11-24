const std = @import("std");
const Allocator = std.mem.Allocator;

const c = @import("../c.zig");
const AnyError = @import("../AnyError.zig");
const Path = @import("../path.zig").Path;
const Version = @import("../Version.zig");

const InstanceHandle = @import("module/InstanceHandle.zig");
const LoadingSet = @import("module/LoadingSet.zig");
const ModuleHandle = @import("module/ModuleHandle.zig");
const System = @import("module/System.zig");

const Context = @import("../context.zig");
const ProxyContext = @import("proxy_context.zig");
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
    errdefer inner.deinit().unref();

    try self.sys.addInstance(inner);
    inner.unlock();
    return instance;
}

/// Removes a pseudo instance.
pub fn removePseudoInstance(
    self: *Self,
    instance: *const ProxyModule.PseudoInstance,
) System.SystemError!ProxyContext {
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
    const context = inner.deinit();
    errdefer context.unref();
    inner_destroyed = true;

    try self.sys.cleanupLooseInstances();
    return context;
}

/// Initializes a new empty loading set.
pub fn addLoadingSet(self: *Self) Allocator.Error!*LoadingSet {
    self.logTrace("creating new loading set", .{}, @src());
    return LoadingSet.init(self.asContext());
}

/// Queries the loading set for a module.
pub fn queryLoadingSetModule(self: *Self, o_set: *ProxyModule.LoadingSet, name: []const u8) bool {
    self.logTrace(
        "querying loading set module, set='{*}', name='{s}'",
        .{ o_set, name },
        @src(),
    );
    const set = LoadingSet.fromProxySet(o_set);
    set.lock();
    defer set.unlock();
    return set.getModule(name) != null;
}

/// Queries the loading set for a symbol.
pub fn queryLoadingSetSymbol(
    self: *Self,
    o_set: *ProxyModule.LoadingSet,
    name: []const u8,
    namespace: []const u8,
    version: Version,
) bool {
    self.logTrace(
        "querying loading set symbol, set='{*}', name='{s}', namespace='{s}', version='{long}'",
        .{ o_set, name, namespace, version },
        @src(),
    );
    const set = LoadingSet.fromProxySet(o_set);
    set.lock();
    defer set.unlock();
    return set.getSymbol(name, namespace, version) != null;
}

/// Adds a callback to the loading set.
///
/// For unloaded modules, the callback will be invoked after trying to initialize a new module
/// instance. Meanwhile, for already loaded, or failed, modules, the callback will be invoked
/// immediately.
pub fn addLoadingSetCallback(
    self: *Self,
    o_set: *ProxyModule.LoadingSet,
    name: []const u8,
    callback: LoadingSet.Callback,
) (Allocator.Error || error{NotFound})!void {
    self.logTrace(
        "adding callback to loading set, set='{*}', name='{s}', callback='{}'",
        .{ o_set, name, callback },
        @src(),
    );
    const set = LoadingSet.fromProxySet(o_set);
    set.lock();
    defer set.unlock();

    const module_info = set.getModule(name) orelse return error.NotFound;
    try module_info.appendCallback(callback);
}

/// Adds a dynamically created module to the loading set.
///
/// Trying to add a module with an existing name or existing exports will abort the operation.
/// The module inherits a strong reference to the same binary to which the owner is assigned to,
/// therefore it won't be possible to unload the owner for as long as the module is contained in
/// the set. Note that the module does not automatically depend on the owner module.
pub fn addLoadingSetModuleDynamic(
    self: *Self,
    o_set: *ProxyModule.LoadingSet,
    owner: *const ProxyModule.OpaqueInstance,
    @"export": *const ProxyModule.Export,
) !void {
    self.logTrace(
        "adding dynamic module to loading set, set='{*}', module='{s}'",
        .{ o_set, @"export".getName() },
        @src(),
    );
    const set = LoadingSet.fromProxySet(o_set);
    set.lock();
    defer set.unlock();

    const owner_handle = InstanceHandle.fromInstancePtr(owner);
    const owner_inner = owner_handle.lock();
    defer owner_inner.unlock();

    try set.addModuleDynamic(owner_inner, @"export");
}

/// Adds the modules at a path to the loading set.
///
/// Opens up a module binary to select which modules to load. If the path points to a directory,
/// this will seach for the module at the path `path/module.fimo_module`. Otherwise, if `path`
/// is `null` this will iterate over the modules accessed through `iterator_fn`. Trying to add
/// a module with an existing name or existing exports will abort the operation. In that case,
/// no modules are added to the set.
pub fn addLoadingSetModulesFromPath(
    self: *Self,
    o_set: *ProxyModule.LoadingSet,
    module_path: ?Path,
    iterator_fn: ModuleHandle.IteratorFn,
    filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
    filter_data: ?*anyopaque,
    bin_ptr: *const anyopaque,
) !void {
    self.logTrace(
        "adding modules to loading set, set='{*}', path='{?}'",
        .{ o_set, module_path },
        @src(),
    );
    const set = LoadingSet.fromProxySet(o_set);
    set.lock();
    defer set.unlock();

    try set.addModulesFromPath(
        module_path,
        iterator_fn,
        filter_fn,
        filter_data,
        bin_ptr,
    );
}

/// Destroys the loading set without loading any modules.
///
/// It is not possible to dismiss a loading set that is currently being loaded.
pub fn dismissLoadingSet(self: *Self, o_set: *ProxyModule.LoadingSet) System.SystemError!void {
    self.logTrace("dismissing loading set, set='{*}'", .{o_set}, @src());
    const set = LoadingSet.fromProxySet(o_set);
    set.lock();
    errdefer set.unlock();

    if (set.is_loading) return error.LoadingInProcess;
    set.unlock();
    set.deinit();
}

/// Destroys the loading set and loads the contained modules.
///
/// This function will not error, if a module could not be be constructed.
/// It is not possible to load a loading set while a loading set is being loaded.
pub fn loadLoadingSet(
    self: *Self,
    o_set: *ProxyModule.LoadingSet,
) (System.SystemError || InstanceHandle.InstanceHandleError)!void {
    self.logTrace("loading the loading set, set='{*}'", .{o_set}, @src());
    self.sys.lock();
    defer self.sys.unlock();

    const set = LoadingSet.fromProxySet(o_set);
    set.lock();
    errdefer set.unlock();
    try self.sys.loadSet(set);
    set.unlock();
    set.deinit();
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

/// Adds a namespace to the includes of an instance.
///
/// After inclusion, the instance use or store any symbol contained in the removed namespace.
/// It is not possible to include a namespace multiple times or the global namespace.
pub fn addInstanceNamespace(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    namespace: []const u8,
) (System.SystemError || InstanceHandle.InstanceHandleError)!void {
    self.logTrace(
        "adding namespace to instance, instance='{s}', namespace='{s}'",
        .{ instance.info.name, namespace },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const handle = InstanceHandle.fromInstancePtr(instance);
    const inner = handle.lock();
    defer inner.unlock();

    if (std.mem.eql(u8, namespace, System.global_namespace)) return error.NotPermitted;
    if (inner.getNamespace(namespace) != null) return error.Duplicate;

    try self.sys.refNamespace(namespace);
    errdefer self.sys.unrefNamespace(namespace);
    try inner.addNamespace(namespace, .dynamic);
}

/// Removes an included namespace from an instance.
///
/// The instance may not use or store any symbol contained in the removed namespace.
/// It is not possible to remove statically defined namespace includes or the global
/// namespace.
pub fn removeInstanceNamespace(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    namespace: []const u8,
) System.SystemError!void {
    self.logTrace(
        "removing namespace from instance, instance='{s}', namespace='{s}'",
        .{ instance.info.name, namespace },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const handle = InstanceHandle.fromInstancePtr(instance);
    const inner = handle.lock();
    defer inner.unlock();

    if (std.mem.eql(u8, namespace, System.global_namespace)) return error.NotPermitted;
    const ns_info = inner.getNamespace(namespace) orelse return error.NotFound;
    if (ns_info.* == .static) return error.NotPermitted;

    inner.removeNamespace(namespace) catch unreachable;
    self.sys.unrefNamespace(namespace);
}

/// Queries the info of an included namespace.
///
/// If the namespace is not included by the instance, this function returns `null`.
/// Otherwise this function queries whether the namespace was included statically or
/// after the construction of the instance.
pub fn queryInstanceNamespace(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    namespace: []const u8,
) ?InstanceHandle.DependencyType {
    self.logTrace(
        "querying namespace info, instance='{s}', namespace='{s}'",
        .{ instance.info.name, namespace },
        @src(),
    );
    const handle = InstanceHandle.fromInstancePtr(instance);
    const inner = handle.lock();
    defer inner.unlock();

    const namespace_info = inner.getNamespace(namespace) orelse return null;
    return namespace_info.*;
}

/// Adds an instance to the dependency list.
///
/// After adding the dependency, the instance is allowed to use and store any resource of the
/// dependency. Cyclic dependencies are not allowed.
pub fn addInstanceDependency(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    other_info: *const ProxyModule.Info,
) (System.SystemError || InstanceHandle.InstanceHandleError)!void {
    self.logTrace(
        "adding dependency to instance, instance='{s}', other='{s}'",
        .{ instance.info.name, other_info.name },
        @src(),
    );
    if (instance.info == other_info) return error.CyclicDependency;
    self.sys.lock();
    defer self.sys.unlock();

    const handle = InstanceHandle.fromInstancePtr(instance);
    const inner = handle.lock();
    defer inner.unlock();

    const other_handle = InstanceHandle.fromInfoPtr(other_info);
    const other_inner = other_handle.lock();
    defer other_inner.unlock();

    try self.sys.linkInstances(inner, other_inner);
}

/// Removes an instance from the dependency list.
///
/// After removing the dependency, the instance is not allowed to use any resource of the
/// removed dependency. It is only possible to remove dynamically added dependencies.
pub fn removeInstanceDependency(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    other_info: *const ProxyModule.Info,
) System.SystemError!void {
    self.logTrace(
        "removing dependency from instance, instance='{s}', other='{s}'",
        .{ instance.info.name, other_info.name },
        @src(),
    );
    if (instance.info == other_info) return error.NotADependency;
    self.sys.lock();
    defer self.sys.unlock();

    const handle = InstanceHandle.fromInstancePtr(instance);
    const inner = handle.lock();
    defer inner.unlock();

    const other_handle = InstanceHandle.fromInfoPtr(other_info);
    const other_inner = other_handle.lock();
    defer other_inner.unlock();

    try self.sys.unlinkInstances(inner, other_inner);
}

/// Queries the info of a dependent instance.
///
/// If `other_info` is not a dependency of the instance, this function returns `null`.
/// Otherwise this function queries whether the dependency was included statically or
/// after the construction of the instance.
pub fn queryInstanceDependency(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    other_info: *const ProxyModule.Info,
) ?InstanceHandle.DependencyType {
    self.logTrace(
        "querying dependency info, instance='{s}', other='{s}'",
        .{ instance.info.name, other_info.name },
        @src(),
    );
    const handle = InstanceHandle.fromInstancePtr(instance);
    const inner = handle.lock();
    defer inner.unlock();

    const other_instance = inner.getDependency(std.mem.span(other_info.name)) orelse return null;
    return other_instance.type;
}

/// Loads a typed symbol from the module subsystem.
///
/// The caller can query the subsystem for a symbol of a loaded module. This is useful for loading
/// optional symbols, or for loading symbols after the creation of a module. The symbol, if it
/// exists, is returned, and can be used until the module relinquishes the dependency to the module
/// that exported the symbol. This function fails, if the module containing the symbol is not a
/// dependency of the module.
pub fn loadSymbol(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    comptime symbol: ProxyModule.Symbol,
) (System.SystemError || error{NotADependency})!*const symbol.symbol {
    const s = self.loadSymbolRaw(
        instance,
        symbol.name,
        symbol.namespace,
        symbol.version,
    );
    return @alignCast(@ptrCast(s));
}

/// Loads an untyped symbol from the module subsystem.
///
/// The caller can query the subsystem for a symbol of a loaded module. This is useful for loading
/// optional symbols, or for loading symbols after the creation of a module. The symbol, if it
/// exists, is returned, and can be used until the module relinquishes the dependency to the module
/// that exported the symbol. This function fails, if the module containing the symbol is not a
/// dependency of the module.
pub fn loadSymbolRaw(
    self: *Self,
    instance: *const ProxyModule.OpaqueInstance,
    name: []const u8,
    namespace: []const u8,
    version: Version,
) (System.SystemError || error{NotADependency})!*const anyopaque {
    self.logTrace(
        "loading symbol, instance='{s}', name='{s}', namespace='{s}', version='{long}'",
        .{ instance.info.name, name, namespace, version },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();

    const symbol_ref = self.sys.getSymbolCompatible(
        name,
        namespace,
        version,
    ) orelse return error.NotFound;

    const handle = InstanceHandle.fromInstancePtr(instance);
    const inner = handle.lock();
    defer inner.unlock();
    if (inner.getDependency(symbol_ref.owner) == null) return error.NotADependency;
    if (inner.getNamespace(namespace) == null and
        std.mem.eql(u8, namespace, System.global_namespace) == false) return error.NotADependency;

    const owner_instance = self.sys.getInstance(symbol_ref.owner) orelse return error.NotFound;
    const owner_handle = InstanceHandle.fromInstancePtr(owner_instance.instance);
    const owner_inner = owner_handle.lock();
    defer owner_inner.unlock();

    const symbol = owner_inner.getSymbol(name, namespace, version) orelse unreachable;
    return symbol.symbol;
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
    inner.deinit().unref();
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
        context: *c.FimoContext,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        const ctx_ = ctx.module.removePseudoInstance(instance) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        context.* = ctx_.intoC();
        return AnyError.intoCResult(null);
    }
    fn addLoadingSet(
        ptr: *anyopaque,
        set: **ProxyModule.LoadingSet,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        const s = ctx.module.addLoadingSet() catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        set.* = @ptrCast(s);
        return AnyError.intoCResult(null);
    }
    fn queryLoadingSetModule(
        ptr: *anyopaque,
        set: *ProxyModule.LoadingSet,
        name: [*:0]const u8,
        exists: *bool,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        exists.* = ctx.module.queryLoadingSetModule(set, std.mem.span(name));
        return AnyError.intoCResult(null);
    }
    fn queryLoadingSetSymbol(
        ptr: *anyopaque,
        set: *ProxyModule.LoadingSet,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: c.FimoVersion,
        exists: *bool,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        exists.* = ctx.module.queryLoadingSetSymbol(
            set,
            std.mem.span(name),
            std.mem.span(namespace),
            Version.initC(version),
        );
        return AnyError.intoCResult(null);
    }
    fn addLoadingSetCallback(
        ptr: *anyopaque,
        set: *ProxyModule.LoadingSet,
        name: [*:0]const u8,
        on_success: *const fn (info: *const ProxyModule.Info, data: ?*anyopaque) callconv(.C) void,
        on_error: *const fn (module: *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) void,
        data: ?*anyopaque,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.addLoadingSetCallback(
            set,
            std.mem.span(name),
            LoadingSet.Callback{
                .data = data,
                .on_success = on_success,
                .on_error = on_error,
            },
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn addLoadingSetModuleDynamic(
        ptr: *anyopaque,
        owner: *const ProxyModule.OpaqueInstance,
        set: *ProxyModule.LoadingSet,
        @"export": *const ProxyModule.Export,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.addLoadingSetModuleDynamic(
            set,
            owner,
            @"export",
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn addLoadingSetModulesFromPath(
        ptr: *anyopaque,
        set: *ProxyModule.LoadingSet,
        module_path: ?[*:0]const u8,
        filter_fn: ?*const fn (@"export": *const ProxyModule.Export, data: ?*anyopaque) callconv(.C) bool,
        filter_data: ?*anyopaque,
        iterator_fn: ModuleHandle.IteratorFn,
        bin_ptr: *const anyopaque,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        const p = if (module_path) |p|
            Path.init(std.mem.span(p)) catch |e| {
                if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
                return AnyError.initError(e).err;
            }
        else
            null;
        ctx.module.addLoadingSetModulesFromPath(
            set,
            p,
            iterator_fn,
            filter_fn,
            filter_data,
            bin_ptr,
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn dismissLoadingSet(
        ptr: *anyopaque,
        set: *ProxyModule.LoadingSet,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.dismissLoadingSet(set) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn loadLoadingSet(
        ptr: *anyopaque,
        set: *ProxyModule.LoadingSet,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.loadLoadingSet(set) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
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
    fn addInstanceNamespace(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        namespace: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.addInstanceNamespace(instance, std.mem.span(namespace)) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn removeInstanceNamespace(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        namespace: [*:0]const u8,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.removeInstanceNamespace(instance, std.mem.span(namespace)) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn queryInstanceNamespace(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        namespace: [*:0]const u8,
        has_dependency: *bool,
        is_static: *bool,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        if (ctx.module.queryInstanceNamespace(instance, std.mem.span(namespace))) |info| {
            has_dependency.* = true;
            is_static.* = info == .static;
        } else {
            has_dependency.* = false;
            is_static.* = false;
        }
        return AnyError.intoCResult(null);
    }
    fn addInstanceDependency(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        other_info: *const ProxyModule.Info,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.addInstanceDependency(instance, other_info) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn removeInstanceDependency(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        other_info: *const ProxyModule.Info,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        ctx.module.removeInstanceDependency(instance, other_info) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
        return AnyError.intoCResult(null);
    }
    fn queryInstanceDependency(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        other_info: *const ProxyModule.Info,
        has_dependency: *bool,
        is_static: *bool,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        if (ctx.module.queryInstanceDependency(instance, other_info)) |info| {
            has_dependency.* = true;
            is_static.* = info == .static;
        } else {
            has_dependency.* = false;
            is_static.* = false;
        }
        return AnyError.intoCResult(null);
    }
    fn loadSymbol(
        ptr: *anyopaque,
        instance: *const ProxyModule.OpaqueInstance,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: c.FimoVersion,
        symbol: **const anyopaque,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.fromProxyPtr(ptr);
        symbol.* = ctx.module.loadSymbolRaw(
            instance,
            std.mem.span(name),
            std.mem.span(namespace),
            Version.initC(version),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| ctx.tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).err;
        };
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
    .set_has_module = &VTableImpl.queryLoadingSetModule,
    .set_has_symbol = &VTableImpl.queryLoadingSetSymbol,
    .set_append_callback = &VTableImpl.addLoadingSetCallback,
    .set_append_freestanding_module = &VTableImpl.addLoadingSetModuleDynamic,
    .set_append_modules = &VTableImpl.addLoadingSetModulesFromPath,
    .set_dismiss = &VTableImpl.dismissLoadingSet,
    .set_finish = &VTableImpl.loadLoadingSet,
    .find_by_name = &VTableImpl.findInstanceByName,
    .find_by_symbol = &VTableImpl.findInstanceBySymbol,
    .namespace_exists = &VTableImpl.queryNamespace,
    .namespace_include = &VTableImpl.addInstanceNamespace,
    .namespace_exclude = &VTableImpl.removeInstanceNamespace,
    .namespace_included = &VTableImpl.queryInstanceNamespace,
    .acquire_dependency = &VTableImpl.addInstanceDependency,
    .relinquish_dependency = &VTableImpl.removeInstanceDependency,
    .has_dependency = &VTableImpl.queryInstanceDependency,
    .load_symbol = &VTableImpl.loadSymbol,
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
