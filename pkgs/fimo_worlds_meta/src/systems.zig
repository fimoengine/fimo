const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;

const fimo_std = @import("fimo_std");
const Error = fimo_std.ctx.Error;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Pool = fimo_tasks_meta.pool.Pool;

const Job = @import("Job.zig");
const Fence = Job.Fence;
const resources = @import("resources.zig");
const TypedResource = resources.TypedResource;
const Resource = resources.Resource;
const symbols = @import("symbols.zig");
const testing = @import("testing.zig");
const worlds = @import("worlds.zig");
const World = worlds.World;

/// A unique handle to a registered system.
pub const System = opaque {
    /// Unregisters the system from the universe.
    ///
    /// Once unregistered, the identifier is invalidated and may be reused by another system.
    /// The system must not be used explicitly by any world when this method is called.
    pub fn unregister(self: *System) void {
        const sym = symbols.system_unregister.getGlobal().get();
        return sym(self);
    }
};

/// A group of systems that can be scheduled together.
pub const SystemGroup = opaque {
    /// Options for creating a new system group.
    pub const CreateOptions = struct {
        /// Optional label of the group.
        label: ?[]const u8 = null,
        /// Executor for the system group.
        ///
        /// A null value will inherit the executor of the world.
        /// If the value is not null, the system group will increase its reference count.
        pool: ?Pool = null,
        /// World to add the group to.
        world: *World,

        /// Descriptor a a new system group.
        pub const Descriptor = extern struct {
            /// Reserved. Must be null.
            next: ?*const anyopaque,
            /// Optional label of the system group.
            label: ?[*]const u8,
            /// Length in characters of the system group label.
            label_len: usize,
            /// Optional executor for the system group.
            ///
            /// A null value will inherit the executor of the world.
            /// If the value is not null, the system group will increase its reference count.
            pool: ?*const Pool,
            /// World to add the group to.
            world: *World,
        };
    };

    /// Initializes a new empty system group.
    pub fn init(options: CreateOptions) Error!*SystemGroup {
        const desc = CreateOptions.Descriptor{
            .next = null,
            .label = if (options.label) |l| l.ptr else null,
            .label_len = if (options.label) |l| l.len else 0,
            .pool = if (options.pool) |*p| p else null,
            .world = options.world,
        };

        var group: *SystemGroup = undefined;
        const sym = symbols.system_group_create.getGlobal().get();
        try sym(&desc, &group).intoErrorUnion();
        return group;
    }

    /// Destroys the system group.
    ///
    /// The caller is blocked until the group is destroyed. The group may not be running
    /// and must be empty.
    pub fn deinit(self: *SystemGroup) void {
        const sym = symbols.system_group_destroy.getGlobal().get();
        sym(self);
    }

    /// Returns the world the group is contained in.
    pub fn getWorld(self: *SystemGroup) *World {
        const sym = symbols.system_group_get_world.getGlobal().get();
        return sym(self);
    }

    /// Returns the label of the system group.
    pub fn getLabel(self: *SystemGroup) ?[]const u8 {
        var len: usize = undefined;
        const sym = symbols.system_group_get_label.getGlobal().get();
        if (sym(self, &len)) |label| return label[0..len];
        return null;
    }

    /// Returns a reference to the executor used by the group.
    pub fn getPool(self: *SystemGroup) Pool {
        const sym = symbols.system_group_get_pool.getGlobal().get();
        return sym(self);
    }

    /// Adds a set of systems to the group.
    ///
    /// Already scheduled operations are not affected by the added systems.
    /// The operation may add systems transitively, if the systems specify an execution order.
    pub fn addSytems(self: *SystemGroup, systems: []const *System) Error!void {
        const sym = symbols.system_group_add_systems.getGlobal().get();
        try sym(self, systems.ptr, systems.len).intoErrorUnion();
    }

    /// Removes a system from the group.
    ///
    /// Already scheduled systems will not be affected. This operation may remove systems added
    /// transitively. The caller will block until the system is removed from the group.
    pub fn removeSystem(self: *SystemGroup, handle: *System) void {
        var fence = Fence{};
        self.removeSystemAsync(handle, &fence);
        fence.wait();
    }

    /// Removes a system from the group.
    ///
    /// Already scheduled systems will not be affected. This operation may remove systems added
    /// transitively. The caller must provide a reference to a fence via `signal`, to be notified
    /// when the system has been removed from the group.
    pub fn removeSystemAsync(
        self: *SystemGroup,
        handle: *System,
        signal: *Fence,
    ) void {
        const sym = symbols.system_group_remove_system.getGlobal().get();
        return sym(self, handle, signal);
    }

    /// Schedules to run all systems contained in the group.
    ///
    /// The group will start executing after all fences in `wait_on` are signaled.
    /// The caller may provide a reference to a fence via `signal`, to be notified when the group
    /// has finished executing all systems.
    ///
    /// Each schedule operation is assigned to one generation of the system group, which is an index
    /// that is increased by one each time the group finishes executing all systems. Multiple generations
    /// are run sequentially.
    ///
    /// Note that the system group must acquire the resources for the contained systems before executing
    /// them. The manner in which this is accomplished is unspecified. A valid implementation would be
    /// to lock all resources for the entire system group exclusively before starting its execution.
    pub fn schedule(self: *SystemGroup, wait_on: []const *Fence, signal: ?*Fence) Error!void {
        const sym = symbols.system_group_schedule.getGlobal().get();
        try sym(self, wait_on.ptr, wait_on.len, signal).intoErrorUnion();
    }

    /// Convenience function to schedule and wait until the completion of the group.
    ///
    /// The group will start executing after all fences in `wait_on` are signaled.
    pub fn run(self: *SystemGroup, wait_on: []const *Fence) Error!void {
        var fence = Fence{};
        try self.schedule(wait_on, &fence);
        fence.wait();
    }
};

