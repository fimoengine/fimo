const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;
const ctx = fimo_std.ctx;
const Symbol = fimo_std.modules.Symbol;
const Duration = fimo_std.time.compat.Duration;
const Instant = fimo_std.time.compat.Instant;

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
const ParkMultipleResult = ParkingLot.ParkMultipleResult;
const UnparkResult = ParkingLot.UnparkResult;
const RequeueOp = ParkingLot.RequeueOp;
const FilterOp = ParkingLot.FilterOp;
const Futex = @import("sync/Futex.zig");
const task = @import("task.zig");
const TaskId = task.Id;
const task_local = @import("task_local.zig");
const TssKey = task_local.OpaqueKey;

/// Namespace for all symbols of the package.
pub const symbol_namespace: [:0]const u8 = "fimo-tasks";

/// Tuple containing all symbols of the package.
pub const all_symbols = .{
    task_id,
    worker_id,
    worker_pool,
    worker_pool_by_id,
    query_worker_pools,
    create_worker_pool,
    yield,
    abort,
    sleep,
    task_local_set,
    task_local_get,
    task_local_clear,
    futex_wait,
    futex_waitv,
    futex_wake,
    futex_requeue,
};

pub const task_id = Symbol{
    .name = "task_id",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (id: *TaskId) callconv(.c) bool,
};
pub const worker_id = Symbol{
    .name = "worker_id",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (id: *Worker) callconv(.c) bool,
};
pub const worker_pool = Symbol{
    .name = "worker_pool",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (pool: *Pool) callconv(.c) bool,
};
pub const worker_pool_by_id = Symbol{
    .name = "worker_pool_by_id",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (id: PoolId, pool: *Pool) callconv(.c) bool,
};
pub const query_worker_pools = Symbol{
    .name = "query_worker_pools",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (query: *Query) callconv(.c) AnyResult,
};
pub const create_worker_pool = Symbol{
    .name = "create_worker_pool",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (config: *const PoolConfig, pool: *Pool) callconv(.c) AnyResult,
};
pub const yield = Symbol{
    .name = "yield",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn () callconv(.c) void,
};
pub const abort = Symbol{
    .name = "abort",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn () callconv(.c) void,
};
pub const sleep = Symbol{
    .name = "sleep",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (duration: Duration) callconv(.c) void,
};

pub const task_local_set = Symbol{
    .name = "task_local_set",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (
        key: *const TssKey,
        value: ?*anyopaque,
        dtor: ?*const fn (value: ?*anyopaque) callconv(.c) void,
    ) callconv(.c) void,
};
pub const task_local_get = Symbol{
    .name = "task_local_get",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (key: *const TssKey) callconv(.c) ?*anyopaque,
};
pub const task_local_clear = Symbol{
    .name = "task_local_clear",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (key: *const TssKey) callconv(.c) void,
};

pub const futex_wait = Symbol{
    .name = "futex_wait",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (
        key: *const anyopaque,
        key_size: usize,
        expect: u64,
        token: usize,
        timeout: ?*const Instant,
    ) callconv(.c) Futex.Status,
};

pub const futex_waitv = Symbol{
    .name = "futex_waitv",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (
        keys: [*]const Futex.KeyExpect,
        key_count: usize,
        timeout: ?*const Instant,
        wake_index: *usize,
    ) callconv(.c) Futex.Status,
};

pub const futex_wake = Symbol{
    .name = "futex_wake",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (
        key: *const anyopaque,
        max_waiters: usize,
        filter: Futex.Filter,
    ) callconv(.c) usize,
};

pub const futex_requeue = Symbol{
    .name = "futex_requeue",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (
        key_from: *const anyopaque,
        key_to: *const anyopaque,
        key_size: usize,
        expect: u64,
        max_wakes: usize,
        max_requeues: usize,
        filter: Futex.Filter,
        result: *Futex.RequeueResult,
    ) callconv(.c) Futex.Status,
};
