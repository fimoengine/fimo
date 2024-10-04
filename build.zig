const std = @import("std");
const builtin = @import("builtin");
const zcc = @import("compile_commands");

const default_target: std.Target.Query = switch (builtin.target.os.tag) {
    .windows => .{ .os_tag = .windows, .abi = .msvc },
    else => .{},
};

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{
        .default_target = default_target,
    });
    const optimize = b.standardOptimizeOption(.{});

    const test_step = b.step("test", "Run unit tests");
    const doc_step = b.step("doc", "Generate documentation");
    const ci_step = b.step("ci", "Run ci checks");
    var packages = std.StringArrayHashMap(*std.Build.Dependency).init(b.allocator);

    const modules_dir = b.pathJoin(&.{ b.install_path, "modules/" });
    defer b.allocator.free(modules_dir);

    const enable_bindings = b.option(
        bool,
        "bindings",
        "Enable bindings",
    ) orelse true;
    if (enable_bindings) {
        _ = add_package(
            b,
            "fimo_std",
            .{
                .target = target,
                .optimize = optimize,
            },
            test_step,
            doc_step,
            &packages,
        );
    }

    const enable_rust_bindings = b.option(
        bool,
        "rust_bindings",
        "Enable Rust bindings",
    ) orelse true;
    if (enable_rust_bindings) {
        _ = add_package2(
            b,
            "fimo_std_rs",
            .{
                .target = target,
                .optimize = optimize,
            },
            test_step,
            doc_step,
            ci_step,
            &packages,
        );
    }

    const enable_modules = b.option(
        bool,
        "modules",
        "Enable modules",
    ) orelse true;
    if (enable_modules) {
        _ = add_module(
            b,
            "fimo_python",
            .{
                .target = target,
                .optimize = optimize,
                .modules_dir = @as([]const u8, @constCast(modules_dir)),
            },
            test_step,
            doc_step,
            &packages,
        );
    }

    // Setup the `cdb` command to generate a `compile_commands.json` file.
    var targets = std.ArrayList(*std.Build.Step.Compile).init(b.allocator);
    var cc_deps = std.ArrayList(*std.Build.Step).init(b.allocator);
    for (packages.values()) |p| {
        extractDependencyCompileCommandsTargets(p, &cc_deps, &targets);
    }
    zcc.createStep(b, "cdb", targets.toOwnedSlice() catch @panic("OOM"));
    const zcc_step = b.top_level_steps.get("cdb") orelse unreachable;
    const cdb_step = zcc_step.step.dependencies.items[0];
    for (cc_deps.items) |cc_dep| {
        cdb_step.dependOn(cc_dep);
    }
}

fn add_module(
    b: *std.Build,
    name: []const u8,
    args: anytype,
    test_step: *std.Build.Step,
    doc_step: *std.Build.Step,
    packages: *std.StringArrayHashMap(*std.Build.Dependency),
) ?*std.Build.Dependency {
    if (packages.contains(name)) {
        @panic(b.fmt("package `{s}` added multiple times", .{name}));
    }

    // Expose an option to disable the inclusion of the package.
    if (!(b.option(
        bool,
        b.fmt("{s}", .{name}),
        b.fmt("Enable the `{s}` module", .{name}),
    ) orelse true)) {
        return null;
    }

    // Add the package.
    const dep = b.dependency(name, args);
    packages.put(name, dep) catch @panic("OOM");
    b.getUninstallStep().dependOn(dep.builder.getUninstallStep());

    // Wire up test and doc steps.
    if (dep.builder.top_level_steps.get("test")) |dep_test_step| {
        test_step.dependOn(&dep_test_step.step);
    }
    if (dep.builder.top_level_steps.get("doc")) |dep_doc_step| {
        doc_step.dependOn(&dep_doc_step.step);
    }

    // Install all artifacts.
    const module_path = b.pathJoin(&.{ dep.builder.install_path, "module" });
    defer b.allocator.free(module_path);
    const artifacts_path = b.path(std.fs.path.relative(
        b.allocator,
        b.build_root.path orelse ".",
        module_path,
    ) catch @panic("OOM"));
    const install_dir = b.fmt("modules/{s}", .{name});
    const install_step = b.addInstallDirectory(.{
        .source_dir = artifacts_path,
        .install_dir = .prefix,
        .install_subdir = install_dir,
    });
    install_step.step.dependOn(dep.builder.getInstallStep());
    b.getInstallStep().dependOn(&install_step.step);

    if (dep.builder.top_level_steps.get("doc")) |dep_doc_step| {
        const install_doc_step = b.addInstallDirectory(.{
            .source_dir = artifacts_path,
            .install_dir = .prefix,
            .install_subdir = install_dir,
        });
        install_doc_step.step.dependOn(&dep_doc_step.step);
        doc_step.dependOn(&install_doc_step.step);
    }

    return dep;
}

