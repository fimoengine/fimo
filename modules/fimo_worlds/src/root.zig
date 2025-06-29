const std = @import("std");

pub const fimo_worlds_meta = @import("fimo_worlds_meta");

pub const fimo_export = @import("fimo_export.zig");
pub const heap = @import("heap.zig");
pub const SystemContext = @import("SystemContext.zig");
pub const SystemGroup = @import("SystemGroup.zig");
pub const Universe = @import("Universe.zig");
pub const World = @import("World.zig");

test {
    std.testing.refAllDeclsRecursive(fimo_export);
    std.testing.refAllDeclsRecursive(heap);
    std.testing.refAllDeclsRecursive(SystemContext);
    std.testing.refAllDeclsRecursive(SystemGroup);
    std.testing.refAllDeclsRecursive(Universe);
    std.testing.refAllDeclsRecursive(World);
}

comptime {
    _ = fimo_export;
}
