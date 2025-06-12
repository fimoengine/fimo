const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Pool = fimo_tasks_meta.pool.Pool;

const resources = @import("resources.zig");
const ResourceId = resources.ResourceId;
const symbols = @import("symbols.zig");
const systems = @import("systems.zig");
const SystemGroup = systems.SystemGroup;
const testing = @import("testing.zig");

/// Options for creating a new world.
pub const CreateOptions = struct {
    /// Optional label of the world.
    label: ?[]const u8 = null,
    /// Executor for the world.
    ///
    /// If this value is `null`, the world will spawn a default executor.
    /// If the value is not null, the world will increase its reference count.
    pool: ?Pool = null,

    /// Descriptor of a new world.
    pub const Descriptor = extern struct {
        /// Reserved. Must be null.
        next: ?*const anyopaque,
        /// Optional label of the world.
        label: ?[*]const u8,
        /// Length in characters of the world label.
        label_len: usize,
        /// Executor for the world.
        ///
        /// If this value is `null`, the world will spawn a default executor.
        /// If the value is not null, the world will increase its reference count.
        pool: ?*const Pool,
    };
};

/// Options for adding a new system group to a world.
pub const AddSystemGroupOptions = struct {
    /// Optional label of the group.
    label: ?[]const u8 = null,
    /// Executor for the system group.
    ///
    /// A default value will inherit the executor of the world.
    pool: ?Pool = null,
};

/// A container for resources and scheduable systems.
pub const World = opaque {
    /// Initializes a new empty world.
    pub fn init(provider: anytype, options: CreateOptions) error{InitFailed}!*World {
        const desc = CreateOptions.Descriptor{
            .next = null,
            .label = if (options.label) |l| l.ptr else null,
            .label_len = if (options.label) |l| l.len else 0,
            .pool = if (options.pool) |*p| p else null,
        };

        var world: *World = undefined;
        const sym = symbols.world_create.requestFrom(provider);
        return switch (sym(&desc, &world)) {
            .Ok => world,
            .OperationFailed => error.InitFailed,
            else => unreachable,
        };
    }

    /// Destroys the world.
    ///
    /// The world must be empty.
    pub fn deinit(self: *World, provider: anytype) void {
        const sym = symbols.world_destroy.requestFrom(provider);
        return sym(self);
    }

    /// Returns the label of the world.
    pub fn getLabel(self: *World, provider: anytype) ?[]const u8 {
        var len: usize = undefined;
        const sym = symbols.world_get_label.requestFrom(provider);
        if (sym(self, &len)) |label| return label[0..len];
        return null;
    }

    /// Returns a reference to the executor used by the world.
    pub fn getPool(self: *World, provider: anytype) Pool {
        const sym = symbols.world_get_pool.requestFrom(provider);
        return sym(self);
    }

    /// Checks if the resource is instantiated in the world.
    pub fn hasResource(self: *World, provider: anytype, id: ResourceId) bool {
        const sym = symbols.world_has_resource.requestFrom(provider);
        return sym(self, id);
    }

    /// Adds the resource to the world.
    pub fn addResource(
        self: *World,
        provider: anytype,
        id: ResourceId,
        value: *const anyopaque,
    ) error{AddFailed}!void {
        const sym = symbols.world_add_resource.requestFrom(provider);
        return switch (sym(self, id, value)) {
            .Ok => {},
            .OperationFailed => error.AddFailed,
            else => unreachable,
        };
    }

    /// Removes the resource from the world.
    pub fn removeResource(
        self: *World,
        provider: anytype,
        id: ResourceId,
        value: *anyopaque,
    ) error{RemoveFailed}!void {
        const sym = symbols.world_remove_resource.requestFrom(provider);
        return switch (sym(self, id, value)) {
            .Ok => {},
            .OperationFailed => error.RemoveFailed,
            else => unreachable,
        };
    }

    /// Acquires a set of exclusive and shared resource references.
    ///
    /// The pointers to the resources are written into `out_resources`, where the indices
    /// `0..exclusive_ids_len` contain the resources in the `exclusive_ids` list, while the
    /// indices `exclusive_ids.len..exclusive_ids.len+shared_ids.len` contain the remaining
    /// resources from the `shared_ids` list.
    ///
    /// The locks to the resources are acquired in increasing resource id order.
    /// The caller will block until all resources are locked.
    pub fn lockResourcesRaw(
        self: *World,
        provider: anytype,
        exclusive_ids: []const ResourceId,
        shared_ids: []const ResourceId,
        out_resources: []*anyopaque,
    ) void {
        std.debug.assert(exclusive_ids.len + shared_ids.len <= out_resources.len);
        const sym = symbols.world_lock_resources.requestFrom(provider);
        sym(
            self,
            exclusive_ids.ptr,
            exclusive_ids.len,
            shared_ids.ptr,
            shared_ids.len,
            out_resources.ptr,
        );
    }

    /// Unlocks an exclusive resource lock.
    pub fn unlockResourceExclusive(self: *World, provider: anytype, id: ResourceId) void {
        const sym = symbols.world_unlock_resource_exclusive.requestFrom(provider);
        return sym(self, id);
    }

    /// Unlocks a shared resource lock.
    pub fn unlockResourceShared(self: *World, provider: anytype, id: ResourceId) void {
        const sym = symbols.world_unlock_resource_shared.requestFrom(provider);
        return sym(self, id);
    }

    /// Adds a new empty system group to the world.
    pub fn addSystemGroup(
        self: *World,
        provider: anytype,
        options: AddSystemGroupOptions,
    ) error{AddFailed}!*SystemGroup {
        return SystemGroup.init(provider, .{
            .label = options.label,
            .pool = options.pool,
            .world = self,
        }) catch error.AddFailed;
    }

    /// Returns the allocator for the world.
    pub fn getAllocator(self: *World, provider: anytype) WorldAllocator(@TypeOf(provider)) {
        return .{ .world = self, .provider = provider };
    }
};

