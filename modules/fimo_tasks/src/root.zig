const std = @import("std");

pub const fimo_tasks_meta = @import("fimo_tasks_meta");

const channel = @import("channel.zig");
const CommandBuffer = @import("CommandBuffer.zig");
const context = @import("context.zig");
pub const fimo_export = @import("fimo_export.zig");
const ParkingLot = @import("ParkingLot.zig");
const Pool = @import("Pool.zig");
const PoolMap = @import("PoolMap.zig");
const Runtime = @import("Runtime.zig");
const Task = @import("Task.zig");
const Worker = @import("Worker.zig");

test {
    std.testing.refAllDeclsRecursive(channel);
    std.testing.refAllDeclsRecursive(CommandBuffer);
    std.testing.refAllDeclsRecursive(context);
    std.testing.refAllDeclsRecursive(fimo_export);
    std.testing.refAllDeclsRecursive(ParkingLot);
    std.testing.refAllDeclsRecursive(Pool);
    std.testing.refAllDeclsRecursive(PoolMap);
    std.testing.refAllDeclsRecursive(Runtime);
    std.testing.refAllDeclsRecursive(Task);
    std.testing.refAllDeclsRecursive(Worker);
}

// Ensure that the module is exported.
comptime {
    _ = fimo_export;
}
