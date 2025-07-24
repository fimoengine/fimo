const std = @import("std");
const atomic = std.atomic;
const ArrayListUnmanaged = std.ArrayListUnmanaged;
const builtin = @import("builtin");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;
const modules = fimo_std.modules;
const tracing = fimo_std.tracing;
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const symbols = fimo_tasks_meta.symbols;

const context = @import("context.zig");
const Pool = @import("Pool.zig");
const Runtime = @import("Runtime.zig");
const Worker = @import("Worker.zig");

pub const default_stack_size: usize = 8 * 1024 * 1024;
pub const default_worker_count: usize = 0; // One worker per cpu core.

pub const Instance = blk: {
    @setEvalBranchQuota(100000);
    break :blk modules.exports.Builder.init("fimo_tasks")
        .withDescription("Multi-threaded tasks runtime")
        .withLicense("MIT OR APACHE 2.0")
        .withParameter(.{
            .name = "default_stack_size",
            .member_name = "default_stack_size",
            .default_value = .{ .u32 = @intCast(default_stack_size) },
            .read_group = .dependency,
            .write_group = .dependency,
        })
        .withParameter(.{
            .name = "default_worker_count",
            .member_name = "default_worker_count",
            .default_value = .{ .u8 = @intCast(default_worker_count) },
            .read_group = .dependency,
            .write_group = .dependency,
        })
        .withExport(.{ .symbol = symbols.task_id }, &taskId)
        .withExport(.{ .symbol = symbols.worker_id }, &workerId)
        .withExport(.{ .symbol = symbols.worker_pool }, &workerPool)
        .withExport(.{ .symbol = symbols.worker_pool_by_id }, &workerPoolById)
        .withExport(.{ .symbol = symbols.query_worker_pools }, &queryWorkerPool)
        .withExport(.{ .symbol = symbols.create_worker_pool }, &createWorkerPool)
        .withExport(.{ .symbol = symbols.yield }, &yield)
        .withExport(.{ .symbol = symbols.abort }, &abort)
        .withExport(.{ .symbol = symbols.sleep }, &sleep)
        .withExport(.{ .symbol = symbols.task_local_set }, &taskLocalSet)
        .withExport(.{ .symbol = symbols.task_local_get }, &taskLocalGet)
        .withExport(.{ .symbol = symbols.task_local_clear }, &taskLocalClear)
        .withExport(.{ .symbol = symbols.futex_wait }, &futexWait)
        .withExport(.{ .symbol = symbols.futex_waitv }, &futexWaitv)
        .withExport(.{ .symbol = symbols.futex_wake }, &futexWake)
        .withExport(.{ .symbol = symbols.futex_requeue }, &futexRequeue)
        .withStateSync(State, State.init, State.deinit)
        .exportModule();
};

// Ensure that the module is exported.
comptime {
    _ = Instance;
}

extern "winmm" fn timeBeginPeriod(uPeriod: c_uint) callconv(.winapi) c_uint;
extern "winmm" fn timeEndPeriod(uPeriod: c_uint) callconv(.winapi) c_uint;

const State = struct {
    debug_allocator: switch (builtin.mode) {
        .Debug, .ReleaseSafe => std.heap.DebugAllocator(.{}),
        else => void,
    },
    runtime: Runtime,

    var global_state: State = undefined;
    var global_instance: atomic.Value(?*const Instance) = .init(null);

    fn init(octx: *const modules.OpaqueInstance, set: modules.LoadingSet) !*State {
        _ = set;
        const ctx: *const Instance = @ptrCast(@alignCast(octx));
        if (global_instance.cmpxchgStrong(null, ctx, .monotonic, .monotonic)) |_| {
            tracing.emitErrSimple("`fimo_tasks` is already initialized", .{}, @src());
            return error.AlreadyInitialized;
        }
        if (comptime builtin.target.os.tag == .windows) {
            if (timeBeginPeriod(1) != 0) {
                tracing.emitWarnSimple(
                    "`timeBeginPeriod` failed, defaulting to default timer resolution",
                    .{},
                    @src(),
                );
            }
        }

        const allocator = switch (builtin.mode) {
            .Debug, .ReleaseSafe => blk: {
                global_state.debug_allocator = .init;
                break :blk global_state.debug_allocator.allocator();
            },
            else => std.heap.smp_allocator,
        };
        global_state.runtime = .initInInstance(allocator, ctx);

        return &global_state;
    }

    fn deinit(octx: *const modules.OpaqueInstance, state: *State) void {
        const ctx: *const Instance = @ptrCast(@alignCast(octx));
        if (global_instance.cmpxchgStrong(ctx, null, .monotonic, .monotonic)) |_|
            @panic("already deinit");
        std.debug.assert(state == &global_state);

        global_state.runtime.deinit();
        switch (builtin.mode) {
            .Debug, .ReleaseSafe => {
                if (global_state.debug_allocator.deinit() == .leak) @panic("memory leak");
            },
            else => {},
        }
        global_state = undefined;
        if (comptime builtin.target.os.tag == .windows) _ = timeEndPeriod(1);
    }
};

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
    const runtime = &State.global_state.runtime;
    const p = runtime.pool_map.queryPoolById(id) orelse return false;
    pool.* = p.asMetaPool();
    return true;
}

