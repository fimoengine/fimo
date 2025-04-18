const std = @import("std");

pub fn main() !void {
    var arena_state = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena_state.deinit();
    const arena = arena_state.allocator();

    const args = try std.process.argsAlloc(arena);

    if (args.len <= 3) fatal("wrong number of arguments", .{});
    const output_file_path = args[1];

    var license_paths = std.ArrayList([]const u8).init(arena);
    defer license_paths.deinit();

    var notices_paths = std.ArrayList([]const u8).init(arena);
    defer {
        for (notices_paths.items) |path| {
            arena.free(path);
        }
        notices_paths.deinit();
    }

    for (args[2..]) |arg| {
        if (std.mem.startsWith(u8, arg, "-L")) {
            const path = arg[2..];
            if (arg.len == 0) {
                fatal("invalid license path `{s}`", .{path});
            }
            try license_paths.append(path);
        } else if (std.mem.startsWith(u8, arg, "-ND")) {
            const dir_path = arg[3..];
            if (arg.len == 0) {
                fatal("invalid license directory path `{s}`", .{dir_path});
            }

            var dir = std.fs.cwd().openDir(dir_path, .{
                .iterate = true,
                .access_sub_paths = false,
            }) catch |err| {
                std.debug.panic("unable to open '{s}' directory: {s}", .{
                    dir_path,
                    @errorName(err),
                });
            };
            defer dir.close();

            var it = dir.iterateAssumeFirstIteration();
            while (it.next() catch @panic("failed to read dir")) |entry| {
                if (entry.kind != .file or !std.mem.startsWith(u8, entry.name, "LICENSE")) {
                    continue;
                }

                const path = try std.fs.path.join(arena, &.{ dir_path, entry.name });
                errdefer arena.free(path);
                try notices_paths.append(path);
            }
        } else {
            fatal("invalid argument `{s}`", .{arg});
        }
    }

    if (license_paths.items.len == 0) {
        fatal("no license files specified", .{});
    }

    var file_contents = std.ArrayList(u8).init(arena);
    defer file_contents.deinit();

    try file_contents.appendSlice(
        \\LICENSE
        \\
        \\This project is licensed under the following licenses.
        \\
        \\---------------------------------------------------------
        \\
        \\
    );
    for (license_paths.items, 0..) |path, i| {
        var file = std.fs.cwd().openFile(path, .{}) catch |err| {
            fatal("unable to open file '{s}': {s}", .{ path, @errorName(err) });
        };

        if (i != 0) {
            try file_contents.appendSlice(
                \\
                \\---
                \\
                \\
            );
        }
        try file.reader().readAllArrayList(&file_contents, std.math.maxInt(usize));
    }
    try file_contents.appendSlice(
        \\
        \\---------------------------------------------------------
        \\
        \\
    );

    if (notices_paths.items.len != 0) {
        try file_contents.appendSlice(
            \\NOTICES
            \\
            \\This project incorporates material as listed below or described in the code.
            \\
            \\
        );
        for (notices_paths.items) |path| {
            const dirname = std.fs.path.dirname(path) orelse "";
            const project_name = std.fs.path.basename(dirname);
            var file = std.fs.cwd().openFile(path, .{}) catch |err| {
                fatal("unable to open file '{s}': {s}", .{ path, @errorName(err) });
            };

            try file_contents.appendSlice(
                \\---------------------------------------------------------
                \\
                \\
            );
            try file_contents.appendSlice(project_name);
            try file_contents.appendSlice(
                \\
                \\
                \\
            );
            try file.reader().readAllArrayList(&file_contents, std.math.maxInt(usize));
            try file_contents.appendSlice(
                \\
                \\
                \\---------------------------------------------------------
                \\
                \\
            );
        }
    }

    var output_file = std.fs.cwd().createFile(output_file_path, .{}) catch |err| {
        fatal("unable to open output '{s}': {s}", .{ output_file_path, @errorName(err) });
    };
    defer output_file.close();

    try output_file.writeAll(file_contents.items);
    return std.process.cleanExit();
}

fn fatal(comptime format: []const u8, args: anytype) noreturn {
    std.debug.print(format, args);
    std.process.exit(1);
}
