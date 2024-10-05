const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // We don't support cross compilation.
    if (!isSupported(target.result, b.host.result)) {
        return;
    }

    // ----------------------------------------------------
    // CPython
    // ----------------------------------------------------

    const cpython = buildCPython(b, target, optimize) orelse return;

    // ----------------------------------------------------
    // fimo_std
    // ----------------------------------------------------

    const fimo_std_dep = b.dependency("fimo_std", .{ .target = target, .optimize = optimize });
    const fimo_std = fimo_std_dep.artifact("fimo_std");

    // ----------------------------------------------------
    // fimo_python_meta
    // ----------------------------------------------------

    const fimo_python_meta_dep = b.dependency(
        "fimo_python_meta",
        .{ .target = target, .optimize = optimize },
    );

    // ----------------------------------------------------
    // fimo_python
    // ----------------------------------------------------

    const lib = b.addSharedLibrary(.{
        .name = "fimo_python",
        .target = target,
        .optimize = optimize,
        .root_source_file = b.path("src/root.zig"),
    });

    lib.addCSourceFiles(.{
        .files = &.{
            "main.c",
        },
        .flags = &.{
            "-DFIMO_CURRENT_MODULE_NAME=\"fimo_python\"",
        },
        .root = b.path("src/"),
    });

    lib.linkLibC();
    lib.linkLibrary(fimo_std);
    lib.addLibraryPath(cpython.binary_dir);
    lib.addLibraryPath(cpython.library_dir);
    if (target.result.os.tag != .windows) {
        if (optimize == .Debug) {
            lib.linkSystemLibrary("python3.13d");
        } else {
            lib.linkSystemLibrary("python3.13");
        }
    } else {
        lib.linkSystemLibrary("python313");
    }

    lib.addIncludePath(fimo_std_dep.path("include"));
    lib.addIncludePath(fimo_python_meta_dep.path("include"));
    lib.addIncludePath(cpython.include_dir);

    // ----------------------------------------------------
    // Module
    // ----------------------------------------------------

    b.getInstallStep().dependOn(&b.addInstallFile(
        lib.getEmittedBin(),
        "module/module.fimo_module",
    ).step);
    if (lib.producesPdbFile()) {
        b.getInstallStep().dependOn(&b.addInstallArtifact(lib, .{
            .dest_dir = .disabled,
            .pdb_dir = .{ .override = .{ .custom = "module" } },
            .h_dir = .disabled,
            .implib_dir = .disabled,
        }).step);
    }
    b.getInstallStep().dependOn(&b.addInstallDirectory(.{
        .source_dir = cpython.binary_dir,
        .install_dir = .{ .custom = "module" },
        .install_subdir = ".",
    }).step);

    // ----------------------------------------------------
    // Test
    // ----------------------------------------------------

    const test_options = b.addOptions();
    test_options.addOption(
        ?[]const u8,
        "modules_path",
        b.option(
            []const u8,
            "modules_dir",
            "Path to the built modules",
        ),
    );

    const test_module = b.addTest(.{
        .target = target,
        .optimize = optimize,
        .root_source_file = b.path("src/root.zig"),
    });
    test_module.linkLibrary(fimo_std);
    test_module.root_module.addImport("test_metadata", test_options.createModule());
    test_module.addIncludePath(fimo_std_dep.path("include"));
    test_module.addIncludePath(fimo_python_meta_dep.path("include"));
    test_module.addIncludePath(cpython.include_dir);

    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&test_module.step);
}

fn isSupported(
    target: std.Target,
    host: std.Target,
) bool {
    return switch (target.os.tag) {
        .windows => true,
        else => target.os.tag == host.os.tag and target.cpu.arch == host.cpu.arch,
    };
}

const CPythonBuild = struct {
    binary_dir: std.Build.LazyPath,
    include_dir: std.Build.LazyPath,
    library_dir: std.Build.LazyPath,
};

const CPythonBuildStep = union(enum) {
    compile: *std.Build.Step.Compile,
    path: std.Build.LazyPath,
};

fn buildCPython(
    b: *std.Build,
    target: std.Build.ResolvedTarget,
    optimize: std.builtin.OptimizeMode,
) ?CPythonBuild {
    if (target.result.os.tag == .windows) {
        return getCpythonWin(b, target, optimize);
    }

    if (b.lazyDependency("cpython_src", .{})) |cpython| {
        return buildCPythonUnix(b, target, optimize, cpython);
    }

    return null;
}

