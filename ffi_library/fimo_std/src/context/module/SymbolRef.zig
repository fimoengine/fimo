const std = @import("std");
const Allocator = std.mem.Allocator;

const Version = @import("../../Version.zig");

const Self = @This();

owner: []const u8,
version: Version,

pub const Id = struct {
    name: []const u8,
    namespace: []const u8,

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
};
