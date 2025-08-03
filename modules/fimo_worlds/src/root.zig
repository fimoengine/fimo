const std = @import("std");

pub const fimo_std = @import("fimo_std");
pub const fimo_worlds_meta = @import("fimo_worlds_meta");

pub const heap = @import("heap.zig");
pub const SystemContext = @import("SystemContext.zig");
pub const SystemGroup = @import("SystemGroup.zig");
pub const Universe = @import("Universe.zig");
pub const World = @import("World.zig");

pub const fimo_module_bundle = fimo_std.modules.ModuleBundle(.{
    @import("FimoWorlds.zig").Module,
});

test {
    std.testing.refAllDeclsRecursive(heap);
    std.testing.refAllDeclsRecursive(SystemContext);
    std.testing.refAllDeclsRecursive(SystemGroup);
    std.testing.refAllDeclsRecursive(Universe);
    std.testing.refAllDeclsRecursive(World);
}

comptime {
    _ = fimo_module_bundle;
}