test "SystemGroup: smoke test" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();
    try std.testing.expectEqual(world, group.getWorld());

    const world_ex = world.getPool();
    defer world_ex.unref();
    const group_ex = group.getPool();
    defer group_ex.unref();
    try std.testing.expectEqual(world_ex.id(), group_ex.id());

    const label = group.getLabel().?;
    try std.testing.expectEqualSlices(u8, "test-group", label);
}

test "SystemGroup: custom pool" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const executor = try Pool.init(&.{});
    defer {
        executor.requestClose();
        executor.unref();
    }

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group", .pool = executor });
    defer group.deinit();

    const ex = group.getPool();
    defer ex.unref();
    try std.testing.expectEqual(executor.id(), ex.id());
}

test "SystemGroup: schedule" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const resource = try TypedResource(u32).register(.{ .label = "test-resource" });
    defer resource.unregister();

    try resource.addToWorld(world, 0);
    defer _ = resource.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct {}, shared: struct { a: *u32 }) void {
            _ = ctx;
            _ = exclusive;
            shared.a.* += 1;
        }
    }.run);
    const sys = try Sys.register(.{ .label = "test-system", .shared = .{resource} });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);

    try group.schedule(&.{}, null);
    try group.schedule(&.{}, null);
    try group.schedule(&.{}, null);
    try group.schedule(&.{}, null);
    try group.run(&.{});

    const ptr = resource.lockInWorldShared(world);
    defer resource.unlockInWorldShared(world);
    try std.testing.expectEqual(5, ptr.*);
}

/// Context of an instantiated system in a system group.
pub const SystemContext = opaque {
    /// Returns the group the system is contained in.
    pub fn getGroup(self: *SystemContext) *SystemGroup {
        const sym = symbols.system_context_get_group.getGlobal().get();
        return sym(self);
    }

    /// Returns the current generation of system group.
    ///
    /// The generation is increased by one each time the group finishes executing all systems.
    pub fn getGeneration(self: *SystemContext) usize {
        const sym = symbols.system_context_get_generation.getGlobal().get();
        return sym(self);
    }

    /// Constructs an allocator using some specific (de)allocation strategy.
    ///
    /// Consult the documentation of the individual strategies for additional info.
    pub fn getAllocator(self: *SystemContext, comptime strategy: AllocatorStrategy) SystemAllocator(strategy) {
        return .{ .context = self };
    }

    /// An allocator that is invalidated after the system has finished executing.
    ///
    /// The memory returned by this allocator is only valid in the scope of the run function of the
    /// system for the current group generation. The allocator is not thread-safe.
    pub fn getTransientAllocator(self: *SystemContext) SystemAllocator(.transient) {
        return self.getAllocator(.transient);
    }

    /// An allocator that is invalidated at the end of the current system group generation.
    ///
    /// The allocator may be utilized to spawn short lived tasks from the system, or to pass
    /// data to systems executing after the current one.
    pub fn getSingleGenerationAllocator(self: *SystemContext) SystemAllocator(.single_generation) {
        return self.getAllocator(.single_generation);
    }

    /// An allocator that is invalidated after four generations.
    ///
    /// The allocator may be utilized to spawn medium-to-short lived tasks from the system, or
    /// to pass data to the systems executing in the next generations.
    pub fn getMultiGenerationAllocator(self: *SystemContext) SystemAllocator(.multi_generation) {
        return self.getAllocator(.multi_generation);
    }

    /// An allocator that is invalidated with the system.
    ///
    /// May be utilized for long-lived/persistent allocations.
    pub fn getSystemPersistentAllocator(self: *SystemContext) SystemAllocator(.system_persistent) {
        return self.getAllocator(.system_persistent);
    }
};

/// Known allocator strategies for a system.
pub const AllocatorStrategy = enum(i32) {
    /// An allocator that is invalidated after the system has finished executing.
    ///
    /// The memory returned by this allocator is only valid in the scope of the run function of the
    /// system for the current group generation. The allocator is not thread-safe.
    transient,
    /// An allocator that is invalidated at the end of the current system group generation.
    ///
    /// The allocator may be utilized to spawn short lived tasks from the system, or to pass
    /// data to systems executing after the current one.
    single_generation,
    /// An allocator that is invalidated after four generations.
    ///
    /// The allocator may be utilized to spawn medium-to-short lived tasks from the system, or
    /// to pass data to the systems executing in the next generations.
    multi_generation,
    /// An allocator that is invalidated with the system.
    ///
    /// May be utilized for long-lived/persistent allocations.
    system_persistent,
    _,
};

