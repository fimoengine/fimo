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

    const modules = b.addWriteFiles();
    b.installDirectory(.{
        .source_dir = modules.getDirectory(),
        .install_dir = .{ .custom = "modules" },
        .install_subdir = ".",
    });

    const test_step = b.step("test", "Run unit tests");
    const doc_step = b.step("doc", "Generate documentation");
    const check_step = b.step("check", "Check if the project compiles");
    var packages = std.StringArrayHashMap(*std.Build.Dependency).init(b.allocator);

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
                .@"build-static" = true,
                .@"build-dynamic" = true,
            },
            modules,
            test_step,
            doc_step,
            check_step,
            &packages,
        );
        _ = add_package(
            b,
            "fimo_python_meta",
            .{ .target = target, .optimize = optimize },
            modules,
            test_step,
            doc_step,
            check_step,
            &packages,
        );
        _ = add_package(
            b,
            "fimo_tasks_meta",
            .{ .target = target, .optimize = optimize },
            modules,
            test_step,
            doc_step,
            check_step,
            &packages,
        );
    }

    const enable_rust_bindings = b.option(
        bool,
        "rust_bindings",
        "Enable Rust bindings",
    ) orelse true;
    if (enable_rust_bindings) {
        _ = add_package(
            b,
            "fimo_std_rs",
            .{
                .target = target,
                .optimize = optimize,
                .manifest = b.path("Cargo.toml"),
            },
            modules,
            test_step,
            doc_step,
            check_step,
            &packages,
        );
    }

    const enable_modules = b.option(
        bool,
        "modules",
        "Enable modules",
    ) orelse true;
    if (enable_modules) {
        _ = add_package(
            b,
            "fimo_python",
            .{
                .target = target,
                .optimize = optimize,
                .modules = modules.getDirectory(),
            },
            modules,
            test_step,
            doc_step,
            check_step,
            &packages,
        );
    }
}

fn add_package(
    b: *std.Build,
    name: []const u8,
    args: anytype,
    modules: *std.Build.Step.WriteFile,
    test_step: *std.Build.Step,
    doc_step: *std.Build.Step,
    check_step: *std.Build.Step,
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

    // Wire up test and ci tests.
    if (dep.builder.top_level_steps.get("test")) |step| {
        test_step.dependOn(&step.step);
    }
    if (dep.builder.top_level_steps.get("check")) |step| {
        check_step.dependOn(&step.step);
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
        _ = modules.addCopyDirectory(path, name, .{});
    }

    return dep;
}