fn getCpythonWin(
    b: *std.Build,
    target: std.Build.ResolvedTarget,
    optimize: std.builtin.OptimizeMode,
) CPythonBuild {
    const python_arch = switch (target.result.cpu.arch) {
        .x86 => "x86",
        .x86_64 => "x64",
        .aarch64 => "arm64",
        else => @panic(@errorName(error.UnsupportedWinCpuArch)),
    };
    const python_name = b.fmt("python{s}.3.13.0-rc3.zip", .{python_arch});
    defer b.allocator.free(python_name);
    const python_path = b.pathResolve(&.{ b.build_root.path orelse ".", "ext", python_name });
    defer b.allocator.free(python_path);
    const python_file = std.fs.openFileAbsolute(python_path, .{}) catch |err| @panic(@errorName(err));
    defer python_file.close();

    const python_zip = b.addWriteFiles();
    extractZipBuffer(
        b,
        python_zip,
        python_file.seekableStream(),
        ".",
        .{},
    ) catch |err| @panic(@errorName(err));

    const python_bin = b.addWriteFiles();
    const python_include = b.addWriteFiles();
    const python_lib = b.addWriteFiles();

    _ = python_bin.addCopyDirectory(
        python_zip.getDirectory().join(b.allocator, "tools/DLLs") catch @panic("OOM"),
        "DLLs",
        .{},
    );
    _ = python_bin.addCopyDirectory(
        python_zip.getDirectory().join(b.allocator, "tools/DLLs") catch @panic("OOM"),
        "Lib",
        .{},
    );
    _ = python_bin.addCopyFile(
        python_zip.getDirectory().join(b.allocator, "tools/LICENSE.txt") catch @panic("OOM"),
        "CPYTHON_LICENSE.txt",
    );
    _ = python_bin.addCopyFile(
        python_zip.getDirectory().join(b.allocator, "tools/python3.dll") catch @panic("OOM"),
        "python3.dll",
    );
    _ = python_bin.addCopyFile(
        python_zip.getDirectory().join(b.allocator, "tools/python313.dll") catch @panic("OOM"),
        "python313.dll",
    );
    _ = python_bin.addCopyFile(
        python_zip.getDirectory().join(b.allocator, "tools/vcruntime140.dll") catch @panic("OOM"),
        "DLLs/vcruntime140.dll",
    );
    _ = python_bin.addCopyFile(
        python_zip.getDirectory().join(b.allocator, "tools/vcruntime140_1.dll") catch @panic("OOM"),
        "DLLs/vcruntime140_1.dll",
    );

    _ = python_include.addCopyDirectory(
        python_zip.getDirectory().join(b.allocator, "tools/include") catch @panic("OOM"),
        ".",
        .{},
    );

    _ = python_lib.addCopyDirectory(
        python_zip.getDirectory().join(b.allocator, "tools/libs") catch @panic("OOM"),
        ".",
        .{},
    );
    if (optimize == .Debug) {
        _ = python_lib.addCopyFile(
            python_zip.getDirectory().join(b.allocator, "tools/libs/python313.lib") catch @panic("OOM"),
            "python313_d.lib",
        );
    }

    return .{
        .binary_dir = python_bin.getDirectory(),
        .include_dir = python_include.getDirectory(),
        .library_dir = python_lib.getDirectory(),
    };
}

fn buildCPythonUnix(
    b: *std.Build,
    target: std.Build.ResolvedTarget,
    optimize: std.builtin.OptimizeMode,
    cpython: *std.Build.Dependency,
) CPythonBuild {
    const configure_path = cpython.builder.pathResolve(
        &.{ cpython.builder.build_root.path orelse ".", "configure" },
    );
    defer b.allocator.free(configure_path);

    const configure_dir = b.addWriteFiles();

    const configure_cpython = b.addSystemCommand(&.{configure_path});
    configure_cpython.setCwd(configure_dir.getDirectory());
    const install_dir = configure_cpython.addPrefixedOutputDirectoryArg(
        "--prefix=",
        "install",
    );
    configure_cpython.addArgs(&.{ "--enable-shared", "--without-static-libpython", "--disable-test-modules", "--with-ensurepip=no" });
    switch (optimize) {
        .Debug => {
            configure_cpython.addArg("--with-pydebug");
        },
        else => {
            configure_cpython.addArg("--enable-optimizations");
            configure_cpython.addArg("--with-lto");
        },
    }

    const build_cpython = b.addSystemCommand(&.{ "make", "install" });
    const cpu_count = std.Thread.getCpuCount() catch 1;
    const jobs_count = @max(cpu_count - 1, 1);
    build_cpython.addArg("-j");
    build_cpython.addArg(b.fmt("{}", .{jobs_count}));
    build_cpython.setCwd(configure_dir.getDirectory());
    build_cpython.step.dependOn(&configure_cpython.step);

    const python_bin = b.addWriteFiles();
    const python_include = b.addWriteFiles();
    const python_lib = b.addWriteFiles();

    python_bin.step.dependOn(&build_cpython.step);
    python_include.step.dependOn(&build_cpython.step);
    python_lib.step.dependOn(&build_cpython.step);

    _ = python_bin.addCopyDirectory(
        install_dir.path(b, "lib/python3.13"),
        "Lib",
        .{},
    );
    switch (optimize) {
        .Debug => {
            if (target.result.isDarwin()) {
                _ = python_bin.addCopyFile(
                    install_dir.path(b, "lib/libpython3.13d.dylib"),
                    "libpython3.13d.so",
                );
            } else {
                _ = python_bin.addCopyFile(
                    install_dir.path(b, "lib/libpython3.13d.so"),
                    "libpython3.13d.so",
                );
                _ = python_bin.addCopyFile(
                    install_dir.path(b, "lib/libpython3.13d.so.1.0"),
                    "libpython3.13d.so.1.0",
                );
            }
            _ = python_include.addCopyDirectory(
                install_dir.path(b, "include/python3.13d"),
                ".",
                .{},
            );
        },
        else => {
            if (target.result.isDarwin()) {
                _ = python_bin.addCopyFile(
                    install_dir.path(b, "lib/libpython3.13.dylib"),
                    "libpython3.13.so",
                );
            } else {
                _ = python_bin.addCopyFile(
                    install_dir.path(b, "lib/libpython3.13.so"),
                    "libpython3.13.so",
                );
                _ = python_bin.addCopyFile(
                    install_dir.path(b, "lib/libpython3.13.so.1.0"),
                    "libpython3.13.so.1.0",
                );
            }
            _ = python_include.addCopyDirectory(
                install_dir.path(b, "include/python3.13d"),
                ".",
                .{},
            );
        },
    }

    return .{
        .binary_dir = python_bin.getDirectory(),
        .include_dir = python_include.getDirectory(),
        .library_dir = python_lib.getDirectory(),
    };
}

