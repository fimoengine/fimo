const std = @import("std");
const atomic = std.atomic;
const ArrayList = std.ArrayList;
const AutoArrayHashMap = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const Status = ctx.Status;
const tracing = fimo_std.tracing;
const spanTrace = tracing.spanTrace;
const spanTraceNamed = tracing.spanTraceNamed;
const modules = fimo_std.modules;
const memory = fimo_std.memory;
const Arena = memory.Arena;
const fimo_tasks = @import("fimo_tasks_meta");
const Task = fimo_tasks.Task;
const CmdBufCmd = fimo_tasks.CmdBufCmd;
const CmdBuf = fimo_tasks.CmdBuf;
const Executor = fimo_tasks.Executor;
const sync = fimo_tasks.sync;
const Mutex = sync.Mutex;
const RwLock = sync.RwLock;
const Fence = sync.Fence;
const TimelineSemaphore = sync.TimelineSemaphore;
const fimo_worlds = @import("fimo_worlds_meta");
const symbols = fimo_worlds.symbols;

world_count: atomic.Value(usize) = .init(0),

pub const Module = modules.Module(@This());
pub const fimo_module_bundle = fimo_std.modules.ModuleBundle(.{
    Module,
});

const world_arena_reserve = 8 * 1024 * 1024; // 8 MiB
const scheduler_arena_reserve = 256 * 1024 * 1024; // 256 MiB

comptime {
    _ = fimo_module_bundle;
}

pub const fimo_module = .{
    .name = .fimo_worlds,
    .author = "fimo",
    .description = "Multi-threaded state processing",
    .license = "MIT OR APACHE 2.0",
};

pub const fimo_imports = .{fimo_tasks.symbols.all_symbols};

pub const fimo_exports = .{
    .{ .symbol = symbols.world_init, .value = &worldInit },
    .{ .symbol = symbols.world_deinit, .value = &worldDeinit },
    .{ .symbol = symbols.world_add_res, .value = &worldAddRes },
    .{ .symbol = symbols.resource_deinit, .value = &resDeinit },
    .{ .symbol = symbols.resource_lock_read, .value = &resLockRead },
    .{ .symbol = symbols.resource_unlock_read, .value = &resUnlockRead },
    .{ .symbol = symbols.resource_lock_write, .value = &resLockWrite },
    .{ .symbol = symbols.resource_unlock_write, .value = &resUnlockWrite },
    .{ .symbol = symbols.world_add_scheduler, .value = &worldAddScheduler },
    .{ .symbol = symbols.scheduler_deinit, .value = &schedulerDeinit },
    .{ .symbol = symbols.scheduler_add_sys, .value = &schedulerAddSys },
    .{ .symbol = symbols.sys_deinit, .value = &sysDeinit },
    .{ .symbol = symbols.scheduler_run, .value = &schedulerRun },
    .{ .symbol = symbols.scheduler_schedule, .value = &schedulerSchedule },
    .{ .symbol = symbols.scheduler_flush, .value = &schedulerFlush },
};

pub const fimo_events = .{
    .init = assertEmpty,
    .deinit = assertEmpty,
};

fn assertEmpty(self: *@This()) void {
    std.debug.assert(self.world_count.load(.monotonic) == 0);
}

pub const WorldDesc = struct {
    label: []const u8 = "",
};

pub const World = struct {
    arena: Arena,
    label: []u8,
    mutex: Mutex = .{},
    res_count: usize = 0,
    res_free_list: ?*Res = null,
    sched_count: usize = 0,
    sched_free_list: ?*Scheduler = null,

    pub fn init(desc: WorldDesc) error{OutOfMemory}!*World {
        var arena = try Arena.init(.{ .reserve = world_arena_reserve });
        errdefer arena.deinit();

        const label = try arena.allocator().dupe(u8, desc.label);
        const world = try arena.allocator().create(World);
        world.* = .{
            .arena = arena,
            .label = label,
        };
        _ = Module.state().world_count.fetchAdd(1, .monotonic);

        return world;
    }

    pub fn deinit(self: *World) void {
        self.mutex.lock();
        std.debug.assert(self.res_count == 0);
        std.debug.assert(self.sched_count == 0);
        while (self.sched_free_list) |sched| {
            self.sched_free_list = sched.next;
            sched.arena.deinit();
            sched.cmd_arena.deinit();
        }

        var arena = self.arena;
        arena.deinit();
        _ = Module.state().world_count.fetchSub(1, .monotonic);
    }

    pub fn addRes(self: *World, desc: ResDesc) *Res {
        self.mutex.lock();
        defer self.mutex.unlock();

        self.res_count += 1;
        const res = if (self.res_free_list) |v| blk: {
            self.res_free_list = v.data.next;
            break :blk v;
        } else self.arena.allocator().create(Res) catch @panic("oom");
        res.* = .{
            .world = self,
            .label = undefined,
            .label_len = undefined,
            .data = undefined,
        };
        res.data = .{ .value = desc.value };
        res.label_len = @min(res.label.len, desc.label.len);
        @memcpy(res.label[0..res.label_len], desc.label[0..res.label_len]);
        return res;
    }

    pub fn addScheduler(self: *World, desc: SchedulerDesc) *Scheduler {
        self.mutex.lock();
        defer self.mutex.unlock();

        self.sched_count += 1;
        const sched = if (self.sched_free_list) |v| blk: {
            self.sched_free_list = v.next;
            var arena = v.arena;
            arena.pos.store(0, .monotonic);
            var cmd_arena = v.cmd_arena;
            cmd_arena.pos.store(0, .monotonic);
            v.* = .{
                .world = self,
                .executor = desc.executor,
                .arena = arena,
                .cmd_arena = cmd_arena,
                .label = undefined,
            };
            break :blk v;
        } else blk: {
            const v = self.arena.allocator().create(Scheduler) catch @panic("oom");
            v.* = .{
                .world = self,
                .executor = desc.executor,
                .arena = Arena.init(.{ .reserve = scheduler_arena_reserve }) catch @panic("oom"),
                .cmd_arena = Arena.init(.{ .reserve = scheduler_arena_reserve }) catch @panic("oom"),
                .label = undefined,
            };
            break :blk v;
        };
        sched.label = sched.arena.allocator().dupe(u8, desc.label) catch @panic("oom");
        return sched;
    }
};

pub const ResDesc = struct {
    label: []const u8 = "",
    value: *anyopaque,
};

