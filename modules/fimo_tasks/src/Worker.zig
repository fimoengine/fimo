const std = @import("std");
const atomic = std.atomic;
const Thread = std.Thread;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const ctx = fimo_std.ctx;
const tracing = fimo_std.tracing;
const Span = tracing.Span;
const CallStack = tracing.CallStack;
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
const FimoTasks = @import("FimoTasks.zig");
const Pool = @import("Pool.zig");
const Task = @import("Task.zig");

pool: *Pool,
id: MetaWorker,
thread: Thread,
active_task: ?*Task = null,
call_stack: ?*CallStack = null,
context: ?context_.Context = null,
pool_sx: Pool.PrivateChannel.Sender,
global_rx: GlobalChannel.Receiver,
private_queue: PrivateChannel = .empty,
worker_count: *const atomic.Value(usize),
global_task_count: *const atomic.Value(usize),
private_task_count: atomic.Value(usize) = .init(0),

const Self = @This();

pub const PrivateChannel = IntrusiveMpscChannel(*Task);
pub const GlobalChannel = UnorderedSpmcChannel(*Task);

threadlocal var _current: ?*Self = null;

/// Returns the pool managing the current thread.
pub fn currentPool() ?*Pool {
    const worker = _current orelse return null;
    return worker.pool;
}

