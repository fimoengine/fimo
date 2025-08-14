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

const context = @import("context.zig");
const Futex = @import("Futex.zig");
const Pool = @import("Pool.zig");
const PoolMap = @import("PoolMap.zig");
const Worker = @import("Worker.zig");

debug_allocator: switch (builtin.mode) {
    .Debug, .ReleaseSafe => std.heap.DebugAllocator(.{}),
    else => void,
},
allocator: Allocator,
futex: Futex,
pool_map: PoolMap = .{},

pub const default_stack_size: usize = 8 * 1024 * 1024;
pub const default_worker_count: usize = 0; // One worker per cpu core.
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
    .default_stack_size = .{
        .default = @as(u32, @intCast(default_stack_size)),
        .read_group = .dependency,
        .write_group = .dependency,
    },
    .default_worker_count = .{
        .default = @as(u8, @intCast(default_worker_count)),
        .read_group = .dependency,
        .write_group = .dependency,
    },
};

pub const fimo_exports = .{
    .{ .symbol = symbols.task_id, .value = &taskId },
    .{ .symbol = symbols.worker_id, .value = &workerId },
    .{ .symbol = symbols.worker_pool, .value = &workerPool },
    .{ .symbol = symbols.worker_pool_by_id, .value = &workerPoolById },
    .{ .symbol = symbols.query_worker_pools, .value = &queryWorkerPool },
    .{ .symbol = symbols.create_worker_pool, .value = &createWorkerPool },
    .{ .symbol = symbols.yield, .value = &yield },
    .{ .symbol = symbols.abort, .value = &abort },
    .{ .symbol = symbols.sleep, .value = &sleep },
    .{ .symbol = symbols.task_local_set, .value = &taskLocalSet },
    .{ .symbol = symbols.task_local_get, .value = &taskLocalGet },
    .{ .symbol = symbols.task_local_clear, .value = &taskLocalClear },
    .{ .symbol = symbols.futex_wait, .value = &futexWait },
    .{ .symbol = symbols.futex_waitv, .value = &futexWaitv },
    .{ .symbol = symbols.futex_wake, .value = &futexWake },
    .{ .symbol = symbols.futex_requeue, .value = &futexRequeue },
};

pub const fimo_events = .{
    .init = init,
    .deinit = deinit,
};

extern "winmm" fn timeBeginPeriod(uPeriod: c_uint) callconv(.winapi) c_uint;
extern "winmm" fn timeEndPeriod(uPeriod: c_uint) callconv(.winapi) c_uint;

fn init(self: *@This()) void {
    if (comptime builtin.target.os.tag == .windows) {
        if (timeBeginPeriod(1) != 0) {
            tracing.logWarn(@src(), "`timeBeginPeriod` failed, defaulting to default timer resolution", .{});
        }
    }

    const allocator = if (@TypeOf(self.debug_allocator) != void) blk: {
        self.debug_allocator = .init;
        break :blk self.debug_allocator.allocator();
    } else std.heap.smp_allocator;
    self.allocator = allocator;
    self.futex = .init(allocator);
}

fn deinit(self: *@This()) void {
    self.pool_map.deinit(self.allocator);
    self.futex.deinit();

    if (@TypeOf(self.debug_allocator) != void)
        if (self.debug_allocator.deinit() == .leak) @panic("memory leak");
    self.* = undefined;
    if (comptime builtin.target.os.tag == .windows) _ = timeEndPeriod(1);
}

pub fn get() *@This() {
    return Module.state();
}

pub fn getDefaultStackSize() usize {
    const min = context.StackAllocator.minStackSize();
    const max = context.StackAllocator.maxStackSize();

    const param = Module.parameters().default_stack_size;
    const size: usize = @intCast(param.read());
    if (size < min) return min;
    if (size > max) return max;
    return size;
}

pub fn getDefaultWorkerCount() usize {
    const param = Module.parameters().default_worker_count;
    const count: usize = @intCast(param.read());
    if (count == 0) return std.Thread.getCpuCount() catch 1;
    return count;
}

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
        pool.* = curr.ref().asMetaPool();
        return true;
    }
    return false;
}

