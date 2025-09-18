const fimo_std = @import("fimo_std");
const Arena = fimo_std.memory.Arena;
const Symbol = fimo_std.modules.Symbol;
const Status = fimo_std.ctx.Status;
const context_version = fimo_std.ctx.context_version;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Executor = fimo_tasks_meta.Executor;
const Fence = fimo_tasks_meta.sync.Fence;

const root = @import("root.zig");
const World = root.World;
const Scheduler = root.Scheduler;
const Sys = root.Sys;

const Res = root.Res(anyopaque);

/// Namespace for all symbols of the package.
pub const symbol_namespace: [:0]const u8 = "fimo-worlds";

/// Tuple containing all symbols of the package.
pub const all_symbols = .{
    world_init,
    world_deinit,
    world_add_res,
    resource_deinit,
    resource_lock_read,
    resource_unlock_read,
    resource_lock_write,
    resource_unlock_write,
    world_add_scheduler,
    scheduler_deinit,
    scheduler_add_sys,
    sys_deinit,
    scheduler_run,
    scheduler_schedule,
    scheduler_flush,
};

pub const world_init = Symbol{
    .name = "world_init",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: **World, desc: *const World.Desc) callconv(.c) Status,
};
pub const world_deinit = Symbol{
    .name = "world_deinit",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World) callconv(.c) void,
};
pub const world_add_res = Symbol{
    .name = "world_add_res",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, desc: *const Res.Desc) callconv(.c) *Res,
};
pub const resource_deinit = Symbol{
    .name = "resource_deinit",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (res: *Res) callconv(.c) void,
};
pub const resource_lock_read = Symbol{
    .name = "resource_lock_read",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (res: *Res) callconv(.c) *anyopaque,
};
pub const resource_unlock_read = Symbol{
    .name = "resource_unlock_read",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (res: *Res) callconv(.c) void,
};
pub const resource_lock_write = Symbol{
    .name = "resource_lock_write",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (res: *Res) callconv(.c) *anyopaque,
};
pub const resource_unlock_write = Symbol{
    .name = "resource_unlock_write",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (res: *Res) callconv(.c) void,
};
pub const world_add_scheduler = Symbol{
    .name = "world_add_scheduler",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, desc: *const Scheduler.Desc) callconv(.c) *Scheduler,
};
pub const scheduler_deinit = Symbol{
    .name = "scheduler_deinit",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (sched: *Scheduler) callconv(.c) void,
};
pub const scheduler_add_sys = Symbol{
    .name = "scheduler_add_sys",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (sched: *Scheduler, desc: *const Sys.Desc, sys: **Sys) callconv(.c) Status,
};
pub const sys_deinit = Symbol{
    .name = "sys_deinit",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (sys: *Sys, fence: ?*Fence) callconv(.c) void,
};
pub const scheduler_run = Symbol{
    .name = "scheduler_run",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (sched: *Scheduler, arena: *Arena) callconv(.c) void,
};
pub const scheduler_schedule = Symbol{
    .name = "scheduler_schedule",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (sched: *Scheduler, arena: *Arena, start: ?*Fence, completion: ?*Fence) callconv(.c) void,
};
pub const scheduler_flush = Symbol{
    .name = "scheduler_flush",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (sched: *Scheduler) callconv(.c) void,
};
