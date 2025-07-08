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

    fn addSystem(self: *Graph, sys: *System, weak: bool) !*SystemContext {
        if (self.systems.get(sys)) |ctx| {
            if (!weak) {
                std.debug.assert(ctx.weak);
                ctx.weak = false;
            }
            return ctx;
        }

        const instance = Universe.getInstance();
        sys.rwlock.lockRead(instance);
        defer sys.rwlock.unlockRead(instance);

        var before_count: usize = 0;
        var after_count: usize = 0;
        errdefer {
            var i: usize = 0;
            for (sys.after.keys(), sys.after.values()) |dep, link| {
                if (i > after_count) break;
                if (link.implicit or link.weak) continue;
                const ctx = self.systems.get(dep) orelse break;
                ctx.reference_count -= 1;
                if (ctx.reference_count == 0 and ctx.weak)
                    self.removeSystem(ctx.sys, null, false);
                i += 1;
            }

            i = 0;
            for (sys.before.keys(), sys.before.values()) |dep, link| {
                if (i > before_count) break;
                if (link.implicit or link.weak) continue;
                const ctx = self.systems.get(dep) orelse break;
                ctx.reference_count -= 1;
                if (ctx.reference_count == 0 and ctx.weak)
                    self.removeSystem(ctx.sys, null, false);
                i += 1;
            }
        }

        for (sys.before.keys(), sys.before.values()) |dep, link| {
            if (link.implicit or link.weak) continue;
            const ctx = try self.addSystem(dep, true);
            ctx.reference_count += 1;
            before_count += 1;
        }
        for (sys.after.keys(), sys.after.values()) |dep, link| {
            if (link.implicit or link.weak) continue;
            const ctx = try self.addSystem(dep, true);
            ctx.reference_count += 1;
            after_count += 1;
        }

        const universe = Universe.getUniverse();
        const group: *SystemGroup = @fieldParentPtr("system_graph", self);
        const ctx = try SystemContext.init(group, sys, weak);
        errdefer ctx.deinit();
        try self.systems.put(universe.allocator, sys, ctx);
        self.dirty = true;
        return ctx;
    }

    fn removeSystem(self: *Graph, sys: *System, fence: ?*Fence, allow_deferred: bool) void {
        const entry = self.systems.fetchSwapRemove(sys) orelse {
            std.debug.assert(fence == null);
            return;
        };

        const instance = Universe.getInstance();
        var queue = DoublyLinkedList{};
        var deinit_stack = DoublyLinkedList{};

        queue.append(&entry.value.node);
        while (queue.popFirst()) |node| {
            const ctx: *SystemContext = @fieldParentPtr("node", node);
            ctx.sys.rwlock.lockRead(instance);
            defer ctx.sys.rwlock.unlockRead(instance);
            for (ctx.sys.before.keys(), ctx.sys.before.values()) |dep, link| {
                if (link.implicit or link.weak) continue;
                const dep_ctx = self.systems.get(dep) orelse unreachable;
                dep_ctx.reference_count -= 1;
                if (dep_ctx.reference_count == 0 and dep_ctx.weak) queue.append(&dep_ctx.node);
            }
            for (ctx.sys.after.keys(), ctx.sys.after.values()) |dep, link| {
                if (link.implicit or link.weak) continue;
                const dep_ctx = self.systems.get(dep) orelse unreachable;
                dep_ctx.reference_count -= 1;
                if (dep_ctx.reference_count == 0 and dep_ctx.weak) queue.append(&dep_ctx.node);
            }
            deinit_stack.append(&ctx.node);
        }

        while (deinit_stack.pop()) |node| {
            const ctx: *SystemContext = @fieldParentPtr("node", node);
            if (allow_deferred and !self.schedule_semaphore.isSignaled(@truncate(self.next_generation))) {
                if (ctx.sys == sys) if (fence) |f| ctx.appendWaiter(f);
                self.deinit_list.append(&ctx.node);
            } else {
                self.clearDeinitList();
                ctx.deinit();
                if (ctx.sys == sys) if (fence) |f| f.signal(instance);
            }
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

        const instance = Universe.getInstance();
        const allocator = self.arena.allocator();
        for (self.systems.values()) |ctx| {
            for (ctx.sys.exclusive_resources) |res| try self.resources.put(allocator, res, undefined);
            for (ctx.sys.shared_resources) |res| try self.resources.put(allocator, res, undefined);

            ctx.sys.rwlock.lockRead(instance);
            defer ctx.sys.rwlock.unlockRead(instance);

            // The deferred pass can be merged if no other system depends on the deferred pass.
            ctx.merge_deferred = blk: {
                for (ctx.sys.before.values()) |link| if (link.ignore_deferred) break :blk false;
                break :blk true;
            };

            // Register the deferred pass dependencies.
            for (ctx.sys.before.keys(), ctx.sys.before.values()) |dep, link| {
                if (link.ignore_deferred) continue;
                const dep_ctx = self.systems.get(dep) orelse {
                    std.debug.assert(link.implicit or link.weak);
                    continue;
                };
                try ctx.deferred_dep.put(allocator, dep, dep_ctx);
            }

            // Allocate the array to store the resource pointers.
            const num_resources = ctx.sys.exclusive_resources.len + ctx.sys.shared_resources.len;
            ctx.resources = try allocator.alloc(*anyopaque, num_resources);
        }

        const TaskInfo = struct {
            ctx: *SystemContext,
            generation: usize,
            task: OpaqueTask = undefined,

            fn taskStart(task: *OpaqueTask) callconv(.c) void {
                const info: *@This() = @fieldParentPtr("task", task);
                const sys = info.ctx;
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
                ctx: *SystemContext,
            ) usize {
                if (markers.get(handle)) |idx| return idx;
                var generation: usize = 0;
                ctx.sys.rwlock.lockRead(Universe.getInstance());
                defer ctx.sys.rwlock.unlockRead(Universe.getInstance());
                for (ctx.sys.before.keys(), ctx.sys.before.values()) |dep, link| {
                    const dep_ctx = graph.systems.get(dep) orelse {
                        std.debug.assert(link.implicit or link.weak);
                        continue;
                    };
                    const idx = toposort(graph, tasks, markers, dep, dep_ctx);
                    generation = @max(generation, tasks.items[idx].generation + 1);
                }

                const idx = tasks.items.len;
                tasks.appendAssumeCapacity(.{
                    .ctx = ctx,
                    .generation = generation,
                    .task = .{
                        .label_ = ctx.sys.label.ptr,
                        .label_len = ctx.sys.label.len,
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
            task.ctx.sys.rwlock.lockRead(instance);
            defer task.ctx.sys.rwlock.unlockRead(instance);

            // Insert synchronization points if the system depends on other systems.
            for (task.ctx.sys.before.keys()) |dep| {
                const entry = running_systems.fetchSwapRemove(dep) orelse continue;
                const entry_idx = entry.value;
                try entries.append(allocator, Entry{
                    .tag = .wait_on_command_indirect,
                    .payload = .{
                        .wait_on_command_indirect = entries.items.len - entry_idx,
                    },
                });
            }

            // For exclusive resources we synchronize unconditionally.
            for (task.ctx.sys.exclusive_resources) |res| {
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
                try res_info.referenced_by.append(allocator, task.ctx.sys);
            }

            // Shared resources are synchronized if there is a writer.
            for (task.ctx.sys.shared_resources) |res| {
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
                try res_info.referenced_by.append(allocator, task.ctx.sys);
            }

            const entry_idx = entries.items.len;
            try running_systems.put(allocator, task.ctx.sys, entry_idx);
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
        errdefer for (handles[0..i]) |id2| self.system_graph.removeSystem(id2, null, false);
        _ = try self.system_graph.addSystem(handle, false);
    }
}

pub fn removeSystem(self: *SystemGroup, handle: *System, fence: *Fence) void {
    Universe.logDebug("removing `{}` from `{*}`", .{ handle, self }, @src());
    const instance = Universe.getInstance();
    self.system_graph.mutex.lock(instance);
    defer self.system_graph.mutex.unlock(instance);

    const sys = self.system_graph.systems.get(handle) orelse @panic("invalid system");
    if (sys.weak) @panic("invalid system");
    self.system_graph.removeSystem(handle, fence, true);
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
