const std = @import("std");
const Allocator = std.mem.Allocator;
const MemoryPool = std.heap.MemoryPool;
const ArenaAllocator = std.heap.ArenaAllocator;
const DoublyLinkedList = std.DoublyLinkedList;
const ArrayListUnmanaged = std.ArrayListUnmanaged;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Pool = fimo_tasks_meta.pool.Pool;
const Entry = fimo_tasks_meta.command_buffer.Entry;
const OpaqueCommandBuffer = fimo_tasks_meta.command_buffer.OpaqueCommandBuffer;
const OpaqueTask = fimo_tasks_meta.task.OpaqueTask;
const Mutex = fimo_tasks_meta.sync.Mutex;
const fimo_worlds_meta = @import("fimo_worlds_meta");
const Job = fimo_worlds_meta.Job;
const Fence = Job.Fence;
const TimelineSemaphore = Job.TimelineSemaphore;

const heap = @import("heap.zig");
const SystemContext = @import("SystemContext.zig");
const Universe = @import("Universe.zig");
const Resource = Universe.Resource;
const System = Universe.System;
const World = @import("World.zig");

const SystemGroup = @This();

label: []u8,
world: *World,
executor: Pool,
generation: usize = 0,
system_graph: Graph = .{},
single_allocator: heap.SingleGenerationAllocator = .{},
multi_allocator: heap.MultiGenerationAllocator = .{},