pub const Res = struct {
    world: *World,
    lock: RwLock = .{},
    label: [128]u8,
    label_len: usize,
    data: union {
        value: *anyopaque,
        next: ?*Res,
    },

    pub fn deinit(self: *Res) void {
        self.world.mutex.lock();
        defer self.world.mutex.unlock();
        self.world.res_count -= 1;
        self.data = .{ .next = self.world.res_free_list };
        self.world.res_free_list = self;
    }

    pub fn lockRead(self: *Res) *anyopaque {
        self.lock.lockRead();
        return self.data.value;
    }

    pub fn unlockRead(self: *Res) void {
        self.lock.unlockRead();
    }

    pub fn lockWrite(self: *Res) *anyopaque {
        self.lock.lockWrite();
        return self.data.value;
    }

    pub fn unlockWrite(self: *Res) void {
        self.lock.unlockWrite();
    }
};

fn Block(T: type, comptime n: usize) type {
    return struct {
        pub const capacity: usize = n;

        entries: [8]?*T = @splat(null),
        next: ?*@This() = null,

        fn count(self: *@This()) usize {
            var num: usize = 0;
            var curr: ?*@This() = self;
            while (curr) |current| : (curr = current.next) {
                for (current.entries) |entry| {
                    if (entry == null) break;
                    num += 1;
                }
            }

            return num;
        }

        fn append(self: *@This(), entry: *T) bool {
            var curr: ?*@This() = self;
            while (curr) |current| : (curr = current.next) {
                for (&current.entries) |*dst| {
                    if (dst.* != null) continue;
                    dst.* = entry;
                    return true;
                }
            }

            return false;
        }

        fn appendBlock(self: *@This(), block: *@This()) void {
            var curr: ?*@This() = self;
            while (curr) |current| : (curr = current.next) {
                if (current.next != null) continue;
                std.debug.assert(current.entries[n - 1] != null);
                current.next = block;
                return;
            }

            unreachable;
        }

        fn remove(self: *@This(), entry: *T) void {
            var curr: ?*@This() = self;
            while (curr) |current| {
                for (current.entries, 0..) |s, i| {
                    if (s != entry) continue;
                    current.entries[i] = current.entries[n - 1];
                    current.entries[n - 1] = null;

                    // NOTE(gabriel): Move the hole to the last slot.
                    var next = current.next;
                    while (next) |block| {
                        current.entries[n - 1] = block.entries[n - 1];
                        block.entries[n - 1] = null;
                        curr = block;
                        next = block.next;
                    }

                    // NOTE(gabriel): We might now have introduced a hole in the middle.
                    std.mem.rotate(?*T, current.entries[i..], 1);
                    return;
                }

                curr = current.next;
            }
        }
    };
}

const ResBlock = Block(Res, 8);
const CondBlock = Block(Cond, 8);
const SysBlock = Block(Sys, 8);

pub const CondDesc = struct {
    read: []const *Res,
    write: []const *Res,
    data: ?*anyopaque,
    cond: fimo_worlds.Sys.Cond,
};

const Cond = struct {
    scheduler: *Scheduler,
    read: ?*ResBlock,
    write: ?*ResBlock,
    data: ?*anyopaque,
    cond: fimo_worlds.Sys.Cond,
    next: ?*Cond = null,
};

pub const SysDesc = struct {
    before: []const *Sys,
    after: []const *Sys,
    conditions: []const CondDesc,
    data: union(enum) {
        sys: struct {
            label: []const u8,
            read: []const *Res,
            write: []const *Res,
            data: ?*anyopaque,
            system: fimo_worlds.Sys.Run,
        },
        set: struct {
            serialize: bool,
            sub_desc: []const SysDesc,
        },
    },

    fn initC(c: *const fimo_worlds.Sys.Desc, arena: *Arena) !SysDesc {
        const allocator = arena.allocator();
        const std_allocator = allocator.adaptIntoStdAllocator();

        var desc: SysDesc = undefined;
        var seen: AutoArrayHashMap(*const fimo_worlds.Sys.Desc, void) = .empty;
        var stack: ArrayList(struct { *const fimo_worlds.Sys.Desc, *SysDesc }) = .empty;
        try stack.append(std_allocator, .{ c, &desc });
        while (stack.pop()) |next| {
            const next_c, const next_desc = next;
            if (seen.contains(next_c)) return error.DuplicateDesc;
            try seen.put(std_allocator, next_c, {});

            next_desc.* = .{
                .before = @ptrCast(@alignCast(next_c.before.intoSliceOrEmpty())),
                .after = @ptrCast(@alignCast(next_c.after.intoSliceOrEmpty())),
                .conditions = undefined,
                .data = undefined,
            };
            next_desc.conditions = if (next_c.conditions.intoSlice()) |conds| blk: {
                const cond_desc = try allocator.alloc(CondDesc, conds.len);
                for (cond_desc, conds) |*dst, src| {
                    dst.* = .{
                        .read = @ptrCast(@alignCast(src.read.intoSliceOrEmpty())),
                        .write = @ptrCast(@alignCast(src.write.intoSliceOrEmpty())),
                        .data = src.data,
                        .cond = src.condition,
                    };
                }
                break :blk cond_desc;
            } else &.{};

            switch (next_c.tag) {
                .sys => {
                    const next_sys = next_c.data.sys;
                    next_desc.data = .{
                        .sys = .{
                            .label = next_sys.label.intoSliceOrEmpty(),
                            .read = @ptrCast(@alignCast(next_sys.read.intoSliceOrEmpty())),
                            .write = @ptrCast(@alignCast(next_sys.write.intoSliceOrEmpty())),
                            .data = next_sys.data,
                            .system = next_sys.system,
                        },
                    };
                },
                .set => {
                    const next_set = next_c.data.set;
                    const sub_desc = try allocator.alloc(SysDesc, next_set.sub_desc.len);
                    next_desc.data = .{
                        .set = .{
                            .serialize = next_set.serialize,
                            .sub_desc = sub_desc,
                        },
                    };

                    for (sub_desc, next_set.sub_desc.intoSliceOrEmpty()) |*dst, *src| {
                        try stack.append(std_allocator, .{ src, dst });
                    }
                },
                else => return error.UnknownTag,
            }
        }

        return desc;
    }
};

