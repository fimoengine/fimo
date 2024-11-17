const std = @import("std");
const Allocator = std.mem.Allocator;
const Mutex = std.Thread.Mutex;

const c = @import("../../c.zig");
const heap = @import("../../heap.zig");
const AnyError = @import("../../AnyError.zig");
const Version = @import("../../Version.zig");
const PathBufferUnmanaged = @import("../../path.zig").PathBufferUnmanaged;
const PathError = @import("../../path.zig").PathError;

const RefCount = @import("../RefCount.zig");

const System = @import("System.zig");
const LoadingSet = @import("LoadingSet.zig");
const SymbolRef = @import("SymbolRef.zig");
const ModuleHandle = @import("ModuleHandle.zig");

const ProxyContext = @import("../proxy_context.zig");
const ProxyModule = @import("../proxy_context/module.zig");

const allocator = heap.fimo_allocator;
const Self = @This();

inner: Inner,
type: InstanceType,
info: ProxyModule.Info,
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
    dtor: ?*const fn (symbol: *anyopaque) callconv(.C) void,

    fn destroySymbol(self: *const Symbol) void {
        if (self.dtor) |dtor| {
            dtor(@constCast(self.symbol));
        }
    }
};

pub const ParameterError = error{
    NotPermitted,
    NotADependency,
    InvalidParameterType,
};

pub const InstanceHandleError = error{
    Detached,
    NotFound,
    InvalidParameterType,
} || PathError || Allocator.Error;

