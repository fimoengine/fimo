const std = @import("std");
const Allocator = std.mem.Allocator;
const Alignment = std.mem.Alignment;

const fimo_tasks_meta = @import("fimo_tasks_meta");
const Pool = fimo_tasks_meta.pool.Pool;

const Job = @import("Job.zig");
const Fence = Job.Fence;
const resources = @import("resources.zig");
const TypedResourceId = resources.TypedResourceId;
const ResourceId = resources.ResourceId;
const symbols = @import("symbols.zig");
const worlds = @import("worlds.zig");
const World = worlds.World;

/// A unique identifier for a registered system.
pub const SystemId = enum(usize) {
    _,

    /// Unregisters the system from the universe.
    ///
    /// Once unregistered, the identifier is invalidated and may be reused by another system.
    /// The system must not be used explicitly by any world when this method is called.
    pub fn unregister(self: SystemId, provider: anytype) void {
        const sym = symbols.system_unregister.requestFrom(provider);
        return sym(self);
    }
};

/// A group of systems that can be scheduled together.
pub const SystemGroup = opaque {
    /// Options for creating a new system group.
    pub const CreateOptions = struct {
        /// Optional label of the group.
        label: ?[]const u8 = null,
        /// Executor for the system group.
        ///
        /// A null value will inherit the executor of the world.
        /// If the value is not null, the system group will increase its reference count.
        pool: ?Pool = null,
        /// World to add the group to.
        world: *World,

        /// Descriptor a a new system group.
        pub const Descriptor = extern struct {
            /// Reserved. Must be null.
            next: ?*const anyopaque,
            /// Optional label of the system group.
            label: ?[*]const u8,
            /// Length in characters of the system group label.
            label_len: usize,
            /// Optional executor for the system group.
            ///
            /// A null value will inherit the executor of the world.
            /// If the value is not null, the system group will increase its reference count.
            pool: ?*const Pool,
            /// World to add the group to.
            world: *World,
        };
    };

    /// Initializes a new empty system group.
    pub fn init(provider: anytype, options: CreateOptions) error{InitFailed}!*SystemGroup {
        const desc = CreateOptions.Descriptor{
            .next = null,
            .label = if (options.label) |l| l.ptr else null,
            .label_len = if (options.label) |l| l.len else 0,
            .pool = if (options.pool) |*p| p else null,
            .world = options.world,
        };

        var group: *SystemGroup = undefined;
        const sym = symbols.system_group_create.requestFrom(provider);
        switch (sym(&desc, &group)) {
            .Ok => {},
            .OperationFailed => error.InitFailed,
            else => unreachable,
        }
    }

    /// Destroys the system group.
    ///
    /// The caller may provide a reference to a fence via `signal`, to be notified when the group
    /// has been destroyed. If no fence is provided, the caller will block until the group is
    /// destroyed. Scheduled operations will be executed.
    pub fn deinit(self: *SystemGroup, provider: anytype, signal: ?*Fence) void {
        const sym = symbols.system_group_destroy.requestFrom(provider);
        return sym(self, signal);
    }

    /// Returns the world the group is contained in.
    pub fn getWorld(self: *SystemGroup, provider: anytype) *World {
        const sym = symbols.system_group_get_world.requestFrom(provider);
        return sym(self);
    }

    /// Returns the label of the system group.
    pub fn getLabel(self: *SystemGroup, provider: anytype) ?[]const u8 {
        var len: usize = undefined;
        const sym = symbols.system_group_get_world.requestFrom(provider);
        if (sym(self, &len)) |label| return label[0..len];
        return null;
    }

    /// Returns a reference to the executor used by the group.
    pub fn getPool(self: *SystemGroup, provider: anytype) Pool {
        const sym = symbols.system_group_get_pool.requestFrom(provider);
        return sym(self);
    }

    /// Adds a set of systems to the group.
    ///
    /// Already scheduled operations are not affected by the added systems.
    /// The operation may add systems transitively, if the systems specify an execution order.
    pub fn addSytems(
        self: *SystemGroup,
        provider: anytype,
        systems: []const SystemId,
    ) error{AddFailed}!void {
        const sym = symbols.system_group_add_systems.requestFrom(provider);
        switch (sym(self, systems.ptr, systems.len)) {
            .Ok => {},
            .OperationFailed => error.AddFailed,
            else => unreachable,
        }
    }

    /// Removes a system from the group.
    ///
    /// Already scheduled systems will not be affected. This operation may remove systems added
    /// transitively. The caller may provide a reference to a fence via `signal`, to be notified
    /// when the system has been removed from the group.
    pub fn removeSystem(self: *SystemGroup, provider: anytype, id: SystemId, signal: ?*Fence) void {
        const sym = symbols.system_group_remove_system.requestFrom(provider);
        return sym(self, id, signal);
    }

    /// Schedules to run all systems contained in the group.
    ///
    /// The group will start executing after all fences in `wait_on` are signaled.
    /// The caller may provide a reference to a fence via `signal`, to be notified when the group
    /// has finished executing all systems.
    ///
    /// Each schedule operation is assigned to one generation of the system group, which is an index
    /// that is increased by one each time the group finishes executing all systems. Multiple generations
    /// are run sequentially.
    pub fn schedule(
        self: *SystemGroup,
        provider: anytype,
        wait_on: []const *Fence,
        signal: ?*Fence,
    ) error{ScheduleFailed}!void {
        const sym = symbols.system_group_add_systems.requestFrom(provider);
        switch (sym(self, wait_on.ptr, wait_on.len, signal)) {
            .Ok => {},
            .OperationFailed => error.ScheduleFailed,
            else => unreachable,
        }
    }

    /// Convenience function to schedule and wait until the completion of the group.
    ///
    /// The group will start executing after all fences in `wait_on` are signaled.
    pub fn run(
        self: *SystemGroup,
        provider: anytype,
        wait_on: []const *Fence,
    ) error{ScheduleFailed}!void {
        var fence = Fence{};
        try self.schedule(provider, wait_on, &fence);
        fence.wait(provider);
    }
};

