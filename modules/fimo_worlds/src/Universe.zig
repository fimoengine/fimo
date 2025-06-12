const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;
const ArrayListUnmanaged = std.ArrayListUnmanaged;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const Tracing = fimo_std.Context.Tracing;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const RwLock = fimo_tasks_meta.sync.RwLock;
const fimo_worlds_meta = @import("fimo_worlds_meta");
const ResourceId = fimo_worlds_meta.resources.ResourceId;
const SystemId = fimo_worlds_meta.systems.SystemId;
const Dependency = fimo_worlds_meta.systems.System.Dependency;

const fimo_export = @import("fimo_export.zig");
const Instance = fimo_export.Instance;

const Self = @This();

rwlock: RwLock = .{},
allocator: Allocator,
num_worlds: usize = 0,
next_resource: ResourceId = @enumFromInt(0),
free_resources: ArrayListUnmanaged(ResourceId) = .empty,
resources: AutoArrayHashMapUnmanaged(ResourceId, *ResourceInfo) = .empty,
next_system: SystemId = @enumFromInt(0),
free_systems: ArrayListUnmanaged(SystemId) = .empty,
systems: AutoArrayHashMapUnmanaged(SystemId, *SystemInfo) = .empty,

pub const RegisterResourceOptions = struct {
    label: ?[]const u8 = null,
    size: usize,
    alignment: Alignment,
};

pub const RegisterSystemOptions = struct {
    label: ?[]const u8 = null,
    exclusive_resources: []const ResourceId = &.{},
    shared_resources: []const ResourceId = &.{},
    before: []const Dependency = &.{},
    after: []const Dependency = &.{},

    factory: ?[]const u8,
    factory_alignment: Alignment,
    factory_deinit: ?*const fn (factory: ?*const anyopaque) callconv(.c) void,

    system_size: usize,
    system_alignment: Alignment,
    system_init: *const fn (
        factory: ?*const anyopaque,
        context: *fimo_worlds_meta.systems.SystemContext,
        system: ?*anyopaque,
    ) callconv(.c) bool,
    system_deinit: ?*const fn (system: ?*anyopaque) callconv(.c) void,
    system_run: *const fn (
        system: ?*anyopaque,
        unique_resources: ?[*]const *anyopaque,
        shared_resources: ?[*]const *anyopaque,
        deferred_signal: *fimo_worlds_meta.Job.Fence,
    ) callconv(.c) void,
};

pub const ResourceInfo = struct {
    label: []u8,
    size: usize,
    alignment: Alignment,
    references: atomic.Value(usize) = .init(0),
};

pub const SystemInfo = struct {
    id: SystemId,
    label: []u8,
    exclusive_resources: []ResourceId,
    shared_resources: []ResourceId,
    before: []Dependency,
    after: []Dependency,
    references: atomic.Value(usize) = .init(0),

    factory: ?[]u8,
    factory_alignment: Alignment,
    factory_deinit: ?*const fn (factory: ?*anyopaque) callconv(.c) void,

    system_size: usize,
    system_alignment: Alignment,
    system_init: *const fn (
        factory: ?*const anyopaque,
        context: *fimo_worlds_meta.systems.SystemContext,
        system: ?*anyopaque,
    ) callconv(.c) bool,
    system_deinit: ?*const fn (system: ?*anyopaque) callconv(.c) void,
    system_run: *const fn (
        system: ?*anyopaque,
        unique_resources: ?[*]const *anyopaque,
        shared_resources: ?[*]const *anyopaque,
        deferred_signal: *fimo_worlds_meta.Job.Fence,
    ) callconv(.c) void,
};

pub fn deinit(self: *Self) void {
    if (self.resources.count() != 0) @panic("resources not empty");
    if (self.systems.count() != 0) @panic("systems not empty");
    self.free_resources.deinit(self.allocator);
    self.resources.deinit(self.allocator);
    self.free_systems.deinit(self.allocator);
    self.systems.deinit(self.allocator);
}

