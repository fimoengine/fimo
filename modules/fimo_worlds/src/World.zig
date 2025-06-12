const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const RwLock = fimo_tasks_meta.sync.RwLock;
const Pool = fimo_tasks_meta.pool.Pool;
const fimo_worlds_meta = @import("fimo_worlds_meta");
const ResourceId = fimo_worlds_meta.resources.ResourceId;

const heap = @import("heap.zig");
const SystemGroup = @import("SystemGroup.zig");
const Universe = @import("Universe.zig");

const World = @This();

rwlock: RwLock = .{},
label: []u8,
executor: Pool,
inherited_executor: bool,
allocator: heap.TracingAllocator,
system_group_count: atomic.Value(usize) = .init(0),
resources: AutoArrayHashMapUnmanaged(ResourceId, *Resource) = .empty,

const Resource = struct {
    rwlock: RwLock = .{},
    value_ptr: *anyopaque,
    info: *Universe.ResourceInfo,
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

        var err: ?AnyError = null;
        errdefer if (err) |e| e.deinit();
        break :blk .{ try Pool.init(
            Universe.getInstance(),
            &.{ .label_ = executor_label.ptr, .label_len = executor_label.len },
            &err,
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
    Universe.getUniverse().notifyWorldInit();
    Universe.logDebug(
        "created `{*}`, label=`{s}`, executor=`{x}`",
        .{ world, label, executor.id() },
        @src(),
    );
    return world;
}

pub fn deinit(self: *World) void {
    Universe.logDebug("destroying `{*}`", .{self}, @src());
    if (self.system_group_count.load(.acquire) != 0) @panic("world not empty");
    if (self.resources.count() != 0) @panic("world not empty");

    if (!self.inherited_executor) self.executor.requestClose();
    self.executor.unref();

    var allocator = self.allocator;
    allocator.deinit();
    Universe.getUniverse().notifyWorldDeinit();
}

pub fn addResource(self: *World, id: ResourceId, value: *const anyopaque) !void {
    const instance = Universe.getInstance();
    const info = blk: {
        const universe = Universe.getUniverse();
        universe.rwlock.lockShared(instance);
        defer universe.rwlock.unlockShared(instance);

        const i = universe.resources.get(id) orelse @panic("invalid resource");
        _ = i.references.fetchAdd(1, .monotonic);
        break :blk i;
    };
    errdefer _ = info.references.fetchSub(1, .monotonic);

    const allocator = self.allocator.allocator();
    const value_start = std.mem.alignForward(usize, @sizeOf(Resource), info.alignment.toByteUnits());
    const memory_len = value_start + info.size;
    const memory = try allocator.alignedAlloc(u8, .of(Resource), memory_len);
    errdefer allocator.free(memory);

    const value_slice = memory[value_start..];
    @memcpy(value_slice, @as([*]const u8, @ptrCast(value))[0..info.size]);

    const resource = std.mem.bytesAsValue(Resource, memory);
    resource.* = Resource{ .info = info, .value_ptr = value_slice.ptr };

    self.rwlock.lockExclusive(instance);
    defer self.rwlock.unlockExclusive(instance);
    if (self.resources.contains(id)) return error.Duplicate;
    try self.resources.put(allocator, id, resource);
    Universe.logDebug("added `{}` to `{*}`", .{ id, self }, @src());
}

pub fn removeResource(self: *World, id: ResourceId, value: *anyopaque) !void {
    Universe.logDebug("removing `{}` from `{*}`", .{ id, self }, @src());
    const resource = blk: {
        const instance = Universe.getInstance();
        self.rwlock.lockExclusive(instance);
        defer self.rwlock.unlockExclusive(instance);

        const res = self.resources.get(id) orelse @panic("invalid resource");
        if (res.references.load(.acquire) != 0) return error.InUse;
        _ = self.resources.swapRemove(id);
        break :blk res;
    };

    const info = resource.info;
    @memcpy(
        @as([*]u8, @ptrCast(value))[0..info.size],
        @as([*]const u8, @ptrCast(resource.value_ptr))[0..info.size],
    );

    const allocator = self.allocator.allocator();
    const value_start = std.mem.alignForward(usize, @sizeOf(Resource), info.alignment.toByteUnits());
    const memory_len = value_start + info.size;
    const memory = std.mem.asBytes(resource).ptr[0..memory_len];
    allocator.free(memory);
    _ = info.references.fetchSub(1, .monotonic);
}

pub fn hasResource(self: *World, id: ResourceId) bool {
    const instance = Universe.getInstance();
    self.rwlock.lockShared(instance);
    defer self.rwlock.unlockShared(instance);
    return self.resources.contains(id);
}

pub fn lockResources(
    self: *World,
    exclusive: []const ResourceId,
    shared: []const ResourceId,
    out: []*anyopaque,
) void {
    if (out.len != exclusive.len + shared.len) @panic("buffer size mismatch");

    const Info = struct {
        index: usize,
        id: ResourceId,
        resource: *Resource,
        lock_type: enum { exclusive, shared },
        fn lessThan(ctx: void, a: @This(), b: @This()) bool {
            _ = ctx;
            return @intFromEnum(a.id) < @intFromEnum(b.id);
        }
    };

    var stack_fallback = std.heap.stackFallback(32 * @sizeOf(Info), Universe.getUniverse().allocator);
    const allocator = stack_fallback.get();

    const infos = allocator.alloc(Info, out.len) catch @panic("oom");
    defer allocator.free(infos);

    const instance = Universe.getInstance();
    {
        self.rwlock.lockShared(instance);
        defer self.rwlock.unlockShared(instance);

        for (exclusive, 0..) |id, i| {
            if (std.mem.indexOfScalar(ResourceId, exclusive[i + 1 ..], id) != null) @panic("deadlock");
            const resource = self.resources.get(id) orelse @panic("invalid resource");
            _ = resource.references.fetchAdd(1, .monotonic);
            infos[i] = Info{ .index = i, .id = id, .resource = resource, .lock_type = .exclusive };
        }
        for (shared, 0..) |id, i| {
            if (std.mem.indexOfScalar(ResourceId, shared[i + 1 ..], id) != null) @panic("duplicate");
            if (std.mem.indexOfScalar(ResourceId, exclusive, id) != null) @panic("deadlock");
            const resource = self.resources.get(id) orelse @panic("invalid resource");
            _ = resource.references.fetchAdd(1, .monotonic);
            const idx = exclusive.len + i;
            infos[idx] = Info{ .index = idx, .id = id, .resource = resource, .lock_type = .shared };
        }
    }

    std.mem.sort(Info, infos, {}, Info.lessThan);
    for (infos) |info| {
        switch (info.lock_type) {
            .exclusive => info.resource.rwlock.lockExclusive(instance),
            .shared => info.resource.rwlock.lockShared(instance),
        }
        out[info.index] = info.resource.value_ptr;
    }
}

pub fn unlockResourceExclusive(self: *World, id: ResourceId) void {
    const instance = Universe.getInstance();
    self.rwlock.lockShared(instance);
    defer self.rwlock.unlockShared(instance);

    const resource = self.resources.get(id) orelse @panic("invalid resource");
    _ = resource.references.fetchSub(1, .monotonic);
    resource.rwlock.unlockExclusive(instance);
}

pub fn unlockResourceShared(self: *World, id: ResourceId) void {
    const instance = Universe.getInstance();
    self.rwlock.lockShared(instance);
    defer self.rwlock.unlockShared(instance);

    const resource = self.resources.get(id) orelse @panic("invalid resource");
    _ = resource.references.fetchSub(1, .monotonic);
    resource.rwlock.unlockShared(instance);
}
