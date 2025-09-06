const std = @import("std");
const testing = std.testing;
const builtin = @import("builtin");

/// A slice of mutable entries.
pub fn Slice(T: type) type {
    return extern struct {
        ptr: ?[*]T = null,
        len: usize = 0,

        pub fn fromSlice(value: ?[]T) @This() {
            const s = value orelse return .{};
            return .{ .ptr = s.ptr, .len = s.len };
        }

        pub fn intoSlice(self: @This()) ?[]T {
            if (self.len == 0) return null;
            const ptr = self.ptr orelse return null;
            return ptr[0..self.len];
        }

        pub fn intoSliceOrEmpty(self: @This()) []T {
            return self.intoSlice() orelse &.{};
        }
    };
}

test "slice default" {
    const s = Slice(i32){};
    try testing.expectEqual(null, s.ptr);
    try testing.expectEqual(0, s.len);
    try testing.expectEqual(null, s.intoSlice());
}

test "slice null" {
    const s = Slice(i32).fromSlice(null);
    try testing.expectEqual(null, s.ptr);
    try testing.expectEqual(0, s.len);
    try testing.expectEqual(null, s.intoSlice());
}

test "slice non-null" {
    var array: [5]i32 = .{ 1, 2, 3, 4, 5 };
    const slice: []i32 = &array;
    const s = Slice(i32).fromSlice(slice);
    try testing.expectEqual(slice.ptr, s.ptr);
    try testing.expectEqual(slice.len, s.len);
    try testing.expectEqual(slice, s.intoSlice());
}

/// A slice of constant entries.
pub fn SliceConst(T: type) type {
    return extern struct {
        ptr: ?[*]const T = null,
        len: usize = 0,

        pub fn fromSlice(value: ?[]const T) @This() {
            const s = value orelse return .{};
            return .{ .ptr = s.ptr, .len = s.len };
        }

        pub fn intoSlice(self: @This()) ?[]const T {
            if (self.len == 0) return null;
            const ptr = self.ptr orelse return null;
            return ptr[0..self.len];
        }

        pub fn intoSliceOrEmpty(self: @This()) []const T {
            return self.intoSlice() orelse &.{};
        }
    };
}

test "slice const default" {
    const s = SliceConst(i32){};
    try testing.expectEqual(null, s.ptr);
    try testing.expectEqual(0, s.len);
    try testing.expectEqual(null, s.intoSlice());
}

test "slice const null" {
    const s = SliceConst(i32).fromSlice(null);
    try testing.expectEqual(null, s.ptr);
    try testing.expectEqual(0, s.len);
    try testing.expectEqual(null, s.intoSlice());
}

test "slice const non-null" {
    const slice: []const i32 = &.{ 1, 2, 3, 4, 5 };
    const s = SliceConst(i32).fromSlice(slice);
    try testing.expectEqual(slice.ptr, s.ptr);
    try testing.expectEqual(slice.len, s.len);
    try testing.expectEqual(slice, s.intoSlice());
}

pub const Uuid = extern union {
    groups: extern struct {
        group1: u32 = 0,
        group2: u16 = 0,
        group3: u16 = 0,
        group4: u16 = 0,
        group5: [6]u8 = @splat(0),
    },
    bytes: [16]u8,
    dwords: [4]u32,
    qwords: [2]u64,

    // Copied from the win32 module.
    const big_endian_hex_offsets = [16]u6{ 0, 2, 4, 6, 9, 11, 14, 16, 19, 21, 24, 26, 28, 30, 32, 34 };
    const little_endian_hex_offsets = [16]u6{ 6, 4, 2, 0, 11, 9, 16, 14, 19, 21, 24, 26, 28, 30, 32, 34 };

    const hex_offsets = switch (builtin.target.cpu.arch.endian()) {
        .big => big_endian_hex_offsets,
        .little => little_endian_hex_offsets,
    };

    fn hexVal(c: u8) u4 {
        if (c <= '9') return @as(u4, @intCast(c - '0'));
        if (c >= 'a') return @as(u4, @intCast(c + 10 - 'a'));
        return @as(u4, @intCast(c + 10 - 'A'));
    }

    fn decodeHexByte(hex: [2]u8) u8 {
        return @as(u8, @intCast(hexVal(hex[0]))) << 4 | hexVal(hex[1]);
    }

    pub fn parseString(s: []const u8) Uuid {
        var uuid: Uuid = .{ .bytes = undefined };
        for (hex_offsets, &uuid.bytes) |hex_offset, *b| {
            b.* = decodeHexByte(.{ s[hex_offset], s[hex_offset + 1] });
        }
        return uuid;
    }
};

comptime {
    std.debug.assert(@sizeOf(Uuid) == 16);
}

test "Uuid" {
    try testing.expect(std.mem.eql(u8, switch (builtin.target.cpu.arch.endian()) {
        .big => "\x01\x23\x45\x67\x89\xAB\xEF\x10\x32\x54\x76\x98\xba\xdc\xfe\x91",
        .little => "\x67\x45\x23\x01\xAB\x89\x10\xEF\x32\x54\x76\x98\xba\xdc\xfe\x91",
    }, &Uuid.parseString("01234567-89AB-EF10-3254-7698badcfe91").bytes));
}
