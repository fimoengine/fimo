const std = @import("std");

pub const c = @import("c");

pub const command_buffer = @import("command_buffer.zig");
pub const future = @import("future.zig");
pub const pool = @import("pool.zig");
pub const symbols = @import("symbols.zig");
pub const sync = @import("sync.zig");
pub const task = @import("task.zig");
pub const task_local = @import("task_local.zig");

test {
    std.testing.refAllDeclsRecursive(command_buffer);
    std.testing.refAllDeclsRecursive(future);
    std.testing.refAllDeclsRecursive(pool);
    std.testing.refAllDeclsRecursive(symbols);
    std.testing.refAllDeclsRecursive(sync);
    std.testing.refAllDeclsRecursive(task);
    std.testing.refAllDeclsRecursive(task_local);
}
