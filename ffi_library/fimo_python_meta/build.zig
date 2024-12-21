const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // ----------------------------------------------------
    // Declare resources.
    // ----------------------------------------------------

    const headers = b.addWriteFiles();
    b.addNamedLazyPath("header", headers.getDirectory());

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

    const module = b.addModule("fimo_python_meta", .{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    module.addIncludePath(b.path("include/"));
    module.addIncludePath(fimo_std_dep.path("include/"));
    module.addImport("fimo_std", fimo_std);

    // ----------------------------------------------------
    // Check
    // ----------------------------------------------------

    const module_check = b.addStaticLibrary(.{
        .name = "fimo_python_meta",
        .root_module = module,
    });

    const check = b.step("check", "Check if fimo_python_meta compiles");
    check.dependOn(&module_check.step);

    // ----------------------------------------------------
    // Test
    // ----------------------------------------------------

    const test_module = b.addTest(.{ .root_module = module });

    const run_lib_unit_tests = b.addRunArtifact(test_module);

    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&run_lib_unit_tests.step);
}
