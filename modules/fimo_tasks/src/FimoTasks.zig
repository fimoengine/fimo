const std = @import("std");
const Allocator = std.mem.Allocator;
const atomic = std.atomic;
const ArrayList = std.ArrayList;
const builtin = @import("builtin");

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const Status = ctx.Status;
const modules = fimo_std.modules;
const tracing = fimo_std.tracing;
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const symbols = fimo_tasks_meta.symbols;
const win32 = @import("win32");

const context = @import("context.zig");
const Executor = @import("Executor.zig");
const Worker = Executor.Worker;
const CmdBuf = Executor.CmdBuf;
const Futex = @import("Futex.zig");

debug_allocator: switch (builtin.mode) {
    .Debug, .ReleaseSafe => std.heap.DebugAllocator(.{}),
    else => void,
},
allocator: Allocator,
futex: Futex,
executor: *Executor,

pub const default_cmd_buf_capacity = 128;
pub const default_worker_count = 0; // One worker per cpu core.
pub const default_max_load_factor = 16;
pub const default_stack_size = 8 * 1024 * 1024;
pub const default_worker_stack_cache_len = 4;
pub const Module = modules.Module(@This());

comptime {
    _ = Module;
}

pub const fimo_module = .{
    .name = .fimo_tasks,
    .author = "fimo",
    .description = "Multi-threaded tasks runtime",
    .license = "MIT OR APACHE 2.0",
};

pub const fimo_parameters = .{
    .default_cmd_buf_capacity = .{
        .default = @as(u16, default_cmd_buf_capacity),
        .read_group = .dependency,
        .write_group = .dependency,
    },
    .default_worker_count = .{
        .default = @as(u8, default_worker_count),
        .read_group = .dependency,
        .write_group = .dependency,
    },
    .default_max_load_factor = .{
        .default = @as(u8, default_max_load_factor),
        .read_group = .dependency,
        .write_group = .dependency,
    },
    .default_stack_size = .{
        .default = @as(u32, default_stack_size),
        .read_group = .dependency,
        .write_group = .dependency,
    },
    .default_worker_stack_cache_len = .{
        .default = @as(u8, default_worker_stack_cache_len),
        .read_group = .dependency,
        .write_group = .dependency,
    },
};

pub const fimo_exports = .{
    .{ .symbol = symbols.task_id, .value = &taskId },
    .{ .symbol = symbols.worker_id, .value = &workerId },
    .{ .symbol = symbols.yield, .value = &yield },
    .{ .symbol = symbols.abort, .value = &abort },
    .{ .symbol = symbols.cancel_requested, .value = &cancelRequested },
    .{ .symbol = symbols.sleep, .value = &sleep },
    .{ .symbol = symbols.task_local_set, .value = &taskLocalSet },
    .{ .symbol = symbols.task_local_get, .value = &taskLocalGet },
    .{ .symbol = symbols.task_local_clear, .value = &taskLocalClear },
    .{ .symbol = symbols.cmd_buf_join, .value = &cmdBufJoin },
    .{ .symbol = symbols.cmd_buf_detach, .value = &cmdBufDetach },
    .{ .symbol = symbols.cmd_buf_cancel, .value = &cmdBufCancel },
    .{ .symbol = symbols.cmd_buf_cancel_detach, .value = &cmdBufCancelDetach },
    .{ .symbol = symbols.executor_global, .value = .{ .init = executorGlobal } },
    .{ .symbol = symbols.executor_new, .value = &executorNew },
    .{ .symbol = symbols.executor_current, .value = &executorCurrent },
    .{ .symbol = symbols.executor_join, .value = &executorJoin },
    .{ .symbol = symbols.executor_join_requested, .value = &executorJoinRequested },
    .{ .symbol = symbols.executor_enqueue, .value = &executorEnqueue },
    .{ .symbol = symbols.executor_enqueue_detached, .value = &executorEnqueueDetached },
    .{ .symbol = symbols.futex_wait, .value = &futexWait },
    .{ .symbol = symbols.futex_waitv, .value = &futexWaitv },
    .{ .symbol = symbols.futex_wake, .value = &futexWake },
    .{ .symbol = symbols.futex_requeue, .value = &futexRequeue },
};

pub const fimo_events = .{
    .init = init,
    .deinit = deinit,
};

fn init(self: *@This()) !void {
    if (comptime builtin.target.os.tag == .windows) {
        if (win32.media.timeBeginPeriod(1) != win32.media.TIMERR_NOERROR) {
            tracing.logWarn(@src(), "`timeBeginPeriod` failed, defaulting to default timer resolution", .{});
        }
    }

    const allocator = if (@TypeOf(self.debug_allocator) != void) blk: {
        self.debug_allocator = .init;
        break :blk self.debug_allocator.allocator();
    } else std.heap.smp_allocator;
    self.allocator = allocator;
    self.futex = .init(allocator);
    self.executor = try Executor.init(.{
        .label = "default executor",
        .futex = &self.futex,
        .allocator = allocator,
        .cmd_buf_capacity = default_cmd_buf_capacity,
        .worker_count = @max(std.Thread.getCpuCount() catch 1, 1),
        .max_load_factor = default_max_load_factor,
        .stack_size = default_stack_size,
        .worker_stack_cache_len = default_worker_stack_cache_len,
    });
}

