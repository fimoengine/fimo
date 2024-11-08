const std = @import("std");
const builtin = @import("builtin");

const default_target: std.Target.Query = switch (builtin.target.os.tag) {
    .windows => .{ .os_tag = .windows, .abi = .msvc },
    else => .{},
};

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{
        .default_target = default_target,
    });
    const optimize = b.standardOptimizeOption(.{});

    // Generate additional build files.
    const wf = b.addWriteFiles();
    const visualizers = generateGDBScripts(b, wf);
    generateLicenseFile(b, wf);

    // Install all resources.
    installResources(b, wf.getDirectory());

    const lib = b.addStaticLibrary(.{
        .name = "fimo_std",
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
        .pic = true,
    });
    lib.bundle_compiler_rt = true;
    configureFimoCSources(
        b,
        wf.getDirectory(),
        visualizers,
        lib,
    );
    b.installArtifact(lib);

    const dylib = b.addSharedLibrary(.{
        .name = "fimo_std_shared",
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
        .pic = true,
    });
    dylib.bundle_compiler_rt = true;
    configureFimoCSources(
        b,
        wf.getDirectory(),
        visualizers,
        dylib,
    );
    b.installArtifact(dylib);

    const lib_unit_tests = b.addTest(.{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    configureFimoCSources(
        b,
        wf.getDirectory(),
        visualizers,
        lib_unit_tests,
    );

    const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);

    const docs = b.addInstallDirectory(.{
        .source_dir = lib.getEmittedDocs(),
        .install_dir = .prefix,
        .install_subdir = "docs",
    });

    const docs_step = b.step("doc", "Generate docs");
    docs_step.dependOn(&docs.step);
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
    root_file_bytes.appendSlice("\tif (builtin.os.tag != .windows and !builtin.target.isDarwin()) {\n") catch unreachable;

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
    cmd.addPrefixedDirectoryArg("-ND", b.path("third_party"));
    cmd.addPrefixedDirectoryArg("-ND", b.path("third_party/btree"));
    cmd.addPrefixedDirectoryArg("-ND", b.path("third_party/hashmap"));
    cmd.addPrefixedDirectoryArg("-ND", b.path("third_party/tinycthread"));
}

fn installResources(
    b: *std.Build,
    config_path: std.Build.LazyPath,
) void {
    // Install the public headers.
    var dir = b.build_root.handle.openDir("include/fimo_std", .{
        .iterate = true,
        .access_sub_paths = false,
    }) catch |err| {
        std.debug.panic("unable to open '{}include/fimo_std' directory: {s}", .{
            b.build_root,
            @errorName(err),
        });
    };
    defer dir.close();

    var it = dir.iterateAssumeFirstIteration();
    while (it.next() catch @panic("failed to read dir")) |entry| {
        // Skip the private headers.
        if (std.mem.eql(u8, entry.name, "internal")) {
            continue;
        }

        switch (entry.kind) {
            .file => {
                const path = b.fmt("include/fimo_std/{s}", .{entry.name});
                b.installFile(path, path);
            },
            .directory => {
                b.installDirectory(.{
                    .source_dir = b.path(b.fmt("include/fimo_std/{s}", .{entry.name})),
                    .install_dir = .header,
                    .install_subdir = b.fmt("fimo_std/{s}", .{entry.name}),
                });
            },
            else => {},
        }
    }

    // Install the natvis files.
    b.installDirectory(.{
        .source_dir = b.path("visualizers/natvis"),
        .install_dir = .header,
        .install_subdir = "fimo_std/impl/natvis",
        .include_extensions = &.{".natvis"},
    });

    // Install the generated headers.
    // b.installDirectory(.{
    //     .source_dir = config_path.path(b, "include/fimo_std"),
    //     .install_dir = .header,
    //     .install_subdir = "fimo_std",
    // });

    // Install the generated license.
    b.getInstallStep().dependOn(
        &b.addInstallFile(
            config_path.path(b, "LICENSE.txt"),
            "include/fimo_std/LICENSE.txt",
        ).step,
    );
}

fn configureFimoCSources(
    b: *std.Build,
    config_path: std.Build.LazyPath,
    visualizers: *std.Build.Module,
    compile: *std.Build.Step.Compile,
) void {
    _ = config_path;
    const c_files = .{
        // Internal headers
        "src/internal/context.c",
        "src/internal/module.c",
        // Public headers
        "src/array_list.c",
        "src/context.c",
        "src/graph.c",
        "src/refcount.c",
    };

    var buffer: [15][]const u8 = undefined;
    var flags = std.ArrayListUnmanaged([]const u8).initBuffer(&buffer);
    flags.appendAssumeCapacity("-std=c17");
    flags.appendAssumeCapacity("-Wall");
    flags.appendAssumeCapacity("-Wextra");
    flags.appendAssumeCapacity("-pedantic");
    flags.appendAssumeCapacity("-Werror");
    flags.appendAssumeCapacity("-fexec-charset=UTF-8");
    flags.appendAssumeCapacity("-finput-charset=UTF-8");
    if (compile.rootModuleTarget().os.tag != .windows) {
        flags.appendAssumeCapacity("-pthread");
    }

    if (compile.isDynamicLibrary()) {
        flags.appendAssumeCapacity("-D FIMO_STD_BUILD_SHARED");
        flags.appendAssumeCapacity("-D FIMO_STD_EXPORT_SYMBOLS");
        if (compile.rootModuleTarget().os.tag != .windows) {
            flags.appendAssumeCapacity("-fPIC");
        }
    }

    const options = b.addOptions();
    options.addOption(bool, "export_dll", compile.isDynamicLibrary());
    compile.root_module.addImport("export_settings", options.createModule());
    compile.root_module.addImport("visualizers", visualizers);

    compile.linkLibC();
    if (compile.rootModuleTarget().os.tag == .windows) {
        compile.linkSystemLibrary("Pathcch");
        compile.dll_export_fns = true;
    }
    compile.addIncludePath(b.path("include/"));
    compile.addCSourceFiles(.{
        .files = &c_files,
        .flags = flags.items,
    });

    // Dependencies.
    if (compile.rootModuleTarget().isDarwin()) {
        compile.addIncludePath(b.path("third_party/tinycthread/include/"));
        compile.addCSourceFile(.{
            .file = b.path("third_party/tinycthread/source/tinycthread.c"),
            .flags = &.{
                "-pthread",
            },
        });
    }
    compile.addIncludePath(b.path("third_party/btree/include/"));
    compile.addCSourceFile(.{
        .file = b.path("third_party/btree/btree.c"),
        .flags = &.{
            "-std=c17",
        },
    });
    compile.addIncludePath(b.path("third_party/hashmap/include/"));
    compile.addCSourceFile(.{
        .file = b.path("third_party/hashmap/hashmap.c"),
        .flags = &.{
            "-std=c17",
        },
    });
}
