//! Module export utilities.
const std = @import("std");
const builtin = @import("builtin");

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const ctx = @import("../ctx.zig");
const modules = @import("../modules.zig");
const paths = @import("../paths.zig");
const tasks = @import("../tasks.zig");
const Fallible = tasks.Fallible;
const OpaqueFuture = tasks.OpaqueFuture;
const utils = @import("../utils.zig");
const SliceConst = utils.SliceConst;
const Version = @import("../Version.zig");

const ExportsRoot = @This();

pub const Parameter = extern struct {
    name: SliceConst(u8),
    tag: modules.ParamTag,
    read_group: modules.ParamAccessGroup,
    write_group: modules.ParamAccessGroup,
    read: ?*const fn (data: modules.OpaqueParamData, value: *anyopaque) callconv(.c) void = null,
    write: ?*const fn (data: modules.OpaqueParamData, value: *const anyopaque) callconv(.c) void = null,
    default_value: extern union { u8: u8, u16: u16, u32: u32, u64: u64, i8: i8, i16: i16, i32: i32, i64: i64 },
};

pub const SymbolExport = extern struct {
    symbol: modules.SymbolId,
    sym_ty: enum(i32) { static = 0, dynamic = 1, _ },
    linkage: enum(i32) { global = 0, _ },
    value: extern union {
        static: *const anyopaque,
        dynamic: extern struct {
            poll_init: *const fn (
                ctx: *modules.OpaqueInstance,
                waker: tasks.Waker,
                result: *tasks.Fallible(*anyopaque),
            ) callconv(.c) bool,
            poll_deinit: ?*const fn (
                ctx: *modules.OpaqueInstance,
                waker: tasks.Waker,
                value: *anyopaque,
            ) callconv(.c) bool = null,
        },
    },
};

pub const events = struct {
    /// Common member of all module events.
    ///
    /// If a module supports an event, it must respond to the event by writing some
    /// data into the provided event buffer.
    pub const Tag = enum(i32) {
        init = 0,
        deinit = 1,
        start = 2,
        stop = 3,
        deinit_export = 4,
        dependencies = 5,
        _,
    };

    pub const Init = extern struct {
        tag: Tag = .init,
        poll: ?*const fn (
            ctx: *modules.OpaqueInstance,
            loader: *modules.Loader,
            waker: tasks.Waker,
            state: *tasks.Fallible(*anyopaque),
        ) callconv(.c) bool = null,
    };
    pub const Deinit = extern struct {
        tag: Tag = .deinit,
        poll: ?*const fn (ctx: *modules.OpaqueInstance, waker: tasks.Waker, state: *anyopaque) callconv(.c) bool = null,
    };
    pub const Start = extern struct {
        tag: Tag = .start,
        poll: ?*const fn (ctx: *modules.OpaqueInstance, waker: tasks.Waker, result: *AnyResult) callconv(.c) bool = null,
    };
    pub const Stop = extern struct {
        tag: Tag = .stop,
        poll: ?*const fn (ctx: *modules.OpaqueInstance, waker: tasks.Waker) callconv(.c) bool = null,
    };
    pub const DeinitExport = extern struct {
        tag: Tag = .deinit_export,
        data: ?*anyopaque = null,
        deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void = null,
    };
    pub const Dependencies = extern struct {
        tag: Tag = .dependencies,
        handles: SliceConst(*modules.Handle) = .fromSlice(null),
    };
};

