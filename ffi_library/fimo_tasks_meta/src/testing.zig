const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const Context = fimo_std.Context;
const Async = Context.Async;
const Tracing = Context.Tracing;
const Module = Context.Module;
const Symbol = Module.Symbol;
const SymbolWrapper = Module.SymbolWrapper;
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
    task_id: SymbolWrapper(symbols.task_id),
    worker_id: SymbolWrapper(symbols.worker_id),
    worker_pool: SymbolWrapper(symbols.worker_pool),
    worker_pool_by_id: SymbolWrapper(symbols.worker_pool_by_id),
    query_worker_pools: SymbolWrapper(symbols.query_worker_pools),
    create_worker_pool: SymbolWrapper(symbols.create_worker_pool),
    yield: SymbolWrapper(symbols.yield),
    abort: SymbolWrapper(symbols.abort),
    sleep: SymbolWrapper(symbols.sleep),
    task_local_set: SymbolWrapper(symbols.task_local_set),
    task_local_get: SymbolWrapper(symbols.task_local_get),
    task_local_clear: SymbolWrapper(symbols.task_local_clear),
    parking_lot_park: SymbolWrapper(symbols.parking_lot_park),
    parking_lot_park_multiple: SymbolWrapper(symbols.parking_lot_park_multiple),
    parking_lot_unpark_one: SymbolWrapper(symbols.parking_lot_unpark_one),
    parking_lot_unpark_all: SymbolWrapper(symbols.parking_lot_unpark_all),
    parking_lot_unpark_filter: SymbolWrapper(symbols.parking_lot_unpark_filter),
    parking_lot_unpark_requeue: SymbolWrapper(symbols.parking_lot_unpark_requeue),

    pub fn deinit(self: *TestContext) void {
        self.instance.deinit();

        var err: ?fimo_std.AnyError = null;
        self.ctx.module().pruneInstances(&err) catch unreachable;
        self.event_loop.join();
        Async.EventLoop.flushWithCurrentThread(self.ctx.@"async"(), &err) catch unreachable;
        self.ctx.tracing().unregisterThread();
        self.ctx.unref();
    }

    pub fn provideSymbol(self: *const TestContext, comptime symbol: Symbol) *const symbol.T {
        if (comptime !std.mem.eql(u8, symbol.namespace, symbols.symbol_namespace))
            @compileError("unknown namespace " ++ symbol.namespace);
        return @field(self, symbol.name).value;
    }
};

pub fn initTestContext() !TestContext {
    const tracing_cfg = Tracing.Config{
        .max_level = .debug,
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
    errdefer if (err) |e| e.deinit();

    errdefer Async.EventLoop.flushWithCurrentThread(ctx.@"async"(), &err) catch unreachable;
    const event_loop = try Async.EventLoop.init(ctx.@"async"(), &err);
    errdefer event_loop.join();

    const async_ctx = try Async.BlockingContext.init(ctx.@"async"(), &err);
    defer async_ctx.deinit();

    const module_path = try fimo_std.path.Path.init("./fimo_tasks/module.fimo_module");

    const set = try Module.LoadingSet.init(ctx.module(), &err);
    defer set.unref();

    try set.addModulesFromPath(
        module_path,
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
        .task_id = try instance.loadSymbol(symbols.task_id, &err),
        .worker_id = try instance.loadSymbol(symbols.worker_id, &err),
        .worker_pool = try instance.loadSymbol(symbols.worker_pool, &err),
        .worker_pool_by_id = try instance.loadSymbol(symbols.worker_pool_by_id, &err),
        .query_worker_pools = try instance.loadSymbol(symbols.query_worker_pools, &err),
        .create_worker_pool = try instance.loadSymbol(symbols.create_worker_pool, &err),
        .yield = try instance.loadSymbol(symbols.yield, &err),
        .abort = try instance.loadSymbol(symbols.abort, &err),
        .sleep = try instance.loadSymbol(symbols.sleep, &err),
        .task_local_set = try instance.loadSymbol(symbols.task_local_set, &err),
        .task_local_get = try instance.loadSymbol(symbols.task_local_get, &err),
        .task_local_clear = try instance.loadSymbol(symbols.task_local_clear, &err),
        .parking_lot_park = try instance.loadSymbol(symbols.parking_lot_park, &err),
        .parking_lot_park_multiple = try instance.loadSymbol(symbols.parking_lot_park_multiple, &err),
        .parking_lot_unpark_one = try instance.loadSymbol(symbols.parking_lot_unpark_one, &err),
        .parking_lot_unpark_all = try instance.loadSymbol(symbols.parking_lot_unpark_all, &err),
        .parking_lot_unpark_filter = try instance.loadSymbol(symbols.parking_lot_unpark_filter, &err),
        .parking_lot_unpark_requeue = try instance.loadSymbol(symbols.parking_lot_unpark_requeue, &err),
    };

    return test_ctx;
}

pub fn initTestContextInTask(func: fn (*const TestContext, *?AnyError) anyerror!void) !void {
    var ctx = try initTestContext();
    defer ctx.deinit();

    var e: ?anyerror = null;
    var err: ?AnyError = null;
    defer if (err) |e_| e_.deinit();

    const p = try Pool.init(ctx, &.{ .worker_count = 4, .label_ = "test", .label_len = 4 }, &err);
    defer {
        p.requestClose();
        p.unref();
    }

    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    const TaskState = extern struct {
        ctx: *const TestContext,
        err: *?AnyError,
        e: *?anyerror,
    };
    const builder_config = TaskBuilderConfig(TaskState){
        .on_start = struct {
            fn f(t: *Task(TaskState)) void {
                func(t.state.ctx, t.state.err) catch |e_| {
                    t.state.e.* = e_;
                };
            }
        }.f,
    };
    const builder = TaskBuilder(builder_config){
        .label = "testTask",
        .state = .{
            .ctx = &ctx,
            .err = &err,
            .e = &e,
        },
    };
    var task_ = builder.build();

    const buffer_builder_config = CommandBufferBuilderConfig(void){};
    var buffer_builder = CommandBufferBuilder(buffer_builder_config){
        .label = "testBuffer",
        .state = {},
    };
    try buffer_builder.enqueueTask(allocator, @ptrCast(&task_));
    var buffer = buffer_builder.build();

    const handle = try p.enqueueCommandBuffer(&buffer, &err);
    defer handle.unref();
    const status = handle.waitOn();
    if (e) |e_| return e_;
    if (status == .aborted) return error.Aborted;
}
