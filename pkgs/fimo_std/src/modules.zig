//! Modules subsystem of the context.
//!
//! Requires an initialized global context.
const std = @import("std");
const builtin = @import("builtin");

const c = @import("c");

const ctx = @import("ctx.zig");
pub const exports = @import("modules/exports.zig");
pub const Export = exports.Export;
pub const Module = exports.Module;
pub const ModuleBundle = exports.ModuleBundle;
const paths = @import("paths.zig");
const tasks = @import("tasks.zig");
const EnqueuedFuture = tasks.EnqueuedFuture;
const Fallible = tasks.Fallible;
const Version = @import("Version.zig");

/// Data type of a module parameter.
pub const ParameterType = enum(i32) {
    u8,
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
    _,
};

/// Access group for a module parameter.
pub const ParameterAccessGroup = enum(i32) {
    /// Parameter can be accessed publicly.
    public,
    /// Parameter can be accessed from dependent modules.
    dependency,
    /// Parameter can only be accessed from the owning module.
    private,
};

/// Parameter accessor.
pub fn Parameter(comptime T: type) type {
    std.debug.assert(T == anyopaque or std.mem.indexOfScalar(
        u16,
        &.{ 8, 16, 32, 64 },
        @typeInfo(T).int.bits,
    ) != null);

    return extern struct {
        vtable: Self.VTable,

        fn isParameter() bool {
            return true;
        }

        const Self = @This();

        /// Parameter info.
        pub const Info = struct {
            tag: ParameterType,
            read_group: ParameterAccessGroup,
            write_group: ParameterAccessGroup,
        };

        /// VTable of a parameter.
        ///
        /// Adding fields to this struct is not a breaking change.
        pub const VTable = extern struct {
            type: *const fn (data: *const Self) callconv(.c) ParameterType,
            read: *const fn (data: *const Self, value: *T) callconv(.c) void,
            write: *const fn (data: *Self, value: *const T) callconv(.c) void,
        };

        /// Returns the value type of the parameter.
        pub fn @"type"(self: *const Self) ParameterType {
            return self.vtable.type(self);
        }

        /// Reads the value from the parameter.
        pub fn read(self: *const Self) T {
            var value: T = undefined;
            self.vtable.read(self, &value);
            return value;
        }

        /// Writes the value into the parameter.
        pub fn write(self: *Self, value: T) void {
            self.vtable.write(self, &value);
        }

        /// Casts the opaque parameter to a typed variant.
        pub fn castFromOpaque(parameter: *OpaqueParameter) *Self {
            if (comptime T == void) {
                return parameter;
            } else {
                const expected_type: ParameterType = switch (comptime @typeInfo(T).int.bits) {
                    8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                    16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                    32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                    64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
                };
                std.debug.assert(parameter.type() == expected_type);
                return @ptrCast(parameter);
            }
        }

        /// Casts the opaque parameter to a typed variant.
        pub fn castFromOpaqueConst(parameter: *const OpaqueParameter) *const Self {
            if (comptime T == void) {
                return parameter;
            } else {
                const expected_type: ParameterType = switch (comptime @typeInfo(T).int.bits) {
                    8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                    16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                    32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                    64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
                };
                std.debug.assert(parameter.type() == expected_type);
                return @ptrCast(parameter);
            }
        }

        /// Casts the parameter to an opaque parameter.
        pub fn castToOpaque(self: *Self) *OpaqueParameter {
            return @ptrCast(self);
        }

        /// Casts the parameter to an opaque parameter.
        pub fn castToOpaqueConst(self: *const Self) *const OpaqueParameter {
            return @ptrCast(self);
        }
    };
}

/// Untyped parameter accessor.
pub const OpaqueParameter = Parameter(anyopaque);

/// Internal state of a parameter.
pub fn ParameterData(comptime T: type) type {
    std.debug.assert(
        T == anyopaque or std.mem.indexOfScalar(
            u16,
            &.{ 8, 16, 32, 64 },
            @typeInfo(T).int.bits,
        ) != null,
    );

    return extern struct {
        data: *anyopaque,
        vtable: *const @This().VTable,

        /// VTable of a parameter data.
        ///
        /// Adding fields to this struct is not a breaking change.
        pub const VTable = extern struct {
            type: *const fn (data: *anyopaque) callconv(.c) ParameterType,
            read: *const fn (data: *anyopaque, value: *T) callconv(.c) void,
            write: *const fn (data: *anyopaque, value: *const T) callconv(.c) void,
        };

        /// Returns the value type of the parameter.
        pub fn @"type"(self: @This()) ParameterType {
            return self.vtable.type(self.data);
        }

        /// Reads the value from the parameter data.
        pub fn read(self: @This()) T {
            var value: T = undefined;
            self.vtable.read(self.data, &value);
            return value;
        }

        /// Writes the value into the parameter data.
        pub fn write(self: @This(), value: T) void {
            self.vtable.write(self.data, &value);
        }

        /// Casts the opaque parameter data to a typed variant.
        pub fn castFromOpaque(parameter: OpaqueParameterData) @This() {
            if (comptime T == void) {
                return parameter;
            } else {
                const expected_type: ParameterType = switch (comptime @typeInfo(T).int.bits) {
                    8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                    16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                    32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                    64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
                };
                std.debug.assert(parameter.type() == expected_type);
                return @bitCast(parameter);
            }
        }

        /// Casts the parameter data to an opaque parameter data.
        pub fn castToOpaque(self: @This()) OpaqueParameterData {
            return @bitCast(self);
        }
    };
}

/// Untyped state of a parameter.
pub const OpaqueParameterData = ParameterData(anyopaque);

