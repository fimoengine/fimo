const std = @import("std");

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const tracing = fimo_std.tracing;
const modules = fimo_std.modules;
const Symbol = modules.Symbol;
const SymbolWrapper = modules.SymbolWrapper;
const SymbolGroup = modules.SymbolGroup;
const RootInstance = modules.RootInstance;
const TestModule = @import("test_module");

const root = @import("root.zig");
const Executor = root.Executor;
const Future = root.Future;
const symbols = @import("symbols.zig");

pub const TestContext = struct {
    instance: *RootInstance,
    symbols: SymbolGroup(symbols.all_symbols),

    pub fn deinit(self: *TestContext) void {
        self.symbols.unregisterGlobal();
        self.instance.deinit();

        modules.pruneInstances() catch unreachable;
        ctx.deinit();
    }

    pub fn provideSymbol(self: *const TestContext, comptime symbol: Symbol) *const symbol.T {
        return symbol.requestFrom(self.symbols);
    }
};

pub fn initTestContext() !TestContext {
    const tracing_cfg = @import("root").tracing_cfg;
    try ctx.init(&.{&tracing_cfg.cfg});
    errdefer ctx.deinit();
    errdefer if (ctx.hasErrorResult()) {
        const e = ctx.takeResult().unwrapErr();
        defer e.deinit();
        tracing.logErr(@src(), "{f}", .{e});
        e.deinit();
    };

    const waiter = try fimo_std.tasks.Waiter.init();
    defer waiter.deinit();

    const loader = try modules.Loader.init();
    defer loader.deinit();

    try loader.addModulesFromIter({}, TestModule.fimo_module_bundle.loaderFilter);
    try loader.commit().intoFuture().awaitBlocking(waiter).unwrap();

    const instance = try modules.RootInstance.init();
    errdefer instance.deinit();

    const handle = try modules.Handle.findByName("fimo_tasks");
    defer handle.unref();

    try instance.addDependency(handle);
    try instance.addNamespace(symbols.symbol_namespace);

    const test_ctx = TestContext{
        .instance = instance,
        .symbols = try instance.loadSymbolGroup(symbols.all_symbols),
    };
    test_ctx.symbols.registerGlobal();

    return test_ctx;
}

pub fn initTestContextInTask(func: fn () anyerror!void) !void {
    var t_ctx = try initTestContext();
    defer t_ctx.deinit();

    const exe = Executor.globalExecutor();
    var fut = Future(func){};
    fut.spawn(exe, .{});
    try fut.join();
}
