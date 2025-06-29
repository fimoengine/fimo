const std = @import("std");
const Allocator = std.mem.Allocator;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;
const ArenaAllocator = std.heap.ArenaAllocator;
const SinglyLinkedList = std.SinglyLinkedList;
const DoublyLinkedList = std.DoublyLinkedList;

const fimo_worlds_meta = @import("fimo_worlds_meta");
const AllocatorStrategy = fimo_worlds_meta.systems.AllocatorStrategy;
const Fence = fimo_worlds_meta.Job.Fence;

const heap = @import("heap.zig");
const TracingAllocator = heap.TracingAllocator;
const SystemGroup = @import("SystemGroup.zig");
const Universe = @import("Universe.zig");
const System = Universe.System;

const SystemContext = @This();

group: *SystemGroup,
sys: *System,

weak: bool,
node: DoublyLinkedList.Node = .{},
waiters: SinglyLinkedList = .{},
references: AutoArrayHashMapUnmanaged(*System, *SystemContext) = .empty,
referenced_by: AutoArrayHashMapUnmanaged(*System, *SystemContext) = .empty,

value_ptr: *anyopaque,
merge_deferred: bool = false,
deferred_fence: Fence = .{},
deferred_dep: AutoArrayHashMapUnmanaged(*System, *SystemContext) = .empty,
resources: []*anyopaque,
arena_allocator: ArenaAllocator,
tracing_allocator: TracingAllocator,

const DeinitWaiter = struct {
    fence: *Fence,
    node: SinglyLinkedList.Node = .{},
};

pub fn init(group: *SystemGroup, sys: *System, weak: bool) !*SystemContext {
    var tracing_allocator = TracingAllocator{};
    const all = tracing_allocator.allocator();

    const num_resources = sys.exclusive_resources.len + sys.shared_resources.len;

    const resources_start = std.mem.alignForward(usize, @sizeOf(SystemContext), @alignOf(*anyopaque));
    const resources_end = resources_start + (@sizeOf(*anyopaque) * num_resources);
    const value_start = std.mem.alignForward(usize, resources_end, sys.system_alignment.toByteUnits());
    const value_end = value_start + sys.system_size;
    const buffer_len = value_end;
    const buffer = try all.alignedAlloc(u8, .of(SystemContext), buffer_len);
    errdefer all.free(buffer);

    const ctx: *SystemContext = std.mem.bytesAsValue(SystemContext, buffer);
    ctx.* = .{
        .group = group,
        .sys = sys,
        .weak = weak,
        .value_ptr = @ptrCast(@alignCast(&buffer[value_start])),
        .resources = @alignCast(std.mem.bytesAsSlice(*anyopaque, buffer[resources_start..resources_end])),
        .arena_allocator = .init(group.single_allocator.allocator()),
        .tracing_allocator = tracing_allocator,
    };

    for (sys.before) |dep| {
        const dep_sys = group.system_graph.systems.get(dep.system).?;
        ctx.addReference(dep_sys);
    }
    for (sys.after) |dep| {
        const dep_sys = group.system_graph.systems.get(dep.system).?;
        ctx.addReference(dep_sys);
    }

    _ = sys.references.fetchAdd(1, .monotonic);
    errdefer _ = sys.references.fetchSub(1, .monotonic);

    const factory = if (sys.factory) |f| f.ptr else null;
    if (!sys.system_init(factory, @ptrCast(ctx), ctx.value_ptr)) return error.InitError;

    return ctx;
}

pub fn deinit(self: *SystemContext) void {
    std.debug.assert(self.references.count() == 0);
    std.debug.assert(self.referenced_by.count() == 0);

    const info = self.sys;
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

pub fn addReference(self: *SystemContext, sys: *SystemContext) void {
    std.debug.assert(!self.references.contains(sys.sys));
    std.debug.assert(!sys.referenced_by.contains(self.sys));

    const all = self.tracing_allocator.allocator();
    const sys_all = sys.tracing_allocator.allocator();
    self.references.put(all, sys.sys, sys) catch @panic("oom");
    sys.referenced_by.put(sys_all, self.sys, self) catch @panic("oom");
}

pub fn removeReference(self: *SystemContext, sys: *SystemContext) void {
    std.debug.assert(self.references.contains(sys.sys));
    std.debug.assert(sys.referenced_by.contains(self.sys));

    _ = self.references.swapRemove(sys.sys);
    _ = sys.referenced_by.swapRemove(self.sys);
}

pub fn isUnloadable(self: *SystemContext) bool {
    return self.weak and self.referenced_by.count() == 0;
}

pub fn run(self: *SystemContext) void {
    self.deferred_fence.reset();
    for (self.deferred_dep.values()) |dep| dep.deferred_fence.wait(Universe.getInstance());
    const exclusive = self.resources[0..self.sys.exclusive_resources.len];
    const shared = self.resources[exclusive.len..self.sys.shared_resources.len];
    self.sys.system_run(self.value_ptr, exclusive.ptr, shared.ptr, &self.deferred_fence);
    _ = self.arena_allocator.reset(.free_all);

    if (self.merge_deferred) self.deferred_fence.wait(Universe.getInstance());
}

pub fn allocator(self: *SystemContext, strategy: AllocatorStrategy) Allocator {
    return switch (strategy) {
        .transient => self.arena_allocator.allocator(),
        .single_generation => self.group.single_allocator.allocator(),
        .multi_generation => self.group.multi_allocator.allocator(),
        .system_persistent => self.tracing_allocator.allocator(),
        else => @panic("unknown strategy"),
    };
}

pub fn appendWaiter(self: *SystemContext, fence: *Fence) void {
    const all = self.tracing_allocator.allocator();
    const waiter = all.create(DeinitWaiter) catch @panic("oom");
    waiter.* = .{ .fence = fence };
    self.waiters.prepend(&waiter.node);
}