pub const Sys = struct {
    scheduler: *Scheduler,
    owner: *Sys,
    label: [128]u8,
    label_len: usize,
    before: ?*SysBlock,
    // NOTE(gabriel):
    //
    // The before list is sufficient to order the graph.
    // It contains both the order depedencies specified by the current
    // system and those that are dependent on the current one.
    // The second list is required for the removal of the system.
    after_subset: ?*SysBlock,
    parent_conditions: ?*CondBlock,
    conditions: ?*CondBlock,
    read: ?*ResBlock,
    write: ?*ResBlock,
    data: ?*anyopaque,
    run: fimo_worlds.Sys.Run,
    deinit_fence: ?*Fence,
    next: ?*Sys = null,

    pub fn deinit(self: *Sys, fence: ?*Fence) void {
        const span = spanTrace(@src());
        defer span.exit();

        var f: Fence = .{};
        defer if (fence == null) f.wait();
        const fence_ptr = fence orelse &f;
        {
            std.debug.assert(self.owner == self);
            const scheduler = self.scheduler;
            scheduler.mutex.lock();
            defer scheduler.mutex.unlock();
            scheduler.dirty = true;

            // NOTE(gabriel): Remove all entries belonging to the same set from the list.
            var head: ?*Sys = null;
            var link: *?*Sys = &scheduler.systems;
            var curr: ?*Sys = scheduler.systems;
            while (curr) |current| {
                if (current.owner == self) {
                    link.* = current.next;
                    curr = current.next;
                    current.next = head;
                    head = current;
                } else {
                    link = &current.next;
                    curr = current.next;
                }
            }

            // NOTE(gabriel): If the system is running we must wait for it's completion.
            if (!scheduler.semaphore.isSignaled(scheduler.next_generation.load(.monotonic) - 1)) {
                self.deinit_fence = fence_ptr;
                while (head) |sys| {
                    head = sys.next;
                    sys.next = self.scheduler.cleanup_systems;
                    self.scheduler.cleanup_systems = sys;
                }
                return;
            }

            while (head) |sys| {
                head = sys.next;
                sys.cleanup();
            }
            fence_ptr.signal();
        }
    }

    fn cleanup(self: *Sys) void {
        const scheduler = self.scheduler;
        if (self.deinit_fence) |f| f.signal();

        while (self.before) |block| {
            self.before = block.next;
            scheduler.deallocSysBlock(block);
        }
        while (self.after_subset) |block| {
            self.after_subset = block.next;
            for (block.entries) |opt_sys| {
                const sys = opt_sys orelse break;
                if (sys.before) |before| before.remove(self);
            }
            scheduler.deallocSysBlock(block);
        }
        while (self.parent_conditions) |block| {
            self.parent_conditions = block.next;
            scheduler.deallocCondBlock(block);
        }
        while (self.conditions) |block| {
            self.conditions = block.next;
            for (block.entries) |opt_cond| {
                const cond = opt_cond orelse break;
                while (cond.read) |block2| {
                    cond.read = block2.next;
                    scheduler.deallocResBlock(block2);
                }
                while (cond.write) |block2| {
                    cond.write = block2.next;
                    scheduler.deallocResBlock(block2);
                }
            }
            scheduler.deallocCondBlock(block);
        }
        while (self.read) |block| {
            self.read = block.next;
            scheduler.deallocResBlock(block);
        }
        while (self.write) |block| {
            self.write = block.next;
            scheduler.deallocResBlock(block);
        }
        scheduler.deallocSys(self);
    }
};

const ResInfo = struct {
    access: enum { read, write },
    res: *Res,

    write: bool,
    ref_by: ArrayList(*Node) = .empty,

    fn lock(self: ResInfo) *anyopaque {
        return switch (self.access) {
            .read => self.res.lockRead(),
            .write => self.res.lockWrite(),
        };
    }

    fn unlock(self: ResInfo) void {
        switch (self.access) {
            .read => self.res.unlockRead(),
            .write => self.res.unlockWrite(),
        }
    }
};

const CondTask = struct {
    read: []*anyopaque,
    write: []*anyopaque,
    cond: *Cond,
    arena: *Arena,
    task: Task,
    dependents: ArrayList(*CmdBufCmd),

    fn run(task: *Task, idx: usize) callconv(.c) void {
        _ = idx;
        const self: *CondTask = @alignCast(@fieldParentPtr("task", task));

        // NOTE(gabriel): Fill the read and write slices.
        var i: usize = 0;
        var curr = self.cond.read;
        blk: while (curr) |current| {
            for (current.entries) |opt_res| {
                const res = opt_res orelse break :blk;
                self.read[i] = res.data.value;
                i += 1;
            }
            curr = current.next;
        }
        i = 0;
        curr = self.cond.write;
        blk: while (curr) |current| {
            for (current.entries) |opt_res| {
                const res = opt_res orelse break :blk;
                self.write[i] = res.data.value;
                i += 1;
            }
            curr = current.next;
        }

        const args: fimo_worlds.Sys.Args = .{
            .world = @ptrCast(self.cond.scheduler.world),
            .sched = @ptrCast(self.cond.scheduler),
            .arena = self.arena,
            .read = .fromSlice(self.read),
            .write = .fromSlice(self.write),
        };
        if (!self.cond.cond(self.cond.data, &args)) {
            for (self.dependents.items) |dep|
                @atomicStore(@TypeOf(dep.tag), &dep.tag, .noop, .release);
        }
    }
};

const SysTask = struct {
    read: []*anyopaque,
    write: []*anyopaque,
    sys: *Sys,
    arena: *Arena,
    task: Task,
    fence: ?*Fence,

    fn run(task: *Task, idx: usize) callconv(.c) void {
        _ = idx;
        const self: *SysTask = @alignCast(@fieldParentPtr("task", task));

        // NOTE(gabriel): Fill the read and write slices.
        var i: usize = 0;
        var curr = self.sys.read;
        blk: while (curr) |current| {
            for (current.entries) |opt_res| {
                const res = opt_res orelse break :blk;
                self.read[i] = res.data.value;
                i += 1;
            }
            curr = current.next;
        }
        i = 0;
        curr = self.sys.write;
        blk: while (curr) |current| {
            for (current.entries) |opt_res| {
                const res = opt_res orelse break :blk;
                self.write[i] = res.data.value;
                i += 1;
            }
            curr = current.next;
        }

        const args: fimo_worlds.Sys.Args = .{
            .world = @ptrCast(self.sys.scheduler.world),
            .sched = @ptrCast(self.sys.scheduler),
            .arena = self.arena,
            .read = .fromSlice(self.read),
            .write = .fromSlice(self.write),
        };
        self.fence = self.sys.run(self.sys.data, &args);
    }
};

