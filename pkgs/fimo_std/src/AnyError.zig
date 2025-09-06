//! Representation of an opaque error type.

const std = @import("std");
const Io = std.Io;
const builtin = @import("builtin");

const win32 = @import("win32");

const utils = @import("utils.zig");
const Slice = utils.Slice;
const Uuid = utils.Uuid;

const Self = @This();

data: ?*anyopaque,
vtable: *const VTable,

/// Errors deriving from an ffi call.
pub const Error = error{FfiError};

/// Error type returned from the platform apis.
pub const PlatformError = switch (builtin.target.os.tag) {
    .windows => std.os.windows.DWORD,
    else => c_int,
};

pub const VTable = extern struct {
    cls: Uuid = unknown_result_cls,
    deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
    write: *const fn (data: ?*anyopaque, dst: Slice(u8), offset: usize, remaining: *usize) callconv(.c) usize,
};

pub const unknown_result_cls: Uuid = .{ .groups = .{} };
pub const ok_result_cls: Uuid = .{ .qwords = @splat(std.math.maxInt(u64)) };

const ok_description = "ok";
const ok_vtable = VTable{ .cls = ok_result_cls, .deinit = null, .write = okWrite };
fn okWrite(ptr: ?*anyopaque, dst: Slice(u8), offset: usize, remaining: *usize) callconv(.c) usize {
    std.debug.assert(ptr == null);
    const src = ok_description[offset..];

    const buffer = dst.intoSliceOrEmpty();
    const write_len = @min(src.len, buffer.len);
    remaining.* = src.len - write_len;
    @memcpy(buffer[0..write_len], src[0..write_len]);
    return write_len;
}

const anyerror_vtable = VTable{ .deinit = null, .write = anyerrorWrite };
fn anyerrorWrite(ptr: ?*anyopaque, dst: Slice(u8), offset: usize, remaining: *usize) callconv(.c) usize {
    const err_int: std.meta.Int(.unsigned, @bitSizeOf(anyerror)) = @intCast(@intFromPtr(ptr));
    const err = @errorFromInt(err_int);
    const err_name = @errorName(err)[offset..];

    const buffer = dst.intoSliceOrEmpty();
    const write_len = @min(err_name.len, buffer.len);
    remaining.* = err_name.len - write_len;
    @memcpy(buffer[0..write_len], err_name[0..write_len]);
    return write_len;
}

const platform_error_vtable = VTable{ .deinit = null, .write = platformErrorWrite };
fn platformErrorWrite(ptr: ?*anyopaque, dst: Slice(u8), offset: usize, remaining: *usize) callconv(.c) usize {
    const code: PlatformError = @intCast(@intFromPtr(ptr));
    switch (builtin.os.tag) {
        .windows => {
            var error_str: ?[*:0]u8 = null;
            const error_len = win32.system.diagnostics.debug.FormatMessageA(
                .{ .ALLOCATE_BUFFER = 1, .FROM_SYSTEM = 1, .IGNORE_INSERTS = 1 },
                null,
                code,
                (win32.system.system_services.SUBLANG_DEFAULT << 10) | win32.system.system_services.SUBLANG_NEUTRAL,
                @ptrCast(&error_str),
                0,
                null,
            );
            defer if (error_len != 0) {
                _ = win32.system.memory.LocalFree(@intCast(@intFromPtr(error_str)));
            };
            const write_str = if (error_len == 0) "error" else blk: {
                const str = error_str.?;
                // Remove the trailing `\r\n` characters.
                str[error_len - 2] = 0;
                break :blk std.mem.span(str)[offset..];
            };

            const buffer = dst.intoSliceOrEmpty();
            const write_len = @min(write_str.len, buffer.len);
            remaining.* = write_str.len - write_len;
            @memcpy(buffer[0..write_len], write_str[0..write_len]);
            return write_len;
        },
        else => {
            const errno: std.posix.E = @enumFromInt(code);
            const errno_str = @tagName(errno)[offset..];

            const buffer = dst.intoSliceOrEmpty();
            const write_len = @min(errno_str.len, buffer.len);
            remaining.* = errno_str.len - write_len;
            @memcpy(buffer[0..write_len], errno_str[0..write_len]);
            return write_len;
        },
    }
}
comptime {
    @export(&platform_error_vtable, .{ .name = "FSTD__ResultVTable_PlatformError" });
}

