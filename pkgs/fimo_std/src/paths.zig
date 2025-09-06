const std = @import("std");
const testing = std.testing;
const unicode = std.unicode;
const Allocator = std.mem.Allocator;
const builtin = @import("builtin");

const utils = @import("utils.zig");
const Slice = utils.Slice;
const SliceConst = utils.SliceConst;

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

/// Redeclaration of the C-API types.
pub const compat = struct {
    pub const PathBuffer = extern struct {
        buffer: ?[*]u8,
        length: usize,
        capacity: usize,
    };
    pub const OwnedPath = Slice(u8);
    pub const OwnedOsPath = extern struct {
        path: ?[*:0]OsPathChar,
        length: usize,
    };
    pub const OsPath = extern struct {
        path: ?[*:0]const OsPathChar,
        length: usize,
    };
    pub const Path = SliceConst(u8);
    pub const Win32Prefix = extern struct {
        type: enum(i32) {
            verbatim = 0,
            verbatim_unc = 1,
            verbatim_disk = 2,
            device_ns = 3,
            unc = 4,
            disk = 5,
        },
        data: extern union {
            verbatim: compat.Path,
            verbatim_unc: extern struct {
                hostname: compat.Path,
                share_name: compat.Path,
            },
            verbatim_disk: u8,
            device_ns: compat.Path,
            unc: extern struct {
                hostname: compat.Path,
                share_name: compat.Path,
            },
            disk: u8,
        },
    };
    pub const Component = extern struct {
        type: enum(i32) {
            prefix = 0,
            root_dir = 1,
            cur_dir = 2,
            parent_dir = 3,
            normal = 4,
        },
        data: extern union {
            prefix: extern struct {
                raw: compat.Path,
                prefix: compat.Win32Prefix,
            },
            root_dir: u8,
            cur_dir: u8,
            parent_dir: u8,
            normal: compat.Path,
        },
    };
    pub const ComponentIterator = extern struct {
        current: compat.Path,
        has_prefix: bool,
        win32_prefix: compat.Win32Prefix,
        has_root_separator: bool,
        front: State,
        back: State,

        pub const State = enum(i32) {
            prefix = 0,
            start_dir = 1,
            body = 2,
            done = 3,
        };
    };
};

pub const PathError = error{
    /// Expected an UTF-8 encoded string.
    InvalidUtf8,
};

/// A growable filesystem path encoded as UTF-8.
pub const PathBuffer = struct {
    buffer: std.ArrayList(u8) = .{},

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: compat.PathBuffer) Self {
        return if (obj.buffer) |buffer|
            .{
                .buffer = std.ArrayList(u8){
                    .items = buffer[0..obj.length],
                    .capacity = obj.capacity,
                },
            }
        else
            .{};
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: Self) compat.PathBuffer {
        return if (self.buffer.items.len == 0) .{
            .buffer = null,
            .length = 0,
            .capacity = 0,
        } else .{
            .buffer = self.buffer.items.ptr,
            .length = self.buffer.items.len,
            .capacity = self.buffer.capacity,
        };
    }

    /// Initialize the capacity to hold `capacity` bytes.
    pub fn initCapacity(allocator: Allocator, capacity: usize) Allocator.Error!Self {
        return .{ .buffer = try std.ArrayList(u8).initCapacity(allocator, capacity) };
    }

    /// Release all allocated memory.
    pub fn deinit(self: *Self, allocator: Allocator) void {
        self.buffer.deinit(allocator);
    }

    /// Extracts a reference to the path.
    pub fn asPath(self: *const Self) Path {
        return .{ .raw = self.buffer.items };
    }

    /// Clears the buffer and returns the old contents.
    pub fn toOwnedPath(self: *Self, allocator: Allocator) Allocator.Error!OwnedPath {
        const slice = try self.buffer.toOwnedSlice(allocator);
        return .{ .raw = slice };
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
            var buffer = std.ArrayList(Path.Component){};
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

            var res = std.ArrayList(u8){};
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

    pub fn format(self: Self, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try w.writeAll(self.buffer.items);
    }
};

/// An owned filesystem path encoded as UTF-8.
pub const OwnedPath = struct {
    raw: []u8,

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: compat.OwnedPath) Self {
        return .{ .raw = obj.intoSliceOrEmpty() };
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: Self) compat.OwnedPath {
        return .fromSlice(self.raw);
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
        return .{ .raw = raw };
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
                return .{ .raw = p };
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
        return .{ .raw = self.raw };
    }

    /// Coerces the owned path to a path buffer.
    pub fn toPathBuffer(self: Self) PathBuffer {
        const buffer = std.ArrayList(u8).fromOwnedSlice(self.raw);
        return PathBuffer{ .buffer = buffer };
    }

    pub fn format(self: Self, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try w.writeAll(self.raw);
    }
};

