const std = @import("std");
const atomic = std.atomic;
const builtin = @import("builtin");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const Status = fimo_std.Context.Status;
const Module = fimo_std.Context.Module;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const pools = fimo_tasks_meta.pool;
const fimo_worlds_meta = @import("fimo_worlds_meta");
const Fence = fimo_worlds_meta.Job.Fence;
const resources = fimo_worlds_meta.resources;
const systems = fimo_worlds_meta.systems;
const worlds = fimo_worlds_meta.worlds;
const symbols = fimo_worlds_meta.symbols;

const System = @import("System.zig");
const SystemGroup = @import("SystemGroup.zig");
const Universe = @import("Universe.zig");
const World = @import("World.zig");

pub const Instance = blk: {
    @setEvalBranchQuota(100000);
    break :blk Module.exports.Builder.init("fimo_worlds")
        .withDescription("Multi-threaded state processing")
        .withLicense("MIT OR APACHE 2.0")
        .withMultipleImports(fimo_tasks_meta.symbols.all_symbols)
        .withExport(.{ .symbol = symbols.resource_register }, &resourceRegister)
        .withExport(.{ .symbol = symbols.resource_unregister }, &resourceUnregister)
        .withExport(.{ .symbol = symbols.system_register }, &systemRegister)
        .withExport(.{ .symbol = symbols.system_unregister }, &systemUnregister)
        .withExport(.{ .symbol = symbols.system_group_create }, &systemGroupCreate)
        .withExport(.{ .symbol = symbols.system_group_destroy }, &systemGroupDestroy)
        .withExport(.{ .symbol = symbols.system_group_get_world }, &systemGroupGetWorld)
        .withExport(.{ .symbol = symbols.system_group_get_label }, &systemGroupGetLabel)
        .withExport(.{ .symbol = symbols.system_group_get_pool }, &systemGroupGetPool)
        .withExport(.{ .symbol = symbols.system_group_add_systems }, &systemGroupAddSystems)
        .withExport(.{ .symbol = symbols.system_group_remove_system }, &systemGroupRemoveSystem)
        .withExport(.{ .symbol = symbols.system_group_schedule }, &systemGroupSchedule)
        .withExport(.{ .symbol = symbols.system_context_get_group }, &systemContextGetGroup)
        .withExport(.{ .symbol = symbols.system_context_get_generation }, &systemContextGetGeneration)
        .withExport(.{ .symbol = symbols.system_context_allocator_alloc }, &systemContextAllocatorAlloc)
        .withExport(.{ .symbol = symbols.system_context_allocator_resize }, &systemContextAllocatorResize)
        .withExport(.{ .symbol = symbols.system_context_allocator_remap }, &systemContextAllocatorRemap)
        .withExport(.{ .symbol = symbols.system_context_allocator_free }, &systemContextAllocatorFree)
        .withExport(.{ .symbol = symbols.world_create }, &worldCreate)
        .withExport(.{ .symbol = symbols.world_destroy }, &worldDestroy)
        .withExport(.{ .symbol = symbols.world_get_label }, &worldGetLabel)
        .withExport(.{ .symbol = symbols.world_get_pool }, &worldGetPool)
        .withExport(.{ .symbol = symbols.world_has_resource }, &worldHasResource)
        .withExport(.{ .symbol = symbols.world_add_resource }, &worldAddResource)
        .withExport(.{ .symbol = symbols.world_remove_resource }, &worldRemoveResource)
        .withExport(.{ .symbol = symbols.world_lock_resources }, &worldLockResources)
        .withExport(.{ .symbol = symbols.world_unlock_resource_exclusive }, &worldUnlockResourceExclusive)
        .withExport(.{ .symbol = symbols.world_unlock_resource_shared }, &worldUnlockResourceShared)
        .withExport(.{ .symbol = symbols.world_allocator_alloc }, &worldAllocatorAlloc)
        .withExport(.{ .symbol = symbols.world_allocator_resize }, &worldAllocatorResize)
        .withExport(.{ .symbol = symbols.world_allocator_remap }, &worldAllocatorRemap)
        .withExport(.{ .symbol = symbols.world_allocator_free }, &worldAllocatorFree)
        .withStateSync(State, State.init, State.deinit)
        .exportModule();
};

pub fn getInstance() *const Instance {
    return State.global_instance.load(.monotonic) orelse unreachable;
}

comptime {
    _ = Instance;
}