/// Context of an instantiated system in a system group.
pub const SystemContext = opaque {
    /// Returns the group the system is contained in.
    pub fn getGroup(self: *SystemContext, provider: anytype) *SystemGroup {
        const sym = symbols.system_context_get_group.requestFrom(provider);
        return sym(self);
    }

    /// Returns the current generation of system group.
    ///
    /// The generation is increased by one each time the group finishes executing all systems.
    pub fn getGeneration(self: *SystemGroup, provider: anytype) usize {
        const sym = symbols.system_context_get_generation.requestFrom(provider);
        return sym(self);
    }

    /// Constructs an allocator using some specific (de)allocation strategy.
    ///
    /// Consult the documentation of the individual strategies for additional info.
    pub fn getAllocator(
        self: *SystemContext,
        provider: anytype,
        comptime strategy: AllocatorStrategy,
    ) SystemAllocator(@TypeOf(provider), strategy) {
        return .{ .context = self, .provider = provider };
    }

    /// An allocator that is invalidated after the system has finished executing.
    ///
    /// The memory returned by this allocator is only valid in the scope of the run function of the
    /// system for the current group generation. The allocator is not thread-safe.
    pub fn getTransientAllocator(
        self: *SystemContext,
        provider: anytype,
    ) SystemAllocator(@TypeOf(provider), .transient) {
        return self.getAllocator(provider, .transient);
    }

    /// An allocator that is invalidated at the end of the current system group generation.
    ///
    /// The allocator may be utilized to spawn short lived tasks from the system, or to pass
    /// data to systems executing after the current one.
    pub fn getSingleGenerationAllocator(
        self: *SystemContext,
        provider: anytype,
    ) SystemAllocator(@TypeOf(provider), .single_generation) {
        return self.getAllocator(provider, .single_generation);
    }

    /// An allocator that is invalidated after four generations.
    ///
    /// The allocator may be utilized to spawn medium-to-short lived tasks from the system, or
    /// to pass data to the systems executing in the next generations.
    pub fn getMultiGenerationAllocator(
        self: *SystemContext,
        provider: anytype,
    ) SystemAllocator(@TypeOf(provider), .multi_generation) {
        return self.getAllocator(provider, .multi_generation);
    }

    /// An allocator that is invalidated with the system.
    ///
    /// May be utilized for long-lived/persistent allocations.
    pub fn getSystemPersistentAllocator(
        self: *SystemContext,
        provider: anytype,
    ) SystemAllocator(@TypeOf(provider), .system_persistent) {
        return self.getAllocator(provider, .system_persistent);
    }
};

