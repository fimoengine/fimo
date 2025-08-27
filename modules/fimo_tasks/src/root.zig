const std = @import("std");

pub const fimo_std = @import("fimo_std");
pub const fimo_tasks_meta = @import("fimo_tasks_meta");

const context = @import("context.zig");

// const channel = @import("channel.zig");
// const CmdBuf = @import("CommandBuffer.zig");
// const Futex = @import("Futex.zig");
// const Pool = @import("Pool.zig");
// const PoolMap = @import("PoolMap.zig");
// const Task = @import("Task.zig");
// const Worker = @import("Worker.zig");

pub const fimo_module_bundle = fimo_std.modules.ModuleBundle(.{
    @import("FimoTasks.zig").Module,
});

test {
    // std.testing.refAllDeclsRecursive(channel);
    // std.testing.refAllDeclsRecursive(CmdBuf);
    std.testing.refAllDeclsRecursive(context);
    // std.testing.refAllDeclsRecursive(Futex);
    // std.testing.refAllDeclsRecursive(Pool);
    // std.testing.refAllDeclsRecursive(PoolMap);
    // std.testing.refAllDeclsRecursive(Task);
    // std.testing.refAllDeclsRecursive(Worker);
}

// Ensure that the module is exported.
comptime {
    _ = fimo_module_bundle;
}