const State = struct {
    debug_allocator: switch (builtin.mode) {
        .Debug, .ReleaseSafe => std.heap.DebugAllocator(.{}),
        else => void,
    },
    universe: Universe = undefined,

    var global_state: State = undefined;
    var global_instance: atomic.Value(?*const Instance) = .init(null);

    fn init(octx: *const Module.OpaqueInstance, set: Module.LoadingSet) !*State {
        _ = set;
        const ctx: *const Instance = @ptrCast(@alignCast(octx));
        if (global_instance.cmpxchgStrong(null, ctx, .monotonic, .monotonic)) |_| {
            ctx.context().tracing().emitErrSimple(
                "`fimo_worlds` is already initialized",
                .{},
                @src(),
            );
            return error.AlreadyInitialized;
        }

        const allocator = switch (builtin.mode) {
            .Debug, .ReleaseSafe => blk: {
                global_state.debug_allocator = .init;
                break :blk global_state.debug_allocator.allocator();
            },
            else => std.heap.smp_allocator,
        };
        global_state.universe = .{ .allocator = allocator };

        return &global_state;
    }

    fn deinit(octx: *const Module.OpaqueInstance, state: *State) void {
        const ctx: *const Instance = @ptrCast(@alignCast(octx));
        if (global_instance.cmpxchgStrong(ctx, null, .monotonic, .monotonic)) |_|
            @panic("already deinit");
        std.debug.assert(state == &global_state);

        global_state.universe.deinit();
        switch (builtin.mode) {
            .Debug, .ReleaseSafe => {
                if (global_state.debug_allocator.deinit() == .leak) @panic("memory leak");
            },
            else => {},
        }
        global_state = undefined;
    }
};

fn resourceRegister(
    resource: *const resources.RegisterOptions.Descriptor,
    id: *resources.ResourceId,
) callconv(.c) Status {
    id.* = State.global_state.universe.registerResource(.{
        .label = if (resource.label_len != 0) resource.label.?[0..resource.label_len] else null,
        .size = resource.size,
        .alignment = .fromByteUnits(resource.alignment),
    }) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    return .ok;
}

fn resourceUnregister(id: resources.ResourceId) callconv(.c) void {
    State.global_state.universe.unregisterResource(id);
}

fn systemRegister(
    system: *const systems.System.Descriptor,
    id: *systems.SystemId,
) callconv(.c) Status {
    id.* = State.global_state.universe.registerSystem(.{
        .label = if (system.label_len != 0) system.label.?[0..system.label_len] else null,
        .exclusive_resources = if (system.exclusive_ids_len != 0)
            system.exclusive_ids.?[0..system.exclusive_ids_len]
        else
            &.{},
        .shared_resources = if (system.shared_ids_len != 0)
            system.shared_ids.?[0..system.shared_ids_len]
        else
            &.{},
        .before = if (system.before_len != 0)
            system.before.?[0..system.before_len]
        else
            &.{},
        .after = if (system.after_len != 0)
            system.after.?[0..system.after_len]
        else
            &.{},
        .factory = if (system.factory_size != 0)
            @as([*]const u8, @ptrCast(system.factory.?))[0..system.factory_size]
        else
            null,
        .factory_alignment = .fromByteUnits(system.factory_alignment),
        .factory_deinit = system.factory_deinit,
        .system_size = system.system_size,
        .system_alignment = .fromByteUnits(system.system_alignment),
        .system_init = system.system_init,
        .system_deinit = system.system_deinit,
        .system_run = system.system_run,
    }) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    return .ok;
}

fn systemUnregister(id: systems.SystemId) callconv(.c) void {
    State.global_state.universe.unregisterSystem(id);
}

fn systemGroupCreate(
    descriptor: *const systems.SystemGroup.CreateOptions.Descriptor,
    group: **systems.SystemGroup,
) callconv(.c) Status {
    const grp = SystemGroup.init(.{
        .label = if (descriptor.label_len != 0) descriptor.label.?[0..descriptor.label_len] else null,
        .executor = if (descriptor.pool) |p| p.* else null,
        .world = @ptrCast(@alignCast(descriptor.world)),
    }) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    group.* = @ptrCast(grp);
    return .ok;
}

fn systemGroupDestroy(group: *systems.SystemGroup) callconv(.c) void {
    const grp: *SystemGroup = @ptrCast(@alignCast(group));
    grp.deinit();
}

