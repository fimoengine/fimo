const std = @import("std");

pub const exports = @import("module/exports.zig");

// Force the inclusion of the required symbols.
comptime {
    _ = exports;
}
