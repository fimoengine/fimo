const std = @import("std");

pub fn build(b: *std.Build) void {
    _ = b;
}

pub const CargoSteps = struct {
    build_step: *std.Build.Step,
    test_step: *std.Build.Step,
    doc_step: *std.Build.Step,
    clippy_step: *std.Build.Step,
    out_dir: std.Build.LazyPath,
};

pub fn add_cargo_crate(
    b: *std.Build,
    target: std.Target,
    optimize: std.builtin.OptimizeMode,
    manifest_path_opt: ?std.Build.LazyPath,
    target_dir_opt: ?std.Build.LazyPath,
    package: ?[]const u8,
    env: anytype,
) CargoSteps {
    const triple = cargo_triplet(target);
    const manifest_path = manifest_path_opt orelse b.path("Cargo.toml");
    const target_dir = target_dir_opt orelse b.path("target");
    const cargo_build = cargo_command(
        b,
        "build",
        triple,
        optimize,
        manifest_path,
        target_dir,
        package,
        env,
    );
    const cargo_test = cargo_command(
        b,
        "test",
        triple,
        optimize,
        manifest_path,
        target_dir,
        package,
        env,
    );
    const cargo_doc = cargo_command(
        b,
        "doc",
        triple,
        optimize,
        manifest_path,
        target_dir,
        package,
        env,
    );
    const cargo_clippy = cargo_command(
        b,
        "clippy",
        triple,
        optimize,
        manifest_path,
        target_dir,
        package,
        env,
    );

    return .{
        .build_step = cargo_build,
        .test_step = cargo_test,
        .doc_step = cargo_doc,
        .clippy_step = cargo_clippy,
        .out_dir = target_dir.path(b, triple),
    };
}

fn cargo_command(
    b: *std.Build,
    comptime command_name: []const u8,
    triple: []const u8,
    optimize: std.builtin.OptimizeMode,
    manifest_path: std.Build.LazyPath,
    target_dir: std.Build.LazyPath,
    package: ?[]const u8,
    env: anytype,
) *std.Build.Step {
    const command = b.addSystemCommand(&.{ "cargo", command_name });
    command.addArg("--manifest-path");
    command.addFileArg(manifest_path);
    command.addArg("--target-dir");
    command.addDirectoryArg(target_dir);
    command.addArg("--target");
    command.addArg(triple);
    // _ = triple;
    if (b.verbose) {
        command.addArg("-vv");
    }
    if (optimize != .Debug) {
        command.addArg("--release");
    }
    if (package) |pkg| {
        command.addArg("--package");
        command.addArg(pkg);
    }
    if (@hasField(@TypeOf(env), command_name)) {
        const args = &@field(env, command_name);
        inline for (@typeInfo(@TypeOf(args.*)).@"struct".fields) |arg| {
            if (arg.type != []u8 and arg.type != []const u8) {
                @panic("unsupported field type `" ++ arg.type ++ "` for " ++ arg.name ++ " in command `" ++ command_name ++ "`");
            }
            command.setEnvironmentVariable(arg.name, @field(args.*, arg.name));
        }
    }

    return &command.step;
}

fn cargo_triplet(
    target: std.Target,
) []const u8 {
    return switch (target.cpu.arch) {
        .aarch64 => cargo_triplet_aarch64(target.os, target.abi),
        .aarch64_be => cargo_triplet_aarch64_be(target.os, target.abi),
        .arm => cargo_triplet_arm(target.os, target.abi),
        .x86 => cargo_triplet_x86(target.os, target.abi),
        .loongarch64 => cargo_triplet_loongarch64(target.os, target.abi),
        .x86_64 => cargo_triplet_x86_64(target.os, target.abi),
        else => @panic("Unsupported CPU arch"),
    };
}

fn cargo_triplet_aarch64(
    os: std.Target.Os,
    abi: std.Target.Abi,
) []const u8 {
    return switch (os.tag) {
        .macos => "aarch64-apple-darwin",
        .ios => switch (abi) {
            .none => "aarch64-apple-ios",
            .macabi => "aarch64-apple-ios-macabi",
            .simulator => "aarch64-apple-ios-sim",
            else => @panic("Unsupported ABI"),
        },
        .tvos => switch (abi) {
            .none => "aarch64-apple-tvos",
            .simulator => "aarch64-apple-tvos-sim",
            else => @panic("Unsupported ABI"),
        },
        .visionos => switch (abi) {
            .none => "aarch64-apple-visionos",
            .simulator => "aarch64-apple-visionos-sim",
            else => @panic("Unsupported ABI"),
        },
        .watchos => switch (abi) {
            .none => "aarch64-apple-watchos",
            .simulator => "aarch64-apple-watchos-sim",
            else => @panic("Unsupported ABI"),
        },
        .linux => switch (abi) {
            .android => "aarch64-linux-android",
            .gnu => "aarch64-unknown-linux-gnu",
            .gnuilp32 => "aarch64-unknown-linux-gnu_ilp32",
            .musl => "aarch64-unknown-linux-musl",
            .ohos => "aarch64-unknown-linux-ohos",
            else => @panic("Unsupported ABI"),
        },
        .windows => switch (abi) {
            .gnu => "aarch64-pc-windows-gnullvm",
            .msvc => "aarch64-pc-windows-msvc",
            else => @panic("Unsupported ABI"),
        },
        .freebsd => "aarch64-unknown-freebsd",
        .fuchsia => "aarch64-unknown-fuchsia",
        .hermit => "aarch64-unknown-hermit",
        .illumos => "aarch64-unknown-illumos",
        .netbsd => "aarch64-unknown-netbsd",
        .freestanding => switch (abi) {
            .none => "aarch64-unknown-none",
            else => @panic("Unsupported ABI"),
        },
        .openbsd => "aarch64-unknown-openbsd",
        .uefi => "aarch64-unknown-uefi",
        else => @panic("Unsupported OS"),
    };
}

