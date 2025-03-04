const std = @import("std");
const Allocator = std.mem.Allocator;
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const Tracing = fimo_std.Context.Tracing;
const Span = Tracing.Span;
const CallStack = Tracing.CallStack;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_pool = fimo_tasks_meta.pool;
const MetaWorker = meta_pool.Worker;

const channel = @import("channel.zig");
const RecvError = channel.RecvError;
const IntrusiveMpscChannel = channel.IntrusiveMpscChannel;
const MultiReceiver = channel.MultiReceiver;
const multi_receiver = channel.multi_receiver;
const UnorderedSpmcChannel = channel.UnorderedSpmcChannel;
const context_ = @import("context.zig");
const Pool = @import("Pool.zig");
const root = @import("root.zig");
const State = root.State;
const Task = @import("Task.zig");

pool: *Pool,
state: *State,
id: MetaWorker,
allocator: Allocator,
active_task: ?*Task = null,
call_stack: ?CallStack = null,
context: ?context_.Context = null,
global_rx: GlobalChannel.Receiver,
private_queue: PrivateChannel = .empty,
worker_count: *const atomic.Value(usize),
global_task_count: *const atomic.Value(usize),
private_task_count: atomic.Value(usize) = .init(0),

const Self = @This();

pub const PrivateChannel = IntrusiveMpscChannel(*Task);
pub const GlobalChannel = UnorderedSpmcChannel(*Task);

pub const PoolMessage = union(enum) {};

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
    complete,
    abort,
    yield,
    sleep: struct {
        timeout: Instant,
    },
};

fn sendMsgToScheduler(msg: TaskMessage) void {
    const worker = _current orelse @panic("not a worker");
    std.debug.assert(worker.active_task != null);
    const context = worker.context.?;
    worker.context = null;

    const tr = context.yieldTo(@intFromPtr(&msg));
    std.debug.assert(tr.data == 0);
    worker.context = tr.context;
}

/// Yields the current task or thread to the scheduler.
pub fn yield() void {
    if (_current) |_| sendMsgToScheduler(.yield) else std.Thread.yield() catch unreachable;
}

/// Yields the current task back to the scheduler of the worker pool.
pub fn yieldTask() void {
    sendMsgToScheduler(.yield);
}

/// Completes the current task.
fn completeTask() noreturn {
    const worker = _current orelse @panic("not a worker");
    const task = worker.active_task orelse @panic("no active task");
    task.exit(false);

    sendMsgToScheduler(.complete);
    unreachable;
}

/// Aborts the current task.
pub fn abortTask() noreturn {
    const worker = _current orelse @panic("not a worker");
    const task = worker.active_task orelse @panic("no active task");
    task.exit(true);

    sendMsgToScheduler(.abort);
    unreachable;
}

/// Puts the current task to sleep for the specified amount of time.
pub fn sleepTask(duration: Duration) void {
    const timeout = Instant.now().addSaturating(duration);
    sendMsgToScheduler(.{ .sleep = .{ .timeout = timeout } });
}

fn fetchTask(self: *Self) RecvError!*Task {
    const num_workers = self.worker_count.load(.monotonic);
    const num_global_tasks = self.global_task_count.load(.monotonic);
    const num_local_tasks = self.private_task_count.load(.monotonic);

    // Try to maintain a balanced distribution of tasks among workers.
    if (num_global_tasks / num_workers > num_local_tasks) {
        const rx = multi_receiver(*Task, .{ self.global_rx, self.private_queue.receiver() });
        return rx.recv(&self.state.parking_lot);
    } else {
        const rx = multi_receiver(*Task, .{ self.private_queue.receiver(), self.global_rx });
        return rx.recv(&self.state.parking_lot);
    }
}

fn swapCallStack(
    tracing: Tracing,
    active: *?CallStack,
    next: *?CallStack,
    mark_blocked: bool,
) void {
    std.debug.assert(active.* == null);
    CallStack.suspendCurrent(tracing, mark_blocked);
    const stack = next.*.?;
    next.* = null;
    active.* = stack.replaceActive();
    CallStack.resumeCurrent(tracing);
}

