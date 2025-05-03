const std = @import("std");

pub const c = @import("c");

pub const Job = @import("Job.zig");
pub const resources = @import("resources.zig");
pub const symbols = @import("symbols.zig");
pub const systems = @import("systems.zig");
pub const worlds = @import("worlds.zig");

test {
    std.testing.refAllDeclsRecursive(Job);
    std.testing.refAllDeclsRecursive(resources);
    std.testing.refAllDeclsRecursive(symbols);
    std.testing.refAllDeclsRecursive(systems);
    std.testing.refAllDeclsRecursive(worlds);
}
