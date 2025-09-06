const std = @import("std");
const Allocator = std.mem.Allocator;
const ArenaAllocator = std.heap.ArenaAllocator;
const Mutex = std.Thread.Mutex;

const AnyError = @import("../../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const context = @import("../../context.zig");
const pub_context = @import("../../ctx.zig");
const pub_modules = @import("../../modules.zig");
const paths = @import("../../paths.zig");
const PathBuffer = paths.PathBuffer;
const pub_tasks = @import("../../tasks.zig");
const OpaqueFuture = pub_tasks.OpaqueFuture;
const FSMFuture = pub_tasks.FSMFuture;
const Fallible = pub_tasks.Fallible;
const utils = @import("../../utils.zig");
const SliceConst = utils.SliceConst;
const Version = @import("../../Version.zig");
const modules = @import("../modules.zig");
const RefCount = @import("../RefCount.zig");
const tasks = @import("../tasks.zig");
const tracing = @import("../tracing.zig");
const Loader = @import("Loader.zig");
const ModuleHandle = @import("ModuleHandle.zig");
const SymbolRef = @import("SymbolRef.zig");

const Self = @This();

inner: Inner,
type: InstanceType,
handle: pub_modules.Handle.Inner,
ref_count: RefCount = .{},

pub const InstanceType = enum { regular, root };

pub const DependencyType = enum { static, dynamic };

pub const InstanceDependency = struct {
    instance: *Self,
    type: DependencyType,
};

pub const Symbol = struct {
    version: Version,
    symbol: *const anyopaque,
    dtor: ?*const fn (
        ctx: *pub_modules.OpaqueInstance,
        waker: pub_tasks.Waker,
        symbol: *anyopaque,
    ) callconv(.c) bool,

    fn destroySymbol(self: *const Symbol, ctx: *pub_modules.OpaqueInstance) void {
        if (self.dtor) |dtor| {
            if (!dtor(ctx, undefined, @constCast(self.symbol))) {
                @panic("TODO");
            }
        }
    }
};

pub const Parameter = struct {
    inner: Data,
    owner: *Self,
    read_group: pub_modules.ParamAccessGroup,
    write_group: pub_modules.ParamAccessGroup,
    proxy: pub_modules.OpaqueParam.Inner,
    read_fn: *const fn (
        data: pub_modules.OpaqueParamData,
        value: *anyopaque,
    ) callconv(.c) void,
    write_fn: *const fn (
        data: pub_modules.OpaqueParamData,
        value: *const anyopaque,
    ) callconv(.c) void,

    const Data = struct {
        value: union(enum) {
            u8: std.atomic.Value(u8),
            u16: std.atomic.Value(u16),
            u32: std.atomic.Value(u32),
            u64: std.atomic.Value(u64),
            i8: std.atomic.Value(i8),
            i16: std.atomic.Value(i16),
            i32: std.atomic.Value(i32),
            i64: std.atomic.Value(i64),
        },

        fn tag(self: *const @This()) pub_modules.ParamTag {
            return switch (self.value) {
                .u8 => .u8,
                .u16 => .u16,
                .u32 => .u32,
                .u64 => .u64,
                .i8 => .i8,
                .i16 => .i16,
                .i32 => .i32,
                .i64 => .i64,
            };
        }

        fn readTo(self: *const @This(), ptr: *anyopaque) void {
            switch (self.value) {
                .u8 => |*v| @as(*u8, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
                .u16 => |*v| @as(*u16, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
                .u32 => |*v| @as(*u32, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
                .u64 => |*v| @as(*u64, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
                .i8 => |*v| @as(*i8, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
                .i16 => |*v| @as(*i16, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
                .i32 => |*v| @as(*i32, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
                .i64 => |*v| @as(*i64, @ptrCast(@alignCast(ptr))).* = v.load(.seq_cst),
            }
        }

        fn writeFrom(self: *@This(), ptr: *const anyopaque) void {
            switch (self.value) {
                .u8 => |*v| v.store(@as(*const u8, @ptrCast(@alignCast(ptr))).*, .seq_cst),
                .u16 => |*v| v.store(@as(*const u16, @ptrCast(@alignCast(ptr))).*, .seq_cst),
                .u32 => |*v| v.store(@as(*const u32, @ptrCast(@alignCast(ptr))).*, .seq_cst),
                .u64 => |*v| v.store(@as(*const u64, @ptrCast(@alignCast(ptr))).*, .seq_cst),
                .i8 => |*v| v.store(@as(*const i8, @ptrCast(@alignCast(ptr))).*, .seq_cst),
                .i16 => |*v| v.store(@as(*const i16, @ptrCast(@alignCast(ptr))).*, .seq_cst),
                .i32 => |*v| v.store(@as(*const i32, @ptrCast(@alignCast(ptr))).*, .seq_cst),
                .i64 => |*v| v.store(@as(*const i64, @ptrCast(@alignCast(ptr))).*, .seq_cst),
            }
        }

        fn asProxyParameter(self: *@This()) pub_modules.OpaqueParamData {
            return .{
                .data = self,
                .vtable = &param_data_vtable,
            };
        }
    };

    fn init(
        owner: *Self,
        inner: Data,
        read_group: pub_modules.ParamAccessGroup,
        write_group: pub_modules.ParamAccessGroup,
        read_fn: ?*const fn (
            data: pub_modules.OpaqueParamData,
            value: *anyopaque,
        ) callconv(.c) void,
        write_fn: ?*const fn (
            data: pub_modules.OpaqueParamData,
            value: *const anyopaque,
        ) callconv(.c) void,
    ) Allocator.Error!*Parameter {
        const Wrapper = struct {
            fn read(this: pub_modules.OpaqueParamData, value: *anyopaque) callconv(.c) void {
                const self: *Data = @ptrCast(@alignCast(this.data));
                self.readTo(value);
            }
            fn write(this: pub_modules.OpaqueParamData, value: *const anyopaque) callconv(.c) void {
                const self: *Data = @ptrCast(@alignCast(this.data));
                self.writeFrom(value);
            }
        };

        const p = try owner.inner.arena.allocator().create(Parameter);
        p.* = .{
            .inner = inner,
            .owner = owner,
            .read_group = read_group,
            .write_group = write_group,
            .proxy = param_inner,
            .read_fn = read_fn orelse &Wrapper.read,
            .write_fn = write_fn orelse &Wrapper.write,
        };
        return p;
    }

    pub fn checkTag(self: *const Parameter, ty: pub_modules.ParamTag) !void {
        if (!(self.inner.tag() == ty)) return error.InvalidParameterTag;
    }

    pub fn checkReadPublic(self: *const Parameter) !void {
        if (!(self.read_group == .public)) return error.NotPermitted;
    }

    pub fn checkWritePublic(self: *const Parameter) !void {
        if (!(self.write_group == .public)) return error.NotPermitted;
    }

    fn checkReadDependency(self: *const Parameter, reader: *const Inner) !void {
        const min_permission: pub_modules.ParamAccessGroup = .dependency;
        if (@intFromEnum(self.read_group) > @intFromEnum(min_permission)) return error.NotPermitted;
        const owner_name = self.owner.handle.name.intoSliceOrEmpty();
        if (!reader.dependencies.contains(owner_name)) return error.NotADependency;
    }

    fn checkWriteDependency(self: *const Parameter, writer: *const Inner) !void {
        const min_permission: pub_modules.ParamAccessGroup = .dependency;
        if (@intFromEnum(self.write_group) > @intFromEnum(min_permission)) return error.NotPermitted;
        const owner_name = self.owner.handle.name.intoSliceOrEmpty();
        if (!writer.dependencies.contains(owner_name)) return error.NotADependency;
    }

    pub fn tag(self: *const Parameter) pub_modules.ParamTag {
        return self.inner.tag();
    }

    pub fn readTo(self: *const Parameter, value: *anyopaque) void {
        self.read_fn(@constCast(self).inner.asProxyParameter(), value);
    }

    pub fn writeFrom(self: *Parameter, value: *const anyopaque) void {
        self.write_fn(self.inner.asProxyParameter(), value);
    }
};

pub const Inner = struct {
    mutex: Mutex = .{},
    state: State = .uninit,
    arena: ArenaAllocator,
    strong_count: usize = 0,
    dependents_count: usize = 0,
    is_detached: bool = false,
    unload_requested: bool = false,
    unload_waiter: ?pub_tasks.Waker = null,
    handle: ?*ModuleHandle = null,
    @"export": ?*const pub_modules.Export = null,
    instance: ?*pub_modules.OpaqueInstance.Inner = null,
    string_cache: std.StringArrayHashMapUnmanaged(void) = .{},
    symbols: std.ArrayHashMapUnmanaged(SymbolRef.Id, Symbol, SymbolRef.Id.HashContext, false) = .{},
    parameters: std.StringArrayHashMapUnmanaged(*Parameter) = .{},
    namespaces: std.StringArrayHashMapUnmanaged(DependencyType) = .{},
    dependencies: std.StringArrayHashMapUnmanaged(InstanceDependency) = .{},

    const State = enum {
        uninit,
        init,
        started,
    };

    pub fn deinit(self: *Inner) void {
        std.debug.assert(self.handle != null);
        const handle = Self.fromInnerPtr(self);
        self.detach();
        self.unlock();
        handle.unref();
    }

    pub fn unlock(self: *Inner) void {
        self.mutex.unlock();
    }

    pub fn isDetached(self: *const Inner) bool {
        return self.is_detached;
    }

    pub fn isUnloading(self: *const Inner) bool {
        return self.unload_requested;
    }

    pub fn canUnload(self: *const Inner) bool {
        std.debug.assert(!self.isDetached());
        return self.strong_count == 0 and self.dependents_count == 0;
    }

    pub fn enqueueUnload(self: *Inner) !void {
        if (self.isUnloading() or self.isDetached()) return;
        self.unload_requested = true;

        const x = Self.fromInnerPtr(self);
        try EnqueueUnloadOp.Data.init(x);
    }

    fn checkAsyncUnload(self: *Inner, waiter: pub_tasks.Waker) enum { noop, wait, unload } {
        if (self.isDetached()) return .noop;
        std.debug.assert(self.unload_waiter == null);
        if (self.canUnload()) return .unload;

        self.unload_waiter = waiter.ref();
        return .wait;
    }

    fn unblockUnload(self: *Inner) void {
        std.debug.assert(!self.isDetached());
        if (!self.unload_requested) return;
        if (!self.canUnload()) return;

        if (self.unload_waiter) |waiter| {
            self.unload_waiter = null;
            waiter.wakeUnref();
        }
    }

    pub fn refStrong(self: *Inner) !void {
        if (self.isDetached()) return error.Detached;
        self.strong_count += 1;
    }

    pub fn unrefStrong(self: *Inner) void {
        std.debug.assert(!self.isDetached());
        self.strong_count -= 1;
        self.unblockUnload();
    }

    fn cacheString(self: *Inner, value: []const u8) Allocator.Error![]const u8 {
        if (self.string_cache.getKey(value)) |v| return v;
        const alloc = self.arena.allocator();
        const cached = try alloc.dupe(u8, value);
        errdefer alloc.free(cached);
        try self.string_cache.put(alloc, cached, {});
        return cached;
    }

    pub fn getSymbol(self: *Inner, name: []const u8, namespace: []const u8, version: Version) ?*Symbol {
        if (self.isDetached()) return null;
        const sym = self.symbols.getPtr(.{
            .name = name,
            .namespace = namespace,
        }) orelse return null;
        if (!sym.version.sattisfies(version)) return null;
        return sym;
    }

    fn addSymbol(self: *Inner, name: []const u8, namespace: []const u8, sym: Symbol) !void {
        if (self.isDetached()) return error.Detached;
        const key = SymbolRef.Id{
            .name = try self.cacheString(name),
            .namespace = try self.cacheString(namespace),
        };
        try self.symbols.put(self.arena.allocator(), key, sym);
    }

    pub fn getParameter(self: *Inner, name: []const u8) ?*Parameter {
        if (self.isDetached()) return null;
        return self.parameters.get(name);
    }

    fn addParameter(self: *Inner, name: []const u8, param: *Parameter) !void {
        if (self.isDetached()) return error.Detached;
        const n = try self.cacheString(name);
        try self.parameters.put(self.arena.allocator(), n, param);
    }

    pub fn getNamespace(self: *Inner, name: []const u8) ?*DependencyType {
        if (self.isDetached()) return null;
        return self.namespaces.getPtr(name);
    }

    pub fn addNamespace(self: *Inner, name: []const u8, @"type": DependencyType) !void {
        if (self.isDetached()) return error.Detached;
        const n = try self.cacheString(name);
        try self.namespaces.put(self.arena.allocator(), n, @"type");
    }

    pub fn removeNamespace(self: *Inner, name: []const u8) !void {
        if (self.isDetached()) return error.Detached;
        if (!self.namespaces.swapRemove(name)) return error.NotFound;
    }

    pub fn getDependency(self: *Inner, name: []const u8) ?*InstanceDependency {
        if (self.isDetached()) return null;
        return self.dependencies.getPtr(name);
    }

    pub fn addDependency(self: *Inner, other: *Inner, @"type": DependencyType) !void {
        std.debug.assert(self != other);
        if (self.isDetached() or other.isDetached()) return error.Detached;

        const handle = Self.fromInnerPtr(other);
        const name = handle.handle.name.intoSliceOrEmpty();
        const dep = InstanceDependency{
            .instance = handle,
            .type = @"type",
        };

        handle.ref();
        errdefer handle.unref();

        const n = try self.cacheString(name);
        try self.dependencies.put(self.arena.allocator(), n, dep);
        other.dependents_count += 1;
    }

    pub fn removeDependency(self: *Inner, other: *Inner) !void {
        std.debug.assert(self != other);
        if (self.isDetached()) return error.Detached;
        std.debug.assert(!other.isDetached());

        const handle = Self.fromInnerPtr(other);
        const name = handle.handle.name.intoSliceOrEmpty();
        if (!self.dependencies.swapRemove(name)) return error.NotFound;
        other.dependents_count -= 1;
        other.unblockUnload();
        handle.unref();
    }

    pub fn clearDependencies(self: *Inner) void {
        std.debug.assert(!self.isDetached());
        for (self.dependencies.values()) |dep| {
            const handle = dep.instance;
            const inner = handle.lock();
            defer inner.unlock();
            inner.dependents_count -= 1;
            inner.unblockUnload();
            handle.unref();
        }
        self.dependencies.clearRetainingCapacity();
    }

    pub fn start(self: *Inner) StartInstanceOp {
        return StartInstanceOp.Data.init(self);
    }

    pub fn stop(self: *Inner) void {
        std.debug.assert(!self.isDetached());
        std.debug.assert(self.state == .started);
        self.is_detached = true;
        if (self.@"export") |@"export"| {
            const stop_event = @"export".eventStop();
            if (stop_event.poll) |poll| {
                self.unlock();
                modules.mutex.unlock();
                // TODO: Implement
                while (!poll(@ptrCast(self.instance.?), undefined)) {
                    @panic("TODO");
                }
                modules.mutex.lock();
                self.mutex.lock();
            }
        }
        self.is_detached = false;
        self.state = .init;
    }

    fn detach(self: *Inner) void {
        std.debug.assert(self.canUnload());
        std.debug.assert(self.state != .started);
        std.debug.assert(self.dependencies.count() == 0);
        const instance = self.instance.?;

        self.is_detached = true;
        if (self.@"export") |exp| {
            if (self.state == .init) {
                const deinit_event = exp.eventDeinit();
                if (deinit_event.poll) |poll| {
                    // TODO: Implement
                    while (!poll(@ptrCast(instance), undefined, instance.state)) {
                        @panic("TODO");
                    }
                }
            }

            const deinit_export_event = exp.eventDeinitExport();
            if (deinit_export_event.deinit) |f| f(deinit_export_event.data);
        }

        for (self.symbols.values()) |sym| sym.destroySymbol(@ptrCast(instance));

        self.symbols.clearRetainingCapacity();
        self.parameters.clearRetainingCapacity();
        self.namespaces.clearRetainingCapacity();
        self.dependencies.clearRetainingCapacity();

        self.handle.?.unref();

        self.handle = null;
        self.instance = null;
        self.@"export" = null;
    }
};

fn init(
    name: []const u8,
    description: ?[]const u8,
    author: ?[]const u8,
    license: ?[]const u8,
    module_path: ?[]const u8,
    handle: *ModuleHandle,
    @"export": ?*const pub_modules.Export,
    @"type": InstanceType,
) !*Self {
    var arena = ArenaAllocator.init(modules.allocator);
    errdefer arena.deinit();
    const allocator = arena.allocator();

    const self = try allocator.create(Self);
    self.* = .{
        .inner = .{
            .arena = undefined,
            .handle = handle,
            .@"export" = @"export",
        },
        .type = @"type",
        .handle = .{
            .name = .fromSlice(try allocator.dupe(u8, name)),
            .description = .fromSlice(if (description) |str| try allocator.dupe(u8, str) else null),
            .author = .fromSlice(if (author) |str| try allocator.dupe(u8, str) else null),
            .license = .fromSlice(if (license) |str| try allocator.dupe(u8, str) else null),
            .module_path = .fromSlice(if (module_path) |str| try allocator.dupe(u8, str) else null),
            .ref = &HandleImpl.ref,
            .unref = &HandleImpl.unref,
            .mark_unloadable = &HandleImpl.markUnloadable,
            .is_loaded = &HandleImpl.isLoaded,
            .try_ref_instance_strong = &HandleImpl.tryRefInstanceStrong,
            .unref_instance_strong = &HandleImpl.unrefInstanceStrong,
        },
    };
    // Move the new state of the arena into the allocated handle.
    self.inner.arena = arena;
    modules.instance_count.increase();

    return self;
}

pub fn initRootInstance(name: []const u8) !*pub_modules.RootInstance {
    const iterator = &pub_modules.exports.ExportIter.fstd__module_export_iter;
    const handle = try ModuleHandle.initLocal(modules.allocator, iterator, iterator);
    errdefer handle.unref();

    const instance_handle = try Self.init(name, null, null, null, null, handle, null, .root);
    errdefer instance_handle.unref();

    const instance = try instance_handle.inner.arena.allocator().create(pub_modules.OpaqueInstance.Inner);
    instance.* = .{
        .vtable = &instance_vtable,
        .parameters = null,
        .resources = null,
        .imports = null,
        .exports = null,
        .handle = @ptrCast(&instance_handle.handle),
        .ctx_handle = @ptrCast(&context.handle),
        .state = undefined,
    };
    instance_handle.inner.state = .started;
    instance_handle.inner.instance = instance;
    return @ptrCast(instance);
}

pub fn fromInstancePtr(instance: *pub_modules.OpaqueInstance) *Self {
    return fromHandlePtr(@ptrCast(@alignCast(instance.handle())));
}

pub fn fromHandlePtr(handle: *pub_modules.Handle.Inner) *Self {
    return @fieldParentPtr("handle", handle);
}

pub fn fromInnerPtr(inner: *Inner) *Self {
    return @fieldParentPtr("inner", inner);
}

fn ref(self: *Self) void {
    self.ref_count.ref();
}

fn unref(self: *Self) void {
    if (self.ref_count.unref() == .noop) return;

    const inner = self.lock();
    if (!inner.isDetached()) inner.detach();
    inner.arena.deinit();
    modules.instance_count.decrease();
}

pub fn lock(self: *Self) *Inner {
    self.inner.mutex.lock();
    return &self.inner;
}

fn addNamespace(self: *Self, namespace: []const u8) !void {
    tracing.logTrace(
        @src(),
        "adding namespace to instance, instance='{s}', namespace='{s}'",
        .{ self.handle.name.intoSliceOrEmpty(), namespace },
    );

    if (std.mem.eql(u8, namespace, modules.global_namespace)) return error.NotPermitted;

    modules.mutex.lock();
    defer modules.mutex.unlock();

    const inner = self.lock();
    defer inner.unlock();

    if (inner.getNamespace(namespace) != null) return error.Duplicate;

    try modules.refNamespace(namespace);
    errdefer modules.unrefNamespace(namespace);
    try inner.addNamespace(namespace, .dynamic);
}

fn removeNamespace(self: *Self, namespace: []const u8) !void {
    tracing.logTrace(
        @src(),
        "removing namespace from instance, instance='{s}', namespace='{s}'",
        .{ self.handle.name.intoSliceOrEmpty(), namespace },
    );

    if (std.mem.eql(u8, namespace, modules.global_namespace)) return error.NotPermitted;

    modules.mutex.lock();
    defer modules.mutex.unlock();

    const inner = self.lock();
    defer inner.unlock();

    const ns_info = inner.getNamespace(namespace) orelse return error.NotFound;
    if (ns_info.* == .static) return error.NotPermitted;

    try inner.removeNamespace(namespace);
    modules.unrefNamespace(namespace);
}

fn addDependency(self: *Self, handle: *pub_modules.Handle) !void {
    tracing.logTrace(
        @src(),
        "adding dependency to instance, instance='{s}', other='{s}'",
        .{ self.handle.name.intoSliceOrEmpty(), handle.name() },
    );

    const info_handle = Self.fromHandlePtr(@ptrCast(@alignCast(handle)));
    if (self == info_handle) return error.CyclicDependency;

    modules.mutex.lock();
    defer modules.mutex.unlock();

    const inner = self.lock();
    defer inner.unlock();

    const info_inner = info_handle.lock();
    defer info_inner.unlock();

    try modules.linkInstances(inner, info_inner);
}

fn removeDependency(self: *Self, handle: *pub_modules.Handle) !void {
    tracing.logTrace(
        @src(),
        "removing dependency from instance, instance='{s}', other='{s}'",
        .{ self.handle.name.intoSliceOrEmpty(), handle.name() },
    );

    const info_handle = Self.fromHandlePtr(@ptrCast(@alignCast(handle)));
    if (self == info_handle) return error.NotADependency;

    modules.mutex.lock();
    defer modules.mutex.unlock();

    const inner = self.lock();
    defer inner.unlock();

    const info_inner = info_handle.lock();
    defer info_inner.unlock();

    try modules.unlinkInstances(inner, info_inner);
}

fn loadSymbol(self: *Self, name: []const u8, namespace: []const u8, version: Version) !*const anyopaque {
    tracing.logTrace(
        @src(),
        "loading symbol, instance='{s}', name='{s}', namespace='{s}', version='{f}'",
        .{ self.handle.name.intoSliceOrEmpty(), name, namespace, version },
    );
    modules.mutex.lock();
    defer modules.mutex.unlock();
    const symbol_ref = modules.getSymbolCompatible(
        name,
        namespace,
        version,
    ) orelse return error.NotFound;

    const inner = self.lock();
    defer inner.unlock();

    if (inner.getDependency(symbol_ref.owner) == null) return error.NotADependency;
    if (inner.getNamespace(namespace) == null and
        !std.mem.eql(u8, namespace, modules.global_namespace))
    {
        return error.NotADependency;
    }

    const owner_instance = modules.getInstance(symbol_ref.owner) orelse return error.NotFound;
    const owner_handle = Self.fromInstancePtr(owner_instance.instance);
    const owner_inner = owner_handle.lock();
    defer owner_inner.unlock();

    const symbol = owner_inner.getSymbol(
        name,
        namespace,
        version,
    ) orelse return error.Detached;
    return symbol.symbol;
}

fn readParameter(self: *Self, value: *anyopaque, tag: pub_modules.ParamTag, module: []const u8, parameter: []const u8) !void {
    tracing.logTrace(
        @src(),
        "reading dependency parameter, reader='{s}', value='{*}', tag='{t}', module='{s}', parameter='{s}'",
        .{ self.handle.name.intoSliceOrEmpty(), value, tag, module, parameter },
    );
    const inner = self.lock();
    defer inner.unlock();

    const module_handle = inner.getDependency(module) orelse return error.NotADependency;
    const module_inner = module_handle.instance.lock();
    defer module_inner.unlock();

    const param = module_inner.getParameter(parameter) orelse return error.NotFound;
    try param.checkReadDependency(inner);
    param.readTo(value);
}

fn writeParameter(self: *Self, value: *const anyopaque, tag: pub_modules.ParamTag, module: []const u8, parameter: []const u8) !void {
    tracing.logTrace(
        @src(),
        "writing dependency parameter, writer='{s}', value='{*}', tag='{t}', module='{s}', parameter='{s}'",
        .{ self.handle.name.intoSliceOrEmpty(), value, tag, module, parameter },
    );
    const inner = self.lock();
    defer inner.unlock();

    const module_handle = inner.getDependency(module) orelse return error.NotADependency;
    const module_inner = module_handle.instance.lock();
    defer module_inner.unlock();

    const param = module_inner.getParameter(parameter) orelse return error.NotFound;
    try param.checkWriteDependency(inner);
    param.writeFrom(value);
}

// ----------------------------------------------------
// Parameter VTable
// ----------------------------------------------------

const ParamVTableImpl = struct {
    fn tag(data: *const pub_modules.OpaqueParam.Inner) callconv(.c) pub_modules.ParamTag {
        const self: *const Parameter = @fieldParentPtr("proxy", data);
        return self.tag();
    }
    fn read(data: *const pub_modules.OpaqueParam.Inner, value: *anyopaque) callconv(.c) void {
        const self: *const Parameter = @fieldParentPtr("proxy", data);
        return self.readTo(value);
    }
    fn write(data: *pub_modules.OpaqueParam.Inner, value: *const anyopaque) callconv(.c) void {
        const self: *Parameter = @fieldParentPtr("proxy", data);
        return self.writeFrom(value);
    }
};

const param_inner = pub_modules.OpaqueParam.Inner{
    .tag = &ParamVTableImpl.tag,
    .read = &ParamVTableImpl.read,
    .write = &ParamVTableImpl.write,
};

// ----------------------------------------------------
// ParameterData VTable
// ----------------------------------------------------

const ParamDataVTableImpl = struct {
    fn tag(data: *anyopaque) callconv(.c) pub_modules.ParamTag {
        const self: *Parameter.Data = @ptrCast(@alignCast(data));
        return self.tag();
    }
    fn read(data: *anyopaque, value: *anyopaque) callconv(.c) void {
        const self: *Parameter.Data = @ptrCast(@alignCast(data));
        return self.readTo(value);
    }
    fn write(data: *anyopaque, value: *const anyopaque) callconv(.c) void {
        const self: *Parameter.Data = @ptrCast(@alignCast(data));
        return self.writeFrom(value);
    }
};

const param_data_vtable = pub_modules.OpaqueParamData.VTable{
    .tag = &ParamDataVTableImpl.tag,
    .read = &ParamDataVTableImpl.read,
    .write = &ParamDataVTableImpl.write,
};

// ----------------------------------------------------
// Info Futures
// ----------------------------------------------------

const EnqueueUnloadOp = FSMFuture(struct {
    handle: *Self,
    ret: void = undefined,

    pub const __no_abort = true;

    fn init(handle: *Self) !void {
        tracing.logTrace(
            @src(),
            "enqueueing unload of instance, instance='{s}'",
            .{handle.handle.name.intoSliceOrEmpty()},
        );
        handle.ref();

        const f = EnqueueUnloadOp.init(@This(){
            .handle = handle,
        }).intoFuture();
        var enqueued = try tasks.Task.initFuture(@TypeOf(f), &f);

        // Detaches the future.
        enqueued.deinit();
    }

    pub fn __ret(self: *@This()) void {
        return self.ret;
    }

    pub fn __unwind0(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = reason;
        self.handle.unref();
    }

    pub fn __state0(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        tracing.logTrace(
            @src(),
            "attempting to unload instance, instance=`{s}`",
            .{self.handle.handle.name.intoSliceOrEmpty()},
        );

        const inner = self.handle.lock();
        defer inner.unlock();
        switch (inner.checkAsyncUnload(waker)) {
            .noop => {
                tracing.logTrace(
                    @src(),
                    "skipping unload, already unloaded, instance=`{s}`",
                    .{self.handle.handle.name.intoSliceOrEmpty()},
                );
                self.ret = {};
                return .ret;
            },
            .wait => {
                tracing.logTrace(
                    @src(),
                    "unload blocked, instance=`{s}`",
                    .{self.handle.handle.name.intoSliceOrEmpty()},
                );
                return .yield;
            },
            .unload => return .next,
        }
    }

    pub fn __unwind1(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = self;
        _ = reason;
    }

    pub fn __state1(self: *@This(), waker: pub_tasks.Waker) void {
        _ = waker;

        modules.mutex.lock();
        defer modules.mutex.unlock();

        tracing.logTrace(
            @src(),
            "unloading instance, instance=`{s}`",
            .{self.handle.handle.name.intoSliceOrEmpty()},
        );
        const inner = self.handle.lock();
        modules.removeInstance(inner) catch |err| @panic(@errorName(err));
        inner.stop();
        inner.deinit();
        self.ret = {};
    }
});

// ----------------------------------------------------
// Handle implementation
// ----------------------------------------------------

const HandleImpl = struct {
    fn ref(handle: *pub_modules.Handle.Inner) callconv(.c) void {
        const x = fromHandlePtr(handle);
        x.ref();
    }
    fn unref(handle: *pub_modules.Handle.Inner) callconv(.c) void {
        const x = fromHandlePtr(handle);
        x.unref();
    }
    fn markUnloadable(handle: *pub_modules.Handle.Inner) callconv(.c) void {
        const x = fromHandlePtr(handle);
        const inner = x.lock();
        defer inner.unlock();
        inner.enqueueUnload() catch |e| @panic(@errorName(e));
    }
    fn isLoaded(handle: *pub_modules.Handle.Inner) callconv(.c) bool {
        const x = fromHandlePtr(handle);
        const inner = x.lock();
        defer inner.unlock();
        return !inner.isDetached();
    }
    fn tryRefInstanceStrong(handle: *pub_modules.Handle.Inner) callconv(.c) bool {
        const x = fromHandlePtr(handle);
        const inner = x.lock();
        defer inner.unlock();
        inner.refStrong() catch return false;
        return true;
    }
    fn unrefInstanceStrong(handle: *pub_modules.Handle.Inner) callconv(.c) void {
        const x = fromHandlePtr(handle);
        const inner = x.lock();
        defer inner.unlock();
        inner.unrefStrong();
    }
};

// ----------------------------------------------------
// Instance Futures
// ----------------------------------------------------

pub const InitExportedOp = FSMFuture(struct {
    /// The state machine is split up into the following states:
    ///
    /// 0:
    ///     1. alloc instance
    ///     2. init parameters
    ///     3. init resources
    ///     4. init imports
    ///     5. if (has state) goto 1 else goto 2
    /// 1:
    ///     1. wait for state future to complete
    ///     2. goto 2
    /// 2:
    ///     1. init exports array
    ///     2. goto 3
    /// 3:
    ///     1. if (no export left) return
    ///     2. poll next export
    ///         2.1. if (error) goto 4
    /// 4:
    ///     1. poll deinit in reverse
    loader: *pub_modules.Loader,
    @"export": *const pub_modules.Export,
    handle: *ModuleHandle,
    instance_handle: *Self = undefined,
    inner: *Inner = undefined,
    instance: *pub_modules.OpaqueInstance.Inner = undefined,
    init_event: pub_modules.exports.events.Init = undefined,
    export_index: usize = 0,
    exports: []*const anyopaque = undefined,
    ret: Error!*pub_modules.OpaqueInstance = undefined,

    pub const Error = anyerror;
    pub const __no_abort = true;

    pub fn __set_err(self: *@This(), trace: ?*std.builtin.StackTrace, err: Error) void {
        if (trace) |tr| tracing.logStackTrace(@src(), tr.*);
        self.ret = err;
    }

    pub fn __ret(self: *@This()) Error!*pub_modules.OpaqueInstance {
        return self.ret;
    }

    pub fn __state0(self: *@This(), waker: pub_tasks.Waker) Error!pub_tasks.FSMOpExt(@This()) {
        _ = waker;

        const instance_handle = try Self.init(
            self.@"export".name.intoSliceOrEmpty(),
            self.@"export".description.intoSlice(),
            self.@"export".author.intoSlice(),
            self.@"export".license.intoSlice(),
            self.handle.path.raw,
            self.handle,
            self.@"export",
            .regular,
        );
        self.handle.ref();
        errdefer instance_handle.unref();
        self.instance_handle = instance_handle;

        const inner = instance_handle.lock();
        errdefer inner.unlock();
        self.inner = inner;

        inner.refStrong() catch unreachable;
        errdefer inner.unrefStrong();

        const instance = try instance_handle.inner.arena
            .allocator().create(pub_modules.OpaqueInstance.Inner);
        instance.* = .{
            .vtable = &instance_vtable,
            .parameters = null,
            .resources = null,
            .imports = null,
            .exports = null,
            .handle = @ptrCast(&instance_handle.handle),
            .ctx_handle = &context.handle,
            .state = undefined,
        };
        inner.instance = instance;
        self.instance = instance;
        const allocator = inner.arena.allocator();

        // Init parameters.
        const exp_parameters = self.@"export".parameters.intoSliceOrEmpty();
        const parameters = try allocator.alloc(*pub_modules.OpaqueParam, exp_parameters.len);
        instance.parameters = @ptrCast(parameters.ptr);
        for (exp_parameters, parameters) |src, *dst| {
            const data = Parameter.Data{
                .value = switch (src.tag) {
                    .u8 => .{ .u8 = std.atomic.Value(u8).init(src.default_value.u8) },
                    .u16 => .{ .u16 = std.atomic.Value(u16).init(src.default_value.u16) },
                    .u32 => .{ .u32 = std.atomic.Value(u32).init(src.default_value.u32) },
                    .u64 => .{ .u64 = std.atomic.Value(u64).init(src.default_value.u64) },
                    .i8 => .{ .i8 = std.atomic.Value(i8).init(src.default_value.i8) },
                    .i16 => .{ .i16 = std.atomic.Value(i16).init(src.default_value.i16) },
                    .i32 => .{ .i32 = std.atomic.Value(i32).init(src.default_value.i32) },
                    .i64 => .{ .i64 = std.atomic.Value(i64).init(src.default_value.i64) },
                    else => return error.InvalidParameterTag,
                },
            };
            var param: *Parameter = try Parameter.init(
                instance_handle,
                data,
                src.read_group,
                src.write_group,
                src.read,
                src.write,
            );
            try inner.addParameter(src.name.intoSliceOrEmpty(), param);
            dst.* = @ptrCast(&param.proxy);
        }

        // Init resources.
        const exp_resources = self.@"export".resources.intoSliceOrEmpty();
        const resources = try allocator.alloc(paths.compat.Path, exp_resources.len);
        instance.resources = @ptrCast(resources.ptr);
        for (exp_resources, resources) |src, *dst| {
            var buf = PathBuffer{};
            try buf.pushPath(inner.arena.allocator(), self.handle.path.asPath());
            try buf.pushString(inner.arena.allocator(), src.intoSliceOrEmpty());
            // Append a null-terminator to ensure that the path can be passed to c interfaces.
            try buf.buffer.append(inner.arena.allocator(), 0);
            dst.* = buf.asPath().intoC();
            dst.len -= 1;
        }

        // Init namespaces.
        for (self.@"export".namespaces.intoSliceOrEmpty()) |imp| {
            const name = imp.intoSliceOrEmpty();
            if (modules.getNamespace(name) == null) return error.NotFound;
            try inner.addNamespace(name, .static);
        }

        // Init imports.
        const exp_imports = self.@"export".imports.intoSliceOrEmpty();
        const imports = try allocator.alloc(*const anyopaque, exp_imports.len);
        instance.imports = @ptrCast(imports.ptr);
        for (exp_imports, imports) |src, *dst| {
            const src_name = src.name.intoSliceOrEmpty();
            const src_namespace = src.namespace.intoSliceOrEmpty();
            const src_version = Version.initC(src.version);
            const sym = modules.getSymbolCompatible(
                src_name,
                src_namespace,
                src_version,
            ) orelse return error.NotFound;

            const owner = modules.getInstance(sym.owner).?;
            const owner_handle = Self.fromInstancePtr(owner.instance);
            const owner_inner = owner_handle.lock();
            defer owner_inner.unlock();

            const owner_sym = owner_inner.getSymbol(
                src_name,
                src_namespace,
                src_version,
            ).?;
            if (inner.getDependency(sym.owner) == null) try inner.addDependency(owner_inner, .static);
            dst.* = owner_sym.symbol;
        }

        // Init instance data.
        const init_event = self.@"export".eventInit();
        if (init_event.poll != null) {
            self.init_event = init_event;
            inner.unlock();
            modules.mutex.unlock();
            return .{ .transition = 1 };
        }
        inner.state = .init;
        return .{ .transition = 2 };
    }

    pub fn __unwind1(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        if (reason != .completed) self.inner.unrefStrong();
        self.inner.unlock();
        if (reason != .completed) self.instance_handle.unref();
    }

    pub fn __state1(self: *@This(), waker: pub_tasks.Waker) Error!pub_tasks.FSMOp {
        const poll = self.init_event.poll.?;
        var state: Fallible(*anyopaque) = undefined;
        if (!poll(@ptrCast(self.instance), self.loader, waker, &state)) return .yield;
        modules.mutex.lock();
        _ = self.instance_handle.lock();
        self.instance.state = try state.unwrap();
        self.inner.state = .init;
        return .next;
    }

    pub fn __state2(self: *@This(), waker: pub_tasks.Waker) Error!void {
        _ = waker;
        const @"export" = self.@"export";
        const inner = self.inner;
        const allocator = inner.arena.allocator();

        // Init exports array.
        const exports = try allocator.alloc(*const anyopaque, @"export".exports.len);
        self.exports = exports;
        self.instance.exports = @ptrCast(exports.ptr);
    }

    pub fn __state3(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        self.inner.unlock();
        modules.mutex.unlock();
        defer {
            modules.mutex.lock();
            _ = self.instance_handle.lock();
        }

        const exports = self.@"export".exports.intoSliceOrEmpty();
        while (self.export_index < exports.len) : (self.export_index += 1) {
            const exp = exports[self.export_index];
            const sym, const dtor = switch (exp.sym_ty) {
                .static => .{ exp.value.static, null },
                .dynamic => blk: {
                    var result: Fallible(*anyopaque) = undefined;
                    if (!exp.value.dynamic.poll_init(@ptrCast(self.instance), waker, &result)) return .yield;
                    const value = result.unwrap() catch |err| {
                        self.ret = err;
                        return .next;
                    };
                    break :blk .{ value, exp.value.dynamic.poll_deinit };
                },
                else => unreachable,
            };

            const exp_name = exp.symbol.name.intoSliceOrEmpty();
            const exp_namespace = exp.symbol.namespace.intoSliceOrEmpty();
            const exp_version = Version.initC(exp.symbol.version);
            self.exports[self.export_index] = sym;
            self.inner.addSymbol(exp_name, exp_namespace, .{
                .symbol = sym,
                .version = exp_version,
                .dtor = dtor,
            }) catch |err| {
                self.ret = err;
                self.export_index += 1;
                return .next;
            };
        }
        self.ret = @ptrCast(self.instance);
        return .ret;
    }

    pub fn __state4(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        self.inner.unlock();
        modules.mutex.unlock();
        defer {
            modules.mutex.lock();
            _ = self.instance_handle.lock();
        }

        // TODO
        _ = waker;
        const exports = self.@"export".exports.intoSliceOrEmpty();
        while (self.export_index > 0) : (self.export_index -= 1) {
            _ = exports;
            @panic("TODO");
        }
        return .ret;
    }
});

pub const StartInstanceOp = FSMFuture(struct {
    inner: *Inner,
    @"export": *const pub_modules.Export,
    instance: *pub_modules.OpaqueInstance,
    start_event: pub_modules.exports.events.Start = undefined,
    future: pub_tasks.OpaqueFuture(pub_tasks.Fallible(void)) = undefined,
    ret: pub_context.Error!void = undefined,

    pub const __no_abort = true;

    pub fn __set_err(self: *@This(), trace: ?*std.builtin.StackTrace, err: pub_context.Error) void {
        if (trace) |tr| tracing.logStackTrace(@src(), tr.*);
        self.ret = err;
    }

    pub fn __ret(self: *@This()) pub_context.Error!void {
        return self.ret;
    }

    pub fn init(
        inner: *Inner,
    ) StartInstanceOp {
        std.debug.assert(!inner.isDetached());
        std.debug.assert(inner.state == .init);
        return StartInstanceOp.init(.{
            .inner = inner,
            .@"export" = inner.@"export".?,
            .instance = @ptrCast(inner.instance.?),
        });
    }

    pub fn __state0(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        _ = waker;
        const start_event = self.@"export".eventStart();
        if (start_event.poll != null) {
            self.inner.unlock();
            modules.mutex.unlock();
            self.start_event = start_event;
            return .next;
        }
        self.inner.state = .started;
        return .ret;
    }

    pub fn __state1(self: *@This(), waker: pub_tasks.Waker) pub_context.Error!pub_tasks.FSMOp {
        var result: AnyResult = undefined;
        const poll = self.start_event.poll.?;
        if (!poll(self.instance, waker, &result)) return .yield;
        modules.mutex.lock();
        self.inner.mutex.lock();

        if (result.isErr()) {
            context.setResult(result);
            return error.OperationFailed;
        }
        self.inner.state = .started;
        return .ret;
    }
});

// ----------------------------------------------------
// Instance VTable
// ----------------------------------------------------

const InstanceVTableImpl = struct {
    fn ref(ctx: *pub_modules.OpaqueInstance.Inner) callconv(.c) void {
        const self = fromInstancePtr(@ptrCast(ctx));
        const inner = self.lock();
        defer inner.unlock();
        inner.refStrong() catch unreachable;
    }
    fn unref(ctx: *pub_modules.OpaqueInstance.Inner) callconv(.c) void {
        const self = fromInstancePtr(@ptrCast(ctx));
        const inner = self.lock();
        defer inner.unlock();
        inner.unrefStrong();
    }
    fn queryNamespace(
        ctx: *pub_modules.OpaqueInstance.Inner,
        ns: SliceConst(u8),
        dependency: *pub_modules.Dependency,
    ) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        tracing.logTrace(
            @src(),
            "querying namespace info, instance='{s}', namespace='{s}'",
            .{ ctx.handle.name(), ns.intoSliceOrEmpty() },
        );

        const inner = self.lock();
        defer inner.unlock();

        if (inner.getNamespace(ns.intoSliceOrEmpty())) |info| {
            dependency.* = if (info.* == .static) .static else .dynamic;
        } else {
            dependency.* = .none;
        }
        return .ok;
    }
    fn addNamespace(ctx: *pub_modules.OpaqueInstance.Inner, ns: SliceConst(u8)) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        self.addNamespace(ns.intoSliceOrEmpty()) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn removeNamespace(ctx: *pub_modules.OpaqueInstance.Inner, ns: SliceConst(u8)) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        self.removeNamespace(ns.intoSliceOrEmpty()) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn queryDependency(
        ctx: *pub_modules.OpaqueInstance.Inner,
        handle: *pub_modules.Handle,
        dependency: *pub_modules.Dependency,
    ) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        tracing.logTrace(
            @src(),
            "querying dependency info, instance='{s}', other='{s}'",
            .{ ctx.handle.name(), handle.name() },
        );

        const inner = self.lock();
        defer inner.unlock();

        if (inner.getDependency(handle.name())) |x| {
            dependency.* = if (x.type == .static) .static else .dynamic;
        } else {
            dependency.* = .none;
        }
        return .ok;
    }
    fn addDependency(ctx: *pub_modules.OpaqueInstance.Inner, handle: *pub_modules.Handle) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        self.addDependency(handle) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn removeDependency(ctx: *pub_modules.OpaqueInstance.Inner, handle: *pub_modules.Handle) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        self.removeDependency(handle) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn loadSymbol(
        ctx: *pub_modules.OpaqueInstance.Inner,
        symbol: pub_modules.SymbolId,
        value: **const anyopaque,
    ) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        value.* = self.loadSymbol(
            symbol.name.intoSliceOrEmpty(),
            symbol.namespace.intoSliceOrEmpty(),
            .initC(symbol.version),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn readParameter(
        ctx: *pub_modules.OpaqueInstance.Inner,
        tag: pub_modules.ParamTag,
        module: SliceConst(u8),
        parameter: SliceConst(u8),
        value: *anyopaque,
    ) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        self.readParameter(
            value,
            tag,
            module.intoSliceOrEmpty(),
            parameter.intoSliceOrEmpty(),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
    fn writeParameter(
        ctx: *pub_modules.OpaqueInstance.Inner,
        tag: pub_modules.ParamTag,
        module: SliceConst(u8),
        parameter: SliceConst(u8),
        value: *const anyopaque,
    ) callconv(.c) pub_context.Status {
        const self = fromInstancePtr(@ptrCast(ctx));
        self.writeParameter(
            value,
            tag,
            module.intoSliceOrEmpty(),
            parameter.intoSliceOrEmpty(),
        ) catch |e| {
            if (@errorReturnTrace()) |tr| tracing.logStackTrace(@src(), tr.*);
            context.setResult(.initErr(.initError(e)));
            return .err;
        };
        return .ok;
    }
};

const instance_vtable = pub_modules.OpaqueInstance.Inner.VTable{
    .ref = &InstanceVTableImpl.ref,
    .unref = &InstanceVTableImpl.unref,
    .query_namespace = &InstanceVTableImpl.queryNamespace,
    .add_namespace = &InstanceVTableImpl.addNamespace,
    .remove_namespace = &InstanceVTableImpl.removeNamespace,
    .query_dependency = &InstanceVTableImpl.queryDependency,
    .add_dependency = &InstanceVTableImpl.addDependency,
    .remove_dependency = &InstanceVTableImpl.removeDependency,
    .load_symbol = &InstanceVTableImpl.loadSymbol,
    .read_parameter = &InstanceVTableImpl.readParameter,
    .write_parameter = InstanceVTableImpl.writeParameter,
};
