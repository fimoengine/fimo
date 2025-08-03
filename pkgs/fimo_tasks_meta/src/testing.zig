const std = @import("std");

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const tasks = fimo_std.tasks;
const tracing = fimo_std.tracing;
const modules = fimo_std.modules;
const Symbol = modules.Symbol;
const SymbolWrapper = modules.SymbolWrapper;
const SymbolGroup = modules.SymbolGroup;
const RootInstance = modules.RootInstance;
const TestModule = @import("test_module");

const command_buffer = @import("command_buffer.zig");
const CommandBufferBuilderConfig = command_buffer.BuilderConfig;
const CommandBufferBuilder = command_buffer.Builder;
const pool = @import("pool.zig");
const Pool = pool.Pool;
const symbols = @import("symbols.zig");
const task = @import("task.zig");
const Task = task.Task;
const TaskBuilderConfig = task.BuilderConfig;
const TaskBuilder = task.Builder;

pub const TestContext = struct {
    instance: *const RootInstance,
    symbols: SymbolGroup(symbols.all_symbols),

    pub fn deinit(self: *TestContext) void {
        self.symbols.unregisterGlobal();
        self.instance.deinit();

        modules.pruneInstances() catch unreachable;
        tracing.unregisterThread();
        ctx.deinit();
    }

    pub fn provideSymbol(self: *const TestContext, comptime symbol: Symbol) *const symbol.T {
        return symbol.requestFrom(self.symbols);
    }
};

pub fn initTestContext() !TestContext {
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
    errdefer if (ctx.hasErrorResult()) {
        const e = ctx.takeResult().unwrapErr();
        defer e.deinit();
        tracing.emitErrSimple("{f}", .{e}, @src());
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

    const info = try modules.Info.findByName("fimo_tasks");
    defer info.unref();

    try instance.addDependency(info);
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

    const p = try Pool.init(&.{ .worker_count = 4, .label_ = "test", .label_len = 4 });
    defer {
        p.requestClose();
        p.unref();
    }

    const future = try @import("future.zig").init(
        p,
        func,
        .{},
        .{ .allocator = std.testing.allocator, .label = "test" },
    );
    defer future.deinit();
    try future.await();
}
