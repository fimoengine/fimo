//! Public interface to the fimo std context.
const std = @import("std");

const c = @import("../c.zig");
const errors = @import("../errors.zig");
const Error = errors.Error;
const Version = @import("../version.zig");

data: *anyopaque,
vtable: *const VTable,

const Context = @This();
pub const Tracing = @import("proxy_context/tracing.zig");
pub const Module = @import("proxy_context/module.zig");
const Self = @This();

comptime {
    _ = Tracing;
    _ = Module;
}

/// Interface version compiled against.
pub const context_version = Version{
    .major = c.FIMO_VERSION_MAJOR,
    .minor = c.FIMO_VERSION_MINOR,
    .patch = c.FIMO_VERSION_PATCH,
    .build = c.FIMO_VERSION_BUILD_NUMBER,
};

/// Id of the fimo std interface types.
pub const TypeId = enum(c.FimoStructType) {
    tracing_creation_config = c.FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG,
    tracing_metadata = c.FIMO_STRUCT_TYPE_TRACING_METADATA,
    tracing_span_desc = c.FIMO_STRUCT_TYPE_TRACING_SPAN_DESC,
    tracing_span = c.FIMO_STRUCT_TYPE_TRACING_SPAN,
    tracing_event = c.FIMO_STRUCT_TYPE_TRACING_EVENT,
    tracing_subscriber = c.FIMO_STRUCT_TYPE_TRACING_SUBSCRIBER,
    module_export = c.FIMO_STRUCT_TYPE_MODULE_EXPORT,
    module_info = c.FIMO_STRUCT_TYPE_MODULE_INFO,
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

/// VTable of the public fimo std context.
pub const VTable = extern struct {
    header: CompatibilityContext.VTable,
    core_v0: CoreVTable,
    tracing_v0: Tracing.VTable,
    module_v0: Module.VTable,
};

/// Initial VTable of the context.
///
/// Changing this definition is a breaking change.
pub const CoreVTable = extern struct {
    acquire: *const fn (ctx: *anyopaque) callconv(.C) void,
    release: *const fn (ctx: *anyopaque) callconv(.C) void,
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
        ) callconv(.C) c.FimoResult,
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
        const err = Error.initC(self.vtable.check_version(self.data, &v));
        defer if (err) |e| e.deinit();
        return err == null;
    }
};

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
    const err = Error.initC(self.vtable.check_version(self.data, &v));
    defer if (err) |e| e.deinit();
    return err == null;
}

/// Acquires a reference to the context.
///
/// Increases the reference count of the context. May abort the program,
/// if doing so is not possible. May only be called with a valid reference
/// to the context.
pub fn acquire(self: Self) void {
    self.vtable.core_v0.acquire(self.data);
}

/// Releases a reference to the context.
///
/// Decrements the reference count of the context. When the reference count
/// reaches zero, this function also destroys the reference. May only be
/// called with a valid reference to the context.
pub fn release(self: Self) void {
    self.vtable.core_v0.release(self.data);
}

/// Returns the interface to the tracing subsystem.
pub fn tracing(self: Self) Tracing {
    return Tracing{ .context = self };
}

/// Returns the interface to the module subsystem.
pub fn module(self: Self) Module {
    return Module{ .context = self };
}

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    export fn fimo_context_check_version(context: c.FimoContext) c.FimoResult {
        const ctx = CompatibilityContext.initC(context);
        return if (!ctx.isCompatibleWithVersion(context_version))
            Error.initError(error.InvalidVersion).err
        else
            Error.intoCResult(null);
    }

    export fn fimo_context_acquire(context: c.FimoContext) void {
        const ctx = Self.initC(context);
        ctx.acquire();
    }

    export fn fimo_context_release(context: c.FimoContext) void {
        const ctx = Self.initC(context);
        ctx.release();
    }
};

comptime {
    _ = ffi;
}
