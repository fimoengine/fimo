const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const Context = fimo_std.Context;
const Module = Context.Module;
const Symbol = Module.Symbol;
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
    instance: *const PseudoInstance,
    task_id: *const symbols.task_id.T,
    worker_id: *const symbols.worker_id.T,
    worker_pool: *const symbols.worker_pool.T,
    worker_pool_by_id: *const symbols.worker_pool_by_id.T,
    query_worker_pools: *const symbols.query_worker_pools.T,
    create_worker_pool: *const symbols.create_worker_pool.T,
    yield: *const symbols.yield.T,
    abort: *const symbols.abort.T,
    sleep: *const symbols.sleep.T,
    task_local_set: *const symbols.task_local_set.T,
    task_local_get: *const symbols.task_local_get.T,
    task_local_clear: *const symbols.task_local_clear.T,
    parking_lot_park: *const symbols.parking_lot_park.T,
    parking_lot_unpark_one: *const symbols.parking_lot_unpark_one.T,
    parking_lot_unpark_all: *const symbols.parking_lot_unpark_all.T,
    parking_lot_unpark_filter: *const symbols.parking_lot_unpark_filter.T,
    parking_lot_unpark_requeue: *const symbols.parking_lot_unpark_requeue.T,

    pub fn deinit(self: *TestContext) void {
        self.instance.deinit();
    }

    pub fn provideSymbol(self: *const TestContext, comptime symbol: Symbol) *const symbol.T {
        if (comptime !std.mem.eql(u8, symbol.namespace, symbols.symbol_namespace))
            @compileError("unknown namespace " ++ symbol.namespace);
        return @field(self, symbol.name);
    }
};

pub fn initTestContext() !TestContext {
    return error.SkipZigTest;
}

pub fn initTestContextInTask(func: fn (*const TestContext, *?AnyError) anyerror!void) !void {
    var ctx = try initTestContext();
    defer ctx.deinit();

    var e: ?anyerror = null;
    var err: ?AnyError = null;
    defer if (err) |e_| e_.deinit();

    const p = try Pool.init(ctx, &.{}, &err);
    defer p.unref();

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
        .state = .{
            .ctx = &ctx,
            .err = &err,
            .e = &e,
        },
    };
    var task_ = builder.build();

    const buffer_builder_config = CommandBufferBuilderConfig(void){};
    var buffer_builder = CommandBufferBuilder(buffer_builder_config){ .state = void{} };
    try buffer_builder.enqueueTask(allocator, @ptrCast(&task_));
    var buffer = buffer_builder.build();

    const handle = try p.enqueueCommandBuffer(&buffer, &err);
    defer handle.unref();
    const status = handle.waitOn();
    if (e) |e_| return e_;
    if (status == .aborted) return error.Aborted;
}