fn workerPoolById(
    id: fimo_tasks_meta.pool.Id,
    pool: *fimo_tasks_meta.pool.Pool,
) callconv(.c) bool {
    const self = get();
    const p = self.pool_map.queryPoolById(id) orelse return false;
    pool.* = p.asMetaPool();
    return true;
}

fn queryWorkerPool(query: *fimo_tasks_meta.pool.Query) callconv(.c) Status {
    const self = get();
    const q = self.pool_map.queryAllPools(self.allocator) catch |err| {
        ctx.setResult(.initErr(.initError(err)));
        return .err;
    };

    const deinitFn = struct {
        fn f(root: ?*fimo_tasks_meta.pool.Query.Node) callconv(.c) void {
            var len: usize = 0;
            var current: ?*fimo_tasks_meta.pool.Query.Node = root orelse return;
            while (current) |curr| {
                const pool: *Pool = @ptrCast(@alignCast(curr.pool.data));
                pool.unref();
                len += 1;
                current = curr.next;
            }

            const nodes = @as([*]fimo_tasks_meta.pool.Query.Node, @ptrCast(root))[0..len];
            const allocator = get().allocator;
            allocator.free(nodes);
        }
    }.f;

    if (q.len == 0) {
        get().allocator.free(q);
        query.* = .{ .root = null, .deinit_fn = &deinitFn };
    } else {
        query.* = .{ .root = &q[0], .deinit_fn = &deinitFn };
    }
    return .ok;
}

