const std = @import("std");

pub const c = @import("c.zig");
pub const module = @import("module.zig");

comptime {
    _ = c;
    _ = module;
}
