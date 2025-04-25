const std = @import("std");
const testing = std.testing;
const unicode = std.unicode;
const Allocator = std.mem.Allocator;
const builtin = @import("builtin");

const c = @import("c");

const separator = if (builtin.os.tag == .windows) '\\' else '/';

// Derived from the Rust project, licensed as MIT and Apache License (Version 2.0).

fn isSeparator(ch: u8) bool {
    return switch (comptime builtin.os.tag) {
        .windows => ch == separator or ch == '/',
        else => ch == separator,
    };
}

fn isSeparatorVerbatim(ch: u8) bool {
    return ch == separator;
}

fn indexOfSeparator(raw: []const u8) ?usize {
    return switch (comptime builtin.os.tag) {
        .windows => std.mem.indexOfAny(u8, raw, [_]u8{separator} ++ "/"),
        else => std.mem.indexOfScalar(u8, raw, separator),
    };
}

fn indexOfSeparatorVerbatim(raw: []const u8) ?usize {
    return std.mem.indexOfScalar(u8, raw, separator);
}

pub const PathError = error{
    /// Expected an UTF-8 encoded string.
    InvalidUtf8,
};

/// A growable filesystem path encoded as UTF-8.
pub const PathBuffer = struct {
    buffer: PathBufferUnmanaged = .{},
    allocator: Allocator,

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoUTF8PathBuf) Self {
        const p = PathBufferUnmanaged.initC(obj);
        return p.toManaged(std.heap.c_allocator);
    }

    /// Casts the object to a ffi object.
    ///
    /// The memory must have been allocated with the c allocator.
    pub fn intoC(self: Self) c.FimoUTF8PathBuf {
        std.debug.assert(self.allocator.vtable == std.heap.c_allocator.vtable);
        return self.buffer.intoC();
    }

    /// Initialize an empty buffer.
    pub fn init(allocator: Allocator) Self {
        return Self{
            .allocator = allocator,
        };
    }

    /// Initialize the capacity to hold `capacity` bytes.
    pub fn initCapacity(allocator: Allocator, capacity: usize) Allocator.Error!Self {
        const buffer = try PathBufferUnmanaged.initCapacity(allocator, capacity);
        return buffer.toManaged(allocator);
    }

    /// Release all allocated memory.
    pub fn deinit(self: *Self) void {
        self.buffer.deinit(self.allocator);
    }

    /// Extracts a reference to the path.
    pub fn asPath(self: *const Self) Path {
        return self.buffer.asPath();
    }

    /// Clears the buffer and returns the old contents.
    pub fn toOwnedPath(self: *Self) Allocator.Error!OwnedPath {
        const path = try self.buffer.toOwnedPath(self.allocator);
        return path.toManaged(self.allocator);
    }

    /// Extends the path buffer with a path.
    ///
    /// If `path` is absolute, it replaces the current path.
    ///
    /// On Windows:
    ///
    /// - if `path` has a root but no prefix (e.g., `\windows`), it replaces everything except for
    ///   the prefix (if any) of `buf`.
    /// - if `path` has a prefix but no root, it replaces `buf`.
    /// - if `buf` has a verbatim prefix (e.g. `\\?\C:\windows`) and `path` is not empty, the new
    ///   path is normalized: all references to `.` and `..` are removed`.
    pub fn pushPath(self: *Self, path: Path) Allocator.Error!void {
        return self.buffer.pushPath(self.allocator, path);
    }

    /// Extends the path buffer with a UTF-8 string.
    ///
    /// Is equivalent `pushPath` after initializing a `Path` from the string.
    pub fn pushString(
        self: *Self,
        path: []const u8,
    ) (Allocator.Error || PathError)!void {
        return self.buffer.pushString(self.allocator, path);
    }

    test pushString {
        var buf = PathBuffer.init(testing.allocator);
        defer buf.deinit();
        try buf.pushString("/tmp");
        try buf.pushString("file.bk");
        try switch (builtin.os.tag) {
            .windows => testing.expectEqualStrings("/tmp\\file.bk", buf.asPath().raw),
            else => testing.expectEqualStrings("/tmp/file.bk", buf.asPath().raw),
        };

        var buf2 = PathBuffer.init(testing.allocator);
        defer buf2.deinit();
        try buf2.pushString("/tmp");
        try buf2.pushString("/etc");
        try testing.expectEqualStrings("/etc", buf2.asPath().raw);
    }

    /// Truncates the path buffer to its parent.
    ///
    /// Returns `false` and does nothing if there is no parent. Otherwise, returns `true`.
    pub fn pop(self: *Self) bool {
        return self.buffer.pop();
    }

    test pop {
        var buf = PathBuffer.init(testing.allocator);
        defer buf.deinit();
        try buf.pushString("/spirited/away.c");

        try testing.expect(buf.pop());
        try testing.expectEqualStrings("/spirited", buf.asPath().raw);

        try testing.expect(buf.pop());
        try testing.expectEqualStrings("/", buf.asPath().raw);
    }

    /// Formats the path.
    pub fn format(
        self: Self,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{}", .{self.buffer});
    }
};

