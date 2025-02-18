const std = @import("std");
const builtin = @import("builtin");

/// Must match the `version` in `build.zig.zon`.
const fimo_version: std.SemanticVersion = .{ .major = 0, .minor = 2, .patch = 0, .pre = "dev" };

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Generate additional build files.
    const wf = b.addWriteFiles();
    const context_version = generateVersion(b, wf);
    const visualizers = generateGDBScripts(b, wf);
    generateLicenseFile(b, wf);

    // ----------------------------------------------------
    // Declare resources.
    // ----------------------------------------------------

    const libs = b.addWriteFiles();
    const bins = b.addWriteFiles();
    const headers = b.addWriteFiles();
    const docs = b.addWriteFiles();

    b.addNamedLazyPath("lib", libs.getDirectory());
    b.addNamedLazyPath("bin", bins.getDirectory());
    b.addNamedLazyPath("header", headers.getDirectory());
    b.addNamedLazyPath("doc", docs.getDirectory());

    // Install the headers.
    _ = headers.addCopyDirectory(b.path("include/"), ".", .{});
    _ = headers.addCopyDirectory(wf.getDirectory().join(b.allocator, "include/") catch unreachable, ".", .{});
    // Install the natvis files.
    _ = headers.addCopyDirectory(b.path("visualizers/natvis"), "fimo_std/impl/natvis", .{});
    // Install the generated license.
    _ = headers.addCopyFile(wf.getDirectory().path(b, "LICENSE.txt"), "fimo_std/LICENSE.txt");

    b.installDirectory(.{
        .source_dir = headers.getDirectory(),
        .install_dir = .header,
        .install_subdir = ".",
    });

    // ----------------------------------------------------
    // Module
    // ----------------------------------------------------

    const module = b.addModule("fimo_std", .{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
        .pic = true,
    });
    module.addImport("context_version", context_version);
    module.addImport("visualizers", visualizers);
    module.addIncludePath(headers.getDirectory());
    if (target.result.os.tag == .windows) module.linkSystemLibrary("advapi32", .{});

    // ----------------------------------------------------
    // Module tests
    // ----------------------------------------------------

    const module_tests = b.addTest(.{ .root_module = module });

    const test_step = b.step("test", "Run unit tests");

    const run_lib_unit_tests = b.addRunArtifact(module_tests);
    test_step.dependOn(&run_lib_unit_tests.step);

    var dir = b.build_root.handle.openDir("tests/", .{ .iterate = true }) catch |err| {
        std.debug.panic("unable to open '{}tests: {s}", .{
            b.build_root,
            @errorName(err),
        });
    };
    defer dir.close();

    var it = dir.iterateAssumeFirstIteration();
    while (it.next() catch @panic("failed to read dir")) |entry| {
        if (entry.kind != .file or !std.mem.endsWith(u8, entry.name, ".zig")) {
            continue;
        }

        const test_exe = b.addExecutable(.{
            .name = b.fmt("{s}_test", .{std.fs.path.stem(entry.name)}),
            .target = target,
            .optimize = optimize,
            .root_source_file = b.path("tests/").path(b, entry.name),
        });
        test_exe.root_module.addImport("fimo_std", module);

        const run_test_exe = b.addRunArtifact(test_exe);
        run_test_exe.expectExitCode(0);
        test_step.dependOn(&run_test_exe.step);
    }

    // ----------------------------------------------------
    // Static library
    // ----------------------------------------------------

    const static_lib = b.addStaticLibrary(.{
        .name = "fimo_std",
        .root_module = module,
    });
    static_lib.bundle_compiler_rt = true;
    if (target.result.os.tag == .windows) static_lib.dll_export_fns = true;

    if (b.option(bool, "build-static", "Build static library") orelse false) {
        installArtifact(b, libs, bins, static_lib);
        b.installArtifact(static_lib);
    }

    // ----------------------------------------------------
    // Dynamic library
    // ----------------------------------------------------

    const dynamic_lib = b.addSharedLibrary(.{
        .name = "fimo_std_shared",
        .root_module = module,
    });

    if (b.option(bool, "build-dynamic", "Build dynamic library") orelse false) {
        installArtifact(b, libs, bins, dynamic_lib);
        b.installArtifact(dynamic_lib);
    }

    // ----------------------------------------------------
    // Check
    // ----------------------------------------------------

    const static_lib_check = b.addStaticLibrary(.{
        .name = "fimo_std",
        .root_module = module,
    });

    const check = b.step("check", "Check if fimo_std compiles");
    check.dependOn(&static_lib_check.step);

    // ----------------------------------------------------
    // Documentation
    // ----------------------------------------------------

    _ = docs.addCopyDirectory(static_lib.getEmittedDocs(), ".", .{});
    const install_doc = b.addInstallDirectory(.{
        .source_dir = static_lib.getEmittedDocs(),
        .install_dir = .prefix,
        .install_subdir = "doc",
    });
    const doc_step = b.step("doc", "Generate documentation");
    doc_step.dependOn(&install_doc.step);
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

fn generateLicenseFile(
    b: *std.Build,
    wf: *std.Build.Step.WriteFile,
) void {
    const licensegen_exe = b.addExecutable(.{
        .name = "licensegen",
        .root_source_file = b.path("tools/licensegen.zig"),
        .target = b.graph.host,
        .optimize = .Debug,
    });

    const cmd = b.addRunArtifact(licensegen_exe);
    _ = wf.addCopyFile(cmd.addOutputFileArg("LICENSE.txt"), "LICENSE.txt");
    cmd.addPrefixedFileArg("-L", b.path("LICENSE-MIT"));
    cmd.addPrefixedFileArg("-L", b.path("LICENSE-APACHE"));
}

fn installArtifact(
    b: *std.Build,
    libs: *std.Build.Step.WriteFile,
    bins: *std.Build.Step.WriteFile,
    compile: *std.Build.Step.Compile,
) void {
    if (compile.isDynamicLibrary()) {
        _ = bins.addCopyFile(compile.getEmittedBin(), compile.out_filename);
        if (compile.producesImplib()) _ = libs.addCopyFile(
            compile.getEmittedImplib(),
            compile.out_lib_filename,
        );
        if (compile.producesPdbFile()) _ = bins.addCopyFile(
            compile.getEmittedPdb(),
            b.fmt("{s}.pdb", .{compile.name}),
        );
    } else if (compile.isStaticLibrary()) {
        _ = libs.addCopyFile(compile.getEmittedBin(), compile.out_filename);
        if (compile.producesPdbFile()) _ = libs.addCopyFile(
            compile.getEmittedPdb(),
            b.fmt("{s}.pdb", .{compile.name}),
        );
    }
}