pub const Parameter = struct {
    pub const Data = struct {
        owner: *const ProxyModule.OpaqueInstance,
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

        pub fn checkOwner(
            self: *const Data,
            instance: *const ProxyModule.OpaqueInstance,
        ) ParameterError!void {
            if (self.owner != instance) return error.NotPermitted;
        }

        pub fn checkType(
            self: *const Data,
            @"type": ProxyModule.ParameterType,
        ) ParameterError!void {
            if (!(self.getType() == @"type")) return error.InvalidParameterType;
        }

        pub fn getType(self: *const Data) ProxyModule.ParameterType {
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

        pub fn readTo(
            self: *const Data,
            ptr: *anyopaque,
            @"type": *ProxyModule.ParameterType,
        ) void {
            switch (self.value) {
                .u8 => |*v| {
                    @as(*u8, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .u8;
                },
                .u16 => |*v| {
                    @as(*u16, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .u16;
                },
                .u32 => |*v| {
                    @as(*u32, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .u32;
                },
                .u64 => |*v| {
                    @as(*u64, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .u64;
                },
                .i8 => |*v| {
                    @as(*i8, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .i8;
                },
                .i16 => |*v| {
                    @as(*i16, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .i16;
                },
                .i32 => |*v| {
                    @as(*i32, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .i32;
                },
                .i64 => |*v| {
                    @as(*i64, @alignCast(@ptrCast(ptr))).* = v.load(.seq_cst);
                    @"type".* = .i64;
                },
            }
        }

        pub fn writeFrom(self: *Data, ptr: *const anyopaque) void {
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
    };

    const GetterFn = *const fn (
        ctx: *const ProxyModule.OpaqueInstance,
        value: *anyopaque,
        type: *ProxyModule.ParameterType,
        data: *const ProxyModule.OpaqueParameterData,
    ) callconv(.C) c.FimoResult;
    const SetterFn = *const fn (
        ctx: *const ProxyModule.OpaqueInstance,
        value: *const anyopaque,
        type: ProxyModule.ParameterType,
        data: *ProxyModule.OpaqueParameterData,
    ) callconv(.C) c.FimoResult;

    data: Data,
    read_group: ProxyModule.ParameterAccessGroup,
    write_group: ProxyModule.ParameterAccessGroup,
    getter: GetterFn,
    setter: SetterFn,

    fn init(
        data: Data,
        read_group: ProxyModule.ParameterAccessGroup,
        write_group: ProxyModule.ParameterAccessGroup,
        getter: GetterFn,
        setter: SetterFn,
    ) Allocator.Error!*Parameter {
        const p = try allocator.create(Parameter);
        p.* = .{
            .data = data,
            .read_group = read_group,
            .write_group = write_group,
            .getter = getter,
            .setter = setter,
        };
        return p;
    }

    fn deinit(self: *Parameter) void {
        allocator.destroy(self);
    }

    pub fn checkReadPublic(self: *const Parameter) ParameterError!void {
        if (!(self.read_group == .public)) return error.NotPermitted;
    }

    pub fn checkWritePublic(self: *const Parameter) ParameterError!void {
        if (!(self.write_group == .public)) return error.NotPermitted;
    }

    pub fn checkReadDependency(self: *const Parameter, reader: *const Inner) ParameterError!void {
        const min_permission = ProxyModule.ParameterAccessGroup.dependency;
        if (@intFromEnum(self.read_group) > @intFromEnum(min_permission)) return error.NotPermitted;
        const owner_name = std.mem.span(self.data.owner.info.name);
        if (!reader.dependencies.contains(owner_name)) return error.NotADependency;
    }

    pub fn checkWriteDependency(self: *const Parameter, writer: *const Inner) ParameterError!void {
        const min_permission = ProxyModule.ParameterAccessGroup.dependency;
        if (@intFromEnum(self.write_group) > @intFromEnum(min_permission)) return error.NotPermitted;
        const owner_name = std.mem.span(self.data.owner.info.name);
        if (!writer.dependencies.contains(owner_name)) return error.NotADependency;
    }

    pub fn checkReadPrivate(
        self: *const Parameter,
        reader: *const ProxyModule.OpaqueInstance,
    ) ParameterError!void {
        if (self.data.owner != reader) return error.NotPermitted;
    }

    pub fn checkWritePrivate(
        self: *const Parameter,
        writer: *const ProxyModule.OpaqueInstance,
    ) ParameterError!void {
        if (self.data.owner != writer) return error.NotPermitted;
    }

    pub fn readTo(
        self: *const Parameter,
        value: *anyopaque,
        @"type": *ProxyModule.ParameterType,
        err: *?AnyError,
    ) AnyError.Error!void {
        const result = self.getter(
            self.data.owner,
            value,
            @"type",
            @ptrCast(&self.data),
        );
        try AnyError.initChecked(err, result);
    }

    pub fn writeFrom(
        self: *Parameter,
        value: *const anyopaque,
        @"type": ProxyModule.ParameterType,
        err: *?AnyError,
    ) AnyError.Error!void {
        const result = self.setter(
            self.data.owner,
            value,
            @"type",
            @ptrCast(&self.data),
        );
        try AnyError.initChecked(err, result);
    }
};

pub const Inner = struct {
    mutex: Mutex = .{},
    lock_count: usize = 0,
    handle: ?*ModuleHandle = null,
    @"export": ?*const ProxyModule.Export = null,
    instance: ?*const ProxyModule.OpaqueInstance = null,
    symbols: std.ArrayHashMapUnmanaged(
        SymbolRef.Id,
        Symbol,
        SymbolRef.Id.HashContext,
        false,
    ) = .{},
    parameters: std.StringArrayHashMapUnmanaged(*Parameter) = .{},
    namespaces: std.StringArrayHashMapUnmanaged(DependencyType) = .{},
    dependencies: std.StringArrayHashMapUnmanaged(InstanceDependency) = .{},

    pub fn deinit(self: *Inner) ProxyContext {
        std.debug.assert(!self.isDetached());
        const handle = Self.fromInnerPtr(self);
        const ctx = self.detach(true);
        self.unlock();
        handle.unref(true);
        return ctx;
    }

    pub fn unlock(self: *Inner) void {
        self.mutex.unlock();
    }

    pub fn isDetached(self: *const Inner) bool {
        return self.handle == null;
    }

    pub fn canUnload(self: *const Inner) bool {
        return self.lock_count == 0;
    }

    pub fn preventUnload(self: *Inner) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        self.lock_count += 1;
    }

    pub fn allowUnload(self: *Inner) void {
        std.debug.assert(!self.isDetached());
        self.lock_count -= 1;
    }

    fn ref(self: *Inner) void {
        self.lock_count += 1;
    }

    fn unref(self: *Inner) void {
        self.lock_count -= 1;
    }

    pub fn getSymbol(self: *Inner, name: []const u8, namespace: []const u8, version: Version) ?*Symbol {
        if (self.isDetached()) return null;
        const sym = self.symbols.getPtr(.{
            .name = @constCast(name),
            .namespace = @constCast(namespace),
        }) orelse return null;
        if (!sym.version.isCompatibleWith(version)) return null;
        return sym;
    }

    fn addSymbol(self: *Inner, name: []const u8, namespace: []const u8, sym: Symbol) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        var key = try SymbolRef.Id.init(name, namespace);
        errdefer key.deinit();
        try self.symbols.put(allocator, key, sym);
    }

    pub fn getParameter(self: *Inner, name: []const u8) ?*Parameter {
        if (self.isDetached()) return null;
        return self.parameters.get(name);
    }

    fn addParameter(self: *Inner, name: []const u8, param: *Parameter) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        const n = try allocator.dupe(u8, name);
        errdefer allocator.free(n);
        try self.parameters.put(allocator, n, param);
    }

    pub fn getNamespace(self: *Inner, name: []const u8) ?*DependencyType {
        if (self.isDetached()) return null;
        return self.namespaces.getPtr(name);
    }

    pub fn addNamespace(self: *Inner, name: []const u8, @"type": DependencyType) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        const n = try allocator.dupe(u8, name);
        errdefer allocator.free(n);
        try self.namespaces.put(allocator, n, @"type");
    }

    pub fn removeNamespace(self: *Inner, name: []const u8) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        const dependency = self.namespaces.fetchSwapRemove(name) orelse return error.NotFound;
        allocator.free(@constCast(dependency.key));
    }

    pub fn getDependency(self: *Inner, name: []const u8) ?*InstanceDependency {
        if (self.isDetached()) return null;
        return self.dependencies.getPtr(name);
    }

    pub fn addDependency(self: *Inner, name: []const u8, dep: InstanceDependency) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        const n = try allocator.dupe(u8, name);
        errdefer allocator.free(n);
        try self.dependencies.put(allocator, n, dep);
    }

    pub fn removeDependency(self: *Inner, name: []const u8) InstanceHandleError!void {
        if (self.isDetached()) return error.Detached;
        const dependency = self.dependencies.fetchSwapRemove(name) orelse return error.NotFound;
        allocator.free(@constCast(dependency.key));
    }

    fn detach(self: *Inner, cleanup: bool) ProxyContext {
        std.debug.assert(!self.isDetached());
        std.debug.assert(self.canUnload());
        const instance = self.instance.?;
        const context = ProxyContext.initC(instance.ctx);

        // Set the handle to null, thereby hindering the modules ability to lock the handle.
        const handle = self.handle.?;
        self.handle = null;

        if (self.@"export") |exp| {
            if (exp.module_destructor) |dtor|
                dtor(instance, @constCast(instance.data));
            if (cleanup) exp.deinit();
        }

        if (instance.parameters) |p| {
            const parameters: [*:null]?*Parameter = @alignCast(@ptrCast(@constCast(p)));
            allocator.free(std.mem.span(parameters));
        }
        if (instance.resources) |res| {
            const resources_ptr: [*:null]?[*:0]u8 = @alignCast(@ptrCast(@constCast(res)));
            const resources = std.mem.span(resources_ptr);
            for (resources) |r| if (r) |r_| allocator.free(std.mem.span(r_));
            allocator.free(resources);
        }
        if (instance.imports) |imp| {
            const imports: [*:null]?*const anyopaque = @alignCast(@ptrCast(@constCast(imp)));
            allocator.free(std.mem.span(imports));
        }
        if (instance.exports) |exp| {
            const exports: [*:null]?*const anyopaque = @alignCast(@ptrCast(@constCast(exp)));
            allocator.free(std.mem.span(exports));
        }
        allocator.destroy(instance);

        var dependencies = self.dependencies.iterator();
        while (dependencies.next()) |dep| allocator.free(dep.key_ptr.*);
        self.dependencies.clearAndFree(allocator);

        var parameters = self.parameters.iterator();
        while (parameters.next()) |entry| {
            allocator.free(entry.key_ptr.*);
            entry.value_ptr.*.deinit();
        }
        self.parameters.clearAndFree(allocator);

        var namespaces_it = self.namespaces.iterator();
        while (namespaces_it.next()) |entry| allocator.free(entry.key_ptr.*);
        self.namespaces.clearAndFree(allocator);

        var symbols = self.symbols.iterator();
        while (symbols.next()) |sym| {
            sym.key_ptr.deinit();
            sym.value_ptr.destroySymbol();
        }
        self.symbols.clearAndFree(allocator);

        handle.unref();
        self.instance = null;
        self.@"export" = null;

        return context;
    }
};

fn init(
    name: []const u8,
    description: ?[]const u8,
    author: ?[]const u8,
    license: ?[]const u8,
    module_path: ?[]const u8,
    handle: *ModuleHandle,
    @"export": ?*const ProxyModule.Export,
    @"type": InstanceType,
) InstanceHandleError!*Self {
    const self = try allocator.create(Self);
    errdefer allocator.destroy(self);

    const FfiInfo = struct {
        fn ref(info: *const ProxyModule.Info) callconv(.C) void {
            const x = Self.fromInfoPtr(info);
            x.ref();
        }
        fn unref(info: *const ProxyModule.Info) callconv(.C) void {
            const x = Self.fromInfoPtr(info);
            x.unref(true);
        }
        fn isLoaded(info: *const ProxyModule.Info) callconv(.C) bool {
            const x = Self.fromInfoPtr(info);
            const inner = x.lock();
            defer inner.unlock();
            return !inner.isDetached();
        }
        fn preventUnload(info: *const ProxyModule.Info) callconv(.C) c.FimoResult {
            const x = Self.fromInfoPtr(info);
            const inner = x.lock();
            defer inner.unlock();
            inner.preventUnload() catch |err| return AnyError.initError(err).err;
            return AnyError.intoCResult(null);
        }
        fn allowUnload(info: *const ProxyModule.Info) callconv(.C) void {
            const x = Self.fromInfoPtr(info);
            const inner = x.lock();
            defer inner.unlock();
            inner.allowUnload();
        }
    };

    self.inner = .{
        .handle = handle,
        .@"export" = @"export",
    };
    self.type = @"type";
    self.info = .{
        .name = undefined,
        .description = undefined,
        .author = undefined,
        .license = undefined,
        .module_path = undefined,
        .acquire_fn = &FfiInfo.ref,
        .release_fn = &FfiInfo.unref,
        .is_loaded_fn = &FfiInfo.isLoaded,
        .lock_unload_fn = &FfiInfo.preventUnload,
        .unlock_unload_fn = &FfiInfo.allowUnload,
    };
    self.info.name = (try allocator.dupeZ(u8, name)).ptr;
    errdefer allocator.free(std.mem.span(self.info.name));
    self.info.description = if (description) |str| (try allocator.dupeZ(u8, str)).ptr else null;
    errdefer if (self.info.description) |str| allocator.free(std.mem.span(str));
    self.info.author = if (author) |str| (try allocator.dupeZ(u8, str)).ptr else null;
    errdefer if (self.info.author) |str| allocator.free(std.mem.span(str));
    self.info.license = if (license) |str| (try allocator.dupeZ(u8, str)).ptr else null;
    errdefer if (self.info.license) |str| allocator.free(std.mem.span(str));
    self.info.module_path = if (module_path) |str| (try allocator.dupeZ(u8, str)).ptr else null;
    errdefer if (self.info.module_path) |str| allocator.free(std.mem.span(str));

    return self;
}

pub fn initPseudoInstance(sys: *System, name: []const u8) !*ProxyModule.PseudoInstance {
    const iterator = &ProxyModule.Export.ExportIter.fimo_impl_module_export_iterator;
    const handle = try ModuleHandle.initLocal(iterator, iterator);
    errdefer handle.unref();

    const instance_handle = try Self.init(
        name,
        null,
        null,
        null,
        null,
        handle,
        null,
        .pseudo,
    );
    errdefer instance_handle.unref(true);

    sys.asContext().ref();
    errdefer sys.asContext().unref();

    const instance = try allocator.create(ProxyModule.PseudoInstance);
    comptime {
        std.debug.assert(@sizeOf(ProxyModule.PseudoInstance) == @sizeOf(ProxyModule.OpaqueInstance));
        std.debug.assert(@alignOf(ProxyModule.PseudoInstance) == @alignOf(ProxyModule.OpaqueInstance));
        std.debug.assert(@offsetOf(ProxyModule.PseudoInstance, "instance") == 0);
    }
    instance.* = .{
        .instance = .{
            .parameters = null,
            .resources = null,
            .imports = null,
            .exports = null,
            .info = &instance_handle.info,
            .ctx = sys.asContext().asProxy().intoC(),
            .data = null,
        },
    };
    instance_handle.inner.instance = &instance.instance;
    return instance;
}

pub fn initExportedInstance(
    sys: *System,
    set: *LoadingSet,
    @"export": *const ProxyModule.Export,
    handle: *ModuleHandle,
    err: *?AnyError,
) (InstanceHandleError || AnyError.Error)!*ProxyModule.OpaqueInstance {
    const instance_handle = try Self.init(
        @"export".getName(),
        @"export".getDescription(),
        @"export".getAuthor(),
        @"export".getLicense(),
        handle.path.raw,
        handle,
        @"export",
        .regular,
    );
    handle.ref();
    errdefer instance_handle.unref(false);

    const inner = instance_handle.lock();
    defer inner.unlock();

    sys.asContext().ref();
    errdefer sys.asContext().unref();

    const instance = try allocator.create(ProxyModule.OpaqueInstance);
    instance.* = .{
        .parameters = null,
        .resources = null,
        .imports = null,
        .exports = null,
        .info = &instance_handle.info,
        .ctx = sys.asContext().asProxy().intoC(),
        .data = null,
    };
    inner.instance = instance;

    // Init parameters.
    var parameters = std.ArrayListUnmanaged(?*Parameter){};
    errdefer parameters.deinit(allocator);
    for (@"export".getParameters()) |p| {
        const data = Parameter.Data{
            .owner = instance,
            .value = switch (p.type) {
                .u8 => .{ .u8 = std.atomic.Value(u8).init(p.default_value.u8) },
                .u16 => .{ .u16 = std.atomic.Value(u16).init(p.default_value.u16) },
                .u32 => .{ .u32 = std.atomic.Value(u32).init(p.default_value.u32) },
                .u64 => .{ .u64 = std.atomic.Value(u64).init(p.default_value.u64) },
                .i8 => .{ .i8 = std.atomic.Value(i8).init(p.default_value.i8) },
                .i16 => .{ .i16 = std.atomic.Value(i16).init(p.default_value.i16) },
                .i32 => .{ .i32 = std.atomic.Value(i32).init(p.default_value.i32) },
                .i64 => .{ .i64 = std.atomic.Value(i64).init(p.default_value.i64) },
                else => return error.InvalidParameterType,
            },
        };
        var param: ?*Parameter = try Parameter.init(
            data,
            p.read_group,
            p.write_group,
            p.getter,
            p.setter,
        );
        errdefer if (param) |pa| pa.deinit();
        try inner.addParameter(std.mem.span(p.name), param.?);
        const param_copy = param;
        param = null;
        try parameters.append(allocator, param_copy);
    }
    try parameters.append(allocator, null);
    instance.parameters = @ptrCast((try parameters.toOwnedSlice(allocator)).ptr);

    // Init resources.
    var resources = std.ArrayListUnmanaged(?[*:0]u8){};
    errdefer resources.deinit(allocator);
    errdefer for (resources.items) |x| if (x) |r| allocator.free(std.mem.span(r));
    for (@"export".getResources()) |res| {
        var buf = PathBufferUnmanaged{};
        defer buf.deinit(allocator);
        try buf.pushPath(allocator, handle.path.asPath());
        try buf.pushString(allocator, std.mem.span(res.path));
        const p = try allocator.dupeZ(u8, buf.asPath().raw);
        errdefer allocator.free(p);
        try resources.append(allocator, p);
    }
    try resources.append(allocator, null);
    instance.resources = @ptrCast((try resources.toOwnedSlice(allocator)).ptr);

    // Init namespaces.
    for (@"export".getNamespaceImports()) |imp| {
        const name = std.mem.span(imp.name);
        if (sys.getNamespace(std.mem.span(imp.name)) == null) return error.NotFound;
        try inner.addNamespace(name, .static);
    }

    // Init imports.
    var imports = std.ArrayListUnmanaged(?*const anyopaque){};
    errdefer imports.deinit(allocator);
    for (@"export".getSymbolImports()) |imp| {
        const imp_name = std.mem.span(imp.name);
        const imp_namespace = std.mem.span(imp.namespace);
        const imp_version = Version.initC(imp.version);
        const sym = sys.getSymbolCompatible(
            imp_name,
            imp_namespace,
            imp_version,
        ) orelse return error.NotFound;

        const owner = sys.getInstance(sym.owner).?;
        const owner_handle = Self.fromInstancePtr(owner.instance);
        const owner_inner = owner_handle.lock();
        defer owner_inner.unlock();

        const owner_sym = owner_inner.getSymbol(
            imp_name,
            imp_namespace,
            imp_version,
        ).?;
        try imports.append(allocator, owner_sym.symbol);
        if (inner.getDependency(sym.owner) == null) {
            try inner.addDependency(sym.owner, .{
                .instance = owner_handle,
                .type = .static,
            });
        }
    }
    try imports.append(allocator, null);
    instance.imports = @ptrCast((try imports.toOwnedSlice(allocator)).ptr);

    // Init instance data.
    if (@"export".module_constructor) |constructor| {
        inner.unlock();
        set.unlock();
        sys.mutex.unlock();
        var data: ?*anyopaque = undefined;
        const result = constructor(instance, @ptrCast(set), &data);
        sys.mutex.lock();
        set.lock();
        _ = instance_handle.lock();
        instance.data = @ptrCast(data);
        try AnyError.initChecked(err, result);
    }

    // Init exports.
    var exports = std.ArrayListUnmanaged(?*const anyopaque){};
    errdefer exports.deinit(allocator);
    for (@"export".getSymbolExports()) |exp| {
        const sym = exp.symbol;
        const exp_name = std.mem.span(exp.name);
        const exp_namespace = std.mem.span(exp.namespace);
        const exp_version = Version.initC(exp.version);
        try inner.addSymbol(exp_name, exp_namespace, .{
            .symbol = sym,
            .version = exp_version,
            .dtor = null,
        });
        try exports.append(allocator, sym);
    }
    for (@"export".getDynamicSymbolExports()) |exp| {
        inner.unlock();
        sys.mutex.unlock();
        var sym: *anyopaque = undefined;
        const result = exp.constructor(instance, &sym);
        sys.mutex.lock();
        _ = instance_handle.lock();
        try AnyError.initChecked(err, result);
        var skip_dtor = false;
        errdefer if (!skip_dtor) exp.destructor(sym);

        const exp_name = std.mem.span(exp.name);
        const exp_namespace = std.mem.span(exp.namespace);
        const exp_version = Version.initC(exp.version);
        try inner.addSymbol(exp_name, exp_namespace, .{
            .symbol = sym,
            .version = exp_version,
            .dtor = exp.destructor,
        });
        skip_dtor = true;
        try exports.append(allocator, sym);
    }
    try exports.append(allocator, null);
    instance.exports = @ptrCast((try exports.toOwnedSlice(allocator)).ptr);
    return instance;
}

pub fn fromInstancePtr(instance: *const ProxyModule.OpaqueInstance) *const Self {
    return fromInfoPtr(instance.info);
}

pub fn fromInfoPtr(info: *const ProxyModule.Info) *const Self {
    return @fieldParentPtr("info", @constCast(info));
}

pub fn fromInnerPtr(inner: *Inner) *const Self {
    return @fieldParentPtr("inner", inner);
}

fn ref(self: *const Self) void {
    const this: *Self = @constCast(self);
    this.ref_count.ref();
}

fn unref(self: *const Self, cleanup: bool) void {
    const this: *Self = @constCast(self);
    if (this.ref_count.unref() == .noop) return;

    const inner = this.lock();
    defer inner.unlock();
    if (!inner.isDetached()) inner.detach(cleanup).release();
}

pub fn lock(self: *const Self) *Inner {
    const this: *Self = @constCast(self);
    this.inner.mutex.lock();
    return &this.inner;
}