/// A growable filesystem path encoded as UTF-8.
pub const PathBufferUnmanaged = struct {
    buffer: std.ArrayListUnmanaged(u8) = .{},

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoUTF8PathBuf) Self {
        return if (obj.buffer) |buffer|
            Self{
                .buffer = std.ArrayListUnmanaged(u8){
                    .items = buffer[0..obj.length],
                    .capacity = obj.capacity,
                },
            }
        else
            Self{};
    }

    /// Casts the object to a ffi object.
    ///
    /// The memory must have been allocated with the fimo allocator.
    pub fn intoC(self: Self) c.FimoUTF8PathBuf {
        return if (self.buffer.items.len == 0) c.FimoUTF8PathBuf{
            .buffer = null,
            .length = 0,
            .capacity = 0,
        } else c.FimoUTF8PathBuf{
            .buffer = self.buffer.items.ptr,
            .length = self.buffer.items.len,
            .capacity = self.buffer.capacity,
        };
    }

    /// Initialize the capacity to hold `capacity` bytes.
    pub fn initCapacity(allocator: Allocator, capacity: usize) Allocator.Error!Self {
        return Self{
            .buffer = try std.ArrayListUnmanaged(u8).initCapacity(
                allocator,
                capacity,
            ),
        };
    }

    /// Release all allocated memory.
    pub fn deinit(self: *Self, allocator: Allocator) void {
        self.buffer.deinit(allocator);
    }

    /// Extracts a reference to the path.
    pub fn asPath(self: *const Self) Path {
        return Path{ .raw = self.buffer.items };
    }

    /// Clears the buffer and returns the old contents.
    pub fn toOwnedPath(self: *Self, allocator: Allocator) Allocator.Error!OwnedPathUnmanaged {
        const slice = try self.buffer.toOwnedSlice(allocator);
        return OwnedPathUnmanaged{ .raw = slice };
    }

    /// Convert the buffer into an analogous memory-managed one.
    ///
    /// The returned buffer has ownership of the underlying memory.
    pub fn toManaged(self: Self, allocator: Allocator) PathBuffer {
        return PathBuffer{ .buffer = self, .allocator = allocator };
    }

    /// Extends the path buffer with a path.
    ///
    /// If `path` is absolute, it replaces the current path.
    ///
    /// On Windows:
    ///
    /// - if `path` has a root but no prefix (e.g., `\windows`), it replaces everything except for
    ///   the prefix (if any) of `buf`.
    /// - if `path` has a prefix but no root, it replaces `buf`.
    /// - if `buf` has a verbatim prefix (e.g. `\\?\C:\windows`) and `path` is not empty, the new
    ///   path is normalized: all references to `.` and `..` are removed`.
    pub fn pushPath(self: *Self, allocator: Allocator, path: Path) Allocator.Error!void {
        var need_sep = if (self.buffer.getLastOrNull()) |ch|
            !isSeparator(ch)
        else
            false;

        var it = self.asPath().iterator();
        if (it.prefixLen() > 0 and it.prefixLen() == it.rest.raw.len and it.prefix.?.isDrive()) {
            need_sep = false;
        }

        if (path.isAbsolute() or path.iterator().prefix != null)
            self.buffer.clearRetainingCapacity()
        else if (it.prefixIsVerbatim() and path.raw.len != 0) {
            var buffer = std.ArrayListUnmanaged(Path.Component){};
            defer buffer.deinit(allocator);
            while (it.next()) |comp| {
                try buffer.append(allocator, comp);
            }

            var p_it = path.iterator();
            while (p_it.next()) |comp| {
                switch (comp) {
                    .root_dir => {
                        buffer.shrinkRetainingCapacity(1);
                        try buffer.append(allocator, comp);
                    },
                    .cur_dir => {},
                    .parent_dir => {
                        if (buffer.getLastOrNull()) |last| {
                            if (last == .normal) _ = buffer.pop();
                        }
                    },
                    else => try buffer.append(allocator, comp),
                }
            }

            var res = std.ArrayListUnmanaged(u8){};
            errdefer res.deinit(allocator);
            var need_sep2 = false;
            for (buffer.items) |comp| {
                if (need_sep2 and comp != .root_dir) try res.append(allocator, separator);
                try res.appendSlice(allocator, comp.asPath().raw);
                need_sep2 = switch (comp) {
                    .root_dir => false,
                    .prefix => |pre| !pre.prefix.isDrive() and pre.prefix.len() > 0,
                    else => true,
                };
            }

            self.buffer.deinit(allocator);
            self.buffer = res;
            return;
        } else if (path.hasRoot()) {
            const prefix_length = self.asPath().iterator().prefixRemaining();
            self.buffer.shrinkRetainingCapacity(prefix_length);
        } else if (need_sep) {
            try self.buffer.append(allocator, separator);
        }

        try self.buffer.appendSlice(allocator, path.raw);
    }

    /// Extends the path buffer with a UTF-8 string.
    ///
    /// Is equivalent `pushPath` after initializing a `Path` from the string.
    pub fn pushString(
        self: *Self,
        allocator: Allocator,
        path: []const u8,
    ) (Allocator.Error || PathError)!void {
        const p = try Path.init(path);
        try self.pushPath(allocator, p);
    }

    /// Truncates the path buffer to its parent.
    ///
    /// Returns `false` and does nothing if there is no parent. Otherwise, returns `true`.
    pub fn pop(self: *Self) bool {
        if (self.asPath().parent()) |parent| {
            self.buffer.shrinkRetainingCapacity(parent.raw.len);
            return true;
        } else return false;
    }

    /// Formats the path.
    pub fn format(
        self: Self,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{s}", .{self.buffer.items});
    }
};

/// An owned filesystem path encoded as UTF-8.
pub const OwnedPath = struct {
    path: OwnedPathUnmanaged,
    allocator: Allocator,

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoOwnedUTF8Path) Self {
        const p = OwnedPathUnmanaged.initC(obj);
        return p.toManaged(std.heap.c_allocator);
    }

    /// Casts the object to a ffi object.
    ///
    /// The memory must have been allocated with the c allocator.
    pub fn intoC(self: Self) c.FimoOwnedUTF8Path {
        std.debug.assert(self.allocator.vtable == std.heap.c_allocator);
        return self.path.intoC();
    }

    /// Constructs a new owned path by copying a UTF-8 string.
    pub fn initString(
        allocator: Allocator,
        path: []const u8,
    ) (Allocator.Error || PathError)!Self {
        const p = try OwnedPathUnmanaged.initString(allocator, path);
        return p.toManaged(allocator);
    }

    test "invalid utf-8" {
        const str = "\xc3\x28";
        try testing.expectError(
            error.InvalidUtf8,
            OwnedPath.initString(testing.allocator, str),
        );
    }

    test initString {
        const path = "foo.txt";
        const owned = try OwnedPath.initString(testing.allocator, path);
        defer owned.deinit();
        try testing.expectEqualStrings("foo.txt", owned.asPath().raw);
    }

    /// Constructs a new owned path by copying the contents of another path.
    pub fn initPath(allocator: Allocator, path: Path) Allocator.Error!Self {
        const p = OwnedPathUnmanaged.initPath(allocator, path);
        return p.toManaged(allocator);
    }

    test initPath {
        const path = try Path.init("foo.txt");
        const owned = try OwnedPath.initPath(testing.allocator, path);
        defer owned.deinit();
        try testing.expectEqualStrings("foo.txt", owned.asPath().raw);
    }

    /// Constructs a new owned path from an os path.
    ///
    /// On Windows the path will re-encode the os path string from UTF-16 to UTF-8. No other
    /// conversions will be performed.
    pub fn initOsPath(
        allocator: Allocator,
        path: OsPath,
    ) (unicode.Utf16LeToUtf8AllocError || PathError)!Self {
        const p = OwnedPathUnmanaged.initOsPath(allocator, path);
        return p.toManaged(allocator);
    }

    test initOsPath {
        const path = &.{ 'f', 'o', 'o', '.', 't', 'x', 't', 0 };
        const os_path = OsPath{ .raw = path[0..7 :0] };
        const owned = try OwnedPath.initOsPath(testing.allocator, os_path);
        defer owned.deinit();
        try testing.expectEqualStrings("foo.txt", owned.asPath().raw);
    }

    /// Releases the memory associated with the path.
    pub fn deinit(self: Self) void {
        self.path.deinit(self.allocator);
    }

    /// Coerces the owned path to a path buffer.
    pub fn asPath(self: Self) Path {
        return self.path.asPath();
    }

    /// Coerces the owned path to a path buffer.
    pub fn toPathBuffer(self: Self) PathBuffer {
        const buffer = self.path.toPathBuffer();
        return buffer.toManaged(self.allocator);
    }

    /// Formats the path.
    pub fn format(
        self: Self,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{}", .{self.path});
    }
};

