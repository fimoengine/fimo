const std = @import("std");
const atomic = std.atomic;
const builtin = @import("builtin");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;
const Context = fimo_std.Context;
const Module = Context.Module;
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;
pub const fimo_tasks_meta = @import("fimo_tasks_meta");
const symbols = fimo_tasks_meta.symbols;

const context = @import("context.zig");
const mpsc_channel = @import("mpsc_channel.zig");
const ParkingLot = @import("ParkingLot.zig");
const Pool = @import("Pool.zig");
const spmc_channel = @import("spmc_channel.zig");
const Task = @import("Task.zig");
const Worker = @import("Worker.zig");

test {
    std.testing.refAllDeclsRecursive(context);
    std.testing.refAllDeclsRecursive(mpsc_channel);
    std.testing.refAllDeclsRecursive(ParkingLot);
    std.testing.refAllDeclsRecursive(Pool);
    std.testing.refAllDeclsRecursive(spmc_channel);
    std.testing.refAllDeclsRecursive(Task);
    std.testing.refAllDeclsRecursive(Worker);
}

pub const Instance = blk: {
    @setEvalBranchQuota(100000);
    break :blk Module.exports.Builder.init("fimo_tasks")
        .withDescription("Multi-threaded tasks runtime")
        .withLicense("MIT OR APACHE 2.0")
        .withExport(symbols.task_id, "task_id", .global, &taskId)
        .withExport(symbols.worker_id, "worker_id", .global, &workerId)
        .withExport(symbols.worker_pool, "worker_pool", .global, &workerPool)
        .withExport(symbols.worker_pool_by_id, "worker_pool_by_id", .global, &workerPoolById)
        .withExport(symbols.query_worker_pools, "query_worker_pools", .global, &queryWorkerPool)
        .withExport(symbols.create_worker_pool, "create_worker_pool", .global, &createWorkerPool)
        .withExport(symbols.yield, "yield", .global, &yield)
        .withExport(symbols.abort, "abort", .global, &abort)
        .withExport(symbols.sleep, "sleep", .global, &sleep)
        .withExport(symbols.task_local_set, "task_local_set", .global, &taskLocalSet)
        .withExport(symbols.task_local_get, "task_local_get", .global, &taskLocalGet)
        .withExport(symbols.task_local_clear, "task_local_clear", .global, &taskLocalClear)
        .withExport(symbols.parking_lot_park, "pl_park", .global, &parkingLotPark)
        .withExport(symbols.parking_lot_park_multiple, "pl_park_multiple", .global, &parkingLotParkMultiple)
        .withExport(symbols.parking_lot_unpark_one, "pl_u_one", .global, &parkingLotUnparkOne)
        .withExport(symbols.parking_lot_unpark_all, "pl_u_all", .global, &parkingLotUnparkAll)
        .withExport(symbols.parking_lot_unpark_filter, "pl_u_filter", .global, &parkingLotUnparkFilter)
        .withExport(symbols.parking_lot_unpark_requeue, "pl_u_requeue", .global, &parkingLotUnparkRequeue)
        .withStateSync(State, State.init, State.deinit)
        .exportModule();
};

comptime {
    _ = Instance;
}

