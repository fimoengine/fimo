const std = @import("std");

pub const fimo_std = @import("fimo_std");
pub const fimo_tasks_meta = @import("fimo_tasks_meta");

const context = @import("context.zig");
pub const Executor = @import("Executor.zig");

pub const fimo_module_bundle = fimo_std.modules.ModuleBundle(.{
    @import("FimoTasks.zig").Module,
});

test {
    std.testing.refAllDeclsRecursive(context);
    std.testing.refAllDeclsRecursive(Executor);
}

// Ensure that the module is exported.
comptime {
    _ = fimo_module_bundle;
}