/// Information about a symbol.
pub const Symbol = struct {
    name: [:0]const u8,
    namespace: [:0]const u8 = "",
    version: Version,
    T: type,

    fn Global(comptime symbol: Symbol) type {
        return struct {
            var lock: std.Thread.Mutex = .{};
            var count: usize = 0;
            var ptr: ?*const symbol.T = null;

            pub fn register(val: *const symbol.T) void {
                lock.lock();
                defer lock.unlock();
                const old_count = @atomicRmw(usize, &count, .Add, 1, .monotonic);
                if (old_count == 0) {
                    const old = @atomicRmw(?*const symbol.T, &ptr, .Xchg, val, .monotonic);
                    std.debug.assert(old == val or old == null);
                }
            }

            pub fn registerFrom(obj: anytype) void {
                const val = symbol.requestFrom(obj);
                register(val);
            }

            pub fn unregister() void {
                lock.lock();
                defer lock.unlock();
                const old_count = @atomicRmw(usize, &count, .Sub, 1, .monotonic);
                if (old_count == 1) @atomicStore(?*const symbol.T, &ptr, null, .monotonic);
            }

            pub fn get() *const symbol.T {
                std.debug.assert(@atomicLoad(usize, &count, .monotonic) != 0);
                return @atomicLoad(?*const symbol.T, &ptr, .monotonic) orelse unreachable;
            }
        };
    }

    pub fn getGlobal(comptime symbol: Symbol) type {
        return Global(symbol);
    }

    /// Requests the symbol from `obj`.
    pub fn requestFrom(comptime symbol: Symbol, obj: anytype) *const symbol.T {
        return obj.provideSymbol(symbol);
    }

    pub fn format(self: Symbol, w: *std.Io.Writer) std.Io.Writer.Error!void {
        if (self.namespace.len != 0) {
            try w.print("{s}::{s}@v{}", .{ self.namespace, self.name, self.version });
        } else {
            try w.print("{s}@v{}", .{ self.name, self.version });
        }
    }
};

/// A wrapper type that binds a symbol to its pointer value.
pub fn SymbolWrapper(comptime symbol: Symbol) type {
    return extern struct {
        value: *const symbol.T,

        pub fn registerGlobal(self: @This()) void {
            const Global = symbol.getGlobal();
            Global.registerFrom(self);
        }

        pub fn unregisterGlobal(self: @This()) void {
            _ = self;
            const Global = symbol.getGlobal();
            Global.unregister();
        }

        /// Asserts that `sym` is compatible to the contained symbol and returns it.
        pub fn provideSymbol(self: @This(), comptime sym: Symbol) *const sym.T {
            comptime {
                std.debug.assert(std.mem.eql(u8, sym.name, symbol.name));
                std.debug.assert(std.mem.eql(u8, sym.namespace, symbol.namespace));
                std.debug.assert(symbol.version.isCompatibleWith(sym.version));
            }
            return self.value;
        }
    };
}

test SymbolWrapper {
    const symbol = Symbol{
        .name = "test",
        .version = .{ .major = 1, .minor = 0, .patch = 0 },
        .T = i32,
    };
    const wrapper = SymbolWrapper(symbol){ .value = &5 };
    try std.testing.expectEqual(5, symbol.requestFrom(wrapper).*);
}

pub fn SymbolGroup(comptime symbols: anytype) type {
    const symbols_info = @typeInfo(@TypeOf(symbols)).@"struct";
    var fields: []const std.builtin.Type.StructField = &.{};
    for (symbols_info.fields) |f| {
        std.debug.assert(f.type == Symbol);
        const symbol: Symbol = @field(symbols, f.name);
        fields = fields ++ [_]std.builtin.Type.StructField{.{
            .name = f.name,
            .type = *const symbol.T,
            .default_value_ptr = null,
            .is_comptime = false,
            .alignment = @alignOf(*const symbol.T),
        }};
    }
    const Inner = @Type(.{
        .@"struct" = .{
            .layout = .auto,
            .fields = fields,
            .decls = &.{},
            .is_tuple = symbols_info.is_tuple,
        },
    });

    return struct {
        symbols: Inner,

        pub fn registerGlobal(self: @This()) void {
            inline for (symbols_info.fields) |f| {
                const sym: Symbol = @field(symbols, f.name);
                const Global = sym.getGlobal();
                Global.registerFrom(self);
            }
        }

        pub fn unregisterGlobal(self: @This()) void {
            _ = self;
            inline for (symbols_info.fields) |f| {
                const sym: Symbol = @field(symbols, f.name);
                const Global = sym.getGlobal();
                Global.unregister();
            }
        }

        /// Asserts that `sym` is compatible to the contained symbol and returns it.
        pub fn provideSymbol(self: @This(), comptime sym: Symbol) *const sym.T {
            const name = comptime blk: {
                for (symbols_info.fields) |f| {
                    const symbol: Symbol = @field(symbols, f.name);
                    if (std.mem.eql(u8, sym.name, symbol.name) and
                        std.mem.eql(u8, sym.namespace, symbol.namespace) and
                        symbol.version.isCompatibleWith(sym.version)) break :blk f.name;
                }
                @compileError(std.fmt.comptimePrint(
                    "the symbol group does not provide the symbol {}",
                    .{sym},
                ));
            };
            return @field(self.symbols, name);
        }
    };
}

test SymbolGroup {
    const a = Symbol{
        .name = "a",
        .version = .{ .major = 1, .minor = 0, .patch = 0 },
        .T = i32,
    };
    const b = Symbol{
        .name = "b",
        .version = .{ .major = 1, .minor = 0, .patch = 0 },
        .T = i32,
    };
    const group = SymbolGroup(.{ a, b }){ .symbols = .{ &5, &10 } };
    try std.testing.expectEqual(5, a.requestFrom(group).*);
    try std.testing.expectEqual(10, b.requestFrom(group).*);
}

