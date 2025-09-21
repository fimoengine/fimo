const std = @import("std");

pub const c = @import("c");
const fimo_std = @import("fimo_std");
const Error = fimo_std.ctx.Error;
const memory = fimo_std.memory;
const Arena = memory.Arena;
const utils = fimo_std.utils;
const SliceConst = utils.SliceConst;
const fimo_tasks = @import("fimo_tasks_meta");
const Task = fimo_tasks.Task;
const CmdBufCmd = fimo_tasks.CmdBufCmd;
const CmdBuf = fimo_tasks.CmdBuf;
const Executor = fimo_tasks.Executor;
const Fence = fimo_tasks.sync.Fence;

pub const symbols = @import("symbols.zig");
const testing = @import("testing.zig");

/// A collection of resources and systems.
pub const World = opaque {
    /// Descriptor for a world.
    pub const Desc = extern struct {
        /// Optional label of the world.
        label: SliceConst(u8) = .fromSlice(null),
    };

    /// Initializes a new empty world.
    pub fn init(desc: Desc) Error!*World {
        var world: *World = undefined;
        const sym = symbols.world_init.getGlobal().get();
        try sym(&world, &desc).intoErrorUnion();
        return world;
    }

    /// Deinitializes an empty world.
    pub fn deinit(self: *World) void {
        const sym = symbols.world_deinit.getGlobal().get();
        sym(self);
    }

    /// Adds a new resource to the world.
    pub fn addRes(self: *World, T: type, desc: Res(T).Desc) *Res(T) {
        const sym = symbols.world_add_res.getGlobal().get();
        return @ptrCast(sym(self, @ptrCast(&desc)));
    }

    /// Adds an empty scheduler to the world.
    pub fn addScheduler(self: *World, desc: Scheduler.Desc) *Scheduler {
        const sym = symbols.world_add_scheduler.getGlobal().get();
        return sym(self, &desc);
    }
};

test "World: smoke test" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    world.deinit();
}

/// A handle to a resource in a world.
///
/// The handle uniquely identifies the resource in the world.
pub fn Res(T: type) type {
    return opaque {
        pub const Value = T;

        /// Descriptor for a resource.
        pub const Desc = extern struct {
            /// Optional label of the resource.
            label: SliceConst(u8) = .fromSlice(null),
            /// Pointer to the resource.
            value: *T,
        };

        /// Initializes a new resource in the world.
        pub fn init(world: *World, desc: Desc) *@This() {
            return world.addRes(T, desc);
        }

        /// Invalidates the resource.
        ///
        /// The resource may not be in use.
        pub fn deinit(self: *@This()) void {
            const sym = symbols.resource_deinit.getGlobal().get();
            sym(@ptrCast(self));
        }

        /// Acquires the resource with read access.
        ///
        /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
        /// strategy to acquire all necessary resources before executing any system.
        pub fn lockRead(self: *@This()) *T {
            const sym = symbols.resource_lock_read.getGlobal().get();
            return @ptrCast(@alignCast(sym(@ptrCast(self))));
        }

        /// Unlocks a resource acquired with read access.
        pub fn unlockRead(self: *@This()) void {
            const sym = symbols.resource_unlock_read.getGlobal().get();
            return sym(@ptrCast(self));
        }

        /// Acquires the resource with write access.
        ///
        /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
        /// strategy to acquire all necessary resources before executing any system.
        pub fn lockWrite(self: *@This()) *T {
            const sym = symbols.resource_lock_write.getGlobal().get();
            return @ptrCast(@alignCast(sym(@ptrCast(self))));
        }

        /// Unlocks a resource acquired with write access.
        pub fn unlockWrite(self: *@This()) void {
            const sym = symbols.resource_unlock_write.getGlobal().get();
            return sym(@ptrCast(self));
        }
    };
}

test "Res: smoke test" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    var value: i32 = 5;
    const res = Res(i32).init(world, .{ .label = .fromSlice("my res"), .value = &value });
    defer res.deinit();

    const value_ptr = res.lockRead();
    defer res.unlockRead();
    try std.testing.expectEqual(&value, value_ptr);
}