fn add_package(
    b: *std.Build,
    name: []const u8,
    args: anytype,
    test_step: *std.Build.Step,
    doc_step: *std.Build.Step,
    packages: *std.StringArrayHashMap(*std.Build.Dependency),
) ?*std.Build.Dependency {
    if (packages.contains(name)) {
        @panic(b.fmt("package `{s}` added multiple times", .{name}));
    }

    // Expose an option to disable the inclusion of the package.
    if (!(b.option(
        bool,
        b.fmt("{s}", .{name}),
        b.fmt("Enable the `{s}` package", .{name}),
    ) orelse true)) {
        return null;
    }

    // Add the package.
    const dep = b.dependency(name, args);
    packages.put(name, dep) catch @panic("OOM");
    b.getUninstallStep().dependOn(dep.builder.getUninstallStep());

    // Wire up test and doc steps.
    if (dep.builder.top_level_steps.get("test")) |dep_test_step| {
        test_step.dependOn(&dep_test_step.step);
    }
    if (dep.builder.top_level_steps.get("doc")) |dep_doc_step| {
        doc_step.dependOn(&dep_doc_step.step);
    }

    // Install all artifacts.
    const artifacts_path = b.path(std.fs.path.relative(
        b.allocator,
        b.build_root.path orelse ".",
        dep.builder.install_path,
    ) catch @panic("OOM"));
    const install_step = b.addInstallDirectory(.{
        .source_dir = artifacts_path,
        .install_dir = .prefix,
        .install_subdir = name,
    });
    install_step.step.dependOn(dep.builder.getInstallStep());
    b.getInstallStep().dependOn(&install_step.step);

    if (dep.builder.top_level_steps.get("doc")) |dep_doc_step| {
        const install_doc_step = b.addInstallDirectory(.{
            .source_dir = artifacts_path,
            .install_dir = .prefix,
            .install_subdir = name,
        });
        install_doc_step.step.dependOn(&dep_doc_step.step);
        doc_step.dependOn(&install_doc_step.step);
    }

    return dep;
}

fn add_package2(
    b: *std.Build,
    name: []const u8,
    args: anytype,
    test_step: *std.Build.Step,
    doc_step: *std.Build.Step,
    ci_step: *std.Build.Step,
    packages: *std.StringArrayHashMap(*std.Build.Dependency),
) ?*std.Build.Dependency {
    if (packages.contains(name)) {
        @panic(b.fmt("package `{s}` added multiple times", .{name}));
    }

    // Expose an option to disable the inclusion of the package.
    if (!(b.option(
        bool,
        b.fmt("{s}", .{name}),
        b.fmt("Enable the `{s}` package", .{name}),
    ) orelse true)) {
        return null;
    }

    // Add the package.
    const dep = b.dependency(name, args);
    packages.put(name, dep) catch @panic("OOM");
    b.getInstallStep().dependOn(dep.builder.getInstallStep());
    b.getUninstallStep().dependOn(dep.builder.getUninstallStep());

    // Wire up test and ci tests.
    if (dep.builder.top_level_steps.get("test")) |step| {
        test_step.dependOn(&step.step);
    }
    if (dep.builder.top_level_steps.get("ci")) |step| {
        ci_step.dependOn(&step.step);
    }

    if (dep.builder.named_lazy_paths.get("lib")) |path| {
        b.installDirectory(.{
            .source_dir = path,
            .install_dir = .lib,
            .install_subdir = ".",
        });
    }
    if (dep.builder.named_lazy_paths.get("bin")) |path| {
        b.installDirectory(.{
            .source_dir = path,
            .install_dir = .bin,
            .install_subdir = ".",
        });
    }
    if (dep.builder.named_lazy_paths.get("header")) |path| {
        b.installDirectory(.{
            .source_dir = path,
            .install_dir = .header,
            .install_subdir = ".",
        });
    }
    if (dep.builder.named_lazy_paths.get("doc")) |path| {
        doc_step.dependOn(&b.addInstallDirectory(.{
            .source_dir = path,
            .install_dir = .{ .custom = "doc" },
            .install_subdir = name,
        }).step);
    }
    if (dep.builder.named_lazy_paths.get("module")) |path| {
        b.installDirectory(.{
            .source_dir = path,
            .install_dir = .{ .custom = "modules" },
            .install_subdir = name,
        });
    }

    return dep;
}

