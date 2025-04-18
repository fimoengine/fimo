const std = @import("std");
const builtin = @import("builtin");

const build_internals = @import("tools/build-internals");

/// Must match the `version` in `build.zig.zon`.
const fimo_version: std.SemanticVersion = .{ .major = 0, .minor = 2, .patch = 0, .pre = "dev" };

pub fn configure(b: *build_internals.FimoBuild) void {
    // Generate additional build files.
    const wf = b.build.addWriteFiles();
    const context_version = generateVersion(b.build, wf);
    const visualizers = generateGDBScripts(b.build, wf);

    const headers = b.build.addWriteFiles();
    _ = headers.addCopyDirectory(b.build.path("include/"), ".", .{});
    _ = headers.addCopyDirectory(wf.getDirectory().path(b.build, "include/"), ".", .{});
    _ = headers.addCopyFile(b.build.path("LICENSE-MIT"), "fimo_std/LICENSE-MIT");
    _ = headers.addCopyFile(b.build.path("LICENSE-APACHE"), "fimo_std/LICENSE-APACHE");

    const module = b.build.addModule("fimo_std", .{
        .root_source_file = b.build.path("src/root.zig"),
        .target = b.graph.target,
        .optimize = b.graph.optimize,
        .link_libc = true,
        .pic = true,
    });
    module.addImport("context_version", context_version);
    module.addImport("visualizers", visualizers);
    module.addIncludePath(headers.getDirectory());

    const pkg = b.addPackage(.{
        .name = "fimo_std",
        .root_module = module,
        .headers = headers.getDirectory(),
    });

    _ = pkg.addTest(.{ .step = .{ .module = module } });

    const event_loop_test = b.build.addExecutable(.{
        .name = "event_loop_test",
        .root_module = b.build.createModule(.{
            .target = b.graph.target,
            .optimize = b.graph.optimize,
            .root_source_file = b.build.path("tests/event_loop.zig"),
        }),
    });
    event_loop_test.root_module.addImport("fimo_std", pkg.root_module);
    _ = pkg.addTest(.{ .name = "event_loop_test", .step = .{ .executable = event_loop_test } });

    const init_ctx_test = b.build.addExecutable(.{
        .name = "init_context_test",
        .root_module = b.build.createModule(.{
            .target = b.graph.target,
            .optimize = b.graph.optimize,
            .root_source_file = b.build.path("tests/init_context.zig"),
        }),
    });
    init_ctx_test.root_module.addImport("fimo_std", pkg.root_module);
    _ = pkg.addTest(.{ .name = "init_context_test", .step = .{ .executable = init_ctx_test } });

    const local_modules_test = b.build.addExecutable(.{
        .name = "local_modules_test",
        .root_module = b.build.createModule(.{
            .target = b.graph.target,
            .optimize = b.graph.optimize,
            .root_source_file = b.build.path("tests/load_local_modules.zig"),
        }),
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
    });
    static_lib.bundle_compiler_rt = true;
    if (target.result.os.tag == .windows) static_lib.dll_export_fns = true;
    if (build_static) b.installArtifact(static_lib);

    const dynamic_lib = b.addLibrary(.{
        .linkage = .dynamic,
        .name = "fimo_std_shared",
        .root_module = pkg.root_module,
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
        fimo_version.major,
        fimo_version.minor,
        fimo_version.patch,
        fimo_version.pre orelse "",
        (fimo_version.pre orelse "").len,
        fimo_version.build orelse "",
        (fimo_version.build orelse "").len,
    });
    _ = wf.add("include/fimo_std/impl/context_version_.h", header_contents);

    const options = b.addOptions();
    options.addOption(std.SemanticVersion, "version", fimo_version);
    return options.createModule();
}

fn generateGDBScripts(
    b: *std.Build,
    wf: *std.Build.Step.WriteFile,
) *std.Build.Module {
    const gdbscript_to_zig_exe = b.addExecutable(.{
        .name = "gdbscript_to_zig",
        .root_source_file = b.path("tools/gdbscript_to_zig.zig"),
        .target = b.graph.host,
        .optimize = .Debug,
    });

    var dir = b.build_root.handle.openDir("visualizers/gdb", .{ .iterate = true }) catch |err| {
        std.debug.panic("unable to open '{}visualizers/gdb' directory: {s}", .{
            b.build_root,
            @errorName(err),
        });
    };
    defer dir.close();

    var root_file_bytes = std.ArrayList(u8).init(b.allocator);
    defer root_file_bytes.deinit();
    root_file_bytes.appendSlice("const builtin = @import(\"builtin\");\n\n") catch unreachable;
    root_file_bytes.appendSlice("comptime {\n") catch unreachable;
    root_file_bytes.appendSlice("\tif (builtin.os.tag != .windows and !builtin.target.os.tag.isDarwin()) {\n") catch unreachable;

    var it = dir.iterateAssumeFirstIteration();
    while (it.next() catch @panic("failed to read dir")) |entry| {
        if (entry.kind != .file or !std.mem.endsWith(u8, entry.name, ".py")) {
            continue;
        }

        const out_basename = b.fmt("{s}.zig", .{std.fs.path.stem(entry.name)});
        const out_path = b.fmt("zig_visualizers/{s}", .{out_basename});
        const cmd = b.addRunArtifact(gdbscript_to_zig_exe);

        root_file_bytes.appendSlice(b.fmt(
            "\t\t_ = @import(\"{s}\");\n",
            .{out_basename},
        )) catch unreachable;

        _ = wf.addCopyFile(cmd.addOutputFileArg(out_basename), out_path);
        cmd.addFileArg(b.path(b.fmt("visualizers/gdb/{s}", .{entry.name})));
    }

    root_file_bytes.appendSlice("\t}\n") catch unreachable;
    root_file_bytes.appendSlice("}\n") catch unreachable;
    const root_file = wf.add("zig_visualizers/root.zig", root_file_bytes.items);
    return b.createModule(.{ .root_source_file = root_file });
}
