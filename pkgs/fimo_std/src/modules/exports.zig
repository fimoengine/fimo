//! Module export utilities.
const std = @import("std");
const builtin = @import("builtin");

const ctx = @import("../ctx.zig");
const modules = @import("../modules.zig");
const paths = @import("../paths.zig");
const tasks = @import("../tasks.zig");
const Fallible = tasks.Fallible;
const EnqueuedFuture = tasks.EnqueuedFuture;
const Version = @import("../Version.zig");

const ExportsRoot = @This();

/// Declaration of a module parameter.
pub const Parameter = extern struct {
    type: modules.ParameterType,
    read_group: modules.ParameterAccessGroup = .private,
    write_group: modules.ParameterAccessGroup = .private,
    read: ?*const fn (
        data: modules.OpaqueParameterData,
        value: *anyopaque,
    ) callconv(.c) void = null,
    write: ?*const fn (
        data: modules.OpaqueParameterData,
        value: *const anyopaque,
    ) callconv(.c) void = null,
    name: [*:0]const u8,
    default_value: extern union {
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,
    },
};

/// Declaration of a module resource.
pub const Resource = extern struct {
    path: [*:0]const u8,
};

/// Declaration of a module namespace import.
pub const Namespace = extern struct {
    name: [*:0]const u8,
};

/// Declaration of a module symbol import.
pub const SymbolImport = extern struct {
    version: Version.CVersion,
    name: [*:0]const u8,
    namespace: [*:0]const u8 = "",
};

/// Linkage of an symbol export.
pub const SymbolLinkage = enum(i32) {
    /// The symbol is visible to other instances and is unique.
    global,
    _,
};

/// Declaration of a static module symbol export.
pub const SymbolExport = extern struct {
    symbol: *const anyopaque,
    linkage: SymbolLinkage,
    version: Version.CVersion,
    name: [*:0]const u8,
    namespace: [*:0]const u8 = "",
};

/// Declaration of a dynamic module symbol export.
pub const DynamicSymbolExport = extern struct {
    constructor: *const fn (
        ctx: *const modules.OpaqueInstance,
    ) callconv(.c) EnqueuedFuture(Fallible(*anyopaque)),
    destructor: *const fn (
        ctx: *const modules.OpaqueInstance,
        symbol: *anyopaque,
    ) callconv(.c) void,
    linkage: SymbolLinkage,
    version: Version.CVersion,
    name: [*:0]const u8,
    namespace: [*:0]const u8 = "",
};

/// A modifier declaration for a module export.
pub const Modifier = extern struct {
    tag: enum(i32) {
        destructor,
        dependency,
        instance_state,
        start_event,
        stop_event,
        _,
    },
    value: extern union {
        destructor: *const Destructor,
        dependency: *const Dependency,
        instance_state: *const InstanceState,
        start_event: *const StartEvent,
        stop_event: *const StopEvent,
    },

    /// A destructor function for a module export.
    ///
    /// Is called once the export is no longer in use by the subsystem. An export may declare
    /// multiple destructors.
    pub const Destructor = extern struct {
        data: ?*anyopaque,
        destructor: *const fn (ptr: ?*anyopaque) callconv(.c) void,
    };

    /// A dependency to an already loaded instance.
    ///
    /// The instance will acquire a static dependency to the provided instance. Multiple
    /// dependencies may be provided.
    pub const Dependency = modules.Info;

    /// A constructor and destructor for the state of a module.
    ///
    /// Can be specified to bind a state to an instance. The constructor will be called before the
    /// modules exports are initialized and returning an error will abort the loading of the
    /// instance. Inversely, the destructor function will be called after all exports have been
    /// deinitialized. May only be specified once.
    pub const InstanceState = extern struct {
        init: *const fn (
            ctx: *const modules.OpaqueInstance,
            set: modules.LoadingSet,
        ) callconv(.c) EnqueuedFuture(Fallible(?*anyopaque)),
        deinit: *const fn (
            ctx: *const modules.OpaqueInstance,
            state: ?*anyopaque,
        ) callconv(.c) void,
    };

    /// A listener for the start event of the instance.
    ///
    /// The event will be dispatched immediately after the instance has been loaded. An error will
    /// result in the destruction of the instance. May only be specified once.
    pub const StartEvent = extern struct {
        on_event: *const fn (ctx: *const modules.OpaqueInstance) callconv(.c) EnqueuedFuture(Fallible(void)),
    };

    /// A listener for the stop event of the instance.
    ///
    /// The event will be dispatched immediately before any exports are deinitialized. May only be
    /// specified once.
    pub const StopEvent = extern struct {
        on_event: *const fn (ctx: *const modules.OpaqueInstance) callconv(.c) void,
    };
};

/// Declaration of a module export.
pub const Export = extern struct {
    next: ?*anyopaque = null,
    version: Version.CVersion = ctx.context_version.intoC(),
    name: [*:0]const u8,
    description: ?[*:0]const u8 = null,
    author: ?[*:0]const u8 = null,
    license: ?[*:0]const u8 = null,
    parameters: ?[*]const Parameter = null,
    parameters_count: usize = 0,
    resources: ?[*]const Resource = null,
    resources_count: usize = 0,
    namespace_imports: ?[*]const Namespace = null,
    namespace_imports_count: usize = 0,
    symbol_imports: ?[*]const SymbolImport = null,
    symbol_imports_count: usize = 0,
    symbol_exports: ?[*]const SymbolExport = null,
    symbol_exports_count: usize = 0,
    dynamic_symbol_exports: ?[*]const DynamicSymbolExport = null,
    dynamic_symbol_exports_count: usize = 0,
    modifiers: ?[*]const Modifier = null,
    modifiers_count: usize = 0,

    /// Runs the registered cleanup routines.
    pub fn deinit(self: *const Export) void {
        for (self.getModifiers()) |modifier| {
            switch (modifier.tag) {
                .destructor => {
                    const destructor = modifier.value.destructor;
                    destructor.destructor(destructor.data);
                },
                .dependency => {
                    const dependency = modifier.value.dependency;
                    dependency.unref();
                },
                .instance_state, .start_event, .stop_event => {},
                else => @panic("Unknown modifier"),
            }
        }
    }

    /// Returns the version of the context compiled agains.
    pub fn getVersion(self: *const Export) Version {
        return Version.initC(self.version);
    }

    /// Returns the name of the export.
    pub fn getName(self: *const Export) []const u8 {
        return std.mem.span(self.name);
    }

    /// Returns the description of the export.
    pub fn getDescription(self: *const Export) ?[]const u8 {
        return if (self.description) |x| std.mem.span(x) else null;
    }

    /// Returns the author of the export.
    pub fn getAuthor(self: *const Export) ?[]const u8 {
        return if (self.author) |x| std.mem.span(x) else null;
    }

    /// Returns the license of the export.
    pub fn getLicense(self: *const Export) ?[]const u8 {
        return if (self.license) |x| std.mem.span(x) else null;
    }

    /// Returns the parameters.
    pub fn getParameters(self: *const Export) []const Parameter {
        return if (self.parameters) |x| x[0..self.parameters_count] else &.{};
    }

    /// Returns the resources.
    pub fn getResources(self: *const Export) []const Resource {
        return if (self.resources) |x| x[0..self.resources_count] else &.{};
    }

    /// Returns the namespace imports.
    pub fn getNamespaceImports(self: *const Export) []const Namespace {
        return if (self.namespace_imports) |x| x[0..self.namespace_imports_count] else &.{};
    }

    /// Returns the symbol imports.
    pub fn getSymbolImports(self: *const Export) []const SymbolImport {
        return if (self.symbol_imports) |x| x[0..self.symbol_imports_count] else &.{};
    }

    /// Returns the static symbol exports.
    pub fn getSymbolExports(self: *const Export) []const SymbolExport {
        return if (self.symbol_exports) |x| x[0..self.symbol_exports_count] else &.{};
    }

    /// Returns the dynamic symbol exports.
    pub fn getDynamicSymbolExports(self: *const Export) []const DynamicSymbolExport {
        return if (self.dynamic_symbol_exports) |x|
            x[0..self.dynamic_symbol_exports_count]
        else
            &.{};
    }

    /// Returns the modifiers.
    pub fn getModifiers(self: *const Export) []const Modifier {
        return if (self.modifiers) |x| x[0..self.modifiers_count] else &.{};
    }

    /// Returns the instance state modifier, if specified.
    pub fn getInstanceStateModifier(self: *const Export) ?*const Modifier.InstanceState {
        for (self.getModifiers()) |mod| {
            if (mod.tag == .instance_state) return mod.value.instance_state;
        }
        return null;
    }

    /// Returns the start event modifier, if specified.
    pub fn getStartEventModifier(self: *const Export) ?*const Modifier.StartEvent {
        for (self.getModifiers()) |mod| {
            if (mod.tag == .start_event) return mod.value.start_event;
        }
        return null;
    }

    /// Returns the start event modifier, if specified.
    pub fn getStopEventModifier(self: *const Export) ?*const Modifier.StopEvent {
        for (self.getModifiers()) |mod| {
            if (mod.tag == .stop_event) return mod.value.stop_event;
        }
        return null;
    }
};

