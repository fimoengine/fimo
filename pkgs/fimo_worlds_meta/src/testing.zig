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
    var gpa = std.heap.DebugAllocator(.{}).init;
    var logger: tracing.StdErrLogger = undefined;
    var t_ctx: ?TestContext = null;

    pub fn init() !void {
        if (t_ctx != null) @panic("context already initialized");
        errdefer if (gpa.deinit() == .leak) @panic("leak");
        try logger.init(.{ .gpa = gpa.allocator() });
        t_ctx = try .init();
    }

    pub fn deinit() void {
        if (t_ctx) |*c| {
            c.deinit();
            logger.deinit();
            if (gpa.deinit() == .leak) @panic("leak");
            gpa = .init;
            t_ctx = null;
        } else @panic("not initialized");
    }

    pub fn provideSymbol(comptime symbol: Symbol) *const symbol.T {
        if (t_ctx) |*c| return symbol.requestFrom(c);
        @panic("not initialized");
    }
};

const TestContext = struct {
    instance: *const RootInstance,
    symbols: SymbolGroup(symbols.all_symbols ++ fimo_tasks_meta.symbols.all_symbols),

    fn init() !@This() {
        const tracing_cfg = tracing.Config{
            .max_level = .warn,
            .subscribers = &.{GlobalCtx.logger.subscriber()},
            .subscriber_count = 1,
        };
        const init_options: [:null]const ?*const ctx.ConfigHead = &.{@ptrCast(&tracing_cfg)};
        try ctx.init(init_options);
        errdefer ctx.deinit();
        errdefer if (ctx.hasErrorResult()) {
            const e = ctx.takeResult().unwrapErr();
            defer e.deinit();
            tracing.logErr(@src(), "{f}", .{e});
            e.deinit();
        };

        const async_ctx = try tasks.BlockingContext.init();
        defer async_ctx.deinit();

        const set = try modules.LoadingSet.init();
        defer set.unref();

        try set.addModulesFromLocal({}, TestModule.fimo_module_bundle.loadingSetFilter, null);
        try set.commit().intoFuture().awaitBlocking(async_ctx).unwrap();

        const instance = try modules.RootInstance.init();
        errdefer instance.deinit();

        const tasks_info = try modules.Info.findByName("fimo_tasks");
        defer tasks_info.unref();
        const worlds_info = try modules.Info.findByName("fimo_worlds");
        defer worlds_info.unref();

        try instance.addDependency(tasks_info);
        try instance.addDependency(worlds_info);
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