const Graph = struct {
    mutex: Mutex = .{},
    dirty: bool = true,
    next_generation: usize = 0,
    schedule_semaphore: TimelineSemaphore = .{},
    entries: []Entry = &.{},
    resources: AutoArrayHashMapUnmanaged(*Resource, *anyopaque) = .empty,
    arena: ArenaAllocator = .init(std.heap.page_allocator),
    systems: AutoArrayHashMapUnmanaged(*System, *SystemContext) = .empty,
    deinit_list: DoublyLinkedList = .{},

    fn clearDeinitList(self: *Graph) void {
        while (self.deinit_list.popFirst()) |n| {
            const sys: *SystemContext = @fieldParentPtr("node", n);
            sys.deinit();
        }
    }

    fn addSystem(
        self: *Graph,
        sys: *System,
        universe: *const Universe,
        group: *SystemGroup,
        weak: bool,
    ) !void {
        if (self.systems.get(sys)) |ctx| {
            if (!weak) {
                std.debug.assert(ctx.weak);
                ctx.weak = false;
            }
            return;
        }

        var before_count: usize = 0;
        var after_count: usize = 0;
        errdefer {
            for (sys.after[0..after_count]) |sys2| {
                var fence = Fence{};
                self.removeSystem(sys2.system, &fence);
                fence.wait(Universe.getInstance());
            }
            for (sys.before[0..before_count]) |sys2| {
                var fence = Fence{};
                self.removeSystem(sys2.system, &fence);
                fence.wait(Universe.getInstance());
            }
        }

        for (sys.before) |before| {
            try self.addSystem(before.system, universe, group, true);
            before_count += 1;
        }
        for (sys.after) |after| {
            try self.addSystem(after.system, universe, group, true);
            after_count += 1;
        }

        const ctx = try SystemContext.init(group, sys, weak);
        errdefer ctx.deinit();
        try self.systems.put(universe.allocator, sys, ctx);
        self.dirty = true;
    }

    fn removeSystem(self: *Graph, sys: *System, fence: ?*Fence) void {
        const entry = self.systems.fetchSwapRemove(sys) orelse {
            std.debug.assert(fence == null);
            return;
        };

        const ctx = entry.value;
        while (ctx.references.count() != 0) {
            const dep = ctx.references.values()[0];
            ctx.removeReference(dep);
            if (ctx.isUnloadable()) self.removeSystem(dep.sys, null);
        }
        while (ctx.referenced_by.count() != 0) {
            const dep = ctx.referenced_by.values()[0];
            self.removeSystem(dep.sys, null);
        }

        if (!self.schedule_semaphore.isSignaled(@truncate(self.next_generation))) {
            if (fence) |f| ctx.appendWaiter(f);
            self.deinit_list.append(&ctx.node);
        } else {
            self.clearDeinitList();
            ctx.deinit();
            if (fence) |f| f.signal(Universe.getInstance());
        }
        self.dirty = true;
    }

    fn populateBuffers(self: *Graph) !void {
        if (!self.dirty) return;
        self.dirty = false;
        self.clearDeinitList();
        self.resources = .empty;
        _ = self.arena.reset(.retain_capacity);
        const group: *SystemGroup = @fieldParentPtr("system_graph", self);
        Universe.logDebug("repopulating buffers of {*}", .{group}, @src());

        const allocator = self.arena.allocator();
        for (self.systems.values()) |sys| {
            for (sys.sys.exclusive_resources) |res| try self.resources.put(allocator, res, undefined);
            for (sys.sys.shared_resources) |res| try self.resources.put(allocator, res, undefined);

            // The deferred pass can be merged if no other system depends on the deferred pass.
            sys.merge_deferred = blk: {
                for (sys.sys.before) |dep| if (dep.ignore_deferred) break :blk false;
                for (sys.referenced_by.values()) |by| {
                    for (by.sys.after) |dep| {
                        if (dep.system == sys.sys and dep.ignore_deferred) break :blk false;
                    }
                }
                break :blk true;
            };

            // Register the deferred pass dependencies.
            for (sys.sys.after) |dep| {
                if (dep.ignore_deferred) continue;
                try sys.deferred_dep.put(allocator, dep.system, self.systems.get(dep.system).?);
            }
            for (sys.referenced_by.values()) |by| {
                for (by.sys.before) |dep| {
                    if (dep.system != sys.sys or dep.ignore_deferred)
                        continue;
                    try sys.deferred_dep.put(allocator, dep.system, self.systems.get(dep.system).?);
                }
            }

            // Allocate the array to store the resource pointers.
            const num_resources = sys.sys.exclusive_resources.len + sys.sys.shared_resources.len;
            sys.resources = try allocator.alloc(*anyopaque, num_resources);
        }

        const TaskInfo = struct {
            system: *SystemContext,
            generation: usize,
            task: OpaqueTask = undefined,

            fn taskStart(task: *OpaqueTask) callconv(.c) void {
                const info: *@This() = @fieldParentPtr("task", task);
                const sys = info.system;
                sys.run();
            }

            fn generationsort(ctx: void, a: @This(), b: @This()) bool {
                _ = ctx;
                return a.generation < b.generation;
            }

            fn toposort(
                graph: *Graph,
                tasks: *ArrayListUnmanaged(@This()),
                markers: *AutoArrayHashMapUnmanaged(*System, usize),
                handle: *System,
                sys: *SystemContext,
            ) usize {
                if (markers.get(handle)) |idx| return idx;
                var generation: usize = 0;
                for (sys.sys.after) |dep| {
                    const dep_sys = graph.systems.get(dep.system).?;
                    const idx = toposort(graph, tasks, markers, dep.system, dep_sys);
                    generation = @max(generation, tasks.items[idx].generation + 1);
                }
                for (sys.referenced_by.values()) |by| {
                    for (by.sys.before) |dep| {
                        if (dep.system != sys.sys) continue;
                        const dep_sys = graph.systems.get(dep.system).?;
                        const idx = toposort(graph, tasks, markers, dep.system, dep_sys);
                        generation = @max(generation, tasks.items[idx].generation + 1);
                    }
                }

                const idx = tasks.items.len;
                tasks.appendAssumeCapacity(.{
                    .system = sys,
                    .generation = generation,
                    .task = .{
                        .label_ = sys.sys.label.ptr,
                        .label_len = sys.sys.label.len,
                        .on_start = &taskStart,
                        .state = {},
                    },
                });
                return idx;
            }
        };

        // Perform a topological sort to find a correct execution order for the systems.
        var systems_it = self.systems.iterator();
        var markers = AutoArrayHashMapUnmanaged(*System, usize).empty;
        try markers.ensureTotalCapacity(allocator, self.systems.count());
        var tasks = try ArrayListUnmanaged(TaskInfo).initCapacity(allocator, self.systems.count());
        while (systems_it.next()) |entry| {
            const id = entry.key_ptr.*;
            const sys = entry.value_ptr.*;
            _ = TaskInfo.toposort(self, &tasks, &markers, id, sys);
        }

        // Now that the tasks are sorted topologically we can reorder them using the generation.
        // Systems assigned to the same generation can not depend on each other.
        std.mem.sortUnstable(TaskInfo, tasks.items, {}, TaskInfo.generationsort);

        // Insert synchronization entries between the tasks.
        const ResourceInfo = struct {
            exclusive: bool = false,
            referenced_by: ArrayListUnmanaged(*System) = .empty,
        };
        const resource_infos = try allocator.alloc(ResourceInfo, self.resources.count());
        for (resource_infos) |*info| info.* = .{};

        var running_systems = AutoArrayHashMapUnmanaged(*System, usize).empty;
        var entries = try ArrayListUnmanaged(Entry).initCapacity(allocator, self.systems.count());
        for (tasks.items) |*task| {
            // Insert synchronization points if the system depends on other systems.
            for (task.system.sys.after) |dep| {
                const entry = running_systems.fetchSwapRemove(dep.system) orelse continue;
                const entry_idx = entry.value;
                try entries.append(allocator, Entry{
                    .tag = .wait_on_command_indirect,
                    .payload = .{
                        .wait_on_command_indirect = entries.items.len - entry_idx,
                    },
                });
            }
            for (task.system.referenced_by.values()) |by| {
                for (by.sys.before) |dep| {
                    if (dep.system != task.system.sys) continue;
                    const entry = running_systems.fetchSwapRemove(dep.system) orelse continue;
                    const entry_idx = entry.value;
                    try entries.append(allocator, Entry{
                        .tag = .wait_on_command_indirect,
                        .payload = .{
                            .wait_on_command_indirect = entries.items.len - entry_idx,
                        },
                    });
                }
            }

            // For exclusive resources we synchronize unconditionally.
            for (task.system.sys.exclusive_resources) |res| {
                const res_idx = self.resources.getIndex(res).?;
                const res_info = &resource_infos[res_idx];
                for (res_info.referenced_by.items) |id| {
                    const entry = running_systems.fetchSwapRemove(id) orelse continue;
                    const entry_idx = entry.value;
                    try entries.append(allocator, Entry{
                        .tag = .wait_on_command_indirect,
                        .payload = .{
                            .wait_on_command_indirect = entries.items.len - entry_idx,
                        },
                    });
                }
                res_info.exclusive = true;
                res_info.referenced_by.clearRetainingCapacity();
                try res_info.referenced_by.append(allocator, task.system.sys);
            }

            // Shared resources are synchronized if there is a writer.
            for (task.system.sys.shared_resources) |res| {
                const res_idx = self.resources.getIndex(res).?;
                const res_info = &resource_infos[res_idx];
                if (res_info.exclusive) {
                    for (res_info.referenced_by.items) |id| {
                        const entry = running_systems.fetchSwapRemove(id) orelse continue;
                        const entry_idx = entry.value;
                        try entries.append(allocator, Entry{
                            .tag = .wait_on_command_indirect,
                            .payload = .{
                                .wait_on_command_indirect = entries.items.len - entry_idx,
                            },
                        });
                    }
                    res_info.referenced_by.clearRetainingCapacity();
                }
                res_info.exclusive = false;
                try res_info.referenced_by.append(allocator, task.system.sys);
            }

            const entry_idx = entries.items.len;
            try running_systems.put(allocator, task.system.sys, entry_idx);
            try entries.append(allocator, Entry{
                .tag = .enqueue_task,
                .payload = .{ .enqueue_task = &task.task },
            });
        }
        self.entries = entries.items;
    }

    fn acquireResources(self: *Graph) void {
        const group: *SystemGroup = @fieldParentPtr("system_graph", self);
        group.world.lockResources(self.resources.keys(), &.{}, self.resources.values());
        for (self.systems.values()) |sys| {
            for (sys.sys.exclusive_resources, 0..) |id, i| {
                sys.resources[i] = self.resources.get(id) orelse unreachable;
            }
            for (sys.sys.shared_resources, sys.sys.exclusive_resources.len..) |id, i| {
                sys.resources[i] = self.resources.get(id) orelse unreachable;
            }
        }
    }
};

