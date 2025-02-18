const std = @import("std");
const windows = std.os.windows;
const Allocator = std.mem.Allocator;
const builtin = @import("builtin");

const Path = @import("../../path.zig").Path;
const PathError = @import("../../path.zig").PathError;
const OsPath = @import("../../path.zig").OsPath;
const OwnedPathUnmanaged = @import("../../path.zig").OwnedPathUnmanaged;
const PathBufferUnmanaged = @import("../../path.zig").PathBufferUnmanaged;
const OwnedOsPathUnmanaged = @import("../../path.zig").OwnedOsPathUnmanaged;
const ProxyModule = @import("../proxy_context/module.zig");
const RefCount = @import("../RefCount.zig");

const Self = @This();
allocator: Allocator,
iterator: IteratorFn,
ref_count: RefCount = .{},
path: OwnedPathUnmanaged,

pub const ModuleHandleError = error{
    InvalidModule,
    InvalidPath,
    DlOpenError,
} || PathError || Allocator.Error;

pub const IteratorFn = *const fn (
    f: *const fn (
        @"export": *const ProxyModule.Export,
        data: ?*anyopaque,
    ) callconv(.c) bool,
    data: ?*anyopaque,
) callconv(.c) void;

const Inner = if (builtin.os.tag == .windows)
    struct {
        const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS = 0x04;
        extern "kernel32" fn GetModuleHandleExW(
            dwFlags: windows.DWORD,
            lpModuleName: ?windows.LPCWSTR,
            phModule: *windows.HMODULE,
        ) callconv(windows.WINAPI) windows.BOOL;
    }
else
    struct {
        const Dl_info = struct {
            dli_fname: [*:0]const u8,
            dli_fbase: *anyopaque,
            dli_sname: ?[*:0]const u8,
            dli_saddr: ?*anyopaque,
        };
        extern "c" fn dladdr(addr: *const anyopaque, info: *Dl_info) callconv(.C) c_int;
    };

pub fn initLocal(allocator: Allocator, iterator: IteratorFn, bin_ptr: *const anyopaque) !*Self {
    if (comptime builtin.os.tag == .windows) {
        var handle: windows.HMODULE = undefined;
        const found_handle = Inner.GetModuleHandleExW(
            Inner.GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            @alignCast(@ptrCast(bin_ptr)),
            &handle,
        );
        if (found_handle == 0) return error.InvalidModule;

        var path_len: usize = windows.MAX_PATH;
        var os_path = OwnedOsPathUnmanaged{
            .raw = undefined,
        };
        while (true) {
            const p = try allocator.alloc(u16, path_len);
            defer allocator.free(p);

            const raw = windows.GetModuleFileNameW(
                handle,
                p.ptr,
                @intCast(p.len),
            ) catch {
                const e = windows.GetLastError();
                if (e == .INSUFFICIENT_BUFFER) {
                    path_len *= 2;
                    continue;
                } else return error.InvalidModule;
            };
            os_path.raw = try allocator.dupeZ(u16, raw);
            break;
        }
        defer os_path.deinit(allocator);

        const p = try OwnedPathUnmanaged.initOsPath(
            allocator,
            os_path.asOsPath(),
        );
        defer p.deinit(allocator);
        const module_dir = p.asPath().parent() orelse return error.InvalidPath;
        const owned_module_dir = try OwnedPathUnmanaged.initPath(
            allocator,
            module_dir,
        );
        errdefer owned_module_dir.deinit(allocator);

        const module_handle = try allocator.create(Self);
        module_handle.* = Self{
            .allocator = allocator,
            .iterator = iterator,
            .path = owned_module_dir,
        };
        return module_handle;
    } else {
        var info: Inner.Dl_info = undefined;
        if (Inner.dladdr(bin_ptr, &info) == 0) return error.InvalidModule;

        const os_path = OsPath{ .raw = std.mem.span(info.dli_fname) };
        const p = try OwnedPathUnmanaged.initOsPath(allocator, os_path);
        defer p.deinit(allocator);
        const module_dir = p.asPath().parent() orelse return error.InvalidPath;
        const owned_module_dir = try OwnedPathUnmanaged.initPath(
            allocator,
            module_dir,
        );
        errdefer owned_module_dir.deinit(allocator);

        const is_current_module = iterator == &ProxyModule.exports.ExportIter.fimo_impl_module_export_iterator;
        const module_path: ?[*:0]const u8 = if (is_current_module)
            null
        else
            info.dli_fname;
        if (comptime builtin.target.os.tag.isDarwin())
            _ = std.c.dlopen(
                module_path,
                .{ .NOW = true, .LOCAL = true, .NOLOAD = true },
            ) orelse return error.InvalidModule
        else
            _ = std.c.dlopen(
                module_path,
                .{ .NOW = true, .NOLOAD = true },
            ) orelse return error.InvalidModule;

        const module_handle = try allocator.create(Self);
        module_handle.* = Self{
            .allocator = allocator,
            .iterator = iterator,
            .path = owned_module_dir,
        };
        return module_handle;
    }
}

