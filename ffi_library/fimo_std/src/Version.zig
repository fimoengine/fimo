//! A version specifier.

const std = @import("std");
const Version = @This();
const c = @import("c.zig");

major: u32,
minor: u32,
patch: u32,
build: u64 = 0,

/// Maximum number of characters required to represent a version without the build number.
pub const max_str_length = 32;

/// Maximum number of characters required to represent a version with the build number.
pub const max_long_str_length = 53;

/// Initializes the object from a ffi version.
pub fn initC(version: c.FimoVersion) Version {
    return Version{
        .major = version.major,
        .minor = version.minor,
        .patch = version.patch,
        .build = version.build,
    };
}

/// Casts the object to a ffi version.
pub fn intoC(self: Version) c.FimoVersion {
    return c.FimoVersion{
        .major = self.major,
        .minor = self.minor,
        .patch = self.patch,
        .build = self.build,
    };
}

/// Returns the order of two versions without considering the build number.
pub fn order(lhs: Version, rhs: Version) std.math.Order {
    if (lhs.major < rhs.major) return .lt;
    if (lhs.major > rhs.major) return .gt;
    if (lhs.minor < rhs.minor) return .lt;
    if (lhs.minor > rhs.minor) return .gt;
    if (lhs.patch < rhs.patch) return .lt;
    if (lhs.patch > rhs.patch) return .gt;
    return .eq;
}

/// Returns the order of two versions, also considering the build number.
pub fn orderLong(lhs: Version, rhs: Version) std.math.Order {
    const order_short = Version.order(lhs, rhs);
    if (order_short != .eq) return order_short;
    if (lhs.build < rhs.build) return .lt;
    if (lhs.build > rhs.build) return .gt;
    return .eq;
}

/// Checks for the compatibility of two versions.
///
/// If `got` is compatible with `required` it indicated that an object which is versioned with the
/// version `got` can be used instead of an object implementing the same interface carrying the
/// version `required`.
pub fn isCompatibleWith(got: Version, required: Version) bool {
    if (got.major != required.major) return false;
    if (got.major == 0 and got.minor != required.minor) return false;
    return got.order(required) != .lt;
}

/// Parses a version from a string.
pub fn parse(text: []const u8) !Version {
    // Parse the required major, minor, and patch numbers.
    const extra_index = std.mem.indexOfScalar(u8, text, '+');
    const required = text[0..(extra_index orelse text.len)];
    var it = std.mem.splitScalar(u8, required, '.');
    var ver = Version{
        .major = try parseNum(u32, it.first()),
        .minor = try parseNum(u32, it.next() orelse return error.InvalidVersion),
        .patch = try parseNum(u32, it.next() orelse return error.InvalidVersion),
    };
    if (it.next() != null) return error.InvalidVersion;
    if (extra_index == null) return ver;

    // Parse the build number
    const extra_idx = extra_index.?;
    if (extra_idx == text.len - 1) return error.InvalidVersion;
    const extra: []const u8 = text[extra_idx + 1 .. text.len];
    ver.build = try parseNum(usize, extra);
    return ver;
}

fn parseNum(T: type, text: []const u8) error{ InvalidVersion, Overflow }!T {
    // Leading zeroes are not allowed.
    if (text.len > 1 and text[0] == '0') return error.InvalidVersion;

    return std.fmt.parseUnsigned(T, text, 10) catch |err| switch (err) {
        error.InvalidCharacter => return error.InvalidVersion,
        error.Overflow => return error.Overflow,
    };
}

/// Formats the version.
///
/// # Format specifiers
///
/// * `{}`: Prints the version without the build number.
/// * `{long}`: Prints the version with the build number.
pub fn format(
    self: Version,
    comptime fmt: []const u8,
    options: std.fmt.FormatOptions,
    out_stream: anytype,
) !void {
    _ = options;
    const with_build = comptime parse_fmt: {
        if (fmt.len == 0) {
            break :parse_fmt false;
        } else if (std.mem.eql(u8, fmt, "long")) {
            break :parse_fmt true;
        } else {
            @compileError("expected {}, or {long}, found {" ++ fmt ++ "}");
        }
    };
    try std.fmt.format(out_stream, "{d}.{d}.{d}", .{ self.major, self.minor, self.patch });
    if (with_build and self.build > 0) try std.fmt.format(out_stream, "+{d}", .{self.build});
}