pub fn ModuleBundle(bundle: anytype) type {
    if (@typeInfo(@TypeOf(bundle)) != .@"struct") @compileError("fimo: invalid module bundle, expected a tuple, found " ++ @typeName(@TypeOf(bundle)));
    if (!@typeInfo(@TypeOf(bundle)).@"struct".is_tuple) @compileError("fimo: invalid module bundle, expected a tuple, found " ++ @typeName(@TypeOf(bundle)));
    for (@typeInfo(@TypeOf(bundle)).@"struct".fields) |f| {
        if (!@hasDecl(@field(bundle, f.name), "fimo_modules_bundle_marker") and !@hasDecl(@field(bundle, f.name), "fimo_modules_marker"))
            @compileError("fimo: invalid module bundle entry, expected a module or a module bundle, found " ++ @typeName(@field(bundle, f.name)));
    }

    return struct {
        pub const bundled = bundle;
        pub const fimo_modules_bundle_marker = {};

        pub fn loadingSetFilter(@"export": *const Export, context: void) modules.LoadingSet.FilterRequest {
            _ = context;
            inline for (@typeInfo(@TypeOf(bundled)).@"struct".fields) |f| {
                if (comptime @hasDecl(@field(bundle, f.name), "fimo_modules_marker")) {
                    if (@"export" == @field(bundled, f.name).@"export") return .load;
                } else if (comptime @hasDecl(@field(bundle, f.name), "fimo_modules_bundle_marker")) {
                    if (@field(bundled, f.name).loadingSetFilter(@"export", {}) == .load) return .load;
                } else unreachable;
            }
            return .skip;
        }
    };
}

