const std = @import("std");
const testing = std.testing;

const fimo_std = @import("fimo_std");
const Version = fimo_std.Version;
const Path = fimo_std.paths.Path;
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
    tracing.logInfo(@src(), "starting instance: name='{s}'", .{inst.info.name});
}

fn stopCModule(inst: *const modules.OpaqueInstance) void {
    tracing.logInfo(@src(), "stopping instance: name='{s}'", .{inst.info.name});
}

const Modules = modules.ModuleBundle(.{
    modules.Module(A),
    modules.Module(B),
    modules.Module(C),
});

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
        tracing.logInfo(@src(), "initializing instance: name='{s}'", .{Module.info().name});

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
        tracing.logTrace(@src(), "empty: '{f}'", .{paths.empty});
        tracing.logTrace(@src(), "a: '{f}'", .{paths.a});
        tracing.logTrace(@src(), "b: '{f}'", .{paths.b});
        tracing.logTrace(@src(), "img: '{f}'", .{paths.img});

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
        tracing.logInfo(@src(), "deinitializing instance: name='{s}'", .{Module.info().name});
    }

    fn on_start() void {
        tracing.logInfo(@src(), "starting instance: name='{s}'", .{Module.info().name});
    }

    fn on_stop() void {
        tracing.logInfo(@src(), "stopping instance: name='{s}'", .{Module.info().name});
    }
};

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
    errdefer if (ctx.hasErrorResult()) {
        const e = ctx.takeResult().unwrapErr();
        defer e.deinit();
        tracing.logErr(@src(), "{f}", .{e});
        e.deinit();
    };

    const async_ctx = try tasks.BlockingContext.init();
    defer async_ctx.deinit();

    const set = try modules.LoadingSet.init();
    defer set.deinit();

    try set.addModulesFromLocal({}, Modules.loadingSetFilter);
    try set.commit().intoFuture().awaitBlocking(async_ctx).unwrap();

    var instance_init = true;
    const instance = try modules.RootInstance.init();
    defer if (instance_init) instance.deinit();

    const a = try modules.Info.findByName("a");
    defer a.unref();
    const b = try modules.Info.findByName("b");
    defer b.unref();
    const c = try modules.Info.findByName("c");
    defer c.unref();

    try testing.expect(a.isLoaded());
    try testing.expect(b.isLoaded());
    try testing.expect(c.isLoaded());

    try instance.addDependency(a);
    try instance.addDependency(b);
    try instance.addDependency(c);

    const a0 = try instance.loadSymbol(A0);
    try testing.expectEqual(a0.value.*, 5);

    try testing.expect(if (instance.loadSymbol(B0)) |_| false else |_| true);
    try instance.addNamespace(B0.namespace);

    _ = try instance.loadSymbol(B0);

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
