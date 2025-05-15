const std = @import("std");
const builtin = @import("builtin");

const build_internals = @import("tools/build-internals");

const meta_list: []const struct {
    name: [:0]const u8,
    dep_name: [:0]const u8,
    pub_name: []const u8,
} = &.{
    .{ .name = "fimo_std", .dep_name = "pkg/fimo_std", .pub_name = "pkg-std" },
    .{ .name = "fimo_tasks_meta", .dep_name = "pkg/fimo_tasks_meta", .pub_name = "pkg-tasks" },
    .{ .name = "fimo_worlds_meta", .dep_name = "pkg/fimo_worlds_meta", .pub_name = "pkg-worlds" },
};

const module_list: []const struct {
    name: [:0]const u8,
    dep_name: [:0]const u8,
    pub_name: []const u8,
} = &.{
    .{ .name = "fimo_tasks", .dep_name = "module/fimo_tasks", .pub_name = "mod-tasks" },
    .{ .name = "fimo_worlds", .dep_name = "module/fimo_worlds", .pub_name = "mod-worlds" },
};

pub fn build(b: *std.Build) void {
    const install_step = b.getInstallStep();
    const test_step = b.step("test", "Run tests");
    const check_step = b.step("check", "Check compilation");

    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const linkage = b.option(
        std.builtin.LinkMode,
        "link-mode",
        "Link mode of the modules (default: static)",
    ) orelse .static;

    const install_headers = b.option(bool, "headers", "Install all headers (default: no)") orelse false;
    const install_tests = b.option(bool, "tests", "Install all tests (default: no)") orelse false;

    const install_standalone = b.option(bool, "standalone-module", "Enable standalone binary (default: yes)") orelse true;
    const install_split = b.option(bool, "split-modules", "Enable split module binaries (default: no)") orelse false;

    const test_filter = b.option([]const u8, "test-filter", "Filter the test execution to one specific package or module (default: none)");

    const pkg_std = b.option(bool, "pkg-std", "Enable the fimo_std package (default: yes)") orelse true;
    const pkg_tasks = b.option(bool, "pkg-tasks", "Enable the fimo_tasks_meta package (default: yes)") orelse true;
    const pkg_worlds = b.option(bool, "pkg-worlds", "Enable the fimo_worlds_meta package (default: yes)") orelse true;

    const mod_tasks = b.option(bool, "module-tasks", "Enable the fimo_tasks module (default: yes)") orelse true;
    const mod_worlds = b.option(bool, "module-worlds", "Enable the fimo_worlds module (default: yes)") orelse true;

    const builder = FimoBuild.init(
        b,
        target,
        optimize,
        .{
            .fimo_std = pkg_std,
            .fimo_tasks_meta = pkg_tasks,
            .fimo_worlds_meta = pkg_worlds,
        },
        .{
            .fimo_tasks = mod_tasks,
            .fimo_worlds = mod_worlds,
        },
    );

    const test_pkg_names = if (test_filter) |filter| blk: {
        for (meta_list) |pkg| if (std.mem.eql(u8, filter, pkg.pub_name)) break :blk pkg.name;
        for (module_list) |mod| if (std.mem.eql(u8, filter, mod.pub_name)) break :blk mod.name;
        break :blk "_";
    } else null;

    for (builder.builder.graph.pkgs.values()) |pkg| {
        const test_dir = std.Build.Step.InstallArtifact.Options.Dir{
            .override = .{
                .custom = b.fmt("tests/pkgs/{s}", .{pkg.name}),
            },
        };
        for (pkg.tests.items) |t| {
            if (test_pkg_names == null or std.mem.eql(u8, test_pkg_names.?, pkg.name))
                test_step.dependOn(&t.getRunArtifact().step);
            if (install_tests) {
                install_step.dependOn(
                    &b.addInstallArtifact(t.getArtifact(), .{ .dest_dir = test_dir }).step,
                );
            }
        }

        if (install_headers) {
            if (pkg.addInstallHeadersEx(b)) |inst| install_step.dependOn(&inst.step);
        }

        const check_target = b.addLibrary(.{
            .linkage = .static,
            .name = b.fmt("pkg_{s}_check", .{pkg.name}),
            .root_module = pkg.root_module,
        });
        check_step.dependOn(&check_target.step);
    }

    for (builder.builder.graph.modules.values()) |mod| {
        const test_dir = std.Build.Step.InstallArtifact.Options.Dir{
            .override = .{
                .custom = b.fmt("tests/modules/{s}", .{mod.name}),
            },
        };
        for (mod.tests.items) |t| {
            if (test_pkg_names == null or std.mem.eql(u8, test_pkg_names.?, mod.name))
                test_step.dependOn(&t.getRunArtifact().step);
            if (install_tests) {
                install_step.dependOn(
                    &b.addInstallArtifact(t.getArtifact(), .{ .dest_dir = test_dir }).step,
                );
            }
        }

        const check_target = b.addLibrary(.{
            .linkage = .static,
            .name = b.fmt("module_{s}_check", .{mod.name}),
            .root_module = mod.getLinkModule(),
        });
        check_step.dependOn(&check_target.step);

        if (install_split) {
            switch (linkage) {
                .static => b.installArtifact(mod.getStaticLib()),
                .dynamic => installModule(b, mod),
            }
        }
    }

    const fimo_module = builder.builder.createModule(.{
        .name = "fimo",
        .module_deps = builder.builder.graph.modules.keys(),
    });

    const fimo_check = b.addLibrary(.{
        .linkage = .static,
        .name = "fimo_check",
        .root_module = fimo_module.getLinkModule(),
    });
    check_step.dependOn(&fimo_check.step);

    if (install_standalone) {
        switch (linkage) {
            .static => b.installArtifact(fimo_module.getStaticLib()),
            .dynamic => installModule(b, fimo_module),
        }
    }
}