fn deinit(self: *@This()) void {
    // TODO(gabriel): This is blocking. Switch to a polling alternative.
    self.executor.join(&self.futex);
    self.futex.deinit();

    if (@TypeOf(self.debug_allocator) != void)
        if (self.debug_allocator.deinit() == .leak) @panic("memory leak");
    self.* = undefined;
    if (comptime builtin.target.os.tag == .windows) _ = win32.media.timeEndPeriod(1);
}

pub fn get() *@This() {
    return Module.state();
}

pub fn getDefaultCmdBufCapacity() usize {
    const param = Module.parameters().default_cmd_buf_capacity;
    const count: usize = @intCast(param.read());
    if (count == 0) return default_cmd_buf_capacity;
    return count;
}

pub fn getDefaultWorkerCount() usize {
    const param = Module.parameters().default_worker_count;
    const count: usize = @intCast(param.read());
    if (count == 0) return std.Thread.getCpuCount() catch 1;
    return count;
}

pub fn getDefaultMaxLoadFactor() usize {
    const param = Module.parameters().default_max_load_factor;
    const count: usize = @intCast(param.read());
    if (count == 0) return default_max_load_factor;
    return count;
}

pub fn getDefaultStackSize() usize {
    const min = context.minStackSize();
    const max = context.maxStackSize();

    const param = Module.parameters().default_stack_size;
    const size: usize = @intCast(param.read());
    if (size < min) return min;
    if (size > max) return max;
    return size;
}

pub fn getWorkerStackCacheLen() usize {
    const param = Module.parameters().default_worker_stack_cache_len;
    const count: usize = @intCast(param.read());
    if (count == 0) return default_worker_stack_cache_len;
    return count;
}

fn taskId(id: *fimo_tasks_meta.TaskId) callconv(.c) bool {
    if (Worker.currentTask()) |curr| {
        id.* = @enumFromInt(@intFromPtr(curr));
        return true;
    }
    return false;
}

fn workerId(id: *fimo_tasks_meta.Worker) callconv(.c) bool {
    if (Worker.currentId()) |curr| {
        id.* = curr;
        return true;
    }
    return false;
}

fn yield() callconv(.c) void {
    Worker.yield();
}

fn abort() callconv(.c) void {
    Worker.abort();
}

fn cancelRequested() callconv(.c) bool {
    return Worker.cancelRequested();
}

fn sleep(duration: fimo_std.time.compat.Duration) callconv(.c) void {
    Worker.sleep(Duration.initC(duration));
}

fn taskLocalSet(
    key: *const fimo_tasks_meta.AnyTssKey,
    value: ?*anyopaque,
    dtor: ?*const fn (value: ?*anyopaque) callconv(.c) void,
) callconv(.c) void {
    const task = Worker.currentTask() orelse @panic("not a task");
    task.setLocal(.{ .key = key, .value = value, .dtor = dtor });
}

fn taskLocalGet(key: *const fimo_tasks_meta.AnyTssKey) callconv(.c) ?*anyopaque {
    const task = Worker.currentTask() orelse @panic("not a task");
    return task.getLocal(key);
}

fn taskLocalClear(key: *const fimo_tasks_meta.AnyTssKey) callconv(.c) void {
    const task = Worker.currentTask() orelse @panic("not a task");
    task.clearLocal(key);
}

pub fn cmdBufJoin(
    handle: *fimo_tasks_meta.CmdBufHandle,
) callconv(.c) fimo_tasks_meta.CmdBufHandle.CompletionStatus {
    const self = get();
    const cmd_buf: *CmdBuf = @ptrCast(@alignCast(handle));
    return cmd_buf.join(&self.futex);
}

pub fn cmdBufDetach(handle: *fimo_tasks_meta.CmdBufHandle) callconv(.c) void {
    const self = get();
    const cmd_buf: *CmdBuf = @ptrCast(@alignCast(handle));
    cmd_buf.detach(&self.futex);
}

pub fn cmdBufCancel(handle: *fimo_tasks_meta.CmdBufHandle) callconv(.c) void {
    const self = get();
    const cmd_buf: *CmdBuf = @ptrCast(@alignCast(handle));
    cmd_buf.cancel(&self.futex);
}

pub fn cmdBufCancelDetach(handle: *fimo_tasks_meta.CmdBufHandle) callconv(.c) void {
    const self = get();
    const cmd_buf: *CmdBuf = @ptrCast(@alignCast(handle));
    cmd_buf.cancelDetach(&self.futex);
}

