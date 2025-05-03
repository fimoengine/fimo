const std = @import("std");

const symbols = @import("symbols.zig");
const worlds = @import("worlds.zig");
const World = worlds.World;

/// Options for registering a new resource.
pub const RegisterOptions = struct {
    label: ?[]const u8 = null,

    /// Description of a resource.
    pub const Descriptor = extern struct {
        next: ?*const anyopaque,
        label: ?[*]const u8,
        label_len: usize,
        size: usize,
        alignment: usize,
    };
};

/// A unique identifier of a registered resource.
pub fn TypedResourceId(T: type) type {
    return enum(usize) {
        _,

        const Self = @This();
        pub const Resource = T;

        /// Registers a new resource to the universe.
        ///
        /// Registered resources may be instantiated by any world that knows its id.
        pub fn register(provider: anytype, options: RegisterOptions) error{RegisterFailed}!Self {
            const desc = RegisterOptions.Descriptor{
                .next = null,
                .label = if (options.label) |l| l.ptr else null,
                .label_len = if (options.label) |l| l.len else 0,
                .size = @sizeOf(T),
                .alignment = @alignOf(T),
            };

            var id: ResourceId = undefined;
            const sym = symbols.resource_register.requestFrom(provider);
            return switch (sym(&desc, &id)) {
                .Ok => fromId(id),
                .OperationFailed => error.RegisterFailed,
                else => unreachable,
            };
        }

        /// Unregister the resource from the universe.
        ///
        /// Once unregistered, the identifier is invalidated and may be reused by another resouce.
        /// The resource must not be used by any world when this method is called.
        pub fn unregister(self: @This(), provider: anytype) void {
            const sym = symbols.resource_unregister.requestFrom(provider);
            return sym(self.asId());
        }

        /// Casts the typed identifier to an untyped one.
        pub fn asId(self: Self) ResourceId {
            return @enumFromInt(@intFromEnum(self));
        }

        /// Casts the untyped identifier to a typed one.
        pub fn fromId(id: ResourceId) Self {
            return @enumFromInt(@intFromEnum(id));
        }

        /// Checks if the resource is instantiated in the world.
        pub fn existsInWorld(self: Self, provider: anytype, world: *World) bool {
            return world.hasResource(provider, self.asId());
        }

        /// Adds the resource to the world.
        pub fn addToWorld(
            self: Self,
            provider: anytype,
            world: *World,
            value: *const T,
        ) error{AddFailed}!void {
            return world.addResource(provider, self.asId(), value);
        }

        /// Removes the resource from the world.
        pub fn removeFromWorld(
            self: Self,
            provider: anytype,
            world: *World,
        ) error{RemoveFailed}!void {
            return world.removeResource(provider, self.asId());
        }

        /// Returns an exclusive reference to the resource in the world.
        ///
        /// The caller will block until there are no active shared or exlusive references to the
        /// resource.
        pub fn lockInWorldExclusive(self: Self, provider: anytype, world: *World) *T {
            var out: *anyopaque = undefined;
            world.lockResourcesRaw(provider, &.{self.asId()}, &.{}, (&out)[0..1]);
            return @ptrCast(@alignCast(out));
        }

        /// Returns a shared reference to the resource in the world.
        ///
        /// The caller will block until there are no active exlusive references to the resource.
        pub fn lockInWorldShared(self: Self, provider: anytype, world: *World) *T {
            var out: *anyopaque = undefined;
            world.lockResourcesRaw(provider, &.{}, &.{self.asId()}, (&out)[0..1]);
            return @ptrCast(@alignCast(out));
        }

        /// Unlocks an exclusive resource lock in the world.
        pub fn unlockInWorldExclusive(self: Self, provider: anytype, world: *World) void {
            world.unlockResourceExclusive(provider, self.asId());
        }

        /// Unlocks a shared resource lock in the world.
        pub fn unlockInWorldShared(self: Self, provider: anytype, world: *World) void {
            world.unlockResourceShared(provider, self.asId());
        }
    };
}

/// A resource id with an unknown resource type.
pub const ResourceId = TypedResourceId(anyopaque);