pub fn Module(T: type) type {
    @setEvalBranchQuota(100000);
    if (!@hasDecl(T, "fimo_module")) @compileError("fimo: invalid module, missing `pub const fimo_module = .foo_name;` declaration: " ++ @typeName(T));

    const Global = struct {
        var is_init: bool = false;
        var state: T = undefined;
        var instance: *const modules.OpaqueInstance = undefined;
    };
    comptime var builder = switch (@typeInfo(@TypeOf(T.fimo_module))) {
        .enum_literal => Builder.init(@tagName(T.fimo_module)),
        .@"struct" => blk: {
            const module = T.fimo_module;
            const M = @TypeOf(module);
            if (!@hasField(M, "name")) @compileError("fimo: invalid module, missing `pub const fimo_module = .{ .name = .foo_name };` declaration: " ++ @typeName(T));
            if (@typeInfo(@TypeOf(module.name)) != .enum_literal) @compileError("fimo: invalid module, expected `pub const fimo_module = .{ .name = .foo_name };` declaration, found: " ++ @typeName(@TypeOf(module.name)));

            var b = Builder.init(@tagName(module.name));
            if (@hasField(M, "description")) b = b.withDescription(module.description);
            if (@hasField(M, "author")) b = b.withAuthor(module.author);
            if (@hasField(M, "license")) b = b.withLicense(module.license);

            break :blk b;
        },
        else => @compileError("fimo: invalid module, expected `pub const fimo_module = .foo_name;` declaration, found: " ++ @typeName(@TypeOf(T.mach_module))),
    };

    if (@hasDecl(T, "fimo_parameters")) {
        if (@typeInfo(@TypeOf(T.fimo_parameters)) != .@"struct") @compileError("fimo: invalid parameters, expected `pub const fimo_parameters = .{ .param = .{ ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_parameters)));
        inline for (std.meta.fieldNames(@TypeOf(T.fimo_parameters))) |name| {
            const param = @field(T.fimo_parameters, name);
            if (!@hasField(@TypeOf(param), "default")) @compileError("fimo: invalid parameter default value, expected `pub const fimo_parameters = .{ .param = .{ .default = @as(..., ...) }, ... };` declaration, found: " ++ @typeName(@TypeOf(param)));
            const default = param.default;
            switch (@TypeOf(default)) {
                u8, u16, u32, u64, i8, i16, i32, i64 => {},
                else => @compileError("fimo: invalid parameter default value type, expected `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32` or `i64` found: " ++ @typeName(@TypeOf(default))),
            }
            const read_group = if (@hasField(@TypeOf(param), "read_group")) blk2: {
                const group = param.read_group;
                if (@typeInfo(@TypeOf(group)) != .enum_literal) @compileError("fimo: invalid parameter read group, expected `private`, `dependency` or `public` found: " ++ @typeName(@TypeOf(default)));
                break :blk2 group;
            } else .private;
            const write_group = if (@hasField(@TypeOf(param), "write_group")) blk2: {
                const group = param.write_group;
                if (@typeInfo(@TypeOf(group)) != .enum_literal) @compileError("fimo: invalid parameter write group, expected `private`, `dependency` or `public` found: " ++ @typeName(@TypeOf(default)));
                break :blk2 group;
            } else .private;
            const read = if (@hasField(@TypeOf(param), "read")) &struct {
                fn f(p: modules.OpaqueParameterData, value: *anyopaque) callconv(.c) void {
                    const func: fn (modules.ParameterData(@TypeOf(default))) @TypeOf(default) = param.read;
                    const d = modules.ParameterData(@TypeOf(default)).castFromOpaque(p);
                    const v: *@TypeOf(default) = @ptrCast(@alignCast(value));
                    v.* = func(d);
                }
            }.f else null;
            const write = if (@hasField(@TypeOf(param), "write")) &struct {
                fn f(p: modules.OpaqueParameterData, value: *const anyopaque) callconv(.c) void {
                    const func: fn (modules.ParameterData(@TypeOf(default)), @TypeOf(default)) void = param.write;
                    const d = modules.ParameterData(@TypeOf(default)).castFromOpaque(p);
                    const v: *const @TypeOf(default) = @ptrCast(@alignCast(value));
                    func(d, v.*);
                }
            }.f else null;

            builder = builder.withParameter(.{
                .name = name,
                .member_name = name,
                .default_value = switch (@TypeOf(default)) {
                    u8 => .{ .u8 = default },
                    u16 => .{ .u16 = default },
                    u32 => .{ .u32 = default },
                    u64 => .{ .u64 = default },
                    i8 => .{ .i8 = default },
                    i16 => .{ .i16 = default },
                    i32 => .{ .i32 = default },
                    i64 => .{ .i64 = default },
                    else => unreachable,
                },
                .read_group = read_group,
                .write_group = write_group,
                .read = read,
                .write = write,
            });
        }
    }

    if (@hasDecl(T, "fimo_paths")) {
        if (@typeInfo(@TypeOf(T.fimo_paths)) != .@"struct") @compileError("fimo: invalid paths, expected `pub const fimo_paths = .{ .path = .{ ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_paths)));
        inline for (std.meta.fieldNames(@TypeOf(T.fimo_paths))) |name| {
            const p = @field(T.fimo_paths, name);
            if (@TypeOf(p) != paths.Path) @compileError("fimo: invalid parameters, expected a path, found: " ++ @typeName(@TypeOf(p)));
            builder = builder.withResource(.{ .name = name, .path = p });
        }
    }

    if (@hasDecl(T, "fimo_imports")) {
        if (@typeInfo(@TypeOf(T.fimo_imports)) != .@"struct") @compileError("fimo: invalid imports, expected `pub const fimo_imports = .{ imp, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_imports)));
        var imps: []const modules.Symbol = &.{};
        inline for (T.fimo_imports) |imp| {
            if (@TypeOf(imp) == modules.Symbol)
                imps = imps ++ [_]modules.Symbol{imp}
            else inline for (imp) |imp2| imps = imps ++ [_]modules.Symbol{imp2};
        }
        const Tup = std.meta.Tuple(&([_]type{modules.Symbol} ** imps.len));
        var tup: Tup = undefined;
        inline for (imps, 0..) |imp, i| {
            tup[i] = imp;
        }
        builder = builder.withMultipleImports(tup);
    }

    if (@hasDecl(T, "fimo_exports")) {
        if (@typeInfo(@TypeOf(T.fimo_exports)) != .@"struct") @compileError("fimo: invalid exports, expected `pub const fimo_exports = .{ .exp = .{ ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_exports)));
        inline for (std.meta.fieldNames(@TypeOf(T.fimo_exports))) |name| {
            const exp = @field(T.fimo_exports, name);
            if (!@hasField(@TypeOf(exp), "symbol")) @compileError("fimo: invalid export, expected `pub const fimo_exports = .{ .exp = .{ .symbol = foo, ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(exp)));
            const symbol: modules.Symbol = exp.symbol;
            const linkage = if (@hasField(@TypeOf(exp), "linkage")) blk2: {
                break :blk2 exp.linkage;
            } else .global;
            if (!@hasField(@TypeOf(exp), "value")) @compileError("fimo: invalid export value, expected `pub const fimo_exports = .{ .exp = .{ .value = ..., ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(exp)));
            const value = exp.value;
            if (@typeInfo(@TypeOf(value)) == .pointer) {
                if (@TypeOf(value) != *const symbol.T) @compileError("fimo: invalid export value, expected `" ++ @typeName(*const symbol.T) ++ "`, found " ++ @typeName(@TypeOf(value)));
                const wrapper = struct {
                    const Sync = struct {
                        const Result = *symbol.T;
                        const Future = tasks.Future(@This(), Result, poll, null);

                        fn init(inst: *const modules.OpaqueInstance) Future {
                            _ = inst;
                            return Future.init(.{});
                        }
                        fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                            _ = this;
                            _ = waker;
                            symbol.getGlobal().register(value);
                            return .{ .ready = @constCast(value) };
                        }
                    };
                    fn deinit(inst: *const modules.OpaqueInstance, sym: *symbol.T) void {
                        _ = inst;
                        _ = sym;
                        symbol.getGlobal().unregister();
                    }
                };
                builder = builder.withDynamicExport(
                    .{ .symbol = symbol, .name = name, .linkage = linkage },
                    wrapper.Sync.Future,
                    wrapper.Sync.init,
                    wrapper.deinit,
                );
            } else {
                const wrapper = struct {
                    const Sync = struct {
                        const Result = @typeInfo(value.init).@"fn".return_type.?;
                        const Future = tasks.Future(@This(), Result, poll, null);

                        fn init(inst: *const modules.OpaqueInstance) Future {
                            _ = inst;
                            return Future.init(.{});
                        }
                        fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                            _ = this;
                            _ = waker;
                            const sym: *symbol.T = if (@typeInfo(Result) == .error_union)
                                value.init() catch |err| return .{ .ready = err }
                            else
                                value.init();
                            symbol.getGlobal().register(sym);
                            return .{ .ready = sym };
                        }
                    };
                    const Async = struct {
                        inner: Inner,

                        const Inner = @typeInfo(@TypeOf(value.init)).@"fn".return_type.?;
                        const Result = Inner.Result;
                        const Future = tasks.Future(@This(), Result, poll, Async.deinit);
                        comptime {
                            if (Inner.Result != *symbol.T) switch (@typeInfo(Inner.Result)) {
                                .error_union => |v| {
                                    if (Inner.Result != v.error_set!*symbol.T) @compileError("fimo: invalid init return type, expected `*T` or `err!*T`, found " ++ @typeName(Inner.Result));
                                },
                                else => @compileError("fimo: invalid init return type, expected `*T` or `err!*T`, found " ++ @typeName(Inner.Result)),
                            };
                        }

                        fn init(inst: *const modules.OpaqueInstance) Future {
                            _ = inst;
                            return Future.init(.{ .inner = value.init() });
                        }
                        fn deinit(this: *@This()) void {
                            if (@hasDecl(Inner, "deinit")) this.inner.deinit();
                        }
                        fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                            switch (this.inner.poll(waker)) {
                                .ready => |v| {
                                    const sym: *symbol.T = if (@typeInfo(Result) == .error_union)
                                        v catch |err| return .{ .ready = err }
                                    else
                                        v;
                                    symbol.getGlobal().register(sym);
                                    return .{ .ready = sym };
                                },
                                .pending => return .pending,
                            }
                        }
                    };
                    fn deinit(inst: *const modules.OpaqueInstance, sym: *symbol.T) void {
                        _ = inst;
                        symbol.getGlobal().unregister();
                        const f: fn (*symbol.T) void = value.deinit;
                        f(sym);
                    }
                };
                if (@typeInfo(@TypeOf(value.init)) == .@"fn") {
                    builder = builder.withDynamicExport(
                        .{ .symbol = symbol, .name = name, .linkage = linkage },
                        wrapper.Sync.Future,
                        wrapper.Sync.init,
                        wrapper.deinit,
                    );
                } else {
                    builder = builder.withDynamicExport(
                        .{ .symbol = symbol, .name = name, .linkage = linkage },
                        wrapper.Async.Future,
                        wrapper.Async.init,
                        wrapper.deinit,
                    );
                }

                const return_type = @typeInfo(@TypeOf(value.init)).@"fn".return_type.?;
                if (return_type == *symbol.T)
                    builder = builder.withOnStartEvent(wrapper.Sync.Future, wrapper.Sync.init)
                else if (@typeInfo(return_type) == .error_union) {
                    const payload = @typeInfo(return_type).error_union.payload;
                    if (payload == *symbol.T)
                        builder = builder.withOnStartEvent(wrapper.Sync.Future, wrapper.Sync.init)
                    else
                        builder.withOnStartEvent(wrapper.Async.Future, wrapper.Async.init);
                } else builder.withOnStartEvent(wrapper.Async.Future, wrapper.Async.init);
            }
        }
    }

    if (@hasDecl(T, "fimo_events")) {
        if (@typeInfo(@TypeOf(T.fimo_events)) != .@"struct") @compileError("fimo: invalid events, expected `pub const fimo_events = .{ .init = ..., ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_events)));
        if (@hasField(@TypeOf(T.fimo_events), "init")) {
            const ev_init = T.fimo_events.init;
            const wrapper = struct {
                const Sync = struct {
                    successfull: bool = false,
                    set: modules.LoadingSet,

                    const EvReturn = @typeInfo(@TypeOf(ev_init)).@"fn".return_type.?;
                    const Result = switch (@typeInfo(EvReturn)) {
                        .error_union => |v| v.error_set!*T,
                        .void => *T,
                        else => @compileError("fimo: invalid init return type, expected `*T` or `err!*T`, found " ++ @typeName(EvReturn)),
                    };
                    const Future = tasks.Future(@This(), Result, poll, Sync.deinit);

                    fn init(inst: *const modules.OpaqueInstance, set: modules.LoadingSet) Future {
                        if (Global.is_init) @panic("already init");
                        ctx.Handle.registerHandle(inst.handle);
                        Global.instance = @ptrCast(@alignCast(inst));
                        Global.is_init = true;
                        return Future.init(.{ .set = set });
                    }
                    fn deinit(this: *@This()) void {
                        if (!this.successfull) {
                            ctx.Handle.unregisterHandle();
                            Global.instance = undefined;
                            Global.is_init = false;
                        }
                    }
                    fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                        _ = waker;
                        if (@typeInfo(T) == .@"struct") inline for (@typeInfo(T).@"struct".fields) |f| {
                            if (f.default_value_ptr) |default| {
                                @field(Global.state, f.name) = @as(*const f.type, @ptrCast(@alignCast(default))).*;
                            }
                        };
                        const Args = std.meta.ArgsTuple(@TypeOf(ev_init));
                        if (std.meta.fields(Args).len > 2) @compileError("fimo: invalid init event, got too many arguments, found: " ++ @typeName(@TypeOf(ev_init)));
                        var args: Args = undefined;
                        inline for (std.meta.fields(Args), 0..) |f, i| {
                            switch (f.type) {
                                *T => args[i] = &Global.state,
                                modules.LoadingSet => args[i] = this.set,
                                else => @compileError("fimo: invalid init event, got invalid parameter type, found: " ++ @typeName(f.type)),
                            }
                        }
                        const result = @call(.auto, ev_init, args);
                        if (@typeInfo(Result) == .error_union) result catch |err| return .{ .ready = err };
                        this.successfull = true;
                        return .{ .ready = &Global.state };
                    }
                };
                const Async = struct {
                    inner: Inner,
                    successfull: bool = false,

                    const Inner = @typeInfo(@TypeOf(ev_init)).@"fn".return_type.?;
                    const Result = switch (@typeInfo(Inner.Result)) {
                        .error_union => |v| v.error_set!*T,
                        .void => *T,
                        else => @compileError("fimo: invalid init return type, expected `*T` or `err!*T`, found " ++ @typeName(Inner.Result)),
                    };
                    const Future = tasks.Future(@This(), Result, poll, Async.deinit);

                    fn init(inst: *const modules.OpaqueInstance, set: modules.LoadingSet) Future {
                        if (Global.is_init) @panic("already init");
                        ctx.Handle.registerHandle(inst.handle);
                        Global.instance = @ptrCast(@alignCast(inst));
                        Global.is_init = true;
                        if (@typeInfo(T) == .@"struct") inline for (@typeInfo(T).@"struct".fields) |f| {
                            if (f.default_value_ptr) |default| {
                                @field(Global.state, f.name) = @as(*const f.type, @ptrCast(@alignCast(default))).*;
                            }
                        };
                        const Args = std.meta.ArgsTuple(@TypeOf(ev_init));
                        if (std.meta.fields(Args).len > 2) @compileError("fimo: invalid init event, got too many arguments, found: " ++ @typeName(@TypeOf(ev_init)));
                        var args: Args = undefined;
                        inline for (std.meta.fields(Args), 0..) |f, i| {
                            switch (f.type) {
                                *T => args[i] = &Global.state,
                                modules.LoadingSet => args[i] = set,
                                else => @compileError("fimo: invalid init event, got invalid parameter type, found: " ++ @typeName(f.type)),
                            }
                        }
                        const result = @call(.auto, ev_init, args);
                        return Future.init(.{ .inner = result });
                    }
                    fn deinit(this: *@This()) void {
                        if (!this.successfull) {
                            ctx.Handle.unregisterHandle();
                            Global.instance = undefined;
                            Global.state = undefined;
                            Global.is_init = false;
                        }
                    }
                    fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                        switch (this.inner.poll(waker)) {
                            .ready => |v| {
                                if (@typeInfo(Result) == .error_union) v catch |err| return .{ .ready = err };
                                this.successfull = true;
                                return .{ .ready = &Global.state };
                            },
                            .pending => return .pending,
                        }
                    }
                };
                fn deinit(inst: *const modules.OpaqueInstance, state: *T) void {
                    _ = inst;
                    if (@hasField(@TypeOf(T.fimo_events), "deinit")) {
                        const f = T.fimo_events.deinit;
                        if (@typeInfo(@TypeOf(f)).@"fn".params.len == 1) f(state) else f();
                    }
                    ctx.Handle.unregisterHandle();
                    Global.instance = undefined;
                    Global.state = undefined;
                    Global.is_init = false;
                }
            };

            switch (@typeInfo(@typeInfo(@TypeOf(ev_init)).@"fn".return_type.?)) {
                .void, .error_union => builder = builder.withState(wrapper.Sync.Future, wrapper.Sync.init, wrapper.deinit),
                else => builder.withState(wrapper.Async.Future, wrapper.Async.init, wrapper.deinit),
            }
        } else {
            const wrapper = struct {
                const Sync = struct {
                    successfull: bool = false,
                    set: modules.LoadingSet,

                    const Result = *T;
                    const Future = tasks.Future(@This(), Result, poll, Sync.deinit);

                    fn init(inst: *const modules.OpaqueInstance, set: modules.LoadingSet) Future {
                        if (Global.is_init) @panic("already init");
                        ctx.Handle.registerHandle(inst.handle);
                        Global.instance = @ptrCast(@alignCast(inst));
                        Global.is_init = true;
                        if (@typeInfo(T) == .@"struct") inline for (@typeInfo(T).@"struct".fields) |f| {
                            if (f.default_value_ptr) |default| {
                                @field(Global.state, f.name) = @as(*const f.type, @ptrCast(@alignCast(default))).*;
                            }
                        };
                        return Future.init(.{ .set = set });
                    }
                    fn deinit(this: *@This()) void {
                        if (!this.successfull) {
                            ctx.Handle.unregisterHandle();
                            Global.instance = undefined;
                            Global.is_init = false;
                        }
                    }
                    fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                        _ = waker;
                        this.successfull = true;
                        return .{ .ready = &Global.state };
                    }
                };
                fn deinit(inst: *const modules.OpaqueInstance, state: *T) void {
                    _ = inst;
                    if (@hasField(@TypeOf(T.fimo_events), "deinit")) {
                        const f = T.fimo_events.deinit;
                        if (@typeInfo(@TypeOf(f)).@"fn".params.len == 1) f(state) else f();
                    }
                    ctx.Handle.unregisterHandle();
                    Global.instance = undefined;
                    Global.state = undefined;
                    Global.is_init = false;
                }
            };
            builder = builder.withState(wrapper.Sync.Future, wrapper.Sync.init, wrapper.deinit);
        }
        if (@hasField(@TypeOf(T.fimo_events), "on_start")) {
            const on_start = T.fimo_events.on_start;
            const wrapper = struct {
                const Sync = struct {
                    const EvReturn = @typeInfo(@TypeOf(on_start)).@"fn".return_type.?;
                    const Result = switch (@typeInfo(EvReturn)) {
                        .error_union => |v| v.error_set!void,
                        .void => void,
                        else => @compileError("fimo: invalid on_start return type, expected `void` or `err!void`, found " ++ @typeName(EvReturn)),
                    };
                    const Future = tasks.Future(@This(), Result, poll, null);

                    fn init(inst: *const modules.OpaqueInstance) Future {
                        _ = inst;
                        return Future.init(.{});
                    }
                    fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                        _ = this;
                        _ = waker;
                        const result = on_start();
                        if (@typeInfo(Result) == .error_union) result catch |err| return .{ .ready = err };
                        return .{ .ready = {} };
                    }
                };
                const Async = struct {
                    inner: Inner,

                    const Inner = @typeInfo(@TypeOf(on_start)).@"fn".return_type.?;
                    const Result = switch (@typeInfo(Inner.Result)) {
                        .error_union => |v| v.error_set!void,
                        .void => void,
                        else => @compileError("fimo: invalid init return type, expected `void` or `err!void`, found " ++ @typeName(Inner.Result)),
                    };
                    const Future = tasks.Future(@This(), Result, poll, Async.deinit);

                    fn init(inst: *const modules.OpaqueInstance) Future {
                        _ = inst;
                        return Future.init(.{ .inner = on_start() });
                    }
                    fn deinit(this: *@This()) void {
                        if (@hasDecl(Inner, "deinit")) this.inner.deinit();
                    }
                    fn poll(this: *@This(), waker: tasks.Waker) tasks.Poll(Result) {
                        switch (this.inner.poll(waker)) {
                            .ready => |v| return v,
                            .pending => return .pending,
                        }
                    }
                };
            };
            switch (@typeInfo(@typeInfo(@TypeOf(on_start)).@"fn".return_type.?)) {
                .void, .error_union => builder = builder.withOnStartEvent(wrapper.Sync.Future, wrapper.Sync.init),
                else => builder.withOnStartEvent(wrapper.Async.Future, wrapper.Async.init),
            }
        }
        if (@hasField(@TypeOf(T.fimo_events), "on_stop")) {
            const on_stop = T.fimo_events.on_stop;
            const wrapper = struct {
                fn f(inst: *const modules.OpaqueInstance) void {
                    _ = inst;
                    on_stop();
                }
            };
            builder = builder.withOnStopEvent(wrapper.f);
        }
    }

    const InstanceT = builder.exportModule();
    return struct {
        pub const is_init: *bool = &Global.is_init;
        pub const @"export" = Instance.@"export";
        pub const Instance = InstanceT;
        pub const fimo_modules_marker = {};

        pub fn instance() *const Instance {
            std.debug.assert(is_init.*);
            return @ptrCast(Global.instance);
        }

        /// Returns the parameter table.
        pub fn parameters() *const Instance.Parameters {
            return instance().parameters();
        }

        /// Returns the paths table.
        pub fn paths() *const Instance.Resources {
            return instance().resources();
        }

        /// Returns the import table.
        pub fn imports() *const Instance.Imports {
            return instance().imports();
        }

        /// Returns the export table.
        pub fn exports() *const Instance.Exports {
            return instance().exports();
        }

        /// Returns the module info.
        pub fn info() *const modules.Info {
            return instance().info;
        }

        /// Returns the context handle.
        pub fn handle() *const ctx.Handle {
            return instance().handle;
        }

        /// Returns the instance state.
        pub fn state() *T {
            return if (comptime @sizeOf(T) == 0) &T{} else &Global.state;
        }

        /// Provides a pointer to the requested symbol.
        pub fn provideSymbol(comptime symbol: modules.Symbol) *const symbol.T {
            return instance().provideSymbol(symbol);
        }

        /// Increases the strong reference count of the module instance.
        ///
        /// Will prevent the module from being unloaded. This may be used to pass data, like callbacks,
        /// between modules, without registering the dependency with the subsystem.
        pub fn ref() void {
            instance().ref();
        }

        /// Decreases the strong reference count of the module instance.
        ///
        /// Should only be called after `ref`, when the dependency is no longer required.
        pub fn unref() void {
            instance().unref();
        }

        /// Checks the status of a namespace from the view of the module.
        ///
        /// Checks if the module includes the namespace. In that case, the module is allowed access
        /// to the symbols in the namespace. Additionally, this function also queries whether the
        /// include is static, i.e., it was specified by the module at load time.
        pub fn queryNamespace(namespace: [:0]const u8) ctx.Error!enum { removed, added, static } {
            return switch (try instance().queryNamespace(namespace)) {
                .removed => .removed,
                .added => .added,
                .static => .static,
            };
        }

        /// Includes a namespace by the module.
        ///
        /// Once included, the module gains access to the symbols of its dependencies that are
        /// exposed in said namespace. A namespace can not be included multiple times.
        pub fn addNamespace(namespace: [:0]const u8) ctx.Error!void {
            try instance().addNamespace(namespace);
        }

        /// Removes a namespace include from the module.
        ///
        /// Once excluded, the caller guarantees to relinquish access to the symbols contained in
        /// said namespace. It is only possible to exclude namespaces that were manually added,
        /// whereas static namespace includes remain valid until the module is unloaded.
        pub fn removeNamespace(namespace: [:0]const u8) ctx.Error!void {
            try instance().removeNamespace(namespace);
        }

        /// Checks if a module depends on another module.
        ///
        /// Checks if the specified module is a dependency of the current instance. In that case
        /// the instance is allowed to access the symbols exported by the module. Additionally,
        /// this function also queries whether the dependency is static, i.e., the dependency was
        /// specified by the module at load time.
        pub fn queryDependency(dep: *const modules.Info) ctx.Error!enum { removed, added, static } {
            return switch (try instance().queryDependency(dep)) {
                .removed => .removed,
                .added => .added,
                .static => .static,
            };
        }

        /// Acquires another module as a dependency.
        ///
        /// After acquiring a module as a dependency, the module is allowed access to the symbols
        /// and protected parameters of said dependency. Trying to acquire a dependency to a module
        /// that is already a dependency, or to a module that would result in a circular dependency
        /// will result in an error.
        pub fn addDependency(dep: *const modules.Info) ctx.Error!void {
            try instance().addDependency(dep);
        }

        /// Removes a module as a dependency.
        ///
        /// By removing a module as a dependency, the caller ensures that it does not own any
        /// references to resources originating from the former dependency, and allows for the
        /// unloading of the module. A module can only relinquish dependencies to modules that were
        /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
        pub fn removeDependency(dep: *const modules.Info) ctx.Error!void {
            try instance().removeDependency(dep);
        }

        /// Loads a group of symbols from the module subsystem.
        ///
        /// Is equivalent to calling `loadSymbol` for each symbol of the group independently.
        pub fn loadSymbolGroup(comptime symbols: anytype) ctx.Error!modules.SymbolGroup(symbols) {
            return try instance().loadSymbolGroup(symbols);
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
        /// loading optional symbols, or for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until the module relinquishes the
        /// dependency to the module that exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbol(comptime symbol: modules.Symbol) ctx.Error!modules.SymbolWrapper(symbol) {
            return try instance().loadSymbol(symbol);
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
        /// loading optional symbols, or for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until the module relinquishes the
        /// dependency to the module that exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbolRaw(
            name: [:0]const u8,
            namespace: [:0]const u8,
            version: Version,
        ) ctx.Error!*const anyopaque {
            return try instance().loadSymbolRaw(name, namespace, version);
        }

        /// Reads a module parameter with dependency read access.
        ///
        /// Reads the value of a module parameter with dependency read access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow reading with a
        /// dependency access.
        pub fn readParameter(
            comptime ParamT: type,
            module: [:0]const u8,
            parameter: [:0]const u8,
        ) ctx.Error!ParamT {
            return try instance().readParameter(ParamT, module, parameter);
        }

        /// Sets a module parameter with dependency write access.
        ///
        /// Sets the value of a module parameter with dependency write access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow writing with a
        /// dependency access.
        pub fn writeParameter(
            comptime ParamT: type,
            value: ParamT,
            module: [:0]const u8,
            parameter: [:0]const u8,
        ) ctx.Error!void {
            try instance().writeParameter(ParamT, value, module, parameter);
        }
    };
}