test "Res: lock read" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    var value: usize = 0;
    const res = Res(usize).init(world, .{ .label = .fromSlice("my res"), .value = &value });
    defer res.deinit();

    const num_writers: usize = 2;
    const num_readers: usize = 4;
    const num_writes: usize = 10000;
    const num_reads: usize = num_writes * 2;

    const Runner = struct {
        writes: *Res(usize),
        reads: std.atomic.Value(usize) = std.atomic.Value(usize).init(0),

        term1: usize = 0,
        term2: usize = 0,
        term_sum: usize = 0,

        read_task: Task = .{ .batch_len = num_readers, .run = reader },
        write_task: Task = .{ .batch_len = num_writers, .run = writer },

        fn reader(task: *Task, idx: usize) callconv(.c) void {
            _ = idx;
            const self: *@This() = @alignCast(@fieldParentPtr("read_task", task));
            while (true) {
                const writes = self.writes.lockRead();
                defer self.writes.unlockRead();

                if (writes.* >= num_writes or self.reads.load(.unordered) >= num_reads)
                    break;

                self.check();

                _ = self.reads.fetchAdd(1, .monotonic);
            }
        }

        fn writer(task: *Task, idx: usize) callconv(.c) void {
            const self: *@This() = @alignCast(@fieldParentPtr("write_task", task));
            var prng = std.Random.DefaultPrng.init(idx);
            var rnd = prng.random();

            while (true) {
                const writes = self.writes.lockWrite();
                defer self.writes.unlockWrite();

                if (writes.* >= num_writes)
                    break;

                self.check();

                const term1 = rnd.int(usize);
                self.term1 = term1;

                fimo_tasks.yield();

                const term2 = rnd.int(usize);
                self.term2 = term2;
                fimo_tasks.yield();

                self.term_sum = term1 +% term2;
                writes.* += 1;
            }
        }

        fn check(self: *const @This()) void {
            const term_sum = self.term_sum;
            fimo_tasks.yield();

            const term2 = self.term2;
            fimo_tasks.yield();

            const term1 = self.term1;
            std.testing.expectEqual(term_sum, term1 +% term2) catch unreachable;
        }
    };
    const executor = Executor.globalExecutor();
    var runner = Runner{ .writes = res };
    var cmds: [2]CmdBufCmd = undefined;
    cmds[0] = .{ .tag = .enqueue_task, .payload = .{ .enqueue_task = &runner.read_task } };
    cmds[1] = .{ .tag = .enqueue_task, .payload = .{ .enqueue_task = &runner.write_task } };
    var cmd_buf = CmdBuf{ .cmds = .init(&cmds) };
    const cmd_handle = executor.enqueue(&cmd_buf);
    try std.testing.expectEqual(.completed, cmd_handle.join());

    const value_ptr = res.lockRead();
    defer res.unlockRead();
    try std.testing.expectEqual(num_writes, value_ptr.*);
}

test "Res: lock write" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    var value: usize = 0;
    const res = Res(usize).init(world, .{ .label = .fromSlice("my res"), .value = &value });
    defer res.deinit();

    const num_jobs = 4;
    const iterations = 1000;

    const Runner = struct {
        res: *Res(usize),
        task: Task = .{ .batch_len = num_jobs, .run = run },

        fn run(task: *Task, idx: usize) callconv(.c) void {
            _ = idx;
            const self: *@This() = @alignCast(@fieldParentPtr("task", task));
            for (0..iterations) |_| {
                const ptr = self.res.lockWrite();
                defer self.res.unlockWrite();
                ptr.* += 1;
            }
        }
    };
    const executor = Executor.globalExecutor();
    var runner = Runner{ .res = res };
    var cmd = CmdBufCmd{
        .tag = .enqueue_task,
        .payload = .{ .enqueue_task = &runner.task },
    };
    var cmd_buf = CmdBuf{ .cmds = .init(@ptrCast(&cmd)) };
    const cmd_handle = executor.enqueue(&cmd_buf);
    try std.testing.expectEqual(.completed, cmd_handle.join());

    const value_ptr = res.lockRead();
    defer res.unlockRead();
    try std.testing.expectEqual(num_jobs * iterations, value_ptr.*);
}