/// A strategy dependent allocator for a system.
pub fn SystemAllocator(comptime strategy: AllocatorStrategy) type {
    return struct {
        context: *SystemContext,

        const Self = @This();
        const vtable_ref = Allocator.VTable{
            .alloc = &allocRef,
            .resize = &resizeRef,
            .remap = &remapRef,
            .free = &freeRef,
        };
        const vtable = Allocator.VTable{
            .alloc = &alloc,
            .resize = &resize,
            .remap = &remap,
            .free = &free,
        };

        fn allocInner(context: *SystemContext, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
            const sym = symbols.system_context_allocator_alloc.getGlobal().get();
            return sym(context, strategy, len, alignment.toByteUnits(), ret_addr);
        }
        fn allocRef(this: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(this));
            return allocInner(self.context, len, alignment, ret_addr);
        }
        fn alloc(context: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
            const ctx: *SystemContext = @ptrCast(@alignCast(context));
            return allocInner(ctx, len, alignment, ret_addr);
        }

        fn resizeInner(context: *SystemContext, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
            const sym = symbols.system_context_allocator_resize.getGlobal().get();
            return sym(context, strategy, memory.ptr, memory.len, alignment.toByteUnits(), new_len, ret_addr);
        }
        fn resizeRef(this: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
            const self: *Self = @ptrCast(@alignCast(this));
            return resizeInner(self.context, memory, alignment, new_len, ret_addr);
        }
        fn resize(context: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
            const ctx: *SystemContext = @ptrCast(@alignCast(context));
            return resizeInner(ctx, memory, alignment, new_len, ret_addr);
        }

        fn remapInner(
            context: *SystemContext,
            memory: []u8,
            alignment: Alignment,
            new_len: usize,
            ret_addr: usize,
        ) ?[*]u8 {
            const sym = symbols.system_context_allocator_remap.getGlobal().get();
            return sym(context, strategy, memory.ptr, memory.len, alignment.toByteUnits(), new_len, ret_addr);
        }
        fn remapRef(
            this: *anyopaque,
            memory: []u8,
            alignment: Alignment,
            new_len: usize,
            ret_addr: usize,
        ) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(this));
            return remapInner(self.context, memory, alignment, new_len, ret_addr);
        }
        fn remap(context: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
            const ctx: *SystemContext = @ptrCast(@alignCast(context));
            return remapInner(ctx, memory, alignment, new_len, ret_addr);
        }

        fn freeInner(context: *SystemContext, memory: []u8, alignment: Alignment, ret_addr: usize) void {
            const sym = symbols.system_context_allocator_free.getGlobal().get();
            sym(context, strategy, memory.ptr, memory.len, alignment.toByteUnits(), ret_addr);
        }
        fn freeRef(this: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
            const self: *Self = @ptrCast(@alignCast(this));
            return freeInner(self.context, memory, alignment, ret_addr);
        }
        fn free(context: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
            const ctx: *SystemContext = @ptrCast(@alignCast(context));
            return freeInner(ctx, memory, alignment, ret_addr);
        }

        pub fn allocatorRef(self: *Self) Allocator {
            return .{ .ptr = self, .vtable = &vtable_ref };
        }

        pub fn allocator(self: Self) Allocator {
            return .{ .ptr = self.context, .vtable = &vtable };
        }
    };
}

test "SystemContext: group" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const group_handle = try TypedResource(*SystemGroup).register(.{ .label = "group" });
    defer group_handle.unregister();
    try group_handle.addToWorld(world, undefined);
    defer _ = group_handle.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct { group: **SystemGroup }, shared: struct {}) void {
            _ = shared;
            exclusive.group.* = ctx.getGroup();
        }
    }.run);
    const sys = try Sys.register(.{ .label = "test-system", .exclusive = .{group_handle} });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);
    try group.run(&.{});

    const ptr = group_handle.lockInWorldExclusive(world);
    defer group_handle.unlockInWorldExclusive(world);
    try std.testing.expectEqual(group, ptr.*);
}

test "SystemContext: generation" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const error_handle = try TypedResource(?anyerror).register(.{ .label = "error" });
    defer error_handle.unregister();
    try error_handle.addToWorld(world, null);
    defer _ = error_handle.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initSimple(struct {
        ctx: *SystemContext,
        generation: ?usize = null,

        pub fn init(context: *SystemContext) !@This() {
            return .{ .ctx = context };
        }
        pub fn run(self: *@This(), exclusive: struct { err: *?anyerror }, shared: struct {}) void {
            _ = shared;
            self.runTest() catch |err| {
                exclusive.err.* = err;
            };
        }
        fn runTest(self: *@This()) !void {
            const generation = self.ctx.getGeneration();
            if (self.generation) |gen| try std.testing.expect(gen + 1 == generation);
            self.generation = generation;
        }
    });
    const sys = try Sys.register(.{ .label = "test-system", .exclusive = .{error_handle} });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);

    for (0..10) |_| try group.schedule(&.{}, null);
    try group.run(&.{});

    const ptr = error_handle.lockInWorldExclusive(world);
    defer error_handle.unlockInWorldExclusive(world);
    if (ptr.*) |err| return err;
}

