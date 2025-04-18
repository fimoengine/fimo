const fimo_std = @import("fimo_std");
const Context = fimo_std.Context;
const Tracing = Context.Tracing;

pub fn main() !void {
    const tracing_cfg = Tracing.Config{
        .max_level = .trace,
        .subscribers = &.{Tracing.default_subscriber},
        .subscriber_count = 1,
    };
    defer tracing_cfg.deinit();
    const init_options: [:null]const ?*const Context.TaggedInStruct = &.{@ptrCast(&tracing_cfg)};

    const ctx = try Context.init(init_options);
    defer ctx.unref();

    ctx.ref();
    ctx.unref();
}