fn systemGroupGetWorld(group: *systems.SystemGroup) callconv(.c) *worlds.World {
    const grp: *SystemGroup = @ptrCast(@alignCast(group));
    return @ptrCast(grp.world);
}

fn systemGroupGetLabel(group: *systems.SystemGroup, len: *usize) callconv(.c) ?[*]const u8 {
    const grp: *SystemGroup = @ptrCast(@alignCast(group));
    len.* = grp.label.len;
    return grp.label.ptr;
}

fn systemGroupGetPool(group: *systems.SystemGroup) callconv(.c) pools.Pool {
    const grp: *SystemGroup = @ptrCast(@alignCast(group));
    return grp.executor.ref();
}

fn systemGroupAddSystems(
    group: *systems.SystemGroup,
    sys: ?[*]const systems.SystemId,
    len: usize,
) callconv(.c) Status {
    const ids = if (len != 0) sys.?[0..len] else &.{};
    const grp: *SystemGroup = @ptrCast(@alignCast(group));
    grp.addSystems(ids) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    return .ok;
}

fn systemGroupRemoveSystem(
    group: *systems.SystemGroup,
    id: systems.SystemId,
    signal: *Fence,
) callconv(.c) void {
    const grp: *SystemGroup = @ptrCast(@alignCast(group));
    grp.removeSystem(id, signal);
}

fn systemGroupSchedule(
    group: *systems.SystemGroup,
    wait_on: ?[*]const *Fence,
    wait_on_len: usize,
    signal: ?*Fence,
) callconv(.c) Status {
    const fences = if (wait_on_len != 0) wait_on.?[0..wait_on_len] else &.{};
    const grp: *SystemGroup = @ptrCast(@alignCast(group));
    grp.schedule(fences, signal) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    return .ok;
}

fn systemContextGetGroup(context: *systems.SystemContext) callconv(.c) *systems.SystemGroup {
    const sys: *System = @ptrCast(@alignCast(context));
    return @ptrCast(sys.group);
}

fn systemContextGetGeneration(context: *systems.SystemContext) callconv(.c) usize {
    const sys: *System = @ptrCast(@alignCast(context));
    return sys.group.generation;
}

fn systemContextAllocatorAlloc(
    context: *systems.SystemContext,
    strategy: systems.AllocatorStrategy,
    len: usize,
    alignment: usize,
    ret_addr: usize,
) callconv(.c) ?[*]u8 {
    const sys: *System = @ptrCast(@alignCast(context));
    return sys.allocator(strategy).rawAlloc(len, .fromByteUnits(alignment), ret_addr);
}

fn systemContextAllocatorResize(
    context: *systems.SystemContext,
    strategy: systems.AllocatorStrategy,
    ptr: ?[*]u8,
    len: usize,
    alignment: usize,
    new_len: usize,
    ret_addr: usize,
) callconv(.c) bool {
    const sys: *System = @ptrCast(@alignCast(context));
    const memory: []u8 = if (ptr) |p| p[0..len] else &.{};
    return sys.allocator(strategy).rawResize(memory, .fromByteUnits(alignment), new_len, ret_addr);
}

fn systemContextAllocatorRemap(
    context: *systems.SystemContext,
    strategy: systems.AllocatorStrategy,
    ptr: ?[*]u8,
    len: usize,
    alignment: usize,
    new_len: usize,
    ret_addr: usize,
) callconv(.c) ?[*]u8 {
    const sys: *System = @ptrCast(@alignCast(context));
    const memory: []u8 = if (ptr) |p| p[0..len] else &.{};
    return sys.allocator(strategy).rawRemap(memory, .fromByteUnits(alignment), new_len, ret_addr);
}

fn systemContextAllocatorFree(
    context: *systems.SystemContext,
    strategy: systems.AllocatorStrategy,
    ptr: ?[*]u8,
    len: usize,
    alignment: usize,
    ret_addr: usize,
) callconv(.c) void {
    const sys: *System = @ptrCast(@alignCast(context));
    const memory: []u8 = if (ptr) |p| p[0..len] else &.{};
    sys.allocator(strategy).rawFree(memory, .fromByteUnits(alignment), ret_addr);
}