test "SystemContext: deferred" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const error_handle = try TypedResource(?anyerror).register(.{ .label = "error" });
    defer error_handle.unregister();
    try error_handle.addToWorld(world, null);
    defer _ = error_handle.removeFromWorld(world) catch unreachable;

    const completed_handle = try TypedResource(bool).register(.{ .label = "completed" });
    defer completed_handle.unregister();
    try completed_handle.addToWorld(world, false);
    defer _ = completed_handle.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initFunctor(struct {
        fn run(
            ctx: *SystemContext,
            exclusive: struct { err: *?anyerror, completed: *bool },
            shared: struct {},
            fence: *Fence,
        ) void {
            _ = shared;
            const executor = ctx.getGroup().getPool();
            defer executor.unref();
            const allocator = ctx.getSingleGenerationAllocator().allocator();
            Job.go(runTest, .{exclusive.completed}, .{
                .executor = executor,
                .allocator = allocator,
                .signal = .{ .fence = fence },
            }) catch |err| {
                exclusive.err.* = err;
            };
        }
        fn runTest(completed: *bool) void {
            completed.* = true;
        }
    }.run);
    const sys = try Sys.register(.{
        .label = "test-system",
        .exclusive = .{ error_handle, completed_handle },
    });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);

    for (0..10) |_| try group.schedule(&.{}, null);
    try group.run(&.{});

    {
        const ptr = error_handle.lockInWorldExclusive(world);
        defer error_handle.unlockInWorldExclusive(world);
        if (ptr.*) |err| return err;
    }

    {
        const ptr = completed_handle.lockInWorldExclusive(world);
        defer completed_handle.unlockInWorldExclusive(world);
        try std.testing.expect(ptr.*);
    }
}

test "SystemContext: transient allocator" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const error_handle = try TypedResource(?anyerror).register(.{ .label = "error" });
    defer error_handle.unregister();
    try error_handle.addToWorld(world, null);
    defer _ = error_handle.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct { err: *?anyerror }, shared: struct {}) void {
            _ = shared;
            testAlloc(ctx) catch |err| {
                exclusive.err.* = err;
            };
        }
        fn testAlloc(ctx: *SystemContext) !void {
            const allocator = ctx.getTransientAllocator().allocator();
            try std.heap.testAllocator(allocator);
            try std.heap.testAllocatorAligned(allocator);
            try std.heap.testAllocatorLargeAlignment(allocator);
            try std.heap.testAllocatorAlignedShrink(allocator);
        }
    }.run);
    const sys = try Sys.register(.{ .label = "test-system", .exclusive = .{error_handle} });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);
    try group.run(&.{});

    const ptr = error_handle.lockInWorldExclusive(world);
    defer error_handle.unlockInWorldExclusive(world);
    if (ptr.*) |err| return err;
}

test "SystemContext: single generation allocator" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const error_handle = try TypedResource(?anyerror).register(.{ .label = "error" });
    defer error_handle.unregister();
    try error_handle.addToWorld(world, null);
    defer _ = error_handle.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct { err: *?anyerror }, shared: struct {}) void {
            _ = shared;
            testAlloc(ctx) catch |err| {
                exclusive.err.* = err;
            };
        }
        fn testAlloc(ctx: *SystemContext) !void {
            const allocator = ctx.getSingleGenerationAllocator().allocator();
            try std.heap.testAllocator(allocator);
            try std.heap.testAllocatorAligned(allocator);
            try std.heap.testAllocatorLargeAlignment(allocator);
            try std.heap.testAllocatorAlignedShrink(allocator);
        }
    }.run);
    const sys = try Sys.register(.{ .label = "test-system", .exclusive = .{error_handle} });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);
    try group.run(&.{});

    const ptr = error_handle.lockInWorldExclusive(world);
    defer error_handle.unlockInWorldExclusive(world);
    if (ptr.*) |err| return err;
}

test "SystemContext: multi generation allocator" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const error_handle = try TypedResource(?anyerror).register(.{ .label = "error" });
    defer error_handle.unregister();
    try error_handle.addToWorld(world, null);
    defer _ = error_handle.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct { err: *?anyerror }, shared: struct {}) void {
            _ = shared;
            testAlloc(ctx) catch |err| {
                exclusive.err.* = err;
            };
        }
        fn testAlloc(ctx: *SystemContext) !void {
            const allocator = ctx.getMultiGenerationAllocator().allocator();
            try std.heap.testAllocator(allocator);
            try std.heap.testAllocatorAligned(allocator);
            try std.heap.testAllocatorLargeAlignment(allocator);
            try std.heap.testAllocatorAlignedShrink(allocator);
        }
    }.run);
    const sys = try Sys.register(.{ .label = "test-system", .exclusive = .{error_handle} });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);
    try group.run(&.{});

    const ptr = error_handle.lockInWorldExclusive(world);
    defer error_handle.unlockInWorldExclusive(world);
    if (ptr.*) |err| return err;
}