/// Info of a loaded module instance.
pub const Info = extern struct {
    next: ?*anyopaque = null,
    name: [*:0]const u8,
    description: ?[*:0]const u8 = null,
    author: ?[*:0]const u8 = null,
    license: ?[*:0]const u8 = null,
    module_path: ?[*:0]const u8 = null,
    vtable: Info.VTable,

    /// VTable of a info instance.
    ///
    /// Adding fields to the vtable is not a breaking change.
    pub const VTable = extern struct {
        ref: *const fn (ctx: *const Info) callconv(.c) void,
        unref: *const fn (ctx: *const Info) callconv(.c) void,
        mark_unloadable: *const fn (ctx: *const Info) callconv(.c) void,
        is_loaded: *const fn (ctx: *const Info) callconv(.c) bool,
        try_ref_instance_strong: *const fn (ctx: *const Info) callconv(.c) bool,
        unref_instance_strong: *const fn (ctx: *const Info) callconv(.c) void,
    };

    /// Increases the reference count of the info instance.
    pub fn ref(self: *const Info) void {
        self.vtable.ref(self);
    }

    /// Decreases the reference count of the info instance.
    pub fn unref(self: *const Info) void {
        self.vtable.unref(self);
    }

    /// Signals that the owning instance may be unloaded.
    ///
    /// The instance will be unloaded once it is no longer actively used by another instance.
    pub fn markUnloadable(self: *const Info) void {
        self.vtable.mark_unloadable(self);
    }

    /// Returns whether the owning module instance is still loaded.
    pub fn isLoaded(self: *const Info) bool {
        return self.vtable.is_loaded(self);
    }

    /// Tries to increase the strong reference count of the module instance.
    ///
    /// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
    /// between modules, without registering the dependency with the subsystem.
    pub fn tryRefInstanceStrong(self: *const Info) bool {
        return self.vtable.try_ref_instance_strong(self);
    }

    /// Decreases the strong reference count of the module instance.
    ///
    /// Should only be called after `tryRefInstanceStrong`, when the dependency is no longer
    /// required.
    pub fn unrefInstanceStrong(self: *const Info) void {
        self.vtable.unref_instance_strong(self);
    }

    /// Searches for a module by its name.
    ///
    /// Queries a module by its unique name. The returned `Info` instance will have its reference
    /// count increased.
    pub fn findByName(module: [:0]const u8) ctx.Error!*const Info {
        var info: *const Info = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.find_by_name(module.ptr, &info).intoErrorUnion();
        return info;
    }

    /// Searches for a module by a symbol it exports.
    ///
    /// Queries the module that exported the specified symbol. The returned `Info` instance will
    /// have its reference count increased.
    pub fn findBySymbol(
        name: [:0]const u8,
        namespace: [:0]const u8,
        version: Version,
    ) ctx.Error!*const Info {
        var info: *const Info = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.find_by_symbol(
            name.ptr,
            namespace.ptr,
            version.intoC(),
            &info,
        ).intoErrorUnion();
        return info;
    }
};

/// Configuration for an instance.
pub const InstanceConfig = struct {
    /// Type of the parameters table.
    ParametersType: type = void,
    /// Type of the resources table.
    ResourcesType: type = void,
    /// Type of the imports table.
    ImportsType: type = void,
    /// Type of the exports table.
    ExportsType: type = void,
    /// Type of the instance state.
    StateType: type = void,
    @"export": ?*const anyopaque = null,
    provider: ?fn (instance: anytype, comptime symbol: Symbol) *const anyopaque = null,
};