/// Builder for a module export.
pub const Builder = struct {
    name: [:0]const u8,
    description: ?[:0]const u8 = null,
    author: ?[:0]const u8 = null,
    license: ?[:0]const u8 = null,
    parameters: []const Builder.Parameter = &.{},
    resources: []const Builder.Resource = &.{},
    namespaces: []const Namespace = &.{},
    imports: []const Builder.SymbolImport = &.{},
    exports: []const Builder.SymbolExport = &.{},
    modifiers: []const Builder.Modifier = &.{},
    stateType: type = void,
    instance_state: ?ExportsRoot.Modifier.InstanceState = null,
    start_event: ?ExportsRoot.Modifier.StartEvent = null,
    stop_event: ?ExportsRoot.Modifier.StopEvent = null,

    pub const Parameter = struct {
        name: []const u8,
        member_name: [:0]const u8,
        default_value: union(enum) {
            u8: u8,
            u16: u16,
            u32: u32,
            u64: u64,
            i8: i8,
            i16: i16,
            i32: i32,
            i64: i64,
        },
        read_group: modules.ParameterAccessGroup = .private,
        write_group: modules.ParameterAccessGroup = .private,
        read: ?*const fn (
            data: modules.OpaqueParameterData,
            value: *anyopaque,
        ) callconv(.c) void = null,
        write: ?*const fn (
            data: modules.OpaqueParameterData,
            value: *const anyopaque,
        ) callconv(.c) void = null,
    };

    pub const Resource = struct {
        name: [:0]const u8,
        path: paths.Path,
    };

    pub const SymbolImport = struct {
        name: ?[:0]const u8 = null,
        symbol: modules.Symbol,
    };

    pub const SymbolExportOptions = struct {
        symbol: modules.Symbol,
        name: ?[:0]const u8 = null,
        linkage: SymbolLinkage = .global,
    };

    const SymbolExport = struct {
        symbol: modules.Symbol,
        name: ?[:0]const u8,
        linkage: SymbolLinkage,
        value: union(enum) {
            static: *const anyopaque,
            dynamic: struct {
                initFn: *const fn (
                    ctx: *const modules.OpaqueInstance,
                ) callconv(.c) EnqueuedFuture(Fallible(*anyopaque)),
                deinitFn: *const fn (
                    ctx: *const modules.OpaqueInstance,
                    symbol: *anyopaque,
                ) callconv(.c) void,
            },
        },
    };

    const Modifier = union(enum) {
        instance_state: void,
        start_event: void,
        stop_event: void,
        _,
    };

    fn EnqueueError(comptime T: type) tasks.EnqueuedFuture(Fallible(T)) {
        return .{
            .data = @ptrCast(@constCast(&{})),
            .poll_fn = &struct {
                fn f(
                    data: **anyopaque,
                    waker: tasks.Waker,
                    res: *Fallible(T),
                ) callconv(.c) bool {
                    _ = data;
                    _ = waker;
                    res.* = Fallible(T).wrap(error.EnqueueError);
                    return true;
                }
            }.f,
            .cleanup_fn = null,
        };
    }

    /// Initializes a new builder.
    pub fn init(comptime name: [:0]const u8) Builder {
        return .{ .name = name };
    }

    /// Adds a description to the module.
    pub fn withDescription(comptime self: Builder, comptime description: ?[:0]const u8) Builder {
        var x = self;
        x.description = description;
        return x;
    }

    /// Adds a author to the module.
    pub fn withAuthor(comptime self: Builder, comptime author: ?[:0]const u8) Builder {
        var x = self;
        x.author = author;
        return x;
    }

    /// Adds a license to the module.
    pub fn withLicense(comptime self: Builder, comptime license: ?[:0]const u8) Builder {
        var x = self;
        x.license = license;
        return x;
    }

    /// Adds a parameter to the module.
    pub fn withParameter(
        comptime self: Builder,
        comptime parameter: Builder.Parameter,
    ) Builder {
        for (self.parameters) |p| {
            if (std.mem.eql(u8, p.name, parameter.name)) @compileError(
                std.fmt.comptimePrint("duplicate parameter name: '{s}'", .{parameter.name}),
            );
            if (std.mem.eql(u8, p.member_name, parameter.member_name)) @compileError(
                std.fmt.comptimePrint(
                    "duplicate parameter member name: '{s}'",
                    .{parameter.member_name},
                ),
            );
        }

        var parameters: [self.parameters.len + 1]Builder.Parameter = undefined;
        @memcpy(parameters[0..self.parameters.len], self.parameters);
        parameters[self.parameters.len] = parameter;

        var x = self;
        x.parameters = &parameters;
        return x;
    }

    /// Adds a resource path to the module.
    pub fn withResource(
        comptime self: Builder,
        comptime resource: Builder.Resource,
    ) Builder {
        for (self.resources) |res| {
            if (std.mem.eql(u8, res.name, resource.name))
                @compileError(
                    std.fmt.comptimePrint(
                        "duplicate resource member name: '{s}'",
                        .{res.name},
                    ),
                );
        }

        var res_paths: [self.resources.len + 1]Builder.Resource = undefined;
        @memcpy(res_paths[0..self.resources.len], self.resources);
        res_paths[self.resources.len] = resource;

        var x = self;
        x.resources = &res_paths;
        return x;
    }

    /// Adds a namespace import to the module.
    ///
    /// A namespace may be imported multiple times.
    pub fn withNamespace(
        comptime self: Builder,
        comptime name: []const u8,
    ) Builder {
        if (std.mem.eql(u8, name, "")) return self;
        for (self.namespaces) |ns| {
            if (std.mem.eql(u8, std.mem.span(ns.name), name)) return self;
        }

        var namespaces: [self.namespaces.len + 1]Namespace = undefined;
        @memcpy(namespaces[0..self.namespaces.len], self.namespaces);
        namespaces[self.namespaces.len] = .{ .name = name ++ "\x00" };

        var x = self;
        x.namespaces = &namespaces;
        return x;
    }

    /// Adds multiple imports to the module.
    ///
    /// Automatically imports the required namespaces.
    pub fn withMultipleImports(comptime self: Builder, comptime imports: anytype) Builder {
        var builder = self;
        const imports_info = @typeInfo(@TypeOf(imports)).@"struct";
        inline for (imports_info.fields) |f| {
            const name = f.name;
            const symbol = @field(imports, name);
            builder = builder.withImport(.{
                .name = if (imports_info.is_tuple) null else name,
                .symbol = symbol,
            });
        }
        return builder;
    }

    /// Adds an import to the module.
    ///
    /// Automatically imports the required namespace.
    pub fn withImport(
        comptime self: Builder,
        comptime import: Builder.SymbolImport,
    ) Builder {
        if (import.name) |import_name| for (self.imports) |imp| {
            const imp_name = imp.name orelse continue;
            if (std.mem.eql(u8, imp_name, import_name))
                @compileError(
                    std.fmt.comptimePrint(
                        "duplicate import member name: '{s}'",
                        .{imp.name},
                    ),
                );
        };

        var imports: [self.imports.len + 1]Builder.SymbolImport = undefined;
        @memcpy(imports[0..self.imports.len], self.imports);
        imports[self.imports.len] = import;

        var x = self.withNamespace(import.symbol.namespace);
        x.imports = &imports;
        return x;
    }

    fn withExportInner(
        comptime self: Builder,
        comptime @"export": Builder.SymbolExport,
    ) Builder {
        if (@"export".name) |export_name| for (self.exports) |exp| {
            const exp_name = exp.name orelse continue;
            if (std.mem.eql(u8, exp_name, export_name))
                @compileError(
                    std.fmt.comptimePrint(
                        "duplicate export member name: '{s}'",
                        .{exp.name},
                    ),
                );
        };

        var exports: [self.exports.len + 1]Builder.SymbolExport = undefined;
        @memcpy(exports[0..self.exports.len], self.exports);
        exports[self.exports.len] = @"export";

        var x = self;
        x.exports = &exports;
        return x;
    }

    /// Adds a static export to the module.
    pub fn withExport(
        comptime self: Builder,
        comptime options: SymbolExportOptions,
        comptime value: *const options.symbol.T,
    ) Builder {
        const exp = Builder.SymbolExport{
            .symbol = options.symbol,
            .name = options.name,
            .linkage = options.linkage,
            .value = .{ .static = value },
        };
        return self.withExportInner(exp);
    }

    /// Adds a dynamic export to the module.
    pub fn withDynamicExport(
        comptime self: Builder,
        comptime options: SymbolExportOptions,
        comptime Fut: type,
        comptime initFn: fn (ctx: *const modules.OpaqueInstance) Fut,
        comptime deinitFn: fn (ctx: *const modules.OpaqueInstance, symbol: *options.symbol.T) void,
    ) Builder {
        const Result = Fut.Result;
        switch (@typeInfo(Result)) {
            .error_union => |x| std.debug.assert(x.payload == *options.symbol.T),
            else => std.debug.assert(Result == *options.symbol.T),
        }
        const Wrapper = struct {
            fn wrapInit(
                context: *const modules.OpaqueInstance,
            ) callconv(.c) tasks.EnqueuedFuture(Fallible(*anyopaque)) {
                const fut = initFn(context).intoFuture();
                var mapped_fut = fut.map(
                    Fallible(*anyopaque),
                    struct {
                        fn map(v: Result) Fallible(*anyopaque) {
                            const x = if (@typeInfo(Result) == .error_union)
                                v catch |err| return Fallible(?*anyopaque).wrap(err)
                            else
                                v;
                            return Fallible(*anyopaque).wrap(@ptrCast(x));
                        }
                    }.map,
                ).intoFuture();
                return mapped_fut.enqueue(null) catch {
                    mapped_fut.deinit();
                    return EnqueueError(*anyopaque);
                };
            }
            fn wrapDeinit(
                context: *const modules.OpaqueInstance,
                symbol_ptr: *anyopaque,
            ) callconv(.c) void {
                return deinitFn(context, @ptrCast(@alignCast(symbol_ptr)));
            }
        };

        const exp = Builder.SymbolExport{
            .symbol = options.symbol,
            .name = options.name,
            .linkage = options.linkage,
            .value = .{
                .dynamic = .{
                    .initFn = &Wrapper.wrapInit,
                    .deinitFn = &Wrapper.wrapDeinit,
                },
            },
        };
        return self.withExportInner(exp);
    }

    fn withModifierInner(
        comptime self: Builder,
        comptime modifier: Builder.Modifier,
    ) Builder {
        var modifiers: [self.modifiers.len + 1]Builder.Modifier = undefined;
        @memcpy(modifiers[0..self.modifiers.len], self.modifiers);
        modifiers[self.modifiers.len] = modifier;

        var x = self;
        x.modifiers = &modifiers;
        return x;
    }

    /// Adds a state to the module.
    pub fn withState(
        comptime self: Builder,
        comptime Fut: type,
        comptime initFn: fn (*const modules.OpaqueInstance, modules.LoadingSet) Fut,
        comptime deinitFn: anytype,
    ) Builder {
        const Result = Fut.Result;
        const T: type = switch (@typeInfo(Result)) {
            .error_union => |x| blk: {
                const payload = x.payload;
                const info = @typeInfo(payload);
                std.debug.assert(payload == *info.pointer.child);
                break :blk info.pointer.child;
            },
            .pointer => |x| blk: {
                std.debug.assert(Result == *x.child);
                break :blk x.child;
            },
            else => @compileError("fimo: invalid return type, expected `*T` or `err!*T`, found " ++ @typeName(Result)),
        };
        const Wrapper = struct {
            fn wrapInit(
                context: *const modules.OpaqueInstance,
                set: modules.LoadingSet,
            ) callconv(.c) tasks.EnqueuedFuture(Fallible(?*anyopaque)) {
                const fut = initFn(context, set).intoFuture();
                var mapped_fut = fut.map(
                    Fallible(?*anyopaque),
                    struct {
                        fn f(v: Result) Fallible(?*anyopaque) {
                            const x = if (@typeInfo(Result) == .error_union)
                                v catch |err| return Fallible(?*anyopaque).wrap(err)
                            else
                                v;
                            return Fallible(?*anyopaque).wrap(@ptrCast(x));
                        }
                    }.f,
                ).intoFuture();
                return mapped_fut.enqueue(null) catch {
                    mapped_fut.deinit();
                    return EnqueueError(?*anyopaque);
                };
            }

            fn wrapDeinit(
                context: *const modules.OpaqueInstance,
                data: ?*anyopaque,
            ) callconv(.c) void {
                const f: fn (*const modules.OpaqueInstance, *T) void = deinitFn;
                const state: *T = @ptrCast(@alignCast(data));
                f(context, state);
            }
        };

        if (self.instance_state != null) @compileError("a state has already been assigned");
        var x = self;
        x.stateType = T;
        x.instance_state = .{
            .init = &Wrapper.wrapInit,
            .deinit = &Wrapper.wrapDeinit,
        };
        return x.withModifierInner(.{ .instance_state = {} });
    }

    pub fn withOnStartEvent(
        comptime self: Builder,
        comptime Fut: type,
        comptime f: fn (ctx: *const modules.OpaqueInstance) Fut,
    ) Builder {
        const Result = Fut.Result;
        const T: type = switch (@typeInfo(Result)) {
            .error_union => |x| x.payload,
            else => Result,
        };
        std.debug.assert(T == void);
        const Wrapper = struct {
            fn wrapF(
                context: *const modules.OpaqueInstance,
            ) callconv(.c) tasks.EnqueuedFuture(Fallible(void)) {
                const fut = f(context).intoFuture();
                var mapped_fut = fut.map(
                    Fallible(void),
                    Fallible(void).wrap,
                ).intoFuture();
                return mapped_fut.enqueue(null) catch {
                    mapped_fut.deinit();
                    return EnqueueError(void);
                };
            }
        };

        if (self.start_event != null)
            @compileError("the `on_start` event is already defined");

        var x = self;
        x.start_event = .{ .on_event = &Wrapper.wrapF };
        return x.withModifierInner(.{ .start_event = {} });
    }

    /// Adds an `on_stop` event to the module.
    pub fn withOnStopEvent(
        comptime self: Builder,
        comptime f: fn (ctx: *const modules.OpaqueInstance) void,
    ) Builder {
        if (self.stop_event != null)
            @compileError("the `on_stop` event is already defined");

        const wrapped = struct {
            fn wrapper(context: *const modules.OpaqueInstance) callconv(.c) void {
                f(context);
            }
        }.wrapper;

        var x = self;
        x.stop_event = .{ .on_event = &wrapped };
        return x.withModifierInner(.{ .stop_event = {} });
    }

    fn ParameterTable(comptime self: Builder) type {
        if (self.parameters.len == 0) return void;
        var fields: [self.parameters.len]std.builtin.Type.StructField = undefined;
        for (self.parameters, &fields) |p, *f| {
            const pType = switch (p.default_value) {
                .u8 => u8,
                .u16 => u16,
                .u32 => u32,
                .u64 => u64,
                .i8 => i8,
                .i16 => i16,
                .i32 => i32,
                .i64 => i64,
            };
            f.* = std.builtin.Type.StructField{
                .name = p.member_name,
                .type = *modules.Parameter(pType),
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*anyopaque),
            };
        }
        const t: std.builtin.Type = .{
            .@"struct" = std.builtin.Type.Struct{
                .layout = .@"extern",
                .fields = &fields,
                .decls = &.{},
                .is_tuple = false,
            },
        };
        return @Type(t);
    }

    fn ResourceTable(comptime self: Builder) type {
        const PathWrapper = extern struct {
            ffi: paths.compat.Path,
            pub fn getCStr(this: @This()) [:0]const u8 {
                return this.ffi.path[0..this.ffi.length :0];
            }
            pub fn getPath(this: @This()) paths.Path {
                return paths.Path.initC(this.ffi);
            }
            pub fn format(this: @This(), w: *std.Io.Writer) std.Io.Writer.Error!void {
                try this.getPath().format(w);
            }
        };

        if (self.resources.len == 0) return void;
        var fields: [self.resources.len]std.builtin.Type.StructField = undefined;
        for (self.resources, &fields) |x, *f| {
            f.* = std.builtin.Type.StructField{
                .name = x.name,
                .type = PathWrapper,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf([*:0]const u8),
            };
        }
        const t: std.builtin.Type = .{
            .@"struct" = std.builtin.Type.Struct{
                .layout = .@"extern",
                .fields = &fields,
                .decls = &.{},
                .is_tuple = false,
            },
        };
        return @Type(t);
    }

    fn ImportTable(comptime self: Builder) type {
        if (self.imports.len == 0) return void;
        var fields: [self.imports.len]std.builtin.Type.StructField = undefined;
        for (self.imports, &fields, 0..) |x, *f, i| {
            @setEvalBranchQuota(10_000);
            var num_buf: [128]u8 = undefined;
            f.* = std.builtin.Type.StructField{
                .name = x.name orelse (std.fmt.bufPrintZ(&num_buf, "{d}", .{i}) catch unreachable),
                .type = *const x.symbol.T,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*const anyopaque),
            };
        }
        const t: std.builtin.Type = .{
            .@"struct" = std.builtin.Type.Struct{
                .layout = .@"extern",
                .fields = &fields,
                .decls = &.{},
                .is_tuple = false,
            },
        };
        return @Type(t);
    }

    fn ExportsTable(comptime self: Builder) type {
        if (self.exports.len == 0) return void;
        var i: usize = 0;
        var fields: [self.exports.len]std.builtin.Type.StructField = undefined;
        for (self.exports) |x| {
            @setEvalBranchQuota(10_000);
            if (x.value != .static) continue;
            var num_buf: [128]u8 = undefined;
            fields[i] = std.builtin.Type.StructField{
                .name = x.name orelse (std.fmt.bufPrintZ(&num_buf, "{d}", .{i}) catch unreachable),
                .type = *const x.symbol.T,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*const anyopaque),
            };
            i += 1;
        }
        for (self.exports) |x| {
            if (x.value != .dynamic) continue;
            var num_buf: [128]u8 = undefined;
            fields[i] = std.builtin.Type.StructField{
                .name = x.name orelse (std.fmt.bufPrintZ(&num_buf, "{d}", .{i}) catch unreachable),
                .type = *const x.symbol.T,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*const anyopaque),
            };
            i += 1;
        }
        const t: std.builtin.Type = .{
            .@"struct" = std.builtin.Type.Struct{
                .layout = .@"extern",
                .fields = &fields,
                .decls = &.{},
                .is_tuple = false,
            },
        };
        return @Type(t);
    }

    fn SymbolProvider(comptime self: Builder) fn (anytype, comptime modules.Symbol) *const anyopaque {
        const SymbolInfo = struct {
            name: [:0]const u8,
            symbol: modules.Symbol,
            location: enum { imp, exp },
        };
        var infos: [self.imports.len + self.exports.len]SymbolInfo = undefined;
        for (infos[0..self.imports.len], self.imports, 0..) |*info, sym, i| {
            @setEvalBranchQuota(10_000);
            info.* = .{
                .name = sym.name orelse std.fmt.comptimePrint("{d}", .{i})[0..],
                .symbol = sym.symbol,
                .location = .imp,
            };
        }
        var i: usize = 0;
        for (infos[self.imports.len..], self.exports) |*info, sym| {
            @setEvalBranchQuota(10_000);
            if (sym.value != .static) continue;
            info.* = .{
                .name = sym.name orelse std.fmt.comptimePrint("{d}", .{i})[0..],
                .symbol = sym.symbol,
                .location = .exp,
            };
            i += 1;
        }
        for (infos[self.imports.len..], self.exports) |*info, sym| {
            @setEvalBranchQuota(10_000);
            if (sym.value != .dynamic) continue;
            info.* = .{
                .name = sym.name orelse std.fmt.comptimePrint("{d}", .{i})[0..],
                .symbol = sym.symbol,
                .location = .exp,
            };
            i += 1;
        }
        const infos_c = infos;
        return struct {
            fn provide(instance: anytype, comptime symbol: modules.Symbol) *const anyopaque {
                const name, const location = comptime blk: {
                    for (&infos_c) |info| {
                        if (std.mem.eql(u8, info.symbol.name, symbol.name) and
                            std.mem.eql(u8, info.symbol.namespace, symbol.namespace) and
                            info.symbol.version.isCompatibleWith(symbol.version))
                        {
                            break :blk .{ info.name, info.location };
                        }
                    }
                    @compileError(std.fmt.comptimePrint(
                        "the instance does not provide the symbol {}",
                        .{symbol},
                    ));
                };
                return if (comptime location == .imp)
                    @field(instance.imports(), name)
                else
                    @field(instance.exports(), name);
            }
        }.provide;
    }

    fn ffiParameters(comptime self: Builder) []const ExportsRoot.Parameter {
        var parameters: [self.parameters.len]ExportsRoot.Parameter = undefined;
        for (self.parameters, &parameters) |src, *dst| {
            dst.* = ExportsRoot.Parameter{
                .name = src.name ++ "\x00",
                .read_group = src.read_group,
                .write_group = src.write_group,
                .read = src.read,
                .write = src.write,
                .type = switch (src.default_value) {
                    .u8 => .u8,
                    .u16 => .u16,
                    .u32 => .u32,
                    .u64 => .u64,
                    .i8 => .i8,
                    .i16 => .i16,
                    .i32 => .i32,
                    .i64 => .i64,
                },
                .default_value = switch (src.default_value) {
                    .u8 => |v| .{ .u8 = v },
                    .u16 => |v| .{ .u16 = v },
                    .u32 => |v| .{ .u32 = v },
                    .u64 => |v| .{ .u64 = v },
                    .i8 => |v| .{ .i8 = v },
                    .i16 => |v| .{ .i16 = v },
                    .i32 => |v| .{ .i32 = v },
                    .i64 => |v| .{ .i64 = v },
                },
            };
        }
        const parameters_c = parameters;
        return &parameters_c;
    }

    fn ffiResources(comptime self: Builder) []const ExportsRoot.Resource {
        var resources: [self.resources.len]ExportsRoot.Resource = undefined;
        for (self.resources, &resources) |src, *dst| {
            dst.* = ExportsRoot.Resource{
                .path = src.path.raw ++ "\x00",
            };
        }
        const resources_c = resources;
        return &resources_c;
    }

    fn ffiNamespaces(comptime self: Builder) []const Namespace {
        var namespaces: [self.namespaces.len]Namespace = undefined;
        for (self.namespaces, &namespaces) |src, *dst| {
            dst.* = Namespace{
                .name = src.name,
            };
        }
        const namespaces_c = namespaces;
        return &namespaces_c;
    }

    fn ffiImports(comptime self: Builder) []const ExportsRoot.SymbolImport {
        var imports: [self.imports.len]ExportsRoot.SymbolImport = undefined;
        for (self.imports, &imports) |src, *dst| {
            dst.* = ExportsRoot.SymbolImport{
                .name = src.symbol.name,
                .namespace = src.symbol.namespace,
                .version = src.symbol.version.intoC(),
            };
        }
        const imports_c = imports;
        return &imports_c;
    }

    fn ffiExports(comptime self: Builder) []const ExportsRoot.SymbolExport {
        var count: usize = 0;
        for (self.exports) |exp| {
            if (exp.value != .static) continue;
            count += 1;
        }

        var i: usize = 0;
        var exports: [count]ExportsRoot.SymbolExport = undefined;
        for (self.exports) |src| {
            if (src.value != .static) continue;
            exports[i] = ExportsRoot.SymbolExport{
                .symbol = src.value.static,
                .linkage = src.linkage,
                .version = src.symbol.version.intoC(),
                .name = src.symbol.name,
                .namespace = src.symbol.namespace,
            };
            i += 1;
        }
        const exports_c = exports;
        return &exports_c;
    }

    fn ffiDynamicExports(comptime self: Builder) []const DynamicSymbolExport {
        var count: usize = 0;
        for (self.exports) |exp| {
            if (exp.value != .dynamic) continue;
            count += 1;
        }

        var i: usize = 0;
        var exports: [count]DynamicSymbolExport = undefined;
        for (self.exports) |src| {
            if (src.value != .dynamic) continue;
            exports[i] = DynamicSymbolExport{
                .constructor = src.value.dynamic.initFn,
                .destructor = src.value.dynamic.deinitFn,
                .linkage = src.linkage,
                .version = src.symbol.version.intoC(),
                .name = src.symbol.name,
                .namespace = src.symbol.namespace,
            };
            i += 1;
        }
        const exports_c = exports;
        return &exports_c;
    }

    fn ffiModifiers(comptime self: Builder) []const ExportsRoot.Modifier {
        var modifiers: [self.modifiers.len]ExportsRoot.Modifier = undefined;
        for (self.modifiers, &modifiers) |src, *dst| {
            switch (src) {
                .instance_state => {
                    const instance_state = self.instance_state.?;
                    dst.* = .{
                        .tag = .instance_state,
                        .value = .{
                            .instance_state = &instance_state,
                        },
                    };
                },
                .start_event => {
                    const start_event = self.start_event.?;
                    dst.* = .{
                        .tag = .start_event,
                        .value = .{
                            .start_event = &start_event,
                        },
                    };
                },
                .stop_event => {
                    const stop_event = self.stop_event.?;
                    dst.* = .{
                        .tag = .stop_event,
                        .value = .{
                            .stop_event = &stop_event,
                        },
                    };
                },
                else => @compileError("unknown modifier"),
            }
        }

        const modifiers_c: [self.modifiers.len]ExportsRoot.Modifier = modifiers;
        return &modifiers_c;
    }

    /// Exports the module with the specified configuration.
    pub fn exportModule(comptime self: Builder) type {
        const parameters = self.ffiParameters();
        const resources = self.ffiResources();
        const namespaces = self.ffiNamespaces();
        const imports = self.ffiImports();
        const exports = self.ffiExports();
        const dynamic_exports = self.ffiDynamicExports();
        const modifiers = self.ffiModifiers();
        const exp = &Export{
            .name = self.name,
            .description = self.description orelse null,
            .author = self.author orelse null,
            .license = self.license orelse null,
            .parameters = if (parameters.len > 0) parameters.ptr else null,
            .parameters_count = parameters.len,
            .resources = if (resources.len > 0) resources.ptr else null,
            .resources_count = resources.len,
            .namespace_imports = if (namespaces.len > 0) namespaces.ptr else null,
            .namespace_imports_count = namespaces.len,
            .symbol_imports = if (imports.len > 0) imports.ptr else null,
            .symbol_imports_count = imports.len,
            .symbol_exports = if (exports.len > 0) exports.ptr else null,
            .symbol_exports_count = exports.len,
            .dynamic_symbol_exports = if (dynamic_exports.len > 0) dynamic_exports.ptr else null,
            .dynamic_symbol_exports_count = dynamic_exports.len,
            .modifiers = if (modifiers.len > 0) modifiers.ptr else null,
            .modifiers_count = modifiers.len,
        };
        embedStaticModuleExport(exp);

        return modules.Instance(.{
            .ParametersType = self.ParameterTable(),
            .ResourcesType = self.ResourceTable(),
            .ImportsType = self.ImportTable(),
            .ExportsType = self.ExportsTable(),
            .StateType = self.stateType,
            .@"export" = exp,
            .provider = self.SymbolProvider(),
        });
    }
};

