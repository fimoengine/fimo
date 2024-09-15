const std = @import("std");
const builtin = @import("builtin");

const c = @import("c.zig");
const heap = @import("heap.zig");

/// Posix error codes.
pub const ErrorCode = enum(c_int) {
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
};

/// A system error code.
pub const SystemErrorCode = switch (builtin.target.os.tag) {
    .windows => std.os.windows.DWORD,
    else => c_int,
};

/// Representation of an opaque error type.
pub const Error = struct {
    /// Contained error. Must always represent an actual error.
    err: c.FimoResult,

    const error_code_vtable = c.FimoResultVTable{
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

    /// Creates an optional error from an error code.
    pub fn initErrorCode(code: ErrorCode) ?Error {
        if (code == .ok) return null;
        const code_int: usize = @intCast(@intFromEnum(code));
        return Error{ .err = .{
            .data = @ptrFromInt(code_int),
            .vtable = &error_code_vtable,
        } };
    }

    /// Creates an optional error from the c result.
    pub fn initC(err: c.FimoResult) ?Error {
        if (c.fimo_result_is_ok(err)) return null;
        return Error{ .err = err };
    }

    /// Cleans up the error.
    pub fn deinit(self: Error) void {
        c.fimo_result_release(self.err);
    }

    /// Unwraps the optional error to the c equivalent.
    pub fn intoCResult(self: ?Error) c.FimoResult {
        if (self) |v| return v.err;
        return .{ .data = null, .vtable = null };
    }

    /// Returns the name string of the error.
    pub fn name(self: *const Error) ErrorString {
        const str = self.err.vtable.*.v0.error_name.?(self.err.data);
        return ErrorString{ .str = str };
    }

    /// Returns the description string of the error.
    pub fn description(self: *const Error) ErrorString {
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
        self: *const Error,
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
};

/// An owned string originating from an error.
pub const ErrorString = struct {
    str: c.FimoResultString,

    /// Initializes the string with a constant string.
    pub fn init(str: [*:0]const u8) ErrorString {
        return ErrorString{ .str = .{ .str = str, .release = null } };
    }

    /// Initializes the string from a dynamically allocated string and a release function.
    pub fn initEX(str: [*:0]u8, release: *fn ([*:0]u8) callconv(.C) void) ErrorString {
        const release_c: *fn ([*:0]const u8) callconv(.C) void = @ptrCast(release);
        return ErrorString{ .str = .{ .str = str, .release = release_c } };
    }

    /// Releases the error string.
    pub fn deinit(self: ErrorString) void {
        if (self.str.release) |release| {
            const str = @constCast(self.str.str);
            const rel: *fn ([*:0]u8) callconv(.C) void = @constCast(release);
            rel(str);
        }
    }

    /// Extracts the contained string.
    pub fn string(self: *const ErrorString) [*:0]const u8 {
        return self.str.str;
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

test "ErrorCode name" {
    inline for (@typeInfo(ErrorCode).@"enum".fields) |field| {
        const error_code: ErrorCode = @enumFromInt(field.value);
        std.debug.print("{dbg}\n", .{error_code});
        try std.testing.expect(std.mem.eql(u8, error_code.name(), field.name));
    }
}

test "ErrorCode description" {
    inline for (@typeInfo(ErrorCode).@"enum".fields) |field| {
        const error_code: ErrorCode = @enumFromInt(field.value);
        std.debug.print("{}\n", .{error_code});
    }
}

test "Error from ErrorCode" {
    const eok = Error.initErrorCode(.ok);
    try std.testing.expect(eok == null);

    const einval = Error.initErrorCode(.inval);
    try std.testing.expect(einval != null);

    const name = einval.?.name();
    defer name.deinit();
    std.debug.print("{}\n", .{name});

    const description = einval.?.description();
    defer description.deinit();
    std.debug.print("{}\n", .{description});
}
