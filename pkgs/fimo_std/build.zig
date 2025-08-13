const std = @import("std");
const builtin = @import("builtin");

const build_internals = @import("tools/build-internals");

pub fn configure(b: *build_internals.FimoBuild) void {
    // const lz4_dependency = b.dependency("lz4", .{
    //     .target = b.graph.target,
    //     .optimize = b.graph.optimize,
    // });
    // const lz4 = lz4_dependency.artifact("lz4");
    // _ = lz4; // autofix

    // Generate additional build files.
    const wf = b.build.addWriteFiles();
    const context_version = generateVersion(b.build, wf);

    const headers = b.build.addWriteFiles();
    _ = headers.addCopyDirectory(b.build.path("include/"), ".", .{});
    _ = headers.addCopyDirectory(wf.getDirectory().path(b.build, "include/"), ".", .{});
    _ = headers.addCopyFile(b.build.path("LICENSE-MIT"), "fimo_std/LICENSE-MIT");
    _ = headers.addCopyFile(b.build.path("LICENSE-APACHE"), "fimo_std/LICENSE-APACHE");

    const translate_c = b.build.addTranslateC(.{
        .root_source_file = headers.getDirectory().path(b.build, "fimo_std/fimo.h"),
        .target = b.graph.target,
        .optimize = b.graph.optimize,
    });
    translate_c.addIncludePath(headers.getDirectory());

    const module = b.build.addModule("fimo_std", .{
        .root_source_file = b.build.path("src/root.zig"),
        .target = b.graph.target,
        .optimize = b.graph.optimize,
        .link_libc = true,
        .pic = true,
    });
    module.addImport("c", translate_c.createModule());
    module.addImport("context_version", context_version);
    module.addIncludePath(headers.getDirectory());

    const pkg = b.addPackage(.{
        .name = "fimo_std",
        .root_module = module,
        .headers = headers.getDirectory(),
    });

    _ = pkg.addTest(.{
        .step = .{
            .module = blk: {
                const t = b.build.addModule("fimo_std", .{
                    .root_source_file = b.build.path("src/root.zig"),
                    .target = b.graph.target,
                    .optimize = b.graph.optimize,
                    .valgrind = b.graph.target.result.os.tag == .linux,
                    .link_libc = true,
                    .pic = true,
                });
                t.addImport("c", translate_c.createModule());
                t.addImport("context_version", context_version);
                t.addIncludePath(headers.getDirectory());
                t.addImport("fimo_std", t);

                break :blk t;
            },
        },
    });

    const event_loop_test = b.build.addExecutable(.{
        .name = "event_loop_test",
        .root_module = b.build.createModule(.{
            .target = b.graph.target,
            .optimize = b.graph.optimize,
            .root_source_file = b.build.path("tests/event_loop.zig"),
            .valgrind = b.graph.target.result.os.tag == .linux,
        }),
        .use_llvm = if (b.graph.target.result.os.tag == .linux) true else null,
    });
    event_loop_test.root_module.addImport("fimo_std", pkg.root_module);
    _ = pkg.addTest(.{ .name = "event_loop_test", .step = .{ .executable = event_loop_test } });

    const init_ctx_test = b.build.addExecutable(.{
        .name = "init_context_test",
        .root_module = b.build.createModule(.{
            .target = b.graph.target,
            .optimize = b.graph.optimize,
            .root_source_file = b.build.path("tests/init_context.zig"),
            .valgrind = b.graph.target.result.os.tag == .linux,
        }),
        .use_llvm = if (b.graph.target.result.os.tag == .linux) true else null,
    });
    init_ctx_test.root_module.addImport("fimo_std", pkg.root_module);
    _ = pkg.addTest(.{ .name = "init_context_test", .step = .{ .executable = init_ctx_test } });

    const local_modules_test = b.build.addExecutable(.{
        .name = "local_modules_test",
        .root_module = b.build.createModule(.{
            .target = b.graph.target,
            .optimize = b.graph.optimize,
            .root_source_file = b.build.path("tests/load_local_modules.zig"),
            .valgrind = b.graph.target.result.os.tag == .linux,
        }),
        .use_llvm = if (b.graph.target.result.os.tag == .linux) true else null,
    });
    local_modules_test.root_module.addImport("fimo_std", pkg.root_module);
    _ = pkg.addTest(.{ .name = "local_modules_test", .step = .{ .executable = local_modules_test } });
}

pub fn build(b: *std.Build) void {
    const install_step = b.getInstallStep();
    const test_step = b.step("test", "Run tests");
    const check_step = b.step("check", "Check compilation");

    const build_standalone = b.option(bool, "build-standalone", "Build the package in standalone mode") orelse false;
    const build_static = b.option(bool, "build-static", "Build static library") orelse false;
    const build_dynamic = b.option(bool, "build-dynamic", "Build dynamic library") orelse false;

    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    if (!build_standalone) return;

    const build_internals_dep = b.dependency("tools/build-internals", .{});
    var build_ = build_internals.FimoBuild.createRoot(.{
        .build = b,
        .build_dep = build_internals_dep,
        .target = target,
        .optimize = optimize,
    });
    configure(build_);

    const pkg = build_.getPackage("fimo_std");
    install_step.dependOn(&pkg.addInstallHeaders().?.step);
    for (pkg.tests.items) |t| test_step.dependOn(&t.getRunArtifact().step);

    const static_lib = b.addLibrary(.{
        .linkage = .static,
        .name = "fimo_std",
        .root_module = pkg.root_module,
        .use_llvm = if (target.result.os.tag == .linux) true else null,
    });
    static_lib.bundle_compiler_rt = true;
    if (target.result.os.tag == .windows) static_lib.dll_export_fns = true;
    if (build_static) b.installArtifact(static_lib);

    const dynamic_lib = b.addLibrary(.{
        .linkage = .dynamic,
        .name = "fimo_std_shared",
        .root_module = pkg.root_module,
        .use_llvm = if (target.result.os.tag == .linux) true else null,
    });
    if (build_dynamic) b.installArtifact(dynamic_lib);

    check_step.dependOn(&static_lib.step);
}

fn generateVersion(
    b: *std.Build,
    wf: *std.Build.Step.WriteFile,
) *std.Build.Module {
    const header_contents = b.fmt(
        \\ // Machine generated
        \\ #define FIMO_CONTEXT_VERSION_MAJOR {}
        \\ #define FIMO_CONTEXT_VERSION_MINOR {}
        \\ #define FIMO_CONTEXT_VERSION_PATCH {}
        \\ #define FIMO_CONTEXT_VERSION_PRE "{s}"
        \\ #define FIMO_CONTEXT_VERSION_PRE_LEN {}
        \\ #define FIMO_CONTEXT_VERSION_BUILD "{s}"
        \\ #define FIMO_CONTEXT_VERSION_BUILD_LEN {}
        \\
    , .{
        build_internals.fimo_version.major,
        build_internals.fimo_version.minor,
        build_internals.fimo_version.patch,
        build_internals.fimo_version.pre orelse "",
        (build_internals.fimo_version.pre orelse "").len,
        build_internals.fimo_version.build orelse "",
        (build_internals.fimo_version.build orelse "").len,
    });
    _ = wf.add("include/fimo_std/impl/context_version_.h", header_contents);

    const options = b.addOptions();
    options.addOption(std.SemanticVersion, "version", build_internals.fimo_version);
    return options.createModule();
}
