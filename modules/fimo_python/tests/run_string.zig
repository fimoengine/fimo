const std = @import("std");

const fimo_std = @import("fimo_std");
const fimo_python_meta = @import("fimo_python_meta");

const Context = fimo_std.Context;
const Async = Context.Async;
const Tracing = Context.Tracing;
const Module = Context.Module;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const tracing_cfg = Tracing.Config{
        .max_level = .trace,
        .subscribers = &.{Tracing.default_subscriber},
        .subscriber_count = 1,
    };
    defer tracing_cfg.deinit();
    const init_options: [:null]const ?*const Context.TaggedInStruct = &.{@ptrCast(&tracing_cfg)};

    const ctx = try Context.init(init_options);
    defer ctx.unref();

    ctx.tracing().registerThread();
    defer ctx.tracing().unregisterThread();

    var err: ?fimo_std.AnyError = null;
    defer if (err) |e| e.deinit();

    defer Async.EventLoop.flushWithCurrentThread(ctx.@"async"(), &err) catch unreachable;
    const event_loop = try Async.EventLoop.init(ctx.@"async"(), &err);
    defer event_loop.join();

    const async_ctx = try Async.BlockingContext.init(ctx.@"async"(), &err);
    defer async_ctx.deinit();

    var module_path = fimo_std.path.PathBuffer.init(allocator);
    defer module_path.deinit();
    try module_path.pushString("fimo_python");
    try module_path.pushString("module.fimo_module");

    const set = try Module.LoadingSet.init(ctx.module(), &err);
    defer set.unref();

    try set.addModulesFromPath(
        module_path.asPath(),
        &{},
        struct {
            fn f(@"export": *const Module.Export, data: *const void) Module.LoadingSet.FilterOp {
                _ = @"export";
                _ = data;
                return .load;
            }
        }.f,
        null,
        &err,
    );
    try set.commit().intoFuture().awaitBlocking(async_ctx).unwrap(&err);

    const instance = try Module.PseudoInstance.init(ctx.module(), &err);
    defer instance.deinit();

    const info = try Module.Info.findByName(ctx.module(), "fimo_python", &err);
    defer info.unref();

    try instance.addDependency(info, &err);
    try instance.addNamespace(fimo_python_meta.symbols.RunString.namespace, &err);
    const run_string = try instance.loadSymbol(fimo_python_meta.symbols.RunString, &err);

    try run_string.call(
        \\import sys
        \\
        \\print("Hello Python!", file=sys.stderr)
    , null, &err);
}