/// State of a loaded module.
///
/// A module is self-contained, and may not be passed to other modules. An instance is valid for
/// as long as the owning module remains loaded. Modules must not leak any resources outside its
/// own module, ensuring that they are destroyed upon module unloading.
pub fn Instance(comptime config: InstanceConfig) type {
    return extern struct {
        vtable: *const Self.VTable,
        parameters_: if (@sizeOf(Self.Parameters) == 0)
            ?*const Self.Parameters
        else
            *const Self.Parameters,
        resources_: if (@sizeOf(Self.Resources) == 0)
            ?*const Self.Resources
        else
            *const Self.Resources,
        imports_: if (@sizeOf(Self.Imports) == 0)
            ?*const Self.Imports
        else
            *const Self.Imports,
        exports_: if (@sizeOf(Self.Exports) == 0)
            ?*const Self.Exports
        else
            *const Self.Exports,
        info: *const Info,
        handle: *const ctx.Handle,
        state_: if (@sizeOf(Self.State) == 0) ?*Self.State else *Self.State,

        const Self = @This();
        pub const Parameters = config.ParametersType;
        pub const Resources = config.ResourcesType;
        pub const Imports = config.ImportsType;
        pub const Exports = config.ExportsType;
        pub const State = config.StateType;

        pub const @"export" = if (config.@"export") |e| @as(*const Export, @ptrCast(@alignCast(e))) else {};
        pub const fimo_module_instance_marker: void = {};

        /// VTable of an Instance.
        ///
        /// Adding fields to the VTable is not a breaking change.
        pub const VTable = extern struct {
            ref: *const fn (ctx: *const OpaqueInstance) callconv(.c) void,
            unref: *const fn (ctx: *const OpaqueInstance) callconv(.c) void,
            query_namespace: *const fn (
                ctx: *const OpaqueInstance,
                namespace: [*:0]const u8,
                has_dependency: *bool,
                is_static: *bool,
            ) callconv(.c) ctx.Status,
            add_namespace: *const fn (
                ctx: *const OpaqueInstance,
                namespace: [*:0]const u8,
            ) callconv(.c) ctx.Status,
            remove_namespace: *const fn (
                ctx: *const OpaqueInstance,
                namespace: [*:0]const u8,
            ) callconv(.c) ctx.Status,
            query_dependency: *const fn (
                ctx: *const OpaqueInstance,
                info: *const Info,
                has_dependency: *bool,
                is_static: *bool,
            ) callconv(.c) ctx.Status,
            add_dependency: *const fn (
                ctx: *const OpaqueInstance,
                info: *const Info,
            ) callconv(.c) ctx.Status,
            remove_dependency: *const fn (
                ctx: *const OpaqueInstance,
                info: *const Info,
            ) callconv(.c) ctx.Status,
            load_symbol: *const fn (
                ctx: *const OpaqueInstance,
                name: [*:0]const u8,
                namespace: [*:0]const u8,
                version: Version.CVersion,
                symbol: **const anyopaque,
            ) callconv(.c) ctx.Status,
            read_parameter: *const fn (
                ctx: *const OpaqueInstance,
                value: *anyopaque,
                type: ParameterType,
                module: [*:0]const u8,
                parameter: [*:0]const u8,
            ) callconv(.c) ctx.Status,
            write_parameter: *const fn (
                ctx: *const OpaqueInstance,
                value: *const anyopaque,
                type: ParameterType,
                module: [*:0]const u8,
                parameter: [*:0]const u8,
            ) callconv(.c) ctx.Status,
        };

        /// Returns the parameter table.
        pub fn parameters(self: *const Self) *const Self.Parameters {
            return if (comptime @sizeOf(Self.Parameters) == 0)
                self.parameters_ orelse &Self.Parameters{}
            else
                self.parameters_;
        }

        /// Returns the resource table.
        pub fn resources(self: *const Self) *const Self.Resources {
            return if (comptime @sizeOf(Self.Resources) == 0)
                self.resources_ orelse &Self.Resources{}
            else
                self.resources_;
        }

        /// Returns the import table.
        pub fn imports(self: *const Self) *const Self.Imports {
            return if (comptime @sizeOf(Self.Imports) == 0)
                self.imports_ orelse &Self.Imports{}
            else
                self.imports_;
        }

        /// Returns the export table.
        pub fn exports(self: *const Self) *const Self.Exports {
            return if (comptime @sizeOf(Self.Exports) == 0)
                self.exports_ orelse &Self.Exports{}
            else
                self.exports_;
        }

        /// Returns the instance state.
        pub fn state(self: *const @This()) *Self.State {
            return if (comptime @sizeOf(Self.State) == 0)
                self.state_ orelse &Self.State{}
            else
                self.state_;
        }

        /// Provides a pointer to the requested symbol.
        pub fn provideSymbol(self: *const @This(), comptime symbol: Symbol) *const symbol.T {
            if (config.provider) |provider|
                return @ptrCast(@alignCast(provider(self, symbol)))
            else
                @compileError("instance configuration does not specify a symbol provider function.");
        }

        /// Increases the strong reference count of the module instance.
        ///
        /// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
        /// between modules, without registering the dependency with the subsystem.
        pub fn ref(self: *const @This()) void {
            self.vtable.ref(self.castOpaque());
        }

        /// Decreases the strong reference count of the module instance.
        ///
        /// Should only be called after `ref`, when the dependency is no longer
        /// required.
        pub fn unref(self: *const @This()) void {
            self.vtable.unref(self.castOpaque());
        }

        /// Checks the status of a namespace from the view of the module.
        ///
        /// Checks if the module includes the namespace. In that case, the module is allowed access
        /// to the symbols in the namespace. Additionally, this function also queries whether the
        /// include is static, i.e., it was specified by the module at load time.
        pub fn queryNamespace(
            self: *const @This(),
            namespace: [:0]const u8,
        ) ctx.Error!enum { removed, added, static } {
            var has_dependency: bool = undefined;
            var is_static: bool = undefined;
            try self.vtable.query_namespace(
                self.castOpaque(),
                namespace.ptr,
                &has_dependency,
                &is_static,
            ).intoErrorUnion();
            if (!has_dependency) return .removed;
            if (!is_static) return .added;
            return .static;
        }

        /// Includes a namespace by the module.
        ///
        /// Once included, the module gains access to the symbols of its dependencies that are
        /// exposed in said namespace. A namespace can not be included multiple times.
        pub fn addNamespace(self: *const @This(), namespace: [:0]const u8) ctx.Error!void {
            try self.vtable.add_namespace(self.castOpaque(), namespace.ptr).intoErrorUnion();
        }

        /// Removes a namespace include from the module.
        ///
        /// Once excluded, the caller guarantees to relinquish access to the symbols contained in
        /// said namespace. It is only possible to exclude namespaces that were manually added,
        /// whereas static namespace includes remain valid until the module is unloaded.
        pub fn removeNamespace(self: *const @This(), namespace: [:0]const u8) ctx.Error!void {
            try self.vtable.remove_namespace(self.castOpaque(), namespace.ptr).intoErrorUnion();
        }

        /// Checks if a module depends on another module.
        ///
        /// Checks if the specified module is a dependency of the current instance. In that case
        /// the instance is allowed to access the symbols exported by the module. Additionally,
        /// this function also queries whether the dependency is static, i.e., the dependency was
        /// specified by the module at load time.
        pub fn queryDependency(
            self: *const @This(),
            info: *const Info,
        ) ctx.Error!enum { removed, added, static } {
            var has_dependency: bool = undefined;
            var is_static: bool = undefined;
            try self.vtable.query_dependency(self.castOpaque(), info, &has_dependency, &is_static)
                .intoErrorUnion();
            if (!has_dependency) return .removed;
            if (!is_static) return .added;
            return .static;
        }

        /// Acquires another module as a dependency.
        ///
        /// After acquiring a module as a dependency, the module is allowed access to the symbols
        /// and protected parameters of said dependency. Trying to acquire a dependency to a module
        /// that is already a dependency, or to a module that would result in a circular dependency
        /// will result in an error.
        pub fn addDependency(self: *const @This(), info: *const Info) ctx.Error!void {
            try self.vtable.add_dependency(self.castOpaque(), info).intoErrorUnion();
        }

        /// Removes a module as a dependency.
        ///
        /// By removing a module as a dependency, the caller ensures that it does not own any
        /// references to resources originating from the former dependency, and allows for the
        /// unloading of the module. A module can only relinquish dependencies to modules that were
        /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
        pub fn removeDependency(self: *const @This(), info: *const Info) ctx.Error!void {
            try self.vtable.remove_dependency(self.castOpaque(), info).intoErrorUnion();
        }

        /// Loads a group of symbols from the module subsystem.
        ///
        /// Is equivalent to calling `loadSymbol` for each symbol of the group independently.
        pub fn loadSymbolGroup(
            self: *const @This(),
            comptime symbols: anytype,
        ) ctx.Error!SymbolGroup(symbols) {
            var group: SymbolGroup(symbols) = undefined;
            inline for (std.meta.fields(@TypeOf(symbols))) |f| {
                const symbol = try self.loadSymbol(@field(symbols, f.name));
                @field(group.symbols, f.name) = symbol.value;
            }
            return group;
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
        /// loading optional symbols, or for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until the module relinquishes the
        /// dependency to the module that exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbol(
            self: *const @This(),
            comptime symbol: Symbol,
        ) ctx.Error!SymbolWrapper(symbol) {
            const s = try self.loadSymbolRaw(
                symbol.name,
                symbol.namespace,
                symbol.version,
            );
            return .{ .value = @ptrCast(@alignCast(s)) };
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
        /// loading optional symbols, or for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until the module relinquishes the
        /// dependency to the module that exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbolRaw(
            self: *const @This(),
            name: [:0]const u8,
            namespace: [:0]const u8,
            version: Version,
        ) ctx.Error!*const anyopaque {
            var sym: *const anyopaque = undefined;
            try self.vtable.load_symbol(self.castOpaque(), name, namespace, version.intoC(), &sym)
                .intoErrorUnion();
            return sym;
        }

        /// Reads a module parameter with dependency read access.
        ///
        /// Reads the value of a module parameter with dependency read access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow reading with a
        /// dependency access.
        pub fn readParameter(
            self: *const @This(),
            comptime T: type,
            module: [:0]const u8,
            parameter: [:0]const u8,
        ) ctx.Error!T {
            std.debug.assert(std.mem.indexOfScalar(
                u16,
                &.{ 8, 16, 32, 64 },
                @typeInfo(T).int.bits,
            ) != null);
            var value: T = undefined;
            const value_type: ParameterType = switch (comptime @typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
            };
            try self.vtable.read_parameter(
                self.castOpaque(),
                &value,
                value_type,
                module.ptr,
                parameter.ptr,
            ).intoErrorUnion();
            return value;
        }

        /// Sets a module parameter with dependency write access.
        ///
        /// Sets the value of a module parameter with dependency write access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow writing with a
        /// dependency access.
        pub fn writeParameter(
            self: *const @This(),
            comptime T: type,
            value: T,
            module: [:0]const u8,
            parameter: [:0]const u8,
        ) ctx.Error!void {
            const value_type: ParameterType = switch (comptime @typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
            };
            try self.vtable.write_parameter(
                self.castOpaque(),
                &value,
                value_type,
                module.ptr,
                parameter.ptr,
            ).intoErrorUnion();
        }

        /// Casts the instance pointer to an opaque instance pointer.
        pub fn castOpaque(self: *const @This()) *const OpaqueInstance {
            return @ptrCast(self);
        }
    };
}

