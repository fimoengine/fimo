const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const tracing = fimo_std.tracing;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const RwLock = fimo_tasks_meta.sync.RwLock;
const Pool = fimo_tasks_meta.pool.Pool;

const FimoWorlds = @import("FimoWorlds.zig");
const heap = @import("heap.zig");
const SystemGroup = @import("SystemGroup.zig");
const Universe = @import("Universe.zig");
const Resource = Universe.Resource;

const World = @This();

rwlock: RwLock = .{},
label: []u8,
executor: Pool,
inherited_executor: bool,
allocator: heap.TracingAllocator,
system_group_count: atomic.Value(usize) = .init(0),
resources: AutoArrayHashMapUnmanaged(*Resource, *ResourceValue) = .empty,

const ResourceValue = struct {
    rwlock: RwLock = .{},
    value_ptr: *anyopaque,
    references: atomic.Value(usize) = .init(0),
};

pub const InitOptions = struct {
    label: ?[]const u8 = null,
    executor: ?Pool = null,
};

pub fn init(options: InitOptions) !*World {
    var world_allocator = heap.TracingAllocator{};
    errdefer world_allocator.deinit();
    const allocator = world_allocator.allocator();

    const label = try allocator.dupe(u8, options.label orelse "<unlabelled>");
    const executor, const inherited_executor = if (options.executor) |ex| .{ ex.ref(), true } else blk: {
        const executor_label = try std.fmt.allocPrint(allocator, "world `{s}` executor", .{label});
        defer allocator.free(executor_label);

        break :blk .{ try Pool.init(
            &.{ .label_ = executor_label.ptr, .label_len = executor_label.len },
        ), false };
    };
    errdefer executor.unref();

    const world = try allocator.create(World);
    world.* = .{
        .label = label,
        .executor = executor,
        .inherited_executor = inherited_executor,
        .allocator = world_allocator,
    };
    FimoWorlds.get().universe.notifyWorldInit();
    tracing.logDebug(@src(), "created `{*}`, label=`{s}`, executor=`{x}`", .{
        world,
        label,
        executor.id(),
    });
    return world;
}

pub fn deinit(self: *World) void {
    tracing.logDebug(@src(), "destroying `{*}`", .{self});
    if (self.system_group_count.load(.acquire) != 0) @panic("world not empty");
    if (self.resources.count() != 0) @panic("world not empty");

    if (!self.inherited_executor) self.executor.requestClose();
    self.executor.unref();

    var allocator = self.allocator;
    allocator.deinit();
    FimoWorlds.get().universe.notifyWorldDeinit();
}

pub fn addResource(self: *World, handle: *Resource, value: *const anyopaque) !void {
    {
        const universe = &FimoWorlds.get().universe;
        universe.rwlock.lockRead();
        defer universe.rwlock.unlockRead();

        if (!universe.resources.contains(handle)) @panic("invalid resource");
        _ = handle.references.fetchAdd(1, .monotonic);
    }
    errdefer _ = handle.references.fetchSub(1, .monotonic);

    const allocator = self.allocator.allocator();
    const value_start = std.mem.alignForward(usize, @sizeOf(ResourceValue), handle.alignment.toByteUnits());
    const memory_len = value_start + handle.size;
    const memory = try allocator.alignedAlloc(u8, .of(ResourceValue), memory_len);
    errdefer allocator.free(memory);

    const value_slice = memory[value_start..];
    @memcpy(value_slice, @as([*]const u8, @ptrCast(value))[0..handle.size]);

    const resource = std.mem.bytesAsValue(ResourceValue, memory);
    resource.* = ResourceValue{ .value_ptr = value_slice.ptr };

    self.rwlock.lockWrite();
    defer self.rwlock.unlockWrite();
    if (self.resources.contains(handle)) return error.Duplicate;
    try self.resources.put(allocator, handle, resource);
    tracing.logDebug(@src(), "added `{}` to `{*}`", .{ handle, self });
}

