const std = @import("std");

const build_internals = @import("tools/build-internals");

pub fn configure(builder: *build_internals.FimoBuild) void {
    const b = builder.build;
    const fimo_std_pkg = builder.getPackage("fimo_std");

    const translate_c = b.addTranslateC(.{
        .root_source_file = b.path("fimo_tasks.h"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
    });
    translate_c.addIncludePath(fimo_std_pkg.headers.?);
    const module_c = translate_c.createModule();

    const module = b.addModule("fimo_tasks_meta", .{
        .root_source_file = b.path("src/root.zig"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
    });
    module.addImport("fimo_std", fimo_std_pkg.root_module);
    module.addImport("c", module_c);

    const pkg = builder.addPackage(.{
        .name = "fimo_tasks_meta",
        .root_module = module,
        .headers = b.path(""),
    });

    const wf = b.addWriteFiles();
    const test_src = wf.addCopyDirectory(b.path("src/"), "src", .{});

    const test_module = b.createModule(.{
        .root_source_file = test_src.path(b, "root.zig"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
        .valgrind = builder.graph.target.result.os.tag == .linux,
    });
    test_module.addImport("fimo_std", fimo_std_pkg.root_module);
    test_module.addImport("c", module_c);

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
