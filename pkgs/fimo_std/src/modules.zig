//! Modules subsystem of the context.
//!
//! Requires an initialized global context.
const std = @import("std");
const builtin = @import("builtin");

const ctx = @import("ctx.zig");
pub const exports = @import("modules/exports.zig");
pub const Export = exports.Export;
pub const Module = exports.Module;
pub const ModuleBundle = exports.ModuleBundle;
const paths = @import("paths.zig");
const Path = paths.Path;
const tasks = @import("tasks.zig");
const Waker = tasks.Waker;
const Poll = tasks.Poll;
const OpaqueFuture = tasks.OpaqueFuture;
const Fallible = tasks.Fallible;
const utils = @import("utils.zig");
const SliceConst = utils.SliceConst;
const Version = @import("Version.zig");

/// Data type of a module parameter.
pub const ParamTag = enum(i32) {
    u8 = 0,
    u16 = 1,
    u32 = 2,
    u64 = 3,
    i8 = 4,
    i16 = 5,
    i32 = 6,
    i64 = 7,
    _,
};

/// Access group for a module parameter.
pub const ParamAccessGroup = enum(i32) {
    /// Parameter can be accessed publicly.
    public = 0,
    /// Parameter can be accessed from dependent modules.
    dependency = 1,
    /// Parameter can only be accessed from the owning module.
    private = 2,
};

/// Data type and access groups of a module parameter.
pub const ParamInfo = extern struct {
    tag: ParamTag,
    read_group: ParamAccessGroup,
    write_group: ParamAccessGroup,
};

/// Parameter accessor.
pub fn Param(comptime T: type) type {
    std.debug.assert(T == anyopaque or std.mem.indexOfScalar(
        u16,
        &.{ 8, 16, 32, 64 },
        @typeInfo(T).int.bits,
    ) != null);

    return opaque {
        const Self = @This();
        pub const Inner = extern struct {
            tag: *const fn (data: *const @This()) callconv(.c) ParamTag,
            read: *const fn (data: *const @This(), value: *T) callconv(.c) void,
            write: *const fn (data: *@This(), value: *const T) callconv(.c) void,
        };

        /// Returns the value type of the parameter.
        pub fn tag(self: *const Self) ParamTag {
            const inner: *const Inner = @ptrCast(@alignCast(self));
            return inner.type(inner);
        }

        /// Reads the value from the parameter.
        pub fn read(self: *const Self) T {
            var value: T = undefined;
            const inner: *const Inner = @ptrCast(@alignCast(self));
            inner.read(inner, &value);
            return value;
        }

        /// Writes the value into the parameter.
        pub fn write(self: *Self, value: T) void {
            const inner: *Inner = @ptrCast(@alignCast(self));
            inner.write(inner, &value);
        }

        /// Casts the opaque parameter to a typed variant.
        pub fn castFromOpaque(parameter: *OpaqueParam) *Self {
            if (comptime T == void) {
                return parameter;
            } else {
                const expected_tag: ParamTag = switch (comptime @typeInfo(T).int.bits) {
                    8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                    16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                    32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                    64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
                };
                std.debug.assert(parameter.tag() == expected_tag);
                return @ptrCast(parameter);
            }
        }

        /// Casts the opaque parameter to a typed variant.
        pub fn castFromOpaqueConst(parameter: *const OpaqueParam) *const Self {
            if (comptime T == void) {
                return parameter;
            } else {
                const expected_tag: ParamTag = switch (comptime @typeInfo(T).int.bits) {
                    8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                    16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                    32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                    64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
                };
                std.debug.assert(parameter.tag() == expected_tag);
                return @ptrCast(parameter);
            }
        }

        /// Casts the parameter to an opaque parameter.
        pub fn castToOpaque(self: *Self) *OpaqueParam {
            return @ptrCast(self);
        }

        /// Casts the parameter to an opaque parameter.
        pub fn castToOpaqueConst(self: *const Self) *const OpaqueParam {
            return @ptrCast(self);
        }
    };
}

/// A type-erased module parameter.
pub const OpaqueParam = Param(anyopaque);

/// Internal state of a parameter.
pub fn ParamData(comptime T: type) type {
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
            tag: *const fn (data: *anyopaque) callconv(.c) ParamTag,
            read: *const fn (data: *anyopaque, value: *T) callconv(.c) void,
            write: *const fn (data: *anyopaque, value: *const T) callconv(.c) void,
        };

        /// Returns the value type of the parameter.
        pub fn tag(self: @This()) ParamTag {
            return self.vtable.tag(self.data);
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
        pub fn castFromOpaque(parameter: OpaqueParamData) @This() {
            if (comptime T == void) {
                return parameter;
            } else {
                const expected_tag: ParamTag = switch (comptime @typeInfo(T).int.bits) {
                    8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                    16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                    32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                    64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
                };
                std.debug.assert(parameter.tag() == expected_tag);
                return @bitCast(parameter);
            }
        }

        /// Casts the parameter data to an opaque parameter data.
        pub fn castToOpaque(self: @This()) OpaqueParamData {
            return @bitCast(self);
        }
    };
}

