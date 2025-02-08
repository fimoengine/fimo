const std = @import("std");
const testing = std.testing;

const fimo_std = @import("fimo_std");
const Version = fimo_std.Version;
const Path = fimo_std.path.Path;
const Context = fimo_std.Context;
const Tracing = Context.Tracing;
const Module = Context.Module;
const Async = Context.Async;

const A0 = Module.Symbol{
    .name = "a_export_0",
    .version = Version.parse("0.1.0") catch unreachable,
    .symbol = i32,
};
const A1 = Module.Symbol{
    .name = "a_export_1",
    .version = Version.parse("0.1.0") catch unreachable,
    .symbol = i32,
};
const B0 = Module.Symbol{
    .name = "b_export_0",
    .namespace = "b",
    .version = Version.parse("0.1.0") catch unreachable,
    .symbol = i32,
};
const B1 = Module.Symbol{
    .name = "b_export_1",
    .namespace = "b",
    .version = Version.parse("0.1.0") catch unreachable,
    .symbol = i32,
};

fn initCModule(octx: *const Module.OpaqueInstance, set: Module.LoadingSet) !void {
    _ = set;
    const ctx: *const C = @alignCast(@ptrCast(octx));
    ctx.context().tracing().emitInfoSimple(
        "initializing instance: name='{s}'",
        .{ctx.info.name},
        @src(),
    );

    const parameters = ctx.parameters;
    try testing.expectEqual(0, parameters.pub_pub.read());
    try testing.expectEqual(1, parameters.pub_dep.read());
    try testing.expectEqual(2, parameters.pub_pri.read());
    try testing.expectEqual(3, parameters.dep_pub.read());
    try testing.expectEqual(4, parameters.dep_dep.read());
    try testing.expectEqual(5, parameters.dep_pri.read());
    try testing.expectEqual(6, parameters.pri_pub.read());
    try testing.expectEqual(7, parameters.pri_dep.read());
    try testing.expectEqual(8, parameters.pri_pri.read());

    parameters.pub_pub.write(0);
    parameters.pub_dep.write(1);
    parameters.pub_pri.write(2);
    parameters.dep_pub.write(3);
    parameters.dep_dep.write(4);
    parameters.dep_pri.write(5);
    parameters.pri_pub.write(6);
    parameters.pri_dep.write(7);
    parameters.pri_pri.write(8);

    const resources = ctx.resources;
    ctx.context().tracing().emitTraceSimple("empty: '{s}'", .{resources.empty}, @src());
    ctx.context().tracing().emitTraceSimple("a: '{s}'", .{resources.a}, @src());
    ctx.context().tracing().emitTraceSimple("b: '{s}'", .{resources.b}, @src());
    ctx.context().tracing().emitTraceSimple("img: '{s}'", .{resources.img}, @src());

    const imports = ctx.imports;
    try testing.expectEqual(imports.a0.*, 5);
    try testing.expectEqual(imports.a1.*, 10);
    try testing.expectEqual(imports.b0.*, -2);
    try testing.expectEqual(imports.b1.*, 77);
}

fn deinitCModule(inst: *const Module.OpaqueInstance) void {
    inst.context().tracing().emitInfoSimple(
        "deinitializing instance: name='{s}'",
        .{inst.info.name},
        @src(),
    );
}

const A = Module.exports.Builder
    .init("a")
    .withDescription("Test module a")
    .withAuthor("fimo")
    .withLicense("MIT and Apache 2.0")
    .withExport(A0, "a0", .global, &5)
    .withExport(A1, "a1", .global, &10)
    .exportModule();

const B = Module.exports.Builder
    .init("b")
    .withDescription("Test module b")
    .withAuthor("fimo")
    .withLicense("MIT and Apache 2.0")
    .withExport(B0, "b0", .global, &-2)
    .withExport(B1, "b1", .global, &77)
    .exportModule();

