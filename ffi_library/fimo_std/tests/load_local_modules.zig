const std = @import("std");
const testing = std.testing;

const fimo_std = @import("fimo_std");

const Version = fimo_std.Version;
const Context = fimo_std.Context;
const Tracing = Context.Tracing;
const Module = Context.Module;

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

const A = Module.Instance(
    void,
    void,
    void,
    extern struct {
        a0: *const i32,
        a1: *const i32,
    },
    void,
);

const B = Module.Instance(
    void,
    void,
    void,
    extern struct {
        b0: *const i32,
        b1: *const i32,
    },
    void,
);

const C = Module.Instance(
    extern struct {
        pub_pub: *Module.Parameter(u32, Module.OpaqueInstance),
        pub_dep: *Module.Parameter(u32, Module.OpaqueInstance),
        pub_pri: *Module.Parameter(u32, Module.OpaqueInstance),
        dep_pub: *Module.Parameter(u32, Module.OpaqueInstance),
        dep_dep: *Module.Parameter(u32, Module.OpaqueInstance),
        dep_pri: *Module.Parameter(u32, Module.OpaqueInstance),
        pri_pub: *Module.Parameter(u32, Module.OpaqueInstance),
        pri_dep: *Module.Parameter(u32, Module.OpaqueInstance),
        pri_pri: *Module.Parameter(u32, Module.OpaqueInstance),
    },
    extern struct {
        empty: [*:0]const u8,
        a: [*:0]const u8,
        b: [*:0]const u8,
        img: [*:0]const u8,
    },
    extern struct {
        a0: *const i32,
        a1: *const i32,
        b0: *const i32,
        b1: *const i32,
    },
    void,
    void,
);

comptime {
    Module.Export.addExport(
        A,
        "a",
        null,
        null,
        null,
        .{},
        .{},
        &.{},
        .{},
        .{
            .a0 = .{ .id = A0, .symbol = &@as(i32, 5) },
            .a1 = .{ .id = A1, .symbol = &@as(i32, 10) },
        },
        &.{},
        null,
        null,
    );
    Module.Export.addExport(
        B,
        "b",
        null,
        null,
        null,
        .{},
        .{},
        &.{},
        .{},
        .{
            .b0 = .{ .id = B0, .symbol = &@as(i32, -2) },
            .b1 = .{ .id = B1, .symbol = &@as(i32, 77) },
        },
        &.{},
        null,
        null,
    );
    Module.Export.addExport(
        C,
        "c",
        null,
        null,
        null,
        .{
            .pub_pub = Module.Export.Parameter{
                .type = .u32,
                .name = "pub_pub",
                .read_group = .public,
                .write_group = .public,
                .default_value = .{ .u32 = 0 },
            },
            .pub_dep = Module.Export.Parameter{
                .type = .u32,
                .name = "pub_dep",
                .read_group = .public,
                .write_group = .dependency,
                .default_value = .{ .u32 = 1 },
            },
            .pub_pri = Module.Export.Parameter{
                .type = .u32,
                .name = "pub_pri",
                .read_group = .public,
                .default_value = .{ .u32 = 2 },
            },
            .dep_pub = Module.Export.Parameter{
                .type = .u32,
                .name = "dep_pub",
                .read_group = .dependency,
                .write_group = .public,
                .default_value = .{ .u32 = 3 },
            },
            .dep_dep = Module.Export.Parameter{
                .type = .u32,
                .name = "dep_dep",
                .read_group = .dependency,
                .write_group = .dependency,
                .default_value = .{ .u32 = 4 },
            },
            .dep_pri = Module.Export.Parameter{
                .type = .u32,
                .name = "dep_pri",
                .read_group = .dependency,
                .default_value = .{ .u32 = 5 },
            },
            .pri_pub = Module.Export.Parameter{
                .type = .u32,
                .name = "pri_pub",
                .write_group = .public,
                .default_value = .{ .u32 = 6 },
            },
            .pri_dep = Module.Export.Parameter{
                .type = .u32,
                .name = "pri_dep",
                .write_group = .dependency,
                .default_value = .{ .u32 = 7 },
            },
            .pri_pri = Module.Export.Parameter{
                .type = .u32,
                .name = "pri_pri",
                .default_value = .{ .u32 = 8 },
            },
        },
        .{
            .empty = Module.Export.Resource{ .path = "" },
            .a = Module.Export.Resource{ .path = "a.bin" },
            .b = Module.Export.Resource{ .path = "b.txt" },
            .img = Module.Export.Resource{ .path = "c/d.img" },
        },
        &.{},
        .{
            .a0 = A0,
            .a1 = A1,
            .b0 = B0,
            .b1 = B1,
        },
        .{},
        &.{},
        initCModule,
        deinitCModule,
    );
}

