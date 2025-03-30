const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // ----------------------------------------------------
    // Declare resources.
    // ----------------------------------------------------

    const headers = b.addWriteFiles();
    b.addNamedLazyPath("header", headers.getDirectory());

    const docs = b.addWriteFiles();
    b.addNamedLazyPath("doc", docs.getDirectory());

    // Install the headers.
    _ = headers.addCopyDirectory(b.path("include/"), ".", .{});

    // ----------------------------------------------------
    // fimo_std
    // ----------------------------------------------------

    const fimo_std_dep = b.dependency("fimo_std", .{
        .target = target,
        .optimize = optimize,
        .@"build-static" = true,
    });
    const fimo_std = fimo_std_dep.module("fimo_std");

    // ----------------------------------------------------
    // Module
    // ----------------------------------------------------

    const module_c = b.addTranslateC(.{
        .root_source_file = b.path("include/fimo_tasks_meta/package.h"),
        .target = target,
        .optimize = optimize,
    });
    module_c.addIncludePath(b.path("include/"));
    module_c.addIncludePath(fimo_std_dep.namedLazyPath("header"));

    const module = b.addModule("fimo_tasks_meta", .{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    module.addImport("fimo_std", fimo_std);
    module.addImport("c", module_c.createModule());

    // ----------------------------------------------------
    // Check
    // ----------------------------------------------------

    const module_check = b.addStaticLibrary(.{
        .name = "fimo_tasks_meta",
        .root_module = module,
    });

    const check = b.step("check", "Check if fimo_tasks_meta compiles");
    check.dependOn(&module_check.step);

    // ----------------------------------------------------
    // Test
    // ----------------------------------------------------

    const modules = b.option(
        std.Build.LazyPath,
        "modules",
        "Path to the modules for testing",
    );
    if (modules) |mod| {
        const test_module = b.addTest(.{
            .root_module = b.createModule(.{
                .root_source_file = b.path("src/root.zig"),
                .target = target,
                .optimize = optimize,
            }),
        });
        test_module.root_module.addImport("fimo_std", fimo_std);
        test_module.root_module.addImport("c", module_c.createModule());
        const run_lib_unit_tests = b.addRunArtifact(test_module);
        run_lib_unit_tests.has_side_effects = true;
        run_lib_unit_tests.setCwd(mod);

        const test_step = b.step("test", "Run tests");
        test_step.dependOn(&run_lib_unit_tests.step);
    }

    // ----------------------------------------------------
    // Documentation
    // ----------------------------------------------------

    _ = docs.addCopyDirectory(module_check.getEmittedDocs(), ".", .{});
    const install_doc = b.addInstallDirectory(.{
        .source_dir = module_check.getEmittedDocs(),
        .install_dir = .prefix,
        .install_subdir = "doc",
    });
    const doc_step = b.step("doc", "Generate documentation");
    doc_step.dependOn(&install_doc.step);
}