const Node = struct {
    depth: ?usize,
    read: ArrayList(*Res),
    write: ArrayList(*Res),
    deps: ArrayList(usize),
    task: union(enum) {
        cond: CondTask,
        sys: SysTask,
    },

    fn sortByDepth(context: void, a: *@This(), b: *@This()) bool {
        _ = context;
        return a.depth.? < b.depth.?;
    }

    fn computeDepth(node: *@This(), graph: *AutoArrayHashMap(usize, *@This())) usize {
        if (node.depth) |gen| return gen;
        var generation: usize = 0;
        for (node.deps.items) |dep_id| {
            const dep = graph.get(dep_id).?;
            const dep_gen = dep.computeDepth(graph);
            generation = @max(generation, dep_gen + 1);
        }
        node.depth = generation;
        return generation;
    }
};

const ScheduleTask = struct {
    sched: *Scheduler,
    arena: *Arena,
    generation: u64,
    start: ?*Fence,
    completion: ?*Fence,
    task: Task,
    cmd: CmdBufCmd,
    cmd_buf: CmdBuf,
    next: ?*ScheduleTask = null,

    fn run(task: *Task, idx: usize) callconv(.c) void {
        _ = idx;
        const self: *ScheduleTask = @alignCast(@fieldParentPtr("task", task));
        if (self.start) |f| f.wait();
        self.sched.runInner(self.arena, self.generation);
    }

    fn deinit(cmd_buf: *CmdBuf) callconv(.c) void {
        const self: *ScheduleTask = @alignCast(@fieldParentPtr("cmd_buf", cmd_buf));
        const sched = self.sched;
        const generation = self.generation;
        const completion = self.completion;
        {
            sched.mutex.lock();
            defer sched.mutex.unlock();
            self.* = undefined;
            self.next = sched.jobs;
            sched.jobs = self;
        }

        // NOTE(gabriel): The semaphore must be signaled here because it
        // is allocated with scheduler arena. The scheduler may be destroyed
        // immediately after the semaphore is signaled.
        if (completion) |f| f.signal();
        sched.semaphore.signal(generation);
    }
};

pub const SchedulerDesc = struct {
    label: []const u8 = "",
    executor: ?*Executor,
};