fn initCModule(inst: *const C, set: *Module.LoadingSet) !void {
    _ = set;
    inst.context().tracing().emitInfoSimple(
        "initializing instance: name='{s}'",
        .{inst.info.name},
        @src(),
    );

    var err: ?fimo_std.AnyError = null;
    defer if (err) |e| e.deinit();

    const parameters = inst.parameters;
    try testing.expectEqual(0, try parameters.pub_pub.read(inst.castOpaque(), &err));
    try testing.expectEqual(1, try parameters.pub_dep.read(inst.castOpaque(), &err));
    try testing.expectEqual(2, try parameters.pub_pri.read(inst.castOpaque(), &err));
    try testing.expectEqual(3, try parameters.dep_pub.read(inst.castOpaque(), &err));
    try testing.expectEqual(4, try parameters.dep_dep.read(inst.castOpaque(), &err));
    try testing.expectEqual(5, try parameters.dep_pri.read(inst.castOpaque(), &err));
    try testing.expectEqual(6, try parameters.pri_pub.read(inst.castOpaque(), &err));
    try testing.expectEqual(7, try parameters.pri_dep.read(inst.castOpaque(), &err));
    try testing.expectEqual(8, try parameters.pri_pri.read(inst.castOpaque(), &err));

    try parameters.pub_pub.write(inst.castOpaque(), 0, &err);
    try parameters.pub_dep.write(inst.castOpaque(), 1, &err);
    try parameters.pub_pri.write(inst.castOpaque(), 2, &err);
    try parameters.dep_pub.write(inst.castOpaque(), 3, &err);
    try parameters.dep_dep.write(inst.castOpaque(), 4, &err);
    try parameters.dep_pri.write(inst.castOpaque(), 5, &err);
    try parameters.pri_pub.write(inst.castOpaque(), 6, &err);
    try parameters.pri_dep.write(inst.castOpaque(), 7, &err);
    try parameters.pri_pri.write(inst.castOpaque(), 8, &err);

    const resources = inst.resources;
    inst.context().tracing().emitTraceSimple("empty: '{s}'", .{resources.empty}, @src());
    inst.context().tracing().emitTraceSimple("a: '{s}'", .{resources.a}, @src());
    inst.context().tracing().emitTraceSimple("b: '{s}'", .{resources.b}, @src());
    inst.context().tracing().emitTraceSimple("img: '{s}'", .{resources.img}, @src());

    const imports = inst.imports;
    try testing.expectEqual(imports.a0.*, 5);
    try testing.expectEqual(imports.a1.*, 10);
    try testing.expectEqual(imports.b0.*, -2);
    try testing.expectEqual(imports.b1.*, 77);
}

fn deinitCModule(inst: *const C) void {
    inst.context().tracing().emitInfoSimple(
        "deinitializing instance: name='{s}'",
        .{inst.info.name},
        @src(),
    );
}

pub fn main() !void {
    const tracing_cfg = Tracing.Config{
        .max_level = .trace,
        .subscribers = &.{Tracing.default_subscriber},
        .subscriber_count = 1,
    };
    const init_options: [:null]const ?*const Context.TaggedInStruct = &.{@ptrCast(&tracing_cfg)};

    const ctx = try Context.init(init_options);
    defer ctx.unref();

    var err: ?fimo_std.AnyError = null;
    defer if (err) |e| e.deinit();

    try ctx.tracing().registerThread(&err);
    defer ctx.tracing().unregisterThread(&err) catch unreachable;

    const set = try Module.LoadingSet.init(ctx.module(), &err);
    try set.addModulesFromPath(
        ctx.module(),
        null,
        &{},
        struct {
            fn f(@"export": *const Module.Export, data: *const void) bool {
                _ = @"export";
                _ = data;
                return true;
            }
        }.f,
        &err,
    );
    try set.commit(ctx.module(), &err);

    const instance = try Module.PseudoInstance.init(ctx.module(), &err);
    errdefer (instance.deinit(&err) catch unreachable).unref();

    const a = try Module.Info.findByName(ctx.module(), "a", &err);
    defer a.release();
    const b = try Module.Info.findByName(ctx.module(), "b", &err);
    defer b.release();
    const c = try Module.Info.findByName(ctx.module(), "c", &err);
    defer c.release();

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

    (instance.deinit(&err) catch unreachable).unref();
    try testing.expect(!a.isLoaded());
    try testing.expect(!b.isLoaded());
    try testing.expect(!c.isLoaded());
}
