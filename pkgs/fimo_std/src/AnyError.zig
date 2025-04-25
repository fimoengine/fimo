//! Representation of an opaque error type.

const std = @import("std");
const builtin = @import("builtin");

const c = @import("c");

const Self = @This();

data: ?*anyopaque,
vtable: *const VTable,

/// Errors deriving from an ffi call.
pub const Error = error{FfiError};

/// VTable of an `AnyError` and `AnyResult`.
///
/// Adding fields to the vtable is a breaking change.
pub const VTable = extern struct {
    deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
    name: *const fn (data: ?*anyopaque) callconv(.c) ErrorString,
    description: *const fn (data: ?*anyopaque) callconv(.c) ErrorString,
};

const anyerror_vtable = VTable{
    .deinit = null,
    .name = anyerror_string,
    .description = anyerror_string,
};
fn anyerror_string(ptr: ?*anyopaque) callconv(.C) ErrorString {
    const err_int: std.meta.Int(.unsigned, @bitSizeOf(anyerror)) = @intCast(@intFromPtr(ptr));
    const err = @errorFromInt(err_int);
    return ErrorString.init(@errorName(err));
}

// Declared in the C header.
export const FIMO_IMPL_RESULT_STATIC_STRING_VTABLE = VTable{
    .deinit = null,
    .name = static_string_string,
    .description = static_string_string,
};
fn static_string_string(ptr: ?*anyopaque) callconv(.C) ErrorString {
    const str: [*:0]const u8 = @constCast(@alignCast(@ptrCast(ptr.?)));
    return ErrorString.init(str);
}

// Declared in the C header.
export const FIMO_IMPL_RESULT_ERROR_CODE_VTABLE = VTable{
    .deinit = null,
    .name = error_code_name,
    .description = error_code_description,
};
fn error_code_name(ptr: ?*anyopaque) callconv(.C) ErrorString {
    const code: ErrorCode = @enumFromInt(@intFromPtr(ptr));
    return ErrorString.init(code.name());
}
fn error_code_description(ptr: ?*anyopaque) callconv(.C) ErrorString {
    const code: ErrorCode = @enumFromInt(@intFromPtr(ptr));
    return ErrorString.init(code.description());
}