const exports_section = switch (builtin.target.os.tag) {
    .macos, .ios, .watchos, .tvos, .visionos => struct {
        const start_exports = @extern(
            [*]const ?*const Export,
            .{ .name = "_start_fimo_module" },
        );
        const stop_exports = @extern(
            [*]const ?*const Export,
            .{ .name = "_stop_fimo_module" },
        );
        const export_visibility = .hidden;
        const section_name = "__DATA,fimo_module";

        // Make shure that the section is created.
        comptime {
            asm (
                \\.pushsection __DATA,fimo_module,regular,no_dead_strip
                \\.align 8
                \\.quad 0
                \\.popsection
                \\
                \\.no_dead_strip __start_fimo_module
                \\.global __start_fimo_module
                \\__start_fimo_module = section$start$__DATA$fimo_module
                \\
                \\.no_dead_strip __stop_fimo_module
                \\.global __stop_fimo_module
                \\__stop_fimo_module = section$end$__DATA$fimo_module
            );
            embedStaticModuleExport(null);
        }
    },
    .windows => struct {
        const a: ?*const Export = null;
        const z: ?*const Export = null;

        const start_exports: [*]const ?*const Export = @ptrCast(&a);
        const stop_exports: [*]const ?*const Export = @ptrCast(&z);
        const export_visibility = .default;
        const section_name = "fi_mod$u";

        // Create the section.
        comptime {
            @export(&a, .{
                .name = "module_export_" ++ @typeName(@This()) ++ "start",
                .section = "fi_mod$a",
                .linkage = .strong,
                .visibility = .default,
            });
            @export(&z, .{
                .name = "module_export_" ++ @typeName(@This()) ++ "end",
                .section = "fi_mod$z",
                .linkage = .strong,
                .visibility = .default,
            });
        }
    },
    else => struct {
        extern const __start_fimo_module: ?*const Export;
        extern const __stop_fimo_module: ?*const Export;

        const start_exports: [*]const ?*const Export = @ptrCast(
            &__start_fimo_module,
        );
        const stop_exports: [*]const ?*const Export = @ptrCast(
            &__stop_fimo_module,
        );
        const export_visibility = .hidden;
        const section_name = "fimo_module";

        // Make shure that the section is created.
        comptime {
            embedStaticModuleExport(null);
            asm (
                \\.pushsection .init_array,"aw",%init_array
                \\.reloc ., BFD_RELOC_NONE, fimo_module
                \\.popsection
            );
        }
    },
};

