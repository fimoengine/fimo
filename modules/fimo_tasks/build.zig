const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // ----------------------------------------------------
    // fimo_std
    // ----------------------------------------------------

    const fimo_std_dep = b.dependency("fimo_std", .{
        .target = target,
        .optimize = optimize,
        .@"build-static" = true,
    });
    const fimo_std = fimo_std_dep.module("fimo_std");

    // ----------------------------------------------------
    // fimo_python_meta
    // ----------------------------------------------------

    const fimo_tasks_meta_dep = b.dependency(
        "fimo_tasks_meta",
        .{ .target = target, .optimize = optimize },
    );
    const fimo_tasks_meta = fimo_tasks_meta_dep.module("fimo_tasks_meta");

    // ----------------------------------------------------
    // Boost context
    // ----------------------------------------------------

    const context_dep = b.dependency(
        "context",
        .{ .target = target, .optimize = optimize },
    );
    const context_path = context_dep.path("src");
    const context = b.addLibrary(.{
        .linkage = .static,
        .name = "context",
        .root_module = b.createModule(.{
            .target = target,
            .optimize = optimize,
        }),
    });
    switch (context.rootModuleTarget().cpu.arch) {
        .aarch64 => switch (context.rootModuleTarget().os.tag) {
            .windows => {
                if (context.rootModuleTarget().abi == .msvc) {
                    context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_arm64_aapcs_pe_armasm.asm"));
                    context.root_module.addAssemblyFile(context_path.path(b, "asm/make_arm64_aapcs_pe_armasm.asm"));
                    context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_arm64_aapcs_pe_armasm.asm"));
                }
            },
            .macos => {
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
        .riscv64 => {
            context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_riscv64_sysv_elf_gas.S"));
            context.root_module.addAssemblyFile(context_path.path(b, "asm/make_riscv64_sysv_elf_gas.S"));
            context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_riscv64_sysv_elf_gas.S"));
        },
        .x86_64 => switch (context.rootModuleTarget().os.tag) {
            .windows => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_x86_64_ms_pe_clang_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_x86_64_ms_pe_clang_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_x86_64_ms_pe_clang_gas.S"));
            },
            .macos => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_i386_x86_64_sysv_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_x86_64_sysv_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_i386_x86_64_sysv_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_x86_64_sysv_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_x86_64_sysv_macho_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_i386_x86_64_sysv_macho_gas.S"));
            },
            else => {
                context.root_module.addAssemblyFile(context_path.path(b, "asm/jump_x86_64_sysv_elf_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/make_x86_64_sysv_elf_gas.S"));
                context.root_module.addAssemblyFile(context_path.path(b, "asm/ontop_x86_64_sysv_elf_gas.S"));
            },
        },
        else => @panic("Invalid arch"),
    }

    // ----------------------------------------------------
    // Module
    // ----------------------------------------------------

    const fimo_tasks = b.addModule("fimo_tasks", .{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    fimo_tasks.addImport("fimo_std", fimo_std);
    fimo_tasks.addImport("fimo_tasks_meta", fimo_tasks_meta);
    fimo_tasks.linkLibrary(context);

    const fimo_tasks_lib = b.addLibrary(.{
        .linkage = .dynamic,
        .name = "fimo_tasks",
        .root_module = fimo_tasks,
    });
    b.installArtifact(fimo_tasks_lib);

    const module = b.addWriteFiles();
    b.addNamedLazyPath("module", module.getDirectory());
    _ = module.addCopyFile(fimo_tasks_lib.getEmittedBin(), "module.fimo_module");
    if (fimo_tasks_lib.producesPdbFile()) {
        _ = module.addCopyFile(fimo_tasks_lib.getEmittedPdb(), "fimo_tasks.pdb");
    }

    // ----------------------------------------------------
    // Check
    // ----------------------------------------------------

    const check = b.step("check", "Check if fimo_tasks compiles");
    check.dependOn(&fimo_tasks_lib.step);

    // ----------------------------------------------------
    // Tests
    // ----------------------------------------------------

    const modules = b.option(std.Build.LazyPath, "modules", "Path to the modules for testing");
    _ = modules;

    const fimo_tasks_test = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/root.zig"),
            .target = target,
            .optimize = optimize,
            .sanitize_thread = target.result.os.tag != .windows,
        }),
    });
    fimo_tasks_test.root_module.addImport("fimo_std", fimo_std);
    fimo_tasks_test.root_module.addImport("fimo_tasks_meta", fimo_tasks_meta);
    fimo_tasks_test.linkLibrary(context);
    const run_lib_unit_tests = b.addRunArtifact(fimo_tasks_test);

    const tests = b.step("test", "Run tests");
    tests.dependOn(&run_lib_unit_tests.step);
}
