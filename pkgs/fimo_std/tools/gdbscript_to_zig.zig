const std = @import("std");

pub fn main() !void {
    var arena_state = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena_state.deinit();
    const arena = arena_state.allocator();

    const args = try std.process.argsAlloc(arena);

    if (args.len < 2) fatal("wrong number of arguments", .{});
    const output_file_path = args[1];

    var file_contents = std.ArrayList(u8).init(arena);
    defer file_contents.deinit();

    try file_contents.appendSlice("comptime {\n");

    for (args[2..]) |input_file_path| {
        var input_file = std.fs.cwd().openFile(input_file_path, .{}) catch |err| {
            fatal("unable to open input '{s}': {s}", .{ input_file_path, @errorName(err) });
        };
        defer input_file.close();

        try file_contents.appendSlice("\t// ");
        try file_contents.appendSlice(input_file_path);
        try file_contents.appendSlice("\n");
        try file_contents.appendSlice("\tasm (\n");
        try file_contents.appendSlice("\t\t\\\\.pushsection \".debug_gdb_scripts\", \"MS\",@progbits,1\n");
        try file_contents.appendSlice("\t\t\\\\.byte 4\n");
        try file_contents.appendSlice("\t\t\\\\.ascii \"gdb.inlined-script\\n\"\n");

        var buf_reader = std.io.bufferedReader(input_file.reader());
        var in_stream = buf_reader.reader();
        var buf: [256]u8 = undefined;
        while (try in_stream.readUntilDelimiterOrEof(&buf, '\n')) |line| {
            var escaped_len: usize = 0;
            var escaped: [512]u8 = undefined;
            for (line) |c| {
                if (c == '\"') {
                    escaped[escaped_len] = '\\';
                    escaped_len += 1;
                }

                escaped[escaped_len] = c;
                escaped_len += 1;
            }

            try file_contents.appendSlice("\t\t\\\\.ascii \"");
            try file_contents.appendSlice(escaped[0..escaped_len]);
            try file_contents.appendSlice("\\n\"\n");
        }

        try file_contents.appendSlice("\t\t\\\\.byte 0\n");
        try file_contents.appendSlice("\t\t\\\\.popsection\n");
        try file_contents.appendSlice("\t);\n");
    }

    try file_contents.appendSlice("}\n");

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
