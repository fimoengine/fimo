//! Module export utilities.
const std = @import("std");
const builtin = @import("builtin");

const AnyError = @import("../../../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const c = @import("../../../c.zig");
const Path = @import("../../../path.zig").Path;
const Version = @import("../../../Version.zig");
const Context = @import("../../proxy_context.zig");
const Async = @import("../async.zig");
const Fallible = Async.Fallible;
const EnqueuedFuture = Async.EnqueuedFuture;
const Module = @import("../module.zig");

const Self = @This();

/// Declaration of a module parameter.
pub const Parameter = extern struct {
    type: Module.ParameterType,
    read_group: Module.ParameterAccessGroup = .private,
    write_group: Module.ParameterAccessGroup = .private,
    read: ?*const fn (
        data: Module.OpaqueParameterData,
        value: *anyopaque,
    ) callconv(.c) void = null,
    write: ?*const fn (
        data: Module.OpaqueParameterData,
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
    version: c.FimoVersion,
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
    version: c.FimoVersion,
    name: [*:0]const u8,
    namespace: [*:0]const u8 = "",
};

/// Declaration of a dynamic module symbol export.
pub const DynamicSymbolExport = extern struct {
    constructor: *const fn (
        ctx: *const Module.OpaqueInstance,
    ) callconv(.c) EnqueuedFuture(Fallible(*anyopaque)),
    destructor: *const fn (
        ctx: *const Module.OpaqueInstance,
        symbol: *anyopaque,
    ) callconv(.c) void,
    linkage: SymbolLinkage,
    version: c.FimoVersion,
    name: [*:0]const u8,
    namespace: [*:0]const u8 = "",
};

/// A modifier declaration for a module export.
pub const Modifier = extern struct {
    tag: enum(i32) {
        destructor,
        dependency,
        debug_info,
        instance_state,
        start_event,
        stop_event,
        _,
    },
    value: extern union {
        destructor: *const Destructor,
        dependency: *const Dependency,
        debug_info: *const DebugInfo,
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
    pub const Dependency = Module.Info;

    /// Accessor to the debug info of a module.
    ///
    /// The subsystem may utilize the debug info to provide additional features, like symbol
    /// tracing. May only be specified once.
    pub const DebugInfo = extern struct {
        data: ?*anyopaque,
        construct: *const fn (ptr: ?*anyopaque, info: *Module.DebugInfo) callconv(.c) AnyResult,
    };

    /// A constructor and destructor for the state of a module.
    ///
    /// Can be specified to bind a state to an instance. The constructor will be called before the
    /// modules exports are initialized and returning an error will abort the loading of the
    /// instance. Inversely, the destructor function will be called after all exports have been
    /// deinitialized. May only be specified once.
    pub const InstanceState = extern struct {
        init: *const fn (
            ctx: *const Module.OpaqueInstance,
            set: Module.LoadingSet,
        ) callconv(.c) EnqueuedFuture(Fallible(?*anyopaque)),
        deinit: *const fn (
            ctx: *const Module.OpaqueInstance,
            state: ?*anyopaque,
        ) callconv(.c) void,
    };

    /// A listener for the start event of the instance.
    ///
    /// The event will be dispatched immediately after the instance has been loaded. An error will
    /// result in the destruction of the instance. May only be specified once.
    pub const StartEvent = extern struct {
        on_event: *const fn (ctx: *const Module.OpaqueInstance) callconv(.c) EnqueuedFuture(Fallible(void)),
    };

    /// A listener for the stop event of the instance.
    ///
    /// The event will be dispatched immediately before any exports are deinitialized. May only be
    /// specified once.
    pub const StopEvent = extern struct {
        on_event: *const fn (ctx: *const Module.OpaqueInstance) callconv(.c) void,
    };
};

/// Declaration of a module export.
pub const Export = extern struct {
    next: ?*Context.TaggedInStruct = null,
    version: c.FimoVersion = Context.context_version.intoC(),
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
                .debug_info, .instance_state, .start_event, .stop_event => {},
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

    /// Returns the debug info modifier, if specified.
    pub fn getDebugInfoModifier(self: *const Export) ?*const Modifier.DebugInfo {
        for (self.getModifiers()) |mod| {
            if (mod.tag == .debug_info) return mod.value.debug_info;
        }
        return null;
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
    modifiers: []const Builder.Modifier = if (!builtin.strip_debug_info)
        &.{.{ .debug_info = {} }}
    else
        &.{},
    debug_info: ?Module.DebugInfo.Builder = if (!builtin.strip_debug_info) .{} else null,
    stateType: type = void,
    instance_state: ?Self.Modifier.InstanceState = null,
    start_event: ?Self.Modifier.StartEvent = null,
    stop_event: ?Self.Modifier.StopEvent = null,

    const Parameter = struct {
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
        read_group: Module.ParameterAccessGroup = .private,
        write_group: Module.ParameterAccessGroup = .private,
        read: ?*const fn (
            data: Module.OpaqueParameterData,
            value: *anyopaque,
        ) callconv(.c) void = null,
        write: ?*const fn (
            data: Module.OpaqueParameterData,
            value: *const anyopaque,
        ) callconv(.c) void = null,
    };

    const Resource = struct {
        name: [:0]const u8,
        path: Path,
    };

    const SymbolImport = struct {
        name: [:0]const u8,
        symbol: Module.Symbol,
    };

    const SymbolExport = struct {
        symbol: Module.Symbol,
        name: [:0]const u8,
        linkage: SymbolLinkage,
        value: union(enum) {
            static: *const anyopaque,
            dynamic: struct {
                initFn: *const fn (
                    ctx: *const Module.OpaqueInstance,
                ) callconv(.c) EnqueuedFuture(Fallible(*anyopaque)),
                deinitFn: *const fn (
                    ctx: *const Module.OpaqueInstance,
                    symbol: *anyopaque,
                ) callconv(.c) void,
            },
        },
    };

    const Modifier = union(enum) {
        debug_info: void,
        instance_state: void,
        start_event: void,
        stop_event: void,
        _,
    };

    fn EnqueueError(comptime T: type) Async.EnqueuedFuture(Fallible(T)) {
        return .{
            .data = @constCast(@ptrCast(&{})),
            .poll_fn = &struct {
                fn f(
                    data: **anyopaque,
                    waker: Async.Waker,
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

        var paths: [self.resources.len + 1]Builder.Resource = undefined;
        @memcpy(paths[0..self.resources.len], self.resources);
        paths[self.resources.len] = resource;

        var x = self;
        x.resources = &paths;
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

    /// Adds an import to the module.
    ///
    /// Automatically imports the required namespace.
    pub fn withImport(
        comptime self: Builder,
        comptime import: Builder.SymbolImport,
    ) Builder {
        for (self.imports) |imp| {
            if (std.mem.eql(u8, imp.name, import.name))
                @compileError(
                    std.fmt.comptimePrint(
                        "duplicate import member name: '{s}'",
                        .{imp.name},
                    ),
                );
        }

        var imports: [self.imports.len + 1]Builder.SymbolImport = undefined;
        @memcpy(imports[0..self.imports.len], self.imports);
        imports[self.imports.len] = import;

        var x = self.withNamespace(import.symbol.namespace);
        x.imports = &imports;
        if (x.debug_info) |*info| info.addImport(import.symbol.T);
        return x;
    }

    fn withExportInner(
        comptime self: Builder,
        comptime @"export": Builder.SymbolExport,
    ) Builder {
        for (self.exports) |exp| {
            if (std.mem.eql(u8, exp.name, @"export".name))
                @compileError(
                    std.fmt.comptimePrint(
                        "duplicate export member name: '{s}'",
                        .{exp.name},
                    ),
                );
        }

        var exports: [self.exports.len + 1]Builder.SymbolExport = undefined;
        @memcpy(exports[0..self.exports.len], self.exports);
        exports[self.exports.len] = @"export";

        var x = self;
        x.exports = &exports;
        if (x.debug_info) |*info| {
            switch (@"export".value) {
                .static => info.addExport(@"export".symbol.T),
                .dynamic => info.addDynamicExport(@"export".symbol.T),
            }
        }
        return x;
    }

    /// Adds a static export to the module.
    pub fn withExport(
        comptime self: Builder,
        comptime symbol: Module.Symbol,
        comptime name: [:0]const u8,
        comptime linkage: SymbolLinkage,
        comptime value: *const symbol.T,
    ) Builder {
        const exp = Builder.SymbolExport{
            .symbol = symbol,
            .name = name,
            .linkage = linkage,
            .value = .{ .static = value },
        };
        return self.withExportInner(exp);
    }

    /// Adds a static export to the module.
    pub fn withDynamicExport(
        comptime self: Builder,
        comptime symbol: Module.Symbol,
        comptime name: [:0]const u8,
        comptime linkage: SymbolLinkage,
        comptime Fut: type,
        comptime initFn: fn (ctx: *const Module.OpaqueInstance) Fut,
        comptime deinitFn: fn (ctx: *const Module.OpaqueInstance, symbol: *symbol.T) void,
    ) Builder {
        const Result = Fut.Result;
        switch (@typeInfo(Result)) {
            .error_union => |x| std.debug.assert(x.payload == *symbol.T),
            else => std.debug.assert(Result == *symbol.T),
        }
        const Wrapper = struct {
            fn wrapInit(
                ctx: *const Module.OpaqueInstance,
            ) callconv(.c) Async.EnqueuedFuture(Fallible(*anyopaque)) {
                const fut = initFn(ctx).intoFuture();
                var mapped_fut = fut.map(
                    Fallible(*anyopaque),
                    struct {
                        fn map(v: Result) Fallible(*anyopaque) {
                            const x = v catch |err| return Fallible(*anyopaque).wrap(err);
                            return Fallible(*anyopaque).wrap(@ptrCast(x));
                        }
                    }.map,
                ).intoFuture();
                var err: ?AnyError = null;
                return mapped_fut.enqueue(ctx.context().@"async"(), null, &err) catch {
                    mapped_fut.deinit();
                    err.?.deinit();
                    return EnqueueError(*anyopaque);
                };
            }
            fn wrapDeinit(
                ctx: *const Module.OpaqueInstance,
                symbol_ptr: *anyopaque,
            ) callconv(.c) void {
                return deinitFn(ctx, @alignCast(@ptrCast(symbol_ptr)));
            }
        };

        const exp = Builder.SymbolExport{
            .symbol = symbol,
            .name = name,
            .linkage = linkage,
            .value = .{
                .dynamic = .{
                    .initFn = &Wrapper.wrapInit,
                    .deinitFn = &Wrapper.wrapDeinit,
                },
            },
        };
        return self.withExportInner(exp);
    }

    /// Adds a static export to the module.
    pub fn withDynamicExportSync(
        comptime self: Builder,
        comptime symbol: Module.Symbol,
        comptime name: [:0]const u8,
        comptime linkage: SymbolLinkage,
        comptime initFn: fn (ctx: *const Module.OpaqueInstance) anyerror!*symbol.T,
        comptime deinitFn: fn (ctx: *const Module.OpaqueInstance, symbol: *symbol.T) void,
    ) Builder {
        const Wrapper = struct {
            instance: *const Module.OpaqueInstance,

            const Result = anyerror!*symbol.T;
            const Future = Async.Future(@This(), Result, poll, null);

            fn init(instance: *const Module.OpaqueInstance) Future {
                return Future.init(.{ .instance = instance });
            }

            fn poll(this: *@This(), waker: Async.Waker) Async.Poll(anyerror!*symbol.T) {
                _ = waker;
                return .{ .ready = initFn(this.instance) };
            }
        };
        return self.withDynamicExport(
            symbol,
            name,
            linkage,
            Wrapper.Future,
            Wrapper.init,
            deinitFn,
        );
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

    /// Ensures that the module embeds the debug info.
    ///
    /// The debug info is embedded automatically, whenever the plugin is built with debug info.
    pub fn withDebugInfo(comptime self: Builder) Builder {
        if (self.debug_info != null) return self;

        var debug_info = Module.DebugInfo.Builder{};
        for (self.imports) |sym| debug_info.addImport(sym.symbol.T);
        for (self.exports) |sym| {
            if (sym.value == .static)
                debug_info.addExport(sym.symbol.T)
            else
                debug_info.addDynamicExport(sym.symbol.T);
        }
        debug_info.addType(self.stateType);

        var x = self;
        x.debug_info = debug_info;
        return x.withModifierInner(.{ .debug_info = {} });
    }

    /// Adds a state to the module.
    pub fn withState(
        comptime self: Builder,
        comptime Fut: type,
        comptime initFn: fn (*const Module.OpaqueInstance, Module.LoadingSet) Fut,
        comptime deinitFn: anytype,
    ) Builder {
        const Result = Fut.Result;
        const T: type = switch (@typeInfo(Result)) {
            .error_union => |x| blk: {
                const payload = x.payload;
                if (@sizeOf(payload) == 0) break :blk payload;
                const info = @typeInfo(payload);
                std.debug.assert(payload == *info.pointer.child);
                break :blk info.pointer.child;
            },
            else => Result,
        };
        const Wrapper = struct {
            fn wrapInit(
                ctx: *const Module.OpaqueInstance,
                set: Module.LoadingSet,
            ) callconv(.c) Async.EnqueuedFuture(Fallible(?*anyopaque)) {
                const fut = initFn(ctx, set).intoFuture();
                var mapped_fut = fut.map(
                    Fallible(?*anyopaque),
                    struct {
                        fn f(v: Result) Fallible(?*anyopaque) {
                            if (comptime @sizeOf(T) == 0) {
                                return Fallible(?*anyopaque).wrap(@constCast(&T{}));
                            } else {
                                const x = v catch |err| return Fallible(?*anyopaque).wrap(err);
                                return Fallible(?*anyopaque).wrap(@ptrCast(x));
                            }
                        }
                    }.f,
                ).intoFuture();
                var err: ?AnyError = null;
                return mapped_fut.enqueue(ctx.context().@"async"(), null, &err) catch {
                    mapped_fut.deinit();
                    err.?.deinit();
                    return EnqueueError(?*anyopaque);
                };
            }

            fn wrapDeinit(
                ctx: *const Module.OpaqueInstance,
                data: ?*anyopaque,
            ) callconv(.c) void {
                if (comptime @sizeOf(T) == 0) {
                    const f: fn (*const Module.OpaqueInstance) void = deinitFn;
                    f(ctx);
                } else {
                    const f: fn (*const Module.OpaqueInstance, *T) void = deinitFn;
                    const state: *T = @alignCast(@ptrCast(data));
                    f(ctx, state);
                }
            }
        };

        if (self.instance_state != null) @compileError("a state has already been assigned");
        var x = self;
        x.stateType = T;
        x.instance_state = .{
            .init = &Wrapper.wrapInit,
            .deinit = &Wrapper.wrapDeinit,
        };
        if (x.debug_info) |*info| _ = info.addType(T);
        return x.withModifierInner(.{ .instance_state = {} });
    }

    /// Adds a state to the module.
    pub fn withStateSync(
        comptime self: Builder,
        comptime T: type,
        comptime initFn: anytype,
        comptime deinitFn: anytype,
    ) Builder {
        const Ret = if (comptime @sizeOf(T) == 0) void else *T;
        const Wrapper = struct {
            instance: *const Module.OpaqueInstance,
            set: Module.LoadingSet,

            const Result = anyerror!Ret;
            const Future = Async.Future(@This(), Result, poll, null);

            fn init(instance: *const Module.OpaqueInstance, set: Module.LoadingSet) Future {
                return Future.init(.{ .instance = instance, .set = set });
            }

            fn poll(this: *@This(), waker: Async.Waker) Async.Poll(anyerror!Ret) {
                _ = waker;
                if (comptime @sizeOf(T) == 0) {
                    const f: fn (*const Module.OpaqueInstance, Module.LoadingSet) anyerror!void = initFn;
                    return .{ .ready = f(this.instance, this.set) };
                } else {
                    const f: fn (*const Module.OpaqueInstance, Module.LoadingSet) anyerror!*T = initFn;
                    return .{ .ready = f(this.instance, this.set) };
                }
            }
        };
        return self.withState(Wrapper.Future, Wrapper.init, deinitFn);
    }

    pub fn withOnStartEvent(
        comptime self: Builder,
        comptime Fut: type,
        comptime f: fn (ctx: *const Module.OpaqueInstance) Fut,
    ) Builder {
        const Result = Fut.Result;
        const T: type = switch (@typeInfo(Result)) {
            .error_union => |x| x.payload,
            else => Result,
        };
        std.debug.assert(T == void);
        const Wrapper = struct {
            fn wrapF(
                ctx: *const Module.OpaqueInstance,
            ) callconv(.c) Async.EnqueuedFuture(Fallible(void)) {
                const fut = f(ctx).intoFuture();
                var mapped_fut = fut.map(
                    Fallible(void),
                    Fallible(void).wrap,
                ).intoFuture();
                var err: ?AnyError = null;
                return mapped_fut.enqueue(ctx.context().@"async"(), null, &err) catch {
                    mapped_fut.deinit();
                    err.?.deinit();
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

    /// Adds an `on_start` event to the module.
    pub fn withOnStartEventSync(
        comptime self: Builder,
        comptime f: fn (ctx: *const Module.OpaqueInstance) anyerror!void,
    ) Builder {
        const Wrapper = struct {
            instance: *const Module.OpaqueInstance,

            const Result = anyerror!void;
            const Future = Async.Future(@This(), Result, poll, null);

            fn init(instance: *const Module.OpaqueInstance) Future {
                return Future.init(.{ .instance = instance });
            }

            fn poll(this: *@This(), waker: Async.Waker) Async.Poll(anyerror!void) {
                _ = waker;
                return .{ .ready = f(this.instance) };
            }
        };
        return self.withOnStartEvent(Wrapper.Future, Wrapper.init);
    }

    /// Adds an `on_stop` event to the module.
    pub fn withOnStopEvent(
        comptime self: Builder,
        comptime f: fn (ctx: *const Module.OpaqueInstance) void,
    ) Builder {
        if (self.stop_event != null)
            @compileError("the `on_stop` event is already defined");

        const wrapped = struct {
            fn wrapper(ctx: *const Module.OpaqueInstance) callconv(.c) void {
                f(ctx);
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
                .type = *Module.Parameter(pType),
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
            ffi: c.FimoUTF8Path,
            pub fn cStr(this: @This()) [:0]const u8 {
                return this.ffi.path[0..this.ffi.length :0];
            }
            pub fn path(this: @This()) Path {
                return Path.initC(this.ffi);
            }
            pub fn format(
                this: @This(),
                comptime fmt: []const u8,
                options: std.fmt.FormatOptions,
                out_stream: anytype,
            ) !void {
                try this.path().format(fmt, options, out_stream);
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
        for (self.imports, &fields) |x, *f| {
            f.* = std.builtin.Type.StructField{
                .name = x.name,
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
            if (x.value != .static) continue;
            fields[i] = std.builtin.Type.StructField{
                .name = x.name,
                .type = *const x.symbol.T,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*const anyopaque),
            };
            i += 1;
        }
        for (self.exports) |x| {
            if (x.value != .dynamic) continue;
            fields[i] = std.builtin.Type.StructField{
                .name = x.name,
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

    fn SymbolProvider(comptime self: Builder) fn (anytype, comptime Module.Symbol) *const anyopaque {
        const SymbolInfo = struct {
            name: [:0]const u8,
            symbol: Module.Symbol,
            location: enum { imp, exp },
        };
        var infos: [self.imports.len + self.exports.len]SymbolInfo = undefined;
        for (infos[0..self.imports.len], self.imports) |*info, sym| {
            info.* = .{ .name = sym.name, .symbol = sym.symbol, .location = .imp };
        }
        for (infos[self.imports.len..], self.exports) |*info, sym| {
            info.* = .{ .name = sym.name, .symbol = sym.symbol, .location = .exp };
        }
        const infos_c = infos;
        return struct {
            fn provide(instance: anytype, comptime symbol: Module.Symbol) *const anyopaque {
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

    fn ffiParameters(comptime self: Builder) []const Self.Parameter {
        var parameters: [self.parameters.len]Self.Parameter = undefined;
        for (self.parameters, &parameters) |src, *dst| {
            dst.* = Self.Parameter{
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

    fn ffiResources(comptime self: Builder) []const Self.Resource {
        var resources: [self.resources.len]Self.Resource = undefined;
        for (self.resources, &resources) |src, *dst| {
            dst.* = Self.Resource{
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

    fn ffiImports(comptime self: Builder) []const Self.SymbolImport {
        var imports: [self.imports.len]Self.SymbolImport = undefined;
        for (self.imports, &imports) |src, *dst| {
            dst.* = Self.SymbolImport{
                .name = src.symbol.name,
                .namespace = src.symbol.namespace,
                .version = src.symbol.version.intoC(),
            };
        }
        const imports_c = imports;
        return &imports_c;
    }

    fn ffiExports(comptime self: Builder) []const Self.SymbolExport {
        var count: usize = 0;
        for (self.exports) |exp| {
            if (exp.value != .static) continue;
            count += 1;
        }

        var i: usize = 0;
        var exports: [count]Self.SymbolExport = undefined;
        for (self.exports) |src| {
            if (src.value != .static) continue;
            exports[i] = Self.SymbolExport{
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

    fn ffiModifiers(comptime self: Builder) []const Self.Modifier {
        var modifiers: [self.modifiers.len]Self.Modifier = undefined;
        for (self.modifiers, &modifiers) |src, *dst| {
            switch (src) {
                .debug_info => {
                    const debug_info = self.debug_info.?.build();
                    const construct = struct {
                        fn f(data: ?*anyopaque, info: *Module.DebugInfo) callconv(.c) AnyResult {
                            _ = data;
                            info.* = debug_info.asFfi();
                            return AnyResult.ok;
                        }
                    }.f;

                    dst.* = .{
                        .tag = .debug_info,
                        .value = .{
                            .debug_info = &.{
                                .data = null,
                                .construct = construct,
                            },
                        },
                    };
                },
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

        const modifiers_c: [self.modifiers.len]Self.Modifier = modifiers;
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

        return Module.Instance(.{
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
                .section = c.FIMO_IMPL_MODULE_SECTION,
                .linkage = .strong,
                .visibility = exports_section.export_visibility,
            });
        }
    };
}
