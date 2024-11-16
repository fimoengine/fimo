const std = @import("std");
const Allocator = std.mem.Allocator;
const testing = std.testing;

const path = @import("../path.zig");

pub const TmpDirError = error{
    TmpDirNotFound,
} || Allocator.Error || std.posix.MakeDirError || path.PathError;

pub const TmpDirUnmanaged = struct {
    path: path.OwnedPathUnmanaged,

    pub fn init(allocator: Allocator, prefix: []const u8) TmpDirError!TmpDirUnmanaged {
        const keys = [_][]const u8{
            "TMPDIR",
            "TEMPDIR",
            "TMP",
            "TEMP",
            "USERPROFILE",
        };
        var tmp_dir: ?[]u8 = null;
        defer allocator.free(tmp_dir.?);
        for (keys) |key| {
            tmp_dir = std.process.getEnvVarOwned(
                allocator,
                key,
            ) catch |err| switch (err) {
                error.OutOfMemory => return error.OutOfMemory,
                error.EnvironmentVariableNotFound => continue,
                error.InvalidWtf8 => unreachable,
            };
            break;
        }
        if (tmp_dir == null) return error.TmpDirNotFound;

        var buf = path.PathBufferUnmanaged{};
        errdefer buf.deinit(allocator);
        try buf.pushString(allocator, tmp_dir.?);

        while (true) {
            var random_bytes: [8]u8 = undefined;
            std.crypto.random.bytes(&random_bytes);
            var suffix: [std.fs.base64_encoder.calcSize(8)]u8 = undefined;
            _ = std.fs.base64_encoder.encode(&suffix, &random_bytes);
            const sub_path = try std.fmt.allocPrint(
                allocator,
                "{s}{s}",
                .{ prefix, suffix },
            );
            defer allocator.free(sub_path);
            try buf.pushString(allocator, sub_path);

            std.posix.mkdir(buf.asPath().raw, std.fs.Dir.default_mode) catch |err|
                switch (err) {
                error.PathAlreadyExists => {
                    _ = buf.pop();
                },
                else => return err,
            };
            break;
        }

        const p = try buf.toOwnedPath(allocator);
        return TmpDirUnmanaged{ .path = p };
    }

    pub fn deinit(self: *TmpDirUnmanaged, allocator: Allocator) void {
        std.fs.deleteDirAbsolute(self.path.raw) catch |err| @panic(@errorName(err));
        self.path.deinit(allocator);
    }
};

test "create tmp dir" {
    var dir: ?TmpDirUnmanaged = try TmpDirUnmanaged.init(testing.allocator, "test_");
    errdefer if (dir != null) dir.?.deinit(testing.allocator);
    var d = dir.?;
    const p = try testing.allocator.dupe(u8, d.path.raw);
    defer testing.allocator.free(p);

    var std_dir = try std.fs.openDirAbsolute(p, .{});
    std_dir.close();

    d.deinit(testing.allocator);
    dir = null;

    try testing.expectError(
        std.fs.Dir.OpenError.FileNotFound,
        std.fs.openDirAbsolute(p, .{}),
    );
}
