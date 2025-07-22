const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const Context = fimo_std.Context;
const Async = Context.Async;
const Tracing = Context.Tracing;
const Module = Context.Module;
const Symbol = Module.Symbol;
const SymbolWrapper = Module.SymbolWrapper;
const SymbolGroup = Module.SymbolGroup;
const PseudoInstance = Module.PseudoInstance;

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
    ctx: Context,
    event_loop: Async.EventLoop,
    instance: *const PseudoInstance,
    symbols: SymbolGroup(symbols.all_symbols),

    pub fn deinit(self: *TestContext) void {
        self.instance.deinit();

        var err: ?fimo_std.AnyError = null;
        self.ctx.module().pruneInstances(&err) catch unreachable;
        self.event_loop.join();
        Async.EventLoop.flushWithCurrentThread(self.ctx.async(), &err) catch unreachable;
        self.ctx.tracing().unregisterThread();
        self.ctx.unref();
    }

    pub fn provideSymbol(self: *const TestContext, comptime symbol: Symbol) *const symbol.T {
        return symbol.requestFrom(self.symbols);
    }
};

comptime {
    @import("test_module").forceExportModules();
}

pub fn initTestContext() !TestContext {
    const tracing_cfg = Tracing.Config{
        .max_level = .warn,
        .subscribers = &.{Tracing.default_subscriber},
        .subscriber_count = 1,
    };
    defer tracing_cfg.deinit();
    const init_options: [:null]const ?*const Context.TaggedInStruct = &.{@ptrCast(&tracing_cfg)};

    const ctx = try Context.init(init_options);
    errdefer ctx.unref();

    ctx.tracing().registerThread();
    errdefer ctx.tracing().unregisterThread();

    var err: ?fimo_std.AnyError = null;
    errdefer if (err) |e| {
        ctx.tracing().emitErrSimple("{f}", .{e}, @src());
        e.deinit();
    };

    errdefer Async.EventLoop.flushWithCurrentThread(ctx.async(), &err) catch unreachable;
    const event_loop = try Async.EventLoop.init(ctx.async(), &err);
    errdefer event_loop.join();

    const async_ctx = try Async.BlockingContext.init(ctx.async(), &err);
    defer async_ctx.deinit();

    const set = try Module.LoadingSet.init(ctx.module(), &err);
    defer set.unref();

    try set.addModulesFromLocal(
        &{},
        struct {
            fn f(@"export": *const Module.Export, data: *const void) Module.LoadingSet.FilterRequest {
                _ = @"export";
                _ = data;
                return .load;
            }
        }.f,
        null,
        &err,
    );
    try set.commit().intoFuture().awaitBlocking(async_ctx).unwrap(&err);

    const instance = try Module.PseudoInstance.init(ctx.module(), &err);
    errdefer instance.deinit();

    const info = try Module.Info.findByName(ctx.module(), "fimo_tasks", &err);
    defer info.unref();

    try instance.addDependency(info, &err);
    try instance.addNamespace(symbols.symbol_namespace, &err);

    const test_ctx = TestContext{
        .ctx = ctx,
        .event_loop = event_loop,
        .instance = instance,
        .symbols = try instance.loadSymbolGroup(symbols.all_symbols, &err),
    };

    return test_ctx;
}

pub fn initTestContextInTask(func: fn (*const TestContext, *?AnyError) anyerror!void) !void {
    var ctx = try initTestContext();
    defer ctx.deinit();

    var err: ?AnyError = null;
    defer if (err) |e| e.deinit();

    const p = try Pool.init(ctx, &.{ .worker_count = 4, .label_ = "test", .label_len = 4 }, &err);
    defer {
        p.requestClose();
        p.unref();
    }

    const future = try @import("future.zig").init(
        p,
        func,
        .{ &ctx, &err },
        .{ .allocator = std.testing.allocator, .label = "test" },
        &err,
    );
    defer future.deinit();
    try future.await();
}
