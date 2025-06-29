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

const fimo_export = @import("fimo_export.zig");
const Instance = fimo_export.Instance;

const Self = @This();

rwlock: RwLock = .{},
allocator: Allocator,
num_worlds: usize = 0,
resources: AutoArrayHashMapUnmanaged(*Resource, void) = .empty,
systems: AutoArrayHashMapUnmanaged(*System, void) = .empty,

pub const RegisterResourceOptions = struct {
    label: ?[]const u8 = null,
    size: usize,
    alignment: Alignment,
};

pub const RegisterSystemOptions = struct {
    label: ?[]const u8 = null,
    exclusive_resources: []const *Resource = &.{},
    shared_resources: []const *Resource = &.{},
    before: []const System.Dependency = &.{},
    after: []const System.Dependency = &.{},

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

pub const Resource = struct {
    label: []u8,
    size: usize,
    alignment: Alignment,
    references: atomic.Value(usize) = .init(0),
};

pub const System = struct {
    label: []u8,
    exclusive_resources: []*Resource,
    shared_resources: []*Resource,
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

    pub const Dependency = extern struct {
        system: *System,
        ignore_deferred: bool = false,
    };
};

pub fn deinit(self: *Self) void {
    if (self.resources.count() != 0) @panic("resources not empty");
    if (self.systems.count() != 0) @panic("systems not empty");
    self.resources.deinit(self.allocator);
    self.systems.deinit(self.allocator);
}

pub fn notifyWorldInit(self: *Self) void {
    const instance = getInstance();
    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);

    if (self.isEmpty()) instance.ref();
    self.num_worlds += 1;
}

pub fn notifyWorldDeinit(self: *Self) void {
    const instance = getInstance();
    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);

    self.num_worlds -= 1;
    if (self.isEmpty()) instance.unref();
}

pub fn registerResource(self: *Self, options: RegisterResourceOptions) !*Resource {
    const instance = getInstance();
    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);

    const was_empty = self.isEmpty();
    const handle = try self.allocator.create(Resource);
    errdefer self.allocator.destroy(handle);
    handle.* = .{
        .label = undefined,
        .size = options.size,
        .alignment = options.alignment,
    };

    handle.label = try self.allocator.dupe(u8, options.label orelse "<unlabelled>");
    errdefer self.allocator.free(handle.label);
    try self.resources.put(self.allocator, handle, {});

    if (was_empty) instance.ref();
    return @ptrCast(handle);
}

pub fn unregisterResource(self: *Self, handle: *Resource) void {
    const instance = getInstance();
    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);

    if (!self.resources.swapRemove(handle)) @panic("invalid resource");
    if (handle.references.load(.acquire) != 0) @panic("resource in use");

    self.allocator.free(handle.label);
    self.allocator.destroy(handle);

    if (self.isEmpty()) instance.unref();
}

pub fn registerSystem(self: *Self, options: RegisterSystemOptions) !*System {
    errdefer if (options.factory_deinit) |f| {
        const factory = options.factory orelse &.{};
        f(factory.ptr);
    };

    const instance = getInstance();
    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);

    for (options.exclusive_resources, 0..) |handle, i| {
        if (!self.resources.contains(handle)) return error.NotFound;
        if (std.mem.indexOfScalar(*Resource, options.exclusive_resources[i + 1 ..], handle) != null)
            return error.Deadlock;
    }
    for (options.shared_resources, 0..) |handle, i| {
        if (!self.resources.contains(handle)) return error.NotFound;
        if (std.mem.indexOfScalar(*Resource, options.exclusive_resources, handle) != null)
            return error.Deadlock;
        if (std.mem.indexOfScalar(*Resource, options.shared_resources[i + 1 ..], handle) != null)
            return error.Duplicate;
    }
    for (options.before) |dep| if (!self.systems.contains(dep.system))
        return error.NotFound;
    for (options.after) |dep| {
        if (!self.systems.contains(dep.system)) return error.NotFound;
        for (options.before) |dep2| {
            if (dep.system == dep2.system) return error.Deadlock;
        }
    }

    const was_empty = self.isEmpty();
    const info = try self.allocator.create(System);
    errdefer self.allocator.destroy(info);
    info.* = .{
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
    info.exclusive_resources = try self.allocator.dupe(*Resource, options.exclusive_resources);
    errdefer self.allocator.free(info.exclusive_resources);
    info.shared_resources = try self.allocator.dupe(*Resource, options.shared_resources);
    errdefer self.allocator.free(info.shared_resources);
    info.before = try self.allocator.dupe(System.Dependency, options.before);
    errdefer self.allocator.free(info.before);
    info.after = try self.allocator.dupe(System.Dependency, options.after);
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
    try self.systems.put(self.allocator, info, {});

    for (options.exclusive_resources) |res| {
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchAdd(1, .monotonic);
    }
    for (options.shared_resources) |res| {
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchAdd(1, .monotonic);
    }
    for (info.before) |dep| {
        std.debug.assert(self.systems.contains(dep.system));
        _ = dep.system.references.fetchAdd(1, .monotonic);
    }
    for (info.after) |dep| {
        std.debug.assert(self.systems.contains(dep.system));
        _ = dep.system.references.fetchAdd(1, .monotonic);
    }

    if (was_empty) instance.ref();
    return info;
}

pub fn unregisterSystem(self: *Self, handle: *System) void {
    const instance = getInstance();
    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);

    if (!self.systems.swapRemove(handle)) @panic("invalid system");
    if (handle.references.load(.acquire) != 0) @panic("system in use");
    if (handle.factory_deinit) |f| if (handle.factory) |factory| f(factory.ptr);

    for (handle.exclusive_resources) |res_handle| {
        const res: *Resource = @alignCast(@ptrCast(res_handle));
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchSub(1, .monotonic);
    }
    for (handle.shared_resources) |res_handle| {
        const res: *Resource = @alignCast(@ptrCast(res_handle));
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchSub(1, .monotonic);
    }
    for (handle.before) |dep| _ = dep.system.references.fetchSub(1, .monotonic);
    for (handle.after) |dep| _ = dep.system.references.fetchSub(1, .monotonic);

    self.allocator.free(handle.label);
    self.allocator.free(handle.exclusive_resources);
    self.allocator.free(handle.shared_resources);
    self.allocator.free(handle.before);
    self.allocator.free(handle.after);
    if (handle.factory) |factory| self.allocator.rawFree(factory, handle.factory_alignment, @returnAddress());
    self.allocator.destroy(handle);

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
