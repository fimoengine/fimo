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
    references: atomic.Value(usize) = .init(0),

    rwlock: RwLock = .{},
    static_before_count: usize,
    static_after_count: usize,
    before: AutoArrayHashMapUnmanaged(*System, Link),
    after: AutoArrayHashMapUnmanaged(*System, Link),

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

    pub const Link = packed struct(usize) {
        implicit: bool,
        weak: bool = false,
        ignore_deferred: bool = false,
        reserved: u61 = undefined,
    };

    pub const Dependency = extern struct {
        system: *System,
        flags: fimo_worlds_meta.systems.Declaration.Flags = .{},
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

    var before: AutoArrayHashMapUnmanaged(*System, System.Link) = .empty;
    errdefer before.deinit(self.allocator);
    try before.ensureTotalCapacity(self.allocator, options.before.len);

    var after: AutoArrayHashMapUnmanaged(*System, System.Link) = .empty;
    errdefer after.deinit(self.allocator);
    try after.ensureTotalCapacity(self.allocator, options.after.len);

    for (options.before) |dep| {
        if (!self.systems.contains(dep.system)) return error.NotFound;
        if (before.contains(dep.system)) return error.Duplicate;
        before.putAssumeCapacity(
            dep.system,
            .{
                .implicit = false,
                .weak = dep.flags.weak,
                .ignore_deferred = dep.flags.ignore_deferred,
            },
        );
    }
    for (options.after) |dep| {
        if (!self.systems.contains(dep.system)) return error.NotFound;
        if (before.contains(dep.system)) return error.Deadlock;
        if (after.contains(dep.system)) return error.Duplicate;
        after.putAssumeCapacity(
            dep.system,
            .{
                .implicit = false,
                .weak = dep.flags.weak,
                .ignore_deferred = dep.flags.ignore_deferred,
            },
        );
    }

    // Check if it introduces a new cycle.
    {
        var stack: ArrayListUnmanaged(*System) = .empty;
        defer stack.deinit(self.allocator);
        var visited: AutoArrayHashMapUnmanaged(*System, void) = .empty;
        defer visited.deinit(self.allocator);

        try stack.appendSlice(self.allocator, after.keys());
        for (after.keys()) |sys| try visited.put(self.allocator, sys, {});
        while (stack.pop()) |sys| {
            if (before.contains(sys)) return error.Deadlock;
            sys.rwlock.lockRead(instance);
            defer sys.rwlock.unlockRead(instance);
            for (sys.after.keys(), sys.after.values()) |s, l| {
                if (l.implicit or visited.contains(s)) continue;
                try visited.put(self.allocator, s, {});
                try stack.append(self.allocator, s);
            }
        }
    }

    const was_empty = self.isEmpty();
    const sys = try self.allocator.create(System);
    errdefer self.allocator.destroy(sys);
    sys.* = .{
        .label = undefined,
        .exclusive_resources = undefined,
        .shared_resources = undefined,
        .static_before_count = before.count(),
        .static_after_count = after.count(),
        .before = before,
        .after = after,
        .factory = undefined,
        .factory_alignment = options.factory_alignment,
        .factory_deinit = options.factory_deinit,
        .system_size = options.system_size,
        .system_alignment = options.system_alignment,
        .system_init = options.system_init,
        .system_deinit = options.system_deinit,
        .system_run = options.system_run,
    };

    sys.label = try self.allocator.dupe(u8, options.label orelse "<unlabelled>");
    errdefer self.allocator.free(sys.label);
    sys.exclusive_resources = try self.allocator.dupe(*Resource, options.exclusive_resources);
    errdefer self.allocator.free(sys.exclusive_resources);
    sys.shared_resources = try self.allocator.dupe(*Resource, options.shared_resources);
    errdefer self.allocator.free(sys.shared_resources);

    errdefer {
        for (before.keys()) |sys2| {
            sys2.rwlock.lockWrite(instance);
            defer sys2.rwlock.unlockWrite(instance);
            _ = sys2.after.swapRemove(sys);
        }
        for (after.keys()) |sys2| {
            sys2.rwlock.lockWrite(instance);
            defer sys2.rwlock.unlockWrite(instance);
            _ = sys2.before.swapRemove(sys);
        }
    }
    sys.rwlock.lockWrite(instance);
    defer sys.rwlock.unlockWrite(instance);
    for (before.keys(), before.values()) |sys2, link| {
        sys2.rwlock.lockWrite(instance);
        defer sys2.rwlock.unlockWrite(instance);
        try sys2.after.put(
            self.allocator,
            sys,
            .{
                .implicit = true,
                .weak = link.weak,
                .ignore_deferred = link.ignore_deferred,
            },
        );
    }
    for (after.keys(), after.values()) |sys2, link| {
        sys2.rwlock.lockWrite(instance);
        defer sys2.rwlock.unlockWrite(instance);
        try sys2.before.put(
            self.allocator,
            sys,
            .{
                .implicit = true,
                .weak = link.weak,
                .ignore_deferred = link.ignore_deferred,
            },
        );
    }

    sys.factory = if (options.factory) |factory| blk: {
        const dupe = self.allocator.rawAlloc(
            factory.len,
            options.factory_alignment,
            @returnAddress(),
        ) orelse return Allocator.Error.OutOfMemory;
        const dupeSlice = dupe[0..factory.len];
        @memcpy(dupeSlice, factory);
        break :blk dupeSlice;
    } else null;
    try self.systems.put(self.allocator, sys, {});

    for (options.exclusive_resources) |res| {
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchAdd(1, .monotonic);
    }
    for (options.shared_resources) |res| {
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchAdd(1, .monotonic);
    }

    if (was_empty) instance.ref();
    return sys;
}

pub fn unregisterSystem(self: *Self, sys: *System) void {
    const instance = getInstance();
    self.rwlock.lockWrite(instance);
    defer self.rwlock.unlockWrite(instance);

    sys.rwlock.lockWrite(instance);

    if (!self.systems.swapRemove(sys)) @panic("invalid system");
    if (sys.before.count() != sys.static_before_count) @panic("system referenced");
    if (sys.after.count() != sys.static_after_count) @panic("system referenced");
    if (sys.references.load(.acquire) != 0) @panic("system in use");
    if (sys.factory_deinit) |f| if (sys.factory) |factory| f(factory.ptr);

    for (sys.exclusive_resources) |res| {
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchSub(1, .monotonic);
    }
    for (sys.shared_resources) |res| {
        std.debug.assert(self.resources.contains(res));
        _ = res.references.fetchSub(1, .monotonic);
    }

    for (sys.before.keys(), sys.before.values()) |sys2, link| {
        if (link.implicit) @panic("system referenced");
        sys2.rwlock.lockWrite(instance);
        defer sys2.rwlock.unlockWrite(instance);
        const entry = sys2.after.fetchSwapRemove(sys) orelse unreachable;
        std.debug.assert(entry.value.implicit);
    }
    for (sys.after.keys(), sys.after.values()) |sys2, link| {
        if (link.implicit) @panic("system referenced");
        sys2.rwlock.lockWrite(instance);
        defer sys2.rwlock.unlockWrite(instance);
        const entry = sys2.before.fetchSwapRemove(sys) orelse unreachable;
        std.debug.assert(entry.value.implicit);
    }

    self.allocator.free(sys.label);
    self.allocator.free(sys.exclusive_resources);
    self.allocator.free(sys.shared_resources);
    sys.before.deinit(self.allocator);
    sys.after.deinit(self.allocator);
    if (sys.factory) |factory| self.allocator.rawFree(factory, sys.factory_alignment, @returnAddress());
    self.allocator.destroy(sys);

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
