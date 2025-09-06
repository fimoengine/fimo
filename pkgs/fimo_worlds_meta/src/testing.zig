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
const RootInstance = modules.RootInstance;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const TestModule = @import("test_module");

const symbols = @import("symbols.zig");

pub const GlobalCtx = struct {
    var t_ctx: ?TestContext = null;

    pub fn init() !void {
        if (t_ctx != null) @panic("context already initialized");
        t_ctx = try .init();
    }

    pub fn deinit() void {
        if (t_ctx) |*c| {
            c.deinit();
            t_ctx = null;
        } else @panic("not initialized");
    }

    pub fn provideSymbol(comptime symbol: Symbol) *const symbol.T {
        if (t_ctx) |*c| return symbol.requestFrom(c);
        @panic("not initialized");
    }
};

const TestContext = struct {
    instance: *RootInstance,
    symbols: SymbolGroup(symbols.all_symbols ++ fimo_tasks_meta.symbols.all_symbols),

    fn init() !@This() {
        const tracing_cfg = @import("root").tracing_cfg;
        try ctx.init(&.{&tracing_cfg.cfg});
        errdefer ctx.deinit();
        errdefer if (ctx.hasErrorResult()) {
            const e = ctx.takeResult().unwrapErr();
            defer e.deinit();
            tracing.logErr(@src(), "{f}", .{e});
            e.deinit();
        };

        const waiter = try tasks.Waiter.init();
        defer waiter.deinit();

        const loader = try modules.Loader.init();
        defer loader.deinit();

        try loader.addModulesFromIter({}, TestModule.fimo_module_bundle.loaderFilter);
        try loader.commit().intoFuture().awaitBlocking(waiter).unwrap();

        const instance = try modules.RootInstance.init();
        errdefer instance.deinit();

        const tasks_handle = try modules.Handle.findByName("fimo_tasks");
        defer tasks_handle.unref();
        const worlds_handle = try modules.Handle.findByName("fimo_worlds");
        defer worlds_handle.unref();

        try instance.addDependency(tasks_handle);
        try instance.addDependency(worlds_handle);
        try instance.addNamespace(symbols.symbol_namespace);
        try instance.addNamespace(fimo_tasks_meta.symbols.symbol_namespace);

        const test_ctx = @This(){
            .instance = instance,
            .symbols = try instance.loadSymbolGroup(symbols.all_symbols ++ fimo_tasks_meta.symbols.all_symbols),
        };
        test_ctx.symbols.registerGlobal();

        return test_ctx;
    }

    pub fn deinit(self: *@This()) void {
        self.symbols.unregisterGlobal();
        self.instance.deinit();

        modules.pruneInstances() catch unreachable;
        ctx.deinit();
    }

    pub fn provideSymbol(self: *const @This(), comptime symbol: Symbol) *const symbol.T {
        return symbol.requestFrom(&self.symbols);
    }
};
