const std = @import("std");
const atomic = std.atomic;
const Thread = std.Thread;

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
const Task = @import("Task.zig");

pool: *Pool,
id: MetaWorker,
thread: Thread,
active_task: ?*Task = null,
call_stack: ?CallStack = null,
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
        const msg = try rx.recv(&self.pool.runtime.futex);
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
        const msg = try rx.recv(&self.pool.runtime.futex);
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
        self.pool_sx.send(&self.pool.runtime.futex, m) catch unreachable
    else
        unreachable;
}

fn swapCallStack(
    tracing: ?Tracing,
    active: *?CallStack,
    next: *?CallStack,
    mark_blocked: bool,
) void {
    const tr = tracing orelse return;
    std.debug.assert(active.* == null);
    CallStack.suspendCurrent(tr, mark_blocked);
    const stack = next.*.?;
    next.* = null;
    active.* = stack.replaceActive();
    CallStack.resumeCurrent(tr);
}

pub fn taskEntry(tr: context_.Transfer) callconv(.c) noreturn {
    std.debug.assert(tr.data == 0);
    const worker = _current orelse @panic("not a worker");
    std.debug.assert(worker.context == null);
    worker.context = tr.context;

    const task = currentTask() orelse @panic("no active task");
    std.debug.assert(task.state == .init);

    {
        const tracing = worker.pool.runtime.tracing();
        const span = if (tracing) |tra| Tracing.Span.initTrace(
            tra,
            null,
            null,
            @src(),
            "pool=`{*}`, worker=`{}`, buffer=`{*}`, task=`{*}`",
            .{ worker.pool, worker.id, task.owner, task },
        ) else null;
        defer if (span) |sp| sp.deinit();

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
    const tracing = self.pool.runtime.tracing();
    if (tracing) |tr| tr.registerThread();
    defer if (tracing) |tr| tr.unregisterThread();

    const span = if (tracing) |tr| Tracing.Span.initTrace(
        tr,
        null,
        null,
        @src(),
        "worker event loop, worker=`{}`",
        .{self.id},
    ) else null;
    defer if (span) |sp| sp.deinit();

    while (true) {
        // Wait until a task is available
        const task = self.fetchTask() catch {
            std.debug.assert(self.private_task_count.load(.monotonic) == 0);
            self.pool.runtime.logDebug("exiting event loop", .{}, @src());
            break;
        };
        self.pool.runtime.logDebug("received `{*}`", .{task}, @src());
        task.ensureReady();

        // Bind the task to the current worker.
        std.debug.assert(task.worker == null or task.worker == self.id);
        if (task.worker == null) {
            self.pool.runtime.logDebug("binding `{*}` to {}", .{ task, self.id }, @src());
            _ = self.private_task_count.fetchAdd(1, .monotonic);
            task.worker = self.id;
        }

        // Set the task as active.
        std.debug.assert(self.active_task == null);
        self.active_task = task;

        // Switch to the task's call stack.
        self.pool.runtime.logDebug("switching to `{*}`", .{task}, @src());
        swapCallStack(tracing, &self.call_stack, &task.call_stack, false);

        // Switch to the task's context.
        std.debug.assert(self.context == null);
        const t_ctx = task.context.?;
        task.context = null;
        const tr = t_ctx.yieldTo(0);
        self.pool.runtime.logDebug("`{*}` switching to event loop", .{task}, @src());
        task.context = tr.context;

        // Set the task as inactive.
        std.debug.assert(task.next == null);
        std.debug.assert(self.active_task != null);
        self.active_task = null;

        const request: *const TaskMessage = @ptrFromInt(tr.data);
        switch (request.*) {
            .complete => {
                // Switch back to the event loop call stack.
                swapCallStack(tracing, &task.call_stack, &self.call_stack, false);
                task.afterExit(false);
                _ = self.private_task_count.fetchSub(1, .monotonic);
                self.pool.runtime.logDebug("`{*}` completed", .{task}, @src());

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
                swapCallStack(tracing, &task.call_stack, &self.call_stack, false);
                task.afterExit(true);
                _ = self.private_task_count.fetchSub(1, .monotonic);
                self.pool.runtime.logDebug("`{*}` aborted", .{task}, @src());

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
                swapCallStack(tracing, &task.call_stack, &self.call_stack, false);
                self.pool.runtime.logDebug("`{*}` yielded", .{task}, @src());

                // Push the task back onto the local queue.
                const sx = self.private_queue.sender();
                sx.send(&self.pool.runtime.futex, task) catch unreachable;
            },
            .sleep => |msg| {
                // If the timeout has already expired, the operation is equivalent to a yield.
                if (Instant.now().order(msg.timeout) != .lt) {
                    // Switch back to the event loop call stack.
                    swapCallStack(tracing, &task.call_stack, &self.call_stack, false);
                    self.pool.runtime.logDebug("`{*}` sleeping, but timeout expired", .{task}, @src());

                    // Push the task back onto the local queue.
                    const sx = self.private_queue.sender();
                    sx.send(&self.pool.runtime.futex, task) catch unreachable;
                    continue;
                }

                // Switch back to the event loop call stack and block.
                swapCallStack(tracing, &task.call_stack, &self.call_stack, true);
                self.pool.runtime.logDebug("`{*}` sleeping", .{task}, @src());

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
                    swapCallStack(tracing, &task.call_stack, &self.call_stack, false);
                    self.pool.runtime.logDebug("`{*}` waiting, but timeout expired", .{task}, @src());

                    // Push the task back onto the local queue.
                    const sx = self.private_queue.sender();
                    sx.send(&self.pool.runtime.futex, task) catch unreachable;
                    continue;
                };

                // Switch back to the event loop call stack.
                swapCallStack(tracing, &task.call_stack, &self.call_stack, true);
                self.pool.runtime.logDebug("`{*}` waiting", .{task}, @src());

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