test "SystemContext: persistent allocator" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const group = try world.addSystemGroup(.{ .label = "test-group" });
    defer group.deinit();

    const error_handle = try TypedResource(?anyerror).register(.{ .label = "error" });
    defer error_handle.unregister();
    try error_handle.addToWorld(world, null);
    defer _ = error_handle.removeFromWorld(world) catch unreachable;

    const Sys = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct { err: *?anyerror }, shared: struct {}) void {
            _ = shared;
            testAlloc(ctx) catch |err| {
                exclusive.err.* = err;
            };
        }
        fn testAlloc(ctx: *SystemContext) !void {
            const allocator = ctx.getSystemPersistentAllocator().allocator();
            try std.heap.testAllocator(allocator);
            try std.heap.testAllocatorAligned(allocator);
            try std.heap.testAllocatorLargeAlignment(allocator);
            try std.heap.testAllocatorAlignedShrink(allocator);
        }
    }.run);
    const sys = try Sys.register(.{ .label = "test-system", .exclusive = .{error_handle} });
    defer sys.unregister();

    try group.addSytems(&.{sys});
    defer group.removeSystem(sys);
    try group.run(&.{});

    const ptr = error_handle.lockInWorldExclusive(world);
    defer error_handle.unlockInWorldExclusive(world);
    if (ptr.*) |err| return err;
}