/// Character type of the native os filesystem path.
pub const OsPathChar = if (builtin.os.tag == .windows) u16 else u8;

/// An owned path that may be passed to the native os apis.
pub const OwnedOsPath = struct {
    raw: [:0]OsPathChar,

    const Self = @This();

    /// Initializes the object from a ffi object.
    pub fn initC(obj: compat.OwnedOsPath) Self {
        return .{ .raw = if (obj.path) |p| p[0..obj.length :0] else @constCast(&.{}) };
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: Self) compat.OwnedOsPath {
        return .{ .path = self.raw.ptr, .length = self.raw.len };
    }

    /// Initializes the os path from another os path.
    pub fn initOsPath(allocator: Allocator, path: OsPath) Allocator.Error!Self {
        const raw = try allocator.dupeZ(OsPathChar, path.raw);
        .{ .raw = raw };
    }

    /// Constructs a new owned os from a UTF-8 path.
    ///
    /// On Windows the path will be re-encoded to UTF-16.
    pub fn initPath(allocator: Allocator, path: Path) (PathError || Allocator.Error)!Self {
        switch (comptime builtin.os.tag) {
            .windows => {
                const raw = try unicode.utf8ToUtf16LeAllocZ(allocator, path.raw);
                return .{ .raw = raw };
            },
            else => {
                const raw = try allocator.dupeZ(OsPathChar, path.raw);
                return .{ .raw = raw };
            },
        }
    }

    /// Frees the memory associated with the os path.
    pub fn deinit(self: Self, allocator: Allocator) void {
        allocator.free(self.raw);
    }

    /// Extracts the os path from the owned os path.
    pub fn asOsPath(self: Self) OsPath {
        return .{ .raw = self.raw };
    }

    pub fn format(self: Self, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try w.writeAll(self.raw);
    }
};