pub fn initPath(allocator: Allocator, p: Path, tmp_dir: Path) ModuleHandleError!*Self {
    var buffer = PathBufferUnmanaged{};
    defer buffer.deinit(allocator);

    const cwd = std.fs.cwd().realpathAlloc(allocator, ".") catch return error.InvalidPath;
    defer allocator.free(cwd);
    try buffer.pushString(allocator, cwd);

    const stat = std.fs.cwd().statFile(p.raw) catch return error.InvalidPath;
    switch (stat.kind) {
        .file => try buffer.pushPath(allocator, p),
        .directory => {
            const default_module_name = Path.init("module.fimo_module") catch unreachable;
            try buffer.pushPath(allocator, default_module_name);
        },
        .sym_link => {
            const link_buffer = try allocator.alloc(u8, std.fs.max_path_bytes);
            defer allocator.free(link_buffer);
            const resolved = std.fs.cwd().readLink(
                p.raw,
                link_buffer,
            ) catch return error.InvalidPath;
            const res_p = Path.init(resolved) catch return error.InvalidPath;
            return Self.initPath(allocator, res_p, tmp_dir);
        },
        else => return error.InvalidPath,
    }

    const module_binary = buffer.asPath().fileName() orelse return error.InvalidPath;
    const module_dir = buffer.asPath().parent() orelse return error.InvalidPath;

    var symlink_path = PathBufferUnmanaged{};
    errdefer symlink_path.deinit(allocator);
    try symlink_path.pushPath(allocator, tmp_dir);
    while (true) {
        var random_bytes: [8]u8 = undefined;
        std.crypto.random.bytes(&random_bytes);
        var suffix: [std.fs.base64_encoder.calcSize(8)]u8 = undefined;
        _ = std.fs.base64_encoder.encode(&suffix, &random_bytes);
        const sub_path = try std.fmt.allocPrint(
            allocator,
            "module_{s}",
            .{suffix},
        );
        defer allocator.free(sub_path);
        try symlink_path.pushString(allocator, sub_path);

        std.fs.cwd().symLink(module_dir.raw, symlink_path.asPath().raw, .{
            .is_directory = true,
        }) catch |err| switch (err) {
            error.PathAlreadyExists => _ = symlink_path.pop(),
            else => return error.InvalidPath,
        };
        break;
    }
    try symlink_path.pushPath(allocator, module_binary);

    const native_path = try OwnedOsPathUnmanaged.initPath(
        allocator,
        symlink_path.asPath(),
    );
    defer native_path.deinit(allocator);

    var handle = try allocator.create(Self);
    errdefer allocator.destroy(handle);
    handle.* = .{
        .allocator = allocator,
        .path = undefined,
        .iterator = undefined,
    };

    _ = symlink_path.pop();
    handle.path = try symlink_path.toOwnedPath(allocator);
    errdefer handle.path.deinit(allocator);

    const raw_handle = if (comptime builtin.os.tag == .windows)
        windows.kernel32.LoadLibraryExW(
            native_path.raw.ptr,
            null,
            @intFromEnum(windows.LoadLibraryFlags.load_library_search_dll_load_dir) |
                @intFromEnum(windows.LoadLibraryFlags.load_library_search_default_dirs),
        ) orelse return error.DlOpenError
    else if (comptime builtin.target.os.tag.isDarwin())
        std.c.dlopen(
            native_path.raw.ptr,
            .{ .NOW = true, .LOCAL = true, .NODELETE = true },
        ) orelse return error.DlOpenError
    else
        std.c.dlopen(
            native_path.raw.ptr,
            .{ .NOW = true, .NODELETE = true },
        ) orelse return error.DlOpenError;

    const iterator = if (comptime builtin.os.tag == .windows)
        windows.kernel32.GetProcAddress(
            raw_handle,
            "fimo_impl_module_export_iterator",
        ) orelse return error.InvalidModule
    else
        std.c.dlsym(
            raw_handle,
            "fimo_impl_module_export_iterator",
        ) orelse return error.InvalidModule;
    handle.iterator = @alignCast(@ptrCast(iterator));

    return handle;
}

pub fn ref(self: *Self) void {
    self.ref_count.ref();
}

pub fn unref(self: *Self) void {
    if (self.ref_count.unref() == .noop) return;

    const allocator = self.allocator;
    self.path.deinit(allocator);
    allocator.destroy(self);
}
