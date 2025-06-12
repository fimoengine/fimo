const std = @import("std");

const build_internals = @import("tools/build-internals");

pub fn configure(builder: *build_internals.FimoBuild) void {
    const b = builder.build;
    const fimo_std_pkg = builder.getPackage("fimo_std");
    const fimo_tasks_meta_pkg = builder.getPackage("fimo_tasks_meta");
    const fimo_worlds_meta_pkg = builder.getPackage("fimo_worlds_meta");

    const fimo_worlds = b.addModule("fimo_worlds", .{
        .root_source_file = b.path("src/root.zig"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
    });
    fimo_worlds.addImport("fimo_std", fimo_std_pkg.root_module);
    fimo_worlds.addImport("fimo_tasks_meta", fimo_tasks_meta_pkg.root_module);
    fimo_worlds.addImport("fimo_worlds_meta", fimo_worlds_meta_pkg.root_module);

    const module = builder.addModule(.{
        .name = "fimo_worlds",
        .root_module = fimo_worlds,
    });

    _ = module.addTest(.{
        .name = "fimo_worlds_test",
        .step = .{
            .module = blk: {
                const t = b.createModule(.{
                    .root_source_file = b.path("src/root.zig"),
                    .target = builder.graph.target,
                    .optimize = builder.graph.optimize,
                    .valgrind = builder.graph.target.result.os.tag == .linux,
                });
                t.addImport("fimo_std", fimo_std_pkg.root_module);
                t.addImport("fimo_tasks_meta", fimo_tasks_meta_pkg.root_module);
                t.addImport("fimo_worlds_meta", fimo_worlds_meta_pkg.root_module);

                break :blk t;
            },
        },
    });
}

pub fn build(b: *std.Build) void {
    _ = b;
}
