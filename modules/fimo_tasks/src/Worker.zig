const std = @import("std");
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_pool = fimo_tasks_meta.pool;
const MetaWorker = meta_pool.Worker;

const context = @import("context.zig");
const mpsc_channel = @import("mpsc_channel.zig");
const MpscChannel = mpsc_channel.Fifo;
const Pool = @import("Pool.zig");
const root = @import("root.zig");
const Task = @import("Task.zig");

pool: *Pool,
id: MetaWorker,
allocator: Allocator,
active_task: ?*Task = null,
transfer: ?context.Transfer = null,
task_queue: MpscChannel(Task) = .empty,

const Self = @This();

threadlocal var _current: ?*Self = null;

/// Returns the pool managing the current thread.
pub fn currentPool() ?*Pool {
    const worker = _current orelse return null;
    return worker.pool;
}

/// Returns the task running on the current thread.
pub fn currentTask() ?*Task {
    const worker = _current orelse return null;
    return worker.active_task;
}

/// Returns the worker ID of the current thread.
pub fn currentId() ?MetaWorker {
    const worker = _current orelse return null;
    return worker.id;
}

const TaskMessage = union(enum) {
    yield,
    abort,
    sleep: struct {
        timeout: Instant,
    },
};

fn sendMsgToScheduler(msg: TaskMessage) void {
    const worker = _current orelse @panic("not a worker");
    const tr = worker.transfer.?;
    worker.transfer = null;

    worker.transfer = tr.context.yieldTo(@intFromPtr(&msg));
}

/// Yields the current task back to the scheduler of the worker pool.
pub fn yieldTask() void {
    sendMsgToScheduler(.yield);
}

/// Aborts the current task.
pub fn abortTask() void {
    sendMsgToScheduler(.abort);
}

/// Puts the current task to sleep for the specified amount of time.
pub fn sleepTask(duration: Duration) void {
    const timeout = Instant.now().addSaturating(duration);
    sendMsgToScheduler(.{ .sleep = .{ .timeout = timeout } });
}

fn eventLoop() void {}