/// An allocator that clears its memory upon destruction of the owning world.
pub fn WorldAllocator(Provider: type) type {
    return struct {
        world: *World,
        provider: Provider,

        const Self = @This();
        const vtable = Allocator.VTable{
            .alloc = &alloc,
            .resize = &resize,
            .remap = &remap,
            .free = &free,
        };

        fn alloc(this: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.world_allocator_alloc.requestFrom(self.provider);
            return sym(self.world, len, alignment.toByteUnits(), ret_addr);
        }

        fn resize(
            this: *anyopaque,
            memory: []u8,
            alignment: Alignment,
            new_len: usize,
            ret_addr: usize,
        ) bool {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.world_allocator_resize.requestFrom(self.provider);
            return sym(
                self.world,
                memory.ptr,
                memory.len,
                alignment.toByteUnits(),
                new_len,
                ret_addr,
            );
        }

        fn remap(
            this: *anyopaque,
            memory: []u8,
            alignment: Alignment,
            new_len: usize,
            ret_addr: usize,
        ) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.world_allocator_remap.requestFrom(self.provider);
            return sym(
                self.world,
                memory.ptr,
                memory.len,
                alignment.toByteUnits(),
                new_len,
                ret_addr,
            );
        }

        fn free(this: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.world_allocator_free.requestFrom(self.provider);
            sym(
                self.world,
                memory.ptr,
                memory.len,
                alignment.toByteUnits(),
                ret_addr,
            );
        }

        pub fn allocator(self: *Self) Allocator {
            return .{ .ptr = self, .vtable = &vtable };
        }
    };
}

test "world: smoke test" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const world = try World.init(&ctx, .{ .label = "test-world" });
    defer world.deinit(&ctx);

    const label = world.getLabel(&ctx).?;
    try std.testing.expectEqualSlices(u8, "test-world", label);
}

test "world: custom pool" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    var err: ?AnyError = null;
    defer if (err) |e| e.deinit();

    const executor = try Pool.init(&ctx, &.{}, &err);
    defer {
        executor.requestClose();
        executor.unref();
    }

    const world = try World.init(&ctx, .{ .label = "test-world", .pool = executor });
    defer world.deinit(&ctx);

    const ex = world.getPool(&ctx);
    defer ex.unref();
    try std.testing.expectEqual(executor.id(), ex.id());
}

test "world allocator: base" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const world = try World.init(&ctx, .{ .label = "test-world" });
    defer world.deinit(&ctx);

    var world_allocator = world.getAllocator(&ctx);
    const allocator = world_allocator.allocator();

    try std.heap.testAllocator(allocator);
    try std.heap.testAllocatorAligned(allocator);
    try std.heap.testAllocatorLargeAlignment(allocator);
    try std.heap.testAllocatorAlignedShrink(allocator);
}

test "world allocator: auto free memory" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const world = try World.init(&ctx, .{ .label = "test-world" });
    defer world.deinit(&ctx);

    var world_allocator = world.getAllocator(&ctx);
    const allocator = world_allocator.allocator();

    _ = try allocator.alloc(u8, 100);
}
