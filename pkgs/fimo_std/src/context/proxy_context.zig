//! Public interface to the fimo std context.
const std = @import("std");

const c = @import("c");
const context_version_opt = @import("context_version");

const AnyError = @import("../AnyError.zig");
const AnyResult = AnyError.AnyResult;
const Inner = @import("../context.zig");
const Version = @import("../Version.zig");
pub const Async = @import("proxy_context/async.zig");
pub const Module = @import("proxy_context/module.zig");
pub const Tracing = @import("proxy_context/tracing.zig");

data: *anyopaque,
vtable: *const VTable,

const Self = @This();

comptime {
    _ = Async;
    _ = Tracing;
    _ = Module;
}

/// Interface version compiled against.
pub const context_version = Version.initSemanticVersion(context_version_opt.version);

/// Id of the fimo std interface types.
pub const TypeId = enum(i32) {
    tracing_config,
    module_config,
    _,
};

/// Base structure for a read-only pointer chain.
pub const TaggedInStruct = extern struct {
    id: TypeId,
    next: ?*const TaggedInStruct,
};

/// Base structure for a pointer chain.
pub const TaggedOutStruct = extern struct {
    id: TypeId,
    next: ?*TaggedOutStruct,
};

/// Status code.
///
/// All positive values are interpreted as successfull operations.
pub const Status = enum(i32) {
    /// Operation completed successfully
    ok = 0,
    /// Operation failed with an unspecified error.
    ///
    /// The specific error may be accessible through the context.
    err = -1,
    _,

    /// Checks if the status indicates a success.
    pub fn is_ok(self: Status) bool {
        return @intFromEnum(self) >= 0;
    }

    /// Checks if the status indicates an error.
    pub fn is_err(self: Status) bool {
        return @intFromEnum(self) < 0;
    }
};

/// VTable of the public fimo std context.
pub const VTable = extern struct {
    header: CompatibilityContext.VTable,
    core_v0: CoreVTable,
    tracing_v0: Tracing.VTable,
    module_v0: Module.VTable,
    async_v0: Async.VTable,
};

/// Initial VTable of the context.
///
/// Changing this definition is a breaking change.
pub const CoreVTable = extern struct {
    acquire: *const fn (ctx: *anyopaque) callconv(.c) void,
    release: *const fn (ctx: *anyopaque) callconv(.c) void,
    has_error_result: *const fn (ctx: *anyopaque) callconv(.c) bool,
    replace_result: *const fn (ctx: *anyopaque, new: AnyResult) callconv(.c) AnyResult,
};

/// Minimal interface of the fimo std context.
pub const CompatibilityContext = struct {
    data: *anyopaque,
    vtable: *const CompatibilityContext.VTable,

    /// VTable of the minimal fimo std interface.
    pub const VTable = extern struct {
        check_version: *const fn (
            ctx: *anyopaque,
            version: *const c.FimoVersion,
        ) callconv(.C) AnyResult,
    };

    /// Initializes the object from a ffi object.
    pub fn initC(obj: c.FimoContext) @This() {
        return @This(){
            .data = obj.data.?,
            .vtable = @alignCast(@ptrCast(obj.vtable)),
        };
    }

    /// Casts the object to a ffi object.
    pub fn intoC(self: @This()) c.FimoContext {
        return c.FimoContext{
            .data = self.data,
            .vtable = @ptrCast(self.vtable),
        };
    }

    /// Casts the minimal interface to the full interface,
    /// making shure that the implementation supports the required version.
    pub fn castToContext(self: CompatibilityContext) error{VersionNotSupported}!Self {
        return if (self.isCompatibleWithVersion(context_version))
            .{
                .data = self.data,
                .vtable = @alignCast(@ptrCast(self.vtable)),
            }
        else
            return error.VersionNotSupported;
    }

    /// Checks whether the context is compatible with the specified interface version.
    pub fn isCompatibleWithVersion(self: CompatibilityContext, version: Version) bool {
        const v = c.FimoVersion{
            .major = version.major,
            .minor = version.minor,
            .patch = version.patch,
            .build = version.build,
        };
        const result = self.vtable.check_version(self.data, &v);
        defer result.deinit();
        return result.isOk();
    }
};

