const std = @import("std");
const testing = std.testing;

const fimo_std = @import("fimo_std");
const Version = fimo_std.Version;
const Path = fimo_std.path.Path;
const ctx = fimo_std.ctx;
const tracing = fimo_std.tracing;
const modules = fimo_std.modules;
const tasks = fimo_std.tasks;

const A0 = modules.Symbol{
    .name = "a_export_0",
    .version = Version.parse("0.1.0") catch unreachable,
    .T = i32,
};
const A1 = modules.Symbol{
    .name = "a_export_1",
    .version = Version.parse("0.1.0") catch unreachable,
    .T = i32,
};
const B0 = modules.Symbol{
    .name = "b_export_0",
    .namespace = "b",
    .version = Version.parse("0.1.0") catch unreachable,
    .T = i32,
};
const B1 = modules.Symbol{
    .name = "b_export_1",
    .namespace = "b",
    .version = Version.parse("0.1.0") catch unreachable,
    .T = i32,
};

fn initCModule(oinst: *const modules.OpaqueInstance, set: modules.LoadingSet) !void {
    _ = set;
    const inst: *const C = @alignCast(@ptrCast(oinst));
    tracing.emitInfoSimple(
        "initializing instance: name='{s}'",
        .{inst.info.name},
        @src(),
    );

    const parameters = inst.parameters();
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

    const resources = inst.resources();
    tracing.emitTraceSimple("empty: '{f}'", .{resources.empty}, @src());
    tracing.emitTraceSimple("a: '{f}'", .{resources.a}, @src());
    tracing.emitTraceSimple("b: '{f}'", .{resources.b}, @src());
    tracing.emitTraceSimple("img: '{f}'", .{resources.img}, @src());

    const imports = inst.imports();
    try testing.expectEqual(imports.@"0".*, 5);
    try testing.expectEqual(imports.@"1".*, 10);
    try testing.expectEqual(imports.b0.*, -2);
    try testing.expectEqual(imports.b1.*, 77);

    try testing.expectEqual(5, inst.provideSymbol(A0).*);
    try testing.expectEqual(10, inst.provideSymbol(A1).*);
    try testing.expectEqual(-2, inst.provideSymbol(B0).*);
    try testing.expectEqual(77, inst.provideSymbol(B1).*);

    try testing.expectEqual(5, A0.requestFrom(inst).*);
    try testing.expectEqual(10, A1.requestFrom(inst).*);
    try testing.expectEqual(-2, B0.requestFrom(inst).*);
    try testing.expectEqual(77, B1.requestFrom(inst).*);
}

fn deinitCModule(inst: *const modules.OpaqueInstance) void {
    tracing.emitInfoSimple("deinitializing instance: name='{s}'", .{inst.info.name}, @src());
}

fn startCModule(inst: *const modules.OpaqueInstance) !void {
    tracing.emitInfoSimple("starting instance: name='{s}'", .{inst.info.name}, @src());
}

fn stopCModule(inst: *const modules.OpaqueInstance) void {
    tracing.emitInfoSimple("stopping instance: name='{s}'", .{inst.info.name}, @src());
}

const A = modules.exports.Builder
    .init("a")
    .withDescription("Test module a")
    .withAuthor("fimo")
    .withLicense("MIT and Apache 2.0")
    .withExport(.{ .symbol = A0 }, &5)
    .withExport(.{ .symbol = A1 }, &10)
    .exportModule();

const B = modules.exports.Builder
    .init("b")
    .withDescription("Test module b")
    .withAuthor("fimo")
    .withLicense("MIT and Apache 2.0")
    .withExport(.{ .symbol = B0, .name = "b0" }, &-2)
    .withExport(.{ .symbol = B1, .name = "b1" }, &77)
    .exportModule();

const C = modules.exports.Builder
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
    .withMultipleImports(.{ A0, A1 })
    .withMultipleImports(.{ .b0 = B0, .b1 = B1 })
    .withStateSync(void, initCModule, deinitCModule)
    .withOnStartEventSync(startCModule)
    .withOnStopEvent(stopCModule)
    .exportModule();

comptime {
    _ = A;
    _ = B;
    _ = C;
}

pub fn main() !void {
    const tracing_cfg = tracing.Config{
        .max_level = .trace,
        .subscribers = &.{tracing.default_subscriber},
        .subscriber_count = 1,
    };
    defer tracing_cfg.deinit();
    const init_options: [:null]const ?*const ctx.ConfigHead = &.{@ptrCast(&tracing_cfg)};

    try ctx.init(init_options);
    defer ctx.deinit();

    var err: ?fimo_std.AnyError = null;
    defer if (err) |e| e.deinit();

    tracing.registerThread();
    defer tracing.unregisterThread();

    const async_ctx = try tasks.BlockingContext.init(&err);
    defer async_ctx.deinit();

    defer modules.pruneInstances(&err) catch unreachable;

    const set = try modules.LoadingSet.init(&err);
    defer set.unref();

    try set.addModulesFromLocal(
        &{},
        struct {
            fn f(@"export": *const modules.Export, data: *const void) modules.LoadingSet.FilterRequest {
                _ = @"export";
                _ = data;
                return .load;
            }
        }.f,
        null,
        &err,
    );
    try set.commit().intoFuture().awaitBlocking(async_ctx).unwrap(&err);

    var instance_init = true;
    const instance = try modules.PseudoInstance.init(&err);
    defer if (instance_init) instance.deinit();

    const a = try modules.Info.findByName("a", &err);
    defer a.unref();
    const b = try modules.Info.findByName("b", &err);
    defer b.unref();
    const c = try modules.Info.findByName("c", &err);
    defer c.unref();

    try testing.expect(a.isLoaded());
    try testing.expect(b.isLoaded());
    try testing.expect(c.isLoaded());

    try instance.addDependency(a, &err);
    try instance.addDependency(b, &err);
    try instance.addDependency(c, &err);

    const a0 = try instance.loadSymbol(A0, &err);
    try testing.expectEqual(a0.value.*, 5);

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
    instance_init = false;
    try testing.expect(a.isLoaded());
    try testing.expect(b.isLoaded());
    try testing.expect(c.isLoaded());
}