pub const InitOptions = struct {
    label: ?[]const u8 = null,
    executor: ?Pool = null,
    world: *World,
};

pub fn init(options: InitOptions) !*SystemGroup {
    const universe = Universe.getUniverse();
    const allocator = universe.allocator;

    const label = try allocator.dupe(u8, options.label orelse "<unlabelled>");
    errdefer allocator.free(label);
    const executor = if (options.executor) |ex| ex.ref() else options.world.executor.ref();
    errdefer executor.unref();

    const group = try allocator.create(SystemGroup);
    group.* = .{ .label = label, .world = options.world, .executor = executor };
    _ = options.world.system_group_count.fetchAdd(1, .monotonic);
    Universe.logDebug(
        "created `{*}`, label=`{s}`, world=`{*}`, executor=`{}`",
        .{ group, label, options.world, executor.id() },
        @src(),
    );
    return group;
}

pub fn deinit(self: *SystemGroup) void {
    Universe.logDebug("destroying `{*}`", .{self}, @src());
    const instance = Universe.getInstance();
    self.system_graph.mutex.lock(instance);

    if (!self.system_graph.schedule_semaphore.isSignaled(@truncate(
        self.system_graph.next_generation,
    ))) @panic("system group still running");
    if (self.system_graph.systems.count() != 0) @panic("system group not empty");

    const universe = Universe.getUniverse();
    const allocator = universe.allocator;

    allocator.free(self.label);
    _ = self.world.system_group_count.fetchSub(1, .monotonic);
    self.executor.unref();
    self.system_graph.systems.clearAndFree(allocator);
    self.system_graph.arena.deinit();
    self.single_allocator.deinit();
    self.multi_allocator.deinit();
    allocator.destroy(self);
}

