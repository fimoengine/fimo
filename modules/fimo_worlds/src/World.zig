const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const RwLock = fimo_tasks_meta.sync.RwLock;
const Pool = fimo_tasks_meta.pool.Pool;

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

pub fn addResource(self: *World, handle: *Resource, value: *const anyopaque) !void {
    const instance = Universe.getInstance();
    {
        const universe = Universe.getUniverse();
        universe.rwlock.lockRead(instance);
        defer universe.rwlock.unlockRead(instance);

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

    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);
    if (self.resources.contains(handle)) return error.Duplicate;
    try self.resources.put(allocator, handle, resource);
    Universe.logDebug("added `{}` to `{*}`", .{ handle, self }, @src());
}

pub fn removeResource(self: *World, handle: *Resource, value: *anyopaque) !void {
    Universe.logDebug("removing `{}` from `{*}`", .{ handle, self }, @src());
    const resource = blk: {
        const instance = Universe.getInstance();
        self.rwlock.lockWrite(instance);
        defer self.rwlock.unlockWrite(instance);

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
    const instance = Universe.getInstance();
    self.rwlock.lockRead(instance);
    defer self.rwlock.unlockRead(instance);
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

    var stack_fallback = std.heap.stackFallback(32 * @sizeOf(Info), Universe.getUniverse().allocator);
    const allocator = stack_fallback.get();

    const infos = allocator.alloc(Info, out.len) catch @panic("oom");
    defer allocator.free(infos);

    const instance = Universe.getInstance();
    {
        self.rwlock.lockRead(instance);
        defer self.rwlock.unlockRead(instance);

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
            .exclusive => info.resource.rwlock.lockWrite(instance),
            .shared => info.resource.rwlock.lockRead(instance),
        }
        out[info.index] = info.resource.value_ptr;
    }
}

pub fn unlockResourceExclusive(self: *World, handle: *Resource) void {
    const instance = Universe.getInstance();
    self.rwlock.lockRead(instance);
    defer self.rwlock.unlockRead(instance);

    const resource = self.resources.get(handle) orelse @panic("invalid resource");
    _ = resource.references.fetchSub(1, .monotonic);
    resource.rwlock.unlockWrite(instance);
}

pub fn unlockResourceShared(self: *World, handle: *Resource) void {
    const instance = Universe.getInstance();
    self.rwlock.lockRead(instance);
    defer self.rwlock.unlockRead(instance);

    const resource = self.resources.get(handle) orelse @panic("invalid resource");
    _ = resource.references.fetchSub(1, .monotonic);
    resource.rwlock.unlockRead(instance);
}
