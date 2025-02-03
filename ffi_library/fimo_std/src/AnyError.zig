//! Representation of an opaque error type.

const std = @import("std");
const builtin = @import("builtin");

const c = @import("c.zig");
const heap = @import("heap.zig");

const Self = @This();

/// Contained error. Must always represent an actual error.
err: c.FimoResult,

/// Errors deriving from an ffi call.
pub const Error = error{FfiError};

const anyerror_vtable = c.FimoResultVTable{
    .v0 = .{
        .release = null,
        .error_name = anyerror_string,
        .error_description = anyerror_string,
    },
};
fn anyerror_string(ptr: ?*anyopaque) callconv(.C) c.FimoResultString {
    const err_int: std.meta.Int(.unsigned, @bitSizeOf(anyerror)) = @intCast(@intFromPtr(ptr));
    const err = @errorFromInt(err_int);
    return ErrorString.init(@errorName(err)).str;
}

// Declared in the C header.
export const FIMO_IMPL_RESULT_STATIC_STRING_VTABLE = c.FimoResultVTable{
    .v0 = .{
        .release = null,
        .error_name = static_string_string,
        .error_description = static_string_string,
    },
};
fn static_string_string(ptr: ?*anyopaque) callconv(.C) c.FimoResultString {
    const str: [*:0]const u8 = @constCast(@alignCast(@ptrCast(ptr.?)));
    return ErrorString.init(str).str;
}

// Declared in the C header.
export const FIMO_IMPL_RESULT_DYNAMIC_STRING_VTABLE = c.FimoResultVTable{
    .v0 = .{
        .release = dynamic_string_release,
        .error_name = dynamic_string_string,
        .error_description = dynamic_string_string,
    },
};
fn dynamic_string_release(ptr: ?*anyopaque) callconv(.C) void {
    const err_c: [*:0]const u8 = @constCast(@alignCast(@ptrCast(ptr.?)));
    const err = std.mem.span(err_c);
    heap.fimo_allocator.free(err);
}
fn dynamic_string_string(ptr: ?*anyopaque) callconv(.C) c.FimoResultString {
    const err_c: [*:0]const u8 = @constCast(@alignCast(@ptrCast(ptr.?)));
    const err = std.mem.span(err_c);

    const string = ErrorString.initDupe(
        heap.fimo_allocator,
        err,
    ) catch |e| Self.initError(e).name();
    return string.str;
}

// Declared in the C header.
export const FIMO_IMPL_RESULT_ERROR_CODE_VTABLE = c.FimoResultVTable{
    .v0 = .{
        .release = null,
        .error_name = error_code_name,
        .error_description = error_code_description,
    },
};
fn error_code_name(ptr: ?*anyopaque) callconv(.C) c.FimoResultString {
    const code: ErrorCode = @enumFromInt(@intFromPtr(ptr));
    const string = ErrorString.init(code.name());
    return string.str;
}
fn error_code_description(ptr: ?*anyopaque) callconv(.C) c.FimoResultString {
    const code: ErrorCode = @enumFromInt(@intFromPtr(ptr));
    const string = ErrorString.init(code.description());
    return string.str;
}