test format {
    // Valid version strings should be accepted.
    for ([_][]const u8{
        "0.0.4",
        "1.2.3",
        "10.20.30",
        "1.1.2+1",
        "1.0.0",
        "2.0.0",
        "1.1.7",
        "2.0.0+1848",
        "2.0.1+1227",
        "1.0.0+5",
        "1.2.3+788",
        "5.7.123",
    }) |valid| try std.testing.expectFmt(valid, "{long}", .{try parse(valid)});

    // Invalid version strings should be rejected.
    for ([_][]const u8{
        "",
        "1",
        "1.2",
        "1.2.3-0123",
        "1.2.3-0123.0123",
        "1.1.2+.123",
        "+invalid",
        "-invalid",
        "-invalid+invalid",
        "-invalid.01",
        "alpha",
        "alpha.beta",
        "alpha.beta.1",
        "alpha.1",
        "alpha+beta",
        "alpha_beta",
        "alpha.",
        "alpha..",
        "beta\\",
        "1.0.0-alpha_beta",
        "-alpha.",
        "1.0.0-alpha..",
        "1.0.0-alpha..1",
        "1.0.0-alpha...1",
        "1.0.0-alpha....1",
        "1.0.0-alpha.....1",
        "1.0.0-alpha......1",
        "1.0.0-alpha.......1",
        "01.1.1",
        "1.01.1",
        "1.1.01",
        "1.2",
        "1.2.3.DEV",
        "1.2-SNAPSHOT",
        "1.2.31.2.3----RC-SNAPSHOT.12.09.1--..12+788",
        "1.2-RC-SNAPSHOT",
        "-1.0.3-gamma+b7718",
        "+justmeta",
        "9.8.7+meta+meta",
        "9.8.7-whatever+meta+meta",
        "2.6.32.11-svn21605",
        "2.11.2(0.329/5/3)",
        "2.13-DEVELOPMENT",
        "2.3-35",
        "1a.4",
        "3.b1.0",
        "1.4beta",
        "2.7.pre",
        "0..3",
        "8.008.",
        "01...",
        "55",
        "foobar",
        "",
        "-1",
        "+4",
        ".",
        "....3",
    }) |invalid| try std.testing.expectError(error.InvalidVersion, parse(invalid));

    // Valid version string that may overflow.
    const big_valid = "99999999999999999999999.999999999999999999.99999999999999999";
    if (parse(big_valid)) |ver| {
        try std.testing.expectFmt(big_valid, "{}", .{ver});
    } else |err| try std.testing.expect(err == error.Overflow);

    // Invalid version string that may overflow.
    const big_invalid = "99999999999999999999999.999999999999999999.99999999999999999----RC-SNAPSHOT.12.09.1--------------------------------..12";
    if (parse(big_invalid)) |ver| std.debug.panic("expected error, found {}", .{ver}) else |_| {}
}

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    const AnyError = @import("AnyError.zig");

    export fn fimo_version_parse_str(
        str: [*]const u8,
        str_len: usize,
        version: *c.FimoVersion,
    ) c.FimoResult {
        const text = str[0..str_len];
        if (Version.parse(text)) |v| {
            version.* = v.intoC();
            return AnyError.intoCResult(null);
        } else |err| return AnyError.initError(err).err;
    }

    export fn fimo_version_str_len(
        version: *const c.FimoVersion,
    ) usize {
        const v = Version.initC(version.*);
        var buffer: [max_str_length]u8 = undefined;
        const print = std.fmt.bufPrint(
            buffer[0..buffer.len],
            "{}",
            .{v},
        ) catch unreachable;
        return print.len;
    }

    export fn fimo_version_str_len_long(
        version: *const c.FimoVersion,
    ) usize {
        const v = Version.initC(version.*);
        var buffer: [max_long_str_length]u8 = undefined;
        const print = std.fmt.bufPrint(
            buffer[0..buffer.len],
            "{long}",
            .{v},
        ) catch unreachable;
        return print.len;
    }

    export fn fimo_version_write_str(
        version: *const c.FimoVersion,
        str: [*]u8,
        str_len: usize,
        written: ?*usize,
    ) c.FimoResult {
        const v = Version.initC(version.*);
        const buffer = str[0..str_len];
        if (std.fmt.bufPrint(buffer, "{}", .{v})) |b| {
            if (written) |w| w.* = b.len;
            if (b.len < buffer.len) buffer[b.len + 1] = '\x00';
            return AnyError.intoCResult(null);
        } else |err| return AnyError.initError(err).err;
    }

    export fn fimo_version_write_str_long(
        version: *const c.FimoVersion,
        str: [*]u8,
        str_len: usize,
        written: ?*usize,
    ) c.FimoResult {
        const v = Version.initC(version.*);
        const buffer = str[0..str_len];
        if (std.fmt.bufPrint(buffer, "{long}", .{v})) |b| {
            if (written) |w| w.* = b.len;
            if (b.len < buffer.len) buffer[b.len + 1] = '\x00';
            return AnyError.intoCResult(null);
        } else |err| return AnyError.initError(err).err;
    }

    export fn fimo_version_cmp(
        lhs: *const c.FimoVersion,
        rhs: *const c.FimoVersion,
    ) c_int {
        const l = Version.initC(lhs.*);
        const r = Version.initC(rhs.*);
        return switch (l.order(r)) {
            .lt => -1,
            .eq => 0,
            .gt => 1,
        };
    }

    export fn fimo_version_cmp_long(
        lhs: *const c.FimoVersion,
        rhs: *const c.FimoVersion,
    ) c_int {
        const l = Version.initC(lhs.*);
        const r = Version.initC(rhs.*);
        return switch (l.orderLong(r)) {
            .lt => -1,
            .eq => 0,
            .gt => 1,
        };
    }

    export fn fimo_version_compatible(
        got: *const c.FimoVersion,
        required: *const c.FimoVersion,
    ) bool {
        const g = Version.initC(got.*);
        const r = Version.initC(required.*);
        return g.isCompatibleWith(r);
    }
};

comptime {
    _ = ffi;
}