/// Known allocator strategies for a system.
pub const AllocatorStrategy = enum(i32) {
    /// An allocator that is invalidated after the system has finished executing.
    ///
    /// The memory returned by this allocator is only valid in the scope of the run function of the
    /// system for the current group generation. The allocator is not thread-safe.
    transient,
    /// An allocator that is invalidated at the end of the current system group generation.
    ///
    /// The allocator may be utilized to spawn short lived tasks from the system, or to pass
    /// data to systems executing after the current one.
    single_generation,
    /// An allocator that is invalidated after four generations.
    ///
    /// The allocator may be utilized to spawn medium-to-short lived tasks from the system, or
    /// to pass data to the systems executing in the next generations.
    multi_generation,
    /// An allocator that is invalidated with the system.
    ///
    /// May be utilized for long-lived/persistent allocations.
    system_persistent,
    _,
};

/// A strategy dependent allocator for a system.
pub fn SystemAllocator(comptime Provider: type, comptime strategy: AllocatorStrategy) type {
    return struct {
        context: *SystemContext,
        provider: Provider,

        const Self = @This();
        const vtable = Allocator.VTable{
            .alloc = &alloc,
            .resize = &resize,
            .remap = &remap,
            .free = &free,
        };

        fn alloc(this: *anyopaque, len: usize, alignment: Alignment, ret_addr: usize) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.system_context_allocator_alloc.requestFrom(self.provider);
            return sym(self.context, strategy, len, alignment.toByteUnits(), ret_addr);
        }

        fn resize(
            this: *anyopaque,
            memory: []u8,
            alignment: Alignment,
            new_len: usize,
            ret_addr: usize,
        ) bool {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.system_context_allocator_resize.requestFrom(self.provider);
            return sym(
                self.context,
                strategy,
                memory.ptr,
                memory.len,
                alignment.toByteUnits(),
                new_len,
                ret_addr,
            );
        }

        fn remap(
            this: *anyopaque,
            memory: []u8,
            alignment: Alignment,
            new_len: usize,
            ret_addr: usize,
        ) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.system_context_allocator_remap.requestFrom(self.provider);
            return sym(
                self.context,
                strategy,
                memory.ptr,
                memory.len,
                alignment.toByteUnits(),
                new_len,
                ret_addr,
            );
        }

        fn free(this: *anyopaque, memory: []u8, alignment: Alignment, ret_addr: usize) ?[*]u8 {
            const self: *Self = @ptrCast(@alignCast(this));
            const sym = symbols.system_context_allocator_free.requestFrom(self.provider);
            return sym(
                self.context,
                strategy,
                memory.ptr,
                memory.len,
                alignment.toByteUnits(),
                ret_addr,
            );
        }

        pub fn allocator(self: *Self) Allocator {
            return .{ .ptr = self, .vtable = &vtable };
        }
    };
}

