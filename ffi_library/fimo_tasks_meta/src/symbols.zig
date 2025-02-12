const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;
const Context = fimo_std.Context;
const Module = Context.Module;
const Symbol = Module.Symbol;
const c = fimo_std.c;
const Duration = c.FimoDuration;
const Time = c.FimoTime;

const pool = @import("pool.zig");
const Pool = pool.Pool;
const PoolConfig = pool.Config;
const PoolId = pool.Id;
const Query = pool.Query;
const Worker = pool.Worker;
const sync = @import("sync.zig");
const ParkingLot = sync.ParkingLot;
const ParkToken = ParkingLot.ParkToken;
const UnparkToken = ParkingLot.UnparkToken;
const ParkResult = ParkingLot.ParkResult;
const UnparkResult = ParkingLot.UnparkResult;
const RequeueOp = ParkingLot.RequeueOp;
const FilterOp = ParkingLot.FilterOp;
const task = @import("task.zig");
const TaskId = task.Id;
const task_local = @import("task_local.zig");
const TssKey = task_local.OpaqueKey;

/// Namespace for all symbols of the package.
pub const symbol_namespace: [:0]const u8 = "fimo-tasks";

pub const task_id = Symbol{
    .name = "task_id",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (id: *TaskId) callconv(.c) bool,
};
pub const worker_id = Symbol{
    .name = "worker_id",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (id: *Worker) callconv(.c) bool,
};
pub const worker_pool = Symbol{
    .name = "worker_pool",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (pool: *Pool) callconv(.c) bool,
};
pub const worker_pool_by_id = Symbol{
    .name = "worker_pool_by_id",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (id: PoolId, pool: *Pool) callconv(.c) bool,
};
pub const query_worker_pools = Symbol{
    .name = "query_worker_pools",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (query: *Query) callconv(.c) AnyResult,
};
pub const create_worker_pool = Symbol{
    .name = "create_worker_pool",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (config: *const PoolConfig, pool: *Pool) callconv(.c) AnyResult,
};
pub const yield = Symbol{
    .name = "yield",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn () callconv(.c) void,
};
pub const abort = Symbol{
    .name = "abort",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn () callconv(.c) void,
};
pub const sleep = Symbol{
    .name = "sleep",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (duration: Duration) callconv(.c) void,
};

pub const task_local_set = Symbol{
    .name = "task_local_set",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (
        key: *const TssKey,
        value: ?*anyopaque,
        dtor: ?*const fn (value: ?*anyopaque) callconv(.c) void,
    ) callconv(.c) void,
};
pub const task_local_get = Symbol{
    .name = "task_local_get",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (key: *const TssKey) callconv(.c) ?*anyopaque,
};
pub const task_local_clear = Symbol{
    .name = "task_local_clear",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (key: *const TssKey) callconv(.c) void,
};

pub const parking_lot_park = Symbol{
    .name = "parking_lot_park",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (
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
        token: ParkToken,
        timeout: ?*const Time,
    ) callconv(.c) ParkResult,
};
pub const parking_lot_unpark_one = Symbol{
    .name = "parking_lot_unpark_one",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (
        key: *const anyopaque,
        callback_data: *anyopaque,
        callback: *const fn (data: *anyopaque, result: UnparkResult) callconv(.c) UnparkToken,
    ) callconv(.c) UnparkResult,
};
pub const parking_lot_unpark_all = Symbol{
    .name = "parking_lot_unpark_all",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (key: *const anyopaque, token: UnparkToken) callconv(.c) usize,
};
pub const parking_lot_unpark_filter = Symbol{
    .name = "parking_lot_unpark_filter",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (
        key: *const anyopaque,
        filter_data: *anyopaque,
        filter: *const fn (data: *anyopaque, token: ParkToken) callconv(.c) FilterOp,
        callback_data: *anyopaque,
        callback: *const fn (data: *anyopaque, result: UnparkResult) callconv(.c) UnparkToken,
    ) callconv(.c) UnparkResult,
};
pub const parking_lot_unpark_requeue = Symbol{
    .name = "parking_lot_unpark_requeue",
    .namespace = symbol_namespace,
    .version = Context.context_version,
    .T = fn (
        key_from: *const anyopaque,
        key_to: *const anyopaque,
        validate_data: *anyopaque,
        validate: *const fn (data: *anyopaque) callconv(.c) RequeueOp,
        callback_data: *anyopaque,
        callback: *const fn (
            data: *anyopaque,
            op: RequeueOp,
            result: UnparkResult,
        ) callconv(.c) UnparkToken,
    ) callconv(.c) UnparkResult,
};
