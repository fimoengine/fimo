const std = @import("std");
const Allocator = std.mem.Allocator;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;
const ArenaAllocator = std.heap.ArenaAllocator;
const SinglyLinkedList = std.SinglyLinkedList;
const DoublyLinkedList = std.DoublyLinkedList;

const fimo_worlds_meta = @import("fimo_worlds_meta");
const AllocatorStrategy = fimo_worlds_meta.systems.AllocatorStrategy;
const SystemId = fimo_worlds_meta.systems.SystemId;
const Fence = fimo_worlds_meta.Job.Fence;

const heap = @import("heap.zig");
const TracingAllocator = heap.TracingAllocator;
const SystemGroup = @import("SystemGroup.zig");
const Universe = @import("Universe.zig");

const System = @This();

group: *SystemGroup,
info: *Universe.SystemInfo,

weak: bool,
node: DoublyLinkedList.Node = .{},
waiters: SinglyLinkedList = .{},
references: AutoArrayHashMapUnmanaged(SystemId, *System) = .empty,
referenced_by: AutoArrayHashMapUnmanaged(SystemId, *System) = .empty,

value_ptr: *anyopaque,
merge_deferred: bool = false,
deferred_fence: Fence = .{},
deferred_dep: AutoArrayHashMapUnmanaged(SystemId, *System) = .empty,
resources: []*anyopaque,
arena_allocator: ArenaAllocator,
tracing_allocator: TracingAllocator,

const DeinitWaiter = struct {
    fence: *Fence,
    node: SinglyLinkedList.Node = .{},
};

pub fn init(group: *SystemGroup, info: *Universe.SystemInfo, weak: bool) !*System {
    var tracing_allocator = TracingAllocator{};
    const all = tracing_allocator.allocator();

    const num_resources = info.exclusive_resources.len + info.shared_resources.len;

    const resources_start = std.mem.alignForward(usize, @sizeOf(System), @alignOf(*anyopaque));
    const resources_end = resources_start + (@sizeOf(*anyopaque) * num_resources);
    const value_start = std.mem.alignForward(usize, resources_end, info.system_alignment.toByteUnits());
    const value_end = value_start + info.system_size;
    const buffer_len = value_end;
    const buffer = try all.alignedAlloc(u8, .of(System), buffer_len);
    errdefer all.free(buffer);

    const sys: *System = std.mem.bytesAsValue(System, buffer);
    sys.* = .{
        .group = group,
        .info = info,
        .weak = weak,
        .value_ptr = @ptrCast(@alignCast(&buffer[value_start])),
        .resources = @alignCast(std.mem.bytesAsSlice(*anyopaque, buffer[resources_start..resources_end])),
        .arena_allocator = .init(group.single_allocator.allocator()),
        .tracing_allocator = tracing_allocator,
    };

    for (info.before) |dep| {
        const dep_sys = group.system_graph.systems.get(dep.system).?;
        sys.addReference(dep_sys);
    }
    for (info.after) |dep| {
        const dep_sys = group.system_graph.systems.get(dep.system).?;
        sys.addReference(dep_sys);
    }

    _ = info.references.fetchAdd(1, .monotonic);
    errdefer _ = info.references.fetchSub(1, .monotonic);

    const factory = if (info.factory) |f| f.ptr else null;
    if (!info.system_init(factory, @ptrCast(sys), sys.value_ptr)) return error.InitError;

    return sys;
}

pub fn deinit(self: *System) void {
    std.debug.assert(self.references.count() == 0);
    std.debug.assert(self.referenced_by.count() == 0);

    const info = self.info;
    var waiters = self.waiters;
    var tracing_allocator = self.tracing_allocator;
    if (info.system_deinit) |f| f(self.value_ptr);
    _ = info.references.fetchSub(1, .monotonic);

    const instance = Universe.getInstance();
    while (waiters.popFirst()) |n| {
        const waiter: *DeinitWaiter = @fieldParentPtr("node", n);
        waiter.fence.signal(instance);
    }
    self.arena_allocator.deinit();
    tracing_allocator.deinit();
}

pub fn addReference(self: *System, sys: *System) void {
    const id = self.info.id;
    const sys_id = sys.info.id;
    std.debug.assert(!self.references.contains(sys_id));
    std.debug.assert(!sys.referenced_by.contains(id));

    const all = self.tracing_allocator.allocator();
    const sys_all = sys.tracing_allocator.allocator();
    self.references.put(all, sys_id, sys) catch @panic("oom");
    sys.referenced_by.put(sys_all, id, self) catch @panic("oom");
}

pub fn removeReference(self: *System, sys: *System) void {
    const id = self.info.id;
    const sys_id = sys.info.id;
    std.debug.assert(self.references.contains(sys_id));
    std.debug.assert(sys.referenced_by.contains(id));

    _ = self.references.swapRemove(sys_id);
    _ = sys.referenced_by.swapRemove(id);
}

pub fn isUnloadable(self: *System) bool {
    return self.weak and self.referenced_by.count() == 0;
}

pub fn run(self: *System) void {
    self.deferred_fence.reset();
    for (self.deferred_dep.values()) |dep| dep.deferred_fence.wait(Universe.getInstance());
    const exclusive = self.resources[0..self.info.exclusive_resources.len];
    const shared = self.resources[exclusive.len..self.info.shared_resources.len];
    self.info.system_run(self.value_ptr, exclusive.ptr, shared.ptr, &self.deferred_fence);
    _ = self.arena_allocator.reset(.free_all);

    if (self.merge_deferred) self.deferred_fence.wait(Universe.getInstance());
}

pub fn allocator(self: *System, strategy: AllocatorStrategy) Allocator {
    return switch (strategy) {
        .transient => self.arena_allocator.allocator(),
        .single_generation => self.group.single_allocator.allocator(),
        .multi_generation => self.group.multi_allocator.allocator(),
        .system_persistent => self.tracing_allocator.allocator(),
        else => @panic("unknown strategy"),
    };
}

pub fn appendWaiter(self: *System, fence: *Fence) void {
    const all = self.tracing_allocator.allocator();
    const waiter = all.create(DeinitWaiter) catch @panic("oom");
    waiter.* = .{ .fence = fence };
    self.waiters.prepend(&waiter.node);
}