/// An optional error.
pub const AnyResult = extern struct {
    data: ?*anyopaque,
    vtable: *const VTable,

    /// A value representing no error.
    pub const ok = AnyResult{
        .data = null,
        .vtable = &ok_vtable,
    };

    /// Initializes a result from an `AnyError`.
    pub fn initErr(err: Self) AnyResult {
        return AnyResult{
            .data = err.data,
            .vtable = err.vtable,
        };
    }

    /// Deinitializes the result.
    pub fn deinit(self: AnyResult) void {
        if (self.vtable.deinit) |f| f(self.data);
    }

    /// Returns whether the result is not an error.
    pub fn isOk(self: *const AnyResult) bool {
        return std.mem.eql(u64, &self.vtable.cls.qwords, &ok_result_cls.qwords);
    }

    /// Returns whether the result is an error.
    pub fn isErr(self: *const AnyResult) bool {
        return !self.isOk();
    }

    /// Unwraps the contained `AnyError`.
    pub fn unwrapErr(self: AnyResult) Self {
        if (self.isOk()) unreachable;
        return Self{ .data = self.data, .vtable = self.vtable };
    }

    /// Constructs an error union from the `AnyResult`.
    pub fn intoErrorUnion(self: AnyResult, err: *?Self) Error!void {
        if (self.isOk()) return;
        err.* = self.unwrapErr();
        return Error.FfiError;
    }

    pub fn format(self: AnyResult, w: *std.Io.Writer) std.Io.Writer.Error!void {
        var offset: usize = 0;
        var remaining: usize = undefined;
        while (true) {
            var buffer: [64]u8 = undefined;
            const written = self.vtable.write(self.data, .fromSlice(&buffer), offset, &remaining);
            try w.writeAll(buffer[0..written]);
            offset += written;
            if (remaining == 0) break;
        }
    }
};

/// Creates an error from a zig error.
///
/// This function is guaranteed to never allocate any memory.
pub fn initError(err: anyerror) Self {
    if (comptime @sizeOf(anyerror) > @sizeOf(usize)) {
        @compileError("Can not pack an `anyerror` into an `AnyError`, as it is too large.");
    }
    return Self{ .data = @ptrFromInt(@intFromError(err)), .vtable = &anyerror_vtable };
}

test initError {
    const err = Self.initError(error.MyError);
    var buffer: [64]u8 = undefined;
    var writer = Io.Writer.fixed(&buffer);
    try writer.print("{f}", .{err});
    try std.testing.expect(std.mem.eql(u8, writer.buffered(), @errorName(error.MyError)));
}

/// Creates an optional error from a platform error.
///
/// This function is guaranteed to never allocate any memory.
pub fn initPlatformError(code: PlatformError) ?Self {
    if (comptime @sizeOf(PlatformError) > @sizeOf(usize)) {
        @compileError("Can not pack an `PlatformError` into an `AnyError`, as it is too large.");
    }
    if (code == 0) return null;
    return Self{
        .data = @ptrFromInt(@as(usize, @intCast(code))),
        .vtable = &platform_error_vtable,
    };
}

test initPlatformError {
    try std.testing.expect(Self.initPlatformError(0) == null);

    const error_code: PlatformError = switch (builtin.os.tag) {
        .windows => @intFromEnum(win32.foundation.ERROR_INVALID_FUNCTION),
        else => @intFromEnum(std.posix.E.@"2BIG"),
    };
    const expected_error = switch (builtin.os.tag) {
        .windows => "Incorrect function.",
        else => "2BIG",
    };

    const err = Self.initPlatformError(error_code).?;
    var buffer: [64]u8 = undefined;
    var writer = Io.Writer.fixed(&buffer);
    try writer.print("{f}", .{err});
    try std.testing.expect(std.mem.eql(u8, writer.buffered(), expected_error));
}

/// Cleans up the error.
pub fn deinit(self: Self) void {
    if (self.vtable.deinit) |f| f(self.data);
}

/// Constructs an `AnyResult` from the error.
pub fn intoResult(self: Self) AnyResult {
    return AnyResult.initErr(self);
}

pub fn format(self: Self, w: *std.Io.Writer) std.Io.Writer.Error!void {
    var offset: usize = 0;
    var remaining: usize = undefined;
    while (true) {
        var buffer: [64]u8 = undefined;
        const written = self.vtable.write(self.data, .fromSlice(&buffer), offset, &remaining);
        try w.writeAll(buffer[0..written]);
        offset += written;
        if (remaining == 0) break;
    }
}