/// Initializes a new context with the given options.
///
/// In case of an error, this function cleans up the configuration options.
pub fn init(options: [:null]const ?*const TaggedInStruct) !Self {
    const inner = try Inner.init(options);
    return inner.asProxy();
}

/// Initializes the object from a ffi object.
pub fn initC(obj: c.FimoContext) Self {
    return Self{
        .data = obj.data.?,
        .vtable = @alignCast(@ptrCast(obj.vtable)),
    };
}

/// Casts the object to a ffi object.
pub fn intoC(self: Self) c.FimoContext {
    return c.FimoContext{
        .data = self.data,
        .vtable = @ptrCast(self.vtable),
    };
}

/// Checks whether the context is compatible with the specified interface version.
pub fn isCompatibleWithVersion(self: Self, version: Version) bool {
    const v = .{
        .major = version.major,
        .minor = version.minor,
        .patch = version.patch,
        .build = version.build,
    };
    const result = self.vtable.check_version(self.data, &v);
    defer result.deinit();
    return result.isOk();
}

/// Acquires a reference to the context.
///
/// Increases the reference count of the context. May abort the program,
/// if doing so is not possible. May only be called with a valid reference
/// to the context.
pub fn ref(self: Self) void {
    self.vtable.core_v0.acquire(self.data);
}

/// Releases a reference to the context.
///
/// Decrements the reference count of the context. When the reference count
/// reaches zero, this function also destroys the reference. May only be
/// called with a valid reference to the context.
pub fn unref(self: Self) void {
    self.vtable.core_v0.release(self.data);
}

/// Checks whether the context has an error stored for the current thread.
pub fn hasErrorResult(self: Self) bool {
    return self.vtable.core_v0.has_error_result(self.data);
}

/// Replaces the thread-local result stored in the context with a new one.
///
/// The old result is returned.
pub fn replaceResult(self: Self, new: AnyResult) AnyResult {
    return self.vtable.core_v0.replace_result(self.data, new);
}

/// Swaps out the thread-local result with the `ok` result.
pub fn takeResult(self: Self) AnyResult {
    return self.replaceResult(.ok);
}

/// Sets the thread-local result, destroying the old one.
pub fn setResult(self: Self, new: AnyResult) void {
    self.replaceResult(new).deinit();
}

/// Returns the interface to the tracing subsystem.
pub fn tracing(self: Self) Tracing {
    return Tracing{ .context = self };
}

/// Returns the interface to the module subsystem.
pub fn module(self: Self) Module {
    return Module{ .context = self };
}

/// Returns the interface to the async subsystem.
pub fn @"async"(self: Self) Async {
    return Async{ .context = self };
}

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    export fn fimo_context_init(options: [*:null]const ?*const TaggedInStruct, context: *c.FimoContext) AnyResult {
        const ctx = Self.init(std.mem.span(options)) catch |err| return AnyError.initError(err).intoResult();
        context.* = ctx.intoC();
        return AnyResult.ok;
    }
};

comptime {
    _ = ffi;
}

test "proxy_context: local error" {
    const ctx = try Self.init(&.{});
    defer ctx.unref();

    try std.testing.expect(!ctx.hasErrorResult());
    try std.testing.expectEqual(
        ctx.replaceResult(.initErr(AnyError.initError(error.FfiError))),
        AnyResult.ok,
    );
    try std.testing.expect(ctx.hasErrorResult());

    const Runner = struct {
        ctx: Self,
        thread: std.Thread,
        err: ?anyerror = null,

        fn run(self: *@This()) !void {
            errdefer |err| self.err = err;
            try std.testing.expect(!self.ctx.hasErrorResult());
            try std.testing.expectEqual(
                self.ctx.replaceResult(.initErr(AnyError.initError(error.FfiError))),
                AnyResult.ok,
            );
            try std.testing.expect(self.ctx.hasErrorResult());
        }
    };
    var runner = Runner{ .ctx = ctx, .thread = undefined };
    runner.thread = try std.Thread.spawn(.{}, Runner.run, .{&runner});
    runner.thread.join();
    if (runner.err) |err| return err;
}