fn worldCreate(
    descriptor: *const worlds.CreateOptions.Descriptor,
    world: **worlds.World,
) callconv(.c) Status {
    const w = World.init(.{
        .label = if (descriptor.label_len != 0) descriptor.label.?[0..descriptor.label_len] else null,
        .executor = if (descriptor.pool) |p| p.* else null,
    }) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    world.* = @ptrCast(w);
    return .ok;
}

fn worldDestroy(world: *worlds.World) callconv(.c) void {
    const w: *World = @ptrCast(@alignCast(world));
    w.deinit();
}

fn worldGetLabel(world: *worlds.World, len: *usize) callconv(.c) ?[*]const u8 {
    const w: *World = @ptrCast(@alignCast(world));
    len.* = w.label.len;
    return w.label.ptr;
}

fn worldGetPool(world: *worlds.World) callconv(.c) pools.Pool {
    const w: *World = @ptrCast(@alignCast(world));
    return w.executor.ref();
}

fn worldHasResource(world: *worlds.World, id: resources.ResourceId) callconv(.c) bool {
    const w: *World = @ptrCast(@alignCast(world));
    return w.hasResource(id);
}

fn worldAddResource(
    world: *worlds.World,
    id: resources.ResourceId,
    value: *const anyopaque,
) callconv(.c) Status {
    const w: *World = @ptrCast(@alignCast(world));
    w.addResource(id, value) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    return .ok;
}

fn worldRemoveResource(
    world: *worlds.World,
    id: resources.ResourceId,
    value: *anyopaque,
) callconv(.c) Status {
    const w: *World = @ptrCast(@alignCast(world));
    w.removeResource(id, value) catch |err| {
        getInstance().context().setResult(.initErr(.initError(err)));
        return .err;
    };
    return .ok;
}

fn worldLockResources(
    world: *worlds.World,
    exclusive_ids: [*]const resources.ResourceId,
    exclusive_ids_len: usize,
    shared_ids: [*]const resources.ResourceId,
    shared_ids_len: usize,
    out_resources: [*]*anyopaque,
) callconv(.c) void {
    const len = exclusive_ids_len + shared_ids_len;
    const exclusive = exclusive_ids[0..exclusive_ids_len];
    const shared = shared_ids[0..shared_ids_len];
    const out = out_resources[0..len];

    const w: *World = @ptrCast(@alignCast(world));
    w.lockResources(exclusive, shared, out);
}

fn worldUnlockResourceExclusive(world: *worlds.World, id: resources.ResourceId) callconv(.c) void {
    const w: *World = @ptrCast(@alignCast(world));
    w.unlockResourceExclusive(id);
}

fn worldUnlockResourceShared(world: *worlds.World, id: resources.ResourceId) callconv(.c) void {
    const w: *World = @ptrCast(@alignCast(world));
    w.unlockResourceShared(id);
}

fn worldAllocatorAlloc(
    world: *worlds.World,
    len: usize,
    alignment: usize,
    ret_addr: usize,
) callconv(.c) ?[*]u8 {
    const w: *World = @ptrCast(@alignCast(world));
    return w.allocator.allocator().rawAlloc(len, .fromByteUnits(alignment), ret_addr);
}

fn worldAllocatorResize(
    world: *worlds.World,
    ptr: ?[*]u8,
    len: usize,
    alignment: usize,
    new_len: usize,
    ret_addr: usize,
) callconv(.c) bool {
    const w: *World = @ptrCast(@alignCast(world));
    const memory: []u8 = if (ptr) |p| p[0..len] else &.{};
    return w.allocator.allocator().rawResize(memory, .fromByteUnits(alignment), new_len, ret_addr);
}

fn worldAllocatorRemap(
    world: *worlds.World,
    ptr: ?[*]u8,
    len: usize,
    alignment: usize,
    new_len: usize,
    ret_addr: usize,
) callconv(.c) ?[*]u8 {
    const w: *World = @ptrCast(@alignCast(world));
    const memory: []u8 = if (ptr) |p| p[0..len] else &.{};
    return w.allocator.allocator().rawRemap(memory, .fromByteUnits(alignment), new_len, ret_addr);
}

fn worldAllocatorFree(
    world: *worlds.World,
    ptr: ?[*]u8,
    len: usize,
    alignment: usize,
    ret_addr: usize,
) callconv(.c) void {
    const w: *World = @ptrCast(@alignCast(world));
    const memory: []u8 = if (ptr) |p| p[0..len] else &.{};
    w.allocator.allocator().rawFree(memory, .fromByteUnits(alignment), ret_addr);
}