pub const Export = extern struct {
    version: Version.compat.Version = ctx.context_version.intoC(),
    name: SliceConst(u8),
    description: SliceConst(u8) = .fromSlice(null),
    author: SliceConst(u8) = .fromSlice(null),
    license: SliceConst(u8) = .fromSlice(null),
    parameters: SliceConst(Parameter) = .fromSlice(null),
    resources: SliceConst(paths.compat.Path) = .fromSlice(null),
    namespaces: SliceConst(SliceConst(u8)) = .fromSlice(null),
    imports: SliceConst(modules.SymbolId) = .fromSlice(null),
    exports: SliceConst(SymbolExport) = .fromSlice(null),
    on_event: *const fn (module: *const Export, tag: *events.Tag) callconv(.c) void,

    pub fn eventInit(self: *const Export) events.Init {
        var event: events.Init = .{};
        self.on_event(self, &event.tag);
        return event;
    }

    pub fn eventDeinit(self: *const Export) events.Deinit {
        var event: events.Deinit = .{};
        self.on_event(self, &event.tag);
        return event;
    }

    pub fn eventStart(self: *const Export) events.Start {
        var event: events.Start = .{};
        self.on_event(self, &event.tag);
        return event;
    }

    pub fn eventStop(self: *const Export) events.Stop {
        var event: events.Stop = .{};
        self.on_event(self, &event.tag);
        return event;
    }

    pub fn eventDeinitExport(self: *const Export) events.DeinitExport {
        var event: events.DeinitExport = .{};
        self.on_event(self, &event.tag);
        return event;
    }

    pub fn eventDependencies(self: *const Export) events.Dependencies {
        var event: events.Dependencies = .{};
        self.on_event(self, &event.tag);
        return event;
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

        pub fn loaderFilter(context: void, module: *const Export) modules.Loader.FilterRequest {
            _ = context;
            inline for (@typeInfo(@TypeOf(bundled)).@"struct".fields) |f| {
                if (comptime @hasDecl(@field(bundle, f.name), "fimo_modules_marker")) {
                    if (module == @field(bundled, f.name).module_export) return .load;
                } else if (comptime @hasDecl(@field(bundle, f.name), "fimo_modules_bundle_marker")) {
                    if (@field(bundled, f.name).loaderFilter({}, module) == .load) return .load;
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
        var instance: *modules.OpaqueInstance = undefined;
    };
    comptime var name: []const u8 = undefined;
    comptime var description: ?[]const u8 = null;
    comptime var author: ?[]const u8 = null;
    comptime var license: ?[]const u8 = null;

    comptime var parameter_types: []const type = &.{};
    comptime var parameter_names: []const [:0]const u8 = &.{};
    comptime var parameter_infos: []const Parameter = &.{};

    comptime var resource_names: []const [:0]const u8 = &.{};
    comptime var resource_infos: []const paths.compat.Path = &.{};

    comptime var namespace_infos: []const SliceConst(u8) = &.{};

    comptime var import_symbols: []const modules.Symbol = &.{};
    comptime var import_infos: []const modules.SymbolId = &.{};

    comptime var export_symbols: []const modules.Symbol = &.{};
    comptime var export_infos: []const SymbolExport = &.{};

    comptime var ev_init_poll: ?*const fn (
        ctx: *modules.OpaqueInstance,
        loader: *modules.Loader,
        waker: tasks.Waker,
        state: *tasks.Fallible(*anyopaque),
    ) callconv(.c) bool = null;

    comptime var ev_deinit_poll: ?*const fn (
        ctx: *modules.OpaqueInstance,
        waker: tasks.Waker,
        state: *anyopaque,
    ) callconv(.c) bool = null;

    comptime var ev_start_poll: ?*const fn (
        ctx: *modules.OpaqueInstance,
        waker: tasks.Waker,
        result: *AnyResult,
    ) callconv(.c) bool = null;

    comptime var ev_stop_poll: ?*const fn (
        ctx: *modules.OpaqueInstance,
        waker: tasks.Waker,
    ) callconv(.c) bool = null;

    switch (@typeInfo(@TypeOf(T.fimo_module))) {
        .enum_literal => name = @tagName(T.fimo_module),
        .@"struct" => {
            const module = T.fimo_module;
            const M = @TypeOf(module);
            if (!@hasField(M, "name")) @compileError("fimo: invalid module, missing `pub const fimo_module = .{ .name = .foo_name };` declaration: " ++ @typeName(T));
            if (@typeInfo(@TypeOf(module.name)) != .enum_literal) @compileError("fimo: invalid module, expected `pub const fimo_module = .{ .name = .foo_name };` declaration, found: " ++ @typeName(@TypeOf(module.name)));

            name = @tagName(module.name);
            if (@hasField(M, "description")) description = module.description;
            if (@hasField(M, "author")) author = module.author;
            if (@hasField(M, "license")) license = module.license;
        },
        else => @compileError("fimo: invalid module, expected `pub const fimo_module = .foo_name;` declaration, found: " ++ @typeName(@TypeOf(T.mach_module))),
    }

    if (@hasDecl(T, "fimo_parameters")) {
        if (@typeInfo(@TypeOf(T.fimo_parameters)) != .@"struct") @compileError("fimo: invalid parameters, expected `pub const fimo_parameters = .{ .param = .{ ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_parameters)));
        const info = @typeInfo(@TypeOf(T.fimo_parameters)).@"struct";
        inline for (info.fields) |field| {
            const param = @field(T.fimo_parameters, field.name);
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
                fn f(p: modules.OpaqueParamData, value: *anyopaque) callconv(.c) void {
                    const func: fn (modules.ParameterData(@TypeOf(default))) @TypeOf(default) = param.read;
                    const d = modules.ParameterData(@TypeOf(default)).castFromOpaque(p);
                    const v: *@TypeOf(default) = @ptrCast(@alignCast(value));
                    v.* = func(d);
                }
            }.f else null;
            const write = if (@hasField(@TypeOf(param), "write")) &struct {
                fn f(p: modules.OpaqueParamData, value: *const anyopaque) callconv(.c) void {
                    const func: fn (modules.ParameterData(@TypeOf(default)), @TypeOf(default)) void = param.write;
                    const d = modules.ParameterData(@TypeOf(default)).castFromOpaque(p);
                    const v: *const @TypeOf(default) = @ptrCast(@alignCast(value));
                    func(d, v.*);
                }
            }.f else null;

            parameter_types = parameter_types ++ [_]type{@TypeOf(default)};
            parameter_names = parameter_names ++ [_][:0]const u8{field.name};
            parameter_infos = parameter_infos ++ [_]Parameter{.{
                .name = .fromSlice(field.name),
                .tag = switch (@TypeOf(default)) {
                    u8 => .u8,
                    u16 => .u16,
                    u32 => .u32,
                    u64 => .u64,
                    i8 => .i8,
                    i16 => .i16,
                    i32 => .i32,
                    i64 => .i64,
                    else => unreachable,
                },
                .read_group = read_group,
                .write_group = write_group,
                .read = read,
                .write = write,
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
            }};
        }
    }

    if (@hasDecl(T, "fimo_paths")) {
        if (@typeInfo(@TypeOf(T.fimo_paths)) != .@"struct") @compileError("fimo: invalid paths, expected `pub const fimo_paths = .{ .path = .{ ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_paths)));
        const info = @typeInfo(@TypeOf(T.fimo_paths)).@"struct";
        inline for (info.fields) |field| {
            const path = @field(T.fimo_paths, field.name);
            if (@TypeOf(path) != paths.Path) @compileError("fimo: invalid parameters, expected a path, found: " ++ @typeName(@TypeOf(path)));
            resource_names = resource_names ++ [_][:0]const u8{field.name};
            resource_infos = resource_infos ++ [_]paths.compat.Path{path.intoC()};
        }
    }

    if (@hasDecl(T, "fimo_imports")) {
        if (@typeInfo(@TypeOf(T.fimo_imports)) != .@"struct") @compileError("fimo: invalid imports, expected `pub const fimo_imports = .{ imp, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_imports)));
        inline for (T.fimo_imports) |imp| {
            if (@TypeOf(imp) == modules.Symbol) {
                const imp_s: modules.Symbol = imp;
                import_symbols = import_symbols ++ [_]modules.Symbol{imp_s};
                import_infos = import_infos ++ [_]modules.SymbolId{imp_s.intoId()};
            } else inline for (imp) |imp2| {
                const imp_s: modules.Symbol = imp2;
                import_symbols = import_symbols ++ [_]modules.Symbol{imp_s};
                import_infos = import_infos ++ [_]modules.SymbolId{imp_s.intoId()};
            }
        }
    }

    blk: inline for (import_symbols) |imp| {
        if (std.mem.eql(u8, imp.namespace, "")) continue;
        for (namespace_infos) |ns| if (std.mem.eql(u8, ns.intoSliceOrEmpty(), imp.namespace)) continue :blk;
        namespace_infos = namespace_infos ++ [_]SliceConst(u8){.fromSlice(imp.namespace)};
    }

    if (@hasDecl(T, "fimo_exports")) {
        if (@typeInfo(@TypeOf(T.fimo_exports)) != .@"struct") @compileError("fimo: invalid exports, expected `pub const fimo_exports = .{ .exp = .{ ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_exports)));
        const info = @typeInfo(@TypeOf(T.fimo_exports)).@"struct";
        inline for (info.fields) |field| {
            const exp = @field(T.fimo_exports, field.name);
            if (!@hasField(@TypeOf(exp), "symbol")) @compileError("fimo: invalid export, expected `pub const fimo_exports = .{ .exp = .{ .symbol = foo, ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(exp)));
            const symbol: modules.Symbol = exp.symbol;
            export_symbols = export_symbols ++ [_]modules.Symbol{symbol};
            const linkage = if (@hasField(@TypeOf(exp), "linkage")) blk2: {
                break :blk2 exp.linkage;
            } else .global;

            if (!@hasField(@TypeOf(exp), "value")) @compileError("fimo: invalid export value, expected `pub const fimo_exports = .{ .exp = .{ .value = ..., ... }, ... };` declaration, found: " ++ @typeName(@TypeOf(exp)));
            const value = exp.value;
            if (@typeInfo(@TypeOf(value)) == .pointer) {
                if (@TypeOf(value) != *const symbol.T) @compileError("fimo: invalid export value, expected `" ++ @typeName(*const symbol.T) ++ "`, found " ++ @typeName(@TypeOf(value)));
                const wrapper = struct {
                    fn pollInit(inst: *modules.OpaqueInstance, waker: tasks.Waker, result: *tasks.Fallible(*anyopaque)) callconv(.c) bool {
                        _ = inst;
                        _ = waker;
                        symbol.getGlobal().register(value);
                        result.* = .wrap(@constCast(value));
                        return true;
                    }
                    fn pollDeinit(inst: *modules.OpaqueInstance, waker: tasks.Waker, val: *anyopaque) callconv(.c) bool {
                        _ = inst;
                        _ = waker;
                        _ = val;
                        symbol.getGlobal().unregister();
                        return true;
                    }
                };

                export_infos = export_infos ++ [_]SymbolExport{.{
                    .symbol = symbol.intoId(),
                    .sym_ty = .dynamic,
                    .linkage = linkage,
                    .value = .{ .dynamic = .{
                        .poll_init = &wrapper.pollInit,
                        .poll_deinit = &wrapper.pollDeinit,
                    } },
                }};
            } else {
                const wrapper = struct {
                    const Sync = struct {
                        const Result = @typeInfo(@TypeOf(value.init)).@"fn".return_type.?;
                        fn pollInit(inst: *modules.OpaqueInstance, waker: tasks.Waker, result: *tasks.Fallible(*anyopaque)) callconv(.c) bool {
                            _ = inst;
                            _ = waker;
                            const sym: *symbol.T = if (@typeInfo(Result) == .error_union)
                                value.init() catch |err| {
                                    result.* = .wrap(err);
                                    return true;
                                }
                            else
                                value.init();
                            symbol.getGlobal().register(sym);
                            result.* = .wrap(sym);
                            return true;
                        }
                        fn pollDeinit(inst: *modules.OpaqueInstance, waker: tasks.Waker, val: *anyopaque) callconv(.c) bool {
                            _ = inst;
                            _ = waker;
                            symbol.getGlobal().unregister();
                            if (comptime @hasField(@TypeOf(value), "deinit")) {
                                const f: fn (*symbol.T) void = value.deinit;
                                f(val);
                            }
                            return true;
                        }
                    };
                    const AsyncInit = struct {
                        const Inner = @typeInfo(@TypeOf(value.init)).@"fn".return_type.?;
                        const Result = Inner.Result;
                        comptime {
                            if (Inner.Result != *symbol.T) switch (@typeInfo(Inner.Result)) {
                                .error_union => |v| {
                                    if (Inner.Result != v.error_set!*symbol.T) @compileError("fimo: invalid init return type, expected `*T` or `err!*T`, found " ++ @typeName(Inner.Result));
                                },
                                else => @compileError("fimo: invalid init return type, expected `*T` or `err!*T`, found " ++ @typeName(Inner.Result)),
                            };
                        }

                        var future: ?Inner = null;
                        fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker, result: *tasks.Fallible(*anyopaque)) callconv(.c) bool {
                            _ = inst;
                            if (future == null) future = value.init();
                            if (future) |*fut| {
                                switch (fut.poll(waker)) {
                                    .ready => |v| {
                                        future = null;
                                        const sym: *symbol.T = if (@typeInfo(Result) == .error_union)
                                            v catch |err| {
                                                result.* = .wrap(err);
                                                return true;
                                            }
                                        else
                                            v;
                                        symbol.getGlobal().register(sym);
                                        result.* = .wrap(sym);
                                        return true;
                                    },
                                    .pending => return false,
                                }
                            }
                        }
                    };
                    const AsyncDeinit = struct {
                        const Inner = @typeInfo(@TypeOf(value.deinit)).@"fn".return_type.?;
                        const Result = Inner.Result;
                        comptime {
                            if (Inner.Result != void) @compileError("fimo: invalid deinit return type, expected `void`, found " ++ @typeName(Inner.Result));
                        }

                        var future: ?Inner = null;
                        fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker, val: *anyopaque) callconv(.c) bool {
                            _ = inst;
                            _ = val;
                            if (future == null) {
                                symbol.getGlobal().unregister();
                                future = value.deinit();
                            }
                            if (future) |*fut| {
                                switch (fut.poll(waker)) {
                                    .ready => {
                                        future = null;
                                        return true;
                                    },
                                    .pending => return false,
                                }
                            }
                        }
                    };
                };

                const init_fn = if (comptime @typeInfo(@TypeOf(value.init)) == .@"fn")
                    &wrapper.Sync.pollInit
                else
                    &wrapper.AsyncInit.poll;
                const deinit_fn = if (comptime !@hasField(@TypeOf(value), "deinit") or @typeInfo(@TypeOf(value.deinit)) == .@"fn")
                    &wrapper.Sync.pollDeinit
                else
                    &wrapper.AsyncDeinit.poll;

                export_infos = export_infos ++ [_]SymbolExport{.{
                    .symbol = symbol.intoId(),
                    .sym_ty = .dynamic,
                    .linkage = linkage,
                    .value = .{ .dynamic = .{
                        .poll_init = init_fn,
                        .poll_deinit = deinit_fn,
                    } },
                }};
            }
        }
    }

    if (@hasDecl(T, "fimo_events")) {
        if (@typeInfo(@TypeOf(T.fimo_events)) != .@"struct") @compileError("fimo: invalid events, expected `pub const fimo_events = .{ .init = ..., ... };` declaration, found: " ++ @typeName(@TypeOf(T.fimo_events)));
        if (@hasField(@TypeOf(T.fimo_events), "init")) {
            const ev_init = T.fimo_events.init;
            const wrapper = struct {
                const Sync = struct {
                    const EvReturn = @typeInfo(@TypeOf(ev_init)).@"fn".return_type.?;
                    const Result = switch (@typeInfo(EvReturn)) {
                        .error_union => |v| v.error_set!void,
                        .void => void,
                        else => @compileError("fimo: invalid init return type, expected `void` or `err!void`, found " ++ @typeName(EvReturn)),
                    };

                    fn poll(inst: *modules.OpaqueInstance, loader: *modules.Loader, waker: tasks.Waker, state: *tasks.Fallible(*anyopaque)) callconv(.c) bool {
                        _ = waker;

                        if (Global.is_init) @panic("already init");
                        ctx.Handle.registerHandle(inst.ctxHandle());
                        Global.is_init = true;
                        Global.instance = inst;

                        if (@typeInfo(T) == .@"struct") inline for (@typeInfo(T).@"struct".fields) |f| {
                            if (f.default_value_ptr) |default| {
                                @field(Global.state, f.name) = @as(*const f.type, @ptrCast(@alignCast(default))).*;
                            }
                        };

                        const inst_imports = @as([*]const *const anyopaque, @ptrCast(@alignCast(inst.imports())));
                        inline for (inst_imports[0..import_infos.len], import_symbols) |imp, sym| {
                            sym.getGlobal().register(@ptrCast(@alignCast(imp)));
                        }

                        const Args = std.meta.ArgsTuple(@TypeOf(ev_init));
                        if (std.meta.fields(Args).len > 2) @compileError("fimo: invalid init event, got too many arguments, found: " ++ @typeName(@TypeOf(ev_init)));
                        var args: Args = undefined;
                        inline for (std.meta.fields(Args), 0..) |f, i| {
                            switch (f.type) {
                                *T => args[i] = &Global.state,
                                *modules.Loader => args[i] = loader,
                                else => @compileError("fimo: invalid init event, got invalid parameter type, found: " ++ @typeName(f.type)),
                            }
                        }
                        const result = @call(.auto, ev_init, args);
                        if (@typeInfo(Result) == .error_union) result catch |err| {
                            state.* = .wrap(err);
                            Global.is_init = false;
                            Global.instance = undefined;
                            return true;
                        };
                        state.* = .wrap(&Global.state);
                        return true;
                    }
                };
                const Async = struct {
                    const Inner = @typeInfo(@TypeOf(ev_init)).@"fn".return_type.?;
                    const Result = switch (@typeInfo(Inner.Result)) {
                        .error_union => |v| v.error_set!void,
                        .void => void,
                        else => @compileError("fimo: invalid init return type, expected `void` or `err!void`, found " ++ @typeName(Inner.Result)),
                    };

                    var future: ?Inner = null;
                    fn poll(inst: *modules.OpaqueInstance, loader: *modules.Loader, waker: tasks.Waker, state: *tasks.Fallible(*anyopaque)) callconv(.c) bool {
                        if (future == null) {
                            if (Global.is_init) @panic("already init");
                            ctx.Handle.registerHandle(inst.ctxHandle());
                            Global.is_init = true;
                            Global.instance = inst;

                            if (@typeInfo(T) == .@"struct") inline for (@typeInfo(T).@"struct".fields) |f| {
                                if (f.default_value_ptr) |default| {
                                    @field(Global.state, f.name) = @as(*const f.type, @ptrCast(@alignCast(default))).*;
                                }
                            };

                            const inst_imports = @as([*]const usize, inst.imports())[0..import_infos.len];
                            inline for (inst_imports, import_symbols) |imp, sym| {
                                sym.getGlobal().register(@ptrFromInt(imp));
                            }

                            const Args = std.meta.ArgsTuple(@TypeOf(ev_init));
                            if (std.meta.fields(Args).len > 2) @compileError("fimo: invalid init event, got too many arguments, found: " ++ @typeName(@TypeOf(ev_init)));
                            var args: Args = undefined;
                            inline for (std.meta.fields(Args), 0..) |f, i| {
                                switch (f.type) {
                                    *T => args[i] = &Global.state,
                                    *modules.Loader => args[i] = loader,
                                    else => @compileError("fimo: invalid init event, got invalid parameter type, found: " ++ @typeName(f.type)),
                                }
                            }
                            future = @call(.auto, ev_init, args);
                        }
                        if (future) |*fut| {
                            switch (fut.poll(waker)) {
                                .ready => |v| {
                                    future = null;
                                    if (@typeInfo(Result) == .error_union) v catch |err| {
                                        state.* = .wrap(err);
                                        Global.is_init = false;
                                        Global.instance = undefined;
                                        return true;
                                    };
                                    state.* = .wrap(&Global.state);
                                    return true;
                                },
                                .pending => return false,
                            }
                        }
                    }
                };
            };

            ev_init_poll = switch (comptime @typeInfo(@typeInfo(@TypeOf(ev_init)).@"fn".return_type.?)) {
                .void, .error_union => &wrapper.Sync.poll,
                else => &wrapper.Async.poll,
            };
        } else {
            const wrapper = struct {
                fn poll(inst: *modules.OpaqueInstance, loader: *modules.Loader, waker: tasks.Waker, state: *tasks.Fallible(*anyopaque)) callconv(.c) bool {
                    _ = loader;
                    _ = waker;

                    if (Global.is_init) @panic("already init");
                    ctx.Handle.registerHandle(inst.ctxHandle());
                    Global.is_init = true;
                    Global.instance = inst;

                    if (@typeInfo(T) == .@"struct") inline for (@typeInfo(T).@"struct".fields) |f| {
                        if (f.default_value_ptr) |default| {
                            @field(Global.state, f.name) = @as(*const f.type, @ptrCast(@alignCast(default))).*;
                        }
                    };

                    const inst_imports = @as([*]const usize, inst.imports())[0..import_infos.len];
                    inline for (inst_imports, import_symbols) |imp, sym| {
                        sym.getGlobal().register(@ptrFromInt(imp));
                    }

                    state.* = .wrap(&Global.state);
                    return true;
                }
            };
            ev_init_poll = &wrapper.poll;
        }

        if (@hasField(@TypeOf(T.fimo_events), "deinit")) {
            const ev_deinit = T.fimo_events.deinit;
            const wrapper = struct {
                const Sync = struct {
                    fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker, state: *anyopaque) callconv(.c) bool {
                        _ = inst;
                        _ = waker;
                        _ = state;

                        if (!Global.is_init) @panic("not init");
                        if (@typeInfo(@TypeOf(ev_deinit)).@"fn".params.len == 1) ev_deinit(&Global.state) else ev_deinit();
                        ctx.Handle.unregisterHandle();
                        Global.instance = undefined;
                        inline for (import_symbols) |sym| sym.getGlobal().unregister();
                        Global.state = undefined;
                        Global.is_init = false;
                        return true;
                    }
                };
                const Async = struct {
                    const Inner = @typeInfo(@TypeOf(ev_deinit)).@"fn".return_type.?;
                    comptime {
                        if (Inner.Result != void)
                            @compileError("fimo: invalid deinit return type, expected `void`, found " ++ @typeName(Inner.Result));
                    }

                    var future: ?Inner = null;
                    fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker, state: *anyopaque) callconv(.c) bool {
                        _ = inst;
                        _ = state;

                        if (future == null) {
                            if (!Global.is_init) @panic("not init");
                            future = if (@typeInfo(@TypeOf(ev_deinit)).@"fn".params.len == 1) ev_deinit(&Global.state) else ev_deinit();
                        }
                        if (future) |*fut| switch (fut.poll(waker)) {
                            .ready => {
                                future = null;
                                ctx.Handle.unregisterHandle();
                                Global.instance = undefined;
                                inline for (import_symbols) |sym| sym.getGlobal().unregister();
                                Global.state = undefined;
                                Global.is_init = false;
                                return true;
                            },
                            .pending => return false,
                        };
                    }
                };
            };

            ev_deinit_poll = switch (comptime @typeInfo(@typeInfo(@TypeOf(ev_deinit)).@"fn".return_type.?)) {
                .void => &wrapper.Sync.poll,
                else => &wrapper.Async.poll,
            };
        } else {
            const wrapper = struct {
                fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker, state: *anyopaque) callconv(.c) bool {
                    _ = inst;
                    _ = waker;
                    _ = state;

                    if (!Global.is_init) @panic("not init");
                    ctx.Handle.unregisterHandle();
                    Global.instance = undefined;
                    inline for (import_symbols) |sym| sym.getGlobal().unregister();
                    Global.state = undefined;
                    Global.is_init = false;
                    return true;
                }
            };
            ev_deinit_poll = &wrapper.poll;
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

                    fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker, result: *AnyResult) callconv(.c) bool {
                        _ = inst;
                        _ = waker;

                        if (!Global.is_init) @panic("not init");
                        const res = on_start();
                        if (@typeInfo(Result) == .error_union) res catch |err| {
                            result.* = .initErr(err);
                            return true;
                        };
                        result.* = .ok;
                        return true;
                    }
                };
                const Async = struct {
                    const Inner = @typeInfo(@TypeOf(on_start)).@"fn".return_type.?;
                    const Result = switch (@typeInfo(Inner.Result)) {
                        .error_union => |v| v.error_set!void,
                        .void => void,
                        else => @compileError("fimo: invalid on_start return type, expected `void` or `err!void`, found " ++ @typeName(Inner.Result)),
                    };

                    var future: ?Inner = null;
                    fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker, result: *AnyResult) callconv(.c) bool {
                        _ = inst;
                        if (future == null) {
                            if (!Global.is_init) @panic("not init");
                            future = on_start();
                        }
                        if (future) |*fut| switch (fut.poll(waker)) {
                            .ready => |v| {
                                future = null;
                                if (@typeInfo(Result) == .error_union) v catch |err| {
                                    result.* = .initErr(err);
                                    return true;
                                };
                                result.* = .ok;
                                return true;
                            },
                            .pending => return false,
                        };
                    }
                };
            };

            ev_start_poll = switch (comptime @typeInfo(@typeInfo(@TypeOf(on_start)).@"fn".return_type.?)) {
                .void => &wrapper.Sync.poll,
                else => &wrapper.Async.poll,
            };
        }

        if (@hasField(@TypeOf(T.fimo_events), "on_stop")) {
            const on_stop = T.fimo_events.on_stop;
            const wrapper = struct {
                const Sync = struct {
                    const EvReturn = @typeInfo(@TypeOf(on_stop)).@"fn".return_type.?;
                    comptime {
                        if (EvReturn != void)
                            @compileError("fimo: invalid on_stop return type, expected `void`, found " ++ @typeName(EvReturn));
                    }

                    fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker) callconv(.c) bool {
                        _ = inst;
                        _ = waker;
                        if (!Global.is_init) @panic("not init");
                        on_stop();
                        return true;
                    }
                };
                const Async = struct {
                    const Inner = @typeInfo(@TypeOf(on_stop)).@"fn".return_type.?;
                    comptime {
                        if (Inner.Result != void)
                            @compileError("fimo: invalid on_stop return type, expected `void`, found " ++ @typeName(Inner.Result));
                    }

                    var future: ?Inner = null;
                    fn poll(inst: *modules.OpaqueInstance, waker: tasks.Waker) callconv(.c) bool {
                        _ = inst;
                        if (future == null) {
                            if (!Global.is_init) @panic("not init");
                            future = on_stop();
                        }
                        if (future) |*fut| switch (fut.poll(waker)) {
                            .ready => {
                                future = null;
                                return true;
                            },
                            .pending => return false,
                        };
                    }
                };
            };

            ev_stop_poll = switch (comptime @typeInfo(@typeInfo(@TypeOf(on_stop)).@"fn".return_type.?)) {
                .void => &wrapper.Sync.poll,
                else => &wrapper.Async.poll,
            };
        }
    }

    const wrapper = struct {
        fn on_event(module: *const Export, tag: *events.Tag) callconv(.c) void {
            _ = module;
            switch (tag.*) {
                .init => if (comptime ev_init_poll != null) {
                    const event: *events.Init = @alignCast(@fieldParentPtr("tag", tag));
                    event.poll = ev_init_poll;
                },
                .deinit => if (comptime ev_deinit_poll != null) {
                    const event: *events.Deinit = @alignCast(@fieldParentPtr("tag", tag));
                    event.poll = ev_deinit_poll;
                },
                .start => if (comptime ev_start_poll != null) {
                    const event: *events.Start = @alignCast(@fieldParentPtr("tag", tag));
                    event.poll = ev_start_poll;
                },
                .stop => if (comptime ev_stop_poll != null) {
                    const event: *events.Stop = @alignCast(@fieldParentPtr("tag", tag));
                    event.poll = ev_stop_poll;
                },

                // Not supported for static modules
                .deinit_export => {},
                .dependencies => {},
                else => {},
            }
        }
        fn provider(instance: anytype, comptime symbol: modules.Symbol) *const anyopaque {
            const field_name, const is_import = comptime blk: {
                for (import_symbols, 0..) |sym, i| {
                    if (std.mem.eql(u8, sym.name, symbol.name) and
                        std.mem.eql(u8, sym.namespace, symbol.namespace) and
                        sym.version.sattisfies(symbol.version))
                        break :blk .{ std.meta.fieldNames(@TypeOf(instance.imports().*))[i], true };
                }
                for (export_symbols, 0..) |sym, i| {
                    if (std.mem.eql(u8, sym.name, symbol.name) and
                        std.mem.eql(u8, sym.namespace, symbol.namespace) and
                        sym.version.sattisfies(symbol.version))
                        break :blk .{ std.meta.fieldNames(@TypeOf(instance.exports().*))[i], false };
                }
            };
            return if (is_import) @field(instance.imports(), field_name) else @field(instance.exports(), field_name);
        }
    };

    const Parameters = if (parameter_types.len != 0) blk: {
        var fields: []const std.builtin.Type.StructField = &.{};
        for (parameter_names, parameter_types) |field_name, param_ty| {
            fields = fields ++ [_]std.builtin.Type.StructField{.{
                .name = field_name,
                .type = *modules.Param(param_ty),
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*anyopaque),
            }};
        }
        break :blk @Type(.{ .@"struct" = .{
            .layout = .@"extern",
            .fields = fields,
            .decls = &.{},
            .is_tuple = false,
        } });
    } else anyopaque;

    const Resources = if (resource_names.len != 0) blk: {
        const Wrapper = extern struct {
            path: paths.compat.Path,
            pub fn get(this: @This()) paths.Path {
                return .initC(this.path);
            }
            pub fn format(this: @This(), w: *std.Io.Writer) std.Io.Writer.Error!void {
                try this.get().format(w);
            }
        };
        var fields: []const std.builtin.Type.StructField = &.{};
        for (resource_names) |field_name| {
            fields = fields ++ [_]std.builtin.Type.StructField{.{
                .name = field_name,
                .type = Wrapper,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(Wrapper),
            }};
        }
        break :blk @Type(.{ .@"struct" = .{
            .layout = .@"extern",
            .fields = fields,
            .decls = &.{},
            .is_tuple = false,
        } });
    } else anyopaque;

    const Imports = if (import_symbols.len != 0) blk: {
        var fields: []const std.builtin.Type.StructField = &.{};
        for (import_symbols, 0..) |sym, i| {
            @setEvalBranchQuota(10_000);
            var num_buf: [128]u8 = undefined;
            fields = fields ++ [_]std.builtin.Type.StructField{.{
                .name = std.fmt.bufPrintZ(&num_buf, "{d}", .{i}) catch unreachable,
                .type = *const sym.T,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*const sym.T),
            }};
        }
        break :blk @Type(.{ .@"struct" = .{
            .layout = .@"extern",
            .fields = fields,
            .decls = &.{},
            .is_tuple = false,
        } });
    } else anyopaque;

    const Exports = if (export_symbols.len != 0) blk: {
        var fields: []const std.builtin.Type.StructField = &.{};
        for (export_symbols, 0..) |sym, i| {
            @setEvalBranchQuota(10_000);
            var num_buf: [128]u8 = undefined;
            fields = fields ++ [_]std.builtin.Type.StructField{.{
                .name = std.fmt.bufPrintZ(&num_buf, "{d}", .{i}) catch unreachable,
                .type = *const sym.T,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(*const sym.T),
            }};
        }
        break :blk @Type(.{ .@"struct" = .{
            .layout = .@"extern",
            .fields = fields,
            .decls = &.{},
            .is_tuple = false,
        } });
    } else anyopaque;

    const exp = &Export{
        .name = .fromSlice(name),
        .description = .fromSlice(description),
        .author = .fromSlice(author),
        .license = .fromSlice(license),
        .parameters = .fromSlice(parameter_infos),
        .resources = .fromSlice(resource_infos),
        .namespaces = .fromSlice(namespace_infos),
        .imports = .fromSlice(import_infos),
        .exports = .fromSlice(export_infos),
        .on_event = &wrapper.on_event,
    };
    embedStaticModuleExport(exp);

    const InstanceT = modules.Instance(.{
        .ParametersType = Parameters,
        .ResourcesType = Resources,
        .ImportsType = Imports,
        .ExportsType = Exports,
        .StateType = T,
        .module_export = exp,
        .provider = wrapper.provider,
    });
    return struct {
        pub const is_init: *bool = &Global.is_init;
        pub const module_export = Instance.module_export;
        pub const Instance = InstanceT;
        pub const fimo_modules_marker = {};

        pub fn instance() *Instance {
            std.debug.assert(is_init.*);
            return @ptrCast(Global.instance);
        }

        /// Returns the parameter table of the module.
        pub fn parameters() *const Instance.Parameters {
            return instance().parameters();
        }

        /// Returns the paths table of the module.
        pub fn paths() *const Instance.Resources {
            return instance().resources();
        }

        /// Returns the import table of the module.
        pub fn imports() *const Instance.Imports {
            return instance().imports();
        }

        /// Returns the exports table of the module.
        ///
        /// Exports are ordered the in declaration order of the module export.
        pub fn exports() *const Instance.Exports {
            return instance().exports();
        }

        /// Returns the shared handle of the module.
        ///
        /// NOTE: The reference count is not modified.
        pub fn handle() *modules.Handle {
            return instance().handle();
        }

        /// Returns the handle to the context.
        pub fn ctxHandle() *ctx.Handle {
            return instance().ctxHandle();
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
        ///
        /// NOTE: Use with caution. Prefer structuring your code in a way that does not necessitate
        /// dependency tracking.
        pub fn ref() void {
            instance().ref();
        }

        /// Decreases the strong reference count of the module instance.
        ///
        /// May only be called after the reference count has been increased.
        pub fn unref() void {
            instance().unref();
        }

        /// Checks the status of a namespace from the view of the module.
        ///
        /// Checks if the module includes the namespace. In that case, the module is allowed access
        /// to the symbols in the namespace. Additionally, this function also queries whether the
        /// include is static, i.e., it was specified by the module at load time.
        pub fn queryNamespace(ns: []const u8) ctx.Error!modules.Dependency {
            return instance().queryNamespace(ns);
        }

        /// Adds a namespace dependency to the module.
        ///
        /// Once added, the module gains access to the symbols of its dependencies that are
        /// exposed in said namespace. A namespace can not be added multiple times.
        pub fn addNamespace(ns: []const u8) ctx.Error!void {
            try instance().addNamespace(ns);
        }

        /// Removes a namespace dependency from the module.
        ///
        /// Once excluded, the caller guarantees to relinquish access to the symbols contained in
        /// said namespace. It is only possible to exclude namespaces that were manually added,
        /// whereas static namespace dependencies remain valid until the module is unloaded.
        pub fn removeNamespace(ns: []const u8) ctx.Error!void {
            try instance().removeNamespace(ns);
        }

        /// Checks if a module depends on another module.
        ///
        /// Checks if the specified module is a dependency of the current instance. In that case
        /// the instance is allowed to access the symbols exported by the module. Additionally,
        /// this function also queries whether the dependency is static, i.e., the dependency was
        /// specified by the module at load time.
        pub fn queryDependency(h: *modules.Handle) ctx.Error!modules.Dependency {
            return instance().queryDependency(h);
        }

        /// Adds another module as a dependency.
        ///
        /// After adding a module as a dependency, the module is allowed access to the symbols
        /// and protected parameters of said dependency. Trying to adding a dependency to a module
        /// that is already a dependency, or to a module that would result in a circular dependency
        /// will result in an error.
        pub fn addDependency(h: *modules.Handle) ctx.Error!void {
            try instance().addDependency(h);
        }

        /// Removes a module as a dependency.
        ///
        /// By removing a module as a dependency, the caller ensures that it does not own any
        /// references to resources originating from the former dependency, and allows for the
        /// unloading of the module. A module can only relinquish dependencies to modules that were
        /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
        pub fn removeDependency(h: *modules.Handle) ctx.Error!void {
            try instance().removeDependency(h);
        }

        /// Loads a group of symbols from the module subsystem.
        ///
        /// Is equivalent to calling `loadSymbol` for each symbol of the group independently.
        pub fn loadSymbolGroup(comptime symbols: anytype) ctx.Error!modules.SymbolGroup(symbols) {
            return instance().loadSymbolGroup(symbols);
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
        /// loading optional symbols, or for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until the module relinquishes the
        /// dependency to the module that exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbol(comptime symbol: modules.Symbol) ctx.Error!modules.SymbolWrapper(symbol) {
            return instance().loadSymbol(symbol);
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
        /// loading optional symbols, or for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until the module relinquishes the
        /// dependency to the module that exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbolRaw(symbol: modules.SymbolId) ctx.Error!*const anyopaque {
            return instance().loadSymbolRaw(symbol);
        }

        /// Reads a module parameter with dependency read access.
        ///
        /// Reads the value of a module parameter with dependency read access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow reading with a
        /// dependency access.
        pub fn readParameter(comptime ParamT: type, module: []const u8, parameter: []const u8) ctx.Error!ParamT {
            return try instance().readParameter(ParamT, module, parameter);
        }

        /// Sets a module parameter with dependency write access.
        ///
        /// Sets the value of a module parameter with dependency write access. The operation fails,
        /// if the parameter does not exist, or if the parameter does not allow writing with a
        /// dependency access.
        pub fn writeParameter(comptime ParamT: type, value: ParamT, module: []const u8, parameter: []const u8) ctx.Error!void {
            try instance().writeParameter(ParamT, value, module, parameter);
        }
    };
}

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

    pub export fn fstd__module_export_iter(
        context: ?*anyopaque,
        inspector: *const fn (context: ?*anyopaque, module: *const Export) callconv(.c) bool,
    ) void {
        var it = ExportIter{};
        while (it.next()) |exp| {
            if (!inspector(context, exp)) {
                return;
            }
        }
    }
};

/// Creates a new unique export in the correct section.
///
/// For internal use only, as the pointer should not generally be null.
fn embedStaticModuleExport(comptime module: ?*const Export) void {
    const name = if (module) |m| m.name.intoSliceOrEmpty() else "unknown";
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