/// Type of an opaque module instance.
pub const OpaqueInstance = Instance(.{});

/// Type of a root module instance.
pub const RootInstance = extern struct {
    instance: OpaqueInstance,

    pub const Parameters = OpaqueInstance.Parameters;
    pub const Resources = OpaqueInstance.Resources;
    pub const Imports = OpaqueInstance.Imports;
    pub const Exports = OpaqueInstance.Exports;
    pub const State = OpaqueInstance.State;

    /// Constructs a new root instance.
    ///
    /// The functions of the module subsystem require that the caller owns a reference to their own
    /// module. This is a problem, as the constructor of the context won't be assigned a module
    /// instance during bootstrapping. As a workaround, we allow for the creation of root
    /// instances, i.e., module handles without an associated module.
    pub fn init() ctx.Error!*const RootInstance {
        var instance: *const RootInstance = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.root_module_new(&instance).intoErrorUnion();
        return instance;
    }

    /// Destroys the root module.
    ///
    /// By destroying the root module, the caller ensures that they relinquished all access to
    /// handles derived by the module subsystem.
    pub fn deinit(self: *const RootInstance) void {
        self.castOpaque().info.markUnloadable();
    }

    /// Checks the status of a namespace from the view of the module.
    ///
    /// Checks if the module includes the namespace. In that case, the module is allowed access to
    /// the symbols in the namespace. Additionally, this function also queries whether the include
    /// is static, i.e., the include was specified by the module at load time.
    pub fn queryNamespace(
        self: *const @This(),
        namespace: [:0]const u8,
    ) ctx.Error!enum { removed, added, static } {
        return self.castOpaque().queryNamespace(namespace);
    }

    /// Includes a namespace by the module.
    ///
    /// Once included, the module gains access to the symbols of its dependencies that are exposed
    /// in said namespace. A namespace can not be included multiple times.
    pub fn addNamespace(self: *const @This(), namespace: [:0]const u8) ctx.Error!void {
        return self.castOpaque().addNamespace(namespace);
    }

    /// Removes a namespace include from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in said
    /// namespace. It is only possible to exclude namespaces that were manually added, whereas
    /// static namespace includes remain valid until the module is unloaded.
    pub fn removeNamespace(self: *const @This(), namespace: [:0]const u8) ctx.Error!void {
        return self.castOpaque().removeNamespace(namespace);
    }

    /// Checks if a module depends on another module.
    ///
    /// Checks if the specified module is a dependency of the current instance. In that case the
    /// instance is allowed to access the symbols exported by the module. Additionally, this
    /// function also queries whether the dependency is static, i.e., the dependency was specified
    /// by the module at load time.
    pub fn queryDependency(
        self: *const @This(),
        info: *const Info,
    ) ctx.Error!enum { removed, added, static } {
        return self.castOpaque().queryDependency(info);
    }

    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module is allowed access to the symbols and
    /// protected parameters of said dependency. Trying to acquire a dependency to a module that is
    /// already a dependency, or to a module that would result in a circular dependency will result
    /// in an error.
    pub fn addDependency(self: *const @This(), info: *const Info) ctx.Error!void {
        return self.castOpaque().addDependency(info);
    }

    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller ensures that it does not own any
    /// references to resources originating from the former dependency, and allows for the
    /// unloading of the module. A module can only relinquish dependencies to modules that were
    /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
    pub fn removeDependency(self: *const @This(), info: *const Info) ctx.Error!void {
        return self.castOpaque().removeDependency(info);
    }

    /// Loads a group of symbols from the module subsystem.
    ///
    /// Is equivalent to calling `loadSymbol` for each symbol of the group independently.
    pub fn loadSymbolGroup(
        self: *const @This(),
        comptime symbols: anytype,
    ) ctx.Error!SymbolGroup(symbols) {
        return self.castOpaque().loadSymbolGroup(symbols);
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until the module relinquishes the
    /// dependency to the module that exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    pub fn loadSymbol(
        self: *const @This(),
        comptime symbol: Symbol,
    ) ctx.Error!SymbolWrapper(symbol) {
        return self.castOpaque().loadSymbol(symbol);
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until the module relinquishes the
    /// dependency to the module that exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    pub fn loadSymbolRaw(
        self: *const @This(),
        name: [:0]const u8,
        namespace: [:0]const u8,
        version: Version,
    ) ctx.Error!*const anyopaque {
        return self.castOpaque().loadSymbolRaw(name, namespace, version);
    }

    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow reading with a dependency
    /// access.
    pub fn readParameter(
        self: *const @This(),
        comptime T: type,
        module: [:0]const u8,
        parameter: [:0]const u8,
    ) ctx.Error!T {
        return self.castOpaque().readParameter(T, module, parameter);
    }

    /// Sets a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow writing with a dependency
    /// access.
    pub fn writeParameter(
        self: *const @This(),
        comptime T: type,
        value: T,
        module: [:0]const u8,
        parameter: [:0]const u8,
    ) ctx.Error!void {
        return self.castOpaque().writeParameter(T, value, module, parameter);
    }

    /// Casts the instance pointer to an opaque instance pointer.
    pub fn castOpaque(self: *const @This()) *const OpaqueInstance {
        return @ptrCast(self);
    }
};