/// A reference to a native filesystem path.
pub const OsPath = struct {
    raw: [:0]const OsPathChar,

    /// Initializes the object from a ffi object.
    pub fn initC(obj: compat.OsPath) OsPath {
        return .{ .raw = if (obj.path) |p| p[0..obj.length :0] else &.{} };
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: OsPath) compat.OsPath {
        return .{ .path = self.raw.ptr, .length = self.raw.len };
    }

    pub fn format(self: OsPath, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try w.writeAll(@ptrCast(self.raw));
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
        pub fn initC(obj: compat.Win32Prefix) Prefix {
            return switch (obj.type) {
                .verbatim => .{ .verbatim = Path.initC(obj.data.verbatim) },
                .verbatim_unc => .{
                    .verbatim_unc = .{
                        .hostname = Path.initC(obj.data.verbatim_unc.hostname),
                        .share_name = Path.initC(obj.data.verbatim_unc.share_name),
                    },
                },
                .verbatim_disk => .{ .verbatim_disk = obj.data.verbatim_disk },
                .device_ns => .{ .device_ns = Path.initC(obj.data.device_ns) },
                .unc => .{ .unc = .{
                    .hostname = Path.initC(obj.data.unc.hostname),
                    .share_name = Path.initC(obj.data.unc.share_name),
                } },
                .disk => .{ .disk = obj.data.disk },
            };
        }

        /// Casts the object to a ffi object.
        pub fn intoC(self: Prefix) compat.Win32Prefix {
            return switch (self) {
                .verbatim => |v| .{
                    .type = .verbatim,
                    .data = .{ .verbatim = v.intoC() },
                },
                .verbatim_unc => |v| .{
                    .type = .verbatim_unc,
                    .data = .{
                        .verbatim_unc = .{
                            .hostname = v.hostname.intoC(),
                            .share_name = v.share_name.intoC(),
                        },
                    },
                },
                .verbatim_disk => |v| .{
                    .type = .verbatim_disk,
                    .data = .{ .verbatim_disk = v },
                },
                .device_ns => |v| .{
                    .type = .device_ns,
                    .data = .{ .device_ns = v.intoC() },
                },
                .unc => |v| .{
                    .type = .unc,
                    .data = .{
                        .unc = .{
                            .hostname = v.hostname.intoC(),
                            .share_name = v.share_name.intoC(),
                        },
                    },
                },
                .disk => |v| .{
                    .type = .disk,
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
                        const hostname: Path = .{ .raw = unc[0..pos] };
                        const rest = unc[pos + 1 ..];

                        const share_name_len = indexOfSeparatorVerbatim(rest) orelse rest.len;
                        const share_name: Path = .{ .raw = rest[0..share_name_len] };

                        return .{ .verbatim_unc = .{ .hostname = hostname, .share_name = share_name } };
                    } else {
                        return .{ .verbatim_unc = .{ .hostname = .{ .raw = unc }, .share_name = .{ .raw = &.{} } } };
                    }
                }

                // Drive prefix `C:`.
                if (verbatim.len > 1 and verbatim[1] == ':') {
                    return .{ .verbatim_disk = verbatim[0] };
                }

                // Normal prefix.
                const prefix_len = indexOfSeparatorVerbatim(verbatim) orelse verbatim.len;
                return .{ .verbatim = .{ .raw = verbatim[0..prefix_len] } };
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
                return .{ .device_ns = .{ .raw = device_ns } };
            }

            // UNC `\\hostname\share_name`
            if (path.len > 1 and isSeparator(path[0]) and isSeparator(path[1])) {
                const unc = path[2..];

                // Separate hostname and share name.
                if (indexOfSeparator(unc)) |pos| {
                    const hostname: Path = .{ .raw = unc[0..pos] };
                    const rest = unc[pos + 1 ..];

                    const share_name_len = indexOfSeparator(rest) orelse rest.len;
                    const share_name: Path = .{ .raw = rest[0..share_name_len] };

                    return .{ .unc = .{ .hostname = hostname, .share_name = share_name } };
                } else {
                    return .{ .unc = .{ .hostname = .{ .raw = unc }, .share_name = .{ .raw = &.{} } } };
                }
            }

            // Disk `C:`.
            if (path.len > 1 and path[1] == ':') {
                return .{ .disk = path[0] };
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
        pub fn initC(obj: compat.Component) Component {
            return switch (obj.type) {
                .prefix => .{ .prefix = .{
                    .raw = Path.initC(obj.data.prefix.raw),
                    .prefix = Prefix.initC(obj.data.prefix.prefix),
                } },
                .root_dir => .{ .root_dir = {} },
                .cur_dir => .{ .cur_dir = {} },
                .parent_dir => .{ .parent_dir = {} },
                .normal => .{ .normal = Path.initC(obj.data.normal) },
            };
        }

        /// Casts the object to a ffi object.
        pub fn intoC(self: Component) compat.Component {
            return switch (self) {
                .prefix => |v| .{
                    .type = .prefix,
                    .data = .{ .prefix = .{
                        .raw = v.raw.intoC(),
                        .prefix = v.prefix.intoC(),
                    } },
                },
                .root_dir => .{
                    .type = .root_dir,
                    .data = .{ .root_dir = 0 },
                },
                .cur_dir => .{
                    .type = .cur_dir,
                    .data = .{ .cur_dir = 0 },
                },
                .parent_dir => .{
                    .type = .parent_dir,
                    .data = .{ .parent_dir = 0 },
                },
                .normal => |v| .{
                    .type = .normal,
                    .data = .{ .normal = v.intoC() },
                },
            };
        }

        /// Extracts the underlying path.
        pub fn asPath(self: *const Component) Path {
            return switch (self.*) {
                .prefix => |v| v.raw,
                .root_dir => .{ .raw = &.{separator} },
                .cur_dir => .{ .raw = "." },
                .parent_dir => .{ .raw = ".." },
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
        pub fn initC(obj: compat.ComponentIterator) Iterator {
            return .{
                .rest = .initC(obj.current),
                .prefix = if (obj.has_prefix) .initC(obj.win32_prefix) else null,
                .has_root_separator = obj.has_root_separator,
                .front_state = @enumFromInt(@intFromEnum(obj.front)),
                .back_state = @enumFromInt(@intFromEnum(obj.back)),
            };
        }

        /// Casts the object to a ffi object.
        pub fn intoC(self: Iterator) compat.ComponentIterator {
            return .{
                .current = self.rest.intoC(),
                .has_prefix = self.prefix != null,
                .win32_prefix = if (self.prefix) |p| p.intoC() else std.mem.zeroes(compat.Win32Prefix),
                .has_root_separator = self.has_root_separator,
                .front = @enumFromInt(@intFromEnum(self.front_state)),
                .back = @enumFromInt(@intFromEnum(self.back_state)),
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
            return Component{ .normal = .{ .raw = path } };
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
                            const raw: Path = .{ .raw = self.rest.raw[0..prefix_length] };
                            self.rest.raw = self.rest.raw[prefix_length..];
                            return .{ .prefix = .{ .raw = raw, .prefix = pre } };
                        }
                    },
                    .start_dir => {
                        self.front_state = .body;
                        if (self.has_root_separator) {
                            std.debug.assert(self.rest.raw.len > 0);
                            self.rest.raw = self.rest.raw[1..];
                            return .{ .root_dir = {} };
                        } else if (self.prefix) |pre| {
                            if (pre.hasImplicitRoot() and !pre.isVerbatim()) return .{ .root_dir = {} };
                        } else if (self.includeCurrentDir()) {
                            std.debug.assert(self.rest.raw.len > 0);
                            self.rest.raw = self.rest.raw[1..];
                            return .{ .cur_dir = {} };
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
                            return .{ .root_dir = {} };
                        } else if (self.prefix) |pre| {
                            if (pre.hasImplicitRoot() and !pre.isVerbatim()) return .{ .root_dir = {} };
                        } else if (self.includeCurrentDir()) {
                            self.rest.raw = self.rest.raw[0 .. self.rest.raw.len - 1];
                            return .{ .cur_dir = {} };
                        }
                    },
                    .prefix => {
                        self.back_state = .done;
                        if (self.prefix) |pre| {
                            return .{ .prefix = .{ .raw = self.rest, .prefix = pre } };
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
    pub fn initC(obj: compat.Path) Path {
        return .{ .raw = obj.intoSliceOrEmpty() };
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: Path) compat.Path {
        return .fromSlice(self.raw);
    }

    /// Initializes a new path, validating that it is valid UTF-8.
    pub fn init(path: []const u8) PathError!Path {
        if (!unicode.utf8ValidateSlice(path)) return error.InvalidUtf8;
        return .{ .raw = path };
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

    pub fn format(self: Path, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try w.writeAll(self.raw);
    }
};

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    const AnyError = @import("AnyError.zig");
    const AnyResult = AnyError.AnyResult;
    const memory = @import("memory.zig");
    const Alloc = memory.Allocator;

    export fn fstd_path_buf_push_alloc(buffer: *compat.PathBuffer, alloc: Alloc, path: compat.Path) AnyResult {
        var b = PathBuffer.initC(buffer.*);
        const p = Path.initC(path);
        b.pushPath(alloc.adaptIntoStdAllocator(), p) catch |err|
            return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    export fn fstd_path_buf_push_str_alloc(buffer: *compat.PathBuffer, alloc: Alloc, path: Slice(u8)) AnyResult {
        var b = PathBuffer.initC(buffer.*);
        const p = path.intoSliceOrEmpty();
        b.pushString(alloc.adaptIntoStdAllocator(), p) catch |err|
            return AnyError.initError(err).intoResult();
        return AnyResult.ok;
    }

    export fn fstd_path_buf_pop(buffer: *compat.PathBuffer) bool {
        var b = PathBuffer.initC(buffer.*);
        return b.pop();
    }

    export fn fstd_path_init(path: *compat.Path, path_str: Slice(u8)) AnyResult {
        const str = path_str.intoSliceOrEmpty();
        const p = Path.init(str) catch |err| return AnyError.initError(err).intoResult();
        path.* = p.intoC();
        return AnyResult.ok;
    }

    export fn fstd_path_is_absolute(path: compat.Path) bool {
        const p = Path.initC(path);
        return p.isAbsolute();
    }

    export fn fstd_path_is_relative(path: compat.Path) bool {
        const p = Path.initC(path);
        return p.isRelative();
    }

    export fn fstd_path_has_root(path: compat.Path) bool {
        const p = Path.initC(path);
        return p.hasRoot();
    }

    export fn fstd_path_parent(path: compat.Path, parent: *compat.Path) bool {
        const p = Path.initC(path);
        const par = p.parent() orelse return false;
        parent.* = par.intoC();
        return true;
    }

    export fn fstd_path_file_name(path: compat.Path, file_name: *compat.Path) bool {
        const p = Path.initC(path);
        const f = p.fileName() orelse return false;
        file_name.* = f.intoC();
        return true;
    }

    export fn fstd_path_iter_new(path: compat.Path) compat.ComponentIterator {
        const p = Path.initC(path);
        return p.iterator().intoC();
    }

    export fn fstd_path_iter_as_path(iter: *const compat.ComponentIterator) compat.Path {
        const it = Path.Iterator.initC(iter.*);
        return it.asPath().intoC();
    }

    export fn fstd_path_iter_next(iter: *compat.ComponentIterator, component: *compat.Component) bool {
        var it = Path.Iterator.initC(iter.*);
        const comp = it.next();
        iter.* = it.intoC();
        component.* = if (comp) |co| co.intoC() else return false;
        return true;
    }

    export fn fstd_path_iter_next_back(iter: *compat.ComponentIterator, component: *compat.Component) bool {
        var it = Path.Iterator.initC(iter.*);
        const comp = it.nextBack();
        iter.* = it.intoC();
        component.* = if (comp) |co| co.intoC() else return false;
        return true;
    }

    export fn fstd_path_component_as_path(component: *const compat.Component) compat.Path {
        const comp = Path.Component.initC(component.*);
        return comp.asPath().intoC();
    }
};

comptime {
    _ = ffi;
}