pub fn currentPoolIfInTask() ?*Pool {
    const worker = _current orelse return null;
    if (worker.active_task != null) return worker.pool;
    return null;
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
    wait: struct {
        value: *const atomic.Value(u32),
        expect: u32,
        timed_out: *bool,
        timeout: ?Instant = null,
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
    const is_task = if (_current) |curr| curr.active_task != null else false;
    if (is_task) sendMsgToScheduler(.yield) else Thread.yield() catch unreachable;
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

/// Puts the current task or thread to sleep for the specified amount of time.
pub fn sleep(duration: Duration) void {
    const is_task = if (_current) |curr| curr.active_task != null else false;
    if (is_task) {
        const timeout = Instant.now().addSaturating(duration);
        sendMsgToScheduler(.{ .sleep = .{ .timeout = timeout } });
    } else {
        const nanos: usize = @truncate(@min(std.math.maxInt(usize), duration.nanos()));
        Thread.sleep(nanos);
    }
}

/// Checks if `ptr` still contains the value `expect` and if so, blocks until either:
/// - The value at `ptr` is no longer equal to `expect`.
/// - The caller is unblocked by a call to `wakeByAddress` on the owning pool.
/// - The caller is unblocked spuriously ("at random").
pub fn waitTask(ptr: *const atomic.Value(u32), expect: u32) void {
    var timed_out: bool = undefined;
    sendMsgToScheduler(.{ .wait = .{
        .value = ptr,
        .expect = expect,
        .timed_out = &timed_out,
    } });
    std.debug.assert(!timed_out);
}

/// Checks if `ptr` still contains the value `expect` and if so, blocks until either:
/// - The value at `ptr` is no longer equal to `expect`.
/// - The caller is unblocked by a call to `wakeByAddress` on the owning pool.
/// - The caller is unblocked spuriously ("at random").
/// - The caller blocks for longer than the given timeout. In which case, `error.Timeout` is returned.
pub fn timedWaitTask(ptr: *const atomic.Value(u32), expect: u32, timeout: Instant) error{Timeout}!void {
    var timed_out: bool = undefined;
    sendMsgToScheduler(.{ .wait = .{
        .value = ptr,
        .expect = expect,
        .timed_out = &timed_out,
        .timeout = timeout,
    } });
    if (timed_out) return error.Timeout;
}

fn fetchTask(self: *Self) RecvError!*Task {
    const num_workers = self.worker_count.load(.monotonic);
    const num_global_tasks = self.global_task_count.load(.monotonic);
    const num_local_tasks = self.private_task_count.load(.monotonic);

    // Try to maintain a balanced distribution of tasks among workers.
    if (num_global_tasks / num_workers > num_local_tasks) {
        const rx = multi_receiver(&.{ *Task, *Task }, .{ self.global_rx, self.private_queue.receiver() });
        const msg = try rx.recv(&FimoTasks.get().futex);
        switch (msg) {
            .@"0" => |v| {
                std.debug.assert(v.worker == null);
                return v;
            },
            .@"1" => |v| {
                std.debug.assert(v.worker == null or v.worker == self.id);
                return v;
            },
        }
    } else {
        const rx = multi_receiver(&.{ *Task, *Task }, .{ self.private_queue.receiver(), self.global_rx });
        const msg = try rx.recv(&FimoTasks.get().futex);
        switch (msg) {
            .@"0" => |v| {
                std.debug.assert(v.worker == null or v.worker == self.id);
                return v;
            },
            .@"1" => |v| {
                std.debug.assert(v.worker == null);
                return v;
            },
        }
    }
}

fn sendToPool(self: *Self, task: *Task, msg: Pool.PrivateMessage) void {
    std.debug.assert(task.msg == null);
    std.debug.assert(msg.next == null);
    task.msg = msg;

    if (task.msg) |*m|
        self.pool_sx.send(&FimoTasks.get().futex, m) catch unreachable
    else
        unreachable;
}

fn swapCallStack(
    active: *?*CallStack,
    next: *?*CallStack,
    mark_blocked: bool,
) void {
    std.debug.assert(active.* == null);
    CallStack.suspendCurrent(mark_blocked);
    const stack = next.*.?;
    next.* = null;
    active.* = stack.swapCurrent();
    CallStack.resumeCurrent();
}

pub fn taskEntry(tr: context_.Transfer) callconv(.c) noreturn {
    std.debug.assert(tr.data == 0);
    const worker = _current orelse @panic("not a worker");
    std.debug.assert(worker.context == null);
    worker.context = tr.context;

    const task = currentTask() orelse @panic("no active task");
    std.debug.assert(task.state == .init);

    {
        const span = tracing.spanTraceNamed(@src(), "pool=`{*}`, worker=`{}`, buffer=`{*}`, task=`{*}`", .{
            worker.pool,
            worker.id,
            task.owner,
            task,
        });
        defer span.exit();

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
    tracing.registerThread();
    defer tracing.unregisterThread();

    const span = tracing.spanTraceNamed(@src(), "worker event loop, worker=`{}`", .{self.id});
    defer span.exit();

    while (true) {
        // Wait until a task is available
        const task = self.fetchTask() catch {
            std.debug.assert(self.private_task_count.load(.monotonic) == 0);
            tracing.logDebug(@src(), "exiting event loop", .{});
            break;
        };
        tracing.logDebug(@src(), "received `{*}`", .{task});
        task.ensureReady();

        // Bind the task to the current worker.
        std.debug.assert(task.worker == null or task.worker == self.id);
        if (task.worker == null) {
            tracing.logDebug(@src(), "binding `{*}` to {}", .{ task, self.id });
            _ = self.private_task_count.fetchAdd(1, .monotonic);
            task.worker = self.id;
        }

        // Set the task as active.
        std.debug.assert(self.active_task == null);
        self.active_task = task;

        // Switch to the task's call stack.
        tracing.logDebug(@src(), "switching to `{*}`", .{task});
        swapCallStack(&self.call_stack, &task.call_stack, false);

        // Switch to the task's context.
        var old_result: fimo_std.AnyError.AnyResult = undefined;
        std.debug.assert(self.context == null);
        const t_ctx = task.context.?;
        task.context = null;
        old_result = ctx.replaceResult(task.local_result);
        const tr = t_ctx.yieldTo(0);
        task.local_result = ctx.replaceResult(old_result);
        tracing.logDebug(@src(), "`{*}` switching to event loop", .{task});
        task.context = tr.context;

        // Set the task as inactive.
        std.debug.assert(task.next == null);
        std.debug.assert(self.active_task != null);
        self.active_task = null;

        const request: *const TaskMessage = @ptrFromInt(tr.data);
        switch (request.*) {
            .complete => {
                // Switch back to the event loop call stack.
                swapCallStack(&task.call_stack, &self.call_stack, false);
                task.afterExit(false);
                _ = self.private_task_count.fetchSub(1, .monotonic);
                tracing.logDebug(@src(), "`{*}` completed", .{task});

                self.sendToPool(task, .{
                    .msg = .{
                        .complete = .{
                            .is_error = false,
                            .task = task,
                        },
                    },
                });
            },
            .abort => {
                // Switch back to the event loop call stack.
                swapCallStack(&task.call_stack, &self.call_stack, false);
                task.afterExit(true);
                _ = self.private_task_count.fetchSub(1, .monotonic);
                tracing.logDebug(@src(), "`{*}` aborted", .{task});

                self.sendToPool(task, .{
                    .msg = .{
                        .complete = .{
                            .is_error = true,
                            .task = task,
                        },
                    },
                });
            },
            .yield => {
                // Switch back to the event loop call stack.
                swapCallStack(&task.call_stack, &self.call_stack, false);
                tracing.logDebug(@src(), "`{*}` yielded", .{task});

                // Push the task back onto the local queue.
                const sx = self.private_queue.sender();
                sx.send(&FimoTasks.get().futex, task) catch unreachable;
            },
            .sleep => |msg| {
                // If the timeout has already expired, the operation is equivalent to a yield.
                if (Instant.now().order(msg.timeout) != .lt) {
                    // Switch back to the event loop call stack.
                    swapCallStack(&task.call_stack, &self.call_stack, false);
                    tracing.logDebug(@src(), "`{*}` sleeping, but timeout expired", .{task});

                    // Push the task back onto the local queue.
                    const sx = self.private_queue.sender();
                    sx.send(&FimoTasks.get().futex, task) catch unreachable;
                    continue;
                }

                // Switch back to the event loop call stack and block.
                swapCallStack(&task.call_stack, &self.call_stack, true);
                tracing.logDebug(@src(), "`{*}` sleeping", .{task});

                self.sendToPool(task, .{
                    .msg = .{
                        .sleep = .{
                            .timeout = .{
                                .task = task,
                                .timeout = msg.timeout,
                            },
                        },
                    },
                });
            },
            .wait => |msg| {
                // If the timeout has already expired, the operation is equivalent to a yield.
                if (msg.timeout) |t| if (Instant.now().order(t) != .lt) {
                    msg.timed_out.* = true;

                    // Switch back to the event loop call stack.
                    swapCallStack(&task.call_stack, &self.call_stack, false);
                    tracing.logDebug(@src(), "`{*}` waiting, but timeout expired", .{task});

                    // Push the task back onto the local queue.
                    const sx = self.private_queue.sender();
                    sx.send(&FimoTasks.get().futex, task) catch unreachable;
                    continue;
                };

                // Switch back to the event loop call stack.
                swapCallStack(&task.call_stack, &self.call_stack, true);
                tracing.logDebug(@src(), "`{*}` waiting", .{task});

                self.sendToPool(task, .{
                    .msg = .{
                        .wait = .{
                            .value = msg.value,
                            .expect = msg.expect,
                            .timed_out = msg.timed_out,
                            .task = task,
                            .timeout = if (msg.timeout) |t| .{
                                .task = task,
                                .timeout = t,
                            } else null,
                        },
                    },
                });
            },
        }
    }
}