pub const State = struct {
    debug_allocator: switch (builtin.mode) {
        .Debug, .ReleaseSafe => std.heap.DebugAllocator(.{}),
        else => void,
    },
    allocator: std.mem.Allocator,
    parking_lot: ParkingLot,

    var global_state: State = undefined;
    var global_instance: atomic.Value(?*const Instance) = .init(null);

    fn init(octx: *const Module.OpaqueInstance, set: Module.LoadingSet) !*State {
        _ = set;
        const ctx: *const Instance = @ptrCast(@alignCast(octx));
        if (State.global_instance.cmpxchgStrong(null, ctx, .monotonic, .monotonic)) |_|
            return error.AlreadyInitialized;

        switch (builtin.mode) {
            .Debug, .ReleaseSafe => {
                global_state.debug_allocator = .init;
                global_state.allocator = global_state.debug_allocator.allocator();
            },
            else => {
                global_state.allocator = std.heap.smp_allocator;
            },
        }
        global_state.parking_lot = .init(global_state.allocator);

        return &global_state;
    }

    fn deinit(octx: *const Module.OpaqueInstance, state: *State) void {
        const ctx: *const Instance = @ptrCast(@alignCast(octx));
        if (global_instance.cmpxchgStrong(ctx, null, .monotonic, .monotonic)) |_|
            @panic("already deinit");
        std.debug.assert(state == &global_state);

        global_state.parking_lot.deinit();
        switch (builtin.mode) {
            .Debug, .ReleaseSafe => {
                if (global_state.debug_allocator.deinit() == .leak) @panic("memory leak");
            },
            else => {},
        }
    }

    /// Returns the global instance of the module.
    ///
    /// May only be called if the instance is still alive. This is always true when called from an
    /// exported function, if invoked from a dependent instance, or if the caller acquired a
    /// strong reference to the instance.
    pub fn getInstance() *const Instance {
        return State.global_instance.load(.monotonic).?;
    }

    /// Tries to acquire a strong reference to the global instance of the module.
    ///
    /// Returns `null` if the instance is not alive. The `info` parameter must match the instance's
    /// info. The returned instance must be released after use.
    pub fn tryAcquireInstance(info: *const Module.Info) ?*const Instance {
        if (!info.tryRefInstanceStrong()) return null;
        const instance = getInstance();
        std.debug.assert(instance.info == info);
        return instance;
    }
};

fn taskId(id: *fimo_tasks_meta.task.Id) callconv(.c) bool {
    if (Worker.currentTask()) |curr| {
        id.* = curr.id;
        return true;
    }
    return false;
}

fn workerId(id: *fimo_tasks_meta.pool.Worker) callconv(.c) bool {
    if (Worker.currentId()) |curr| {
        id.* = curr;
        return true;
    }
    return false;
}

fn workerPool(pool: *fimo_tasks_meta.pool.Pool) callconv(.c) bool {
    if (Worker.currentPool()) |curr| {
        pool.* = curr.asMetaPool();
        return true;
    }
    return false;
}

fn workerPoolById(
    id: fimo_tasks_meta.pool.Id,
    pool: *fimo_tasks_meta.pool.Pool,
) callconv(.c) bool {
    _ = id;
    _ = pool;
    @panic("not implemented");
}

fn queryWorkerPool(query: *fimo_tasks_meta.pool.Query) callconv(.c) AnyResult {
    _ = query;
    return AnyError.initError(error.NotImplemented).intoResult();
}

fn createWorkerPool(
    config: *const fimo_tasks_meta.pool.Config,
    pool: *fimo_tasks_meta.pool.Pool,
) callconv(.c) AnyResult {
    _ = config;
    _ = pool;
    return AnyError.initError(error.NotImplemented).intoResult();
}

fn yield() callconv(.c) void {
    Worker.yieldTask();
}

fn abort() callconv(.c) void {
    Worker.abortTask();
}

fn sleep(duration: fimo_std.c.FimoDuration) callconv(.c) void {
    Worker.sleepTask(Duration.initC(duration));
}

fn taskLocalSet(
    key: *const fimo_tasks_meta.task_local.OpaqueKey,
    value: ?*anyopaque,
    dtor: ?*const fn (value: ?*anyopaque) callconv(.c) void,
) callconv(.c) void {
    _ = dtor;
    _ = value;
    _ = key;
    @panic("not implemented");
}

fn taskLocalGet(key: *const fimo_tasks_meta.task_local.OpaqueKey) callconv(.c) ?*anyopaque {
    _ = key;
    @panic("not implemented");
}

fn taskLocalClear(key: *const fimo_tasks_meta.task_local.OpaqueKey) callconv(.c) void {
    _ = key;
    @panic("not implemented");
}

