const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;

const fimo_std = @import("fimo_std");
const Error = fimo_std.ctx.Error;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Pool = fimo_tasks_meta.pool.Pool;

const resources = @import("resources.zig");
const Resource = resources.Resource;
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
    pub fn init(options: CreateOptions) Error!*World {
        const desc = CreateOptions.Descriptor{
            .next = null,
            .label = if (options.label) |l| l.ptr else null,
            .label_len = if (options.label) |l| l.len else 0,
            .pool = if (options.pool) |*p| p else null,
        };

        var world: *World = undefined;
        const sym = symbols.world_create.getGlobal().get();
        try sym(&desc, &world).intoErrorUnion();
        return world;
    }

    /// Destroys the world.
    ///
    /// The world must be empty.
    pub fn deinit(self: *World) void {
        const sym = symbols.world_destroy.getGlobal().get();
        return sym(self);
    }

    /// Returns the label of the world.
    pub fn getLabel(self: *World) ?[]const u8 {
        var len: usize = undefined;
        const sym = symbols.world_get_label.getGlobal().get();
        if (sym(self, &len)) |label| return label[0..len];
        return null;
    }

    /// Returns a reference to the executor used by the world.
    pub fn getPool(self: *World) Pool {
        const sym = symbols.world_get_pool.getGlobal().get();
        return sym(self);
    }

    /// Checks if the resource is instantiated in the world.
    pub fn hasResource(self: *World, handle: *Resource) bool {
        const sym = symbols.world_has_resource.getGlobal().get();
        return sym(self, handle);
    }

    /// Adds the resource to the world.
    pub fn addResource(self: *World, handle: *Resource, value: *const anyopaque) Error!void {
        const sym = symbols.world_add_resource.getGlobal().get();
        try sym(self, handle, value).intoErrorUnion();
    }

    /// Removes the resource from the world.
    pub fn removeResource(self: *World, handle: *Resource, value: *anyopaque) Error!void {
        const sym = symbols.world_remove_resource.getGlobal().get();
        try sym(self, handle, value).intoErrorUnion();
    }

    /// Acquires a set of exclusive and shared resource references.
    ///
    /// The pointers to the resources are written into `out_resources`, where the indices
    /// `0..exclusive_handles.len` contain the resources in the `exclusive_ids` list, while the
    /// indices `exclusive_handles.len..exclusive_handles.len+shared_handles.len` contain the
    /// remaining resources from the `shared_handles` list.
    ///
    /// The locks to the resources are acquired in increasing resource id order.
    /// The caller will block until all resources are locked.
    pub fn lockResourcesRaw(
        self: *World,
        exclusive_handles: []const *Resource,
        shared_handles: []const *Resource,
        out_resources: []*anyopaque,
    ) void {
        std.debug.assert(exclusive_handles.len + shared_handles.len <= out_resources.len);
        const sym = symbols.world_lock_resources.getGlobal().get();
        sym(
            self,
            exclusive_handles.ptr,
            exclusive_handles.len,
            shared_handles.ptr,
            shared_handles.len,
            out_resources.ptr,
        );
    }

    /// Unlocks an exclusive resource lock.
    pub fn unlockResourceExclusive(self: *World, handle: *Resource) void {
        const sym = symbols.world_unlock_resource_exclusive.getGlobal().get();
        return sym(self, handle);
    }

    /// Unlocks a shared resource lock.
    pub fn unlockResourceShared(self: *World, handle: *Resource) void {
        const sym = symbols.world_unlock_resource_shared.getGlobal().get();
        return sym(self, handle);
    }

    /// Adds a new empty system group to the world.
    pub fn addSystemGroup(self: *World, options: AddSystemGroupOptions) Error!*SystemGroup {
        return SystemGroup.init(.{ .label = options.label, .pool = options.pool, .world = self });
    }

    /// Returns the allocator for the world.
    pub fn getAllocator(self: *World) WorldAllocator {
        return .{ .world = self };
    }
};

/// An allocator that clears its memory upon destruction of the owning world.
pub const WorldAllocator = struct {
    world: *World,

    const Self = @This();
    const vtable = Allocator.VTable{
        .alloc = &alloc,
        .resize = &resize,
        .remap = &remap,
        .free = &free,
    };

    fn alloc(this: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
        const self: *Self = @ptrCast(@alignCast(this));
        const sym = symbols.world_allocator_alloc.getGlobal().get();
        return sym(self.world, len, alignment.toByteUnits(), ret_addr);
    }

    fn resize(this: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) bool {
        const self: *Self = @ptrCast(@alignCast(this));
        const sym = symbols.world_allocator_resize.getGlobal().get();
        return sym(self.world, memory.ptr, memory.len, alignment.toByteUnits(), new_len, ret_addr);
    }

    fn remap(this: *anyopaque, memory: []u8, alignment: Alignment, new_len: usize, ret_addr: usize) ?[*]u8 {
        const self: *Self = @ptrCast(@alignCast(this));
        const sym = symbols.world_allocator_remap.getGlobal().get();
        return sym(self.world, memory.ptr, memory.len, alignment.toByteUnits(), new_len, ret_addr);
    }

    fn free(this: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) void {
        const self: *Self = @ptrCast(@alignCast(this));
        const sym = symbols.world_allocator_free.getGlobal().get();
        sym(self.world, memory.ptr, memory.len, alignment.toByteUnits(), ret_addr);
    }

    pub fn allocator(self: *Self) Allocator {
        return .{ .ptr = self, .vtable = &vtable };
    }
};

test "World: smoke test" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    const label = world.getLabel().?;
    try std.testing.expectEqualSlices(u8, "test-world", label);
}

test "World: custom pool" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const executor = try Pool.init(&.{});
    defer {
        executor.requestClose();
        executor.unref();
    }

    const world = try World.init(.{ .label = "test-world", .pool = executor });
    defer world.deinit();

    const ex = world.getPool();
    defer ex.unref();
    try std.testing.expectEqual(executor.id(), ex.id());
}

test "WorldAllocator: base" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    var world_allocator = world.getAllocator();
    const allocator = world_allocator.allocator();

    try std.heap.testAllocator(allocator);
    try std.heap.testAllocatorAligned(allocator);
    try std.heap.testAllocatorLargeAlignment(allocator);
    try std.heap.testAllocatorAlignedShrink(allocator);
}

test "WorldAllocator: auto free memory" {
    const GlobalCtx = testing.GlobalCtx;
    try GlobalCtx.init();
    defer GlobalCtx.deinit();

    const world = try World.init(.{ .label = "test-world" });
    defer world.deinit();

    var world_allocator = world.getAllocator();
    const allocator = world_allocator.allocator();

    _ = try allocator.alloc(u8, 100);
}
