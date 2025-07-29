const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const ctx = fimo_std.ctx;
const modules = fimo_std.modules;
const tracing = fimo_std.tracing;
const tasks = fimo_std.tasks;
const Symbol = modules.Symbol;
const SymbolWrapper = modules.SymbolWrapper;
const SymbolGroup = modules.SymbolGroup;
const PseudoInstance = modules.PseudoInstance;
const fimo_tasks_meta = @import("fimo_tasks_meta");

const symbols = @import("symbols.zig");

comptime {
    @import("test_module").forceExportModules();
}

pub const GlobalCtx = struct {
    var t_ctx: ?TestContext = null;

    pub fn init(self: @This()) !void {
        _ = self;
        if (t_ctx != null) @panic("context already initialized");
        t_ctx = try .init();
    }

    pub fn deinit(self: @This()) void {
        _ = self;
        if (t_ctx) |*c| {
            c.deinit();
            t_ctx = null;
        } else @panic("not initialized");
    }

    pub fn provideSymbol(self: @This(), comptime symbol: Symbol) *const symbol.T {
        _ = self;
        if (t_ctx) |*c| return symbol.requestFrom(c);
        @panic("not initialized");
    }
}{};

const TestContext = struct {
    instance: *const PseudoInstance,
    symbols: SymbolGroup(symbols.all_symbols ++ fimo_tasks_meta.symbols.all_symbols),

    fn init() !@This() {
        const tracing_cfg = tracing.Config{
            .max_level = .warn,
            .subscribers = &.{tracing.default_subscriber},
            .subscriber_count = 1,
        };
        defer tracing_cfg.deinit();
        const init_options: [:null]const ?*const ctx.ConfigHead = &.{@ptrCast(&tracing_cfg)};
        try ctx.init(init_options);
        errdefer ctx.deinit();

        tracing.registerThread();
        errdefer tracing.unregisterThread();

        var err: ?fimo_std.AnyError = null;
        errdefer if (err) |e| {
            tracing.emitErrSimple("{f}", .{e}, @src());
            e.deinit();
        };

        const async_ctx = try tasks.BlockingContext.init(&err);
        defer async_ctx.deinit();

        const set = try modules.LoadingSet.init(&err);
        defer set.unref();

        try set.addModulesFromLocal(
            &{},
            struct {
                fn f(@"export": *const modules.Export, data: *const void) modules.LoadingSet.FilterRequest {
                    _ = @"export";
                    _ = data;
                    return .load;
                }
            }.f,
            null,
            &err,
        );
        try set.commit().intoFuture().awaitBlocking(async_ctx).unwrap(&err);

        const instance = try modules.PseudoInstance.init(&err);
        errdefer instance.deinit();

        const tasks_info = try modules.Info.findByName("fimo_tasks", &err);
        defer tasks_info.unref();
        const worlds_info = try modules.Info.findByName("fimo_worlds", &err);
        defer worlds_info.unref();

        try instance.addDependency(tasks_info, &err);
        try instance.addDependency(worlds_info, &err);
        try instance.addNamespace(symbols.symbol_namespace, &err);
        try instance.addNamespace(fimo_tasks_meta.symbols.symbol_namespace, &err);

        const test_ctx = @This(){
            .instance = instance,
            .symbols = try instance.loadSymbolGroup(
                symbols.all_symbols ++ fimo_tasks_meta.symbols.all_symbols,
                &err,
            ),
        };

        return test_ctx;
    }

    pub fn deinit(self: *@This()) void {
        self.instance.deinit();

        var err: ?fimo_std.AnyError = null;
        modules.pruneInstances(&err) catch unreachable;
        tracing.unregisterThread();
        ctx.deinit();
    }

    pub fn provideSymbol(self: *const @This(), comptime symbol: Symbol) *const symbol.T {
        return symbol.requestFrom(&self.symbols);
    }
};
