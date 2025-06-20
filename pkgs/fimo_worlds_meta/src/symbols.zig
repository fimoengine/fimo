const c = @import("c");
const fimo_std = @import("fimo_std");
const Symbol = fimo_std.Context.Module.Symbol;
const Status = fimo_std.Context.Status;
const context_version = fimo_std.Context.context_version;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const Pool = fimo_tasks_meta.pool.Pool;

const Job = @import("Job.zig");
const Fence = Job.Fence;
const resources = @import("resources.zig");
const RegisterResourceOptions = resources.RegisterOptions;
const ResourceId = resources.ResourceId;
const systems = @import("systems.zig");
const SystemId = systems.SystemId;
const SystemGroup = systems.SystemGroup;
const SystemContext = systems.SystemContext;
const AllocatorStrategy = systems.AllocatorStrategy;
const System = systems.System;
const worlds = @import("worlds.zig");
const CreateWorldOptions = worlds.CreateOptions;
const World = worlds.World;

/// Namespace for all symbols of the package.
pub const symbol_namespace: [:0]const u8 = "fimo-worlds";

/// Tuple containing all symbols of the package.
pub const all_symbols = .{
    resource_register,
    resource_unregister,

    system_register,
    system_unregister,

    system_group_create,
    system_group_destroy,
    system_group_get_world,
    system_group_get_label,
    system_group_get_pool,
    system_group_add_systems,
    system_group_remove_system,
    system_group_schedule,
    system_context_get_group,
    system_context_get_generation,
    system_context_allocator_alloc,
    system_context_allocator_resize,
    system_context_allocator_remap,
    system_context_allocator_free,

    world_create,
    world_destroy,
    world_get_label,
    world_get_pool,
    world_has_resource,
    world_add_resource,
    world_remove_resource,
    world_lock_resources,
    world_unlock_resource_exclusive,
    world_unlock_resource_shared,
    world_allocator_alloc,
    world_allocator_resize,
    world_allocator_remap,
    world_allocator_free,
};

pub const resource_register = Symbol{
    .name = "resource_register",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        resource: *const RegisterResourceOptions.Descriptor,
        id: *ResourceId,
    ) callconv(.c) Status,
};

pub const resource_unregister = Symbol{
    .name = "resource_unregister",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (id: ResourceId) callconv(.c) void,
};

pub const system_register = Symbol{
    .name = "system_register",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        system: *const System.Descriptor,
        id: *SystemId,
    ) callconv(.c) Status,
};

pub const system_unregister = Symbol{
    .name = "system_unregister",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (id: SystemId) callconv(.c) void,
};

pub const system_group_create = Symbol{
    .name = "system_group_create",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        descriptor: *const SystemGroup.CreateOptions.Descriptor,
        group: **SystemGroup,
    ) callconv(.c) Status,
};

pub const system_group_destroy = Symbol{
    .name = "system_group_destroy",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (group: *SystemGroup) callconv(.c) void,
};

pub const system_group_get_world = Symbol{
    .name = "system_group_get_world",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (group: *SystemGroup) callconv(.c) *World,
};

pub const system_group_get_label = Symbol{
    .name = "system_group_get_label",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (group: *SystemGroup, len: *usize) callconv(.c) ?[*]const u8,
};

pub const system_group_get_pool = Symbol{
    .name = "system_group_get_pool",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (group: *SystemGroup) callconv(.c) Pool,
};

pub const system_group_add_systems = Symbol{
    .name = "system_group_add_systems",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (group: *SystemGroup, systems: ?[*]const SystemId, len: usize) callconv(.c) Status,
};

pub const system_group_remove_system = Symbol{
    .name = "system_group_remove_system",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (group: *SystemGroup, id: SystemId, signal: *Fence) callconv(.c) void,
};

pub const system_group_schedule = Symbol{
    .name = "system_group_schedule",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        group: *SystemGroup,
        wait_on: ?[*]const *Fence,
        wait_on_len: usize,
        signal: ?*Fence,
    ) callconv(.c) Status,
};

pub const system_context_get_group = Symbol{
    .name = "system_context_get_group",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (context: *SystemContext) callconv(.c) *SystemGroup,
};

