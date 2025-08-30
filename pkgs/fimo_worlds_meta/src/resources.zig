const std = @import("std");

const fimo_std = @import("fimo_std");
const Error = fimo_std.ctx.Error;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Task = fimo_tasks_meta.Task;
const CmdBufCmd = fimo_tasks_meta.CmdBufCmd;
const CmdBuf = fimo_tasks_meta.CmdBuf;
const Executor = fimo_tasks_meta.Executor;

const Job = @import("Job.zig");
const Fence = Job.Fence;
const symbols = @import("symbols.zig");
const testing = @import("testing.zig");
const worlds = @import("worlds.zig");
const World = worlds.World;

/// Options for registering a new resource.
pub const RegisterOptions = struct {
    label: ?[]const u8 = null,

    /// Description of a resource.
    pub const Descriptor = extern struct {
        next: ?*const anyopaque,
        label: ?[*]const u8,
        label_len: usize,
        size: usize,
        alignment: usize,
    };
};

/// A resource handle with an unknown resource type.
pub const Resource = opaque {};

/// A handle to a registered resource.
pub fn TypedResource(T: type) type {
    return opaque {
        pub const Self = @This();
        pub const ResourceType = T;

        /// Registers a new resource to the universe.
        ///
        /// Registered resources may be instantiated by any world that knows its id.
        pub fn register(options: RegisterOptions) Error!*Self {
            const desc = RegisterOptions.Descriptor{
                .next = null,
                .label = if (options.label) |l| l.ptr else null,
                .label_len = if (options.label) |l| l.len else 0,
                .size = @sizeOf(T),
                .alignment = @alignOf(T),
            };

            var handle: *Resource = undefined;
            const sym = symbols.resource_register.getGlobal().get();
            try sym(&desc, &handle).intoErrorUnion();
            return fromUntyped(handle);
        }

        /// Unregister the resource from the universe.
        ///
        /// Once unregistered, the identifier is invalidated and may be reused by another resouce.
        /// The resource must not be used by any world when this method is called.
        pub fn unregister(self: *Self) void {
            const sym = symbols.resource_unregister.getGlobal().get();
            return sym(self.asUntyped());
        }

        /// Casts the typed identifier to an untyped one.
        pub fn asUntyped(self: *Self) *Resource {
            return @ptrCast(self);
        }

        /// Casts the untyped handle to a typed one.
        pub fn fromUntyped(handle: *Resource) *Self {
            return @ptrCast(handle);
        }

        /// Checks if the resource is instantiated in the world.
        pub fn existsInWorld(self: *Self, world: *World) bool {
            return world.hasResource(self.asUntyped());
        }

        /// Adds the resource to the world.
        pub fn addToWorld(self: *Self, world: *World, value: T) Error!void {
            return world.addResource(self.asUntyped(), @ptrCast(&value));
        }

        /// Removes the resource from the world.
        pub fn removeFromWorld(self: *Self, world: *World) Error!T {
            var value: T = undefined;
            try world.removeResource(self.asUntyped(), @ptrCast(&value));
            return value;
        }

        /// Returns an exclusive reference to the resource in the world.
        ///
        /// The caller will block until there are no active shared or exlusive references to the
        /// resource.
        pub fn lockInWorldExclusive(self: *Self, world: *World) *T {
            var out: *anyopaque = undefined;
            world.lockResourcesRaw(&.{self.asUntyped()}, &.{}, (&out)[0..1]);
            return @ptrCast(@alignCast(out));
        }

        /// Returns a shared reference to the resource in the world.
        ///
        /// The caller will block until there are no active exlusive references to the resource.
        pub fn lockInWorldShared(self: *Self, world: *World) *T {
            var out: *anyopaque = undefined;
            world.lockResourcesRaw(&.{}, &.{self.asUntyped()}, (&out)[0..1]);
            return @ptrCast(@alignCast(out));
        }

        /// Unlocks an exclusive resource lock in the world.
        pub fn unlockInWorldExclusive(self: *Self, world: *World) void {
            world.unlockResourceExclusive(self.asUntyped());
        }

        /// Unlocks a shared resource lock in the world.
        pub fn unlockInWorldShared(self: *Self, world: *World) void {
            world.unlockResourceShared(self.asUntyped());
        }
    };
}

test "resource: smoke test" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const handle_1 = try TypedResource(i32).register(.{ .label = "resource-1" });
    defer handle_1.unregister();

    const handle_2 = try TypedResource(i32).register(.{ .label = "resource-2" });
    defer handle_2.unregister();
    try std.testing.expect(handle_1 != handle_2);
}