pub const Scheduler = struct {
    world: *World,
    executor: ?*Executor,
    arena: Arena,
    cmd_arena: Arena,
    label: []u8,
    semaphore: TimelineSemaphore = .init(0),
    next_generation: atomic.Value(u64) = .init(1),
    mutex: Mutex = .{},
    dirty: bool = true,
    jobs: ?*ScheduleTask = null,
    systems: ?*Sys = null,
    free_systems: ?*Sys = null,
    cleanup_systems: ?*Sys = null,
    free_conditions: ?*Cond = null,
    free_res_blocks: ?*ResBlock = null,
    free_cond_blocks: ?*CondBlock = null,
    free_sys_blocks: ?*SysBlock = null,
    cmds: []CmdBufCmd = undefined,
    res_infos: []ResInfo = undefined,
    nodes: []*Node = undefined,
    next: ?*Scheduler = null,

    pub fn deinit(self: *Scheduler) void {
        self.world.mutex.lock();
        defer self.world.mutex.unlock();

        self.mutex.lock();
        defer self.mutex.unlock();
        std.debug.assert(self.semaphore.isSignaled(self.next_generation.load(.monotonic) - 1));
        std.debug.assert(self.systems == null);
        std.debug.assert(self.cleanup_systems == null);
        self.world.sched_count -= 1;

        self.next = self.world.sched_free_list;
        self.world.sched_free_list = self;
    }

    pub fn addSys(self: *Scheduler, desc: SysDesc) !*Sys {
        const scratch = ctx.getScratchArena(null);
        const pos = scratch.pos.load(.monotonic);
        defer scratch.pos.store(pos, .monotonic);
        const allocator = scratch.allocator();
        const std_allocator = allocator.adaptIntoStdAllocator();

        self.mutex.lock();
        defer self.mutex.unlock();

        const DescInfo = struct {
            desc: *const SysDesc,
            after: ?*const SysDesc,
            parent: ?*const SysDesc,
            before_idx: usize,
            after_idx: usize,
            cond_idx: usize,
            pop: bool,
        };

        var desc_map: AutoArrayHashMap(*const SysDesc, ArrayList(*Sys)) = .empty;
        try desc_map.put(std_allocator, &desc, .empty);

        var remaining: ArrayList(DescInfo) = .empty;
        try remaining.append(std_allocator, .{
            .desc = &desc,
            .after = null,
            .parent = null,
            .before_idx = 0,
            .after_idx = 0,
            .cond_idx = 0,
            .pop = true,
        });

        var before_list: AutoArrayHashMap(*Sys, void) = .empty;
        var after_list: AutoArrayHashMap(*Sys, void) = .empty;

        var before_stack: ArrayList(*Sys) = .empty;
        var after_stack: ArrayList(*Sys) = .empty;
        var cond_stack: ArrayList(*Cond) = .empty;
        var cond_end_idx: usize = 0;
        errdefer for (0..cond_stack.items.len - cond_end_idx) |_| {
            const cond = cond_stack.pop().?;
            self.deallocCond(cond);
        };

        var head: ?*Sys = null;
        var tail: ?*Sys = null;
        errdefer while (head) |sys| {
            head = sys.next;
            sys.cleanup();
        };

        // NOTE(gabriel): Flatten the descriptor into a list of systems.
        while (remaining.pop()) |curr_info| {
            const curr = curr_info.desc;
            for (curr.before) |value| if (value.scheduler != self) return error.SchedulerMismatch;
            for (curr.after) |value| if (value.scheduler != self) return error.SchedulerMismatch;

            for (curr.before) |value| try before_list.put(std_allocator, value, {});
            for (curr.after) |value| try after_list.put(std_allocator, value, {});

            try before_stack.appendSlice(std_allocator, curr.before);
            try after_stack.appendSlice(std_allocator, curr.after);
            for (curr.conditions) |cond_desc| {
                const cond = self.allocCond();
                try cond_stack.append(std_allocator, cond);
                cond.* = .{
                    .scheduler = self,
                    .read = self.allocFillResBlock(cond_desc.read),
                    .write = self.allocFillResBlock(cond_desc.write),
                    .data = cond_desc.data,
                    .cond = cond_desc.cond,
                };
            }

            switch (curr.data) {
                .sys => |sys_desc| {
                    const sys = self.allocSys();
                    if (curr_info.parent) |parent|
                        try desc_map.getPtr(parent).?.append(std_allocator, sys);
                    if (head == null) head = sys;
                    const label_len = @min(sys.label.len, sys_desc.label.len);
                    sys.* = .{
                        .scheduler = self,
                        .owner = head.?,
                        .label = undefined,
                        .label_len = label_len,
                        .before = self.allocFillSysBlock(before_stack.items),
                        .after_subset = undefined,
                        .parent_conditions = self.allocFillCondBlock(cond_stack.items[0 .. cond_stack.items.len - curr.conditions.len]),
                        .conditions = self.allocFillCondBlock(cond_stack.items[cond_stack.items.len - curr.conditions.len ..]),
                        .read = self.allocFillResBlock(sys_desc.read),
                        .write = self.allocFillResBlock(sys_desc.write),
                        .data = sys_desc.data,
                        .run = sys_desc.system,
                        .deinit_fence = null,
                    };
                    @memcpy(sys.label[0..label_len], sys_desc.label[0..label_len]);

                    const after: []*Sys = if (curr_info.after) |after_desc| blk: {
                        const after_sys = desc_map.get(after_desc).?;
                        break :blk try std.mem.concat(std_allocator, *Sys, &.{ after_stack.items, after_sys.items });
                    } else after_stack.items;
                    sys.after_subset = self.allocFillSysBlock(after);

                    if (tail) |t| t.next = sys;
                    tail = sys;

                    cond_end_idx = curr_info.cond_idx;
                    while (before_stack.items.len > curr_info.before_idx) _ = before_stack.pop();
                    while (after_stack.items.len > curr_info.after_idx) _ = after_stack.pop();
                    while (cond_stack.items.len > curr_info.cond_idx) _ = cond_stack.pop();
                },
                .set => |set_desc| {
                    if (set_desc.sub_desc.len == 0) return error.EmptySubSet;
                    var last: ?*const SysDesc = null;
                    var it = std.mem.reverseIterator(set_desc.sub_desc);
                    while (it.nextPtr()) |sub_desc| : (last = sub_desc) {
                        if (desc_map.get(sub_desc) != null) return error.DuplicateDesc;
                        try desc_map.put(std_allocator, sub_desc, .empty);
                        try remaining.append(std_allocator, .{
                            .desc = sub_desc,
                            .after = if (set_desc.serialize) last else null,
                            .parent = curr,
                            .before_idx = if (curr_info.pop) curr_info.before_idx else before_stack.items.len - curr.before.len,
                            .after_idx = if (curr_info.pop) curr_info.after_idx else after_stack.items.len - curr.after.len,
                            .cond_idx = if (curr_info.pop) curr_info.cond_idx else cond_stack.items.len - curr.conditions.len,
                            .pop = last == null,
                        });
                    }
                },
            }
        }

        // NOTE(gabriel): Before we insert the system list we must ensure that
        // it does not introduce any cycles in our dependency graph. We can check
        // this by computing the reachable subgraph from all systems in the `before`
        // list and checking if it contains some system from the `after` list.
        var reachable: AutoArrayHashMap(*Sys, void) = .empty;
        var remaining_sys: ArrayList(*Sys) = .empty;
        for (before_list.keys()) |value| {
            try reachable.put(std_allocator, value, {});
            try remaining_sys.append(std_allocator, value);
            if (after_list.contains(value)) return error.CyclicDependency;
        }

        while (remaining_sys.pop()) |sys| {
            var curr: ?*SysBlock = sys.before;
            while (curr) |block| : (curr = block.next) {
                for (block.entries) |opt_entry| {
                    const entry = opt_entry orelse break;
                    if (reachable.contains(entry)) continue;
                    if (after_list.contains(entry)) return error.CyclicDependency;
                    try reachable.put(std_allocator, entry, {});
                    try remaining_sys.append(std_allocator, entry);
                }
            }
        }

        var curr = head;
        while (curr) |current| : (curr = current.next) {
            var curr_block = current.after_subset;
            while (curr_block) |block| : (curr_block = block.next) {
                for (&block.entries) |opt_entry| {
                    const entry = opt_entry orelse break;
                    if (entry.before) |before| {
                        if (!before.append(current)) {
                            const new_block = self.allocSysBlock();
                            new_block.* = .{};
                            new_block.entries[0] = current;
                            before.appendBlock(new_block);
                        }
                    } else {
                        const new_block = self.allocSysBlock();
                        new_block.* = .{};
                        new_block.entries[0] = current;
                        entry.before = new_block;
                    }
                }
            }
        }

        tail.?.next = self.systems;
        self.systems = head;
        return head.?;
    }

    fn allocSys(self: *Scheduler) *Sys {
        return if (self.free_systems) |value|
            value
        else
            self.arena.allocator().create(Sys) catch @panic("oom");
    }

    fn deallocSys(self: *Scheduler, value: *Sys) void {
        value.* = undefined;
        value.next = self.free_systems;
        self.free_systems = value;
    }

    fn allocCond(self: *Scheduler) *Cond {
        return if (self.free_conditions) |value|
            value
        else
            self.arena.allocator().create(Cond) catch @panic("oom");
    }

    fn deallocCond(self: *Scheduler, value: *Cond) void {
        value.* = undefined;
        value.next = self.free_conditions;
        self.free_conditions = value;
    }

    fn allocFillResBlock(self: *Scheduler, res: []const *Res) ?*ResBlock {
        var head: ?*ResBlock = null;
        var tail: ?*ResBlock = null;
        var iter = std.mem.window(*Res, res, ResBlock.capacity, ResBlock.capacity);
        while (iter.next()) |value| {
            if (value.len == 0) break;
            const block = self.allocResBlock();
            block.* = .{};
            for (value, block.entries[0..value.len]) |bef, *entry| entry.* = bef;
            if (tail) |t| t.next = block else head = block;
            tail = block;
        }

        return head;
    }

    fn allocResBlock(self: *Scheduler) *ResBlock {
        return if (self.free_res_blocks) |value|
            value
        else
            self.arena.allocator().create(ResBlock) catch @panic("oom");
    }

    fn deallocResBlock(self: *Scheduler, value: *ResBlock) void {
        value.* = undefined;
        value.next = self.free_res_blocks;
        self.free_res_blocks = value;
    }

    fn allocFillCondBlock(self: *Scheduler, conds: []const *Cond) ?*CondBlock {
        var head: ?*CondBlock = null;
        var tail: ?*CondBlock = null;
        var iter = std.mem.window(*Cond, conds, CondBlock.capacity, CondBlock.capacity);
        while (iter.next()) |value| {
            if (value.len == 0) break;
            const block = self.allocCondBlock();
            block.* = .{};
            for (value, block.entries[0..value.len]) |bef, *entry| entry.* = bef;
            if (tail) |t| t.next = block else head = block;
            tail = block;
        }

        return head;
    }

    fn allocCondBlock(self: *Scheduler) *CondBlock {
        return if (self.free_cond_blocks) |value|
            value
        else
            self.arena.allocator().create(CondBlock) catch @panic("oom");
    }

    fn deallocCondBlock(self: *Scheduler, value: *CondBlock) void {
        value.* = undefined;
        value.next = self.free_cond_blocks;
        self.free_cond_blocks = value;
    }

    fn allocFillSysBlock(self: *Scheduler, sys: []const *Sys) ?*SysBlock {
        var head: ?*SysBlock = null;
        var tail: ?*SysBlock = null;
        var iter = std.mem.window(*Sys, sys, SysBlock.capacity, SysBlock.capacity);
        while (iter.next()) |value| {
            if (value.len == 0) break;
            const block = self.allocSysBlock();
            block.* = .{};
            for (value, block.entries[0..value.len]) |bef, *entry| entry.* = bef;
            if (tail) |t| t.next = block else head = block;
            tail = block;
        }

        return head;
    }

    fn allocSysBlock(self: *Scheduler) *SysBlock {
        return if (self.free_sys_blocks) |value|
            value
        else
            self.arena.allocator().create(SysBlock) catch @panic("oom");
    }

    fn deallocSysBlock(self: *Scheduler, value: *SysBlock) void {
        value.* = undefined;
        value.next = self.free_sys_blocks;
        self.free_sys_blocks = value;
    }

    fn rebuildCmds(self: *Scheduler) void {
        const span = spanTrace(@src());
        defer span.exit();
        self.mutex.lock();
        defer self.mutex.unlock();
        if (!self.dirty) return;
        self.cmd_arena.pos.store(0, .monotonic);

        const allocator = self.cmd_arena.allocator();
        const std_allocator = allocator.adaptIntoStdAllocator();

        var num_sys: usize = 0;
        var num_conds: usize = 0;
        var graph: AutoArrayHashMap(usize, *Node) = .empty;

        // NOTE(gabriel): Populate the nodes.
        //
        // The dependencies of a system can be modelled as a DAG,
        // where a system depends on all of its conditions and the
        // systems ordered before it. Additionally we impose the
        // constraint that the conditions of a system also depend
        // on all systems constrained to run before the system they
        // guard. This ensures that the condition can see the latest
        // state of the system before running.
        var head_sys = self.systems;
        while (head_sys) |sys| : (head_sys = sys.next) {
            num_sys += 1;
            const node = allocator.create(Node) catch @panic("oom");
            graph.put(std_allocator, @intFromPtr(sys), node) catch @panic("oom");
            node.* = .{
                .depth = null,
                .read = .empty,
                .write = .empty,
                .deps = .empty,
                .task = .{
                    .sys = .{
                        .read = undefined,
                        .write = undefined,
                        .sys = sys,
                        .arena = undefined,
                        .task = .{ .label = .init(sys.label[0..sys.label_len]), .run = &SysTask.run },
                        .fence = null,
                    },
                },
            };

            {
                var curr = sys.read;
                while (curr) |block| : (curr = block.next) {
                    for (block.entries) |opt_res| {
                        const res = opt_res orelse break;
                        node.read.append(std_allocator, res) catch @panic("oom");
                    }
                }
            }
            {
                var curr = sys.write;
                while (curr) |block| : (curr = block.next) {
                    for (block.entries) |opt_res| {
                        const res = opt_res orelse break;
                        node.write.append(std_allocator, res) catch @panic("oom");
                    }
                }
            }
            node.task.sys.read = allocator.alloc(*anyopaque, node.read.items.len) catch @panic("oom");
            node.task.sys.write = allocator.alloc(*anyopaque, node.write.items.len) catch @panic("oom");

            if (sys.conditions != null) {
                var curr = sys.conditions;
                while (curr) |block| : (curr = block.next) {
                    for (block.entries) |opt_dep| {
                        const dep = opt_dep orelse break;
                        num_conds += 1;
                        const dep_node = allocator.create(Node) catch @panic("oom");
                        graph.put(std_allocator, @intFromPtr(dep), dep_node) catch @panic("oom");
                        node.deps.append(std_allocator, @intFromPtr(dep)) catch @panic("oom");
                        dep_node.* = .{
                            .depth = null,
                            .read = .empty,
                            .write = .empty,
                            .deps = .empty,
                            .task = .{
                                .cond = .{
                                    .read = undefined,
                                    .write = undefined,
                                    .cond = dep,
                                    .arena = undefined,
                                    .task = .{ .run = &CondTask.run },
                                    .dependents = .empty,
                                },
                            },
                        };
                        {
                            var dep_curr = dep.read;
                            while (dep_curr) |block2| : (dep_curr = block2.next) {
                                for (block2.entries) |opt_res| {
                                    const res = opt_res orelse break;
                                    dep_node.read.append(std_allocator, res) catch @panic("oom");
                                }
                            }
                        }
                        {
                            var dep_curr = sys.write;
                            while (dep_curr) |dep_block| : (dep_curr = dep_block.next) {
                                for (dep_block.entries) |opt_res| {
                                    const res = opt_res orelse break;
                                    dep_node.write.append(std_allocator, res) catch @panic("oom");
                                }
                            }
                        }
                        dep_node.task.sys.read = allocator.alloc(*anyopaque, dep_node.read.items.len) catch @panic("oom");
                        dep_node.task.sys.write = allocator.alloc(*anyopaque, dep_node.write.items.len) catch @panic("oom");

                        var curr2 = sys.before;
                        while (curr2) |block2| : (curr2 = block2.next) {
                            for (block2.entries) |opt_dep2| {
                                const dep2 = opt_dep2 orelse break;
                                dep_node.deps.append(std_allocator, @intFromPtr(dep2)) catch @panic("oom");
                            }
                        }
                    }
                }
            } else {
                var curr = sys.before;
                while (curr) |block| : (curr = block.next) {
                    for (block.entries) |opt_dep| {
                        const dep = opt_dep orelse break;
                        node.deps.append(std_allocator, @intFromPtr(dep)) catch @panic("oom");
                    }
                }
            }
            {
                var curr = sys.parent_conditions;
                while (curr) |block| : (curr = block.next) {
                    for (block.entries) |opt_dep| {
                        const dep = opt_dep orelse break;
                        node.deps.append(std_allocator, @intFromPtr(dep)) catch @panic("oom");
                    }
                }
            }
        }

        // NOTE(gabriel): Now that we know all tasks we perform a topological
        // sort to get a correct (even if suboptimal) execution order.
        for (graph.values()) |node| _ = node.computeDepth(&graph);

        // NOTE(gabriel): We try to improve the runtime performance by minimizing
        // the number of synchronization operations to execute all tasks. We accomplish
        // this by grouping all tasks with an equal depth. All tasks with the same depth
        // can be executed in parallel, since they can't possibly depend on each other.
        const nodes = allocator.dupe(*Node, graph.values()) catch @panic("oom");
        std.mem.sortUnstable(*Node, nodes, {}, Node.sortByDepth);

        // NOTE(gabriel): Now we insert all remaining synchronization points.
        var cmds = ArrayList(CmdBufCmd).initCapacity(std_allocator, nodes.len) catch @panic("oom");
        var res_infos: AutoArrayHashMap(*Res, ResInfo) = .empty;
        var running: AutoArrayHashMap(*Node, usize) = .empty;
        var node_map: AutoArrayHashMap(*Node, usize) = .empty;
        for (nodes) |node| {
            // NOTE(gabriel): Insert synchronization points on all dependencies.
            for (node.deps.items) |dep_id| {
                const dep = graph.get(dep_id).?;
                const entry = running.fetchSwapRemove(dep) orelse continue;
                const entry_idx = entry.value;
                cmds.append(std_allocator, .{
                    .tag = .wait_on_cmd_indirect,
                    .payload = .{ .wait_on_cmd_indirect = cmds.items.len - entry_idx },
                }) catch @panic("oom");
            }

            // NOTE(gabriel): For reads we only synchronize if someone is writing.
            for (node.read.items) |res| {
                const info_entry = res_infos.getOrPut(std_allocator, res) catch @panic("oom");
                if (!info_entry.found_existing)
                    info_entry.value_ptr.* = .{ .access = .read, .res = res, .write = false };
                const info = info_entry.value_ptr;
                if (info.write) {
                    for (info.ref_by.items) |dep| {
                        const entry = running.fetchSwapRemove(dep) orelse continue;
                        const entry_idx = entry.value;
                        cmds.append(std_allocator, .{
                            .tag = .wait_on_cmd_indirect,
                            .payload = .{ .wait_on_cmd_indirect = cmds.items.len - entry_idx },
                        }) catch @panic("oom");
                    }
                    info.write = false;
                    info.ref_by.clearRetainingCapacity();
                }
                info.ref_by.append(std_allocator, node) catch @panic("oom");
            }

            // NOTE(gabriel): For writes we must synchronize unconditionally.
            for (node.write.items) |res| {
                const info_entry = res_infos.getOrPut(std_allocator, res) catch @panic("oom");
                if (!info_entry.found_existing)
                    info_entry.value_ptr.* = .{ .access = .write, .res = res, .write = true };
                const info = info_entry.value_ptr;

                for (info.ref_by.items) |dep| {
                    const entry = running.fetchSwapRemove(dep) orelse continue;
                    const entry_idx = entry.value;
                    cmds.append(std_allocator, .{
                        .tag = .wait_on_cmd_indirect,
                        .payload = .{ .wait_on_cmd_indirect = cmds.items.len - entry_idx },
                    }) catch @panic("oom");
                }
                info.access = .write;
                info.write = true;
                info.ref_by.clearRetainingCapacity();
                info.ref_by.append(std_allocator, node) catch @panic("oom");
            }

            const entry_idx = cmds.items.len;
            cmds.append(std_allocator, .{
                .tag = .enqueue_task,
                .payload = .{
                    .enqueue_task = switch (node.task) {
                        .cond => |*v| &v.task,
                        .sys => |*v| &v.task,
                    },
                },
            }) catch @panic("oom");
            running.put(std_allocator, node, entry_idx) catch @panic("oom");
            node_map.put(std_allocator, node, entry_idx) catch @panic("oom");
        }

        // NOTE(gabriel): As a final step we must insert the backwards
        // connections for the condition tasks. This allows us to skip
        // tasks that should not run.
        for (nodes) |node| {
            const cmd_idx = node_map.get(node).?;
            for (node.deps.items) |dep_id| {
                const dep = graph.get(dep_id).?;
                switch (dep.task) {
                    .cond => |*cond| cond.dependents.append(
                        std_allocator,
                        &cmds.items[cmd_idx],
                    ) catch @panic("oom"),
                    else => {},
                }
            }
        }

        self.nodes = nodes;
        self.cmds = cmds.items;
        self.res_infos = res_infos.values();
    }

    fn cleanupSystems(self: *Scheduler) void {
        const span = spanTrace(@src());
        defer span.exit();
        while (self.cleanup_systems) |sys| {
            self.cleanup_systems = sys.next;
            sys.cleanup();
        }
    }

    fn runInner(self: *Scheduler, arena: *Arena, generation: u64) void {
        self.semaphore.wait(generation - 1);
        self.rebuildCmds();
        for (self.res_infos) |info| _ = info.lock();
        for (self.nodes) |node| switch (node.task) {
            .cond => |*v| v.arena = arena,
            .sys => |*v| v.arena = arena,
        };

        const scratch = &self.cmd_arena;
        const pos = scratch.pos.load(.monotonic);
        const cmds = scratch.allocator().dupe(CmdBufCmd, self.cmds) catch @panic("oom");

        if (self.executor) |exe| {
            var buffer: CmdBuf = .{
                .label = .init(self.label),
                .cmds = .init(cmds),
            };
            const handle = exe.enqueue(&buffer);
            _ = handle.join();
        } else {
            const span = spanTraceNamed(@src(), "{s}", .{self.label});
            defer span.exit();
            for (cmds) |cmd| {
                switch (cmd.tag) {
                    .enqueue_task => {
                        const task = cmd.payload.enqueue_task;
                        const task_span = spanTraceNamed(@src(), "{?s}", .{task.label.get()});
                        defer task_span.exit();
                        task.run(task, 0);
                    },
                    else => {},
                }
            }
        }

        // NOTE(gabriel): Some systems may spawn additional jobs.
        // Now that all systems have been run, we can wait for them to complete.
        for (self.nodes) |node| switch (node.task) {
            .cond => {},
            .sys => |*v| if (v.fence) |f| f.wait(),
        };
        scratch.pos.store(pos, .monotonic);

        for (self.res_infos) |info| info.unlock();
        self.cleanupSystems();
    }

    fn run(self: *Scheduler, arena: *Arena) void {
        const span = spanTrace(@src());
        defer span.exit();
        if (self.executor != null) {
            var completion: Fence = .{};
            self.schedule(arena, null, &completion);
            completion.wait();
        } else {
            const generation = self.next_generation.fetchAdd(1, .monotonic);
            self.runInner(arena, generation);
            self.semaphore.signal(generation);
        }
    }

    fn schedule(self: *Scheduler, arena: *Arena, start: ?*Fence, completion: ?*Fence) void {
        const span = spanTrace(@src());
        defer span.exit();
        if (self.executor) |exe| {
            const task = blk: {
                self.mutex.lock();
                defer self.mutex.unlock();
                if (self.jobs) |j| {
                    self.jobs = j.next;
                    break :blk j;
                } else break :blk self.arena.allocator().create(ScheduleTask) catch @panic("oom");
            };
            task.* = .{
                .sched = self,
                .arena = arena,
                .generation = self.next_generation.fetchAdd(1, .monotonic),
                .start = start,
                .completion = completion,
                .task = .{ .label = .init(self.label), .run = &ScheduleTask.run },
                .cmd = .{ .tag = .enqueue_task, .payload = .{ .enqueue_task = &task.task } },
                .cmd_buf = .{ .cmds = .init(@ptrCast(&task.cmd)), .deinit = &ScheduleTask.deinit },
            };
            exe.enqueueDetached(&task.cmd_buf);
        } else {
            if (start) |f| f.wait();
            self.run(arena);
            if (completion) |f| f.signal();
        }
    }

    fn flush(self: *Scheduler) void {
        const generation = self.next_generation.load(.monotonic);
        self.semaphore.wait(generation - 1);
    }
};