/// Untyped state of a parameter.
pub const OpaqueParamData = ParamData(anyopaque);

/// Identifier of a symbol.
pub const SymbolId = extern struct {
    name: SliceConst(u8),
    namespace: SliceConst(u8) = .fromSlice(""),
    version: Version.compat.Version,

    pub fn fromSymbol(symbol: Symbol) SymbolId {
        return .{
            .name = .fromSlice(symbol.name),
            .namespace = .fromSlice(symbol.namespace),
            .version = symbol.version.intoC(),
        };
    }
};

/// Information about a symbol.
pub const Symbol = struct {
    name: []const u8,
    namespace: []const u8 = "",
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

    pub fn intoId(self: Symbol) SymbolId {
        return .fromSymbol(self);
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
                std.debug.assert(symbol.version.sattisfies(sym.version));
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
                        symbol.version.sattisfies(sym.version)) break :blk f.name;
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

/// Shared handle to a loaded instance.
pub const Handle = opaque {
    pub const Inner = extern struct {
        name: SliceConst(u8),
        description: SliceConst(u8),
        author: SliceConst(u8),
        license: SliceConst(u8),
        module_path: paths.compat.Path,
        ref: *const fn (ctx: *Inner) callconv(.c) void,
        unref: *const fn (ctx: *Inner) callconv(.c) void,
        mark_unloadable: *const fn (ctx: *Inner) callconv(.c) void,
        is_loaded: *const fn (ctx: *Inner) callconv(.c) bool,
        try_ref_instance_strong: *const fn (ctx: *Inner) callconv(.c) bool,
        unref_instance_strong: *const fn (ctx: *Inner) callconv(.c) void,
    };

    /// Returns the name of the module.
    pub fn name(self: *Handle) []const u8 {
        const inner: *Inner = @ptrCast(@alignCast(self));
        return inner.name.intoSliceOrEmpty();
    }

    /// Returns the description of the module.
    pub fn description(self: *Handle) []const u8 {
        const inner: *Inner = @ptrCast(@alignCast(self));
        return inner.description.intoSliceOrEmpty();
    }

    /// Returns the author of the module.
    pub fn author(self: *Handle) []const u8 {
        const inner: *Inner = @ptrCast(@alignCast(self));
        return inner.author.intoSliceOrEmpty();
    }

    /// Returns the license of the module.
    pub fn license(self: *Handle) []const u8 {
        const inner: *Inner = @ptrCast(@alignCast(self));
        return inner.license.intoSliceOrEmpty();
    }

    /// Returns the path of the module.
    pub fn modulePath(self: *Handle) Path {
        const inner: *Inner = @ptrCast(@alignCast(self));
        return .initC(inner.module_path);
    }

    /// Increases the reference count of the handle.
    pub fn ref(self: *Handle) void {
        const inner: *Inner = @ptrCast(@alignCast(self));
        inner.ref(inner);
    }

    /// Decreases the reference count of the handle.
    pub fn unref(self: *Handle) void {
        const inner: *Inner = @ptrCast(@alignCast(self));
        inner.unref(inner);
    }

    /// Signals that the owning instance may be unloaded.
    ///
    /// The instance will be unloaded once it is no longer actively used by another instance.
    pub fn markUnloadable(self: *Handle) void {
        const inner: *Inner = @ptrCast(@alignCast(self));
        inner.mark_unloadable(inner);
    }

    /// Returns whether the owning instance is still loaded.
    pub fn isLoaded(self: *Handle) bool {
        const inner: *Inner = @ptrCast(@alignCast(self));
        return inner.is_loaded(inner);
    }

    /// Tries to increase the strong reference count of the owning instance.
    ///
    /// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
    /// between modules, without registering the dependency with the subsystem.
    ///
    /// NOTE: Use with caution. Prefer structuring your code in a way that does not necessitate
    /// dependency tracking.
    pub fn tryRefInstanceStrong(self: *Handle) bool {
        const inner: *Inner = @ptrCast(@alignCast(self));
        return inner.try_ref_instance_strong(inner);
    }

    /// Decreases the strong reference count of the owning instance.
    ///
    /// May only be called after the reference count of the instance has been increased.
    pub fn unrefInstanceStrong(self: *Handle) void {
        const inner: *Inner = @ptrCast(@alignCast(self));
        inner.unref_instance_strong(inner);
    }

    /// Searches for a module by its name.
    ///
    /// Queries a module by its unique name.
    /// The returned handle will have its reference count increased.
    pub fn findByName(module: []const u8) ctx.Error!*Handle {
        var h: *Handle = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.handle_find_by_name(&h, .fromSlice(module)).intoErrorUnion();
        return h;
    }

    /// Searches for a module by a symbol it exports.
    ///
    /// Queries the module that exported the specified symbol.
    /// The returned handle will have its reference count increased.
    pub fn findBySymbol(symbol: SymbolId) ctx.Error!*Handle {
        var h: *Handle = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.handle_find_by_symbol(&h, symbol).intoErrorUnion();
        return h;
    }
};

pub const Dependency = enum(i32) {
    none = 0,
    static = 1,
    dynamic = 2,
};

/// Configuration for an instance.
pub const InstanceConfig = struct {
    /// Type of the parameters table.
    ParametersType: type = anyopaque,
    /// Type of the resources table.
    ResourcesType: type = anyopaque,
    /// Type of the imports table.
    ImportsType: type = anyopaque,
    /// Type of the exports table.
    ExportsType: type = anyopaque,
    /// Type of the instance state.
    StateType: type = anyopaque,
    module_export: ?*const anyopaque = null,
    provider: ?fn (instance: anytype, comptime symbol: Symbol) *const anyopaque = null,
};

pub fn Instance(comptime config: InstanceConfig) type {
    return opaque {
        pub const Parameters = config.ParametersType;
        pub const Resources = config.ResourcesType;
        pub const Imports = config.ImportsType;
        pub const Exports = config.ExportsType;
        pub const State = config.StateType;

        pub const Inner = extern struct {
            pub const VTable = extern struct {
                ref: *const fn (ctx: *Inner) callconv(.c) void,
                unref: *const fn (ctx: *Inner) callconv(.c) void,
                query_namespace: *const fn (ctx: *Inner, ns: SliceConst(u8), dependency: *Dependency) callconv(.c) ctx.Status,
                add_namespace: *const fn (ctx: *Inner, ns: SliceConst(u8)) callconv(.c) ctx.Status,
                remove_namespace: *const fn (ctx: *Inner, ns: SliceConst(u8)) callconv(.c) ctx.Status,
                query_dependency: *const fn (ctx: *Inner, handle: *Handle, dependency: *Dependency) callconv(.c) ctx.Status,
                add_dependency: *const fn (ctx: *Inner, handle: *Handle) callconv(.c) ctx.Status,
                remove_dependency: *const fn (ctx: *Inner, handle: *Handle) callconv(.c) ctx.Status,
                load_symbol: *const fn (ctx: *Inner, symbol: SymbolId, value: **const anyopaque) callconv(.c) ctx.Status,
                read_parameter: *const fn (ctx: *Inner, tag: ParamTag, module: SliceConst(u8), parameter: SliceConst(u8), value: *anyopaque) callconv(.c) ctx.Status,
                write_parameter: *const fn (ctx: *Inner, tag: ParamTag, module: SliceConst(u8), parameter: SliceConst(u8), value: *const anyopaque) callconv(.c) ctx.Status,
            };

            // Note: Some fields may be undefined.
            vtable: *const Inner.VTable,
            parameters: if (Parameters == anyopaque or @sizeOf(Parameters) == 0) ?*const Parameters else *const Parameters,
            resources: if (Resources == anyopaque or @sizeOf(Resources) == 0) ?*const Resources else *const Resources,
            imports: if (Imports == anyopaque or @sizeOf(Imports) == 0) ?*const Imports else *const Imports,
            exports: if (Exports == anyopaque or @sizeOf(Exports) == 0) ?*const Exports else *const Exports,
            handle: *Handle,
            ctx_handle: *ctx.Handle,
            state: *State,
        };

        const Self = @This();

        pub const module_export = if (config.module_export) |e| @as(*const Export, @ptrCast(@alignCast(e))) else {};
        pub const fimo_module_instance_marker: void = {};

        /// Provides a pointer to the requested symbol.
        pub fn provideSymbol(self: *Self, comptime symbol: Symbol) *const symbol.T {
            if (config.provider) |provider|
                return @ptrCast(@alignCast(provider(self, symbol)))
            else
                @compileError("instance configuration does not specify a symbol provider function.");
        }

        /// Returns the parameter table of the module.
        pub fn parameters(self: *Self) *const Parameters {
            const inner: *Inner = @ptrCast(@alignCast(self));
            if (comptime Parameters != anyopaque and @sizeOf(Parameters) != 0) {
                return inner.parameters;
            } else {
                return inner.parameters orelse if (Parameters == anyopaque) @ptrFromInt(1) else &Parameters{};
            }
        }

        /// Returns the resource table of the module.
        pub fn resources(self: *Self) *const Resources {
            const inner: *Inner = @ptrCast(@alignCast(self));
            if (comptime Resources != anyopaque and @sizeOf(Resources) != 0) {
                return inner.resources;
            } else {
                return inner.resources orelse if (Resources == anyopaque) @ptrFromInt(1) else &Resources{};
            }
        }

        /// Returns the import table of the module.
        pub fn imports(self: *Self) *const Imports {
            const inner: *Inner = @ptrCast(@alignCast(self));
            if (comptime Imports != anyopaque and @sizeOf(Imports) != 0) {
                return inner.imports;
            } else {
                return inner.imports orelse if (Imports == anyopaque) @ptrFromInt(1) else &Imports{};
            }
        }

        /// Returns the exports table of the module.
        ///
        /// Exports are ordered the in declaration order of the module export.
        /// The exports are populated in declaration order and depopulated in reverse declaration order.
        pub fn exports(self: *Self) *const Exports {
            const inner: *Inner = @ptrCast(@alignCast(self));
            if (comptime Exports != anyopaque and @sizeOf(Exports) != 0) {
                return inner.exports;
            } else {
                return inner.exports orelse if (Exports == anyopaque) @ptrFromInt(1) else &Exports{};
            }
        }

        /// Returns the shared handle of the module.
        ///
        /// NOTE: The reference count is not modified.
        pub fn handle(self: *Self) *Handle {
            const inner: *Inner = @ptrCast(@alignCast(self));
            return inner.handle;
        }

        /// Returns the handle to the context.
        pub fn ctxHandle(self: *Self) *ctx.Handle {
            const inner: *Inner = @ptrCast(@alignCast(self));
            return inner.ctx_handle;
        }

        /// Returns the state of the module.
        ///
        /// NOTE: Return value is undefined until after the execution of the module constructor and
        /// after the execution of the module destructor.
        pub fn state(self: *Self) *State {
            const inner: *Inner = @ptrCast(@alignCast(self));
            if (comptime State != anyopaque and @sizeOf(State) != 0) {
                return inner.state;
            } else {
                return inner.state orelse if (State == anyopaque) @ptrFromInt(1) else &State{};
            }
        }

        /// Increases the strong reference count of the module instance.
        ///
        /// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
        /// between modules, without registering the dependency with the subsystem.
        ///
        /// NOTE: Use with caution. Prefer structuring your code in a way that does not necessitate
        /// dependency tracking.
        pub fn ref(self: *Self) void {
            const inner: *Inner = @ptrCast(@alignCast(self));
            inner.vtable.ref(inner);
        }

        /// Decreases the strong reference count of the module instance.
        ///
        /// May only be called after the reference count has been increased.
        pub fn unref(self: *Self) void {
            const inner: *Inner = @ptrCast(@alignCast(self));
            inner.vtable.unref(inner);
        }

        /// Checks the status of a namespace from the view of the module.
        ///
        /// Checks if the module includes the namespace. In that case, the module is allowed access
        /// to the symbols in the namespace. Additionally, this function also queries whether the
        /// include is static, i.e., it was specified by the module at load time.
        pub fn queryNamespace(self: *Self, ns: []const u8) ctx.Error!Dependency {
            var dependency: Dependency = undefined;
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.query_namespace(inner, .fromSlice(ns), &dependency).intoErrorUnion();
            return dependency;
        }

        /// Adds a namespace dependency to the module.
        ///
        /// Once added, the module gains access to the symbols of its dependencies that are
        /// exposed in said namespace. A namespace can not be added multiple times.
        pub fn addNamespace(self: *Self, ns: []const u8) ctx.Error!void {
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.add_namespace(inner, .fromSlice(ns)).intoErrorUnion();
        }

        /// Removes a namespace dependency from the module.
        ///
        /// Once excluded, the caller guarantees to relinquish access to the symbols contained in
        /// said namespace. It is only possible to exclude namespaces that were manually added,
        /// whereas static namespace dependencies remain valid until the module is unloaded.
        pub fn removeNamespace(self: *Self, ns: []const u8) ctx.Error!void {
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.remove_namespace(inner, .fromSlice(ns)).intoErrorUnion();
        }

        /// Checks if a module depends on another module.
        ///
        /// Checks if the specified module is a dependency of the current instance. In that case
        /// the instance is allowed to access the symbols exported by the module. Additionally,
        /// this function also queries whether the dependency is static, i.e., the dependency was
        /// specified by the module at load time.
        pub fn queryDependency(self: *Self, h: *Handle) ctx.Error!Dependency {
            var dependency: Dependency = undefined;
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.query_dependency(inner, h, &dependency).intoErrorUnion();
            return dependency;
        }

        /// Adds another module as a dependency.
        ///
        /// After adding a module as a dependency, the module is allowed access to the symbols
        /// and protected parameters of said dependency. Trying to adding a dependency to a module
        /// that is already a dependency, or to a module that would result in a circular dependency
        /// will result in an error.
        pub fn addDependency(self: *Self, h: *Handle) ctx.Error!void {
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.add_dependency(inner, h).intoErrorUnion();
        }

        /// Removes a module as a dependency.
        ///
        /// By removing a module as a dependency, the caller ensures that it does not own any
        /// references to resources originating from the former dependency, and allows for the
        /// unloading of the module. A module can only relinquish dependencies to modules that were
        /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
        pub fn removeDependency(self: *Self, h: *Handle) ctx.Error!void {
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.remove_dependency(inner, h).intoErrorUnion();
        }

        /// Loads a group of symbols from the module subsystem.
        ///
        /// Is equivalent to calling `loadSymbol` for each symbol of the group independently.
        pub fn loadSymbolGroup(self: *Self, comptime symbols: anytype) ctx.Error!SymbolGroup(symbols) {
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
        pub fn loadSymbol(self: *Self, comptime symbol: Symbol) ctx.Error!SymbolWrapper(symbol) {
            const sym = symbol.intoId();
            const value = try self.loadSymbolRaw(sym);
            return .{ .value = @ptrCast(@alignCast(value)) };
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
        /// loading optional symbols, or for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until the module relinquishes the
        /// dependency to the module that exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbolRaw(self: *Self, symbol: SymbolId) ctx.Error!*const anyopaque {
            var value: *const anyopaque = undefined;
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.load_symbol(inner, symbol, &value).intoErrorUnion();
            return value;
        }

        /// Reads a module parameter with dependency read access.
        ///
        /// Reads the value of a module parameter with dependency read access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow reading with a
        /// dependency access.
        pub fn readParameter(self: *Self, comptime T: type, module: []const u8, parameter: []const u8) ctx.Error!T {
            std.debug.assert(std.mem.indexOfScalar(u16, &.{ 8, 16, 32, 64 }, @typeInfo(T).int.bits) != null);
            var value: T = undefined;
            const value_tag: ParamTag = switch (comptime @typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
            };
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.read_parameter(inner, value_tag, .fromSlice(module), .fromSlice(parameter), &value).intoErrorUnion();
            return value;
        }

        /// Sets a module parameter with dependency write access.
        ///
        /// Sets the value of a module parameter with dependency write access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow writing with a
        /// dependency access.
        pub fn writeParameter(self: *Self, comptime T: type, value: T, module: []const u8, parameter: []const u8) ctx.Error!void {
            const value_tag: ParamTag = switch (comptime @typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
                16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
                32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
                64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
            };
            const inner: *Inner = @ptrCast(@alignCast(self));
            try inner.vtable.write_parameter(inner, value_tag, .fromSlice(module), .fromSlice(parameter), &value).intoErrorUnion();
        }
    };
}

/// Type of an opaque module instance.
pub const OpaqueInstance = Instance(.{});

/// A root instance is a dynamically created "fake" module, which can not be depended from
/// by any other module. By their nature, root instances can not export any symbols, but can
/// depend on other modules and import their symbols dynamically.
pub const RootInstance = opaque {
    /// Constructs a new root instance.
    pub fn init() ctx.Error!*RootInstance {
        var instance: *RootInstance = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.root_instance_init(&instance).intoErrorUnion();
        return instance;
    }

    /// Destroys the root module.
    ///
    /// The handle may not be used afterwards.
    pub fn deinit(self: *RootInstance) void {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        inner.handle().markUnloadable();
    }

    /// Checks the status of a namespace from the view of the module.
    ///
    /// Checks if the module includes the namespace. In that case, the module is allowed access
    /// to the symbols in the namespace. Additionally, this function also queries whether the
    /// include is static, i.e., it was specified by the module at load time.
    pub fn queryNamespace(self: *RootInstance, ns: []const u8) ctx.Error!Dependency {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.queryNamespace(ns);
    }

    /// Adds a namespace dependency to the module.
    ///
    /// Once added, the module gains access to the symbols of its dependencies that are
    /// exposed in said namespace. A namespace can not be added multiple times.
    pub fn addNamespace(self: *RootInstance, ns: []const u8) ctx.Error!void {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.addNamespace(ns);
    }

    /// Removes a namespace dependency from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in
    /// said namespace. It is only possible to exclude namespaces that were manually added,
    /// whereas static namespace dependencies remain valid until the module is unloaded.
    pub fn removeNamespace(self: *RootInstance, ns: []const u8) ctx.Error!void {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.removeNamespace(ns);
    }

    /// Checks if a module depends on another module.
    ///
    /// Checks if the specified module is a dependency of the current instance. In that case
    /// the instance is allowed to access the symbols exported by the module. Additionally,
    /// this function also queries whether the dependency is static, i.e., the dependency was
    /// specified by the module at load time.
    pub fn queryDependency(self: *RootInstance, handle: *Handle) ctx.Error!Dependency {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.queryDependency(handle);
    }

    /// Adds another module as a dependency.
    ///
    /// After adding a module as a dependency, the module is allowed access to the symbols
    /// and protected parameters of said dependency. Trying to adding a dependency to a module
    /// that is already a dependency, or to a module that would result in a circular dependency
    /// will result in an error.
    pub fn addDependency(self: *RootInstance, handle: *Handle) ctx.Error!void {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.addDependency(handle);
    }

    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller ensures that it does not own any
    /// references to resources originating from the former dependency, and allows for the
    /// unloading of the module. A module can only relinquish dependencies to modules that were
    /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
    pub fn removeDependency(self: *RootInstance, handle: *Handle) ctx.Error!void {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.removeDependency(handle);
    }

    /// Loads a group of symbols from the module subsystem.
    ///
    /// Is equivalent to calling `loadSymbol` for each symbol of the group independently.
    pub fn loadSymbolGroup(self: *RootInstance, comptime symbols: anytype) ctx.Error!SymbolGroup(symbols) {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.loadSymbolGroup(symbols);
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until the module relinquishes the
    /// dependency to the module that exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    pub fn loadSymbol(self: *RootInstance, comptime symbol: Symbol) ctx.Error!SymbolWrapper(symbol) {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.loadSymbol(symbol);
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until the module relinquishes the
    /// dependency to the module that exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    pub fn loadSymbolRaw(self: *RootInstance, symbol: SymbolId) ctx.Error!*const anyopaque {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.loadSymbolRaw(symbol);
    }

    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read access. The operation fails,
    /// if the parameter does not exist, or if the parameter does not allow reading with a
    /// dependency access.
    pub fn readParameter(self: *RootInstance, comptime T: type, module: []const u8, parameter: []const u8) ctx.Error!T {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.readParameter(T, module, parameter);
    }

    /// Sets a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write access. The operation fails,
    /// if the parameter does not exist, or if the parameter does not allow writing with a
    /// dependency access.
    pub fn writeParameter(self: *RootInstance, comptime T: type, value: T, module: []const u8, parameter: []const u8) ctx.Error!void {
        const inner: *OpaqueInstance = @ptrCast(@alignCast(self));
        return inner.writeParameter(T, value, module, parameter);
    }
};

/// Handle to a module loader.
///
/// Modules can only be loaded after all of their dependencies have been resolved uniquely.
/// A module loader batches the loading of multiple modules, procedurally determining an appropriate
/// loading order for as many modules as possible.
pub const Loader = opaque {
    /// Constructs a new loader.
    pub fn init() ctx.Error!*Loader {
        var loader: *Loader = undefined;
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.loader_init(&loader).intoErrorUnion();
        return loader;
    }

    /// Drops the loader.
    ///
    /// Scheduled operations will be completed, but the caller invalidates their reference to the handle.
    pub fn deinit(self: *Loader) void {
        const handle = ctx.Handle.getHandle();
        handle.modules_v0.loader_deinit(self);
    }

    /// Checks whether the loader contains some module.
    pub fn containsModule(self: *Loader, module: []const u8) bool {
        const handle = ctx.Handle.getHandle();
        return handle.modules_v0.loader_contains_module(self, .fromSlice(module));
    }

    /// Checks whether the loader contains some symbol.
    pub fn containsSymbol(self: *Loader, symbol: Symbol) bool {
        return self.containsSymbolRaw(symbol.name, symbol.namespace, symbol.version);
    }

    /// Checks whether the loader contains some symbol.
    pub fn containsSymbolRaw(self: *Loader, symbol: SymbolId) bool {
        const handle = ctx.Handle.getHandle();
        return handle.modules_v0.loader_contains_symbol(self, symbol);
    }

    /// Resolved result of `pollModule`.
    pub const ResolvedModule = extern struct {
        /// Handle to the loaded instance.
        ///
        /// Must be released.
        handle: ?*Handle,
        export_handle: *const Export,
    };

    /// Polls the loader for the state of the specified module.
    ///
    /// If the module has not been processed at the time of calling, the waker will be
    /// signaled once the function can be polled again.
    pub fn pollModule(self: *Loader, waker: Waker, module: []const u8) Poll(ctx.Error!ResolvedModule) {
        var result: Fallible(ResolvedModule) = undefined;
        const handle = ctx.Handle.getHandle();
        if (handle.modules_v0.loader_poll_module(self, waker, .fromSlice(module), &result)) {
            return .{ .ready = result.unwrap() };
        }
        return .pending;
    }

    /// Adds a module to the loader.
    ///
    /// Adds a module to the loader, so that it may be loaded by a future call to `commit`. Trying to
    /// include an invalid module, a module with duplicate exports or duplicate name will result in
    /// an error. This function allows for the loading of dynamic modules, i.e. modules that are
    /// created at runtime, like non-native modules, which may require a runtime to be executed in.
    /// The new module inherits a strong reference to the same binary as the caller's module.
    ///
    /// Note that the new module is not setup to automatically depend on the owner, but may prevent
    /// it from being unloaded while the loader exists.
    pub fn addModule(self: *Loader, owner: *OpaqueInstance, module: *const Export) ctx.Error!void {
        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.loader_add_module(self, owner, module).intoErrorUnion();
    }

    /// Operation of the filter function.
    pub const FilterRequest = enum(i32) {
        skip = 0,
        load = 1,
    };

    /// Adds modules to the loader.
    ///
    /// Opens up a module binary to select which modules to load.
    /// If the path points to a file, the function will try to load the file.
    /// If it points to a directory, it will search for a file named `module.fimo_module` in the same
    /// directory.
    ///
    /// The filter function can determine which modules to load.
    /// Trying to load a module with duplicate exports or duplicate name will result in an error.
    /// Invalid modules may not get passed to the filter function, and should therefore not be utilized
    /// to list the modules contained in a binary.
    ///
    /// This function returns an error, if the binary does not contain the symbols necessary to query
    /// the exported modules, but does not return an error, if it does not export any modules.
    pub fn addModulesFromPath(
        self: *Loader,
        path: Path,
        context: anytype,
        filter: fn (context: @TypeOf(context), module: *const Export) FilterRequest,
    ) ctx.Error!void {
        const Wrapper = struct {
            fn f(ctx_ptr: ?*anyopaque, module: *const Export) callconv(.c) FilterRequest {
                const f_ctx = @as(*@TypeOf(context), @ptrCast(ctx_ptr)).*;
                return filter(f_ctx, module);
            }
        };

        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.loader_add_modules_from_path(
            self,
            path.intoC(),
            @constCast(&context),
            &Wrapper.f,
        ).intoErrorUnion();
    }

    /// Adds modules to the loader.
    ///
    /// Iterates over the exported modules of the current binary.
    ///
    /// The filter function can determine which modules to load.
    /// Trying to load a module with duplicate exports or duplicate name will result in an error.
    /// Invalid modules may not get passed to the filter function, and should therefore not be utilized
    /// to list the modules contained in a binary.
    pub fn addModulesFromIter(
        self: *Loader,
        context: anytype,
        filter: fn (context: @TypeOf(context), module: *const Export) FilterRequest,
    ) ctx.Error!void {
        const Wrapper = struct {
            fn f(ctx_ptr: ?*anyopaque, module: *const Export) callconv(.c) FilterRequest {
                const f_ctx = @as(*@TypeOf(context), @ptrCast(ctx_ptr)).*;
                return filter(f_ctx, module);
            }
        };

        const handle = ctx.Handle.getHandle();
        try handle.modules_v0.loader_add_modules_from_iter(
            self,
            @constCast(&context),
            &Wrapper.f,
            exports.ExportIter.fstd__module_export_iter,
            @ptrCast(&exports.ExportIter.fstd__module_export_iter),
        ).intoErrorUnion();
    }

    /// Loads the modules contained in the loader.
    ///
    /// If the returned future is successfull, the contained modules and their resources are made
    /// available to the remaining modules. Some conditions may hinder the loading of some module,
    /// like missing dependencies, duplicates, and other loading errors. In those cases, the
    /// modules will be skipped without erroring.
    ///
    /// It is possible to submit multiple concurrent commit requests, even from the same  loader.
    /// In that case, the requests will be handled atomically, in an unspecified order.
    pub fn commit(self: *Loader) OpaqueFuture(Fallible(void)) {
        const handle = ctx.Handle.getHandle();
        return handle.modules_v0.loader_commit(self);
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
/// Some features may be mutually exclusive, while other may
/// require additional feature dependencies.
pub const FeatureTag = enum(u16) {
    _,
};

/// Request for an optional feature.
pub const FeatureRequest = extern struct {
    tag: FeatureTag,
    flag: enum(u16) { required = 0, on = 1, off = 2 },
};

/// Status of an optional feature.
pub const FeatureStatus = extern struct {
    tag: FeatureTag,
    flag: enum(u16) { on = 0, off = 1 },
};

/// Configuration for the module subsystem.
pub const Cfg = extern struct {
    cfg: ctx.Cfg = .{ .id = .modules },
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
    features: *const fn () callconv(.c) SliceConst(FeatureStatus),
    root_instance_init: *const fn (instance: **RootInstance) callconv(.c) ctx.Status,
    loader_init: *const fn (loader: **Loader) callconv(.c) ctx.Status,
    loader_deinit: *const fn (loader: *Loader) callconv(.c) void,
    loader_contains_module: *const fn (loader: *Loader, module: SliceConst(u8)) callconv(.c) bool,
    loader_contains_symbol: *const fn (loader: *Loader, symbol: SymbolId) callconv(.c) bool,
    loader_poll_module: *const fn (
        loader: *Loader,
        waker: Waker,
        module: SliceConst(u8),
        result: *Fallible(Loader.ResolvedModule),
    ) callconv(.c) bool,
    loader_add_module: *const fn (
        loader: *Loader,
        owner: *OpaqueInstance,
        module: *const Export,
    ) callconv(.c) ctx.Status,
    loader_add_modules_from_path: *const fn (
        loader: *Loader,
        path: paths.compat.Path,
        context: ?*anyopaque,
        filter_fn: *const fn (
            context: ?*anyopaque,
            module: *const Export,
        ) callconv(.c) Loader.FilterRequest,
    ) callconv(.c) ctx.Status,
    loader_add_modules_from_iter: *const fn (
        loader: *Loader,
        context: ?*anyopaque,
        filter_fn: *const fn (
            context: ?*anyopaque,
            module: *const Export,
        ) callconv(.c) Loader.FilterRequest,
        iterator_fn: *const fn (
            context: ?*anyopaque,
            f: *const fn (context: ?*anyopaque, module: *const Export) callconv(.c) bool,
        ) callconv(.c) void,
        bin_ptr: *const anyopaque,
    ) callconv(.c) ctx.Status,
    loader_commit: *const fn (loader: *Loader) callconv(.c) OpaqueFuture(Fallible(void)),
    handle_find_by_name: *const fn (handle: **Handle, name: SliceConst(u8)) callconv(.c) ctx.Status,
    handle_find_by_symbol: *const fn (handle: **Handle, symbol: SymbolId) callconv(.c) ctx.Status,
    namespace_exists: *const fn (ns: SliceConst(u8)) callconv(.c) bool,
    prune_instances: *const fn () callconv(.c) ctx.Status,
    query_parameter: *const fn (
        module: SliceConst(u8),
        parameter: SliceConst(u8),
        info: *ParamInfo,
    ) callconv(.c) ctx.Status,
    read_parameter: *const fn (
        type: ParamTag,
        module: SliceConst(u8),
        parameter: SliceConst(u8),
        value: *anyopaque,
    ) callconv(.c) ctx.Status,
    write_parameter: *const fn (
        type: ParamTag,
        module: SliceConst(u8),
        parameter: SliceConst(u8),
        value: *const anyopaque,
    ) callconv(.c) ctx.Status,
};

/// Returns the active profile of the module subsystem.
pub fn profile() Profile {
    const handle = ctx.Handle.getHandle();
    return handle.modules_v0.profile();
}

/// Returns the status of all features known to the subsystem.
pub fn features() []const FeatureStatus {
    const handle = ctx.Handle.getHandle();
    return handle.modules_v0.features().intoSliceOrEmpty();
}

/// Checks for the presence of a namespace in the module subsystem.
///
/// A namespace exists, if at least one loaded module exports one symbol in said namespace.
pub fn namespaceExists(namespace: [:0]const u8) bool {
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
pub fn queryParameter(module: []const u8, parameter: []const u8) ctx.Error!ParamInfo {
    var info: ParamInfo = undefined;
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.query_parameter(.fromSlice(module), .fromSlice(parameter), &info).intoErrorUnion();
    return info;
}

/// Reads a module parameter with public read access.
///
/// Reads the value of a module parameter with public read access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow reading with a public access.
pub fn readParameter(comptime T: type, module: []const u8, parameter: []const u8) ctx.Error!T {
    std.debug.assert(std.mem.indexOfScalar(u16, &.{ 8, 16, 32, 64 }, @typeInfo(T).int.bits) != null);
    var value: T = undefined;
    const value_tag: ParamTag = switch (comptime @typeInfo(T).int.bits) {
        8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
        16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
        32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
        64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
    };
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.read_parameter(
        value_tag,
        .fromSlice(module),
        .fromSlice(parameter),
        &value,
    ).intoErrorUnion();
    return value;
}

/// Sets a module parameter with public write access.
///
/// Sets the value of a module parameter with public write access. The operation fails, if the
/// parameter does not exist, or if the parameter does not allow writing with a public access.
pub fn writeParameter(
    comptime T: type,
    module: []const u8,
    parameter: []const u8,
    value: T,
) ctx.Error!void {
    std.debug.assert(std.mem.indexOfScalar(u16, &.{ 8, 16, 32, 64 }, @typeInfo(T).int.bits) != null);
    const value_tag: ParamTag = switch (comptime @typeInfo(T).int.bits) {
        8 => if (@typeInfo(T).int.signedness == .signed) .i8 else .u8,
        16 => if (@typeInfo(T).int.signedness == .signed) .i16 else .u16,
        32 => if (@typeInfo(T).int.signedness == .signed) .i32 else .u32,
        64 => if (@typeInfo(T).int.signedness == .signed) .i64 else .u64,
    };
    const handle = ctx.Handle.getHandle();
    try handle.modules_v0.write_parameter(
        value_tag,
        .fromSlice(module),
        .fromSlice(parameter),
        &value,
    ).intoErrorUnion();
}