fn cargo_triplet_aarch64_be(
    os: std.Target.Os,
    abi: std.Target.Abi,
) []const u8 {
    return switch (os.tag) {
        .linux => switch (abi) {
            .gnu => "aarch64_be-unknown-linux-gnu",
            .gnuilp32 => "aarch64_be-unknown-linux-gnu_ilp32",
            else => @panic("Unsupported ABI"),
        },
        .netbsd => "aarch64_be-unknown-netbsd",
        else => @panic("Unsupported OS"),
    };
}

fn cargo_triplet_arm(
    os: std.Target.Os,
    abi: std.Target.Abi,
) []const u8 {
    return switch (os.tag) {
        .linux => switch (abi) {
            .android => "arm-linux-androideabi",
            .gnueabi => "arm-unknown-linux-gnueabi",
            .gnueabihf => "arm-unknown-linux-gnueabihf",
            .musleabi => "arm-unknown-linux-musleabi",
            .musleabihf => "arm-unknown-linux-musleabihf",
            else => @panic("Unsupported ABI"),
        },
        else => @panic("Unsupported OS"),
    };
}

fn cargo_triplet_x86(
    os: std.Target.Os,
    abi: std.Target.Abi,
) []const u8 {
    return switch (os.tag) {
        .macos => "i686-apple-darwin",
        .ios => "i386-apple-ios",
        .hurd => switch (abi) {
            .gnu => "i686-unknown-hurd-gnu",
            else => @panic("Unsupported ABI"),
        },
        .linux => switch (abi) {
            .android => "i686-linux-android",
            .gnu => "i686-unknown-linux-gnu",
            .musl => "i686-unknown-linux-musl",
            else => @panic("Unsupported ABI"),
        },
        .windows => switch (abi) {
            .gnu => "i686-pc-windows-gnullvm",
            .msvc => "i686-pc-windows-msvc",
            else => @panic("Unsupported ABI"),
        },
        .freebsd => "i686-unknown-freebsd",
        .haiku => "i686-unknown-haiku",
        .netbsd => "i686-unknown-netbsd",
        .openbsd => "i686-unknown-openbsd",
        .uefi => "i686-unknown-uefi",
        else => @panic("Unsupported OS"),
    };
}

fn cargo_triplet_loongarch64(
    os: std.Target.Os,
    abi: std.Target.Abi,
) []const u8 {
    return switch (os.tag) {
        .linux => switch (abi) {
            .gnu => "loongarch64-unknown-linux-gnu",
            .musl => "loongarch64-unknown-linux-musl",
            .ohos => "loongarch64-unknown-linux-ohos",
            else => @panic("Unsupported ABI"),
        },
        .freestanding => switch (abi) {
            .none => "loongarch64-unknown-none",
            else => @panic("Unsupported ABI"),
        },
        else => @panic("Unsupported OS"),
    };
}

fn cargo_triplet_x86_64(
    os: std.Target.Os,
    abi: std.Target.Abi,
) []const u8 {
    return switch (os.tag) {
        .macos => "x86_64-apple-darwin",
        .ios => switch (abi) {
            .none => "x86_64-apple-ios",
            .macabi => "x86_64-apple-ios-macabi",
            else => @panic("Unsupported ABI"),
        },
        .tvos => "x86_64-apple-tvos",
        .watchos => switch (abi) {
            .simulator => "x86_64-apple-watchos-sim",
            else => @panic("Unsupported ABI"),
        },
        .linux => switch (abi) {
            .android => "x86_64-linux-android",
            .gnu => "x86_64-unknown-linux-gnu",
            .gnux32 => "x86_64-unknown-linux-gnux32",
            .musl => "x86_64-unknown-linux-musl",
            .none => "x86_64-unknown-linux-none",
            .ohos => "x86_64-unknown-linux-ohos",
            else => @panic("Unsupported ABI"),
        },
        .solaris => "x86_64-pc-solaris",
        .windows => switch (abi) {
            .gnu => "x86_64-pc-windows-gnullvm",
            .msvc => "x86_64-pc-windows-msvc",
            else => @panic("Unsupported ABI"),
        },
        .dragonfly => "x86_64-unknown-dragonfly",
        .freebsd => "x86_64-unknown-freebsd",
        .fuchsia => "x86_64-unknown-fuchsia",
        .haiku => "x86_64-unknown-haiku",
        .hermit => "x86_64-unknown-hermit",
        .hurd => switch (abi) {
            .gnu => "x86_64-unknown-hurd-gnu",
            else => @panic("Unsupported ABI"),
        },
        .illumos => "x86_64-unknown-illumos",
        .netbsd => "x86_64-unknown-netbsd",
        .freestanding => switch (abi) {
            .none => "x86_64-unknown-none",
            else => @panic("Unsupported ABI"),
        },
        .openbsd => "x86_64-unknown-openbsd",
        .uefi => "x86_64-unknown-uefi",
        else => @panic("Unsupported OS"),
    };
}