/// Type-erased set of modules to load by the subsystem.
pub const LoadingSet = extern struct {
    data: *anyopaque,
    vtable: *const LoadingSet.VTable,

    /// Operation of the filter function.
    pub const FilterRequest = enum(i32) {
        skip,
        load,
    };

    /// VTable of a loading set.
    ///
    /// Adding to the VTable is not a breaking change.
    pub const VTable = extern struct {
        ref: *const fn (ctx: *anyopaque) callconv(.c) void,
        unref: *const fn (ctx: *anyopaque) callconv(.c) void,
        query_module: *const fn (
            ctx: *anyopaque,
            module: [*:0]const u8,
        ) callconv(.c) bool,
        query_symbol: *const fn (
            ctx: *anyopaque,
            symbol: [*:0]const u8,
            namespace: [*:0]const u8,
            version: Version.CVersion,
        ) callconv(.c) bool,
        add_callback: *const fn (
            ctx: *anyopaque,
            module: [*:0]const u8,
            on_success: *const fn (info: *const Info, data: ?*anyopaque) callconv(.c) void,
            on_error: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) void,
            on_abort: ?*const fn (data: ?*anyopaque) callconv(.c) void,
            data: ?*anyopaque,
        ) callconv(.c) ctx.Status,
        add_module: *const fn (
            ctx: *anyopaque,
            owner: *const OpaqueInstance,
            module: *const Export,
        ) callconv(.c) ctx.Status,
        add_modules_from_path: *const fn (
            ctx: *anyopaque,
            path: paths.compat.Path,
            filter_fn: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) FilterRequest,
            filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
            filter_data: ?*anyopaque,
        ) callconv(.c) ctx.Status,
        add_modules_from_local: *const fn (
            ctx: *anyopaque,
            filter_fn: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) FilterRequest,
            filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
            filter_data: ?*anyopaque,
            iterator_fn: *const fn (
                f: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
                data: ?*anyopaque,
            ) callconv(.c) void,
            bin_ptr: *const anyopaque,
        ) callconv(.c) ctx.Status,
        commit: *const fn (
            ctx: *anyopaque,
        ) callconv(.c) EnqueuedFuture(Fallible(void)),
    };

    /// Constructs a new empty set.
    ///
    /// Modules can only be loaded, if all of their dependencies can be resolved, which requires us
    /// to determine a suitable load order. A loading set is a utility to facilitate this process,
    /// by automatically computing a suitable load order for a batch of modules.
    pub fn init() ctx.Error!LoadingSet {
        var set: LoadingSet = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.set_new(&set).intoErrorUnion();
        return set;
    }

    /// Increases the reference count of the instance.
    pub fn ref(self: LoadingSet) void {
        self.vtable.ref(self.data);
    }

    /// Decreases the reference count of the instance.
    pub fn unref(self: LoadingSet) void {
        self.vtable.unref(self.data);
    }

    /// Checks whether the set contains a specific module.
    pub fn queryModule(
        self: LoadingSet,
        module: [:0]const u8,
    ) bool {
        return self.vtable.query_module(self.data, module);
    }

    /// Checks whether the set contains a specific symbol.
    pub fn querySymbol(
        self: LoadingSet,
        name: [:0]const u8,
        namespace: [:0]const u8,
        version: Version,
    ) bool {
        return self.vtable.query_symbol(
            self.data,
            name.ptr,
            namespace.ptr,
            version.intoC(),
        );
    }

    /// Adds a status callback to the set.
    ///
    /// Adds a callback to report a successful or failed loading of a module. The success path will
    /// be called if the set was able to load all requested modules, whereas the error path will be
    /// called immediately after the failed loading of the module. Since the module set can be in a
    /// partially loaded state at the time of calling this function, the error path may be invoked
    /// immediately. The callbacks will be provided with a user-specified data pointer, which they
    /// are in charge of cleaning up. If an error occurs during the execution of the function, it
    /// will invoke the optional `on_abort` function. If the requested module does not exist, the
    /// function will return an error.
    pub fn addCallback(
        self: LoadingSet,
        module: [:0]const u8,
        obj: anytype,
        comptime callback: fn (
            status: union(enum) { ok: *const Info, err: *const Export, abort },
            data: @TypeOf(obj),
        ) void,
        comptime on_abort: ?fn (data: @TypeOf(obj)) void,
    ) ctx.Error!void {
        const Ptr = @TypeOf(obj);
        std.debug.assert(@typeInfo(Ptr) == .pointer);
        std.debug.assert(@typeInfo(Ptr).pointer.size == .one);
        const Callbacks = struct {
            fn onOk(info: *const Info, data: ?*anyopaque) callconv(.c) void {
                const o: Ptr = @ptrCast(@alignCast(@constCast(data)));
                callback(.{ .ok = info }, o);
            }
            fn onErr(mod: *const Export, data: ?*anyopaque) callconv(.c) void {
                const o: Ptr = @ptrCast(@alignCast(@constCast(data)));
                callback(.{ .err = mod }, o);
            }
            fn onAbort(data: ?*anyopaque) callconv(.c) void {
                if (on_abort) |f| {
                    const o: Ptr = @ptrCast(@alignCast(@constCast(data)));
                    f(o);
                }
            }
        };
        return self.addCallbackCustom(
            module.ptr,
            @constCast(obj),
            Callbacks.onOk,
            Callbacks.onErr,
            if (on_abort) Callbacks.onAbort else null,
        );
    }

    /// Adds a status callback to the set.
    ///
    /// Adds a callback to report a successful or failed loading of a module. The success path will
    /// be called if the set was able to load all requested modules, whereas the error path will be
    /// called immediately after the failed loading of the module. Since the module set can be in a
    /// partially loaded state at the time of calling this function, the error path may be invoked
    /// immediately. The callbacks will be provided with a user-specified data pointer, which they
    /// are in charge of cleaning up. If an error occurs during the execution of the function, it
    /// will invoke the optional `on_abort` function. If the requested module does not exist, the
    /// function will return an error.
    pub fn addCallbackCustom(
        self: LoadingSet,
        module: [*:0]const u8,
        data: ?*anyopaque,
        on_success: *const fn (info: *const Info, data: ?*anyopaque) callconv(.c) void,
        on_error: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) void,
        on_abort: ?*const fn (data: ?*anyopaque) callconv(.c) void,
    ) ctx.Error!void {
        try self.vtable.add_callback(
            self.data,
            module,
            on_success,
            on_error,
            on_abort,
            data,
        ).intoErrorUnion();
    }

    /// Adds a module to the set.
    ///
    /// Adds a module to the set, so that it may be loaded by a future call to `commit`. Trying to
    /// include an invalid module, a module with duplicate exports or duplicate name will result in
    /// an error. Unlike `addModulesFromPath`, this function allows for the loading of dynamic
    /// modules, i.e. modules that are created at runtime, like non-native modules, which may
    /// require a runtime to be executed in. The new module inherits a strong reference to the same
    /// binary as the caller's module.
    ///
    /// Note that the new module is not setup to automatically depend on the owner, but may prevent
    /// it from being unloaded while the set exists.
    pub fn addModule(
        self: LoadingSet,
        owner: *const OpaqueInstance,
        module: *const Export,
    ) ctx.Error!void {
        try self.vtable.add_module(self.data, owner, module).intoErrorUnion();
    }

    /// Adds modules to the set.
    ///
    /// Opens up a module binary to select which modules to load. If the path points to a file, the
    /// function will try to load the file as a binary, whereas, if it points to a directory, it
    /// will try to load a file named `module.fimo_module` contained in the directory. Each
    /// exported module is then passed to the filter, along with the provided data, which can then
    /// filter which modules to load. This function may skip invalid module exports. Trying to
    /// include a module with duplicate exports or duplicate name will result in an error. This
    /// function signals an error, if the binary does not contain the symbols necessary to query
    /// the exported modules, but does not return an error, if it does not export any modules. The
    /// necessary symbols are set up automatically, if the binary was linked with the fimo library.
    /// In case of an error, no modules are appended to the set.
    pub fn addModulesFromPath(
        self: LoadingSet,
        p: paths.Path,
        context: anytype,
        comptime filter: fn (module: *const Export, data: @TypeOf(context)) LoadingSet.FilterRequest,
        comptime filter_deinit: ?fn (data: @TypeOf(context)) void,
    ) ctx.Error!void {
        const Context = @TypeOf(context);
        const Callbacks = struct {
            fn f(module: *const Export, data: ?*anyopaque) callconv(.c) LoadingSet.FilterRequest {
                const context_: Context = if (comptime @typeInfo(Context) == .pointer)
                    @ptrCast(@alignCast(@constCast(data)))
                else
                    @as(*const Context, @ptrCast(@alignCast(data))).*;
                return filter(module, context_);
            }
            fn deinit(data: ?*anyopaque) callconv(.c) void {
                if (filter_deinit) {
                    const context_: Context = if (comptime @typeInfo(Context) == .pointer)
                        @ptrCast(@alignCast(@constCast(data)))
                    else
                        @as(*const Context, @ptrCast(@alignCast(data))).*;
                    filter_deinit(context_);
                }
            }
        };
        return self.addModulesFromPathCustom(
            p,
            if (comptime @typeInfo(Context) == .pointer) @constCast(context) else @constCast(&context),
            Callbacks.f,
            if (filter_deinit != null) &Callbacks.deinit else null,
        );
    }

    /// Adds modules to the set.
    ///
    /// Opens up a module binary to select which modules to load. If the path points to a file, the
    /// function will try to load the file as a binary, whereas, if it points to a directory, it
    /// will try to load a file named `module.fimo_module` contained in the directory. Each
    /// exported module is then passed to the filter, along with the provided data, which can then
    /// filter which modules to load. This function may skip invalid module exports. Trying to
    /// include a module with duplicate exports or duplicate name will result in an error. This
    /// function signals an error, if the binary does not contain the symbols necessary to query
    /// the exported modules, but does not return an error, if it does not export any modules. The
    /// necessary symbols are set up automatically, if the binary was linked with the fimo library.
    /// In case of an error, no modules are appended to the set.
    pub fn addModulesFromPathCustom(
        self: LoadingSet,
        p: paths.Path,
        filter_data: ?*anyopaque,
        filter: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) FilterRequest,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
    ) ctx.Error!void {
        try self.vtable.add_modules_from_path(
            self.data,
            p.intoC(),
            filter,
            filter_deinit,
            filter_data,
        ).intoErrorUnion();
    }

    /// Adds modules to the set.
    ///
    /// Iterates over the exported modules of the current binary. Each exported module is then
    /// passed to the filter, along with the provided data, which can then filter which modules to
    /// load. This function may skip invalid module exports. Trying to include a module with
    /// duplicate exports or duplicate name will result in an error. This function signals an
    /// error, if the binary does not contain the symbols necessary to query the exported modules,
    /// but does not return an error, if it does not export any modules. The necessary symbols are
    /// set up automatically, if the binary was linked with the fimo library. In case of an error,
    /// no modules are appended to the set.
    pub fn addModulesFromLocal(
        self: LoadingSet,
        context: anytype,
        comptime filter: fn (module: *const Export, data: @TypeOf(context)) LoadingSet.FilterRequest,
        comptime filter_deinit: ?fn (data: @TypeOf(context)) void,
    ) ctx.Error!void {
        const Context = @TypeOf(context);
        const Callbacks = struct {
            fn f(module: *const Export, data: ?*anyopaque) callconv(.c) LoadingSet.FilterRequest {
                const context_: Context = if (comptime @typeInfo(Context) == .pointer)
                    @ptrCast(@alignCast(@constCast(data)))
                else
                    @as(*const Context, @ptrCast(@alignCast(data))).*;
                return filter(module, context_);
            }
            fn deinit(data: ?*anyopaque) callconv(.c) void {
                if (filter_deinit) {
                    const context_: Context = if (comptime @typeInfo(Context) == .pointer)
                        @ptrCast(@alignCast(@constCast(data)))
                    else
                        @as(*const Context, @ptrCast(@alignCast(data))).*;
                    filter_deinit(context_);
                }
            }
        };
        return self.addModulesFromLocalCustom(
            if (comptime @typeInfo(Context) == .pointer) @constCast(context) else @constCast(&context),
            Callbacks.f,
            if (filter_deinit != null) &Callbacks.deinit else null,
        );
    }

    /// Adds modules to the set.
    ///
    /// Iterates over the exported modules of the current binary. Each exported module is then
    /// passed to the filter, along with the provided data, which can then filter which modules to
    /// load. This function may skip invalid module exports. Trying to include a module with
    /// duplicate exports or duplicate name will result in an error. This function signals an
    /// error, if the binary does not contain the symbols necessary to query the exported modules,
    /// but does not return an error, if it does not export any modules. The necessary symbols are
    /// set up automatically, if the binary was linked with the fimo library. In case of an error,
    /// no modules are appended to the set.
    pub fn addModulesFromLocalCustom(
        self: LoadingSet,
        filter_data: ?*anyopaque,
        filter: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) FilterRequest,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
    ) ctx.Error!void {
        try self.vtable.add_modules_from_local(
            self.data,
            filter,
            filter_deinit,
            filter_data,
            exports.ExportIter.fimo_impl_module_export_iterator,
            @ptrCast(&exports.ExportIter.fimo_impl_module_export_iterator),
        ).intoErrorUnion();
    }

    /// Loads the modules contained in the set.
    ///
    /// If the returned future is successfull, the contained modules and their resources are made
    /// available to the remaining modules. Some conditions may hinder the loading of some module,
    /// like missing dependencies, duplicates, and other loading errors. In those cases, the
    /// modules will be skipped without erroring.
    ///
    /// It is possible to submit multiple concurrent commit requests, even from the same loading
    /// set. In that case, the requests will be handled atomically, in an unspecified order.
    pub fn commit(self: LoadingSet) EnqueuedFuture(Fallible(void)) {
        return self.vtable.commit(self.data);
    }
};

