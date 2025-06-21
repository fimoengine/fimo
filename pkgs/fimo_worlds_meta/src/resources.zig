const std = @import("std");

const fimo_tasks_meta = @import("fimo_tasks_meta");

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

/// A resource id with an unknown resource type.
pub const ResourceId = enum(usize) {
    _,
};

/// A unique identifier of a registered resource.
pub fn TypedResourceId(T: type) type {
    return enum(usize) {
        _,

        const Self = @This();
        pub const Resource = T;

        /// Registers a new resource to the universe.
        ///
        /// Registered resources may be instantiated by any world that knows its id.
        pub fn register(provider: anytype, options: RegisterOptions) error{RegisterFailed}!Self {
            const desc = RegisterOptions.Descriptor{
                .next = null,
                .label = if (options.label) |l| l.ptr else null,
                .label_len = if (options.label) |l| l.len else 0,
                .size = @sizeOf(T),
                .alignment = @alignOf(T),
            };

            var id: ResourceId = undefined;
            const sym = symbols.resource_register.requestFrom(provider);
            if (sym(&desc, &id).isErr()) return error.RegisterFailed;
            return fromId(id);
        }

        /// Unregister the resource from the universe.
        ///
        /// Once unregistered, the identifier is invalidated and may be reused by another resouce.
        /// The resource must not be used by any world when this method is called.
        pub fn unregister(self: @This(), provider: anytype) void {
            const sym = symbols.resource_unregister.requestFrom(provider);
            return sym(self.asId());
        }

        /// Casts the typed identifier to an untyped one.
        pub fn asId(self: Self) ResourceId {
            return @enumFromInt(@intFromEnum(self));
        }

        /// Casts the untyped identifier to a typed one.
        pub fn fromId(id: ResourceId) Self {
            return @enumFromInt(@intFromEnum(id));
        }

        /// Checks if the resource is instantiated in the world.
        pub fn existsInWorld(self: Self, provider: anytype, world: *World) bool {
            return world.hasResource(provider, self.asId());
        }

        /// Adds the resource to the world.
        pub fn addToWorld(
            self: Self,
            provider: anytype,
            world: *World,
            value: T,
        ) error{AddFailed}!void {
            return world.addResource(provider, self.asId(), &value);
        }

        /// Removes the resource from the world.
        pub fn removeFromWorld(
            self: Self,
            provider: anytype,
            world: *World,
        ) error{RemoveFailed}!T {
            var value: T = undefined;
            try world.removeResource(provider, self.asId(), &value);
            return value;
        }

        /// Returns an exclusive reference to the resource in the world.
        ///
        /// The caller will block until there are no active shared or exlusive references to the
        /// resource.
        pub fn lockInWorldExclusive(self: Self, provider: anytype, world: *World) *T {
            var out: *anyopaque = undefined;
            world.lockResourcesRaw(provider, &.{self.asId()}, &.{}, (&out)[0..1]);
            return @ptrCast(@alignCast(out));
        }

        /// Returns a shared reference to the resource in the world.
        ///
        /// The caller will block until there are no active exlusive references to the resource.
        pub fn lockInWorldShared(self: Self, provider: anytype, world: *World) *T {
            var out: *anyopaque = undefined;
            world.lockResourcesRaw(provider, &.{}, &.{self.asId()}, (&out)[0..1]);
            return @ptrCast(@alignCast(out));
        }

        /// Unlocks an exclusive resource lock in the world.
        pub fn unlockInWorldExclusive(self: Self, provider: anytype, world: *World) void {
            world.unlockResourceExclusive(provider, self.asId());
        }

        /// Unlocks a shared resource lock in the world.
        pub fn unlockInWorldShared(self: Self, provider: anytype, world: *World) void {
            world.unlockResourceShared(provider, self.asId());
        }
    };
}

test "resource: smoke test" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const id1 = try TypedResourceId(i32).register(GlobalCtx, .{ .label = "resource-1" });
    defer id1.unregister(GlobalCtx);

    const id2 = try TypedResourceId(i32).register(GlobalCtx, .{ .label = "resource-2" });
    defer id2.unregister(GlobalCtx);
    try std.testing.expect(id1 != id2);
}

test "resource: add to world" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const id = try TypedResourceId(i32).register(GlobalCtx, .{ .label = "resource-1" });
    defer id.unregister(GlobalCtx);

    const world = try World.init(GlobalCtx, .{ .label = "test-world" });
    defer world.deinit(GlobalCtx);

    const value: i32 = 5;
    try std.testing.expect(!id.existsInWorld(GlobalCtx, world));
    try id.addToWorld(GlobalCtx, world, value);
    defer _ = id.removeFromWorld(GlobalCtx, world) catch unreachable;
    try std.testing.expect(id.existsInWorld(GlobalCtx, world));

    const ptr = id.lockInWorldExclusive(GlobalCtx, world);
    defer id.unlockInWorldExclusive(GlobalCtx, world);
    try std.testing.expectEqual(value, ptr.*);
}

