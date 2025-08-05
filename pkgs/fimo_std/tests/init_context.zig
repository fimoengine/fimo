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

    const tracing_cfg = tracing.Config{
        .max_level = .trace,
        .subscribers = &.{logger.subscriber()},
        .subscriber_count = 1,
    };
    const init_options: [:null]const ?*const ctx.ConfigHead = &.{@ptrCast(&tracing_cfg)};

    try ctx.init(init_options);
    defer ctx.deinit();
}
