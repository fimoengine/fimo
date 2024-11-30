const std = @import("std");
const zig_cargo = @import("zig_cargo");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const target_dir = b.addWriteFiles();
    const cargo_commands = zig_cargo.add_cargo_crate(
        b,
        target.result,
        optimize,
        b.option(std.Build.LazyPath, "manifest", "Manifest Path") orelse b.path("Cargo.toml"),
        target_dir.getDirectory(),
        "fimo_std",
        .{},
    );

    b.getInstallStep().dependOn(cargo_commands.build_step);

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(cargo_commands.test_step);

    const doc_step = b.step("doc", "Generate documentation");
    const install_doc = b.addInstallDirectory(.{
        .source_dir = cargo_commands.out_dir.path(b, "doc"),
        .install_dir = .prefix,
        .install_subdir = "doc",
    });
    install_doc.step.dependOn(cargo_commands.doc_step);
    doc_step.dependOn(&install_doc.step);

    const ci_step = b.step("ci", "Run ci checks");
    ci_step.dependOn(cargo_commands.clippy_step);

    const docs = b.addWriteFiles().addCopyDirectory(
        cargo_commands.out_dir.path(b, "doc"),
        ".",
        .{},
    );
    docs.generated.file.step.dependOn(cargo_commands.doc_step);
    b.addNamedLazyPath("doc", docs);
}