/// Iterator over all exports of the current binary.
pub const ExportIter = struct {
    /// Iterator position. Does not necessarily point to a valid export.
    exports: [*]const ?*const Export = exports_section.start_exports,

    /// Returns the next export in the export link.
    pub fn next(self: *@This()) ?*const Export {
        while (self.exports != exports_section.stop_exports) {
            const element_ptr = self.exports;
            self.exports += 1;

            const element = element_ptr[0];
            if (element != null) return element;
        }
        return null;
    }

    pub export fn fimo_impl_module_export_iterator(
        inspector: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
        data: ?*anyopaque,
    ) void {
        var it = ExportIter{};
        while (it.next()) |exp| {
            if (!inspector(exp, data)) {
                return;
            }
        }
    }
};

/// Creates a new unique export in the correct section.
///
/// For internal use only, as the pointer should not generally be null.
fn embedStaticModuleExport(comptime module: ?*const Export) void {
    const name = if (module) |m| std.mem.span(m.name) else "unknown";
    _ = struct {
        const data = module;
        comptime {
            @export(&data, .{
                .name = "module_export_" ++ name ++ "_" ++ @typeName(@This()),
                .section = exports_section.section_name,
                .linkage = .strong,
                .visibility = exports_section.export_visibility,
            });
        }
    };
}