/// Handle to a system scheduler within a world.
pub const Scheduler = opaque {
    /// Descriptor for a system scheduler.
    pub const Desc = extern struct {
        /// Optional label of the scheduler.
        label: SliceConst(u8) = .fromSlice(null),
        /// Optional executor for the scheduler.
        ///
        /// If no scheduler is provided, the scheduler will be created in
        /// single threaded mode. And all systems will be run in the thread
        /// that starts the schedule operation.
        executor: ?*Executor = null,
    };

    /// Initializes a new scheduler in the world.
    pub fn init(world: *World, desc: Desc) *Scheduler {
        return world.addScheduler(desc);
    }

    /// Invalidates the scheduler.
    ///
    /// The scheduler may not be running.
    pub fn deinit(self: *Scheduler) void {
        const sym = symbols.scheduler_deinit.getGlobal().get();
        sym(self);
    }

    /// Adds a system(-set) to the scheduler.
    pub fn addSys(self: *Scheduler, desc: Sys.Desc) Error!*Sys {
        var sys: *Sys = undefined;
        const sym = symbols.scheduler_add_sys.getGlobal().get();
        try sym(self, &desc, &sys).intoErrorUnion();
        return sys;
    }

    /// Starts a new run of the systems, blocking the current thread until it completes.
    pub fn run(self: *Scheduler, arena: *Arena) void {
        const sym = symbols.scheduler_run.getGlobal().get();
        sym(self, arena);
    }

    /// Schedules the systems of the scheduler to be run asynchronously.
    ///
    /// Multiple concurrent schedule operations are serialized. The arena must remain valid until `completion` is signaled.
    /// The systems will start running after `start` is signaled. If no executor is associated with the scheduler, this
    /// operation will block the calling thread until all systems are run.
    pub fn schedule(self: *Scheduler, arena: *Arena, start: ?*Fence, completion: ?*Fence) void {
        const sym = symbols.scheduler_schedule.getGlobal().get();
        sym(self, arena, start, completion);
    }

    /// Blocks the calling thread until all scheduled operations are completed.
    pub fn flush(self: *Scheduler) void {
        const sym = symbols.scheduler_flush.getGlobal().get();
        sym(self);
    }
};

test "Scheduler: smoke test" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    const scheduler = Scheduler.init(world, .{ .label = .fromSlice("my scheduler") });
    defer scheduler.deinit();
}

test "Scheduler: run (single-threaded)" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    var value: usize = 0;
    const res = Res(usize).init(world, .{ .label = .fromSlice("my res"), .value = &value });
    defer res.deinit();

    const scheduler = Scheduler.init(world, .{ .label = .fromSlice("my scheduler") });
    defer scheduler.deinit();

    const sys = try scheduler.addSys(.{
        .tag = .sys,
        .data = .{
            .sys = .{
                .label = .fromSlice("my sys"),
                .read = .fromSlice(null),
                .write = .fromSlice(@ptrCast(&res)),
                .data = null,
                .system = &struct {
                    fn run(ctx: ?*anyopaque, args: *const Sys.Args) callconv(.c) ?*Fence {
                        _ = ctx;
                        const v = args.getWrite(usize, 0);
                        v.* += 1;
                        return null;
                    }
                }.run,
            },
        },
    });
    defer {
        var fence: Fence = .{};
        sys.deinit(&fence);
        fence.wait();
    }

    const arena = fimo_std.ctx.getScratchArena(null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.run(arena);

    const value_ptr = res.lockRead();
    defer res.unlockRead();
    try std.testing.expectEqual(5, value_ptr.*);
}

test "Scheduler: run (multi-threaded)" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    var value: usize = 0;
    const res = Res(usize).init(world, .{ .label = .fromSlice("my res"), .value = &value });
    defer res.deinit();

    const scheduler = Scheduler.init(world, .{
        .label = .fromSlice("my scheduler"),
        .executor = Executor.globalExecutor(),
    });
    defer scheduler.deinit();

    const sys = try scheduler.addSys(.{
        .tag = .sys,
        .data = .{
            .sys = .{
                .label = .fromSlice("my sys"),
                .read = .fromSlice(null),
                .write = .fromSlice(@ptrCast(&res)),
                .data = null,
                .system = &struct {
                    fn run(ctx: ?*anyopaque, args: *const Sys.Args) callconv(.c) ?*Fence {
                        _ = ctx;
                        const v = args.getWrite(usize, 0);
                        v.* += 1;
                        return null;
                    }
                }.run,
            },
        },
    });
    defer {
        var fence: Fence = .{};
        sys.deinit(&fence);
        fence.wait();
    }

    const arena = fimo_std.ctx.getScratchArena(null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.flush();

    const value_ptr = res.lockRead();
    defer res.unlockRead();
    try std.testing.expectEqual(5, value_ptr.*);
}