test "resource: unique lock" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const id = try TypedResourceId(usize).register(GlobalCtx, .{ .label = "resource-1" });
    defer id.unregister(GlobalCtx);

    const world = try World.init(GlobalCtx, .{ .label = "test-world" });
    defer world.deinit(GlobalCtx);

    const executor = world.getPool(GlobalCtx);
    defer executor.unref();

    try id.addToWorld(GlobalCtx, world, 0);
    defer _ = id.removeFromWorld(GlobalCtx, world) catch unreachable;

    const num_jobs = 4;
    const iterations = 1000;

    const Runner = struct {
        id: TypedResourceId(usize),
        world: *World,

        fn run(self: @This()) void {
            for (0..iterations) |_| {
                const ptr = self.id.lockInWorldExclusive(GlobalCtx, self.world);
                defer self.id.unlockInWorldExclusive(GlobalCtx, self.world);
                ptr.* += 1;
            }
        }
    };

    var fences = [_]Fence{.{}} ** num_jobs;
    for (&fences) |*fence| try Job.go(
        GlobalCtx,
        Runner.run,
        .{.{ .id = id, .world = world }},
        .{
            .allocator = std.testing.allocator,
            .executor = executor,
            .signal = .{ .fence = fence },
        },
    );
    for (&fences) |*fence| fence.wait(GlobalCtx);

    const ptr = id.lockInWorldExclusive(GlobalCtx, world);
    defer id.unlockInWorldExclusive(GlobalCtx, world);
    try std.testing.expectEqual(num_jobs * iterations, ptr.*);
}

test "resource: shared lock" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const id = try TypedResourceId(usize).register(GlobalCtx, .{ .label = "resource-1" });
    defer id.unregister(GlobalCtx);

    const world = try World.init(GlobalCtx, .{ .label = "test-world" });
    defer world.deinit(GlobalCtx);

    const executor = world.getPool(GlobalCtx);
    defer executor.unref();

    try id.addToWorld(GlobalCtx, world, 0);
    defer _ = id.removeFromWorld(GlobalCtx, world) catch unreachable;

    const num_writers: usize = 2;
    const num_readers: usize = 4;
    const num_writes: usize = 10000;
    const num_reads: usize = num_writes * 2;

    const Runner = struct {
        world: *World,

        writes: TypedResourceId(usize),
        reads: std.atomic.Value(usize) = std.atomic.Value(usize).init(0),

        term1: usize = 0,
        term2: usize = 0,
        term_sum: usize = 0,

        const Self = @This();

        fn reader(self: *Self) void {
            while (true) {
                const writes = self.writes.lockInWorldShared(GlobalCtx, self.world);
                defer self.writes.unlockInWorldShared(GlobalCtx, self.world);

                if (writes.* >= num_writes or self.reads.load(.unordered) >= num_reads)
                    break;

                self.check();

                _ = self.reads.fetchAdd(1, .monotonic);
            }
        }

        fn writer(self: *Self, thread_idx: usize) void {
            var prng = std.Random.DefaultPrng.init(thread_idx);
            var rnd = prng.random();

            while (true) {
                const writes = self.writes.lockInWorldExclusive(GlobalCtx, self.world);
                defer self.writes.unlockInWorldExclusive(GlobalCtx, self.world);

                if (writes.* >= num_writes)
                    break;

                self.check();

                const term1 = rnd.int(usize);
                self.term1 = term1;

                fimo_tasks_meta.task.yield(GlobalCtx);

                const term2 = rnd.int(usize);
                self.term2 = term2;
                fimo_tasks_meta.task.yield(GlobalCtx);

                self.term_sum = term1 +% term2;
                writes.* += 1;
            }
        }

        fn check(self: *const Self) void {
            const term_sum = self.term_sum;
            fimo_tasks_meta.task.yield(GlobalCtx);

            const term2 = self.term2;
            fimo_tasks_meta.task.yield(GlobalCtx);

            const term1 = self.term1;
            std.testing.expectEqual(term_sum, term1 +% term2) catch unreachable;
        }
    };

    var runner = Runner{ .world = world, .writes = id };
    var fences = [_]Fence{.{}} ** (num_writers + num_readers);

    for (fences[0..num_writers], 0..) |*f, i| try Job.go(
        GlobalCtx,
        Runner.writer,
        .{ &runner, i },
        .{ .allocator = std.testing.allocator, .executor = executor, .signal = .{ .fence = f } },
    );
    for (fences[num_writers..]) |*f| try Job.go(
        GlobalCtx,
        Runner.reader,
        .{&runner},
        .{ .allocator = std.testing.allocator, .executor = executor, .signal = .{ .fence = f } },
    );

    for (&fences) |*fence| fence.wait(GlobalCtx);

    const writes = id.lockInWorldShared(GlobalCtx, world);
    defer id.unlockInWorldShared(GlobalCtx, world);
    try std.testing.expectEqual(num_writes, writes.*);
}