/// Profile of the module subsystem.
///
/// Each profile enables a set of default features.
pub const Profile = enum(i32) {
    release,
    dev,
    _,
};

/// Optional features recognized by the module subsystem.
///
/// Some features may be mutually exclusive.
pub const FeatureTag = enum(u16) {
    _,
};

/// Request for an optional feature.
pub const FeatureRequest = extern struct {
    tag: FeatureTag,
    flag: enum(u16) { required, on, off },
};

/// Status of an optional feature.
pub const FeatureStatus = extern struct {
    tag: FeatureTag,
    flag: enum(u16) { on, off },
};

/// Configuration for the module subsystem.
pub const Config = extern struct {
    id: ctx.ConfigId = .modules,
    /// Feature profile of the subsystem.
    profile: Profile = switch (builtin.mode) {
        .Debug => .dev,
        .ReleaseSafe, .ReleaseFast, .ReleaseSmall => .release,
    },
    /// List of optional feature requests.
    features: ?[*]const FeatureRequest = null,
    /// Number of optional feature requests.
    feature_count: usize = 0,
};

/// VTable of the module subsystem.
///
/// Changing the VTable is a breaking change.
pub const VTable = extern struct {
    profile: *const fn () callconv(.c) Profile,
    features: *const fn (features: *?[*]const FeatureStatus) callconv(.c) usize,
    root_module_new: *const fn (instance: **const RootInstance) callconv(.c) ctx.Status,
    set_new: *const fn (fut: *LoadingSet) callconv(.c) ctx.Status,
    find_by_name: *const fn (name: [*:0]const u8, info: **const Info) callconv(.c) ctx.Status,
    find_by_symbol: *const fn (
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: Version.CVersion,
        info: **const Info,
    ) callconv(.c) ctx.Status,
    namespace_exists: *const fn (namespace: [*:0]const u8, exists: *bool) callconv(.c) ctx.Status,
    prune_instances: *const fn () callconv(.c) ctx.Status,
    query_parameter: *const fn (
        module: [*:0]const u8,
        parameter: [*:0]const u8,
        type: *ParameterType,
        read_group: *ParameterAccessGroup,
        write_group: *ParameterAccessGroup,
    ) callconv(.c) ctx.Status,
    read_parameter: *const fn (
        value: *anyopaque,
        type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) ctx.Status,
    write_parameter: *const fn (
        value: *const anyopaque,
        type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) ctx.Status,
};

