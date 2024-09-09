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

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Declare the library dependencies.
    const btree = b.addStaticLibrary(.{
        .name = "btree",
        .target = target,
        .optimize = optimize,
    });
    btree.linkLibC();
    btree.addIncludePath(b.path("third_party/btree/include/"));
    btree.addCSourceFile(.{
        .file = b.path("third_party/btree/btree.c"),
        .flags = &.{
            "-std=c17",
        },
    });
    b.installArtifact(btree);

    const hashmap = b.addStaticLibrary(.{
        .name = "hashmap",
        .target = target,
        .optimize = optimize,
    });
    hashmap.linkLibC();
    hashmap.addIncludePath(b.path("third_party/hashmap/include/"));
    hashmap.addCSourceFile(.{
        .file = b.path("third_party/hashmap/hashmap.c"),
        .flags = &.{
            "-std=c17",
        },
    });
    b.installArtifact(hashmap);

    const tinycthread = b.addStaticLibrary(.{
        .name = "tinycthread",
        .target = target,
        .optimize = optimize,
    });
    tinycthread.linkLibC();
    tinycthread.addIncludePath(b.path("third_party/tinycthread/include/"));
    tinycthread.addCSourceFile(.{
        .file = b.path("third_party/tinycthread/source/tinycthread.c"),
        .flags = &.{
            "-pthread",
        },
    });
    if (tinycthread.rootModuleTarget().isDarwin()) {
        b.installArtifact(tinycthread);
    }

    // Generate additional build files.
    const wf = b.addWriteFiles();
    generateGDBScripts(b, wf);
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
        allocator,
        b,
        wf.getDirectory(),
        lib,
        btree,
        hashmap,
        tinycthread,
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
        allocator,
        b,
        wf.getDirectory(),
        dylib,
        btree,
        hashmap,
        tinycthread,
    );
    b.installArtifact(dylib);

    const lib_unit_tests = b.addTest(.{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    configureFimoCSources(
        allocator,
        b,
        wf.getDirectory(),
        lib_unit_tests,
        btree,
        hashmap,
        tinycthread,
    );

    const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);
}

fn generateGDBScripts(
    b: *std.Build,
    wf: *std.Build.Step.WriteFile,
) void {
    const gdbscript_to_c_exe = b.addExecutable(.{
        .name = "gdbscript_to_c",
        .root_source_file = b.path("tools/gdbscript_to_c.zig"),
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

    var it = dir.iterateAssumeFirstIteration();
    while (it.next() catch @panic("failed to read dir")) |entry| {
        if (entry.kind != .file or !std.mem.endsWith(u8, entry.name, ".py")) {
            continue;
        }

        const out_basename = b.fmt("{s}.h", .{std.fs.path.stem(entry.name)});
        const out_path = b.fmt("include/fimo_std/impl/gdb_scripts/{s}", .{out_basename});
        const cmd = b.addRunArtifact(gdbscript_to_c_exe);

        _ = wf.addCopyFile(cmd.addOutputFileArg(out_basename), out_path);
        cmd.addFileArg(b.path(b.fmt("visualizers/gdb/{s}", .{entry.name})));
    }
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
    b.installDirectory(.{
        .source_dir = config_path.path(b, "include/fimo_std"),
        .install_dir = .header,
        .install_subdir = "fimo_std",
    });

    // Install the generated license.
    b.getInstallStep().dependOn(
        &b.addInstallFile(
            config_path.path(b, "LICENSE.txt"),
            "include/fimo_std/LICENSE.txt",
        ).step,
    );
}

fn configureFimoCSources(
    allocator: std.mem.Allocator,
    b: *std.Build,
    config_path: std.Build.LazyPath,
    compile: *std.Build.Step.Compile,
    btree: *std.Build.Step.Compile,
    hashmap: *std.Build.Step.Compile,
    tinycthread: *std.Build.Step.Compile,
) void {
    const c_files = .{
        // Internal headers
        "src/internal/context.c",
        "src/internal/module.c",
        "src/internal/tracing.c",
        // Public implementation headers
        "src/impl/tracing.c",
        // Public headers
        "src/array_list.c",
        "src/context.c",
        "src/error.c",
        "src/graph.c",
        "src/memory.c",
        "src/module.c",
        "src/path.c",
        "src/refcount.c",
        "src/time.c",
        "src/tracing.c",
        "src/version.c",
    };

    var flags = StringArrayList.init(allocator);
    defer flags.deinit();
    flags.append("-std=c17");
    flags.append("-Wall");
    flags.append("-Wextra");
    flags.append("-pedantic");
    flags.append("-Werror");
    flags.append("-fexec-charset=UTF-8");
    flags.append("-finput-charset=UTF-8");
    if (compile.rootModuleTarget().os.tag != .windows) {
        flags.append("-pthread");
    }

    if (compile.isDynamicLibrary()) {
        flags.append("-D FIMO_STD_BUILD_SHARED");
        flags.append("-D FIMO_STD_EXPORT_SYMBOLS");
        if (compile.rootModuleTarget().os.tag != .windows) {
            flags.append("-fPIC");
        }
    }

    compile.linkLibC();
    compile.linkLibrary(btree);
    compile.linkLibrary(hashmap);
    if (compile.rootModuleTarget().isDarwin()) {
        compile.linkLibrary(tinycthread);
        compile.addIncludePath(b.path("third_party/tinycthread/include/"));
    }
    if (compile.rootModuleTarget().os.tag == .windows) {
        compile.linkSystemLibrary("Pathcch");
    }
    compile.addIncludePath(b.path("third_party/btree/include/"));
    compile.addIncludePath(b.path("third_party/hashmap/include/"));
    compile.addIncludePath(b.path("include/"));
    compile.addIncludePath(config_path.path(b, "include"));
    compile.addCSourceFiles(.{
        .files = &c_files,
        .flags = flags.items(),
    });
}

const StringArrayList = struct {
    const Self = @This();
    flags: std.ArrayList([]const u8),

    pub fn init(allocator: std.mem.Allocator) Self {
        return Self{
            .flags = std.ArrayList([]const u8).init(allocator),
        };
    }

    pub fn deinit(self: *Self) void {
        for (self.items()) |flag| {
            self.flags.allocator.free(flag);
        }
        self.flags.deinit();
    }

    pub fn append(self: *Self, flag: []const u8) void {
        const duplicate = self.flags.allocator.dupe(u8, flag) catch @panic("OOM");
        self.flags.append(duplicate) catch @panic("OOM");
    }

    pub fn items(self: *const Self) []const []const u8 {
        return self.flags.items;
    }
};