/// Interface of a system.
pub const Declaration = struct {
    Options: type,
    Factory: type,
    System: type,
    deinit_factory: ?*const fn (factory: ?*const anyopaque) callconv(.c) void,

    ExclusiveResourceHandlesT: type,
    SharedResourceHandlesT: type,
    init: *const fn (
        factory: ?*const anyopaque,
        context: *SystemContext,
        system: ?*anyopaque,
    ) callconv(.c) bool,
    deinit: ?*const fn (system: ?*anyopaque) callconv(.c) void,
    run: *const fn (
        system: ?*anyopaque,
        unique_resources: ?[*]const *anyopaque,
        shared_resources: ?[*]const *anyopaque,
        deferred_fence: *Fence,
    ) callconv(.c) void,

    /// Flags for a system dependency.
    pub const Flags = packed struct(usize) {
        /// Whether to treat the dependency as a weak dependency.
        ///
        /// Weak dependencies impose order constraints on the system scheduler, but
        /// don't force the inclusion of the dependency. In other words, a weak
        /// dependency can be thought of as an optional dependency.
        weak: bool = false,
        /// Whether to ignore any deferred subjob of the system.
        ///
        /// If set to `true`, the system will start after the other systems `run`
        /// function is run to completion. Otherwise, the system will start after
        /// all subjobs of the system also complete their execution.
        ignore_deferred: bool = false,
        reserved: @Type(.{ .int = .{
            .bits = @bitSizeOf(usize) - 2,
            .signedness = .unsigned,
        } }) = undefined,
    };

    /// Descriptor of a system dependency.
    pub const Dependency = extern struct {
        /// System to depend on / be depended from.
        system: *System,
        /// Options of the dependency.
        flags: Flags = .{},
    };

    /// Descriptor of a new system.
    pub const Descriptor = extern struct {
        /// Reserved. Must be null.
        next: ?*const anyopaque,
        /// Optional label of the system.
        label: ?[*]const u8,
        /// Length in characters of the system label.
        label_len: usize,
        /// Optional array of resources to require with exclusive access.
        exclusive_handles: ?[*]const *Resource,
        /// Length of the `exclusive_handles` array.
        exclusive_handles_len: usize,
        /// Optional array of resources to require with shared access.
        shared_handles: ?[*]const *Resource,
        /// Length of the `shared_handles` array.
        shared_handles_len: usize,
        /// Optional array of systems to depend on.
        ///
        /// The system will start executing after all systems have been executed.
        before: ?[*]const Dependency,
        /// Length of the `before` array.
        before_len: usize,
        /// Optional array of systems to be depended from.
        ///
        /// The systems will start executing after the new system completes its execution.
        after: ?[*]const Dependency,
        /// Length of the `after` array.
        after_len: usize,

        /// Pointer to the factory for the system.
        ///
        /// The factory will be copied into the universe.
        factory: ?*const anyopaque,
        /// Size in bytes of the factory.
        factory_size: usize,
        /// Alignment in bytes of the factory. Must be a power-of-two.
        factory_alignment: usize,
        /// Optional function to call when destroying the factory.
        factory_deinit: ?*const fn (factory: ?*const anyopaque) callconv(.c) void,

        /// Size in bytes of the system state.
        system_size: usize,
        /// Alignment in bytes of the system state. Must be a power-of-two.
        system_alignment: usize,
        /// Function called when instantiating a new system.
        ///
        /// The system is provided with a system context, that shares the same lifetime,
        /// as the system itself. The context provides additional utilities, like allocators.
        /// The state of the system must be written into the provided `system` pointer.
        /// On success, the function must return true.
        system_init: *const fn (
            factory: ?*const anyopaque,
            context: *SystemContext,
            system: ?*anyopaque,
        ) callconv(.c) bool,
        /// Optional function to call when destroying a system.
        system_deinit: ?*const fn (system: ?*anyopaque) callconv(.c) void,
        /// Function called on each system run.
        ///
        /// The requested exclusive and shared resources are provided in the order defined by
        /// the `exclusive_ids` and `shared_ids`. Additionally, the system is provided with a
        /// pointer to an unsignaled fence. The fence may be used to spawn additional jobs from
        /// within the run function and synchronize other systems waiting on the completion of
        /// the current system. The system must signal the fence after it has completed. Failure
        /// of doing such will lead to a deadlock. The fence is guaranteed to not have any waiters
        /// until after the run function returns.
        system_run: *const fn (
            system: ?*anyopaque,
            unique_resources: ?[*]const *anyopaque,
            shared_resources: ?[*]const *anyopaque,
            deferred_signal: *Fence,
        ) callconv(.c) void,
    };

    /// Registers a new system with the universe.
    ///
    /// Registered resources may be added to system group of any world.
    pub fn register(comptime self: Declaration, options: self.Options) Error!*System {
        const exclusive = if (@hasField(self.Options, "exclusive")) blk: {
            var arr: [options.exclusive.len]*Resource = undefined;
            inline for (0..arr.len) |i| arr[i] = options.exclusive[i].asUntyped();
            break :blk arr;
        } else @as([0]*Resource, .{});
        const shared = if (@hasField(self.Options, "shared")) blk: {
            var arr: [options.shared.len]*Resource = undefined;
            inline for (0..arr.len) |i| arr[i] = options.shared[i].asUntyped();
            break :blk arr;
        } else @as([0]*Resource, .{});

        const desc = Descriptor{
            .next = null,
            .label = if (options.label) |l| l.ptr else null,
            .label_len = if (options.label) |l| l.len else 0,
            .exclusive_handles = exclusive[0..].ptr,
            .exclusive_handles_len = exclusive.len,
            .shared_handles = shared[0..].ptr,
            .shared_handles_len = shared.len,
            .before = options.before.ptr,
            .before_len = options.before.len,
            .after = options.after.ptr,
            .after_len = options.after.len,

            .factory = if (@hasField(self.Options, "factory")) &options.factory else null,
            .factory_size = @sizeOf(self.Factory),
            .factory_alignment = @alignOf(self.Factory),
            .factory_deinit = self.deinit_factory,

            .system_size = @sizeOf(self.System),
            .system_alignment = @alignOf(self.System),
            .system_init = self.init,
            .system_deinit = self.deinit,
            .system_run = self.run,
        };

        var handle: *System = undefined;
        const sym = symbols.system_register.getGlobal().get();
        try sym(&desc, &handle).intoErrorUnion();
        return handle;
    }

    /// A simple system using only a stateless function.
    pub fn initFunctor(comptime runFn: anytype) Declaration {
        const run_info = @typeInfo(@TypeOf(runFn)).@"fn";
        if (run_info.params.len != 3 and run_info.params.len != 4)
            @compileError("The system run function must take three or four parameters.");
        const ExclusiveArgs = run_info.params[1].type.?;
        const SharedArgs = run_info.params[2].type.?;
        const has_signal = run_info.params.len == 4;

        const Wrapper = struct {
            context: *SystemContext,

            pub fn init(context: *SystemContext) !@This() {
                return .{ .context = context };
            }

            pub fn run(self: *@This(), exclusive: ExclusiveArgs, shared: SharedArgs, deferred_fence: *Fence) void {
                if (comptime has_signal) {
                    runFn(self.context, exclusive, shared, deferred_fence);
                } else {
                    runFn(self.context, exclusive, shared);
                    deferred_fence.state.store(Fence.signaled, .release);
                }
            }
        };
        return initSimple(Wrapper);
    }

    /// A system with custom initialization and deinitialization logic.
    pub fn initSimple(T: type) Declaration {
        if (!@hasDecl(T, "run")) @compileError(@typeName(T) ++ " does not contain a `run` function");
        const run_info = @typeInfo(@TypeOf(T.run)).@"fn";
        if (run_info.params.len != 3 and run_info.params.len != 4)
            @compileError("The system run function must take three or four parameters.");
        if (run_info.params[0].type.? != *T)
            @compileError("The first argument of the run function must be a pointer to " ++ @typeName(T));

        const ExclusiveArgs = run_info.params[1].type.?;
        const SharedArgs = run_info.params[2].type.?;
        const has_signal = run_info.params.len == 4;

        const Factory = struct {
            const F = @This();
            pub const System = struct {
                sys: T,

                const S = @This();

                pub fn init(factory: *const F, context: *SystemContext) !S {
                    _ = factory;
                    return .{ .sys = try T.init(context) };
                }

                pub const deinit = if (std.meta.hasFn(T, "deinit"))
                    struct {
                        fn f(self: *S) void {
                            self.sys.deinit();
                        }
                    }.f
                else
                    undefined;

                pub fn run(
                    self: *S,
                    exclusive: ExclusiveArgs,
                    shared: SharedArgs,
                    deferred_fence: *Fence,
                ) void {
                    if (comptime has_signal) {
                        self.sys.run(exclusive, shared, deferred_fence);
                    } else {
                        self.sys.run(exclusive, shared);
                        deferred_fence.state.store(Fence.signaled, .release);
                    }
                }
            };
        };
        return initFactory(Factory);
    }

    /// A system with a custom system factory.
    pub fn initFactory(T: type) Declaration {
        if (!@hasDecl(T, "System") or @TypeOf(T.System) != type)
            @compileError(@typeName(T) ++ " does not define a `System` type");

        const Sys = T.System;
        if (!@hasDecl(Sys, "run")) @compileError(@typeName(Sys) ++ " does not contain a `run` function");
        const run_info = @typeInfo(@TypeOf(Sys.run)).@"fn";
        if (run_info.params.len != 3 and run_info.params.len != 4)
            @compileError("The system run function must take three or four parameters.");
        if (run_info.params[0].type.? != *Sys)
            @compileError("The first argument of the run function must be a pointer to " ++ @typeName(Sys));

        const ExclusiveArgs = run_info.params[1].type.?;
        const SharedArgs = run_info.params[2].type.?;
        const has_signal = run_info.params.len == 4;

        const ExclusiveResourceHandles = if (@sizeOf(ExclusiveArgs) == 0) std.meta.Tuple(&.{}) else blk: {
            const fields = @typeInfo(ExclusiveArgs).@"struct".fields;
            var types: [fields.len]type = undefined;
            for (0..fields.len) |i| {
                const F = fields[i].type;
                const ResourceValue = @typeInfo(F).pointer.child;
                if (F != *ResourceValue) @compileError("Resource parameters must be non-const pointers");
                types[i] = *TypedResource(ResourceValue);
            }
            break :blk std.meta.Tuple(&types);
        };

        const SharedResourceHandles = if (@sizeOf(SharedArgs) == 0) std.meta.Tuple(&.{}) else blk: {
            const fields = @typeInfo(SharedArgs).@"struct".fields;
            var types: [fields.len]type = undefined;
            for (0..fields.len) |i| {
                const F = fields[i].type;
                const ResourceValue = @typeInfo(F).pointer.child;
                if (F != *ResourceValue) @compileError("Resource parameters must be non-const pointers");
                types[i] = *TypedResource(ResourceValue);
            }
            break :blk std.meta.Tuple(&types);
        };

        const Options = blk: {
            var fields_arr: [6]std.builtin.Type.StructField = undefined;
            var fields = std.ArrayListUnmanaged(std.builtin.Type.StructField).initBuffer(&fields_arr);
            const default_dependency_slice: []const Dependency = &.{};
            fields.appendAssumeCapacity(.{
                .name = "label",
                .type = ?[]const u8,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(?[]const u8),
            });
            if (@sizeOf(T) != 0) fields.appendAssumeCapacity(.{
                .name = "factory",
                .type = T,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(T),
            });
            if (@sizeOf(ExclusiveResourceHandles) != 0) fields.appendAssumeCapacity(.{
                .name = "exclusive",
                .type = ExclusiveResourceHandles,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(ExclusiveResourceHandles),
            });
            if (@sizeOf(SharedResourceHandles) != 0) fields.appendAssumeCapacity(.{
                .name = "shared",
                .type = SharedResourceHandles,
                .default_value_ptr = null,
                .is_comptime = false,
                .alignment = @alignOf(SharedResourceHandles),
            });
            fields.appendAssumeCapacity(.{
                .name = "before",
                .type = []const Dependency,
                .default_value_ptr = @ptrCast(&default_dependency_slice),
                .is_comptime = false,
                .alignment = @alignOf([]const Dependency),
            });
            fields.appendAssumeCapacity(.{
                .name = "after",
                .type = []const Dependency,
                .default_value_ptr = @ptrCast(&default_dependency_slice),
                .is_comptime = false,
                .alignment = @alignOf([]const Dependency),
            });

            break :blk @Type(.{ .@"struct" = .{
                .layout = .auto,
                .decls = &.{},
                .fields = fields.items,
                .is_tuple = false,
            } });
        };

        const Wrapper = struct {
            const deinitFactory = if (std.meta.hasFn(T, "deinit")) struct {
                fn f(factory: ?*const anyopaque) callconv(.c) void {
                    const fact: *const T = if (comptime @sizeOf(T) != 0)
                        @ptrCast(@alignCast(factory))
                    else
                        &T{};
                    fact.deinit();
                }
            }.f else undefined;

            fn initSys(
                factory: ?*const anyopaque,
                context: *SystemContext,
                system: ?*anyopaque,
            ) callconv(.c) bool {
                const fact: *const T = if (comptime @sizeOf(T) != 0)
                    @ptrCast(@alignCast(factory))
                else
                    &T{};
                const sys: *Sys = if (comptime @sizeOf(Sys) != 0)
                    @ptrCast(@alignCast(system))
                else
                    &Sys{};
                sys.* = Sys.init(fact, context) catch return false;
                return true;
            }

            const deinitSys = if (std.meta.hasFn(Sys, "deinit")) struct {
                fn f(system: ?*anyopaque) callconv(.c) void {
                    const sys: *Sys = if (comptime @sizeOf(Sys) != 0)
                        @ptrCast(@alignCast(system))
                    else
                        &Sys{};
                    sys.deinit();
                }
            }.f else undefined;

            fn runSys(
                system: ?*anyopaque,
                exclusive_resources: ?[*]const *anyopaque,
                shared_resources: ?[*]const *anyopaque,
                deferred_fence: *Fence,
            ) callconv(.c) void {
                const sys: *Sys = if (comptime @sizeOf(Sys) != 0)
                    @ptrCast(@alignCast(system))
                else
                    &Sys{};
                var exclusive: ExclusiveArgs = undefined;
                if (comptime @sizeOf(ExclusiveArgs) == 0) exclusive = ExclusiveArgs{} else {
                    const arr = exclusive_resources.?;
                    const fields = std.meta.fields(ExclusiveArgs);
                    inline for (fields, 0..) |field, i| {
                        @field(exclusive, field.name) = @ptrCast(@alignCast(arr[i]));
                    }
                }
                var shared: SharedArgs = undefined;
                if (comptime @sizeOf(SharedArgs) == 0) shared = SharedArgs{} else {
                    const arr = shared_resources.?;
                    const fields = std.meta.fields(SharedArgs);
                    inline for (fields, 0..) |field, i| {
                        @field(shared, field.name) = @ptrCast(@alignCast(arr[i]));
                    }
                }

                if (comptime has_signal) {
                    sys.run(exclusive, shared, deferred_fence);
                } else {
                    sys.run(exclusive, shared);
                    deferred_fence.state.store(Fence.signaled, .release);
                }
            }
        };

        return .{
            .Options = Options,
            .Factory = T,
            .deinit_factory = if (std.meta.hasFn(Wrapper, "deinitFactory")) &Wrapper.deinitFactory else null,
            .System = Sys,
            .ExclusiveResourceHandlesT = ExclusiveResourceHandles,
            .SharedResourceHandlesT = SharedResourceHandles,
            .init = &Wrapper.initSys,
            .deinit = if (std.meta.hasFn(Wrapper, "deinitSys")) &Wrapper.deinitSys else null,
            .run = &Wrapper.runSys,
        };
    }
};