// Declared in the C header.
export const FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE = c.FimoResultVTable{
    .v0 = .{
        .release = null,
        .error_name = system_error_code_name,
        .error_description = system_error_code_description,
    },
};
fn system_error_code_name(ptr: ?*anyopaque) callconv(.C) c.FimoResultString {
    const code: SystemErrorCode = @intCast(@intFromPtr(ptr));
    const string = ErrorString.initFmt(
        heap.fimo_allocator,
        "SystemError({})",
        .{code},
    ) catch |e| Self.initError(e).name();
    return string.str;
}
fn system_error_code_description(ptr: ?*anyopaque) callconv(.C) c.FimoResultString {
    const code: SystemErrorCode = @intCast(@intFromPtr(ptr));
    switch (builtin.target.os.tag) {
        .windows => {
            const FreeImpl = struct {
                fn free(p: [*c]const u8) callconv(.c) void {
                    std.os.windows.LocalFree(@constCast(p));
                }
            };

            var error_description: ?std.os.windows.LPSTR = null;
            const format_result = FormatMessageA(
                std.os.windows.FORMAT_MESSAGE_ALLOCATE_BUFFER | std.os.windows.FORMAT_MESSAGE_FROM_SYSTEM | std.os.windows.FORMAT_MESSAGE_IGNORE_INSERTS,
                null,
                code,
                (std.os.windows.SUBLANG.DEFAULT << 10) | std.os.windows.LANG.NEUTRAL,
                @ptrCast(&error_description),
                0,
                null,
            );
            if (format_result == 0) {
                return ErrorString.init("SystemError(\"unknown error\")").str;
            }
            // Remove the trailing `\r\n` characters.
            error_description.?[format_result - 2] = 0;
            return c.FimoResultString{
                .str = @ptrCast(@constCast(error_description)),
                .release = FreeImpl.free,
            };
        },
        else => {
            const errno: std.posix.E = @enumFromInt(code);
            return ErrorString.init(@tagName(errno)).str;
        },
    }
}
extern fn FormatMessageA(
    dwFlags: std.os.windows.DWORD,
    lpSource: ?std.os.windows.LPCVOID,
    dwMessageId: std.os.windows.DWORD,
    dwLanguageId: std.os.windows.DWORD,
    lpBuffer: std.os.windows.LPSTR,
    nSize: std.os.windows.DWORD,
    Arguments: ?std.os.windows.va_list,
) std.os.windows.DWORD;

// Declared in the C header.
export const FIMO_IMPL_RESULT_OK = c.FimoResult{
    .data = null,
    .vtable = null,
};
export const FIMO_IMPL_RESULT_INVALID_ERROR = c.FimoResult{
    .data = @ptrCast(@constCast(@as([*:0]const u8, "invalid error"))),
    .vtable = &FIMO_IMPL_RESULT_STATIC_STRING_VTABLE,
};
export const FIMO_IMPL_RESULT_OK_NAME = ErrorString.init("ok").str;
export const FIMO_IMPL_RESULT_OK_DESCRIPTION = ErrorString.init("ok").str;

/// Creates an error from a zig error.
///
/// This function is guaranteed to never allocate any memory.
pub fn initError(err: anyerror) Self {
    if (comptime @sizeOf(anyerror) > @sizeOf(usize)) {
        @compileError("Can not pack an `anyerror` into an `AnyError`, as it is too large.");
    }

    return Self{ .err = .{
        .data = @ptrFromInt(@intFromError(err)),
        .vtable = &anyerror_vtable,
    } };
}

test initError {
    const err = Self.initError(error.MyError);
    try std.testing.expect(std.mem.eql(u8, err.name().string(), @errorName(error.MyError)));
}

/// Creates an optional error from an error code.
///
/// This function is guaranteed to never allocate any memory.
pub fn initErrorCode(code: ErrorCode) ?Self {
    if (comptime @sizeOf(ErrorCode) > @sizeOf(usize)) {
        @compileError("Can not pack an `ErrorCode` into an `AnyError`, as it is too large.");
    }
    if (code == .ok) return null;
    const code_int: usize = @intCast(@intFromEnum(code));
    return Self{ .err = .{
        .data = @ptrFromInt(code_int),
        .vtable = &FIMO_IMPL_RESULT_ERROR_CODE_VTABLE,
    } };
}

test initErrorCode {
    const eok = Self.initErrorCode(.ok);
    try std.testing.expect(eok == null);

    const einval = Self.initErrorCode(.inval);
    try std.testing.expect(einval != null);

    const error_name = einval.?.name();
    defer error_name.deinit();

    const error_description = einval.?.description();
    defer error_description.deinit();
}

/// Creates an optional error from a system error code.
///
/// This function is guaranteed to never allocate any memory.
pub fn initSystemErrorCode(code: SystemErrorCode) ?Self {
    if (comptime @sizeOf(SystemErrorCode) > @sizeOf(usize)) {
        @compileError("Can not pack an `SystemErrorCode` into an `AnyError`, as it is too large.");
    }
    if (code == 0) return null;
    return Self{ .err = .{
        .data = @ptrFromInt(@as(usize, @intCast(code))),
        .vtable = &FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE,
    } };
}