const C = Module.exports.Builder
    .init("c")
    .withDescription("Test module c")
    .withAuthor("fimo")
    .withLicense("MIT and Apache 2.0")
    .withParameter(.{
    .name = "pub_pub",
    .member_name = "pub_pub",
    .read_group = .public,
    .write_group = .public,
    .default_value = .{ .u32 = 0 },
})
    .withParameter(.{
    .name = "pub_dep",
    .member_name = "pub_dep",
    .read_group = .public,
    .write_group = .dependency,
    .default_value = .{ .u32 = 1 },
})
    .withParameter(.{
    .name = "pub_pri",
    .member_name = "pub_pri",
    .read_group = .public,
    .default_value = .{ .u32 = 2 },
})
    .withParameter(.{
    .name = "dep_pub",
    .member_name = "dep_pub",
    .read_group = .dependency,
    .write_group = .public,
    .default_value = .{ .u32 = 3 },
})
    .withParameter(.{
    .name = "dep_dep",
    .member_name = "dep_dep",
    .read_group = .dependency,
    .write_group = .dependency,
    .default_value = .{ .u32 = 4 },
})
    .withParameter(.{
    .name = "dep_pri",
    .member_name = "dep_pri",
    .read_group = .dependency,
    .default_value = .{ .u32 = 5 },
})
    .withParameter(.{
    .name = "pri_pub",
    .member_name = "pri_pub",
    .write_group = .public,
    .default_value = .{ .u32 = 6 },
})
    .withParameter(.{
    .name = "pri_dep",
    .member_name = "pri_dep",
    .write_group = .dependency,
    .default_value = .{ .u32 = 7 },
})
    .withParameter(.{
    .name = "pri_pri",
    .member_name = "pri_pri",
    .default_value = .{ .u32 = 8 },
})
    .withResource(.{ .name = "empty", .path = Path.init("") catch unreachable })
    .withResource(.{ .name = "a", .path = Path.init("a.bin") catch unreachable })
    .withResource(.{ .name = "b", .path = Path.init("b.txt") catch unreachable })
    .withResource(.{ .name = "img", .path = Path.init("c/d.img") catch unreachable })
    .withImport(.{ .name = "a0", .symbol = A0 })
    .withImport(.{ .name = "a1", .symbol = A1 })
    .withImport(.{ .name = "b0", .symbol = B0 })
    .withImport(.{ .name = "b1", .symbol = B1 })
    .withState(void, initCModule, deinitCModule)
    .exportModule();

comptime {
    _ = A;
    _ = B;
    _ = C;
}

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

    var err: ?fimo_std.AnyError = null;
    defer if (err) |e| e.deinit();

    ctx.tracing().registerThread();
    defer ctx.tracing().unregisterThread();

    defer Async.EventLoop.flushWithCurrentThread(ctx.@"async"(), &err) catch unreachable;
    const event_loop = try Async.EventLoop.init(ctx.@"async"(), &err);
    defer event_loop.join();

    const async_ctx = try Async.BlockingContext.init(ctx.@"async"(), &err);
    defer async_ctx.deinit();

    defer ctx.module().pruneInstances(&err) catch unreachable;

    const set = try Module.LoadingSet.init(ctx.module(), &err);
    defer set.unref();

    try set.addModulesFromLocal(
        &{},
        struct {
            fn f(@"export": *const Module.Export, data: *const void) Module.LoadingSet.FilterRequest {
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

    const a = try Module.Info.findByName(ctx.module(), "a", &err);
    defer a.unref();
    const b = try Module.Info.findByName(ctx.module(), "b", &err);
    defer b.unref();
    const c = try Module.Info.findByName(ctx.module(), "c", &err);
    defer c.unref();

    try testing.expect(a.isLoaded());
    try testing.expect(b.isLoaded());
    try testing.expect(c.isLoaded());

    try instance.addDependency(a, &err);
    try instance.addDependency(b, &err);
    try instance.addDependency(c, &err);

    const a0 = try instance.loadSymbol(A0, &err);
    try testing.expectEqual(a0.*, 5);

    try testing.expectError(error.FfiError, instance.loadSymbol(B0, &err));
    try instance.addNamespace(B0.namespace, &err);

    _ = try instance.loadSymbol(B0, &err);

    // Increase the strong reference to ensure that it is not unloaded.
    const info = instance.castOpaque().info;
    info.ref();
    defer info.unref();

    try testing.expect(info.tryRefInstanceStrong());
    defer info.unrefInstanceStrong();

    instance.deinit();
    try testing.expect(a.isLoaded());
    try testing.expect(b.isLoaded());
    try testing.expect(c.isLoaded());
}