pub fn worldInit(world: **fimo_worlds.World, desc: *const fimo_worlds.World.Desc) callconv(.c) Status {
    const d: WorldDesc = .{ .label = desc.label.intoSliceOrEmpty() };
    world.* = @ptrCast(World.init(d) catch |err| {
        ctx.setResult(.initErr(.initError(err)));
        return .err;
    });
    return .ok;
}

pub fn worldDeinit(world: *fimo_worlds.World) callconv(.c) void {
    const w: *World = @ptrCast(@alignCast(world));
    w.deinit();
}

pub fn worldAddRes(
    world: *fimo_worlds.World,
    desc: *const fimo_worlds.Res(anyopaque).Desc,
) callconv(.c) *fimo_worlds.Res(anyopaque) {
    const d: ResDesc = .{ .label = desc.label.intoSliceOrEmpty(), .value = desc.value };
    const w: *World = @ptrCast(@alignCast(world));
    return @ptrCast(w.addRes(d));
}

pub fn worldAddScheduler(
    world: *fimo_worlds.World,
    desc: *const fimo_worlds.Scheduler.Desc,
) callconv(.c) *fimo_worlds.Scheduler {
    const d: SchedulerDesc = .{ .label = desc.label.intoSliceOrEmpty(), .executor = desc.executor };
    const w: *World = @ptrCast(@alignCast(world));
    return @ptrCast(w.addScheduler(d));
}