test initSystemErrorCode {
    try std.testing.expect(Self.initSystemErrorCode(0) == null);

    const error_code: SystemErrorCode = switch (builtin.os.tag) {
        .windows => @intFromEnum(std.os.windows.Win32Error.INVALID_FUNCTION),
        else => @intFromEnum(std.posix.E.@"2BIG"),
    };
    const expected_error_name = std.fmt.comptimePrint("SystemError({})", .{error_code});
    const expected_error_description = switch (builtin.os.tag) {
        .windows => "Incorrect function.",
        else => "2BIG",
    };

    const err = Self.initSystemErrorCode(error_code);
    try std.testing.expect(err != null);

    const error_name = err.?.name();
    defer error_name.deinit();
    try std.testing.expect(std.mem.eql(u8, error_name.string(), expected_error_name));

    const error_description = err.?.description();
    defer error_description.deinit();
    try std.testing.expect(std.mem.eql(u8, error_description.string(), expected_error_description));
}

/// Creates an optional error from the c result.
pub fn initC(err: c.FimoResult) ?Self {
    if (c.fimo_result_is_ok(err)) return null;
    return Self{ .err = err };
}

/// Checks whether the pointed to value contains an error.
pub fn checkError(err: *?Self) Error!void {
    if (err.* != null) return error.FfiError;
}

/// Initializes the error and checks whether it contains an error.
pub fn initChecked(err: *?Self, c_result: c.FimoResult) Error!void {
    if (err.*) |e| e.deinit();
    err.* = Self.initC(c_result);
    return checkError(err);
}

/// Cleans up the error.
pub fn deinit(self: Self) void {
    c.fimo_result_release(self.err);
}

/// Unwraps the optional error to the c equivalent.
pub fn intoCResult(self: ?Self) c.FimoResult {
    if (self) |v| return v.err;
    return .{ .data = null, .vtable = null };
}

/// Returns the name string of the error.
pub fn name(self: *const Self) ErrorString {
    const str = self.err.vtable.*.v0.error_name.?(self.err.data);
    return ErrorString{ .str = str };
}

/// Returns the description string of the error.
pub fn description(self: *const Self) ErrorString {
    const str = self.err.vtable.*.v0.error_description.?(self.err.data);
    return ErrorString{ .str = str };
}

/// Formats the error.
///
/// # Format specifiers
///
/// * `{}`: Prints the error description.
/// * `{dbg}`: Prints the error name.
pub fn format(
    self: *const Self,
    comptime fmt: []const u8,
    options: std.fmt.FormatOptions,
    writer: anytype,
) !void {
    _ = options;

    const debug = comptime parse_fmt: {
        if (fmt.len == 0) {
            break :parse_fmt false;
        } else if (std.mem.eql(u8, fmt, "dbg")) {
            break :parse_fmt true;
        } else {
            @compileError("expected {}, or {dbg}, found {" ++ fmt ++ "}");
        }
    };

    const string = if (debug) self.name() else self.description();
    defer string.deinit();
    try writer.writeAll(string.string());
}

