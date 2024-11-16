const std = @import("std");
const Allocator = std.mem.Allocator;

const heap = @import("../../heap.zig");
const Version = @import("../../version.zig");

const allocator = heap.fimo_allocator;
const Self = @This();

owner: []u8,
version: Version,

pub const Id = struct {
    name: []u8,
    namespace: []u8,

    pub const HashContext = struct {
        pub fn eql(self: HashContext, a: Id, b: Id, b_index: usize) bool {
            _ = self;
            _ = b_index;
            return std.mem.eql(u8, a.name, b.name) and
                std.mem.eql(u8, a.namespace, b.namespace);
        }
        pub fn hash(self: HashContext, key: Id) u32 {
            _ = self;
            var hasher = std.hash.Wyhash.init(0);
            std.hash.autoHashStrat(&hasher, key, .Deep);
            return @truncate(hasher.final());
        }
    };

    pub fn init(name: []const u8, namespace: []const u8) Allocator.Error!Id {
        const n = try allocator.dupe(u8, name);
        errdefer allocator.free(n);
        const ns = try allocator.dupe(u8, namespace);
        return Id{ .name = n, .namespace = ns };
    }

    pub fn deinit(self: Id) void {
        allocator.free(self.name);
        allocator.free(self.namespace);
    }
};

pub fn init(owner: []const u8, version: Version) Allocator.Error!Self {
    const o = try allocator.dupe(u8, owner);
    return Self{ .owner = o, .version = version };
}

pub fn deinit(self: Self) void {
    allocator.free(self.owner);
}