test "System: system definitions" {
    const TupleTester = struct {
        fn assertTuple(comptime expected: anytype, comptime Actual: type) void {
            const info = @typeInfo(Actual);
            if (info != .@"struct")
                @compileError("Expected struct type");
            if (!info.@"struct".is_tuple)
                @compileError("Struct type must be a tuple type");

            const fields_list = std.meta.fields(Actual);
            if (expected.len != fields_list.len)
                @compileError("Argument count mismatch");

            inline for (fields_list, 0..) |fld, i| {
                if (expected[i] != fld.type) {
                    @compileError("Field " ++ fld.name ++ " expected to be type " ++ @typeName(expected[i]) ++ ", but was type " ++ @typeName(fld.type));
                }
            }
        }
    };

    const Dummy = struct {
        fn simple0(ctx: *SystemContext, exclusive: struct {}, shared: struct {}) void {
            _ = ctx;
            _ = exclusive;
            _ = shared;
        }
        fn simple1(ctx: *SystemContext, exclusive: struct { *i32, *u32 }, shared: struct {}) void {
            _ = ctx;
            _ = exclusive;
            _ = shared;
        }
        fn simple2(ctx: *SystemContext, exclusive: struct {}, shared: struct { *i32, *u32 }) void {
            _ = ctx;
            _ = exclusive;
            _ = shared;
        }
        fn simple3(ctx: *SystemContext, exclusive: struct { *i32, *u32 }, shared: struct { *f32 }) void {
            _ = ctx;
            _ = exclusive;
            _ = shared;
        }
    };

    const Simple0 = Declaration.initFunctor(Dummy.simple0);
    TupleTester.assertTuple(.{}, Simple0.ExclusiveResourceHandlesT);
    TupleTester.assertTuple(.{}, Simple0.SharedResourceHandlesT);

    const Simple1 = Declaration.initFunctor(Dummy.simple1);
    TupleTester.assertTuple(.{ *TypedResource(i32), *TypedResource(u32) }, Simple1.ExclusiveResourceHandlesT);
    TupleTester.assertTuple(.{}, Simple1.SharedResourceHandlesT);

    const Simple2 = Declaration.initFunctor(Dummy.simple2);
    TupleTester.assertTuple(.{}, Simple2.ExclusiveResourceHandlesT);
    TupleTester.assertTuple(.{ *TypedResource(i32), *TypedResource(u32) }, Simple2.SharedResourceHandlesT);

    const Simple3 = Declaration.initFunctor(Dummy.simple3);
    TupleTester.assertTuple(.{ *TypedResource(i32), *TypedResource(u32) }, Simple3.ExclusiveResourceHandlesT);
    TupleTester.assertTuple(.{*TypedResource(f32)}, Simple3.SharedResourceHandlesT);
}

test "System: smoke test" {
    const Dummy = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct {}, shared: struct {}) void {
            _ = ctx;
            _ = exclusive;
            _ = shared;
        }
    }.run);

    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const sys0 = try Dummy.register(.{ .label = "system-0" });
    defer sys0.unregister();
}

test "System: cyclic dependency" {
    const Dummy = Declaration.initFunctor(struct {
        fn run(ctx: *SystemContext, exclusive: struct {}, shared: struct {}) void {
            _ = ctx;
            _ = exclusive;
            _ = shared;
        }
    }.run);

    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const sys0 = try Dummy.register(.{ .label = "system-0" });
    defer sys0.unregister();

    const sys1 = try Dummy.register(.{ .label = "system-1", .after = &.{.{ .system = sys0 }} });
    defer sys1.unregister();

    const sys2 = Dummy.register(.{
        .label = "system-1",
        .before = &.{.{ .system = sys0 }},
        .after = &.{.{ .system = sys1 }},
    }) catch return;
    sys2.unregister();
    try std.testing.expect(false);
}
