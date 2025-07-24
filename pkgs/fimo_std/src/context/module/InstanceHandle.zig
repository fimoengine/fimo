const std = @import("std");
const Allocator = std.mem.Allocator;
const ArenaAllocator = std.heap.ArenaAllocator;
const Mutex = std.Thread.Mutex;

const c = @import("c");

const AnyError = @import("../../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const Context = @import("../../context.zig");
const pub_modules = @import("../../modules.zig");
const PathBufferUnmanaged = @import("../../path.zig").PathBufferUnmanaged;
const PathError = @import("../../path.zig").PathError;
const pub_tasks = @import("../../tasks.zig");
const EnqueuedFuture = pub_tasks.EnqueuedFuture;
const FSMFuture = pub_tasks.FSMFuture;
const Fallible = pub_tasks.Fallible;
const Version = @import("../../Version.zig");
const Async = @import("../async.zig");
const RefCount = @import("../RefCount.zig");
const LoadingSet = @import("LoadingSet.zig");
const ModuleHandle = @import("ModuleHandle.zig");
const SymbolRef = @import("SymbolRef.zig");
const System = @import("System.zig");

const Self = @This();

sys: *System,
inner: Inner,
type: InstanceType,
info: pub_modules.Info,
ref_count: RefCount = .{},

pub const InstanceType = enum { regular, pseudo };

pub const DependencyType = enum { static, dynamic };

pub const InstanceDependency = struct {
    instance: *const Self,
    type: DependencyType,
};

pub const Symbol = struct {
    version: Version,
    symbol: *const anyopaque,
    dtor: ?*const fn (
        ctx: *const pub_modules.OpaqueInstance,
        symbol: *anyopaque,
    ) callconv(.c) void,

    fn destroySymbol(self: *const Symbol, ctx: *const pub_modules.OpaqueInstance) void {
        if (self.dtor) |dtor| {
            dtor(ctx, @constCast(self.symbol));
        }
    }
};

pub const ParameterError = error{
    NotPermitted,
    NotADependency,
    InvalidParameterType,
    NotFound,
};

pub const InstanceHandleError = error{
    Detached,
    NotFound,
    InvalidParameterType,
} || PathError || Allocator.Error;

pub const Parameter = struct {
    inner: Data,
    owner: *Self,
    read_group: pub_modules.ParameterAccessGroup,
    write_group: pub_modules.ParameterAccessGroup,
    proxy: pub_modules.OpaqueParameter,
    read_fn: *const fn (
        data: pub_modules.OpaqueParameterData,
        value: *anyopaque,
    ) callconv(.c) void,
    write_fn: *const fn (
        data: pub_modules.OpaqueParameterData,
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

        fn @"type"(self: *const @This()) pub_modules.ParameterType {
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
                .u8 => |*v| @as(*u8, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
                .u16 => |*v| @as(*u16, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
                .u32 => |*v| @as(*u32, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
                .u64 => |*v| @as(*u64, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
                .i8 => |*v| @as(*i8, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
                .i16 => |*v| @as(*i16, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
                .i32 => |*v| @as(*i32, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
                .i64 => |*v| @as(*i64, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst),
            }
        }

        fn writeFrom(self: *@This(), ptr: *const anyopaque) void {
            switch (self.value) {
                .u8 => |*v| v.store(@as(*const u8, @alignCast(@ptrCast(ptr))).*, .seq_cst),
                .u16 => |*v| v.store(@as(*const u16, @alignCast(@ptrCast(ptr))).*, .seq_cst),
                .u32 => |*v| v.store(@as(*const u32, @alignCast(@ptrCast(ptr))).*, .seq_cst),
                .u64 => |*v| v.store(@as(*const u64, @alignCast(@ptrCast(ptr))).*, .seq_cst),
                .i8 => |*v| v.store(@as(*const i8, @alignCast(@ptrCast(ptr))).*, .seq_cst),
                .i16 => |*v| v.store(@as(*const i16, @alignCast(@ptrCast(ptr))).*, .seq_cst),
                .i32 => |*v| v.store(@as(*const i32, @alignCast(@ptrCast(ptr))).*, .seq_cst),
                .i64 => |*v| v.store(@as(*const i64, @alignCast(@ptrCast(ptr))).*, .seq_cst),
            }
        }

        fn asProxyParameter(self: *@This()) pub_modules.OpaqueParameterData {
            return .{
                .data = self,
                .vtable = &param_data_vtable,
            };
        }
    };

    fn init(
        owner: *Self,
        inner: Data,
        read_group: pub_modules.ParameterAccessGroup,
        write_group: pub_modules.ParameterAccessGroup,
        read_fn: ?*const fn (
            data: pub_modules.OpaqueParameterData,
            value: *anyopaque,
        ) callconv(.c) void,
        write_fn: ?*const fn (
            data: pub_modules.OpaqueParameterData,
            value: *const anyopaque,
        ) callconv(.c) void,
    ) Allocator.Error!*Parameter {
        const Wrapper = struct {
            fn read(this: pub_modules.OpaqueParameterData, value: *anyopaque) callconv(.c) void {
                const self: *Data = @alignCast(@ptrCast(this.data));
                self.readTo(value);
            }
            fn write(this: pub_modules.OpaqueParameterData, value: *const anyopaque) callconv(.c) void {
                const self: *Data = @alignCast(@ptrCast(this.data));
                self.writeFrom(value);
            }
        };

        const p = try owner.inner.arena.allocator().create(Parameter);
        p.* = .{
            .inner = inner,
            .owner = owner,
            .read_group = read_group,
            .write_group = write_group,
            .proxy = .{
                .vtable = param_vtable,
            },
            .read_fn = read_fn orelse &Wrapper.read,
            .write_fn = write_fn orelse &Wrapper.write,
        };
        return p;
    }

    pub fn checkType(
        self: *const Parameter,
        ty: pub_modules.ParameterType,
    ) ParameterError!void {
        if (!(self.inner.type() == ty)) return error.InvalidParameterType;
    }

    pub fn checkReadPublic(self: *const Parameter) ParameterError!void {
        if (!(self.read_group == .public)) return error.NotPermitted;
    }

    pub fn checkWritePublic(self: *const Parameter) ParameterError!void {
        if (!(self.write_group == .public)) return error.NotPermitted;
    }

    fn checkReadDependency(self: *const Parameter, reader: *const Inner) ParameterError!void {
        const min_permission = pub_modules.ParameterAccessGroup.dependency;
        if (@intFromEnum(self.read_group) > @intFromEnum(min_permission)) return error.NotPermitted;
        const owner_name = std.mem.span(self.owner.info.name);
        if (!reader.dependencies.contains(owner_name)) return error.NotADependency;
    }

    fn checkWriteDependency(self: *const Parameter, writer: *const Inner) ParameterError!void {
        const min_permission = pub_modules.ParameterAccessGroup.dependency;
        if (@intFromEnum(self.write_group) > @intFromEnum(min_permission)) return error.NotPermitted;
        const owner_name = std.mem.span(self.owner.info.name);
        if (!writer.dependencies.contains(owner_name)) return error.NotADependency;
    }

    pub fn @"type"(self: *const Parameter) pub_modules.ParameterType {
        return self.inner.type();
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
    instance: ?*const pub_modules.OpaqueInstance = null,
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

    pub fn canUnload(self: *const Inner) bool {
        std.debug.assert(!self.isDetached());
        return self.strong_count == 0 and self.dependents_count == 0;
    }

    pub fn enqueueUnload(self: *Inner) !void {
        if (self.unload_requested or self.isDetached()) return;
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

    pub fn refStrong(self: *Inner) InstanceHandleError!void {
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
        if (!sym.version.isCompatibleWith(version)) return null;
        return sym;
    }

    fn addSymbol(self: *Inner, name: []const u8, namespace: []const u8, sym: Symbol) InstanceHandleError!void {
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

    fn addParameter(self: *Inner, name: []const u8, param: *Parameter) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        const n = try self.cacheString(name);
        try self.parameters.put(self.arena.allocator(), n, param);
    }

    pub fn getNamespace(self: *Inner, name: []const u8) ?*DependencyType {
        if (self.isDetached()) return null;
        return self.namespaces.getPtr(name);
    }

    pub fn addNamespace(self: *Inner, name: []const u8, @"type": DependencyType) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        const n = try self.cacheString(name);
        try self.namespaces.put(self.arena.allocator(), n, @"type");
    }

    pub fn removeNamespace(self: *Inner, name: []const u8) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        if (!self.namespaces.swapRemove(name)) return error.NotFound;
    }

    pub fn getDependency(self: *Inner, name: []const u8) ?*InstanceDependency {
        if (self.isDetached()) return null;
        return self.dependencies.getPtr(name);
    }

    pub fn addDependency(self: *Inner, other: *Inner, @"type": DependencyType) InstanceHandleError!void {
        std.debug.assert(self != other);
        if (self.isDetached() or other.isDetached()) return error.Detached;

        const handle = Self.fromInnerPtr(other);
        const name = std.mem.span(handle.info.name);
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

    pub fn removeDependency(self: *Inner, other: *Inner) InstanceHandleError!void {
        std.debug.assert(self != other);
        if (self.isDetached()) return error.Detached;
        std.debug.assert(!other.isDetached());

        const handle = Self.fromInnerPtr(other);
        const name = std.mem.span(handle.info.name);
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

    pub fn start(self: *Inner, sys: *System, err: *?AnyError) StartInstanceOp {
        return StartInstanceOp.Data.init(self, sys, err);
    }

    pub fn stop(self: *Inner, sys: *System) void {
        std.debug.assert(!self.isDetached());
        std.debug.assert(self.state == .started);
        self.is_detached = true;
        if (self.@"export") |@"export"| {
            if (@"export".getStopEventModifier()) |event| {
                self.unlock();
                sys.mutex.unlock();
                event.on_event(self.instance.?);
                sys.mutex.lock();
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
                if (exp.getInstanceStateModifier()) |state|
                    state.deinit(instance, @constCast(instance.state_));
            }
            exp.deinit();
        }

        for (self.symbols.values()) |sym| sym.destroySymbol(instance);

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
    sys: *System,
    name: []const u8,
    description: ?[]const u8,
    author: ?[]const u8,
    license: ?[]const u8,
    module_path: ?[]const u8,
    handle: *ModuleHandle,
    @"export": ?*const pub_modules.Export,
    @"type": InstanceType,
) InstanceHandleError!*Self {
    var arena = ArenaAllocator.init(sys.allocator);
    errdefer arena.deinit();
    const allocator = arena.allocator();

    const self = try allocator.create(Self);
    self.* = .{
        .sys = sys,
        .inner = .{
            .arena = undefined,
            .handle = handle,
            .@"export" = @"export",
        },
        .type = @"type",
        .info = .{
            .name = (try allocator.dupeZ(u8, name)).ptr,
            .description = if (description) |str| (try allocator.dupeZ(u8, str)).ptr else null,
            .author = if (author) |str| (try allocator.dupeZ(u8, str)).ptr else null,
            .license = if (license) |str| (try allocator.dupeZ(u8, str)).ptr else null,
            .module_path = if (module_path) |str| (try allocator.dupeZ(u8, str)).ptr else null,
            .vtable = info_vtable,
        },
    };
    // Move the new state of the arena into the allocated handle.
    self.inner.arena = arena;

    return self;
}

pub fn initPseudoInstance(sys: *System, name: []const u8) !*pub_modules.PseudoInstance {
    const iterator = &pub_modules.exports.ExportIter.fimo_impl_module_export_iterator;
    const handle = try ModuleHandle.initLocal(sys.allocator, iterator, iterator);
    errdefer handle.unref();

    const instance_handle = try Self.init(
        sys,
        name,
        null,
        null,
        null,
        null,
        handle,
        null,
        .pseudo,
    );
    errdefer instance_handle.unref();

    const instance = try instance_handle.inner.arena.allocator().create(pub_modules.PseudoInstance);
    comptime {
        std.debug.assert(@sizeOf(pub_modules.PseudoInstance) == @sizeOf(pub_modules.OpaqueInstance));
        std.debug.assert(@alignOf(pub_modules.PseudoInstance) == @alignOf(pub_modules.OpaqueInstance));
        std.debug.assert(@offsetOf(pub_modules.PseudoInstance, "instance") == 0);
    }
    instance.* = .{
        .instance = .{
            .vtable = &instance_vtable,
            .parameters_ = null,
            .resources_ = null,
            .imports_ = null,
            .exports_ = null,
            .info = &instance_handle.info,
            .handle = &Context.handle,
            .state_ = null,
        },
    };
    instance_handle.inner.state = .started;
    instance_handle.inner.instance = &instance.instance;
    return instance;
}

pub fn fromInstancePtr(instance: *const pub_modules.OpaqueInstance) *const Self {
    std.debug.assert(instance.vtable == &instance_vtable);
    return fromInfoPtr(instance.info);
}

pub fn fromInfoPtr(info: *const pub_modules.Info) *const Self {
    return @fieldParentPtr("info", @constCast(info));
}

pub fn fromInnerPtr(inner: *Inner) *const Self {
    return @fieldParentPtr("inner", inner);
}

fn logTrace(self: *const Self, comptime fmt: []const u8, args: anytype, location: std.builtin.SourceLocation) void {
    self.sys.logTrace(fmt, args, location);
}

fn ref(self: *const Self) void {
    const this: *Self = @constCast(self);
    this.ref_count.ref();
}

fn unref(self: *const Self) void {
    const this: *Self = @constCast(self);
    if (this.ref_count.unref() == .noop) return;

    const inner = this.lock();
    if (!inner.isDetached()) inner.detach();
    inner.arena.deinit();
}

pub fn lock(self: *const Self) *Inner {
    const this: *Self = @constCast(self);
    this.inner.mutex.lock();
    return &this.inner;
}

fn addNamespace(self: *const Self, namespace: []const u8) !void {
    self.logTrace(
        "adding namespace to instance, instance='{s}', namespace='{s}'",
        .{ self.info.name, namespace },
        @src(),
    );

    if (std.mem.eql(u8, namespace, System.global_namespace)) return error.NotPermitted;

    self.sys.lock();
    defer self.sys.unlock();

    const inner = self.lock();
    defer inner.unlock();

    if (inner.getNamespace(namespace) != null) return error.Duplicate;

    try self.sys.refNamespace(namespace);
    errdefer self.sys.unrefNamespace(namespace);
    try inner.addNamespace(namespace, .dynamic);
}

fn removeNamespace(self: *const Self, namespace: []const u8) !void {
    self.logTrace(
        "removing namespace from instance, instance='{s}', namespace='{s}'",
        .{ self.info.name, namespace },
        @src(),
    );

    if (std.mem.eql(u8, namespace, System.global_namespace)) return error.NotPermitted;

    self.sys.lock();
    defer self.sys.unlock();

    const inner = self.lock();
    defer inner.unlock();

    const ns_info = inner.getNamespace(namespace) orelse return error.NotFound;
    if (ns_info.* == .static) return error.NotPermitted;

    try inner.removeNamespace(namespace);
    self.sys.unrefNamespace(namespace);
}

fn addDependency(self: *const Self, info: *const pub_modules.Info) !void {
    self.logTrace(
        "adding dependency to instance, instance='{s}', other='{s}'",
        .{ self.info.name, info.name },
        @src(),
    );

    const info_handle = Self.fromInfoPtr(info);
    if (self == info_handle) return error.CyclicDependency;

    self.sys.lock();
    defer self.sys.unlock();

    const inner = self.lock();
    defer inner.unlock();

    const info_inner = info_handle.lock();
    defer info_inner.unlock();

    try self.sys.linkInstances(inner, info_inner);
}

fn removeDependency(self: *const Self, info: *const pub_modules.Info) !void {
    self.logTrace(
        "removing dependency from instance, instance='{s}', other='{s}'",
        .{ self.info.name, info.name },
        @src(),
    );

    const info_handle = Self.fromInfoPtr(info);
    if (self == info_handle) return error.NotADependency;

    self.sys.lock();
    defer self.sys.unlock();

    const inner = self.lock();
    defer inner.unlock();

    const info_inner = info_handle.lock();
    defer info_inner.unlock();

    try self.sys.unlinkInstances(inner, info_inner);
}

fn loadSymbol(self: *const Self, name: []const u8, namespace: []const u8, version: Version) !*const anyopaque {
    self.logTrace(
        "loading symbol, instance='{s}', name='{s}', namespace='{s}', version='{f}'",
        .{ self.info.name, name, namespace, version },
        @src(),
    );
    self.sys.lock();
    defer self.sys.unlock();
    const symbol_ref = self.sys.getSymbolCompatible(
        name,
        namespace,
        version,
    ) orelse return error.NotFound;

    const inner = self.lock();
    defer inner.unlock();

    if (inner.getDependency(symbol_ref.owner) == null) return error.NotADependency;
    if (inner.getNamespace(namespace) == null and
        !std.mem.eql(u8, namespace, System.global_namespace))
    {
        return error.NotADependency;
    }

    const owner_instance = self.sys.getInstance(symbol_ref.owner) orelse return error.NotFound;
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

fn readParameter(
    self: *const Self,
    value: *anyopaque,
    @"type": pub_modules.ParameterType,
    module: []const u8,
    parameter: []const u8,
) ParameterError!void {
    self.logTrace(
        "reading dependency parameter, reader='{s}', value='{*}', type='{s}', module='{s}', parameter='{s}'",
        .{ self.info.name, value, @tagName(@"type"), module, parameter },
        @src(),
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

fn writeParameter(
    self: *const Self,
    value: *const anyopaque,
    @"type": pub_modules.ParameterType,
    module: []const u8,
    parameter: []const u8,
) ParameterError!void {
    self.logTrace(
        "writing dependency parameter, writer='{s}', value='{*}', type='{s}', module='{s}', parameter='{s}'",
        .{ self.info.name, value, @tagName(@"type"), module, parameter },
        @src(),
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
    fn @"type"(data: *const pub_modules.OpaqueParameter) callconv(.c) pub_modules.ParameterType {
        const self: *const Parameter = @fieldParentPtr("proxy", data);
        return self.type();
    }
    fn read(data: *const pub_modules.OpaqueParameter, value: *anyopaque) callconv(.c) void {
        const self: *const Parameter = @fieldParentPtr("proxy", data);
        return self.readTo(value);
    }
    fn write(data: *pub_modules.OpaqueParameter, value: *const anyopaque) callconv(.c) void {
        const self: *Parameter = @fieldParentPtr("proxy", data);
        return self.writeFrom(value);
    }
};

const param_vtable = pub_modules.OpaqueParameter.VTable{
    .type = &ParamVTableImpl.type,
    .read = &ParamVTableImpl.read,
    .write = &ParamVTableImpl.write,
};

// ----------------------------------------------------
// ParameterData VTable
// ----------------------------------------------------

const ParamDataVTableImpl = struct {
    fn @"type"(data: *anyopaque) callconv(.c) pub_modules.ParameterType {
        const self: *Parameter.Data = @alignCast(@ptrCast(data));
        return self.type();
    }
    fn read(data: *anyopaque, value: *anyopaque) callconv(.c) void {
        const self: *Parameter.Data = @alignCast(@ptrCast(data));
        return self.readTo(value);
    }
    fn write(data: *anyopaque, value: *const anyopaque) callconv(.c) void {
        const self: *Parameter.Data = @alignCast(@ptrCast(data));
        return self.writeFrom(value);
    }
};

const param_data_vtable = pub_modules.OpaqueParameterData.VTable{
    .type = &ParamDataVTableImpl.type,
    .read = &ParamDataVTableImpl.read,
    .write = &ParamDataVTableImpl.write,
};

// ----------------------------------------------------
// Info Futures
// ----------------------------------------------------

const EnqueueUnloadOp = FSMFuture(struct {
    handle: *const Self,
    ret: void = undefined,

    pub const __no_abort = true;

    fn init(handle: *const Self) !void {
        handle.sys.logTrace(
            "enqueueing unload of instance, instance='{s}'",
            .{handle.info.name},
            @src(),
        );
        handle.ref();

        const context = handle.sys.asContext();
        const f = EnqueueUnloadOp.init(@This(){
            .handle = handle,
        }).intoFuture();
        var enqueued = try Async.Task.initFuture(
            @TypeOf(f),
            &context.async.sys,
            &f,
        );

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
        self.handle.logTrace(
            "attempting to unload instance, instance=`{s}`",
            .{self.handle.info.name},
            @src(),
        );

        const inner = self.handle.lock();
        defer inner.unlock();
        switch (inner.checkAsyncUnload(waker)) {
            .noop => {
                self.handle.logTrace(
                    "skipping unload, already unloaded, instance=`{s}`",
                    .{self.handle.info.name},
                    @src(),
                );
                self.ret = {};
                return .ret;
            },
            .wait => {
                self.handle.logTrace(
                    "unload blocked, instance=`{s}`",
                    .{self.handle.info.name},
                    @src(),
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

        const sys = self.handle.sys;
        sys.lock();
        defer sys.unlock();

        self.handle.logTrace(
            "unloading instance, instance=`{s}`",
            .{self.handle.info.name},
            @src(),
        );

        const inner = self.handle.lock();
        sys.removeInstance(inner) catch |err| @panic(@errorName(err));
        inner.stop(sys);
        inner.deinit();
        self.ret = {};
    }
});

// ----------------------------------------------------
// Info VTable
// ----------------------------------------------------

const InfoVTableImpl = struct {
    fn ref(info: *const pub_modules.Info) callconv(.c) void {
        const x = Self.fromInfoPtr(info);
        x.ref();
    }
    fn unref(info: *const pub_modules.Info) callconv(.c) void {
        const x = Self.fromInfoPtr(info);
        x.unref();
    }
    fn markUnloadable(info: *const pub_modules.Info) callconv(.c) void {
        const x = Self.fromInfoPtr(info);
        const inner = x.lock();
        defer inner.unlock();
        inner.enqueueUnload() catch |e| @panic(@errorName(e));
    }
    fn isLoaded(info: *const pub_modules.Info) callconv(.c) bool {
        const x = Self.fromInfoPtr(info);
        const inner = x.lock();
        defer inner.unlock();
        return !inner.isDetached();
    }
    fn tryRefInstanceStrong(info: *const pub_modules.Info) callconv(.c) bool {
        const x = Self.fromInfoPtr(info);
        const inner = x.lock();
        defer inner.unlock();
        inner.refStrong() catch return false;
        return true;
    }
    fn unrefInstanceStrong(info: *const pub_modules.Info) callconv(.c) void {
        const x = Self.fromInfoPtr(info);
        const inner = x.lock();
        defer inner.unlock();
        inner.unrefStrong();
    }
};

const info_vtable = pub_modules.Info.VTable{
    .ref = &InfoVTableImpl.ref,
    .unref = &InfoVTableImpl.unref,
    .mark_unloadable = &InfoVTableImpl.markUnloadable,
    .is_loaded = &InfoVTableImpl.isLoaded,
    .try_ref_instance_strong = &InfoVTableImpl.tryRefInstanceStrong,
    .unref_instance_strong = &InfoVTableImpl.unrefInstanceStrong,
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
    ///     1. init static exports
    ///     2. init dynamic exports
    ///     3. goto 3
    /// 3:
    ///     1. if (no dynamic symbol left) return
    ///     2. init export future
    /// 4:
    ///     1. wait for export future
    ///     2. goto 3
    sys: *System,
    set: pub_modules.LoadingSet,
    @"export": *const pub_modules.Export,
    handle: *ModuleHandle,
    err: *?AnyError,
    instance_handle: *Self = undefined,
    inner: *Inner = undefined,
    instance: *pub_modules.OpaqueInstance = undefined,
    state_future: EnqueuedFuture(Fallible(?*anyopaque)) = undefined,
    dyn_export_index: usize = 0,
    exports: []*const anyopaque = undefined,
    dyn_export_future: EnqueuedFuture(Fallible(*anyopaque)) = undefined,
    ret: Error!*pub_modules.OpaqueInstance = undefined,

    pub const Error = (InstanceHandleError || AnyError.Error);
    pub const __no_abort = true;

    pub fn __set_err(self: *@This(), trace: ?*std.builtin.StackTrace, err: Error) void {
        if (trace) |tr| self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
        self.ret = err;
    }

    pub fn __ret(self: *@This()) Error!*pub_modules.OpaqueInstance {
        return self.ret;
    }

    pub fn __state0(self: *@This(), waker: pub_tasks.Waker) Error!pub_tasks.FSMOpExt(@This()) {
        _ = waker;

        const instance_handle = try Self.init(
            self.sys,
            self.@"export".getName(),
            self.@"export".getDescription(),
            self.@"export".getAuthor(),
            self.@"export".getLicense(),
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

        const instance = try instance_handle.inner.arena.allocator().create(pub_modules.OpaqueInstance);
        instance.* = .{
            .vtable = &instance_vtable,
            .parameters_ = null,
            .resources_ = null,
            .imports_ = null,
            .exports_ = null,
            .info = &instance_handle.info,
            .handle = &Context.handle,
            .state_ = null,
        };
        inner.instance = instance;
        self.instance = instance;
        const allocator = inner.arena.allocator();

        // Init parameters.
        const exp_parameters = self.@"export".getParameters();
        const parameters = try allocator.alloc(*pub_modules.OpaqueParameter, exp_parameters.len);
        instance.parameters_ = @ptrCast(parameters.ptr);
        for (exp_parameters, parameters) |src, *dst| {
            const data = Parameter.Data{
                .value = switch (src.type) {
                    .u8 => .{ .u8 = std.atomic.Value(u8).init(src.default_value.u8) },
                    .u16 => .{ .u16 = std.atomic.Value(u16).init(src.default_value.u16) },
                    .u32 => .{ .u32 = std.atomic.Value(u32).init(src.default_value.u32) },
                    .u64 => .{ .u64 = std.atomic.Value(u64).init(src.default_value.u64) },
                    .i8 => .{ .i8 = std.atomic.Value(i8).init(src.default_value.i8) },
                    .i16 => .{ .i16 = std.atomic.Value(i16).init(src.default_value.i16) },
                    .i32 => .{ .i32 = std.atomic.Value(i32).init(src.default_value.i32) },
                    .i64 => .{ .i64 = std.atomic.Value(i64).init(src.default_value.i64) },
                    else => return error.InvalidParameterType,
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
            try inner.addParameter(std.mem.span(src.name), param);
            dst.* = &param.proxy;
        }

        // Init resources.
        const exp_resources = self.@"export".getResources();
        const resources = try allocator.alloc(c.FimoUTF8Path, exp_resources.len);
        instance.resources_ = @ptrCast(resources.ptr);
        for (exp_resources, resources) |src, *dst| {
            var buf = PathBufferUnmanaged{};
            try buf.pushPath(inner.arena.allocator(), self.handle.path.asPath());
            try buf.pushString(inner.arena.allocator(), std.mem.span(src.path));
            // Append a null-terminator to ensure that the path can be passed to c interfaces.
            try buf.buffer.append(inner.arena.allocator(), 0);
            dst.* = buf.asPath().intoC();
            dst.length -= 1;
        }

        // Init namespaces.
        for (self.@"export".getNamespaceImports()) |imp| {
            const name = std.mem.span(imp.name);
            if (self.sys.getNamespace(name) == null) return error.NotFound;
            try inner.addNamespace(name, .static);
        }

        // Init imports.
        const exp_imports = self.@"export".getSymbolImports();
        const imports = try allocator.alloc(*const anyopaque, exp_imports.len);
        instance.imports_ = @ptrCast(imports.ptr);
        for (exp_imports, imports) |src, *dst| {
            const src_name = std.mem.span(src.name);
            const src_namespace = std.mem.span(src.namespace);
            const src_version = Version.initC(src.version);
            const sym = self.sys.getSymbolCompatible(
                src_name,
                src_namespace,
                src_version,
            ) orelse return error.NotFound;

            const owner = self.sys.getInstance(sym.owner).?;
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
        if (self.@"export".getInstanceStateModifier()) |state| {
            inner.unlock();
            self.sys.mutex.unlock();
            self.state_future = state.init(instance, self.set);
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
        errdefer self.state_future.deinit();
        switch (self.state_future.poll(waker)) {
            .ready => |result| {
                self.sys.mutex.lock();
                _ = self.instance_handle.lock();
                self.instance.state_ = @ptrCast(try result.unwrap(self.err));
                self.state_future.deinit();
                self.inner.state = .init;
                return .next;
            },
            .pending => return .yield,
        }
    }

    pub fn __state2(self: *@This(), waker: pub_tasks.Waker) Error!void {
        _ = waker;
        const @"export" = self.@"export";
        const inner = self.inner;
        const allocator = inner.arena.allocator();

        // Init static exports.
        const exp_exports = @"export".getSymbolExports();
        const exp_dyn_exports = @"export".getDynamicSymbolExports();
        const exports = try allocator.alloc(*const anyopaque, exp_exports.len + exp_dyn_exports.len);
        self.exports = exports;
        self.instance.exports_ = @ptrCast(exports.ptr);
        for (exp_exports, exports[0..exp_exports.len]) |src, *dst| {
            const sym = src.symbol;
            const src_name = std.mem.span(src.name);
            const src_namespace = std.mem.span(src.namespace);
            const src_version = Version.initC(src.version);
            try inner.addSymbol(src_name, src_namespace, .{
                .symbol = sym,
                .version = src_version,
                .dtor = null,
            });
            dst.* = sym;
        }
    }

    pub fn __state3(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        _ = waker;
        // Check if there is another dynamic export.
        const exp_dyn_exports = self.@"export".getDynamicSymbolExports();
        if (self.dyn_export_index >= exp_dyn_exports.len) {
            self.ret = self.instance;
            return .ret;
        }

        // Initialize the future.
        const src = exp_dyn_exports[self.dyn_export_index];
        self.inner.unlock();
        self.sys.mutex.unlock();
        self.dyn_export_future = src.constructor(self.instance);
        return .next;
    }

    pub fn __unwind4(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        if (reason != .completed) self.dyn_export_future.deinit();
    }

    pub fn __state4(self: *@This(), waker: pub_tasks.Waker) Error!pub_tasks.FSMOpExt(@This()) {
        // Wait for the future and jump back to the loop in state 3.
        switch (self.dyn_export_future.poll(waker)) {
            .ready => |result| {
                self.sys.mutex.lock();
                _ = self.instance_handle.lock();
                const sym = try result.unwrap(self.err);

                const i = self.dyn_export_index;
                const src = self.@"export".getDynamicSymbolExports()[i];
                const dst = &self.exports[i];
                var skip_dtor = false;
                errdefer if (!skip_dtor) src.destructor(self.instance, sym);

                const src_name = std.mem.span(src.name);
                const src_namespace = std.mem.span(src.namespace);
                const src_version = Version.initC(src.version);
                try self.inner.addSymbol(src_name, src_namespace, .{
                    .symbol = sym,
                    .version = src_version,
                    .dtor = src.destructor,
                });
                skip_dtor = true;
                dst.* = sym;

                self.dyn_export_future.deinit();
                self.dyn_export_index += 1;
                return .{ .transition = 3 };
            },
            .pending => return .yield,
        }
    }
});

pub const StartInstanceOp = FSMFuture(struct {
    inner: *Inner,
    sys: *System,
    err: *?AnyError,
    @"export": *const pub_modules.Export,
    instance: *const pub_modules.OpaqueInstance,
    future: pub_tasks.EnqueuedFuture(pub_tasks.Fallible(void)) = undefined,
    ret: AnyError.Error!void = undefined,

    pub const __no_abort = true;

    pub fn __set_err(self: *@This(), trace: ?*std.builtin.StackTrace, err: AnyError.Error) void {
        if (trace) |tr| self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
        self.ret = err;
    }

    pub fn __ret(self: *@This()) AnyError.Error!void {
        return self.ret;
    }

    pub fn init(
        inner: *Inner,
        sys: *System,
        err: *?AnyError,
    ) StartInstanceOp {
        std.debug.assert(!inner.isDetached());
        std.debug.assert(inner.state == .init);
        return StartInstanceOp.init(.{
            .inner = inner,
            .sys = sys,
            .err = err,
            .@"export" = inner.@"export".?,
            .instance = inner.instance.?,
        });
    }

    pub fn __state0(self: *@This(), waker: pub_tasks.Waker) pub_tasks.FSMOp {
        _ = waker;
        if (self.@"export".getStartEventModifier()) |event| {
            self.inner.unlock();
            self.sys.mutex.unlock();
            self.future = event.on_event(self.instance);
            return .next;
        }
        self.inner.state = .started;
        return .ret;
    }

    pub fn __unwind1(self: *@This(), reason: pub_tasks.FSMUnwindReason) void {
        _ = reason;
        self.future.deinit();
    }

    pub fn __state1(self: *@This(), waker: pub_tasks.Waker) AnyError.Error!pub_tasks.FSMOp {
        switch (self.future.poll(waker)) {
            .ready => |result| {
                self.sys.mutex.lock();
                self.inner.mutex.lock();
                try result.unwrap(self.err);
                self.inner.state = .started;
                return .ret;
            },
            .pending => return .yield,
        }
    }
});

// ----------------------------------------------------
// Instance VTable
// ----------------------------------------------------

const InstanceVTableImpl = struct {
    fn ref(ctx: *const pub_modules.OpaqueInstance) callconv(.c) void {
        const x = Self.fromInstancePtr(ctx);
        const inner = x.lock();
        defer inner.unlock();
        inner.refStrong() catch unreachable;
    }
    fn unref(ctx: *const pub_modules.OpaqueInstance) callconv(.c) void {
        const x = Self.fromInstancePtr(ctx);
        const inner = x.lock();
        defer inner.unlock();
        inner.unrefStrong();
    }
    fn queryNamespace(
        ctx: *const pub_modules.OpaqueInstance,
        namespace: [*:0]const u8,
        has_dependency: *bool,
        is_static: *bool,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);
        const namespace_ = std.mem.span(namespace);
        self.logTrace(
            "querying namespace info, instance='{s}', namespace='{s}'",
            .{ ctx.info.name, namespace },
            @src(),
        );

        const inner = self.lock();
        defer inner.unlock();

        if (inner.getNamespace(namespace_)) |info| {
            has_dependency.* = true;
            is_static.* = info.* == .static;
        } else {
            has_dependency.* = false;
            is_static.* = false;
        }
        return AnyResult.ok;
    }
    fn addNamespace(
        ctx: *const pub_modules.OpaqueInstance,
        namespace: [*:0]const u8,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);
        const namespace_ = std.mem.span(namespace);

        self.addNamespace(namespace_) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn removeNamespace(
        ctx: *const pub_modules.OpaqueInstance,
        namespace: [*:0]const u8,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);
        const namespace_ = std.mem.span(namespace);

        self.removeNamespace(namespace_) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn queryDependency(
        ctx: *const pub_modules.OpaqueInstance,
        info: *const pub_modules.Info,
        has_dependency: *bool,
        is_static: *bool,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);
        const dependency = std.mem.span(info.name);
        self.logTrace(
            "querying dependency info, instance='{s}', other='{s}'",
            .{ ctx.info.name, dependency },
            @src(),
        );

        const inner = self.lock();
        defer inner.unlock();

        if (inner.getDependency(dependency)) |x| {
            has_dependency.* = true;
            is_static.* = x.type == .static;
        } else {
            has_dependency.* = false;
            is_static.* = false;
        }
        return AnyResult.ok;
    }
    fn addDependency(
        ctx: *const pub_modules.OpaqueInstance,
        info: *const pub_modules.Info,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);

        self.addDependency(info) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn removeDependency(
        ctx: *const pub_modules.OpaqueInstance,
        info: *const pub_modules.Info,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);

        self.removeDependency(info) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn loadSymbol(
        ctx: *const pub_modules.OpaqueInstance,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: Version.CVersion,
        symbol: **const anyopaque,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);
        const name_ = std.mem.span(name);
        const namespace_ = std.mem.span(namespace);
        const version_ = Version.initC(version);

        symbol.* = self.loadSymbol(name_, namespace_, version_) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn readParameter(
        ctx: *const pub_modules.OpaqueInstance,
        value: *anyopaque,
        @"type": pub_modules.ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);
        const module_ = std.mem.span(module);
        const parameter_ = std.mem.span(parameter);

        self.readParameter(value, @"type", module_, parameter_) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
    fn writeParameter(
        ctx: *const pub_modules.OpaqueInstance,
        value: *const anyopaque,
        @"type": pub_modules.ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) AnyResult {
        const self = Self.fromInstancePtr(ctx);
        const module_ = std.mem.span(module);
        const parameter_ = std.mem.span(parameter);

        self.writeParameter(value, @"type", module_, parameter_) catch |e| {
            if (@errorReturnTrace()) |tr|
                self.sys.asContext().tracing.emitStackTraceSimple(tr.*, @src());
            return AnyError.initError(e).intoResult();
        };
        return AnyResult.ok;
    }
};

const instance_vtable = pub_modules.OpaqueInstance.VTable{
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
