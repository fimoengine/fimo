const std = @import("std");

const build_internals = @import("tools/build-internals");

pub fn configure(builder: *build_internals.FimoBuild) void {
    const b = builder.build;
    const fimo_std_pkg = builder.getPackage("fimo_std");
    const fimo_tasks_meta_pkg = builder.getPackage("fimo_tasks_meta");

    const context_dep = b.dependency(
        "context",
        .{
            .target = builder.graph.target,
            .optimize = builder.graph.optimize,
        },
    );
    const context_path = context_dep.path("src");
    const context = b.addLibrary(.{
        .linkage = .static,
        .name = "context",
        .root_module = b.createModule(.{
            .target = builder.graph.target,
            .optimize = builder.graph.optimize,
        }),
    });
    switch (context.rootModuleTarget().cpu.arch) {
        .aarch64 => switch (context.rootModuleTarget().os.tag) {
            .windows => {
                if (context.rootModuleTarget().abi == .msvc) {
                    context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_arm64_aapcs_pe_armasm.asm"));
                    context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_arm64_aapcs_pe_armasm.asm"));
                }
            },
            .macos, .ios, .watchos, .tvos, .visionos => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_arm64_aapcs_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_arm64_aapcs_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_arm64_aapcs_macho_gas.S"));
            },
            else => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_arm64_aapcs_elf_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_arm64_aapcs_elf_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_arm64_aapcs_elf_gas.S"));
            },
        },
        .x86_64 => switch (context.rootModuleTarget().os.tag) {
            .windows => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_x86_64_ms_pe_clang_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_x86_64_ms_pe_clang_gas.S"));
            },
            .macos, .ios, .watchos, .tvos, .visionos => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_x86_64_sysv_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_x86_64_sysv_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_x86_64_sysv_macho_gas.S"));
            },
            else => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_x86_64_sysv_elf_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_x86_64_sysv_elf_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_x86_64_sysv_elf_gas.S"));
            },
        },
        else => @panic("Invalid arch"),
    }

    const module = b.addModule("fimo_tasks", .{
        .root_source_file = b.path("src/root.zig"),
        .target = builder.graph.target,
        .optimize = builder.graph.optimize,
    });
    module.addImport("fimo_std", fimo_std_pkg.root_module);
    module.addImport("fimo_tasks_meta", fimo_tasks_meta_pkg.root_module);
    module.linkLibrary(context);

    const mod = builder.addModule(.{
        .name = "fimo_tasks",
        .root_module = module,
    });

    _ = mod.addTest(.{
        .name = "fimo_tasks_test",
        .step = .{
            .module = blk: {
                const t = b.createModule(.{
                    .root_source_file = b.path("src/root.zig"),
                    .target = builder.graph.target,
                    .optimize = builder.graph.optimize,
                    .valgrind = builder.graph.target.result.os.tag == .linux,
                });
                t.addImport("fimo_std", fimo_std_pkg.root_module);
                t.addImport("fimo_tasks_meta", fimo_tasks_meta_pkg.root_module);
                t.linkLibrary(context);

                break :blk t;
            },
        },
    });
}

pub fn build(b: *std.Build) void {
    _ = b;
}