test "Scheduler: run (sub-task)" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    var value: usize = 0;
    const res = Res(usize).init(world, .{ .label = .fromSlice("my res"), .value = &value });
    defer res.deinit();

    const scheduler = Scheduler.init(world, .{
        .label = .fromSlice("my scheduler"),
        .executor = Executor.globalExecutor(),
    });
    defer scheduler.deinit();

    const sys = try scheduler.addSys(.{
        .tag = .sys,
        .data = .{
            .sys = .{
                .label = .fromSlice("my sys"),
                .read = .fromSlice(null),
                .write = .fromSlice(@ptrCast(&res)),
                .data = null,
                .system = &struct {
                    v: *usize,
                    fence: Fence = .{},
                    task: Task = .{ .run = &runTask },
                    cmd: CmdBufCmd,
                    cmd_buf: CmdBuf,

                    fn runTask(task: *Task, idx: usize) callconv(.c) void {
                        _ = idx;
                        const self: *@This() = @alignCast(@fieldParentPtr("task", task));
                        self.v.* += 1;
                    }

                    fn deinit(cmd_buf: *CmdBuf) callconv(.c) void {
                        const self: *@This() = @alignCast(@fieldParentPtr("cmd_buf", cmd_buf));
                        self.fence.signal();
                    }

                    fn run(ctx: ?*anyopaque, args: *const Sys.Args) callconv(.c) ?*Fence {
                        _ = ctx;

                        const self = args.arena.allocator().create(@This()) catch unreachable;
                        self.* = .{
                            // NOTE(gabriel): Is only legal in this specific circumstance.
                            .v = args.getWrite(usize, 0),
                            .cmd = .{
                                .tag = .enqueue_task,
                                .payload = .{ .enqueue_task = &self.task },
                            },
                            .cmd_buf = .{ .cmds = .init(@ptrCast(&self.cmd)), .deinit = &deinit },
                        };
                        const executor = Executor.globalExecutor();
                        executor.enqueueDetached(&self.cmd_buf);
                        return &self.fence;
                    }
                }.run,
            },
        },
    });
    defer {
        var fence: Fence = .{};
        sys.deinit(&fence);
        fence.wait();
    }

    const arena = fimo_std.ctx.getScratchArena(null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.schedule(arena, null, null);
    scheduler.flush();

    const value_ptr = res.lockRead();
    defer res.unlockRead();
    try std.testing.expectEqual(5, value_ptr.*);
}

/// A handle to a registered system(-set).
///
/// The handle uniquely identifies the system(-set) in the scheduler.
pub const Sys = opaque {
    /// Arguments passed to a system function.
    ///
    /// Should not be copied, as it may be extended in the future.
    pub const Args = extern struct {
        world: *World,
        sched: *Scheduler,
        arena: *Arena,
        read: SliceConst(*anyopaque),
        write: SliceConst(*anyopaque),

        /// Accesses a read-only resource.
        pub fn getRead(self: *const Args, T: type, idx: usize) *T {
            const read = self.read.intoSliceOrEmpty();
            const ptr = read[idx];
            return @ptrCast(@alignCast(ptr));
        }

        /// Accesses a read-write resource.
        pub fn getWrite(self: *const Args, T: type, idx: usize) *T {
            const read = self.write.intoSliceOrEmpty();
            const ptr = read[idx];
            return @ptrCast(@alignCast(ptr));
        }
    };

    /// Function that will be called when a scheduler runs a system.
    ///
    /// The system may spawn additional tasks which outlive the system.
    /// In that case, the system may return a fence to indicate when the sub-tasks are done.
    /// The scheduler will then wait upon the completion of all sub-tasks before completing it's run.
    pub const Run = *const fn (data: ?*anyopaque, args: *const Args) callconv(.c) ?*Fence;

    /// A condition function that is executed to determine whether the execute a system(-set).
    ///
    /// If the function returns `false`, the scheduler will skip the execution of the system(-set).
    pub const Cond = *const fn (data: ?*anyopaque, args: *const Args) callconv(.c) bool;

    pub const CondDesc = extern struct {
        /// List of required resrources with read access.
        read: SliceConst(*Res(anyopaque)),
        /// List of required resrources with write access.
        write: SliceConst(*Res(anyopaque)),
        /// Data to pass to the condition function.
        data: ?*anyopaque,
        /// Condition function to invoke.
        condition: Cond,
    };

    /// Description of a system(-set).
    pub const Desc = extern struct {
        /// Active tag in the union.
        tag: enum(i32) { sys = 0, set = 1, _ },
        /// Systems to run before the systems of the current set.
        ///
        /// The current set will see the effects of all listed systems.
        before: SliceConst(*Sys) = .fromSlice(null),
        /// Systems to run after the current set.
        ///
        /// These systems will see the effects of the current set.
        after: SliceConst(*Sys) = .fromSlice(null),
        /// Conditions to evaluate whether to run or skip the current set.
        ///
        /// All conditions must return `true` to execute the set.
        /// Each condition will only be evaluated once.
        conditions: SliceConst(CondDesc) = .fromSlice(null),
        data: extern union {
            sys: extern struct {
                label: SliceConst(u8) = .fromSlice(null),
                /// List of required resrources with read access.
                read: SliceConst(*Res(anyopaque)) = .fromSlice(null),
                /// List of required resrources with write access.
                write: SliceConst(*Res(anyopaque)) = .fromSlice(null),
                /// Data to pass to the system function.
                data: ?*anyopaque = null,
                /// System function to invoke.
                system: Run,
            },
            set: extern struct {
                /// Whether to serialize the execution of the systems in the set.
                serialize: bool = false,
                /// List of sub-systems in the set.
                sub_desc: SliceConst(Desc),
            },
        },
    };

    /// Initializes a new system in the scheduler.
    pub fn init(scheduler: *Scheduler, desc: *Desc) Error!*Sys {
        return scheduler.addSys(desc);
    }

    /// Removes the system from the scheduler.
    ///
    /// The handle is invalidated after calling this function.
    /// The operation signals the fence on completion.
    /// If no fence is provided, this function blocks until completion.
    pub fn deinit(self: *Sys, fence: ?*Fence) void {
        const sym = symbols.sys_deinit.getGlobal().get();
        sym(self, fence);
    }
};