pub const system_context_get_generation = Symbol{
    .name = "system_context_get_generation",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (context: *SystemContext) callconv(.c) usize,
};

pub const system_context_allocator_alloc = Symbol{
    .name = "system_context_allocator_alloc",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        context: *SystemContext,
        strategy: AllocatorStrategy,
        len: usize,
        alignment: usize,
        ret_addr: usize,
    ) callconv(.c) ?[*]u8,
};

pub const system_context_allocator_resize = Symbol{
    .name = "system_context_allocator_resize",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        context: *SystemContext,
        strategy: AllocatorStrategy,
        ptr: ?[*]u8,
        len: usize,
        alignment: usize,
        new_len: usize,
        ret_addr: usize,
    ) callconv(.c) bool,
};

pub const system_context_allocator_remap = Symbol{
    .name = "system_context_allocator_remap",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        context: *SystemContext,
        strategy: AllocatorStrategy,
        ptr: ?[*]u8,
        len: usize,
        alignment: usize,
        new_len: usize,
        ret_addr: usize,
    ) callconv(.c) ?[*]u8,
};

pub const system_context_allocator_free = Symbol{
    .name = "system_context_allocator_free",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        context: *SystemContext,
        strategy: AllocatorStrategy,
        ptr: ?[*]u8,
        len: usize,
        alignment: usize,
        ret_addr: usize,
    ) callconv(.c) void,
};

pub const world_create = Symbol{
    .name = "world_create",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (descriptor: *const CreateWorldOptions.Descriptor, world: **World) callconv(.c) Status,
};

pub const world_destroy = Symbol{
    .name = "world_destroy",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World) callconv(.c) void,
};

pub const world_get_label = Symbol{
    .name = "world_get_label",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, len: *usize) callconv(.c) ?[*]const u8,
};

pub const world_get_pool = Symbol{
    .name = "world_get_pool",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World) callconv(.c) Pool,
};

pub const world_has_resource = Symbol{
    .name = "world_has_resource",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, id: ResourceId) callconv(.c) bool,
};

pub const world_add_resource = Symbol{
    .name = "world_add_resource",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, id: ResourceId, value: *const anyopaque) callconv(.c) Status,
};

pub const world_remove_resource = Symbol{
    .name = "world_remove_resource",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, id: ResourceId, value: *anyopaque) callconv(.c) Status,
};

pub const world_lock_resources = Symbol{
    .name = "world_lock_resources",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        world: *World,
        exclusive_ids: [*]const ResourceId,
        exclusive_ids_len: usize,
        shared_ids: [*]const ResourceId,
        shared_ids_len: usize,
        out_resources: [*]*anyopaque,
    ) callconv(.c) void,
};

pub const world_unlock_resource_exclusive = Symbol{
    .name = "world_unlock_resource_exclusive",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, id: ResourceId) callconv(.c) void,
};

pub const world_unlock_resource_shared = Symbol{
    .name = "world_unlock_resource_shared",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (world: *World, id: ResourceId) callconv(.c) void,
};

pub const world_allocator_alloc = Symbol{
    .name = "world_allocator_alloc",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        world: *World,
        len: usize,
        alignment: usize,
        ret_addr: usize,
    ) callconv(.c) ?[*]u8,
};

pub const world_allocator_resize = Symbol{
    .name = "world_allocator_resize",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        world: *World,
        ptr: ?[*]u8,
        len: usize,
        alignment: usize,
        new_len: usize,
        ret_addr: usize,
    ) callconv(.c) bool,
};

pub const world_allocator_remap = Symbol{
    .name = "world_allocator_remap",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        world: *World,
        ptr: ?[*]u8,
        len: usize,
        alignment: usize,
        new_len: usize,
        ret_addr: usize,
    ) callconv(.c) ?[*]u8,
};

pub const world_allocator_free = Symbol{
    .name = "world_allocator_free",
    .namespace = symbol_namespace,
    .version = context_version,
    .T = fn (
        world: *World,
        ptr: ?[*]u8,
        len: usize,
        alignment: usize,
        ret_addr: usize,
    ) callconv(.c) void,
};
