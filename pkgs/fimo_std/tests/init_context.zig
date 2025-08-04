const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const tracing = fimo_std.tracing;

pub fn main() !void {
    const tracing_cfg = tracing.Config{
        .max_level = .trace,
        .subscribers = &.{tracing.default_subscriber},
        .subscriber_count = 1,
    };
    const init_options: [:null]const ?*const ctx.ConfigHead = &.{@ptrCast(&tracing_cfg)};

    try ctx.init(init_options);
    defer ctx.deinit();
}