/// Posix error codes.
pub const ErrorCode = enum(i32) {
    ok = c.FIMO_ERROR_CODE_OK,
    toobig = c.FIMO_ERROR_CODE_2BIG,
    acces = c.FIMO_ERROR_CODE_ACCES,
    addrinuse = c.FIMO_ERROR_CODE_ADDRINUSE,
    addrnotavail = c.FIMO_ERROR_CODE_ADDRNOTAVAIL,
    afnosupport = c.FIMO_ERROR_CODE_AFNOSUPPORT,
    again = c.FIMO_ERROR_CODE_AGAIN,
    already = c.FIMO_ERROR_CODE_ALREADY,
    bade = c.FIMO_ERROR_CODE_BADE,
    badf = c.FIMO_ERROR_CODE_BADF,
    badfd = c.FIMO_ERROR_CODE_BADFD,
    badmsg = c.FIMO_ERROR_CODE_BADMSG,
    badr = c.FIMO_ERROR_CODE_BADR,
    badrqc = c.FIMO_ERROR_CODE_BADRQC,
    badslt = c.FIMO_ERROR_CODE_BADSLT,
    busy = c.FIMO_ERROR_CODE_BUSY,
    canceled = c.FIMO_ERROR_CODE_CANCELED,
    child = c.FIMO_ERROR_CODE_CHILD,
    chrng = c.FIMO_ERROR_CODE_CHRNG,
    comm = c.FIMO_ERROR_CODE_COMM,
    connaborted = c.FIMO_ERROR_CODE_CONNABORTED,
    connrefused = c.FIMO_ERROR_CODE_CONNREFUSED,
    connreset = c.FIMO_ERROR_CODE_CONNRESET,
    deadlk = c.FIMO_ERROR_CODE_DEADLK,
    deadlock = c.FIMO_ERROR_CODE_DEADLOCK,
    destaddrreq = c.FIMO_ERROR_CODE_DESTADDRREQ,
    dom = c.FIMO_ERROR_CODE_DOM,
    dquot = c.FIMO_ERROR_CODE_DQUOT,
    exist = c.FIMO_ERROR_CODE_EXIST,
    fault = c.FIMO_ERROR_CODE_FAULT,
    fbig = c.FIMO_ERROR_CODE_FBIG,
    hostdown = c.FIMO_ERROR_CODE_HOSTDOWN,
    hostunreach = c.FIMO_ERROR_CODE_HOSTUNREACH,
    hwpoison = c.FIMO_ERROR_CODE_HWPOISON,
    idrm = c.FIMO_ERROR_CODE_IDRM,
    ilseq = c.FIMO_ERROR_CODE_ILSEQ,
    inprogress = c.FIMO_ERROR_CODE_INPROGRESS,
    intr = c.FIMO_ERROR_CODE_INTR,
    inval = c.FIMO_ERROR_CODE_INVAL,
    io = c.FIMO_ERROR_CODE_IO,
    isconn = c.FIMO_ERROR_CODE_ISCONN,
    isdir = c.FIMO_ERROR_CODE_ISDIR,
    isnam = c.FIMO_ERROR_CODE_ISNAM,
    keyexpired = c.FIMO_ERROR_CODE_KEYEXPIRED,
    keyrejected = c.FIMO_ERROR_CODE_KEYREJECTED,
    keyrevoked = c.FIMO_ERROR_CODE_KEYREVOKED,
    l2hlt = c.FIMO_ERROR_CODE_L2HLT,
    l2nsync = c.FIMO_ERROR_CODE_L2NSYNC,
    l3hlt = c.FIMO_ERROR_CODE_L3HLT,
    l3rst = c.FIMO_ERROR_CODE_L3RST,
    libacc = c.FIMO_ERROR_CODE_LIBACC,
    libbad = c.FIMO_ERROR_CODE_LIBBAD,
    libmax = c.FIMO_ERROR_CODE_LIBMAX,
    libscn = c.FIMO_ERROR_CODE_LIBSCN,
    libexec = c.FIMO_ERROR_CODE_LIBEXEC,
    lnrng = c.FIMO_ERROR_CODE_LNRNG,
    loop = c.FIMO_ERROR_CODE_LOOP,
    mediumtype = c.FIMO_ERROR_CODE_MEDIUMTYPE,
    mfile = c.FIMO_ERROR_CODE_MFILE,
    mlink = c.FIMO_ERROR_CODE_MLINK,
    msgsize = c.FIMO_ERROR_CODE_MSGSIZE,
    multihop = c.FIMO_ERROR_CODE_MULTIHOP,
    nametoolong = c.FIMO_ERROR_CODE_NAMETOOLONG,
    netdown = c.FIMO_ERROR_CODE_NETDOWN,
    netreset = c.FIMO_ERROR_CODE_NETRESET,
    netunreach = c.FIMO_ERROR_CODE_NETUNREACH,
    nfile = c.FIMO_ERROR_CODE_NFILE,
    noano = c.FIMO_ERROR_CODE_NOANO,
    nobufs = c.FIMO_ERROR_CODE_NOBUFS,
    nodata = c.FIMO_ERROR_CODE_NODATA,
    nodev = c.FIMO_ERROR_CODE_NODEV,
    noent = c.FIMO_ERROR_CODE_NOENT,
    noexec = c.FIMO_ERROR_CODE_NOEXEC,
    nokey = c.FIMO_ERROR_CODE_NOKEY,
    nolck = c.FIMO_ERROR_CODE_NOLCK,
    nolink = c.FIMO_ERROR_CODE_NOLINK,
    nomedium = c.FIMO_ERROR_CODE_NOMEDIUM,
    nomem = c.FIMO_ERROR_CODE_NOMEM,
    nomsg = c.FIMO_ERROR_CODE_NOMSG,
    nonet = c.FIMO_ERROR_CODE_NONET,
    nopkg = c.FIMO_ERROR_CODE_NOPKG,
    noprotoopt = c.FIMO_ERROR_CODE_NOPROTOOPT,
    nospc = c.FIMO_ERROR_CODE_NOSPC,
    nosr = c.FIMO_ERROR_CODE_NOSR,
    nostr = c.FIMO_ERROR_CODE_NOSTR,
    nosys = c.FIMO_ERROR_CODE_NOSYS,
    notblk = c.FIMO_ERROR_CODE_NOTBLK,
    notconn = c.FIMO_ERROR_CODE_NOTCONN,
    notdir = c.FIMO_ERROR_CODE_NOTDIR,
    notempty = c.FIMO_ERROR_CODE_NOTEMPTY,
    notrecoverable = c.FIMO_ERROR_CODE_NOTRECOVERABLE,
    notsock = c.FIMO_ERROR_CODE_NOTSOCK,
    notsup = c.FIMO_ERROR_CODE_NOTSUP,
    notty = c.FIMO_ERROR_CODE_NOTTY,
    notuniq = c.FIMO_ERROR_CODE_NOTUNIQ,
    nxio = c.FIMO_ERROR_CODE_NXIO,
    opnotsupp = c.FIMO_ERROR_CODE_OPNOTSUPP,
    overflow = c.FIMO_ERROR_CODE_OVERFLOW,
    ownerdead = c.FIMO_ERROR_CODE_OWNERDEAD,
    perm = c.FIMO_ERROR_CODE_PERM,
    pfnosupport = c.FIMO_ERROR_CODE_PFNOSUPPORT,
    pipe = c.FIMO_ERROR_CODE_PIPE,
    proto = c.FIMO_ERROR_CODE_PROTO,
    protonosupport = c.FIMO_ERROR_CODE_PROTONOSUPPORT,
    prototype = c.FIMO_ERROR_CODE_PROTOTYPE,
    range = c.FIMO_ERROR_CODE_RANGE,
    remchg = c.FIMO_ERROR_CODE_REMCHG,
    remote = c.FIMO_ERROR_CODE_REMOTE,
    remoteio = c.FIMO_ERROR_CODE_REMOTEIO,
    restart = c.FIMO_ERROR_CODE_RESTART,
    rfkill = c.FIMO_ERROR_CODE_RFKILL,
    rofs = c.FIMO_ERROR_CODE_ROFS,
    shutdown = c.FIMO_ERROR_CODE_SHUTDOWN,
    spipe = c.FIMO_ERROR_CODE_SPIPE,
    socktnosupport = c.FIMO_ERROR_CODE_SOCKTNOSUPPORT,
    srch = c.FIMO_ERROR_CODE_SRCH,
    stale = c.FIMO_ERROR_CODE_STALE,
    strpipe = c.FIMO_ERROR_CODE_STRPIPE,
    time = c.FIMO_ERROR_CODE_TIME,
    timedout = c.FIMO_ERROR_CODE_TIMEDOUT,
    toomanyrefs = c.FIMO_ERROR_CODE_TOOMANYREFS,
    txtbsy = c.FIMO_ERROR_CODE_TXTBSY,
    uclean = c.FIMO_ERROR_CODE_UCLEAN,
    unatch = c.FIMO_ERROR_CODE_UNATCH,
    users = c.FIMO_ERROR_CODE_USERS,
    wouldblock = c.FIMO_ERROR_CODE_WOULDBLOCK,
    xdev = c.FIMO_ERROR_CODE_XDEV,
    xfull = c.FIMO_ERROR_CODE_XFULL,

    pub fn name(self: ErrorCode) [:0]const u8 {
        return @tagName(self);
    }

    pub fn description(self: ErrorCode) [:0]const u8 {
        return switch (self) {
            .ok => "operation completed successfully",
            .toobig => "argument list too long",
            .acces => "permission denied",
            .addrinuse => "address already in use",
            .addrnotavail => "address not available",
            .afnosupport => "address family not supported",
            .again => "resource temporarily unavailable",
            .already => "connection already in progress",
            .bade => "invalid exchange",
            .badf => "bad file descriptor",
            .badfd => "file descriptor in bad state",
            .badmsg => "bad message",
            .badr => "invalid request descriptor",
            .badrqc => "invalid request code",
            .badslt => "invalid slot",
            .busy => "device or resource busy",
            .canceled => "operation canceled",
            .child => "no child processes",
            .chrng => "channel number out of range",
            .comm => "communication error on send",
            .connaborted => "connection aborted",
            .connrefused => "connection refused",
            .connreset => "connection reset",
            .deadlk => "resource deadlock avoided",
            .deadlock => "file locking deadlock error",
            .destaddrreq => "destination address required",
            .dom => "mathematics argument out of domain of function",
            .dquot => "disk quota exceeded",
            .exist => "file exists",
            .fault => "bad address",
            .fbig => "file too large",
            .hostdown => "host is down",
            .hostunreach => "host is unreachable",
            .hwpoison => "memory page has hardware error",
            .idrm => "identifier removed",
            .ilseq => "invalid or incomplete multibyte or wide character",
            .inprogress => "operation in progress",
            .intr => "interrupted function call",
            .inval => "invalid argument",
            .io => "input/output error",
            .isconn => "socket is connected",
            .isdir => "is a directory",
            .isnam => "is a named type file",
            .keyexpired => "key has expired",
            .keyrejected => "key was rejected by service",
            .keyrevoked => "key has been revoked",
            .l2hlt => "level 2 halted",
            .l2nsync => "level 2 not synchronized",
            .l3hlt => "level 3 halted",
            .l3rst => "level 3 reset",
            .libacc => "cannot access a needed shared library",
            .libbad => "accessing a corrupted shared library",
            .libmax => "attempting to link in too many shared libraries",
            .libscn => ".lib section in a.out corrupted",
            .libexec => "cannot exec a shared library directly",
            .lnrng => "link number out of range",
            .loop => "too many levels of symbolic links",
            .mediumtype => "wrong medium type",
            .mfile => "too many open files",
            .mlink => "too many links",
            .msgsize => "message too long",
            .multihop => "multihop attempted",
            .nametoolong => "filename too long",
            .netdown => "network is down",
            .netreset => "connection aborted by network",
            .netunreach => "network unreachable",
            .nfile => "too many open files in system",
            .noano => "no anode",
            .nobufs => "no buffer space available",
            .nodata => "the named attribute does not exist, or the process has no access to this attribute",
            .nodev => "no such device",
            .noent => "no such file or directory",
            .noexec => "exec format error",
            .nokey => "required key not available",
            .nolck => "no locks available",
            .nolink => "link has been severed",
            .nomedium => "no medium found",
            .nomem => "not enough space/cannot allocate memory",
            .nomsg => "no message of the desired type",
            .nonet => "machine is not on the network",
            .nopkg => "package not installed",
            .noprotoopt => "protocol not available",
            .nospc => "no space left on device",
            .nosr => "no STREAM resources",
            .nostr => "not a STREAM",
            .nosys => "function not implemented",
            .notblk => "block device required",
            .notconn => "the socket is not connected",
            .notdir => "not a directory",
            .notempty => "directory not empty",
            .notrecoverable => "state not recoverable",
            .notsock => "not a socket",
            .notsup => "operation not supported",
            .notty => "inappropriate I/O control operation",
            .notuniq => "name not unique on network",
            .nxio => "no such device or address",
            .opnotsupp => "operation not supported on socket",
            .overflow => "value too large to be stored in data type",
            .ownerdead => "owner died",
            .perm => "operation not permitted",
            .pfnosupport => "protocol family not supported",
            .pipe => "broken pipe",
            .proto => "protocol error",
            .protonosupport => "protocol not supported",
            .prototype => "protocol wrong type for socket",
            .range => "result too large",
            .remchg => "remote address changed",
            .remote => "object is remote",
            .remoteio => "remote I/O error",
            .restart => "interrupted system call should be restarted",
            .rfkill => "operation not possible due to RF-kill",
            .rofs => "read-only filesystem",
            .shutdown => "cannot send after transport endpoint shutdown",
            .spipe => "invalid seek",
            .socktnosupport => "socket type not supported",
            .srch => "no such process",
            .stale => "stale file handle",
            .strpipe => "streams pipe error",
            .time => "timer expired",
            .timedout => "connection timed out",
            .toomanyrefs => "too many references: cannot splice",
            .txtbsy => "text file busy",
            .uclean => "structure needs cleaning",
            .unatch => "protocol driver not attached",
            .users => "too many users",
            .wouldblock => "operation would block",
            .xdev => "invalid cross-device link",
            .xfull => "exchange full",
        };
    }

    /// Formats the error code.
    ///
    /// # Format specifiers
    ///
    /// * `{}`: Prints the error code description.
    /// * `{dbg}`: Prints the error code name.
    pub fn format(
        self: ErrorCode,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        writer: anytype,
    ) !void {
        _ = options;

        const debug = comptime parse_fmt: {
            if (fmt.len == 0) {
                break :parse_fmt false;
            } else if (std.mem.eql(u8, fmt, "dbg")) {
                break :parse_fmt true;
            } else {
                @compileError("expected {}, or {dbg}, found {" ++ fmt ++ "}");
            }
        };

        const string = if (debug) self.name() else self.description();
        try writer.writeAll(string);
    }

    export fn fimo_error_code_name(code: c.FimoErrorCode) [*:0]const u8 {
        if (code >= @typeInfo(ErrorCode).@"enum".fields.len) {
            return "FIMO_ERROR_CODE_UNKNOWN";
        }
        return @as(ErrorCode, @enumFromInt(code)).name();
    }

    export fn fimo_error_code_description(code: c.FimoErrorCode) [*:0]const u8 {
        if (code >= @typeInfo(ErrorCode).@"enum".fields.len) {
            return "unknown error code";
        }
        return @as(ErrorCode, @enumFromInt(code)).description();
    }
};