// Declared in the C header.
export const FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE = VTable{
    .deinit = null,
    .name = system_error_code_name,
    .description = system_error_code_description,
};
fn system_error_code_name(ptr: ?*anyopaque) callconv(.C) ErrorString {
    const code: SystemErrorCode = @intCast(@intFromPtr(ptr));
    return ErrorString.initFmt(std.heap.c_allocator, "SystemError({})", .{code}) catch |e|
        Self.initError(e).name();
}
fn system_error_code_description(ptr: ?*anyopaque) callconv(.C) ErrorString {
    const code: SystemErrorCode = @intCast(@intFromPtr(ptr));
    switch (builtin.target.os.tag) {
        .windows => {
            const FreeImpl = struct {
                fn free(p: [*:0]const u8) callconv(.c) void {
                    if (LocalFree(@constCast(p)) != null) unreachable;
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
                return ErrorString.init("SystemError(\"unknown error\")");
            }
            // Remove the trailing `\r\n` characters.
            error_description.?[format_result - 2] = 0;
            return ErrorString{
                .data = @ptrCast(@constCast(error_description)),
                .deinit_fn = FreeImpl.free,
            };
        },
        else => {
            const errno: std.posix.E = @enumFromInt(code);
            return ErrorString.init(@tagName(errno));
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
) callconv(.winapi) std.os.windows.DWORD;
extern fn LocalFree(in: ?std.os.windows.HLOCAL) callconv(.winapi) ?std.os.windows.HLOCAL;

// Declared in the C header.
export const FIMO_IMPL_RESULT_OK = AnyResult.ok.intoC();
export const FIMO_IMPL_RESULT_INVALID_ERROR = AnyResult{
    .data = @ptrCast(@constCast(@as([*:0]const u8, "invalid error"))),
    .vtable = &FIMO_IMPL_RESULT_STATIC_STRING_VTABLE,
};
export const FIMO_IMPL_RESULT_OK_NAME = ErrorString.init("ok");
export const FIMO_IMPL_RESULT_OK_DESCRIPTION = ErrorString.init("ok");

/// An optional error.
pub const AnyResult = extern struct {
    data: ?*anyopaque,
    vtable: ?*const VTable,

    /// A value representing no error.
    pub const ok = AnyResult{
        .data = null,
        .vtable = null,
    };

    /// Initializes a result from an `AnyError`.
    pub fn initErr(err: Self) AnyResult {
        return AnyResult{
            .data = err.data,
            .vtable = err.vtable,
        };
    }

    /// Creates a result from the c result.
    pub fn initC(err: c.FimoResult) AnyResult {
        return AnyResult{
            .data = err.data,
            .vtable = @ptrCast(@alignCast(err.vtable)),
        };
    }

    /// Deinitializes the result.
    pub fn deinit(self: AnyResult) void {
        if (self.vtable) |vtable| {
            if (vtable.deinit) |f| f(self.data);
        }
    }

    /// Unwraps the result to a c result.
    pub fn intoC(self: AnyResult) c.FimoResult {
        return c.FimoResult{
            .data = self.data,
            .vtable = @ptrCast(@alignCast(self.vtable)),
        };
    }

    /// Returns whether the result is not an error.
    pub fn isOk(self: *const AnyResult) bool {
        return self.vtable == null;
    }

    /// Returns whether the result is an error.
    pub fn isErr(self: *const AnyResult) bool {
        return self.vtable != null;
    }

    /// Unwraps the contained `AnyError`.
    pub fn unwrapErr(self: AnyResult) Self {
        return Self{ .data = self.data, .vtable = self.vtable.? };
    }

    /// Constructs an error union from the `AnyResult`.
    pub fn intoErrorUnion(self: AnyResult, err: *?Self) Error!void {
        if (self.isOk()) return;
        err.* = self.unwrapErr();
        return Error.FfiError;
    }

    /// Returns the name string of the result.
    pub fn name(self: *const AnyResult) ErrorString {
        if (self.vtable) |vtable| return vtable.name(self.data);
        return FIMO_IMPL_RESULT_OK_NAME;
    }

    /// Returns the description string of the result.
    pub fn description(self: *const AnyResult) ErrorString {
        if (self.vtable) |vtable| return vtable.description(self.data);
        return FIMO_IMPL_RESULT_OK_DESCRIPTION;
    }

    /// Formats the result.
    ///
    /// # Format specifiers
    ///
    /// * `{}`: Prints the result description.
    /// * `{dbg}`: Prints the result name.
    pub fn format(
        self: *const AnyResult,
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
        if (self.isOk())
            try writer.print("AnyResult.ok(\"{}\")", .{string.string()})
        else
            try writer.print("AnyResult.err(\"{}\")", .{string.string()});
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
    return Self{ .data = @ptrFromInt(code_int), .vtable = &FIMO_IMPL_RESULT_ERROR_CODE_VTABLE };
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
    return Self{
        .data = @ptrFromInt(@as(usize, @intCast(code))),
        .vtable = &FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE,
    };
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

/// Creates an error from the c result.
pub fn initC(err: c.FimoResult) Self {
    if (c.fimo_result_is_ok(err)) unreachable;
    return Self{ .data = err.data, .vtable = @ptrCast(@alignCast(err.vtable)) };
}

/// Cleans up the error.
pub fn deinit(self: Self) void {
    if (self.vtable.deinit) |f| f(self.data);
}

/// Constructs an `AnyResult` from the error.
pub fn intoResult(self: Self) AnyResult {
    return AnyResult.initErr(self);
}

/// Unwraps the optional error to the c equivalent.
pub fn intoC(self: Self) c.FimoResult {
    return .{ .data = self.data, .vtable = @ptrCast(@alignCast(self.vtable)) };
}

/// Returns the name string of the error.
pub fn name(self: *const Self) ErrorString {
    return self.vtable.name(self.data);
}

/// Returns the description string of the error.
pub fn description(self: *const Self) ErrorString {
    return self.vtable.description(self.data);
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
pub const ErrorString = extern struct {
    data: [*:0]const u8,
    deinit_fn: ?*const fn (data: [*:0]const u8) callconv(.c) void,

    // List of known allocators that we specialize against.
    const known_allocators = .{
        std.heap.c_allocator,
        std.heap.raw_c_allocator,
        std.heap.page_allocator,
    };

    /// Initializes the string with a constant string.
    ///
    /// The string is assumed to have a static lifetime.
    pub fn init(str: [*:0]const u8) ErrorString {
        return ErrorString{ .data = str, .deinit_fn = null };
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
            return ErrorString{
                .data = dupe[@sizeOf(std.mem.Allocator)..].ptr,
                .deinit_fn = free_func,
            };
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
            return ErrorString{
                .data = dupe.ptr,
                .deinit_fn = free_func,
            };
        }
    }

    test initDupe {
        comptime {
            const err = "error message";
            const error_string = try ErrorString.initDupe(std.testing.allocator, err);
            defer error_string.deinit();
            try std.testing.expect(std.mem.eql(u8, error_string.string(), err));
            try std.testing.expect(error_string.deinit_fn == null);
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
            return ErrorString{
                .data = buff[@sizeOf(std.mem.Allocator)..].ptr,
                .deinit_fn = free_func,
            };
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
            return ErrorString{
                .data = buff.ptr,
                .deinit_fn = free_func,
            };
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
            try std.testing.expect(error_string.deinit_fn == null);
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
        if (self.deinit_fn) |f| f(self.data);
    }

    /// Extracts the contained string.
    pub fn string(self: *const ErrorString) [:0]const u8 {
        return std.mem.span(self.data);
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