pub fn notifyWorldInit(self: *Self) void {
    const instance = getInstance();
    self.rwlock.lockExclusive(instance);
    defer self.rwlock.unlockExclusive(instance);

    if (self.isEmpty()) instance.ref();
    self.num_worlds += 1;
}

pub fn notifyWorldDeinit(self: *Self) void {
    const instance = getInstance();
    self.rwlock.lockExclusive(instance);
    defer self.rwlock.unlockExclusive(instance);

    self.num_worlds -= 1;
    if (self.isEmpty()) instance.unref();
}

pub fn registerResource(self: *Self, options: RegisterResourceOptions) !ResourceId {
    const instance = getInstance();
    self.rwlock.lockExclusive(instance);
    defer self.rwlock.unlockExclusive(instance);

    const was_empty = self.isEmpty();
    const info = try self.allocator.create(ResourceInfo);
    errdefer self.allocator.destroy(info);
    info.* = .{
        .label = undefined,
        .size = options.size,
        .alignment = options.alignment,
    };

    info.label = try self.allocator.dupe(u8, options.label orelse "<unlabelled>");
    errdefer self.allocator.free(info.label);

    const id, const from_list = if (self.free_resources.getLastOrNull()) |x|
        .{ x, true }
    else
        .{ self.next_resource, false };
    try self.resources.put(self.allocator, id, info);

    if (from_list)
        _ = self.free_resources.pop()
    else {
        self.next_resource = @enumFromInt(@intFromEnum(self.next_resource) + 1);
    }

    if (was_empty) instance.ref();
    return id;
}

pub fn unregisterResource(self: *Self, id: ResourceId) void {
    const instance = getInstance();
    self.rwlock.lockExclusive(instance);
    defer self.rwlock.unlockExclusive(instance);

    const kv = self.resources.fetchSwapRemove(id) orelse @panic("invalid resource");
    const info = kv.value;
    if (info.references.load(.acquire) != 0) @panic("resource in use");

    self.allocator.free(info.label);
    self.allocator.destroy(info);
    self.free_resources.append(self.allocator, id) catch @panic("oom");

    if (self.isEmpty()) instance.unref();
}

pub fn registerSystem(self: *Self, options: RegisterSystemOptions) !SystemId {
    errdefer if (options.factory_deinit) |f| {
        const factory = options.factory orelse &.{};
        f(factory.ptr);
    };

    const instance = getInstance();
    self.rwlock.lockExclusive(instance);
    defer self.rwlock.unlockExclusive(instance);

    for (options.exclusive_resources, 0..) |id, i| {
        if (!self.resources.contains(id)) return error.NotFound;
        if (std.mem.indexOfScalar(ResourceId, options.exclusive_resources[i + 1 ..], id) != null)
            return error.Deadlock;
    }
    for (options.shared_resources, 0..) |id, i| {
        if (!self.resources.contains(id)) return error.NotFound;
        if (std.mem.indexOfScalar(ResourceId, options.exclusive_resources, id) != null)
            return error.Deadlock;
        if (std.mem.indexOfScalar(ResourceId, options.shared_resources[i + 1 ..], id) != null)
            return error.Duplicate;
    }
    for (options.before) |dep| if (!self.systems.contains(dep.system)) return error.NotFound;
    for (options.after) |dep| {
        if (!self.systems.contains(dep.system)) return error.NotFound;
        for (options.before) |dep2| {
            if (dep.system == dep2.system) return error.Deadlock;
        }
    }

    const was_empty = self.isEmpty();
    const info = try self.allocator.create(SystemInfo);
    errdefer self.allocator.destroy(info);
    info.* = .{
        .id = undefined,
        .label = undefined,
        .exclusive_resources = undefined,
        .shared_resources = undefined,
        .before = undefined,
        .after = undefined,
        .factory = undefined,
        .factory_alignment = options.factory_alignment,
        .factory_deinit = options.factory_deinit,
        .system_size = options.system_size,
        .system_alignment = options.system_alignment,
        .system_init = options.system_init,
        .system_deinit = options.system_deinit,
        .system_run = options.system_run,
    };

    info.label = try self.allocator.dupe(u8, options.label orelse "<unlabelled>");
    errdefer self.allocator.free(info.label);
    info.exclusive_resources = try self.allocator.dupe(ResourceId, options.exclusive_resources);
    errdefer self.allocator.free(info.exclusive_resources);
    info.shared_resources = try self.allocator.dupe(ResourceId, options.shared_resources);
    errdefer self.allocator.free(info.shared_resources);
    info.before = try self.allocator.dupe(Dependency, options.before);
    errdefer self.allocator.free(info.before);
    info.after = try self.allocator.dupe(Dependency, options.after);
    errdefer self.allocator.free(info.after);

    info.factory = if (options.factory) |factory| blk: {
        const dupe = self.allocator.rawAlloc(
            factory.len,
            options.factory_alignment,
            @returnAddress(),
        ) orelse return Allocator.Error.OutOfMemory;
        const dupeSlice = dupe[0..factory.len];
        @memcpy(dupeSlice, factory);
        break :blk dupeSlice;
    } else null;

    const id, const from_list = if (self.free_systems.getLastOrNull()) |x|
        .{ x, true }
    else
        .{ self.next_system, false };
    info.id = id;
    try self.systems.put(self.allocator, id, info);

    if (from_list)
        _ = self.free_systems.pop()
    else {
        self.next_system = @enumFromInt(@intFromEnum(self.next_system) + 1);
    }

    for (options.exclusive_resources) |id2| {
        const res = self.resources.get(id2).?;
        _ = res.references.fetchAdd(1, .monotonic);
    }
    for (options.shared_resources) |id2| {
        const res = self.resources.get(id2).?;
        _ = res.references.fetchAdd(1, .monotonic);
    }
    for (info.before) |dep| _ = self.systems.get(dep.system).?.references.fetchAdd(1, .monotonic);
    for (info.after) |dep| _ = self.systems.get(dep.system).?.references.fetchAdd(1, .monotonic);

    if (was_empty) instance.ref();
    return id;
}