pub fn removeResource(self: *World, handle: *Resource, value: *anyopaque) !void {
    tracing.logDebug(@src(), "removing `{}` from `{*}`", .{ handle, self });
    const resource = blk: {
        self.rwlock.lockWrite();
        defer self.rwlock.unlockWrite();

        const res = self.resources.get(handle) orelse @panic("invalid resource");
        if (res.references.load(.acquire) != 0) return error.InUse;
        _ = self.resources.swapRemove(handle);
        break :blk res;
    };

    @memcpy(
        @as([*]u8, @ptrCast(value))[0..handle.size],
        @as([*]const u8, @ptrCast(resource.value_ptr))[0..handle.size],
    );

    const allocator = self.allocator.allocator();
    const value_start = std.mem.alignForward(usize, @sizeOf(ResourceValue), handle.alignment.toByteUnits());
    const memory_len = value_start + handle.size;
    const memory = std.mem.asBytes(resource).ptr[0..memory_len];
    allocator.free(memory);
    _ = handle.references.fetchSub(1, .monotonic);
}

pub fn hasResource(self: *World, handle: *Resource) bool {
    self.rwlock.lockRead();
    defer self.rwlock.unlockRead();
    return self.resources.contains(handle);
}

pub fn lockResources(
    self: *World,
    exclusive: []const *Resource,
    shared: []const *Resource,
    out: []*anyopaque,
) void {
    if (out.len != exclusive.len + shared.len) @panic("buffer size mismatch");

    const Info = struct {
        index: usize,
        handle: *Resource,
        resource: *ResourceValue,
        lock_type: enum { exclusive, shared },
        fn lessThan(ctx: void, a: @This(), b: @This()) bool {
            _ = ctx;
            return @intFromPtr(a.handle) < @intFromPtr(b.handle);
        }
    };

    var stack_fallback = std.heap.stackFallback(32 * @sizeOf(Info), FimoWorlds.get().allocator);
    const allocator = stack_fallback.get();

    const infos = allocator.alloc(Info, out.len) catch @panic("oom");
    defer allocator.free(infos);

    {
        self.rwlock.lockRead();
        defer self.rwlock.unlockRead();

        for (exclusive, 0..) |handle, i| {
            if (std.mem.indexOfScalar(*Resource, exclusive[i + 1 ..], handle) != null) @panic("deadlock");
            const resource = self.resources.get(handle) orelse @panic("invalid resource");
            _ = resource.references.fetchAdd(1, .monotonic);
            infos[i] = Info{ .index = i, .handle = handle, .resource = resource, .lock_type = .exclusive };
        }
        for (shared, 0..) |handle, i| {
            if (std.mem.indexOfScalar(*Resource, shared[i + 1 ..], handle) != null) @panic("duplicate");
            if (std.mem.indexOfScalar(*Resource, exclusive, handle) != null) @panic("deadlock");
            const resource = self.resources.get(handle) orelse @panic("invalid resource");
            _ = resource.references.fetchAdd(1, .monotonic);
            const idx = exclusive.len + i;
            infos[idx] = Info{ .index = idx, .handle = handle, .resource = resource, .lock_type = .shared };
        }
    }

    std.mem.sort(Info, infos, {}, Info.lessThan);
    for (infos) |info| {
        switch (info.lock_type) {
            .exclusive => info.resource.rwlock.lockWrite(),
            .shared => info.resource.rwlock.lockRead(),
        }
        out[info.index] = info.resource.value_ptr;
    }
}

pub fn unlockResourceExclusive(self: *World, handle: *Resource) void {
    self.rwlock.lockRead();
    defer self.rwlock.unlockRead();

    const resource = self.resources.get(handle) orelse @panic("invalid resource");
    _ = resource.references.fetchSub(1, .monotonic);
    resource.rwlock.unlockWrite();
}

pub fn unlockResourceShared(self: *World, handle: *Resource) void {
    self.rwlock.lockRead();
    defer self.rwlock.unlockRead();

    const resource = self.resources.get(handle) orelse @panic("invalid resource");
    _ = resource.references.fetchSub(1, .monotonic);
    resource.rwlock.unlockRead();
}