fn queryWorkerPool(query: *fimo_tasks_meta.pool.Query) callconv(.c) AnyResult {
    const runtime = &State.global_state.runtime;
    const q = runtime.pool_map.queryAllPools(runtime.allocator) catch |err| {
        return AnyError.initError(err).intoResult();
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
            const allocator = State.global_state.runtime.allocator;
            allocator.free(nodes);
        }
    }.f;

    if (q.len == 0) {
        runtime.allocator.free(q);
        query.* = .{ .root = null, .deinit_fn = &deinitFn };
    } else {
        query.* = .{ .root = &q[0], .deinit_fn = &deinitFn };
    }
    return AnyResult.ok;
}

fn createWorkerPool(
    config: *const fimo_tasks_meta.pool.Config,
    pool: *fimo_tasks_meta.pool.Pool,
) callconv(.c) AnyResult {
    const runtime = &State.global_state.runtime;
    const allocator = runtime.allocator;

    if (config.next != null) {
        runtime.logErr(
            "the next key is reserved for future use, pool=`{s}`",
            .{config.label()},
            @src(),
        );
        return AnyError.initError(error.InvalidConfig).intoResult();
    }
    if (config.stacks_len == 0) {
        runtime.logErr("expected at least one stack, pool=`{s}`", .{config.label()}, @src());
        return AnyError.initError(error.InvalidConfig).intoResult();
    }
    if (config.default_stack_index >= config.stacks_len) {
        runtime.logErr(
            "default stack index out of bounds, pool=`{s}`, stacks=`{}`, default=`{}`",
            .{ config.label(), config.stacks_len, config.default_stack_index },
            @src(),
        );
        return AnyError.initError(error.InvalidConfig).intoResult();
    }
    for (config.stacks(), 0..) |stack, i| {
        if (stack.next != null) {
            runtime.logErr(
                "the next key is reserved for future use, pool=`{s}`, stack_index=`{}`",
                .{ config.label(), i },
                @src(),
            );
            return AnyError.initError(error.InvalidConfig).intoResult();
        }
        if (stack.preallocated_count > stack.max_allocated) {
            runtime.logErr(
                "number of preallocated stacks exceeds the specified maximum number of stacks," ++
                    " pool=`{s}`, stack_index=`{}`, preallocated=`{}`, max_allocated=`{}`",
                .{ config.label(), i, stack.preallocated_count, stack.max_allocated },
                @src(),
            );
            return AnyError.initError(error.InvalidConfig).intoResult();
        }
        if (stack.cold_count + stack.hot_count < stack.preallocated_count) {
            runtime.logErr(
                "number of preallocated stacks stacks exceeds the combined cold and hot stacks count," ++
                    " pool=`{s}`, stack_index=`{}`, preallocated=`{}`, cold=`{}`, hot=`{}`",
                .{ config.label(), i, stack.preallocated_count, stack.cold_count, stack.hot_count },
                @src(),
            );
            return AnyError.initError(error.InvalidConfig).intoResult();
        }
        if (stack.cold_count + stack.hot_count > stack.max_allocated) {
            runtime.logErr(
                "number of cold and hot stacks exceeds the specified maximum number of stacks," ++
                    " pool=`{s}`, stack_index=`{}`, cold=`{}`, hot=`{}`, max_allocated=`{}`",
                .{ config.label(), i, stack.cold_count, stack.hot_count, stack.max_allocated },
                @src(),
            );
            return AnyError.initError(error.InvalidConfig).intoResult();
        }
    }

    const min_stack_size = context.StackAllocator.minStackSize();
    const max_stack_size = context.StackAllocator.maxStackSize();

    const Pair = struct {
        size: usize,
        idx: usize,
        is_default: bool,
        skip: bool = false,

        fn cmp(ctx: void, a: @This(), b: @This()) bool {
            _ = ctx;
            return a.size < b.size;
        }
    };
    var stacks = ArrayListUnmanaged(Pair)
        .initCapacity(allocator, config.stacks_len) catch |err| {
        return AnyError.initError(err).intoResult();
    };
    defer stacks.deinit(allocator);

    for (config.stacks(), 0..) |stack, i| {
        const size = if (stack.size != .default)
            @max(@min(@intFromEnum(stack.size), max_stack_size), min_stack_size)
        else
            runtime.getDefaultStackSize();
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
    var stack_cfg = ArrayListUnmanaged(Pool.InitOptions.StackOptions)
        .initCapacity(allocator, num_stacks) catch |err| {
        return AnyError.initError(err).intoResult();
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
        .runtime = runtime,
        .allocator = allocator,
        .label = config.label(),
        .stacks = stack_cfg.items,
        .default_stack = default_stack_idx,
        .worker_count = if (config.worker_count == 0)
            runtime.getDefaultWorkerCount()
        else
            config.worker_count,
        .is_public = config.is_queryable,
    };

    const p = runtime.pool_map.spawnPool(runtime.allocator, options) catch |err| {
        return AnyError.initError(err).intoResult();
    };
    pool.* = p.asMetaPool();
    return AnyResult.ok;
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
    State.global_state.runtime.futex.wait(
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
    wake_index.* = State.global_state.runtime.futex.waitv(
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
    return State.global_state.runtime.futex.wakeFilter(key, max_waiters, filter);
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
    result.* = State.global_state.runtime.futex.requeueFilter(
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
