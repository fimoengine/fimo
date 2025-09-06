const std = @import("std");

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const tracing = fimo_std.tracing;

pub fn main() !void {
    var gpa = std.heap.DebugAllocator(.{}).init;
    defer _ = gpa.deinit();

    var logger: tracing.StdErrLogger = undefined;
    try logger.init(.{ .gpa = gpa.allocator() });
    defer logger.deinit();

    const tracing_cfg = tracing.Cfg{
        .max_level = .trace,
        .subscribers = .fromSlice(&.{logger.subscriber()}),
    };

    try ctx.init(&.{&tracing_cfg.cfg});
    defer ctx.deinit();
}
