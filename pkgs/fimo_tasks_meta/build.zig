const std = @import("std");

const build_internals = @import("tools/build-internals");

pub fn configure(builder: *build_internals.FimoBuild) void {
    const b = builder.build;
    const fimo_std_pkg = builder.getPackage("fimo_std");

    const module_c = b.addTranslateC(.{
        .root_source_file = b.path("include/fimo_tasks_meta/package.h"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
    });
    module_c.addIncludePath(b.path("include/"));
    module_c.addIncludePath(fimo_std_pkg.headers.?);

    const module = b.addModule("fimo_tasks_meta", .{
        .root_source_file = b.path("src/root.zig"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
    });
    module.addImport("fimo_std", fimo_std_pkg.root_module);
    module.addImport("c", module_c.createModule());

    const pkg = builder.addPackage(.{
        .name = "fimo_tasks_meta",
        .root_module = module,
        .headers = b.path("include/"),
    });

    const wf = b.addWriteFiles();
    const test_c_headers = wf.addCopyDirectory(b.path("include/"), "include", .{});
    const test_src = wf.addCopyDirectory(b.path("src/"), "src", .{});

    const test_module_c = b.addTranslateC(.{
        .root_source_file = test_c_headers.path(b, "fimo_tasks_meta/package.h"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
    });
    test_module_c.addIncludePath(b.path("include/"));
    test_module_c.addIncludePath(fimo_std_pkg.headers.?);

    const test_module = b.createModule(.{
        .root_source_file = test_src.path(b, "root.zig"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
        .valgrind = builder.graph.target.result.os.tag == .linux,
    });
    test_module.addImport("fimo_std", fimo_std_pkg.root_module);
    test_module.addImport("c", test_module_c.createModule());

    _ = pkg.addTest(.{
        .name = "fimo_tasks_meta_test",
        .step = .{ .module = test_module },
        .configure = &struct {
            fn f(t: *build_internals.FimoBuild.Test) void {
                const fimo_tasks = t.owner.getModule("fimo_tasks");
                t.step.module.addImport("test_module", fimo_tasks.getLinkModule());
            }
        }.f,
    });
}

pub fn build(b: *std.Build) void {
    _ = b;
}