fn parkingLotPark(
    key: *const anyopaque,
    validation_data: *anyopaque,
    validation: *const fn (data: *anyopaque) callconv(.c) bool,
    before_sleep_data: *anyopaque,
    before_sleep: *const fn (data: *anyopaque) callconv(.c) void,
    timed_out_data: *anyopaque,
    timed_out: *const fn (
        data: *anyopaque,
        key: *const anyopaque,
        is_last: bool,
    ) callconv(.c) void,
    token: fimo_tasks_meta.sync.ParkingLot.ParkToken,
    timeout: ?*const fimo_std.c.FimoInstant,
) callconv(.c) fimo_tasks_meta.sync.ParkingLot.ParkResult {
    return State.global_state.parking_lot.park(
        key,
        validation_data,
        validation,
        before_sleep_data,
        before_sleep,
        timed_out_data,
        timed_out,
        token,
        if (timeout) |t| Instant.initC(t.*) else null,
    );
}

fn parkingLotParkMultiple(
    keys: [*]const *const anyopaque,
    key_count: usize,
    validation_data: *anyopaque,
    validation: *const fn (data: *anyopaque, key_index: usize) callconv(.c) bool,
    before_sleep_data: *anyopaque,
    before_sleep: *const fn (data: *anyopaque) callconv(.c) void,
    token: fimo_tasks_meta.sync.ParkingLot.ParkToken,
    timeout: ?*const fimo_std.c.FimoInstant,
) callconv(.c) fimo_tasks_meta.sync.ParkingLot.ParkMultipleResult {
    return State.global_state.parking_lot.parkMultiple(
        keys[0..key_count],
        validation_data,
        validation,
        before_sleep_data,
        before_sleep,
        token,
        if (timeout) |t| Instant.initC(t.*) else null,
    );
}

fn parkingLotUnparkOne(
    key: *const anyopaque,
    callback_data: *anyopaque,
    callback: *const fn (
        data: *anyopaque,
        result: fimo_tasks_meta.sync.ParkingLot.UnparkResult,
    ) callconv(.c) fimo_tasks_meta.sync.ParkingLot.UnparkToken,
) callconv(.c) fimo_tasks_meta.sync.ParkingLot.UnparkResult {
    return State.global_state.parking_lot.unparkOne(key, callback_data, callback);
}

fn parkingLotUnparkAll(
    key: *const anyopaque,
    token: fimo_tasks_meta.sync.ParkingLot.UnparkToken,
) callconv(.c) usize {
    return State.global_state.parking_lot.unparkAll(key, token);
}

fn parkingLotUnparkFilter(
    key: *const anyopaque,
    filter_data: *anyopaque,
    filter: *const fn (
        data: *anyopaque,
        token: fimo_tasks_meta.sync.ParkingLot.ParkToken,
    ) callconv(.c) fimo_tasks_meta.sync.ParkingLot.FilterOp,
    callback_data: *anyopaque,
    callback: *const fn (
        data: *anyopaque,
        result: fimo_tasks_meta.sync.ParkingLot.UnparkResult,
    ) callconv(.c) fimo_tasks_meta.sync.ParkingLot.UnparkToken,
) callconv(.c) fimo_tasks_meta.sync.ParkingLot.UnparkResult {
    return State.global_state.parking_lot.unparkFilter(
        key,
        filter_data,
        filter,
        callback_data,
        callback,
    );
}

fn parkingLotUnparkRequeue(
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    validate_data: *anyopaque,
    validate: *const fn (data: *anyopaque) callconv(.c) fimo_tasks_meta.sync.ParkingLot.RequeueOp,
    callback_data: *anyopaque,
    callback: *const fn (
        data: *anyopaque,
        op: fimo_tasks_meta.sync.ParkingLot.RequeueOp,
        result: fimo_tasks_meta.sync.ParkingLot.UnparkResult,
    ) callconv(.c) fimo_tasks_meta.sync.ParkingLot.UnparkToken,
) callconv(.c) fimo_tasks_meta.sync.ParkingLot.UnparkResult {
    return State.global_state.parking_lot.unparkRequeue(
        key_from,
        key_to,
        validate_data,
        validate,
        callback_data,
        callback,
    );
}
