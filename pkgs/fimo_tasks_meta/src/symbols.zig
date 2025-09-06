const std = @import("std");

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const Status = ctx.Status;
const Symbol = fimo_std.modules.Symbol;
const Duration = fimo_std.time.compat.Duration;
const Instant = fimo_std.time.compat.Instant;
const SliceConst = fimo_std.utils.SliceConst;

const root = @import("root.zig");
const TaskId = root.TaskId;
const AnyTssKey = root.AnyTssKey;
const CmdBuf = root.CmdBuf;
const CmdBufHandle = root.CmdBufHandle;
const Worker = root.Worker;
const ExecutorCfg = root.ExecutorCfg;
const Executor = root.Executor;
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

/// Namespace for all symbols of the package.
pub const symbol_namespace: [:0]const u8 = "fimo-tasks";

/// Tuple containing all symbols of the package.
pub const all_symbols = .{
    task_id,
    worker_id,

    yield,
    abort,
    cancel_requested,
    sleep,

    task_local_set,
    task_local_get,
    task_local_clear,

    cmd_buf_join,
    cmd_buf_detach,
    cmd_buf_cancel,
    cmd_buf_cancel_detach,

    executor_global,
    executor_init,
    executor_current,
    executor_join,
    executor_join_requested,
    executor_enqueue,
    executor_enqueue_detached,

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
pub const cancel_requested = Symbol{
    .name = "cancel_requested",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn () callconv(.c) bool,
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
        key: *const AnyTssKey,
        value: ?*anyopaque,
        dtor: ?*const fn (value: ?*anyopaque) callconv(.c) void,
    ) callconv(.c) void,
};
pub const task_local_get = Symbol{
    .name = "task_local_get",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (key: *const AnyTssKey) callconv(.c) ?*anyopaque,
};
pub const task_local_clear = Symbol{
    .name = "task_local_clear",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (key: *const AnyTssKey) callconv(.c) void,
};

pub const cmd_buf_join = Symbol{
    .name = "cmd_buf_join",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (cmd_buf: *CmdBufHandle) callconv(.c) CmdBufHandle.CompletionStatus,
};
pub const cmd_buf_detach = Symbol{
    .name = "cmd_buf_detach",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (cmd_buf: *CmdBufHandle) callconv(.c) void,
};
pub const cmd_buf_cancel = Symbol{
    .name = "cmd_buf_cancel",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (cmd_buf: *CmdBufHandle) callconv(.c) void,
};
pub const cmd_buf_cancel_detach = Symbol{
    .name = "cmd_buf_cancel_detach",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (cmd_buf: *CmdBufHandle) callconv(.c) void,
};

pub const executor_global = Symbol{
    .name = "executor_global",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = Executor,
};
pub const executor_init = Symbol{
    .name = "executor_init",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (ex: **Executor, cfg: *const ExecutorCfg) callconv(.c) Status,
};
pub const executor_current = Symbol{
    .name = "executor_current",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn () callconv(.c) ?*Executor,
};
pub const executor_join = Symbol{
    .name = "executor_join",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (ex: *Executor) callconv(.c) void,
};
pub const executor_join_requested = Symbol{
    .name = "executor_join_requested",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (ex: *Executor) callconv(.c) bool,
};
pub const executor_enqueue = Symbol{
    .name = "executor_enqueue",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (ex: *Executor, cmds: *CmdBuf) callconv(.c) *CmdBufHandle,
};
pub const executor_enqueue_detached = Symbol{
    .name = "executor_enqueue_detached",
    .namespace = symbol_namespace,
    .version = ctx.context_version,
    .T = fn (ex: *Executor, cmds: *CmdBuf) callconv(.c) void,
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
        keys: SliceConst(Futex.KeyExpect),
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