fn installModule(b: *std.Build, module: *build_internals.FimoBuild.Module) void {
    const lib = module.getDynamicLib();
    b.getInstallStep().dependOn(&b.addInstallFileWithDir(
        lib.getEmittedBin(),
        .{ .custom = b.fmt("modules/{s}", .{module.name}) },
        "module.fimo_module",
    ).step);
    if (lib.producesPdbFile()) {
        b.getInstallStep().dependOn(&b.addInstallFileWithDir(
            lib.getEmittedBin(),
            .{ .custom = b.fmt("modules/{s}", .{module.name}) },
            b.fmt("{s}.pdb", .{lib.name}),
        ).step);
    }
}

pub const MetaSelect = blk: {
    var fields: []const std.builtin.Type.StructField = &.{};
    for (meta_list) |pkg| {
        fields = fields ++ [_]std.builtin.Type.StructField{.{
            .name = pkg.name,
            .type = bool,
            .default_value_ptr = @as(*const anyopaque, @ptrCast(&false)),
            .is_comptime = false,
            .alignment = @alignOf(bool),
        }};
    }
    break :blk @Type(.{
        .@"struct" = .{
            .layout = .auto,
            .fields = fields,
            .decls = &.{},
            .is_tuple = false,
        },
    });
};

pub const ModulesSelect = blk: {
    var fields: []const std.builtin.Type.StructField = &.{};
    for (module_list) |mod| {
        fields = fields ++ [_]std.builtin.Type.StructField{.{
            .name = mod.name,
            .type = bool,
            .default_value_ptr = @as(*const anyopaque, @ptrCast(&false)),
            .is_comptime = false,
            .alignment = @alignOf(bool),
        }};
    }
    break :blk @Type(.{
        .@"struct" = .{
            .layout = .auto,
            .fields = fields,
            .decls = &.{},
            .is_tuple = false,
        },
    });
};

// The Fimo build system.
pub const FimoBuild = struct {
    builder: *build_internals.FimoBuild,

    const Self = @This();

    pub fn init(
        b: *std.Build,
        target: std.Build.ResolvedTarget,
        optimize: std.builtin.OptimizeMode,
        meta_select: MetaSelect,
        module_select: ModulesSelect,
    ) Self {
        const build_internals_dep = b.dependencyFromBuildZig(build_internals, .{});
        const self = Self{
            .builder = .createRoot(.{
                .build = b,
                .build_dep = build_internals_dep,
                .target = target,
                .optimize = optimize,
            }),
        };

        inline for (meta_list) |pkg| {
            if (@field(meta_select, pkg.name))
                _ = self.builder.lazyDependency(pkg.dep_name) orelse unreachable;
        }

        inline for (module_list) |mod| {
            if (@field(module_select, mod.name))
                _ = self.builder.lazyDependency(mod.dep_name) orelse unreachable;
        }

        return self;
    }
};