pub fn resDeinit(res: *fimo_worlds.Res(anyopaque)) callconv(.c) void {
    const r: *Res = @ptrCast(@alignCast(res));
    r.deinit();
}

pub fn resLockRead(res: *fimo_worlds.Res(anyopaque)) callconv(.c) *anyopaque {
    const r: *Res = @ptrCast(@alignCast(res));
    return r.lockRead();
}

pub fn resUnlockRead(res: *fimo_worlds.Res(anyopaque)) callconv(.c) void {
    const r: *Res = @ptrCast(@alignCast(res));
    r.unlockRead();
}

pub fn resLockWrite(res: *fimo_worlds.Res(anyopaque)) callconv(.c) *anyopaque {
    const r: *Res = @ptrCast(@alignCast(res));
    return r.lockWrite();
}

pub fn resUnlockWrite(res: *fimo_worlds.Res(anyopaque)) callconv(.c) void {
    const r: *Res = @ptrCast(@alignCast(res));
    r.unlockWrite();
}

pub fn schedulerDeinit(sched: *fimo_worlds.Scheduler) callconv(.c) void {
    const s: *Scheduler = @ptrCast(@alignCast(sched));
    s.deinit();
}

pub fn schedulerAddSys(
    sched: *fimo_worlds.Scheduler,
    desc: *const fimo_worlds.Sys.Desc,
    sys: **fimo_worlds.Sys,
) callconv(.c) Status {
    const scratch = ctx.getScratchArena(null);
    const pos = scratch.pos.load(.monotonic);
    defer scratch.pos.store(pos, .monotonic);

    const d = SysDesc.initC(desc, scratch) catch |err| {
        ctx.setResult(.initErr(.initError(err)));
        return .err;
    };
    const s: *Scheduler = @ptrCast(@alignCast(sched));
    sys.* = @ptrCast(s.addSys(d) catch |err| {
        ctx.setResult(.initErr(.initError(err)));
        return .err;
    });
    return .ok;
}

pub fn schedulerRun(sched: *fimo_worlds.Scheduler, arena: *Arena) callconv(.c) void {
    const s: *Scheduler = @ptrCast(@alignCast(sched));
    s.run(arena);
}

pub fn schedulerSchedule(
    sched: *fimo_worlds.Scheduler,
    arena: *Arena,
    start: ?*Fence,
    completion: ?*Fence,
) callconv(.c) void {
    const s: *Scheduler = @ptrCast(@alignCast(sched));
    s.schedule(arena, start, completion);
}

pub fn schedulerFlush(sched: *fimo_worlds.Scheduler) callconv(.c) void {
    const s: *Scheduler = @ptrCast(@alignCast(sched));
    s.flush();
}

pub fn sysDeinit(sys: *fimo_worlds.Sys, fence: ?*Fence) callconv(.c) void {
    const s: *Sys = @ptrCast(@alignCast(sys));
    s.deinit(fence);
}