fn extractZipBuffer(
    b: *std.Build,
    dest: *std.Build.Step.WriteFile,
    stream: anytype,
    sub_path: []const u8,
    options: std.zip.ExtractOptions,
) !void {
    var iter = try std.zip.Iterator(@TypeOf(stream)).init(stream);
    while (try iter.next()) |entry| {
        var filename_buf: [std.fs.max_path_bytes]u8 = undefined;
        const filename = filename_buf[0..entry.filename_len];

        try stream.seekTo(entry.header_zip_offset + @sizeOf(std.zip.CentralDirectoryFileHeader));
        {
            const len = try stream.context.reader().readAll(filename);
            if (len != filename.len)
                return error.ZipBadFileOffset;
        }

        const local_data_header_offset: u64 = local_data_header_offset: {
            const local_header = blk: {
                try stream.seekTo(entry.file_offset);
                break :blk try stream.context.reader().readStructEndian(std.zip.LocalFileHeader, .little);
            };
            if (!std.mem.eql(u8, &local_header.signature, &std.zip.local_file_header_sig))
                return error.ZipBadFileOffset;
            if (local_header.version_needed_to_extract != entry.version_needed_to_extract)
                return error.ZipMismatchVersionNeeded;
            if (local_header.last_modification_time != entry.last_modification_time)
                return error.ZipMismatchModTime;
            if (local_header.last_modification_date != entry.last_modification_date)
                return error.ZipMismatchModDate;

            if (@as(u16, @bitCast(local_header.flags)) != @as(u16, @bitCast(entry.flags)))
                return error.ZipMismatchFlags;
            if (local_header.crc32 != 0 and local_header.crc32 != entry.crc32)
                return error.ZipMismatchCrc32;
            if (local_header.compressed_size != 0 and
                local_header.compressed_size != entry.compressed_size)
                return error.ZipMismatchCompLen;
            if (local_header.uncompressed_size != 0 and
                local_header.uncompressed_size != entry.uncompressed_size)
                return error.ZipMismatchUncompLen;
            if (local_header.filename_len != entry.filename_len)
                return error.ZipMismatchFilenameLen;

            break :local_data_header_offset @as(u64, local_header.filename_len) +
                @as(u64, local_header.extra_len);
        };

        if (options.allow_backslashes) {
            std.mem.replaceScalar(u8, filename, '\\', '/');
        } else {
            if (std.mem.indexOfScalar(u8, filename, '\\')) |_|
                return error.ZipFilenameHasBackslash;
        }

        // All entries that end in '/' are directories
        if (filename[filename.len - 1] == '/') {
            if (entry.uncompressed_size != 0)
                return error.ZipBadDirectorySize;
            continue;
        }

        const out_buffer = try b.allocator.alloc(u8, entry.uncompressed_size);
        defer b.allocator.free(out_buffer);
        var out_buffer_stream = std.io.fixedBufferStream(out_buffer);

        const local_data_file_offset: u64 =
            @as(u64, entry.file_offset) +
            @as(u64, @sizeOf(std.zip.LocalFileHeader)) +
            local_data_header_offset;
        try stream.seekTo(local_data_file_offset);
        var limited_reader = std.io.limitedReader(stream.context.reader(), entry.compressed_size);
        const crc32 = try std.zip.decompress(
            entry.compression_method,
            entry.uncompressed_size,
            limited_reader.reader(),
            out_buffer_stream.writer(),
        );
        if (limited_reader.bytes_left != 0)
            return error.ZipDecompressTruncated;
        if (crc32 != entry.crc32)
            return error.ZipCrcMismatch;

        const dst_path = b.pathJoin(&.{ sub_path, filename });
        defer b.allocator.free(dst_path);
        _ = dest.add(dst_path, out_buffer);
    }
}