fn extractDependencyCompileCommandsTargets(
    dependency: *std.Build.Dependency,
    steps: *std.ArrayList(*std.Build.Step),
    targets: *std.ArrayList(*std.Build.Step.Compile),
) void {
    for (dependency.builder.top_level_steps.values()) |dep_step| {
        for (dep_step.step.dependencies.items) |dep_step_dep| {
            var compile: *std.Build.Step.Compile = undefined;
            if (dep_step_dep.cast(std.Build.Step.InstallArtifact)) |x| {
                compile = x.artifact;
            } else if (dep_step_dep.cast(std.Build.Step.Compile)) |x| {
                compile = x;
            } else {
                continue;
            }

            if (std.mem.indexOfScalar(*std.Build.Step.Compile, targets.items, compile) == null) {
                targets.append(compile) catch @panic("OOM");
                extractModuleIncludesSteps(&compile.root_module, steps);
                extractModuleLinkObjectsSteps(&compile.root_module, steps);
            }
        }
    }
}

fn extractModuleIncludesSteps(
    module: *std.Build.Module,
    steps: *std.ArrayList(*std.Build.Step),
) void {
    for (module.include_dirs.items) |include_dir| {
        switch (include_dir) {
            .path => |v| {
                appendLazyPathStep(v, steps);
            },
            .path_system => |v| {
                appendLazyPathStep(v, steps);
            },
            .path_after => |v| {
                appendLazyPathStep(v, steps);
            },
            .framework_path => |v| {
                appendLazyPathStep(v, steps);
            },
            .framework_path_system => |v| {
                appendLazyPathStep(v, steps);
            },
            .other_step => |v| {
                steps.append(&v.step) catch @panic("OOM");
            },
            .config_header_step => |v| {
                steps.append(&v.step) catch @panic("OOM");
            },
        }
    }
}

fn extractModuleLinkObjectsSteps(
    module: *std.Build.Module,
    steps: *std.ArrayList(*std.Build.Step),
) void {
    for (module.link_objects.items) |link_object| {
        switch (link_object) {
            .static_path => |v| {
                appendLazyPathStep(v, steps);
            },
            .other_step => |v| {
                steps.append(&v.step) catch @panic("OOM");
            },
            .system_lib => {
                continue;
            },
            .assembly_file => |v| {
                appendLazyPathStep(v, steps);
            },
            .win32_resource_file => |v| {
                appendLazyPathStep(v.file, steps);
                for (v.include_paths) |inc| {
                    appendLazyPathStep(inc, steps);
                }
            },
            .c_source_file => |v| {
                appendLazyPathStep(v.file, steps);
            },
            .c_source_files => |v| {
                appendLazyPathStep(v.root, steps);
            },
        }
    }
}

fn appendLazyPathStep(path: std.Build.LazyPath, steps: *std.ArrayList(*std.Build.Step)) void {
    switch (path) {
        .generated => |v| {
            steps.append(v.file.step) catch @panic("OOM");
        },
        else => {},
    }
}