/// An owned filesystem path encoded as UTF-8.
pub const OwnedPathUnmanaged = struct {
    raw: []u8,

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoOwnedUTF8Path) Self {
        return Self{ .raw = obj.path[0..obj.length] };
    }

    /// Casts the object to a ffi object.
    ///
    /// The memory must have been allocated with the fimo allocator.
    pub fn intoC(self: Self) c.FimoOwnedUTF8Path {
        return c.FimoOwnedUTF8Path{ .path = self.raw.ptr, .length = self.raw.len };
    }

    /// Constructs a new owned path by copying a UTF-8 string.
    pub fn initString(
        allocator: Allocator,
        path: []const u8,
    ) (Allocator.Error || PathError)!Self {
        const p = try Path.init(path);
        return initPath(allocator, p);
    }

    /// Constructs a new owned path by copying the contents of another path.
    pub fn initPath(allocator: Allocator, path: Path) Allocator.Error!Self {
        const raw = try allocator.dupe(u8, path.raw);
        return Self{ .raw = raw };
    }

    /// Constructs a new owned path from an os path.
    ///
    /// On Windows the path will re-encode the os path string from UTF-16 to UTF-8. No other
    /// conversions will be performed.
    pub fn initOsPath(
        allocator: Allocator,
        path: OsPath,
    ) (unicode.Utf16LeToUtf8AllocError || PathError)!Self {
        switch (comptime builtin.os.tag) {
            .windows => {
                const p = try unicode.utf16LeToUtf8Alloc(allocator, path.raw);
                return Self{ .raw = p };
            },
            else => return initString(allocator, path.raw),
        }
    }

    /// Releases the memory associated with the path.
    pub fn deinit(self: Self, allocator: Allocator) void {
        allocator.free(self.raw);
    }

    /// Coerces the owned path to a path buffer.
    pub fn asPath(self: Self) Path {
        return Path{ .raw = self.raw };
    }

    /// Convert the path into an analogous memory-managed one.
    ///
    /// The returned path has ownership of the underlying memory.
    pub fn toManaged(self: Self, allocator: Allocator) OwnedPath {
        return OwnedPath{ .path = self, .allocator = allocator };
    }

    /// Coerces the owned path to a path buffer.
    pub fn toPathBuffer(self: Self) PathBufferUnmanaged {
        const buffer = std.ArrayListUnmanaged(u8).fromOwnedSlice(self.raw);
        return PathBufferUnmanaged{ .buffer = buffer };
    }

    /// Formats the path.
    pub fn format(
        self: Self,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{s}", .{self.raw});
    }
};

/// Character type of the native os filesystem path.
pub const OsPathChar = if (builtin.os.tag == .windows) u16 else u8;

/// An owned path that may be passed to the native os apis.
pub const OwnedOsPath = struct {
    path: OwnedOsPathUnmanaged,
    allocator: Allocator,

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoOwnedOSPath) Self {
        const p = OwnedOsPathUnmanaged.initC(obj);
        return p.toManaged(std.heap.c_allocator);
    }

    /// Casts the object to a ffi object.
    ///
    /// The memory must have been allocated with the c allocator.
    pub fn intoC(self: Self) c.FimoOwnedOSPath {
        std.debug.assert(self.allocator.vtable == std.heap.c_allocator);
        return self.path.intoC();
    }

    /// Initializes the os path from another os path.
    pub fn initOsPath(allocator: Allocator, path: OsPath) Allocator.Error!Self {
        const p = try OwnedOsPathUnmanaged.initOsPath(allocator, path);
        return p.toManaged(allocator);
    }

    /// Constructs a new owned os from a UTF-8 path.
    ///
    /// On Windows the path will be re-encoded to UTF-16.
    pub fn initPath(allocator: Allocator, path: Path) (PathError || Allocator.Error)!Self {
        const p = try OwnedOsPathUnmanaged.initPath(allocator, path);
        return p.toManaged(allocator);
    }

    /// Frees the memory associated with the os path.
    pub fn deinit(self: Self) void {
        self.path.deinit(self.allocator);
    }

    /// Extracts the os path from the owned os path.
    pub fn asOsPath(self: Self) OsPath {
        return self.path.asOsPath();
    }

    /// Formats the path.
    pub fn format(
        self: OwnedOsPath,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{}", .{self.path});
    }
};

/// An owned path that may be passed to the native os apis.
pub const OwnedOsPathUnmanaged = struct {
    raw: [:0]OsPathChar,

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoOwnedOSPath) Self {
        return Self{ .raw = obj.path[0..obj.length :0] };
    }

    /// Casts the object to a ffi object.
    ///
    /// The memory must have been allocated with the fimo allocator.
    pub fn intoC(self: Self) c.FimoOwnedOSPath {
        return c.FimoOwnedOSPath{ .path = self.raw.ptr, .length = self.raw.len };
    }

    /// Initializes the os path from another os path.
    pub fn initOsPath(allocator: Allocator, path: OsPath) Allocator.Error!Self {
        const raw = try allocator.dupeZ(OsPathChar, path.raw);
        Self{ .raw = raw };
    }

    /// Constructs a new owned os from a UTF-8 path.
    ///
    /// On Windows the path will be re-encoded to UTF-16.
    pub fn initPath(allocator: Allocator, path: Path) (PathError || Allocator.Error)!Self {
        switch (comptime builtin.os.tag) {
            .windows => {
                const raw = try unicode.utf8ToUtf16LeAllocZ(allocator, path.raw);
                return Self{ .raw = raw };
            },
            else => {
                const raw = try allocator.dupeZ(OsPathChar, path.raw);
                return Self{ .raw = raw };
            },
        }
    }

    /// Frees the memory associated with the os path.
    pub fn deinit(self: Self, allocator: Allocator) void {
        allocator.free(self.raw);
    }

    /// Extracts the os path from the owned os path.
    pub fn asOsPath(self: Self) OsPath {
        return OsPath{ .raw = self.raw };
    }

    /// Convert the path into an analogous memory-managed one.
    ///
    /// The returned path has ownership of the underlying memory.
    pub fn toManaged(self: Self, allocator: Allocator) OwnedOsPath {
        return OwnedOsPath{ .path = self, .allocator = allocator };
    }

    /// Formats the path.
    pub fn format(
        self: OwnedOsPathUnmanaged,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{s}", .{self.raw});
    }
};

/// A reference to a native filesystem path.
pub const OsPath = struct {
    raw: [:0]const OsPathChar,

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoOSPath) OsPath {
        return OsPath{ .raw = obj.path[0..obj.length :0] };
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: OsPath) c.FimoOSPath {
        return c.FimoOSPath{ .path = self.raw.ptr, .length = self.raw.len };
    }

    /// Formats the path.
    pub fn format(
        self: OsPath,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{s}", .{self.raw});
    }
};