/// Interface of a system.
pub const System = struct {
    FactoryT: type,
    deinit_factory: ?*const fn (factory: ?*anyopaque) callconv(.c) void,

    SystemT: type,
    ExclusiveResourceIdsT: type,
    SharedResourceIdsT: type,
    init: *const fn (
        factory: ?*const anyopaque,
        world: *World,
        group: *SystemGroup,
        system: ?*anyopaque,
    ) callconv(.c) bool,
    deinit: ?*const fn (system: ?*anyopaque) callconv(.c) void,
    run: *const fn (
        system: ?*anyopaque,
        unique_resources: ?[*]const *anyopaque,
        shared_resources: ?[*]const *anyopaque,
    ) callconv(.c) ?*Fence,

    /// Descriptor of a system dependency.
    pub const Dependency = extern struct {
        /// System to depend on / be depended from.
        system: SystemId,
        /// Whether to ignore any deferred subjob of the system.
        ///
        /// If set to `true`, the system will start after the other systems `run`
        /// function is run to completion. Otherwise, the system will start after
        /// all subjobs of the system also complete their execution.
        ignore_deferred: bool = false,
    };

    /// Descriptor of a new system.
    pub const Descriptor = extern struct {
        /// Reserved. Must be null.
        next: ?*const anyopaque,
        /// Optional label of the system.
        label: ?[*]const u8,
        /// Length in characters of the system label.
        label_len: usize,
        /// Optional array of resources to require with exclusive access.
        unique_ids: ?[*]const ResourceId,
        /// Length of the `exclusive_ids` array.
        unique_ids_len: usize,
        /// Optional array of resources to require with shared access.
        shared_ids: ?[*]const ResourceId,
        /// Length of the `shared_ids` array.
        shared_ids_len: usize,
        /// Optional array of systems to depend on.
        ///
        /// The system will start executing after all systems have been executed.
        before: ?[*]const Dependency,
        /// Length of the `before` array.
        before_len: usize,
        /// Optional array of systems to be depended from.
        ///
        /// The systems will start executing after the new system completes its execution.
        after: ?[*]const Dependency,
        /// Length of the `after` array.
        after_len: usize,

        /// Pointer to the factory for the system.
        ///
        /// The factory will be copied into the universe.
        factory: ?*const anyopaque,
        /// Size in bytes of the factory.
        factory_size: usize,
        /// Alignment in bytes of the factory. Must be a power-of-two.
        factory_alignment: usize,
        /// Optional function to call when destroying the factory.
        factory_deinit: ?*const fn (factory: ?*anyopaque) callconv(.c) void,

        /// Size in bytes of the system state.
        system_size: usize,
        /// Alignment in bytes of the system state. Must be a power-of-two.
        system_alignment: usize,
        /// Function called when instantiating a new system.
        ///
        /// The system is provided with a system context, that shares the same lifetime,
        /// as the system itself. The context provides additional utilities, like allocators.
        /// The state of the system must be written into the provided `system` pointer.
        /// On success, the function must return true.
        system_init: *const fn (
            factory: ?*const anyopaque,
            context: *SystemContext,
            system: ?*anyopaque,
        ) callconv(.c) bool,
        /// Optional function to call when destroying a system.
        system_deinit: ?*const fn (system: ?*anyopaque) callconv(.c) void,
        /// Function called on each system run.
        ///
        /// The requested exclusive and shared resources are provided in the order defined by
        /// the `exclusive_ids` and `shared_ids`. Additionally, the system is provided with a
        /// pointer to an unsignaled fence. The fence may be used to spawn additional jobs from
        /// within the run function and synchronize other systems waiting on the completion of
        /// the current system. The system must signal the fence after it has completed. Failure
        /// of doing such will lead to a deadlock. The fence is guaranteed to not have any waiters
        /// until after the run function returns.
        system_run: *const fn (
            system: ?*anyopaque,
            unique_resources: ?[*]const *anyopaque,
            shared_resources: ?[*]const *anyopaque,
            deferred_signal: *Fence,
        ) callconv(.c) void,
    };

    /// Registers a new system with the universe.
    ///
    /// Registered resources may be added to system group of any world.
    pub fn register(
        comptime self: System,
        provider: anytype,
        label: ?[]const u8,
        factory: self.FactoryT,
        exclusive_ids: self.ExclusiveResourceIdsT,
        shared_ids: self.SharedResourceIdsT,
        before: []Dependency,
        after: []Dependency,
    ) error{RegisterFailed}!SystemId {
        var unique: [exclusive_ids.len]ResourceId = undefined;
        inline for (0..unique.len) |i| unique[i] = exclusive_ids[i].asId();
        var shared: [shared_ids.len]ResourceId = undefined;
        inline for (0..shared.len) |i| shared[i] = shared_ids[i].asId();
        const desc = Descriptor{
            .next = null,
            .label = if (label) |l| l.ptr else null,
            .label_len = if (label) |l| l.len else 0,
            .unique_ids = unique[0..].ptr,
            .unique_ids_len = unique.len,
            .shared_ids = shared[0..].ptr,
            .shared_ids_len = shared.len,
            .before = before.ptr,
            .before_len = before.len,
            .after = after.ptr,
            .after_len = after.len,

            .factory = &factory,
            .factory_size = @sizeOf(self.FactoryT),
            .factory_alignment = @alignOf(self.FactoryT),
            .factory_deinit = self.deinit_factory,

            .system_size = @sizeOf(self.SystemT),
            .system_alignment = @alignOf(self.SystemT),
            .system_init = self.init,
            .system_deinit = self.deinit,
            .system_run = self.run,
        };

        var id: SystemId = undefined;
        const sym = symbols.system_register.requestFrom(provider);
        return switch (sym(&desc, &id)) {
            .Ok => id,
            .OperationFailed => error.RegisterFailed,
            else => unreachable,
        };
    }

    /// A simple system using only a stateless function.
    pub fn simple(comptime runFn: anytype) System {
        const run_info = @typeInfo(@TypeOf(runFn)).@"fn";
        if (run_info.params.len != 3)
            @compileError("The system run function must take three parameters.");
        const ExclusiveArgs = run_info.params[1];
        const SharedArgs = run_info.params[2];
        const has_signal = run_info.return_type.? != void;

        const Wrapper = struct {
            context: *SystemContext,

            fn init(factory: *const void, context: *SystemContext) !@This() {
                _ = factory;
                return .{ .context = context };
            }

            fn run(self: *@This(), exclusive: ExclusiveArgs, shared: SharedArgs) ?*Fence {
                if (has_signal) {
                    return runFn(self.context, exclusive, shared);
                } else {
                    runFn(self.context, exclusive, shared);
                    return null;
                }
            }
        };

        return complex(void, Wrapper, null, Wrapper.init, null, Wrapper.run);
    }

    /// Constructs a new instance from a factory.
    pub fn complex(
        FactoryT: type,
        SystemT: type,
        comptime deinitFactoryFn: ?fn (factory: *FactoryT) void,
        comptime initFn: fn (
            factory: *const FactoryT,
            context: *SystemContext,
        ) anyerror!SystemT,
        comptime deinitFn: ?fn (system: *SystemT) void,
        comptime runFn: anytype,
    ) System {
        const run_info = @typeInfo(@TypeOf(runFn)).@"fn";
        if (run_info.params.len != 3)
            @compileError("The system run function must take three parameters.");
        if (run_info.params[0].type.? != *SystemT)
            @compileError("The first argument of the run function must be a pointer to the state");

        const ExclusiveArgs = run_info.params[1];
        const SharedArgs = run_info.params[2];
        const has_signal = run_info.return_type.? != void;

        const ExclusiveResourceIdsT = if (@sizeOf(ExclusiveArgs) == 0) struct {} else blk: {
            const fields = @typeInfo(ExclusiveArgs).@"struct".fields;
            var types: [fields.len]type = undefined;
            for (0..fields.len) |i| {
                const T = fields[i].type;
                const Resource = @typeInfo(T).pointer.child;
                if (T != *Resource) @compileError("Resource parameters must be non-const pointers");
                types[i] = TypedResourceId(Resource);
            }
            break :blk std.meta.Tuple(types);
        };

        const SharedResourceIdsT = if (@sizeOf(SharedArgs) == 0) struct {} else blk: {
            const fields = @typeInfo(SharedArgs).@"struct".fields;
            var types: [fields.len]type = undefined;
            for (0..fields.len) |i| {
                const T = fields[i].type;
                const Resource = @typeInfo(T).pointer.child;
                if (T != *Resource) @compileError("Resource parameters must be non-const pointers");
                types[i] = TypedResourceId(Resource);
            }
            break :blk std.meta.Tuple(types);
        };

        const Wrapper = struct {
            fn deinitFactory(factory: ?*anyopaque) callconv(.c) void {
                const f = deinitFactoryFn.?;
                if (comptime @sizeOf(FactoryT) != 0)
                    f(@ptrCast(@alignCast(factory.?)))
                else
                    f(&FactoryT{});
            }

            fn init(
                factory: ?*const anyopaque,
                context: *SystemContext,
                system: ?*anyopaque,
            ) callconv(.c) bool {
                const fact: *const FactoryT = if (comptime @sizeOf(FactoryT) != 0)
                    @ptrCast(@alignCast(factory.?))
                else
                    &FactoryT{};
                const sys: *SystemT = if (comptime @sizeOf(SystemT) != 0)
                    @ptrCast(@alignCast(system.?))
                else
                    &SystemT{};
                sys.* = initFn(fact, context) catch return false;
                return true;
            }

            fn deinit(system: ?*anyopaque) callconv(.c) void {
                const f = deinitFn.?;
                if (comptime @sizeOf(SystemT) != 0)
                    f(@ptrCast(@alignCast(system.?)))
                else
                    f(&SystemT{});
            }

            fn run(
                system: ?*anyopaque,
                exclusive_resources: ?[*]const *anyopaque,
                shared_resources: ?[*]const *anyopaque,
            ) callconv(.c) ?*Fence {
                const sys: *SystemT = if (comptime @sizeOf(SystemT) != 0)
                    @ptrCast(@alignCast(system.?))
                else
                    &SystemT{};
                var exclusive: ExclusiveArgs = undefined;
                if (comptime @sizeOf(ExclusiveArgs) == 0) exclusive = ExclusiveArgs{} else {
                    const arr = exclusive_resources.?;
                    const fields = std.meta.fields(ExclusiveArgs);
                    inline for (0..fields.len) |i| {
                        exclusive[i] = @ptrCast(@alignCast(arr[i]));
                    }
                }
                var shared: SharedArgs = undefined;
                if (comptime @sizeOf(SharedArgs) == 0) shared = SharedArgs{} else {
                    const arr = shared_resources.?;
                    const fields = std.meta.fields(SharedArgs);
                    inline for (0..fields.len) |i| {
                        shared[i] = @ptrCast(@alignCast(arr[i]));
                    }
                }

                if (comptime has_signal) {
                    return runFn(sys, exclusive, shared);
                } else {
                    runFn(sys, exclusive, shared);
                    return null;
                }
            }
        };

        return .{
            .FactoryT = FactoryT,
            .deinit_factory = if (deinitFactoryFn != null) &Wrapper.deinitFactory else null,
            .SystemT = SystemT,
            .ExclusiveResourceIdsT = ExclusiveResourceIdsT,
            .SharedResourceIdsT = SharedResourceIdsT,
            .init = &Wrapper.init,
            .deinit = if (deinitFn != null) &Wrapper.deinit else null,
            .run = &Wrapper.run,
        };
    }
};