pub fn taskEntry(tr: context_.Transfer) callconv(.c) noreturn {
    std.debug.assert(tr.data == 0);
    const worker = _current orelse @panic("not a worker");
    std.debug.assert(worker.context == null);
    worker.context = tr.context;

    const task = currentTask() orelse @panic("no active task");
    std.debug.assert(task.state == .init);

    {
        const tracing = State.getInstance().context().tracing();

        const pool_label: []const u8 = worker.pool.label orelse "<unlabelled>";
        const task_label: []const u8 = task.task.label() orelse "<unlabelled>";
        const span = Tracing.Span.initTrace(
            tracing,
            null,
            null,
            @src(),
            "pool: `{s}`, worker: `{}`, task: `{s}`, id: `{}`",
            .{ pool_label, @intFromEnum(worker.id), task_label, @intFromEnum(task.id) },
        );
        defer span.deinit();

        task.state = .running;
        task.task.on_start(task.task);
    }

    completeTask();
}

pub fn run(self: *Self) void {
    // Initialize the thread local.
    _current = self;
    defer _current = null;

    // Initialize the tracing for the worker.
    const ctx = self.pool.instance.context();
    const tracing = ctx.tracing();
    tracing.registerThread();
    defer tracing.unregisterThread();

    const span = Tracing.Span.initTrace(
        tracing,
        null,
        null,
        @src(),
        "worker event loop, worker: `{}`",
        .{@intFromEnum(self.id)},
    );
    defer span.deinit();

    while (true) {
        // Wait until a task is available
        const task = self.fetchTask() catch {
            std.debug.assert(self.private_task_count.load(.monotonic) == 0);
            break;
        };
        task.ensureReady();

        // Bind the task to the current worker.
        std.debug.assert(task.worker == null or task.worker == self.id);
        if (task.worker == null) _ = self.private_task_count.fetchAdd(1, .monotonic);
        task.worker = self.id;

        // Set the task as active.
        std.debug.assert(self.active_task == null);
        self.active_task = task;

        // Switch to the task's call stack.
        swapCallStack(tracing, &self.call_stack, &task.call_stack, false);

        // Switch to the task's context.
        std.debug.assert(self.context == null);
        const t_ctx = task.context.?;
        task.context = null;
        const tr = t_ctx.yieldTo(0);
        task.context = tr.context;

        // Set the task as inactive.
        std.debug.assert(self.active_task != null);
        self.active_task = null;

        const request: *const TaskMessage = @ptrFromInt(tr.data);
        switch (request.*) {
            .complete => {
                // Switch back to the event loop call stack.
                swapCallStack(tracing, &task.call_stack, &self.call_stack, false);
                task.afterExit(false);
                _ = self.private_task_count.fetchSub(1, .monotonic);

                // TODO: Send completion notification to the pool.
                {
                    @panic("unimplemented");
                }
            },
            .abort => {
                // Switch back to the event loop call stack.
                swapCallStack(tracing, &task.call_stack, &self.call_stack, false);
                task.afterExit(true);
                _ = self.private_task_count.fetchSub(1, .monotonic);

                // TODO: Send abort notification to the pool.
                {
                    @panic("unimplemented");
                }
            },
            .yield => {
                // Switch back to the event loop call stack.
                swapCallStack(tracing, &task.call_stack, &self.call_stack, false);

                // Push the task back onto the local queue.
                const sx = self.private_queue.sender();
                sx.send(&self.state.parking_lot, task) catch unreachable;
            },
            .sleep => |msg| {
                // If the timeout has already expired, the operation is equivalent to a yield.
                if (Instant.now().order(msg.timeout) != .lt) {
                    // Switch back to the event loop call stack.
                    swapCallStack(tracing, &task.call_stack, &self.call_stack, false);

                    // Push the task back onto the local queue.
                    const sx = self.private_queue.sender();
                    sx.send(&self.state.parking_lot, task) catch unreachable;
                    continue;
                }

                // Switch back to the event loop call stack and block.
                swapCallStack(tracing, &task.call_stack, &self.call_stack, true);

                // TODO: Notify the pool that the task is sleeping.
                {
                    @panic("unimplemented");
                }
            },
        }
    }
}