pub fn executorGlobal() callconv(.c) *fimo_tasks_meta.Executor {
    const self = get();
    return @ptrCast(self.executor);
}

pub fn executorNew(
    exe: **fimo_tasks_meta.Executor,
    cfg: *const fimo_tasks_meta.ExecutorCfg,
) callconv(.c) Status {
    const self = get();
    const options: Executor.InitOptions = .{
        .label = cfg.label.get(),
        .futex = &self.futex,
        .allocator = self.allocator,
        .cmd_buf_capacity = if (cfg.cmd_buf_capacity == 0) getDefaultCmdBufCapacity() else cfg.cmd_buf_capacity,
        .worker_count = if (cfg.worker_count == 0) getDefaultWorkerCount() else cfg.worker_count,
        .max_load_factor = if (cfg.max_load_factor == 0) getDefaultMaxLoadFactor() else cfg.max_load_factor,
        .stack_size = if (cfg.stack_size == 0) getDefaultStackSize() else cfg.stack_size,
        .worker_stack_cache_len = if (cfg.worker_stack_cache_len == 0) getWorkerStackCacheLen() else cfg.worker_stack_cache_len,
        .disable_stack_cache = cfg.disable_stack_cache,
    };
    const executor = Executor.init(options) catch |e| {
        ctx.setResult(.initErr(.initError(e)));
        return .err;
    };
    exe.* = @ptrCast(executor);
    return .ok;
}

pub fn executorCurrent() callconv(.c) ?*fimo_tasks_meta.Executor {
    return @ptrCast(Worker.currentExecutor());
}

pub fn executorJoin(exe: *fimo_tasks_meta.Executor) callconv(.c) void {
    const self = get();
    const executor: *Executor = @ptrCast(@alignCast(exe));
    std.debug.assert(executor != self.executor);
    executor.join(&self.futex);
}

pub fn executorJoinRequested(exe: *fimo_tasks_meta.Executor) callconv(.c) bool {
    const executor: *Executor = @ptrCast(@alignCast(exe));
    return executor.joinRequested();
}

pub fn executorEnqueue(
    exe: *fimo_tasks_meta.Executor,
    cmd_buf: *fimo_tasks_meta.CmdBuf,
) callconv(.c) *fimo_tasks_meta.CmdBufHandle {
    const self = get();
    const executor: *Executor = @ptrCast(@alignCast(exe));
    return @ptrCast(executor.enqueue(&self.futex, cmd_buf));
}

pub fn executorEnqueueDetached(
    exe: *fimo_tasks_meta.Executor,
    cmd_buf: *fimo_tasks_meta.CmdBuf,
) callconv(.c) void {
    const self = get();
    const executor: *Executor = @ptrCast(@alignCast(exe));
    executor.enqueueDetached(&self.futex, cmd_buf);
}

fn futexWait(
    key: *const anyopaque,
    key_size: usize,
    expect: u64,
    token: usize,
    timeout: ?*const fimo_std.time.compat.Instant,
) callconv(.c) fimo_tasks_meta.sync.Futex.Status {
    const self = get();
    self.futex.wait(
        key,
        key_size,
        expect,
        token,
        if (timeout) |t| Instant.initC(t.*) else null,
    ) catch |err| switch (err) {
        error.Invalid => return .Invalid,
        error.Timeout => return .Timeout,
    };
    return .Ok;
}

fn futexWaitv(
    keys: [*]const fimo_tasks_meta.sync.Futex.KeyExpect,
    key_count: usize,
    timeout: ?*const fimo_std.time.compat.Instant,
    wake_index: *usize,
) callconv(.c) fimo_tasks_meta.sync.Futex.Status {
    const self = get();
    wake_index.* = self.futex.waitv(
        keys[0..key_count],
        if (timeout) |t| Instant.initC(t.*) else null,
    ) catch |err| switch (err) {
        error.KeyError => return .KeyError,
        error.Invalid => return .Invalid,
        error.Timeout => return .Timeout,
    };
    return .Ok;
}

fn futexWake(
    key: *const anyopaque,
    max_waiters: usize,
    filter: fimo_tasks_meta.sync.Futex.Filter,
) callconv(.c) usize {
    const self = get();
    return self.futex.wakeFilter(key, max_waiters, filter);
}

fn futexRequeue(
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    key_size: usize,
    expect: u64,
    max_wakes: usize,
    max_requeues: usize,
    filter: fimo_tasks_meta.sync.Futex.Filter,
    result: *fimo_tasks_meta.sync.Futex.RequeueResult,
) callconv(.c) fimo_tasks_meta.sync.Futex.Status {
    const self = get();
    result.* = self.futex.requeueFilter(
        key_from,
        key_to,
        key_size,
        expect,
        max_wakes,
        max_requeues,
        filter,
    ) catch |err| switch (err) {
        error.Invalid => return .Invalid,
    };
    return .Ok;
}