pub fn addSystems(self: *SystemGroup, handles: []const *System) !void {
    Universe.logDebug("adding `{any}` to `{*}`", .{ handles, self }, @src());
    const instance = Universe.getInstance();
    self.system_graph.mutex.lock(instance);
    defer self.system_graph.mutex.unlock(instance);

    const universe = Universe.getUniverse();
    universe.rwlock.lockRead(instance);
    defer universe.rwlock.unlockRead(instance);

    for (handles) |handle| {
        if (!universe.systems.contains(handle)) @panic("invalid system");
        if (self.system_graph.systems.get(handle)) |sys| {
            if (!sys.weak) return error.Duplicate;
        }
    }
    for (handles, 0..) |handle, i| {
        errdefer for (handles[0..i]) |id2| {
            var fence = Fence{};
            self.system_graph.removeSystem(id2, &fence);
            fence.wait(instance);
        };
        try self.system_graph.addSystem(handle, universe, self, false);
    }
}

pub fn removeSystem(self: *SystemGroup, handle: *System, fence: *Fence) void {
    Universe.logDebug("removing `{}` from `{*}`", .{ handle, self }, @src());
    const instance = Universe.getInstance();
    self.system_graph.mutex.lock(instance);
    defer self.system_graph.mutex.unlock(instance);

    const sys = self.system_graph.systems.get(handle) orelse @panic("invalid system");
    if (sys.weak) @panic("invalid system");
    self.system_graph.removeSystem(handle, fence);
}

pub fn schedule(self: *SystemGroup, fences: []const *Fence, fence: ?*Fence) !void {
    const instance = Universe.getInstance();
    self.system_graph.mutex.lock(instance);
    defer self.system_graph.mutex.unlock(instance);
    const generation = self.system_graph.next_generation;
    Universe.logDebug(
        "scheduling generation {} of `{*}`",
        .{ generation, self },
        @src(),
    );

    const universe = Universe.getUniverse();
    try Job.go(
        instance,
        run,
        .{ self, generation },
        .{
            .executor = self.executor,
            .allocator = universe.allocator,
            .label = self.label,
            .fences = fences,
            .semaphores = &.{.{
                .semaphore = &self.system_graph.schedule_semaphore,
                .counter = generation,
            }},
            .signal = if (fence) |f| .{ .fence = f } else null,
        },
    );
    self.system_graph.next_generation +%= 1;
}

fn run(self: *SystemGroup, generation: usize) void {
    std.debug.assert(self.generation == generation);
    Universe.logDebug("running generation `{}` of `{*}`", .{ generation, self }, @src());
    const instance = Universe.getInstance();
    {
        self.system_graph.mutex.lock(instance);
        defer self.system_graph.mutex.unlock(instance);
        self.system_graph.populateBuffers() catch |err| @panic(@errorName(err));
        self.system_graph.acquireResources();
    }

    var e: ?AnyError = null;
    const allocator = self.single_allocator.allocator();
    const label = std.fmt.allocPrint(allocator, "{*} systems", .{self}) catch @panic("oom");
    var buffer = OpaqueCommandBuffer{
        .label_ = label.ptr,
        .label_len = label.len,
        .entries_ = self.system_graph.entries.ptr,
        .entries_len = self.system_graph.entries.len,
        .state = {},
    };
    const handle = self.executor.enqueueCommandBuffer(&buffer, &e) catch |err| @panic(@errorName(err));
    _ = handle.waitOn();
    handle.unref();

    var it = self.system_graph.resources.iterator();
    while (it.next()) |res| {
        res.value_ptr.* = undefined;
        self.world.unlockResourceExclusive(res.key_ptr.*);
    }

    self.generation +%= 1;
    self.single_allocator.endGeneration();
    self.multi_allocator.endGeneration();
    self.system_graph.schedule_semaphore.signal(instance, @truncate(self.generation));
}