pub fn unregisterSystem(self: *Self, id: SystemId) void {
    const instance = getInstance();
    self.rwlock.lockExclusive(instance);
    defer self.rwlock.unlockExclusive(instance);

    const kv = self.systems.fetchSwapRemove(id) orelse @panic("invalid system");
    const info = kv.value;
    if (info.references.load(.acquire) != 0) @panic("system in use");
    if (info.factory_deinit) |f| if (info.factory) |factory| f(factory.ptr);

    for (info.exclusive_resources) |id2| {
        const res = self.resources.get(id2).?;
        _ = res.references.fetchSub(1, .monotonic);
    }
    for (info.shared_resources) |id2| {
        const res = self.resources.get(id2).?;
        _ = res.references.fetchSub(1, .monotonic);
    }
    for (info.before) |dep| _ = self.systems.get(dep.system).?.references.fetchSub(1, .monotonic);
    for (info.after) |dep| _ = self.systems.get(dep.system).?.references.fetchSub(1, .monotonic);

    self.allocator.free(info.label);
    self.allocator.free(info.exclusive_resources);
    self.allocator.free(info.shared_resources);
    self.allocator.free(info.before);
    self.allocator.free(info.after);
    if (info.factory) |factory| self.allocator.rawFree(factory, info.factory_alignment, @returnAddress());
    self.allocator.destroy(info);
    self.free_systems.append(self.allocator, id) catch @panic("oom");

    if (self.isEmpty()) instance.unref();
}

fn isEmpty(self: *Self) bool {
    return self.num_worlds == 0 and self.resources.count() == 0 and self.systems.count() == 0;
}

pub fn getUniverse() *Self {
    const instance = getInstance();
    return &instance.state().universe;
}

pub fn getInstance() *const Instance {
    return fimo_export.getInstance();
}

/// Returns the tracing subsystem of the owner instance.
pub fn tracing() Tracing {
    const instance = getInstance();
    return instance.context().tracing();
}

/// Logs an error message.
pub fn logErr(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    tracing().emitErrSimple(fmt, args, location);
}

/// Logs a debug message.
pub fn logDebug(
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    tracing().emitDebugSimple(fmt, args, location);
}