/// A reference to a filesystem path encoded as UTF-8.
pub const Path = struct {
    /// A Windows path prefix.
    pub const Prefix = union(enum) {
        /// `\\?\prefix`
        verbatim: Path,
        /// `\\?\UNC\hostname\share_name`
        verbatim_unc: struct { hostname: Path, share_name: Path },
        /// `\\?\C:`
        verbatim_disk: u8,
        /// `\\.\NS`
        device_ns: Path,
        /// `\\hostname\share_name`
        unc: struct { hostname: Path, share_name: Path },
        /// `C:`
        disk: u8,

        /// Initializes the object from a ffi object.
        pub fn initC(obj: c.FimoUTF8PathPrefix) Prefix {
            return switch (obj.type) {
                c.FIMO_UTF8_PATH_PREFIX_VERBATIM => Prefix{
                    .verbatim = Path.initC(obj.data.verbatim),
                },
                c.FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC => Prefix{
                    .verbatim_unc = .{
                        .hostname = Path.initC(obj.data.verbatim_unc.hostname),
                        .share_name = Path.initC(obj.data.verbatim_unc.share_name),
                    },
                },
                c.FIMO_UTF8_PATH_PREFIX_VERBATIM_DISK => Prefix{
                    .verbatim_disk = obj.data.verbatim_disk,
                },
                c.FIMO_UTF8_PATH_PREFIX_DEVICE_NS => Prefix{
                    .device_ns = Path.initC(obj.data.device_ns),
                },
                c.FIMO_UTF8_PATH_PREFIX_UNC => Prefix{
                    .unc = .{
                        .hostname = Path.initC(obj.data.unc.hostname),
                        .share_name = Path.initC(obj.data.unc.share_name),
                    },
                },
                c.FIMO_UTF8_PATH_PREFIX_DISK => Prefix{ .disk = obj.data.disk },
                else => @panic("Unknown prefix variant"),
            };
        }

        /// Casts the object to a ffi object.
        pub fn intoC(self: Prefix) c.FimoUTF8PathPrefix {
            return switch (self) {
                .verbatim => |v| .{
                    .type = c.FIMO_UTF8_PATH_PREFIX_VERBATIM,
                    .data = .{
                        .verbatim = v.intoC(),
                    },
                },
                .verbatim_unc => |v| .{
                    .type = c.FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC,
                    .data = .{
                        .verbatim_unc = .{
                            .hostname = v.hostname.intoC(),
                            .share_name = v.share_name.intoC(),
                        },
                    },
                },
                .verbatim_disk => |v| .{
                    .type = c.FIMO_UTF8_PATH_PREFIX_VERBATIM_DISK,
                    .data = .{ .verbatim_disk = v },
                },
                .device_ns => |v| .{
                    .type = c.FIMO_UTF8_PATH_PREFIX_DEVICE_NS,
                    .data = .{
                        .device_ns = v.intoC(),
                    },
                },
                .unc => |v| .{
                    .type = c.FIMO_UTF8_PATH_PREFIX_UNC,
                    .data = .{
                        .unc = .{
                            .hostname = v.hostname.intoC(),
                            .share_name = v.share_name.intoC(),
                        },
                    },
                },
                .disk => |v| .{
                    .type = c.FIMO_UTF8_PATH_PREFIX_DISK,
                    .data = .{ .disk = v },
                },
            };
        }

        fn parse(path: []const u8) ?Prefix {
            // Only windows has prefixes.
            if (comptime builtin.os.tag != .windows) return null;

            // Verbatim prefix `\\?\...`
            if (std.mem.startsWith(u8, path, "\\\\?\\")) {
                const verbatim = path[4..];

                // UNC prefix `UNC\hostname\share_name`.
                if (std.mem.startsWith(u8, verbatim, "UNC\\")) {
                    const unc = verbatim[4..];

                    // Separate hostname and share name.
                    if (indexOfSeparatorVerbatim(unc)) |pos| {
                        const hostname = Path{ .raw = unc[0..pos] };
                        const rest = unc[pos + 1 ..];

                        const share_name_len = indexOfSeparatorVerbatim(rest) orelse rest.len;
                        const share_name = Path{ .raw = rest[0..share_name_len] };

                        return Prefix{
                            .verbatim_unc = .{ .hostname = hostname, .share_name = share_name },
                        };
                    } else {
                        return Prefix{
                            .verbatim_unc = .{
                                .hostname = Path{ .raw = unc },
                                .share_name = .{ .raw = &.{} },
                            },
                        };
                    }
                }

                // Drive prefix `C:`.
                if (verbatim.len > 1 and verbatim[1] == ':') {
                    return Prefix{ .verbatim_disk = verbatim[0] };
                }

                // Normal prefix.
                const prefix_len = indexOfSeparatorVerbatim(verbatim) orelse verbatim.len;
                return Prefix{ .verbatim = Path{ .raw = verbatim[0..prefix_len] } };
            }

            // Device NS `\\.\NS`.
            if (path.len > 3 and isSeparator(path[0]) and isSeparator(path[1]) and
                path[2] == '.' and isSeparator(path[3]))
            {
                const rest = path[4..];
                const device_ns = if (indexOfSeparator(rest)) |pos|
                    rest[0..pos]
                else
                    rest;
                return Prefix{ .device_ns = Path{ .raw = device_ns } };
            }

            // UNC `\\hostname\share_name`
            if (path.len > 1 and isSeparator(path[0]) and isSeparator(path[1])) {
                const unc = path[2..];

                // Separate hostname and share name.
                if (indexOfSeparator(unc)) |pos| {
                    const hostname = Path{ .raw = unc[0..pos] };
                    const rest = unc[pos + 1 ..];

                    const share_name_len = indexOfSeparator(rest) orelse rest.len;
                    const share_name = Path{ .raw = rest[0..share_name_len] };

                    return Prefix{
                        .unc = .{ .hostname = hostname, .share_name = share_name },
                    };
                } else {
                    return Prefix{
                        .unc = .{
                            .hostname = Path{ .raw = unc },
                            .share_name = .{ .raw = &.{} },
                        },
                    };
                }
            }

            // Disk `C:`.
            if (path.len > 1 and path[1] == ':') {
                return Prefix{ .disk = path[0] };
            }

            return null;
        }

        fn isVerbatim(self: *const Prefix) bool {
            return switch (self.*) {
                .verbatim, .verbatim_unc, .verbatim_disk => true,
                else => false,
            };
        }

        fn isDrive(self: *const Prefix) bool {
            return self.* == .disk;
        }

        fn hasImplicitRoot(self: *const Prefix) bool {
            return !isDrive(self);
        }

        fn len(self: *const Prefix) usize {
            return switch (self.*) {
                .verbatim => |v| v.raw.len + 4,
                .verbatim_unc => |v| if (v.share_name.raw.len == 0)
                    v.hostname.raw.len + 8
                else
                    v.hostname.raw.len + v.share_name.raw.len + 9,
                .verbatim_disk => 6,
                .device_ns => |v| v.raw.len + 4,
                .unc => |v| if (v.share_name.raw.len == 0)
                    v.hostname.raw.len + 2
                else
                    v.hostname.raw.len + v.share_name.raw.len + 3,
                .disk => 2,
            };
        }
    };

    /// Definition of all possible path components.
    pub const Component = union(enum) {
        prefix: struct { raw: Path, prefix: Prefix },
        root_dir: void,
        cur_dir: void,
        parent_dir: void,
        normal: Path,

        /// Initializes the object from a ffi object.
        pub fn initC(obj: c.FimoUTF8PathComponent) Component {
            return switch (obj.type) {
                c.FIMO_UTF8_PATH_COMPONENT_PREFIX => .{
                    .prefix = .{
                        .raw = Path.initC(obj.data.prefix.raw),
                        .prefix = Prefix.initC(obj.data.prefix.prefix),
                    },
                },
                c.FIMO_UTF8_PATH_COMPONENT_ROOT_DIR => .{ .root_dir = {} },
                c.FIMO_UTF8_PATH_COMPONENT_CUR_DIR => .{ .cur_dir = {} },
                c.FIMO_UTF8_PATH_COMPONENT_PARENT_DIR => .{ .parent_dir = {} },
                c.FIMO_UTF8_PATH_COMPONENT_NORMAL => .{ .normal = Path.initC(obj.data.normal) },
                else => @panic("Unknown component variant"),
            };
        }

        /// Casts the object to a ffi object.
        pub fn intoC(self: Component) c.FimoUTF8PathComponent {
            return switch (self) {
                .prefix => |v| .{
                    .type = c.FIMO_UTF8_PATH_COMPONENT_PREFIX,
                    .data = .{
                        .prefix = .{
                            .raw = v.raw.intoC(),
                            .prefix = v.prefix.intoC(),
                        },
                    },
                },
                .root_dir => .{
                    .type = c.FIMO_UTF8_PATH_COMPONENT_ROOT_DIR,
                    .data = .{ .root_dir = 0 },
                },
                .cur_dir => .{
                    .type = c.FIMO_UTF8_PATH_COMPONENT_CUR_DIR,
                    .data = .{ .cur_dir = 0 },
                },
                .parent_dir => .{
                    .type = c.FIMO_UTF8_PATH_COMPONENT_PARENT_DIR,
                    .data = .{ .parent_dir = 0 },
                },
                .normal => |v| .{
                    .type = c.FIMO_UTF8_PATH_COMPONENT_NORMAL,
                    .data = .{ .normal = v.intoC() },
                },
            };
        }

        /// Extracts the underlying path.
        pub fn asPath(self: *const Component) Path {
            return switch (self.*) {
                .prefix => |v| v.raw,
                .root_dir => Path{ .raw = &.{separator} },
                .cur_dir => Path{ .raw = "." },
                .parent_dir => Path{ .raw = ".." },
                .normal => |v| v,
            };
        }
    };

    pub const Iterator = struct {
        /// State of the iterator.
        pub const State = enum(u2) {
            prefix,
            start_dir,
            body,
            done,
        };

        rest: Path,
        prefix: ?Prefix,
        has_root_separator: bool,
        front_state: State = .prefix,
        back_state: State = .body,

        /// Initializes the object from a ffi object.
        pub fn initC(obj: c.FimoUTF8PathComponentIterator) Iterator {
            return Iterator{
                .rest = Path.initC(obj.current),
                .prefix = if (obj.has_prefix) Prefix.initC(obj.prefix) else null,
                .has_root_separator = obj.has_root_separator,
                .front_state = @enumFromInt(obj.front),
                .back_state = @enumFromInt(obj.back),
            };
        }

        /// Casts the object to a ffi object.
        pub fn intoC(self: Iterator) c.FimoUTF8PathComponentIterator {
            return c.FimoUTF8PathComponentIterator{
                .current = self.rest.intoC(),
                .has_prefix = self.prefix != null,
                .prefix = if (self.prefix) |p| p.intoC() else std.mem.zeroes(c.FimoUTF8PathPrefix),
                .has_root_separator = self.has_root_separator,
                .front = @intCast(@intFromEnum(self.front_state)),
                .back = @intCast(@intFromEnum(self.back_state)),
            };
        }

        fn hasRootSeparator(path: []const u8, prefix: ?*const Prefix) bool {
            if (prefix) |pre| {
                if (comptime builtin.target.os.tag != .windows) return false;
                const prefix_length = pre.len();
                if (path.len > prefix_length) {
                    return (pre.isVerbatim() and isSeparatorVerbatim(path[prefix_length])) or
                        isSeparator(path[prefix_length]);
                } else return false;
            } else return path.len > 0 and isSeparator(path[0]);
        }

        fn prefixLen(self: *const Iterator) usize {
            return if (self.prefix) |pre| pre.len() else 0;
        }

        fn prefixIsVerbatim(self: *const Iterator) bool {
            return if (self.prefix) |pre| pre.isVerbatim() else false;
        }

        fn prefixRemaining(self: *const Iterator) usize {
            return if (self.front_state == .prefix) self.prefixLen() else 0;
        }

        fn lenBeforeBody(self: *const Iterator) usize {
            const root: usize = if (@intFromEnum(self.front_state) <= @intFromEnum(State.start_dir) and
                self.has_root_separator) 1 else 0;
            const cur_dir: usize = if (@intFromEnum(self.front_state) <= @intFromEnum(State.start_dir) and
                self.includeCurrentDir()) 1 else 0;
            return self.prefixRemaining() + root + cur_dir;
        }

        fn isFinished(self: *const Iterator) bool {
            return self.front_state == .done or
                self.back_state == .done or
                @intFromEnum(self.front_state) > @intFromEnum(self.back_state);
        }

        fn checkSeparator(self: *const Iterator, ch: u8) bool {
            return if (self.prefixIsVerbatim()) isSeparatorVerbatim(ch) else isSeparator(ch);
        }

        fn hasRoot(self: *const Iterator) bool {
            return if (self.has_root_separator)
                true
            else if (self.prefix) |pre|
                pre.hasImplicitRoot()
            else
                false;
        }

        fn includeCurrentDir(self: *const Iterator) bool {
            if (self.hasRoot()) return false;
            const prefix_remaining = self.prefixRemaining();
            const rest = self.rest.raw[prefix_remaining..];
            if (rest.len == 1) return rest[0] == '.';
            if (rest.len > 1 and rest[0] == '.') return self.checkSeparator(rest[1]);
            return false;
        }

        fn parseSingleComponent(self: *const Iterator, path: []const u8) ?Component {
            if (path.len == 0) return null;
            if (std.mem.eql(u8, path, ".")) {
                return if (self.prefixIsVerbatim())
                    Component{ .cur_dir = {} }
                else
                    null;
            }
            if (std.mem.eql(u8, path, "..")) {
                return Component{ .parent_dir = {} };
            }
            return Component{ .normal = Path{ .raw = path } };
        }

        fn nextSeparatorPosFront(self: *const Iterator) ?usize {
            for (0.., self.rest.raw) |i, ch| {
                if (self.checkSeparator(ch)) return i;
            }
            return null;
        }

        fn nextSeparatorPosBack(self: *const Iterator) ?usize {
            const start = self.lenBeforeBody();
            const path = self.rest.raw[start..];
            var it = std.mem.reverseIterator(path);
            var i: usize = 0;
            while (it.next()) |ch| : (i += 1) {
                if (self.checkSeparator(ch)) return path.len - i - 1;
            }
            return null;
        }

        fn parseNextComponentFront(self: *const Iterator) struct {
            consumed_bytes: usize,
            component: ?Component,
        } {
            const sep = self.nextSeparatorPosFront();
            const extra: usize = if (sep != null) 1 else 0;
            const slice = if (sep) |pos| self.rest.raw[0..pos] else self.rest.raw;
            return .{
                .consumed_bytes = extra + slice.len,
                .component = self.parseSingleComponent(slice),
            };
        }

        fn parseNextComponentBack(self: *const Iterator) struct {
            consumed_bytes: usize,
            component: ?Component,
        } {
            const start = self.lenBeforeBody();
            const sep = self.nextSeparatorPosBack();
            const extra: usize = if (sep != null) 1 else 0;
            const slice = if (sep) |pos|
                self.rest.raw[start + pos + 1 ..]
            else
                self.rest.raw[start..];
            return .{
                .consumed_bytes = extra + slice.len,
                .component = self.parseSingleComponent(slice),
            };
        }

        fn trimLeft(self: *Iterator) void {
            while (self.rest.raw.len > 0) {
                const parsed = self.parseNextComponentFront();
                if (parsed.component != null) return;
                self.rest.raw = self.rest.raw[parsed.consumed_bytes..];
            }
        }

        fn trimRight(self: *Iterator) void {
            const beforeBody = self.lenBeforeBody();
            while (self.rest.raw.len > beforeBody) {
                const parsed = self.parseNextComponentBack();
                if (parsed.component != null) return;
                self.rest.raw = self.rest.raw[0 .. self.rest.raw.len - parsed.consumed_bytes];
            }
        }

        /// Extracts a path corresponding to the portion of the path remaining for iteration.
        pub fn asPath(self: *const Iterator) Path {
            var it = self.*;
            if (it.front_state == .body) it.trimLeft();
            if (it.back_state == .body) it.trimRight();
            return it.rest;
        }

        /// Performs an iteration step.
        ///
        /// Extracts the next component from the front of the iterator.
        pub fn next(self: *Iterator) ?Component {
            while (!self.isFinished()) {
                switch (self.front_state) {
                    .prefix => {
                        self.front_state = .start_dir;
                        if (self.prefix) |pre| {
                            const prefix_length = pre.len();
                            const raw = Path{ .raw = self.rest.raw[0..prefix_length] };
                            self.rest.raw = self.rest.raw[prefix_length..];
                            return Component{ .prefix = .{ .raw = raw, .prefix = pre } };
                        }
                    },
                    .start_dir => {
                        self.front_state = .body;
                        if (self.has_root_separator) {
                            std.debug.assert(self.rest.raw.len > 0);
                            self.rest.raw = self.rest.raw[1..];
                            return Component{ .root_dir = {} };
                        } else if (self.prefix) |pre| {
                            if (pre.hasImplicitRoot() and !pre.isVerbatim()) return Component{ .root_dir = {} };
                        } else if (self.includeCurrentDir()) {
                            std.debug.assert(self.rest.raw.len > 0);
                            self.rest.raw = self.rest.raw[1..];
                            return Component{ .cur_dir = {} };
                        }
                    },
                    .body => {
                        if (self.rest.raw.len > 0) {
                            const parsed = self.parseNextComponentFront();
                            self.rest.raw = self.rest.raw[parsed.consumed_bytes..];
                            if (parsed.component) |comp| return comp;
                            continue;
                        }
                        self.front_state = .done;
                    },
                    .done => unreachable,
                }
            }
            return null;
        }

        /// Performs an iteration step.
        ///
        /// Extracts the next component from the back of the iterator.
        pub fn nextBack(self: *Iterator) ?Component {
            while (!self.isFinished()) {
                switch (self.back_state) {
                    .body => {
                        if (self.rest.raw.len > self.lenBeforeBody()) {
                            const parsed = self.parseNextComponentBack();
                            self.rest.raw = self.rest.raw[0 .. self.rest.raw.len - parsed.consumed_bytes];
                            if (parsed.component) |comp| return comp;
                            continue;
                        }
                        self.back_state = .start_dir;
                    },
                    .start_dir => {
                        self.back_state = .prefix;
                        if (self.has_root_separator) {
                            self.rest.raw = self.rest.raw[0 .. self.rest.raw.len - 1];
                            return Component{ .root_dir = {} };
                        } else if (self.prefix) |pre| {
                            if (pre.hasImplicitRoot() and !pre.isVerbatim()) return Component{ .root_dir = {} };
                        } else if (self.includeCurrentDir()) {
                            self.rest.raw = self.rest.raw[0 .. self.rest.raw.len - 1];
                            return Component{ .cur_dir = {} };
                        }
                    },
                    .prefix => {
                        self.back_state = .done;
                        if (self.prefix) |pre| {
                            return Component{ .prefix = .{ .raw = self.rest, .prefix = pre } };
                        } else return null;
                    },
                    .done => unreachable,
                }
            }
            return null;
        }
    };

    raw: []const u8 = "",

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoUTF8Path) Path {
        return Path{ .raw = obj.path[0..obj.length] };
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: Path) c.FimoUTF8Path {
        return c.FimoUTF8Path{ .path = self.raw.ptr, .length = self.raw.len };
    }

    /// Initializes a new path, validating that it is valid UTF-8.
    pub fn init(path: []const u8) PathError!Path {
        if (!unicode.utf8ValidateSlice(path)) return error.InvalidUtf8;
        return Path{ .raw = path };
    }

    test "invalid utf-8 encoding" {
        const str = "\xc3\x28";
        try testing.expectError(error.InvalidUtf8, Path.init(str));
    }

    test init {
        const path = try Path.init("foo.txt");
        try testing.expectEqualStrings("foo.txt", path.raw);
    }

    /// Returns whether the path is absolute, i.e., if it is independent of the current directory.
    pub fn isAbsolute(self: Path) bool {
        return switch (comptime builtin.os.tag) {
            .windows => self.iterator().prefix != null,
            else => self.hasRoot(),
        };
    }

    test isAbsolute {
        const relative = try Path.init("foo");
        try std.testing.expect(!relative.isAbsolute());

        const absolute_str = switch (builtin.os.tag) {
            .windows => "c:\\windows",
            else => "/foo",
        };
        const absolute = try Path.init(absolute_str);
        try std.testing.expect(absolute.isAbsolute());
    }

    /// Returns whether the path is relative, i.e., if it is dependent of the current directory.
    pub fn isRelative(self: Path) bool {
        return !self.isAbsolute();
    }

    test isRelative {
        const relative = try Path.init("foo");
        try std.testing.expect(relative.isRelative());

        const absolute_str = switch (builtin.os.tag) {
            .windows => "c:\\windows",
            else => "/foo",
        };
        const absolute = try Path.init(absolute_str);
        try std.testing.expect(!absolute.isRelative());
    }

    /// Returns if the path has a root.
    pub fn hasRoot(self: Path) bool {
        return self.iterator().hasRoot();
    }

    test hasRoot {
        const p = try Path.init("foo");
        try std.testing.expect(!p.hasRoot());
    }

    test "no prefix with separator" {
        if (builtin.os.tag != .windows) return;
        const p = try Path.init("\\windows");
        try std.testing.expect(p.hasRoot());
    }

    test "prefix with separator" {
        if (builtin.os.tag != .windows) return;
        const p = try Path.init("c:\\windows");
        try std.testing.expect(p.hasRoot());
    }

    test "non-disk prefix" {
        if (builtin.os.tag != .windows) return;
        const p = try Path.init("\\\\server\\share");
        try std.testing.expect(p.hasRoot());
    }

    test "root path" {
        if (builtin.os.tag == .windows) return;
        const p = try Path.init("/foo");
        try std.testing.expect(p.hasRoot());
    }

    /// Returns the path without its final component, if there is one.
    pub fn parent(self: Path) ?Path {
        var iter = self.iterator();
        const comp = iter.nextBack() orelse return null;
        return switch (comp) {
            .normal, .cur_dir, .parent_dir => iter.asPath(),
            else => null,
        };
    }

    test parent {
        const path = try Path.init("/foo/bar");
        const par = path.parent().?;
        try std.testing.expectEqualStrings("/foo", par.raw);
        const gr_par = par.parent().?;
        try std.testing.expectEqualStrings("/", gr_par.raw);
        try std.testing.expect(gr_par.parent() == null);
    }

    test "parent relative path" {
        const path = try Path.init("foo/bar");
        const par = path.parent().?;
        try std.testing.expectEqualStrings("foo", par.raw);
        const gr_par = par.parent().?;
        try std.testing.expectEqualStrings("", gr_par.raw);
        try std.testing.expect(gr_par.parent() == null);
    }

    /// Returns the final component of the path, if there is one.
    pub fn fileName(self: Path) ?Path {
        var iter = self.iterator();
        const comp = iter.nextBack() orelse return null;
        return switch (comp) {
            .normal => |v| v,
            else => null,
        };
    }

    test fileName {
        const path = try Path.init("/usr/bin/");
        const file_name = path.fileName().?;
        try std.testing.expectEqualStrings("bin", file_name.raw);
    }

    test "filename file" {
        const path = try Path.init("tmp/foo.txt");
        const file_name = path.fileName().?;
        try std.testing.expectEqualStrings("foo.txt", file_name.raw);
    }

    test "filename file non-normalized" {
        const path = try Path.init("foo.txt/.");
        const file_name = path.fileName().?;
        try std.testing.expectEqualStrings("foo.txt", file_name.raw);
    }

    test "filename file non-normalized 2" {
        const path = try Path.init("foo.txt/.//");
        const file_name = path.fileName().?;
        try std.testing.expectEqualStrings("foo.txt", file_name.raw);
    }

    test "filename ends with '..'" {
        const path = try Path.init("foo.txt/..");
        try std.testing.expect(path.fileName() == null);
    }

    test "filename root" {
        const path = try Path.init("/");
        try std.testing.expect(path.fileName() == null);
    }

    /// Constructs an iterator over the components of a path.
    pub fn iterator(self: Path) Iterator {
        const prefix = Prefix.parse(self.raw);
        return Iterator{
            .rest = self,
            .prefix = prefix,
            .has_root_separator = Iterator.hasRootSeparator(
                self.raw,
                if (prefix) |p| &p else null,
            ),
        };
    }

    test iterator {
        const path = try Path.init("/tmp/foo.txt");
        var it = path.iterator();

        var component = it.next().?;
        try std.testing.expect(component == .root_dir);
        component = it.next().?;
        try std.testing.expectEqualStrings("tmp", component.normal.raw);
        component = it.next().?;
        try std.testing.expectEqualStrings("foo.txt", component.normal.raw);
        try std.testing.expect(it.next() == null);
    }

    test "iterator backwards" {
        const path = try Path.init("/tmp/foo.txt");
        var it = path.iterator();

        var component = it.nextBack().?;
        try std.testing.expectEqualStrings("foo.txt", component.normal.raw);
        component = it.nextBack().?;
        try std.testing.expectEqualStrings("tmp", component.normal.raw);
        component = it.nextBack().?;
        try std.testing.expect(component == .root_dir);
        try std.testing.expect(it.nextBack() == null);
    }

    /// Formats the path.
    pub fn format(
        self: Path,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        out_stream: anytype,
    ) !void {
        _ = options;
        _ = fmt;
        try std.fmt.format(out_stream, "{s}", .{self.raw});
    }
};

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    const AnyError = @import("AnyError.zig");
    const AnyResult = AnyError.AnyResult;

    export fn fimo_utf8_path_buf_new() c.FimoUTF8PathBuf {
        const p = PathBufferUnmanaged{};
        return p.intoC();
    }

    export fn fimo_utf8_path_buf_with_capacity(
        capacity: usize,
        buf: *c.FimoUTF8PathBuf,
    ) AnyResult {
        const p = PathBufferUnmanaged.initCapacity(
            std.heap.c_allocator,
            capacity,
        ) catch |err| return AnyError.initError(err).intoResult();
        buf.* = p.intoC();
        return AnyResult.ok;
    }

    export fn fimo_utf8_path_buf_free(buf: *c.FimoUTF8PathBuf) void {
        var p = PathBufferUnmanaged.initC(buf.*);
        p.deinit(std.heap.c_allocator);
    }

    export fn fimo_utf8_path_buf_as_path(buf: *const c.FimoUTF8PathBuf) c.FimoUTF8Path {
        const p = PathBufferUnmanaged.initC(buf.*);
        return p.asPath().intoC();
    }

    export fn fimo_utf8_path_buf_into_owned_path(
        buf: *c.FimoUTF8PathBuf,
        owned: *c.FimoOwnedUTF8Path,
    ) AnyResult {
        var p = PathBufferUnmanaged.initC(buf.*);
        const o = p.toOwnedPath(std.heap.c_allocator) catch |err| return AnyError.initError(err).intoResult();
        owned.* = o.intoC();
        return AnyResult.ok;
    }

    export fn fimo_utf8_path_buf_push_path(
        buf: *c.FimoUTF8PathBuf,
        path: c.FimoUTF8Path,
    ) AnyResult {
        var b = PathBufferUnmanaged.initC(buf.*);
        const p = Path.initC(path);
        b.pushPath(std.heap.c_allocator, p) catch |err| return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    export fn fimo_utf8_path_buf_push_string(
        buf: *c.FimoUTF8PathBuf,
        path: [*:0]const u8,
    ) AnyResult {
        var b = PathBufferUnmanaged.initC(buf.*);
        const p = std.mem.span(path);
        b.pushString(std.heap.c_allocator, p) catch |err| return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    export fn fimo_utf8_path_buf_pop(buf: *c.FimoUTF8PathBuf) bool {
        var b = PathBufferUnmanaged.initC(buf.*);
        return b.pop();
    }

    export fn fimo_owned_utf8_path_from_string(
        path: [*:0]const u8,
        owned: *c.FimoOwnedUTF8Path,
    ) AnyResult {
        const p = std.mem.span(path);
        const o = OwnedPathUnmanaged.initString(
            std.heap.c_allocator,
            p,
        ) catch |err| return AnyError.initError(err).intoResult();
        owned.* = o.intoC();
        return AnyResult.ok;
    }

    export fn fimo_owned_utf8_path_from_path(
        path: c.FimoUTF8Path,
        owned: *c.FimoOwnedUTF8Path,
    ) AnyResult {
        const p = Path.initC(path);
        const o = OwnedPathUnmanaged.initPath(
            std.heap.c_allocator,
            p,
        ) catch |err| return AnyError.initError(err).intoResult();
        owned.* = o.intoC();
        return AnyResult.ok;
    }

    export fn fimo_owned_utf8_path_from_os_path(
        path: c.FimoOSPath,
        owned: *c.FimoOwnedUTF8Path,
    ) AnyResult {
        const p = OsPath.initC(path);
        const o = OwnedPathUnmanaged.initOsPath(
            std.heap.c_allocator,
            p,
        ) catch |err| return AnyError.initError(err).intoResult();
        owned.* = o.intoC();
        return AnyResult.ok;
    }

    export fn fimo_owned_utf8_path_free(path: c.FimoOwnedUTF8Path) void {
        var o = OwnedPathUnmanaged.initC(path);
        o.deinit(std.heap.c_allocator);
    }

    export fn fimo_owned_utf8_path_as_path(path: c.FimoOwnedUTF8Path) c.FimoUTF8Path {
        const o = OwnedPathUnmanaged.initC(path);
        return o.asPath().intoC();
    }

    export fn fimo_owned_utf8_path_into_path_buf(path: c.FimoOwnedUTF8Path) c.FimoUTF8PathBuf {
        const o = OwnedPathUnmanaged.initC(path);
        const buf = o.toPathBuffer();
        return buf.intoC();
    }

    export fn fimo_utf8_path_new(path_str: [*:0]const u8, path: *c.FimoUTF8Path) AnyResult {
        const str = std.mem.span(path_str);
        const p = Path.init(str) catch |err| return AnyError.initError(err).intoResult();
        path.* = p.intoC();
        return AnyResult.ok;
    }

    export fn fimo_utf8_path_is_absolute(path: c.FimoUTF8Path) bool {
        const p = Path.initC(path);
        return p.isAbsolute();
    }

    export fn fimo_utf8_path_is_relative(path: c.FimoUTF8Path) bool {
        const p = Path.initC(path);
        return p.isRelative();
    }

    export fn fimo_utf8_path_has_root(path: c.FimoUTF8Path) bool {
        const p = Path.initC(path);
        return p.hasRoot();
    }

    export fn fimo_utf8_path_parent(path: c.FimoUTF8Path, parent: *c.FimoUTF8Path) bool {
        const p = Path.initC(path);
        const par = p.parent() orelse return false;
        parent.* = par.intoC();
        return true;
    }

    export fn fimo_utf8_path_file_name(path: c.FimoUTF8Path, file_name: *c.FimoUTF8Path) bool {
        const p = Path.initC(path);
        const f = p.fileName() orelse return false;
        file_name.* = f.intoC();
        return true;
    }

    export fn fimo_owned_os_path_from_path(
        path: c.FimoUTF8Path,
        os_path: *c.FimoOwnedOSPath,
    ) AnyResult {
        const p = Path.initC(path);
        const o = OwnedOsPathUnmanaged.initPath(
            std.heap.c_allocator,
            p,
        ) catch |err| return AnyError.initError(err).intoResult();
        os_path.* = o.intoC();
        return AnyResult.ok;
    }

    export fn fimo_owned_os_path_free(path: c.FimoOwnedOSPath) void {
        const o = OwnedOsPathUnmanaged.initC(path);
        o.deinit(std.heap.c_allocator);
    }

    export fn fimo_owned_os_path_as_os_path(path: c.FimoOwnedOSPath) c.FimoOSPath {
        const o = OwnedOsPathUnmanaged.initC(path);
        return o.asOsPath().intoC();
    }

    export fn fimo_os_path_new(path: [*:0]const c.FimoOSPathChar) c.FimoOSPath {
        const p = OsPath{ .raw = std.mem.span(path) };
        return p.intoC();
    }

    export fn fimo_utf8_path_component_iter_new(path: c.FimoUTF8Path) c.FimoUTF8PathComponentIterator {
        const p = Path.initC(path);
        return p.iterator().intoC();
    }

    export fn fimo_utf8_path_component_iter_as_path(
        iter: *const c.FimoUTF8PathComponentIterator,
    ) c.FimoUTF8Path {
        const it = Path.Iterator.initC(iter.*);
        return it.asPath().intoC();
    }

    export fn fimo_utf8_path_component_iter_next(
        iter: *c.FimoUTF8PathComponentIterator,
        component: *c.FimoUTF8PathComponent,
    ) bool {
        var it = Path.Iterator.initC(iter.*);
        const comp = it.next();
        iter.* = it.intoC();
        component.* = if (comp) |co| co.intoC() else return false;
        return true;
    }

    export fn fimo_utf8_path_component_iter_next_back(
        iter: *c.FimoUTF8PathComponentIterator,
        component: *c.FimoUTF8PathComponent,
    ) bool {
        var it = Path.Iterator.initC(iter.*);
        const comp = it.nextBack();
        iter.* = it.intoC();
        component.* = if (comp) |co| co.intoC() else return false;
        return true;
    }

    export fn fimo_utf8_path_component_as_path(
        component: *const c.FimoUTF8PathComponent,
    ) c.FimoUTF8Path {
        const comp = Path.Component.initC(component.*);
        return comp.asPath().intoC();
    }
};

comptime {
    _ = ffi;
}