/// Returns the active profile of the module subsystem.
pub fn profile() Profile {
    const handle = ctx.Handle.getHandle();
    return handle.modules_v0.profile();
}

/// Returns the status of all features known to the subsystem.
pub fn features() []const FeatureStatus {
    var ptr: ?[*]const FeatureStatus = undefined;
    const handle = ctx.Handle.getHandle();
    const len = handle.modules_v0.features(&ptr);
    if (ptr) |p| return p[0..len];
    return &.{};
}

/// Checks for the presence of a namespace in the module subsystem.
///
/// A namespace exists, if at least one loaded module exports one symbol in said namespace.
pub fn namespaceExists(namespace: [:0]const u8) ctx.Error!bool {
    var exists: bool = undefined;
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.namespace_exists(namespace.ptr, &exists).intoErrorUnion();
    return exists;
}

/// Marks all instances as unloadable.
///
/// Tries to unload all instances that are not referenced by any other modules. If the instance is
/// still referenced, this will mark the instance as unloadable and enqueue it for unloading.
pub fn pruneInstances() ctx.Error!void {
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.prune_instances().intoErrorUnion();
}

/// Queries the info of a module parameter.
///
/// This function can be used to query the datatype, the read access, and the write access of a
/// module parameter. This function fails, if the parameter can not be found.
pub fn queryParameter(module: [:0]const u8, parameter: [:0]const u8) ctx!OpaqueParameter.Info {
    var tag: ParameterType = undefined;
    var read_group: ParameterAccessGroup = undefined;
    var write_group: ParameterAccessGroup = undefined;
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.query_parameter(
        module.ptr,
        parameter.ptr,
        &tag,
        &read_group,
        &write_group,
    ).intoErrorUnion();
    return .{
        .tag = tag,
        .read_group = read_group,
        .write_group = write_group,
    };
}

/// Reads a module parameter with public read access.
///
/// Reads the value of a module parameter with public read access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow reading with a public access.
pub fn readParameter(comptime T: type, module: [:0]const u8, parameter: [:0]const u8) ctx.Error!T {
    std.debug.assert(std.mem.indexOfScalar(
        u16,
        &.{ 8, 16, 32, 64 },
        @typeInfo(T).int.bits,
    ) != null);
    var value: T = undefined;
    const value_type: ParameterType = switch (comptime @typeInfo(T).int.bits) {
        8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
        16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
        32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
        64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
    };
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.read_parameter(
        &value,
        value_type,
        module.ptr,
        parameter.ptr,
    ).intoErrorUnion();
    return value;
}

/// Sets a module parameter with public write access.
///
/// Sets the value of a module parameter with public write access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow writing with a public access.
pub fn writeParameter(
    comptime T: type,
    value: T,
    module: [:0]const u8,
    parameter: [:0]const u8,
) ctx.Error!void {
    std.debug.assert(std.mem.indexOfScalar(
        u16,
        &.{ 8, 16, 32, 64 },
        @typeInfo(T).int.bits,
    ) != null);
    const value_type: ParameterType = switch (comptime @typeInfo(T).int.bits) {
        8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
        16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
        32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
        64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
    };
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.write_parameter(
        &value,
        value_type,
        module.ptr,
        parameter.ptr,
    ).intoErrorUnion();
}