test "resource: add to world" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const handle = try TypedResource(i32).register(.{ .label = "resource-1" });
    defer handle.unregister();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const value: i32 = 5;
    try std.testing.expect(!handle.existsInWorld(world));
    try handle.addToWorld(world, value);
    defer _ = handle.removeFromWorld(world) catch unreachable;
    try std.testing.expect(handle.existsInWorld(world));

    const ptr = handle.lockInWorldExclusive(world);
    defer handle.unlockInWorldExclusive(world);
    try std.testing.expectEqual(value, ptr.*);
}

test "resource: unique lock" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const handle = try TypedResource(usize).register(.{ .label = "resource-1" });
    defer handle.unregister();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const executor = world.getExecutor();
    try handle.addToWorld(world, 0);
    defer _ = handle.removeFromWorld(world) catch unreachable;

    const num_jobs = 4;
    const iterations = 1000;

    const Runner = struct {
        handle: *TypedResource(usize),
        world: *World,
        task: Task = .{ .batch_len = num_jobs, .run = run },

        fn run(task: *Task, idx: usize) callconv(.c) void {
            _ = idx;
            const self: *@This() = @alignCast(@fieldParentPtr("task", task));
            for (0..iterations) |_| {
                const ptr = self.handle.lockInWorldExclusive(self.world);
                defer self.handle.unlockInWorldExclusive(self.world);
                ptr.* += 1;
            }
        }
    };
    var runner = Runner{ .handle = handle, .world = world };
    var cmd = CmdBufCmd{
        .tag = .enqueue_task,
        .payload = .{ .enqueue_task = &runner.task },
    };
    var cmd_buf = CmdBuf{ .cmds = .init(@ptrCast(&cmd)) };
    const cmd_handle = executor.enqueue(&cmd_buf);
    try std.testing.expectEqual(.completed, cmd_handle.join());

    const ptr = handle.lockInWorldExclusive(world);
    defer handle.unlockInWorldExclusive(world);
    try std.testing.expectEqual(num_jobs * iterations, ptr.*);
}

test "resource: shared lock" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const handle = try TypedResource(usize).register(.{ .label = "resource-1" });
    defer handle.unregister();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const executor = world.getExecutor();

    try handle.addToWorld(world, 0);
    defer _ = handle.removeFromWorld(world) catch unreachable;

    const num_writers: usize = 2;
    const num_readers: usize = 4;
    const num_writes: usize = 10000;
    const num_reads: usize = num_writes * 2;

    const Runner = struct {
        world: *World,

        writes: *TypedResource(usize),
        reads: std.atomic.Value(usize) = std.atomic.Value(usize).init(0),

        term1: usize = 0,
        term2: usize = 0,
        term_sum: usize = 0,

        read_task: Task = .{ .batch_len = num_readers, .run = reader },
        write_task: Task = .{ .batch_len = num_writers, .run = writer },

        const Self = @This();

        fn reader(task: *Task, idx: usize) callconv(.c) void {
            _ = idx;
            const self: *@This() = @alignCast(@fieldParentPtr("read_task", task));
            while (true) {
                const writes = self.writes.lockInWorldShared(self.world);
                defer self.writes.unlockInWorldShared(self.world);

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
                const writes = self.writes.lockInWorldExclusive(self.world);
                defer self.writes.unlockInWorldExclusive(self.world);

                if (writes.* >= num_writes)
                    break;

                self.check();

                const term1 = rnd.int(usize);
                self.term1 = term1;

                fimo_tasks_meta.yield();

                const term2 = rnd.int(usize);
                self.term2 = term2;
                fimo_tasks_meta.yield();

                self.term_sum = term1 +% term2;
                writes.* += 1;
            }
        }

        fn check(self: *const Self) void {
            const term_sum = self.term_sum;
            fimo_tasks_meta.yield();

            const term2 = self.term2;
            fimo_tasks_meta.yield();

            const term1 = self.term1;
            std.testing.expectEqual(term_sum, term1 +% term2) catch unreachable;
        }
    };
    var runner = Runner{ .world = world, .writes = handle };
    var cmds: [2]CmdBufCmd = undefined;
    cmds[0] = .{ .tag = .enqueue_task, .payload = .{ .enqueue_task = &runner.read_task } };
    cmds[1] = .{ .tag = .enqueue_task, .payload = .{ .enqueue_task = &runner.write_task } };
    var cmd_buf = CmdBuf{ .cmds = .init(&cmds) };
    const cmd_handle = executor.enqueue(&cmd_buf);

    try std.testing.expectEqual(.completed, cmd_handle.join());

    const writes = handle.lockInWorldShared(world);
    defer handle.unlockInWorldShared(world);
    try std.testing.expectEqual(num_writes, writes.*);
}
