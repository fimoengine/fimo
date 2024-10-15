//! Internal inteface of the fimo std library.
const c = @import("c.zig");

const Tracing = @import("context/tracing.zig");
pub const ProxyContext = @import("context/proxy_context.zig");

pub const Context = extern struct {
    refcount: extern struct {
        strong: usize,
        weak: usize,
    },
    tracing: *Tracing,
};

comptime {
    _ = Context;
}