/// A system error code.
pub const SystemErrorCode = switch (builtin.target.os.tag) {
    .windows => std.os.windows.DWORD,
    else => c_int,
};

/// An owned string originating from an error.
pub const ErrorString = struct {
    str: c.FimoResultString,

    // List of known allocators that we specialize against.
    const known_allocators = .{
        heap.fimo_allocator,
        std.heap.c_allocator,
        std.heap.raw_c_allocator,
        std.heap.page_allocator,
    };

    /// Initializes the string with a constant string.
    ///
    /// The string is assumed to have a static lifetime.
    pub fn init(str: [*:0]const u8) ErrorString {
        return ErrorString{ .str = .{
            .str = str,
            .release = null,
        } };
    }

    test init {
        const err = "error message";
        const error_string = ErrorString.init(err);
        defer error_string.deinit();
        try std.testing.expect(std.mem.eql(u8, error_string.string(), err));
    }

    /// Initializes the error string by duplicating an existing string.
    pub fn initDupe(allocator: std.mem.Allocator, err: []const u8) !ErrorString {
        if (@inComptime()) {
            return ErrorString.dupeComptime(allocator, err);
        } else {
            inline for (known_allocators) |all| {
                if (all.vtable == allocator.vtable) {
                    return ErrorString.dupeComptime(all, err);
                }
            }

            const free_func = struct {
                fn free(ptr_c: [*c]const u8) callconv(.C) void {
                    const ptr: [*:0]u8 = @ptrCast(@constCast(ptr_c));
                    const buffer_begin = ptr - @sizeOf(std.mem.Allocator);
                    const all = std.mem.bytesToValue(
                        std.mem.Allocator,
                        buffer_begin[0..@sizeOf(std.mem.Allocator)],
                    );
                    const buffer_len = @sizeOf(std.mem.Allocator) + std.mem.len(ptr);
                    all.free(buffer_begin[0..buffer_len :0]);
                }
            }.free;
            const dupe = try std.mem.concatWithSentinel(
                allocator,
                u8,
                &.{ std.mem.asBytes(&allocator), err },
                0,
            );
            return ErrorString{ .str = .{
                .str = dupe[@sizeOf(std.mem.Allocator)..].ptr,
                .release = free_func,
            } };
        }
    }

    fn dupeComptime(comptime allocator: std.mem.Allocator, err: []const u8) !ErrorString {
        if (@inComptime()) {
            return ErrorString.init(err ++ "\x00");
        } else {
            const free_func = struct {
                fn free(ptr_c: [*c]const u8) callconv(.C) void {
                    const ptr: [*:0]u8 = @ptrCast(@constCast(ptr_c));
                    const buff = std.mem.span(ptr);
                    allocator.free(buff);
                }
            }.free;
            const dupe = try allocator.dupeZ(u8, err);
            return ErrorString{ .str = .{
                .str = dupe.ptr,
                .release = free_func,
            } };
        }
    }

    test initDupe {
        comptime {
            const err = "error message";
            const error_string = try ErrorString.initDupe(std.testing.allocator, err);
            defer error_string.deinit();
            try std.testing.expect(std.mem.eql(u8, error_string.string(), err));
            try std.testing.expect(error_string.str.release == null);
        }

        const err = "error message";
        const error_string = try ErrorString.initDupe(std.testing.allocator, err);
        defer error_string.deinit();
        try std.testing.expect(std.mem.eql(u8, error_string.string(), err));
    }

    /// Initializes the error string by rendering the provided arguments with the format template.
    pub fn initFmt(allocator: std.mem.Allocator, comptime fmt: []const u8, args: anytype) !ErrorString {
        if (@inComptime()) {
            return ErrorString.fmtComptime(allocator, fmt, args);
        } else {
            inline for (known_allocators) |all| {
                if (all.vtable == allocator.vtable) {
                    return ErrorString.fmtComptime(all, fmt, args);
                }
            }

            const free_func = struct {
                fn free(ptr_c: [*c]const u8) callconv(.C) void {
                    const ptr: [*:0]u8 = @ptrCast(@constCast(ptr_c));
                    const buffer_begin = ptr - @sizeOf(std.mem.Allocator);
                    const all = std.mem.bytesToValue(
                        std.mem.Allocator,
                        buffer_begin[0..@sizeOf(std.mem.Allocator)],
                    );
                    const buffer_len = @sizeOf(std.mem.Allocator) + std.mem.len(ptr);
                    all.free(buffer_begin[0..buffer_len :0]);
                }
            }.free;
            const buff = try std.fmt.allocPrintZ(
                allocator,
                std.mem.zeroes([@sizeOf(std.mem.Allocator)]u8) ++ fmt,
                args,
            );
            @as(*align(1) std.mem.Allocator, @ptrCast(buff.ptr)).* = allocator;
            return ErrorString{ .str = .{
                .str = buff[@sizeOf(std.mem.Allocator)..].ptr,
                .release = free_func,
            } };
        }
    }

    fn fmtComptime(comptime allocator: std.mem.Allocator, comptime fmt: []const u8, args: anytype) !ErrorString {
        if (@inComptime()) {
            const err = std.fmt.comptimePrint(fmt, args);
            return ErrorString.init(err);
        } else {
            const free_func = struct {
                fn free(ptr_c: [*c]const u8) callconv(.C) void {
                    const ptr: [*:0]u8 = @ptrCast(@constCast(ptr_c));
                    const buff = std.mem.span(ptr);
                    allocator.free(buff);
                }
            }.free;
            const buff = try std.fmt.allocPrintZ(allocator, fmt, args);
            return ErrorString{ .str = .{
                .str = buff.ptr,
                .release = free_func,
            } };
        }
    }

    test initFmt {
        const err_fmt = "error: {}";
        const err = std.fmt.comptimePrint(err_fmt, .{5});

        comptime {
            const error_string = try ErrorString.initFmt(
                std.testing.allocator,
                err_fmt,
                .{5},
            );
            defer error_string.deinit();
            try std.testing.expect(std.mem.eql(u8, error_string.string(), err));
            try std.testing.expect(error_string.str.release == null);
        }

        const error_string = try ErrorString.initFmt(
            std.testing.allocator,
            err_fmt,
            .{5},
        );
        defer error_string.deinit();
        try std.testing.expect(std.mem.eql(u8, error_string.string(), err));
    }

    /// Releases the error string.
    pub fn deinit(self: ErrorString) void {
        if (self.str.release) |release| {
            release(self.str.str);
        }
    }

    /// Extracts the contained string.
    pub fn string(self: *const ErrorString) [:0]const u8 {
        return std.mem.span(self.str.str);
    }

    /// Formats the string.
    pub fn format(
        self: *const ErrorString,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        writer: anytype,
    ) !void {
        _ = fmt;
        _ = options;
        try writer.print("{s}", .{self.string()});
    }
};
