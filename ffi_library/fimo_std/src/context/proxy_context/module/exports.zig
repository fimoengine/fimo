//! Module export utilities.
const std = @import("std");
const builtin = @import("builtin");

const AnyError = @import("../../../AnyError.zig");
const c = @import("../../../c.zig");
const Path = @import("../../../path.zig").Path;
const Version = @import("../../../Version.zig");
const Context = @import("../../proxy_context.zig");
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

/// Declaration of a static module symbol export.
pub const SymbolExport = extern struct {
    symbol: *const anyopaque,
    version: c.FimoVersion,
    name: [*:0]const u8,
    namespace: [*:0]const u8 = "",
};

/// Declaration of a dynamic module symbol export.
pub const DynamicSymbolExport = extern struct {
    constructor: *const fn (
        ctx: *const Module.OpaqueInstance,
        symbol: **anyopaque,
    ) callconv(.c) c.FimoResult,
    destructor: *const fn (symbol: *anyopaque) callconv(.c) void,
    version: c.FimoVersion,
    name: [*:0]const u8,
    namespace: [*:0]const u8 = "",
};

/// A modifier declaration for a module export.
pub const Modifier = extern struct {
    tag: enum(i32) {
        destructor = c.FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR,
        dependency = c.FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY,
        debug_info = c.FIMO_MODULE_EXPORT_MODIFIER_DEBUG_INFO,
        _,
    },
    value: extern union {
        destructor: *const extern struct {
            data: ?*anyopaque,
            destructor: *const fn (ptr: ?*anyopaque) callconv(.c) void,
        },
        dependency: *const Module.Info,
        debug_info: *const extern struct {
            data: ?*anyopaque,
            construct: *const fn (
                ptr: ?*anyopaque,
                info: *Module.DebugInfo,
            ) callconv(.c) c.FimoResult,
        },
    },
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
    parameters_count: u32 = 0,
    resources: ?[*]const Resource = null,
    resources_count: u32 = 0,
    namespace_imports: ?[*]const Namespace = null,
    namespace_imports_count: u32 = 0,
    symbol_imports: ?[*]const SymbolImport = null,
    symbol_imports_count: u32 = 0,
    symbol_exports: ?[*]const SymbolExport = null,
    symbol_exports_count: u32 = 0,
    dynamic_symbol_exports: ?[*]const DynamicSymbolExport = null,
    dynamic_symbol_exports_count: u32 = 0,
    modifiers: ?[*]const Modifier = null,
    modifiers_count: u32 = 0,
    constructor: ?*const fn (
        ctx: *const Module.OpaqueInstance,
        set: Module.LoadingSet,
        data: *?*anyopaque,
    ) callconv(.c) c.FimoResult = null,
    destructor: ?*const fn (
        ctx: *const Module.OpaqueInstance,
        data: ?*anyopaque,
    ) callconv(.c) void = null,
    on_start_event: ?*const fn (ctx: *const Module.OpaqueInstance) callconv(.c) c.FimoResult = null,
    on_stop_event: ?*const fn (ctx: *const Module.OpaqueInstance) callconv(.c) void = null,

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
                .debug_info => {},
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
    stateType: type = void,
    constructor: ?*const fn (
        ctx: *const Module.OpaqueInstance,
        set: Module.LoadingSet,
        data: *?*anyopaque,
    ) callconv(.c) c.FimoResult = null,
    destructor: ?*const fn (
        ctx: *const Module.OpaqueInstance,
        data: ?*anyopaque,
    ) callconv(.c) void = null,
    on_start_event: ?*const fn (ctx: *const Module.OpaqueInstance) callconv(.c) c.FimoResult = null,
    on_stop_event: ?*const fn (ctx: *const Module.OpaqueInstance) callconv(.c) void = null,
    debug_info: ?Module.DebugInfo.Builder = if (!builtin.strip_debug_info) .{} else null,

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
        name: [:0]const u8,
        symbol: Module.Symbol,
        value: union(enum) {
            static: *const anyopaque,
            dynamic: struct {
                initFn: *const fn (
                    ctx: *const Module.OpaqueInstance,
                    symbol: **anyopaque,
                ) callconv(.c) c.FimoResult,
                deinitFn: *const fn (symbol: *anyopaque) callconv(.c) void,
            },
        },
    };

    const Modifier = union(enum) {
        debug_info: void,
        _,
    };

    fn State(comptime T: type) type {
        if (@sizeOf(T) == 0) {
            return struct {
                const InitFn = fn (
                    ctx: *const Module.OpaqueInstance,
                    set: Module.LoadingSet,
                ) anyerror!void;
                const DeinitFn = fn (ctx: *const Module.OpaqueInstance) void;
                fn wrapInit(comptime f: InitFn) fn (
                    ctx: *const Module.OpaqueInstance,
                    set: Module.LoadingSet,
                    data: *?*anyopaque,
                ) callconv(.c) c.FimoResult {
                    return struct {
                        fn wrapper(
                            ctx: *const Module.OpaqueInstance,
                            set: Module.LoadingSet,
                            data: *?*anyopaque,
                        ) callconv(.c) c.FimoResult {
                            f(ctx, set) catch |err| {
                                if (@errorReturnTrace()) |tr|
                                    ctx.context().tracing().emitStackTraceSimple(tr.*, @src());
                                return AnyError.initError(err).err;
                            };
                            data.* = null;
                            return AnyError.intoCResult(null);
                        }
                    }.wrapper;
                }
                fn wrapDeinit(comptime f: DeinitFn) fn (
                    ctx: *const Module.OpaqueInstance,
                    data: ?*anyopaque,
                ) callconv(.c) void {
                    return struct {
                        fn wrapper(
                            ctx: *const Module.OpaqueInstance,
                            data: ?*anyopaque,
                        ) callconv(.c) void {
                            std.debug.assert(data == null);
                            f(ctx);
                        }
                    }.wrapper;
                }
            };
        } else {
            return struct {
                const InitFn = fn (
                    ctx: *const Module.OpaqueInstance,
                    set: Module.LoadingSet,
                ) anyerror!*T;
                const DeinitFn = fn (ctx: *const Module.OpaqueInstance, state: *T) void;
                fn wrapInit(comptime f: InitFn) fn (
                    ctx: *const Module.OpaqueInstance,
                    set: Module.LoadingSet,
                    data: *?*anyopaque,
                ) callconv(.c) c.FimoResult {
                    return struct {
                        fn wrapper(
                            ctx: *const Module.OpaqueInstance,
                            set: Module.LoadingSet,
                            data: *?*anyopaque,
                        ) callconv(.c) c.FimoResult {
                            data.* = f(ctx, set) catch |err| {
                                if (@errorReturnTrace()) |tr|
                                    ctx.context().tracing().emitStackTraceSimple(tr.*, @src());
                                return AnyError.initError(err).err;
                            };
                            return AnyError.intoCResult(null);
                        }
                    }.wrapper;
                }
                fn wrapDeinit(comptime f: DeinitFn) fn (
                    ctx: *const Module.OpaqueInstance,
                    data: ?*anyopaque,
                ) callconv(.c) void {
                    return struct {
                        fn wrapper(
                            ctx: *const Module.OpaqueInstance,
                            data: ?*anyopaque,
                        ) callconv(.c) void {
                            const state: ?*T = @alignCast(@ptrCast(data));
                            f(ctx, state.?);
                        }
                    }.wrapper;
                }
            };
        }
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
        if (x.debug_info) |*info| info.addImport(import.symbol.symbol);
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
                .static => info.addExport(@"export".symbol.symbol),
                .dynamic => info.addDynamicExport(@"export".symbol.symbol),
            }
        }
        return x;
    }

    /// Adds a static export to the module.
    pub fn withExport(
        comptime self: Builder,
        comptime T: Module.Symbol,
        comptime name: [:0]const u8,
        comptime value: *const T.symbol,
    ) Builder {
        const exp = Builder.SymbolExport{
            .name = name,
            .symbol = T,
            .value = .{ .static = value },
        };
        return self.withExportInner(exp);
    }

    /// Adds a static export to the module.
    pub fn withDynamicExport(
        comptime self: Builder,
        comptime T: Module.Symbol,
        comptime name: [:0]const u8,
        comptime initFn: fn (ctx: *const Module.OpaqueInstance) anyerror!*T.symbol,
        comptime deinitFn: fn (symbol: *T.symbol) void,
    ) Builder {
        const initWrapped = struct {
            fn f(ctx: *const Module.OpaqueInstance, out: **anyopaque) callconv(.c) c.FimoResult {
                out.* = initFn(ctx) catch |err| return AnyError.initError(err).err;
                return AnyError.intoCResult(null);
            }
        }.f;
        const deinitWrapped = struct {
            fn f(symbol: *anyopaque) callconv(.c) void {
                return deinitFn(@alignCast(@ptrCast(symbol)));
            }
        }.f;
        const exp = Builder.SymbolExport{
            .name = name,
            .symbol = T,
            .value = .{
                .dynamic = .{
                    .initFn = &initWrapped,
                    .deinitFn = &deinitWrapped,
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

    /// Ensures that the module embeds the debug info.
    ///
    /// The debug info is embedded automatically, whenever the plugin is built with debug info.
    pub fn withDebugInfo(comptime self: Builder) Builder {
        if (self.debug_info != null) return self;

        var debug_info = Module.DebugInfo.Builder{};
        for (self.imports) |sym| debug_info.addImport(sym.symbol.symbol);
        for (self.exports) |sym| {
            if (sym.value == .static)
                debug_info.addExport(sym.symbol.symbol)
            else
                debug_info.addDynamicExport(sym.symbol.symbol);
        }
        debug_info.addType(self.stateType);

        var x = self;
        x.debug_info = debug_info;
        return x.withModifierInner(.{ .debug_info = {} });
    }

    /// Adds a state to the module.
    pub fn withState(
        comptime self: Builder,
        comptime T: type,
        comptime initFn: State(T).InitFn,
        comptime deinitFn: State(T).DeinitFn,
    ) Builder {
        var x = self;
        x.stateType = T;
        x.constructor = &State(T).wrapInit(initFn);
        x.destructor = &State(T).wrapDeinit(deinitFn);
        if (x.debug_info) |*info| _ = info.addType(T);
        return x;
    }

    /// Adds an `on_start` event to the module.
    pub fn withOnStartEvent(
        comptime self: Builder,
        comptime f: fn (ctx: *const Module.OpaqueInstance) anyerror!void,
    ) Builder {
        if (self.on_start_event != null)
            @compileError("the `on_start` event is already defined");

        const wrapped = struct {
            fn wrapper(ctx: *const Module.OpaqueInstance) callconv(.c) c.FimoResult {
                f(ctx) catch |err| {
                    if (@errorReturnTrace()) |tr|
                        ctx.context().tracing().emitStackTraceSimple(tr.*, @src());
                    return AnyError.initError(err).err;
                };
                return AnyError.intoCResult(null);
            }
        }.wrapper;

        var x = self;
        x.on_start_event = &wrapped;
        return x;
    }

    /// Adds an `on_stop` event to the module.
    pub fn withOnStopEvent(
        comptime self: Builder,
        comptime f: fn (ctx: *const Module.OpaqueInstance) void,
    ) Builder {
        if (self.on_stop_event != null)
            @compileError("the `on_stop` event is already defined");

        const wrapped = struct {
            fn wrapper(ctx: *const Module.OpaqueInstance) callconv(.c) void {
                f(ctx);
            }
        }.wrapper;

        var x = self;
        x.on_stop_event = &wrapped;
        return x;
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
        if (self.resources.len == 0) return void;
        var fields: [self.resources.len]std.builtin.Type.StructField = undefined;
        for (self.resources, &fields) |x, *f| {
            f.* = std.builtin.Type.StructField{
                .name = x.name,
                .type = [*:0]const u8,
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
                .type = *const x.symbol.symbol,
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
                .type = *const x.symbol.symbol,
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
                .type = *const x.symbol.symbol,
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
                .name = src.symbol.name,
                .namespace = src.symbol.namespace,
                .version = src.symbol.version.intoC(),
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
                .name = src.symbol.name,
                .namespace = src.symbol.namespace,
                .version = src.symbol.version.intoC(),
                .constructor = src.value.dynamic.initFn,
                .destructor = src.value.dynamic.deinitFn,
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
                        fn f(data: ?*anyopaque, info: *Module.DebugInfo) callconv(.c) c.FimoResult {
                            _ = data;
                            info.* = debug_info.asFfi();
                            return AnyError.intoCResult(null);
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
            .constructor = self.constructor,
            .destructor = self.destructor,
        };
        embedStaticModuleExport(exp);

        return Module.Instance(
            self.ParameterTable(),
            self.ResourceTable(),
            self.ImportTable(),
            self.ExportsTable(),
            self.stateType,
        );
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
    position: [*]const ?*const Export,

    /// Initializes the iterator. Does not need to be deinitialized.
    pub fn init() @This() {
        return .{
            .position = @ptrCast(exports_section.start_exports),
        };
    }

    /// Returns the next export in the export link.
    pub fn next(self: *@This()) ?*const Export {
        while (true) {
            if (self.position == exports_section.stop_exports) {
                return null;
            }
            const element_ptr = self.position;
            self.position += 1;

            const element = element_ptr[0];
            if (element != null) {
                return element;
            }
        }
    }

    pub export fn fimo_impl_module_export_iterator(
        inspector: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
        data: ?*anyopaque,
    ) void {
        var it = ExportIter.init();
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
    _ = struct {
        const data = module;
        comptime {
            @export(&data, .{
                .name = "module_export_" ++ @typeName(@This()),
                .section = c.FIMO_IMPL_MODULE_SECTION,
                .linkage = .strong,
                .visibility = exports_section.export_visibility,
            });
        }
    };
}
