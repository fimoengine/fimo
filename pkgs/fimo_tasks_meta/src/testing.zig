const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const ctx = fimo_std.ctx;
const tasks = fimo_std.tasks;
const tracing = fimo_std.tracing;
const modules = fimo_std.modules;
const Symbol = modules.Symbol;
const SymbolWrapper = modules.SymbolWrapper;
const SymbolGroup = modules.SymbolGroup;
const PseudoInstance = modules.PseudoInstance;

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
    event_loop: tasks.EventLoop,
    instance: *const PseudoInstance,
    symbols: SymbolGroup(symbols.all_symbols),

    pub fn deinit(self: *TestContext) void {
        self.instance.deinit();

        var err: ?fimo_std.AnyError = null;
        modules.pruneInstances(&err) catch unreachable;
        self.event_loop.join();
        tasks.EventLoop.flushWithCurrentThread(&err) catch unreachable;
        tracing.unregisterThread();
        ctx.deinit();
    }

    pub fn provideSymbol(self: *const TestContext, comptime symbol: Symbol) *const symbol.T {
        return symbol.requestFrom(self.symbols);
    }
};

comptime {
    @import("test_module").forceExportModules();
}

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

    var err: ?fimo_std.AnyError = null;
    errdefer if (err) |e| {
        tracing.emitErrSimple("{f}", .{e}, @src());
        e.deinit();
    };

    errdefer tasks.EventLoop.flushWithCurrentThread(&err) catch unreachable;
    const event_loop = try tasks.EventLoop.init(&err);
    errdefer event_loop.join();

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

    const info = try modules.Info.findByName("fimo_tasks", &err);
    defer info.unref();

    try instance.addDependency(info, &err);
    try instance.addNamespace(symbols.symbol_namespace, &err);

    const test_ctx = TestContext{
        .event_loop = event_loop,
        .instance = instance,
        .symbols = try instance.loadSymbolGroup(symbols.all_symbols, &err),
    };

    return test_ctx;
}

pub fn initTestContextInTask(func: fn (*const TestContext, *?AnyError) anyerror!void) !void {
    var t_ctx = try initTestContext();
    defer t_ctx.deinit();

    var err: ?AnyError = null;
    defer if (err) |e| e.deinit();

    const p = try Pool.init(t_ctx, &.{ .worker_count = 4, .label_ = "test", .label_len = 4 }, &err);
    defer {
        p.requestClose();
        p.unref();
    }

    const future = try @import("future.zig").init(
        p,
        func,
        .{ &t_ctx, &err },
        .{ .allocator = std.testing.allocator, .label = "test" },
        &err,
    );
    defer future.deinit();
    try future.await();
}