fn createWorkerPool(
    config: *const fimo_tasks_meta.pool.Config,
    pool: *fimo_tasks_meta.pool.Pool,
) callconv(.c) Status {
    const self = get();
    const allocator = self.allocator;

    if (config.next != null) {
        tracing.logErr(@src(), "the next key is reserved for future use, pool=`{s}`", .{config.label()});
        ctx.setResult(.initErr(.initError(error.InvalidConfig)));
        return .err;
    }
    if (config.stacks_len == 0) {
        tracing.logErr(@src(), "expected at least one stack, pool=`{s}`", .{config.label()});
        ctx.setResult(.initErr(.initError(error.InvalidConfig)));
        return .err;
    }
    if (config.default_stack_index >= config.stacks_len) {
        tracing.logErr(@src(), "default stack index out of bounds, pool=`{s}`, stacks=`{}`, default=`{}`", .{
            config.label(),
            config.stacks_len,
            config.default_stack_index,
        });
        ctx.setResult(.initErr(.initError(error.InvalidConfig)));
        return .err;
    }
    for (config.stacks(), 0..) |stack, i| {
        if (stack.next != null) {
            tracing.logErr(@src(), "the next key is reserved for future use, pool=`{s}`, stack_index=`{}`", .{
                config.label(),
                i,
            });
            ctx.setResult(.initErr(.initError(error.InvalidConfig)));
            return .err;
        }
        if (stack.preallocated_count > stack.max_allocated) {
            tracing.logErr(@src(), "number of preallocated stacks exceeds the specified maximum number of stacks," ++
                " pool=`{s}`, stack_index=`{}`, preallocated=`{}`, max_allocated=`{}`", .{
                config.label(),
                i,
                stack.preallocated_count,
                stack.max_allocated,
            });
            ctx.setResult(.initErr(.initError(error.InvalidConfig)));
            return .err;
        }
        if (stack.cold_count + stack.hot_count < stack.preallocated_count) {
            tracing.logErr(@src(), "number of preallocated stacks stacks exceeds the combined cold and hot stacks count," ++
                " pool=`{s}`, stack_index=`{}`, preallocated=`{}`, cold=`{}`, hot=`{}`", .{
                config.label(),
                i,
                stack.preallocated_count,
                stack.cold_count,
                stack.hot_count,
            });
            ctx.setResult(.initErr(.initError(error.InvalidConfig)));
            return .err;
        }
        if (stack.cold_count + stack.hot_count > stack.max_allocated) {
            tracing.logErr(@src(), "number of cold and hot stacks exceeds the specified maximum number of stacks," ++
                " pool=`{s}`, stack_index=`{}`, cold=`{}`, hot=`{}`, max_allocated=`{}`", .{
                config.label(),
                i,
                stack.cold_count,
                stack.hot_count,
                stack.max_allocated,
            });
            ctx.setResult(.initErr(.initError(error.InvalidConfig)));
            return .err;
        }
    }

    const min_stack_size = context.StackAllocator.minStackSize();
    const max_stack_size = context.StackAllocator.maxStackSize();

    const Pair = struct {
        size: usize,
        idx: usize,
        is_default: bool,
        skip: bool = false,

        fn cmp(c: void, a: @This(), b: @This()) bool {
            _ = c;
            return a.size < b.size;
        }
    };
    var stacks = ArrayList(Pair).initCapacity(allocator, config.stacks_len) catch |err| {
        ctx.setResult(.initErr(.initError(err)));
        return .err;
    };
    defer stacks.deinit(allocator);

    for (config.stacks(), 0..) |stack, i| {
        const size = if (stack.size != .default)
            @max(@min(@intFromEnum(stack.size), max_stack_size), min_stack_size)
        else
            getDefaultStackSize();
        stacks.appendAssumeCapacity(.{
            .size = size,
            .idx = i,
            .is_default = i == config.default_stack_index,
        });
    }
    std.mem.sort(Pair, stacks.items, {}, Pair.cmp);

    var num_filtered_stacks: usize = 0;
    for (stacks.items[0 .. stacks.items.len - 1], stacks.items[1..]) |*curr, *next| {
        if (curr.size == next.size) {
            curr.skip = true;
            next.is_default = curr.is_default;
            curr.is_default = false;
            num_filtered_stacks += 1;
        }
    }

    const num_stacks = config.stacks_len - num_filtered_stacks;
    var default_stack_idx: usize = undefined;
    var stack_cfg = ArrayList(Pool.InitOptions.StackOptions)
        .initCapacity(allocator, num_stacks) catch |err| {
        ctx.setResult(.initErr(.initError(err)));
        return .err;
    };
    defer stack_cfg.deinit(allocator);
    for (stacks.items) |stack| {
        if (stack.skip) continue;
        if (stack.is_default) default_stack_idx = stack_cfg.items.len;
        const cfg = config.stacks()[stack.idx];
        stack_cfg.appendAssumeCapacity(.{
            .size = stack.size,
            .preallocated = cfg.preallocated_count,
            .cold = cfg.cold_count,
            .hot = cfg.hot_count,
            .max_allocated = if (cfg.max_allocated != 0)
                cfg.max_allocated
            else
                std.math.maxInt(usize),
        });
    }

    const options = Pool.InitOptions{
        .allocator = allocator,
        .label = config.label(),
        .stacks = stack_cfg.items,
        .default_stack = default_stack_idx,
        .worker_count = if (config.worker_count == 0)
            getDefaultWorkerCount()
        else
            config.worker_count,
        .is_public = config.is_queryable,
    };

    const p = self.pool_map.spawnPool(self.allocator, options) catch |err| {
        ctx.setResult(.initErr(.initError(err)));
        return .err;
    };
    pool.* = p.asMetaPool();
    return .ok;
}

fn yield() callconv(.c) void {
    Worker.yield();
}

fn abort() callconv(.c) void {
    Worker.abortTask();
}

fn sleep(duration: fimo_std.time.compat.Duration) callconv(.c) void {
    Worker.sleep(Duration.initC(duration));
}

fn taskLocalSet(
    key: *const fimo_tasks_meta.task_local.OpaqueKey,
    value: ?*anyopaque,
    dtor: ?*const fn (value: ?*anyopaque) callconv(.c) void,
) callconv(.c) void {
    const task = Worker.currentTask() orelse @panic("not a task");
    task.setLocal(key, .{ .value = value, .dtor = dtor }) catch @panic("oom");
}

fn taskLocalGet(key: *const fimo_tasks_meta.task_local.OpaqueKey) callconv(.c) ?*anyopaque {
    const task = Worker.currentTask() orelse @panic("not a task");
    return task.getLocal(key);
}

fn taskLocalClear(key: *const fimo_tasks_meta.task_local.OpaqueKey) callconv(.c) void {
    const task = Worker.currentTask() orelse @panic("not a task");
    task.clearLocal(key);
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
