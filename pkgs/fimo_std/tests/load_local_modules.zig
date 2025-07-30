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

fn startCModule(inst: *const modules.OpaqueInstance) !void {
    tracing.emitInfoSimple("starting instance: name='{s}'", .{inst.info.name}, @src());
}

fn stopCModule(inst: *const modules.OpaqueInstance) void {
    tracing.emitInfoSimple("stopping instance: name='{s}'", .{inst.info.name}, @src());
}

const A = struct {
    pub const fimo_module = .{
        .name = .a,
        .description = "Test module a",
        .author = "fimo",
        .license = "MIT or Apache 2.0",
    };
    pub const fimo_exports = .{
        .{ .symbol = A0, .value = &@as(i32, 5) },
        .{ .symbol = A1, .value = &@as(i32, 10) },
    };

    comptime {
        _ = modules.Module(@This());
    }
};

const B = struct {
    pub const fimo_module = .{
        .name = .b,
        .description = "Test module b",
        .author = "fimo",
        .license = "MIT or Apache 2.0",
    };
    pub const fimo_exports = .{
        .{ .symbol = B0, .value = &@as(i32, -2) },
        .{ .symbol = B1, .value = &@as(i32, 77) },
    };

    comptime {
        _ = modules.Module(@This());
    }
};

const C = struct {
    const Module = modules.Module(@This());
    pub const fimo_module = .{
        .name = .c,
        .description = "Test module c",
        .author = "fimo",
        .license = "MIT or Apache 2.0",
    };
    pub const fimo_parameters = .{
        .pub_pub = .{ .read_group = .public, .write_group = .public, .default = @as(u32, 0) },
        .pub_dep = .{ .read_group = .public, .write_group = .dependency, .default = @as(u32, 1) },
        .pub_pri = .{ .read_group = .public, .write_group = .private, .default = @as(u32, 2) },
        .dep_pub = .{ .read_group = .dependency, .write_group = .public, .default = @as(u32, 3) },
        .dep_dep = .{ .read_group = .dependency, .write_group = .dependency, .default = @as(u32, 4) },
        .dep_pri = .{ .read_group = .dependency, .write_group = .private, .default = @as(u32, 5) },
        .pri_pub = .{ .read_group = .private, .write_group = .public, .default = @as(u32, 6) },
        .pri_dep = .{ .read_group = .private, .write_group = .dependency, .default = @as(u32, 7) },
        .pri_pri = .{ .read_group = .private, .write_group = .private, .default = @as(u32, 8) },
    };
    pub const fimo_paths = .{
        .empty = Path.init("") catch unreachable,
        .a = Path.init("a.bin") catch unreachable,
        .b = Path.init("b.txt") catch unreachable,
        .img = Path.init("c/d.img") catch unreachable,
    };
    pub const fimo_imports = .{ .{ A0, A1 }, B0, B1 };
    pub const fimo_events = .{
        .init = init,
        .deinit = deinit,
        .on_start = on_start,
        .on_stop = on_stop,
    };

    fn init() !void {
        tracing.emitInfoSimple("initializing instance: name='{s}'", .{Module.info().name}, @src());

        const parameters = Module.parameters();
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

        const paths = Module.paths();
        tracing.emitTraceSimple("empty: '{f}'", .{paths.empty}, @src());
        tracing.emitTraceSimple("a: '{f}'", .{paths.a}, @src());
        tracing.emitTraceSimple("b: '{f}'", .{paths.b}, @src());
        tracing.emitTraceSimple("img: '{f}'", .{paths.img}, @src());

        const imports = Module.imports();
        try testing.expectEqual(imports.@"0".*, 5);
        try testing.expectEqual(imports.@"1".*, 10);
        try testing.expectEqual(imports.@"2".*, -2);
        try testing.expectEqual(imports.@"3".*, 77);

        try testing.expectEqual(5, Module.provideSymbol(A0).*);
        try testing.expectEqual(10, Module.provideSymbol(A1).*);
        try testing.expectEqual(-2, Module.provideSymbol(B0).*);
        try testing.expectEqual(77, Module.provideSymbol(B1).*);

        try testing.expectEqual(5, A0.requestFrom(Module).*);
        try testing.expectEqual(10, A1.requestFrom(Module).*);
        try testing.expectEqual(-2, B0.requestFrom(Module).*);
        try testing.expectEqual(77, B1.requestFrom(Module).*);

        try testing.expectEqual(5, A0.getGlobal().get().*);
        try testing.expectEqual(10, A1.getGlobal().get().*);
        try testing.expectEqual(-2, B0.getGlobal().get().*);
        try testing.expectEqual(77, B1.getGlobal().get().*);
    }

    fn deinit() void {
        tracing.emitInfoSimple("deinitializing instance: name='{s}'", .{Module.info().name}, @src());
    }

    fn on_start() void {
        tracing.emitInfoSimple("starting instance: name='{s}'", .{Module.info().name}, @src());
    }

    fn on_stop() void {
        tracing.emitInfoSimple("stopping instance: name='{s}'", .{Module.info().name}, @src());
    }

    comptime {
        _ = Module;
    }
};

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