test "Sys: empty description" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    const scheduler = Scheduler.init(world, .{ .label = .fromSlice("my scheduler") });
    defer scheduler.deinit();

    var desc: Sys.Desc = undefined;
    desc = .{
        .tag = .set,
        .data = .{
            .set = .{
                .sub_desc = .fromSlice(null),
            },
        },
    };
    try std.testing.expectError(error.OperationFailed, scheduler.addSys(desc));
}

test "Sys: duplicate description" {
    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    const scheduler = Scheduler.init(world, .{ .label = .fromSlice("my scheduler") });
    defer scheduler.deinit();

    var sub_desc: [1]Sys.Desc = .{undefined};
    sub_desc[0] = .{
        .tag = .set,
        .data = .{
            .set = .{
                .sub_desc = .fromSlice(&sub_desc),
            },
        },
    };
    try std.testing.expectError(error.OperationFailed, scheduler.addSys(sub_desc[0]));
}

test "Sys: cyclic dependency" {
    const Dummy = struct {
        fn run(ctx: ?*anyopaque, args: *const Sys.Args) callconv(.c) ?*Fence {
            _ = ctx;
            _ = args;
            return null;
        }
    }.run;

    try testing.GlobalCtx.init();
    defer testing.GlobalCtx.deinit();

    const world = try World.init(.{ .label = .fromSlice("my world") });
    defer world.deinit();

    const scheduler = Scheduler.init(world, .{ .label = .fromSlice("my scheduler") });
    defer scheduler.deinit();

    const sys_0 = try scheduler.addSys(.{
        .tag = .sys,
        .data = .{
            .sys = .{ .system = &Dummy },
        },
    });
    defer {
        var f: Fence = .{};
        sys_0.deinit(&f);
        f.wait();
    }

    const sys_1_before: [1]*Sys = .{sys_0};
    const sys_1 = try scheduler.addSys(.{
        .tag = .sys,
        .before = .fromSlice(&sys_1_before),
        .data = .{
            .sys = .{ .system = &Dummy },
        },
    });
    defer {
        var f: Fence = .{};
        sys_1.deinit(&f);
        f.wait();
    }

    const sys_2_before: [1]*Sys = .{sys_1};
    const sys_2_after: [1]*Sys = .{sys_0};
    const result = scheduler.addSys(.{
        .tag = .sys,
        .before = .fromSlice(&sys_2_before),
        .after = .fromSlice(&sys_2_after),
        .data = .{
            .sys = .{ .system = &Dummy },
        },
    });
    try std.testing.expectError(error.OperationFailed, result);
}
