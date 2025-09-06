//! A version specifier.

const std = @import("std");

const utils = @import("utils.zig");
const Slice = utils.Slice;
const SliceConst = utils.SliceConst;

major: usize,
minor: usize,
patch: usize,
pre: ?[]const u8 = null,
build: ?[]const u8 = null,

const Version = @This();

/// Redeclaration of the C-API types.
pub const compat = struct {
    pub const Version = extern struct {
        major: usize,
        minor: usize,
        patch: usize,
        pre: SliceConst(u8) = .fromSlice(""),
        build: SliceConst(u8) = .fromSlice(""),
    };
};

/// Initializes the object from a semantic version.
pub fn initSemanticVersion(version: std.SemanticVersion) Version {
    return Version{
        .major = version.major,
        .minor = version.minor,
        .patch = version.patch,
        .pre = version.pre,
        .build = version.build,
    };
}

/// Casts the object to a semantic version.
pub fn intoSemanticVersion(self: Version) std.SemanticVersion {
    return std.SemanticVersion{
        .major = self.major,
        .minor = self.minor,
        .patch = self.patch,
        .pre = self.pre,
        .build = self.build,
    };
}

/// Initializes the object from a ffi version.
pub fn initC(version: compat.Version) Version {
    return Version{
        .major = version.major,
        .minor = version.minor,
        .patch = version.patch,
        .pre = version.pre.intoSlice(),
        .build = version.build.intoSlice(),
    };
}

/// Casts the object to a ffi version.
pub fn intoC(self: Version) compat.Version {
    return compat.Version{
        .major = self.major,
        .minor = self.minor,
        .patch = self.patch,
        .pre = .fromSlice(self.pre),
        .build = .fromSlice(self.build),
    };
}

/// Returns the order of two versions.
pub fn order(lhs: Version, rhs: Version) std.math.Order {
    const lhs_sem = lhs.intoSemanticVersion();
    const rhs_sem = rhs.intoSemanticVersion();
    return lhs_sem.order(rhs_sem);
}

/// Checks for the compatibility of two versions.
///
/// If `got` sattisfies `required` it indicates that an object which is versioned with the
/// version `got` can be used instead of an object of the same type carrying the version
/// `required`.
///
/// The compatibility of `got` with `required` is determined by the following algorithm:
///
/// 1. The major versions of `got` and `required` must be equal.
/// 2. If the major version is `0`, the minor versions must be equal.
/// 3. `got >= required`.
pub fn sattisfies(got: Version, required: Version) bool {
    if (got.major != required.major) return false;
    if (got.major == 0 and got.minor != required.minor) return false;
    const got_sem = got.intoSemanticVersion();
    const required_sem = required.intoSemanticVersion();
    return got_sem.order(required_sem) != .lt;
}

/// Parses a version from a string.
pub fn parse(text: []const u8) !Version {
    const sem = try std.SemanticVersion.parse(text);
    return initSemanticVersion(sem);
}

pub fn format(self: Version, w: *std.Io.Writer) std.Io.Writer.Error!void {
    try self.intoSemanticVersion().format(w);
}

test format {
    // Taken from the zig standard library.

    // Valid version strings should be accepted.
    for ([_][]const u8{
        "0.0.4",
        "1.2.3",
        "10.20.30",
        "1.1.2-prerelease+meta",
        "1.1.2+meta",
        "1.1.2+meta-valid",
        "1.0.0-alpha",
        "1.0.0-beta",
        "1.0.0-alpha.beta",
        "1.0.0-alpha.beta.1",
        "1.0.0-alpha.1",
        "1.0.0-alpha0.valid",
        "1.0.0-alpha.0valid",
        "1.0.0-alpha-a.b-c-somethinglong+build.1-aef.1-its-okay",
        "1.0.0-rc.1+build.1",
        "2.0.0-rc.1+build.123",
        "1.2.3-beta",
        "10.2.3-DEV-SNAPSHOT",
        "1.2.3-SNAPSHOT-123",
        "1.0.0",
        "2.0.0",
        "1.1.7",
        "2.0.0+build.1848",
        "2.0.1-alpha.1227",
        "1.0.0-alpha+beta",
        "1.2.3----RC-SNAPSHOT.12.9.1--.12+788",
        "1.2.3----R-S.12.9.1--.12+meta",
        "1.2.3----RC-SNAPSHOT.12.9.1--.12",
        "1.0.0+0.build.1-rc.10000aaa-kk-0.1",
        "5.4.0-1018-raspi",
        "5.7.123",
    }) |valid| try std.testing.expectFmt(valid, "{f}", .{try parse(valid)});

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
        try std.testing.expectFmt(big_valid, "{f}", .{ver});
    } else |err| try std.testing.expect(err == error.Overflow);

    // Invalid version string that may overflow.
    const big_invalid = "99999999999999999999999.999999999999999999.99999999999999999----RC-SNAPSHOT.12.09.1--------------------------------..12";
    if (parse(big_invalid)) |ver| std.debug.panic("expected error, found {f}", .{ver}) else |_| {}
}

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    const AnyError = @import("AnyError.zig");
    const AnyResult = AnyError.AnyResult;

    fn numDigits(num: usize) usize {
        const x = std.math.log10(num);
        if (std.math.pow(usize, 10, x) < num) return x + 1;
        return x;
    }

    export fn fstd_version_init_str(version: *compat.Version, version_str: SliceConst(u8)) AnyResult {
        const text = version_str.intoSliceOrEmpty();
        if (Version.parse(text)) |v| {
            version.* = v.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fstd_version_str_len(version: *const compat.Version) usize {
        const major_len: usize = numDigits(version.major);
        const minor_len: usize = numDigits(version.minor);
        const patch_len: usize = numDigits(version.patch);
        return major_len + minor_len + patch_len + 2;
    }

    export fn fstd_version_str_len_full(version: *const compat.Version) usize {
        const v = Version.initC(version.*);
        var len: usize = 0;
        len += numDigits(v.major);
        len += numDigits(v.minor);
        len += numDigits(v.patch);
        if (v.pre) |pre| {
            if (pre.len != 0) len += pre.len + 1;
        }
        if (v.build) |build| {
            if (build.len != 0) len += build.len + 1;
        }
        return len;
    }

    export fn fstd_version_write_str(version: *const compat.Version, dst: Slice(u8), written: ?*usize) AnyResult {
        var v = Version.initC(version.*);
        v.pre = null;
        v.build = null;
        const buffer = dst.intoSliceOrEmpty();
        if (std.fmt.bufPrint(buffer, "{f}", .{v})) |b| {
            if (written) |w| w.* = b.len;
            if (b.len < buffer.len) buffer[b.len + 1] = '\x00';
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fstd_version_write_full_str(version: *const compat.Version, dst: Slice(u8), written: ?*usize) AnyResult {
        const v = Version.initC(version.*);
        const buffer = dst.intoSliceOrEmpty();
        if (std.fmt.bufPrint(buffer, "{f}", .{v})) |b| {
            if (written) |w| w.* = b.len;
            if (b.len < buffer.len) buffer[b.len + 1] = '\x00';
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fstd_version_order(lhs: *const compat.Version, rhs: *const compat.Version) c_int {
        const l = Version.initC(lhs.*);
        const r = Version.initC(rhs.*);
        return switch (l.order(r)) {
            .lt => -1,
            .eq => 0,
            .gt => 1,
        };
    }

    export fn fstd_version_sattisfies(got: *const compat.Version, required: *const compat.Version) bool {
        const g = Version.initC(got.*);
        const r = Version.initC(required.*);
        return g.sattisfies(r);
    }
};

comptime {
    _ = ffi;
}
