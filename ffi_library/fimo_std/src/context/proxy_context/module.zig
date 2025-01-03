//! Public interface of the module subsystem.

const std = @import("std");
const builtin = @import("builtin");

const AnyError = @import("../../AnyError.zig");
const c = @import("../../c.zig");
const Path = @import("../../path.zig").Path;
const Version = @import("../../Version.zig");
const Context = @import("../proxy_context.zig");

const EnqueuedFuture = Async.EnqueuedFuture;
const Fallible = Async.Fallible;

context: Context,

const Module = @This();
const Async = @import("async.zig");

pub const DebugInfo = @import("module/DebugInfo.zig");
pub const exports = @import("module/exports.zig");
pub const Export = exports.Export;

/// Data type of a module parameter.
pub const ParameterType = enum(c.FimoModuleParamType) {
    u8,
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
    _,
};

/// Access group for a module parameter.
pub const ParameterAccessGroup = enum(c.FimoModuleParamAccessGroup) {
    /// Parameter can be accessed publicly.
    public,
    /// Parameter can be accessed from dependent modules.
    dependency,
    /// Parameter can only be accessed from the owning module.
    private,
};

/// A typed parameter.
pub fn Parameter(comptime T: type, comptime InstanceT: type) type {
    std.debug.assert(
        std.mem.indexOfScalar(u16, &.{ 8, 16, 32, 64 }, @typeInfo(T).int.bits) != null,
    );
    return opaque {
        const Self = @This();

        fn unwrapValue(value: OpaqueParameter.Value) T {
            return switch (@typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) value.i8 else value.u8,
                16 => if (@typeInfo(T).int.signedness == .signed) value.i16 else value.u16,
                32 => if (@typeInfo(T).int.signedness == .signed) value.i32 else value.u32,
                64 => if (@typeInfo(T).int.signedness == .signed) value.i64 else value.u64,
                else => @compileError("Invalid parameter bit size"),
            };
        }
        fn wrapValue(value: T) OpaqueParameter.Value {
            return switch (@typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) .{ .i8 = value } else .{ .u8 = value },
                16 => if (@typeInfo(T).int.signedness == .signed) .{ .i16 = value } else .{ .u16 = value },
                32 => if (@typeInfo(T).int.signedness == .signed) .{ .i32 = value } else .{ .u32 = value },
                64 => if (@typeInfo(T).int.signedness == .signed) .{ .i64 = value } else .{ .u64 = value },
                else => @compileError("Invalid parameter bit size"),
            };
        }

        /// Reads a module parameter with public read access.
        ///
        /// Reads the value of a module parameter with public read access.
        /// The operation fails, if the parameter does not exist, or if
        /// the parameter does not allow reading with a public access.
        pub fn readPublic(
            ctx: Module,
            module: [:0]const u8,
            parameter: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!T {
            const value = try OpaqueParameter.readPublic(
                ctx,
                module,
                parameter,
                err,
            );
            return Self.unwrapValue(value);
        }

        /// Reads a module parameter with dependency read access.
        ///
        /// Reads the value of a module parameter with dependency read
        /// access. The operation fails, if the parameter does not exist,
        /// or if the parameter does not allow reading with a dependency
        /// access.
        pub fn readDependency(
            instance: *const InstanceT,
            module: [:0]const u8,
            parameter: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!T {
            const value = try OpaqueParameter.readDependency(
                instance.castOpaque(),
                module,
                parameter,
                err,
            );
            return Self.unwrapValue(value);
        }

        /// Getter for a module parameter.
        ///
        /// Reads the value of a module parameter with private read access.
        pub fn read(
            self: *const Self,
            instance: *const InstanceT,
            err: *?AnyError,
        ) AnyError.Error!T {
            const value = try self.castOpaqueConst().read(instance.castOpaque(), err);
            return Self.unwrapValue(value);
        }

        /// Sets a module parameter with public write access.
        ///
        /// Sets the value of a module parameter with public write access.
        /// The operation fails, if the parameter does not exist, or if
        /// the parameter does not allow writing with a public access.
        pub fn writePublic(
            ctx: Module,
            value: T,
            module: [:0]const u8,
            parameter: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!void {
            const v = Self.wrapValue(value);
            try OpaqueParameter.writePublic(ctx, v, module, parameter, err);
        }

        /// Sets a module parameter with dependency write access.
        ///
        /// Sets the value of a module parameter with dependency write
        /// access. The operation fails, if the parameter does not exist,
        /// or if the parameter does not allow writing with a dependency
        /// access.
        pub fn writeDependency(
            instance: *const InstanceT,
            value: T,
            module: [:0]const u8,
            parameter: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!void {
            const v = Self.wrapValue(value);
            try OpaqueParameter.writeDependency(
                instance.castOpaque(),
                v,
                module,
                parameter,
                err,
            );
        }

        /// Setter for a module parameter.
        ///
        /// Sets the value of a module parameter with private write access.
        pub fn write(
            self: *Self,
            instance: *const InstanceT,
            value: T,
            err: *?AnyError,
        ) AnyError.Error!void {
            const v = Self.wrapValue(value);
            try self.castOpaque().write(instance.castOpaque(), v, err);
        }

        /// Casts the parameter to an opaque parameter pointer.
        pub fn castOpaque(self: *@This()) *OpaqueParameter {
            return @ptrCast(self);
        }

        /// Casts the parameter to an opaque parameter pointer.
        pub fn castOpaqueConst(self: *const @This()) *const OpaqueParameter {
            return @ptrCast(self);
        }
    };
}

/// An opaque parameter.
pub const OpaqueParameter = opaque {
    /// Possible values contained in the parameter.
    pub const Value = OpaqueParameterData.Value;
    const ValueUntagged = OpaqueParameterData.ValueUntagged;

    /// Parameter info.
    pub const Info = struct {
        tag: ParameterType,
        read_group: ParameterAccessGroup,
        write_group: ParameterAccessGroup,
    };

    fn readPublicFfi(
        ctx: Module,
        value: *anyopaque,
        value_type: *ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) c.FimoResult {
        return ctx.context.vtable.module_v0.param_get_public(
            ctx.context.data,
            value,
            value_type,
            module,
            parameter,
        );
    }
    fn writePublicFfi(
        ctx: Module,
        value: *const anyopaque,
        value_type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) c.FimoResult {
        return ctx.context.vtable.module_v0.param_set_public(
            ctx.context.data,
            value,
            value_type,
            module,
            parameter,
        );
    }
    fn readDependencyFfi(
        instance: *const OpaqueInstance,
        value: *anyopaque,
        value_type: *ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) c.FimoResult {
        const ctx = Context.initC(instance.ctx);
        return ctx.vtable.module_v0.param_get_dependency(
            ctx.data,
            instance,
            value,
            value_type,
            module,
            parameter,
        );
    }
    fn writeDependencyFfi(
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        value_type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) c.FimoResult {
        const ctx = Context.initC(instance.ctx);
        return ctx.vtable.module_v0.param_set_dependency(
            ctx.data,
            instance,
            value,
            value_type,
            module,
            parameter,
        );
    }
    fn readPrivateFfi(
        self: *const OpaqueParameter,
        instance: *const OpaqueInstance,
        value: *anyopaque,
        value_type: *ParameterType,
    ) c.FimoResult {
        const ctx = Context.initC(instance.ctx);
        return ctx.vtable.module_v0.param_get_private(
            ctx.data,
            instance,
            value,
            value_type,
            self,
        );
    }
    fn writePrivateFfi(
        self: *OpaqueParameter,
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        value_type: ParameterType,
    ) c.FimoResult {
        const ctx = Context.initC(instance.ctx);
        return ctx.vtable.module_v0.param_set_private(
            ctx.data,
            instance,
            value,
            value_type,
            self,
        );
    }

    /// Queries the info of a module parameter.
    ///
    /// This function can be used to query the datatype, the read access,
    /// and the write access of a module parameter. This function fails,
    /// if the parameter can not be found.
    pub fn query(
        ctx: Module,
        module: [:0]const u8,
        parameter: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!OpaqueParameter.Info {
        var tag: ParameterType = undefined;
        var read_group: ParameterAccessGroup = undefined;
        var write_group: ParameterAccessGroup = undefined;
        const result = ctx.context.vtable.module_v0.param_query(
            ctx.context.data,
            module.ptr,
            parameter.ptr,
            &tag,
            &read_group,
            &write_group,
        );
        try AnyError.initChecked(err, result);
        return .{
            .tag = tag,
            .read_group = read_group,
            .write_group = write_group,
        };
    }

    /// Reads a module parameter with public read access.
    ///
    /// Reads the value of a module parameter with public read access.
    /// The operation fails, if the parameter does not exist, or if
    /// the parameter does not allow reading with a public access.
    pub fn readPublic(
        ctx: Module,
        module: [:0]const u8,
        parameter: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!OpaqueParameter.Value {
        var value: OpaqueParameter.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        const result = OpaqueParameter.readPublicFfi(
            ctx,
            &value,
            &value_type,
            module.ptr,
            parameter.ptr,
        );
        try AnyError.initChecked(err, result);
        return OpaqueParameter.Value.fromUntagged(value, value_type);
    }

    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read
    /// access. The operation fails, if the parameter does not exist,
    /// or if the parameter does not allow reading with a dependency
    /// access.
    pub fn readDependency(
        instance: *const OpaqueInstance,
        module: [:0]const u8,
        parameter: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!OpaqueParameter.Value {
        var value: OpaqueParameter.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        const result = OpaqueParameter.readDependencyFfi(
            instance,
            &value,
            &value_type,
            module.ptr,
            parameter.ptr,
        );
        try AnyError.initChecked(err, result);
        return OpaqueParameter.Value.fromUntagged(value, value_type);
    }

    /// Getter for a module parameter.
    ///
    /// Reads the value of a module parameter with private read access.
    pub fn read(
        self: *const OpaqueParameter,
        instance: *const OpaqueInstance,
        err: *?AnyError,
    ) AnyError.Error!OpaqueParameter.Value {
        var value: OpaqueParameter.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        const result = self.readPrivateFfi(
            instance,
            &value,
            &value_type,
        );
        try AnyError.initChecked(err, result);
        return OpaqueParameter.Value.fromUntagged(value, value_type);
    }

    /// Sets a module parameter with public write access.
    ///
    /// Sets the value of a module parameter with public write access.
    /// The operation fails, if the parameter does not exist, or if
    /// the parameter does not allow writing with a public access.
    pub fn writePublic(
        ctx: Module,
        value: OpaqueParameter.Value,
        module: [:0]const u8,
        parameter: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!void {
        var val: OpaqueParameter.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        value.toUntagged(&val, &value_type);
        const result = OpaqueParameter.writePublicFfi(
            ctx,
            &value,
            value_type,
            module.ptr,
            parameter.ptr,
        );
        try AnyError.initChecked(err, result);
    }

    /// Sets a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write
    /// access. The operation fails, if the parameter does not exist,
    /// or if the parameter does not allow writing with a dependency
    /// access.
    pub fn writeDependency(
        instance: *const OpaqueInstance,
        value: OpaqueParameter.Value,
        module: [:0]const u8,
        parameter: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!void {
        var val: OpaqueParameter.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        value.toUntagged(&val, &value_type);
        const result = OpaqueParameter.writeDependencyFfi(
            instance,
            &value,
            value_type,
            module.ptr,
            parameter.ptr,
        );
        try AnyError.initChecked(err, result);
    }

    /// Setter for a module parameter.
    ///
    /// Sets the value of a module parameter with private write access.
    pub fn write(
        self: *OpaqueParameter,
        instance: *const OpaqueInstance,
        value: OpaqueParameter.Value,
        err: *?AnyError,
    ) AnyError.Error!void {
        var val: OpaqueParameter.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        value.toUntagged(&val, &value_type);
        const result = self.writePrivateFfi(
            instance,
            &val,
            value_type,
        );
        try AnyError.initChecked(err, result);
    }
};

/// Typed internal state of a parameter.
pub fn ParameterData(comptime T: type, comptime InstanceT: type) type {
    std.debug.assert(
        std.mem.indexOfScalar(u16, &.{ 8, 16, 32, 64 }, @typeInfo(T).int.bits) != null,
    );
    return opaque {
        /// Internal getter for a module parameter.
        pub fn read(
            self: *const @This(),
            instance: *const InstanceT,
            err: *?AnyError,
        ) AnyError.Error!T {
            const value = try self.castOpaque().read(instance.castOpaque(), err);
            return switch (@typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) value.i8 else value.u8,
                16 => if (@typeInfo(T).int.signedness == .signed) value.i16 else value.u16,
                32 => if (@typeInfo(T).int.signedness == .signed) value.i32 else value.u32,
                64 => if (@typeInfo(T).int.signedness == .signed) value.i64 else value.u64,
            };
        }

        /// Internal setter for a module parameter.
        pub fn write(
            self: *@This(),
            instance: *const InstanceT,
            value: T,
            err: *?AnyError,
        ) AnyError.Error!void {
            const v: OpaqueParameterData.Value = switch (@typeInfo(T).int.bits) {
                8 => if (@typeInfo(T).int.signedness == .signed) .{ .i8 = value } else .{ .u8 = value },
                16 => if (@typeInfo(T).int.signedness == .signed) .{ .i16 = value } else .{ .u16 = value },
                32 => if (@typeInfo(T).int.signedness == .signed) .{ .i32 = value } else .{ .u32 = value },
                64 => if (@typeInfo(T).int.signedness == .signed) .{ .i64 = value } else .{ .u64 = value },
            };
            try self.castOpaque().write(instance.castOpaque(), v, err);
        }

        /// Casts the parameter data to an opaque parameter data pointer.
        pub fn castOpaque(self: *@This()) *OpaqueParameterData {
            return @ptrCast(self);
        }

        /// Casts the parameter data to an opaque parameter data pointer.
        pub fn castOpaqueConst(self: *const @This()) *const OpaqueParameterData {
            return @ptrCast(self);
        }
    };
}

/// Internal state of a parameter.
pub const OpaqueParameterData = opaque {
    /// Possible values contained in the parameter.
    pub const Value = union(enum) {
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,

        fn fromUntagged(untagged: ValueUntagged, value_type: ParameterType) Value {
            return switch (value_type) {
                .u8 => .{ .u8 = untagged.u8 },
                .u16 => .{ .u16 = untagged.u16 },
                .u32 => .{ .u32 = untagged.u32 },
                .u64 => .{ .u64 = untagged.u64 },
                .i8 => .{ .i8 = untagged.i8 },
                .i16 => .{ .i16 = untagged.i16 },
                .i32 => .{ .i32 = untagged.i32 },
                .i64 => .{ .i64 = untagged.i64 },
                else => @panic("unknown parameter type"),
            };
        }

        fn toUntagged(self: Value, untagged: *ValueUntagged, value_type: *ParameterType) void {
            switch (self) {
                .u8 => |v| {
                    untagged.* = .{ .u8 = v };
                    value_type.* = .u8;
                },
                .u16 => |v| {
                    untagged.* = .{ .u16 = v };
                    value_type.* = .u16;
                },
                .u32 => |v| {
                    untagged.* = .{ .u32 = v };
                    value_type.* = .u32;
                },
                .u64 => |v| {
                    untagged.* = .{ .u64 = v };
                    value_type.* = .u64;
                },
                .i8 => |v| {
                    untagged.* = .{ .i8 = v };
                    value_type.* = .i8;
                },
                .i16 => |v| {
                    untagged.* = .{ .i16 = v };
                    value_type.* = .i16;
                },
                .i32 => |v| {
                    untagged.* = .{ .i32 = v };
                    value_type.* = .i32;
                },
                .i64 => |v| {
                    untagged.* = .{ .i64 = v };
                    value_type.* = .i64;
                },
            }
        }
    };
    const ValueUntagged = extern union {
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,
    };

    /// Internal getter for the module parameter data.
    pub fn readFfi(
        instance: *const OpaqueInstance,
        value: *anyopaque,
        value_type: *ParameterType,
        self: *const OpaqueParameterData,
    ) callconv(.c) c.FimoResult {
        const ctx = Context.initC(instance.ctx);
        return ctx.vtable.module_v0.param_get_inner(
            ctx.data,
            instance,
            value,
            value_type,
            self,
        );
    }

    /// Internal setter for the module parameter data.
    pub fn writeFfi(
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        value_type: ParameterType,
        self: *OpaqueParameterData,
    ) callconv(.c) c.FimoResult {
        const ctx = Context.initC(instance.ctx);
        return ctx.vtable.module_v0.param_set_inner(
            ctx.data,
            instance,
            value,
            value_type,
            self,
        );
    }

    /// Internal getter for a module parameter.
    pub fn read(
        self: *const OpaqueParameterData,
        instance: *const OpaqueInstance,
        err: *?AnyError,
    ) AnyError.Error!OpaqueParameterData.Value {
        var value: OpaqueParameterData.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        const result = OpaqueParameterData.readFfi(
            instance,
            &value,
            &value_type,
            self,
        );
        try AnyError.initChecked(err, result);
        return OpaqueParameterData.Value.fromUntagged(value, value_type);
    }

    /// Internal setter for a module parameter.
    pub fn write(
        self: *const OpaqueParameterData,
        instance: *const OpaqueInstance,
        value: OpaqueParameterData.Value,
        err: *?AnyError,
    ) AnyError.Error!void {
        var val: OpaqueParameterData.ValueUntagged = undefined;
        var value_type: ParameterType = undefined;
        value.toUntagged(&val, &value_type);
        const result = OpaqueParameterData.writeFfi(
            instance,
            &val,
            value_type,
            self,
        );
        try AnyError.initChecked(err, result);
    }
};

/// Information about a symbol.
pub const Symbol = struct {
    name: [:0]const u8,
    namespace: [:0]const u8 = "",
    version: Version,
    symbol: type,
};

/// Info of a loaded module instance.
pub const Info = extern struct {
    id: Context.TypeId = .module_info,
    next: ?*Context.TaggedInStruct = null,
    name: [*:0]const u8,
    description: ?[*:0]const u8 = null,
    author: ?[*:0]const u8 = null,
    license: ?[*:0]const u8 = null,
    module_path: ?[*:0]const u8 = null,
    acquire_fn: *const fn (ctx: *const Info) callconv(.c) void,
    release_fn: *const fn (ctx: *const Info) callconv(.c) void,
    is_loaded_fn: *const fn (ctx: *const Info) callconv(.c) bool,
    acquire_module_strong_fn: *const fn (ctx: *const Info) callconv(.c) c.FimoResult,
    release_module_strong_fn: *const fn (ctx: *const Info) callconv(.c) void,

    /// Increases the reference count of the info instance.
    pub fn ref(self: *const Info) void {
        self.acquire_fn(self);
    }

    /// Decreases the reference count of the info instance.
    pub fn unref(self: *const Info) void {
        self.release_fn(self);
    }

    /// Returns whether the owning module instance is still loaded.
    pub fn isLoaded(self: *const Info) bool {
        return self.is_loaded_fn(self);
    }

    /// Increases the strong reference count of the module instance.
    ///
    /// Will prevent the module from being unloaded. This may be used to pass
    /// data, like callbacks, between modules, without registering the dependency
    /// with the subsystem.
    pub fn refInstanceStrong(self: *const Info, err: *?AnyError) AnyError.Error!void {
        const result = self.acquire_module_strong_fn(self);
        try AnyError.initChecked(err, result);
    }

    /// Decreases the strong reference count of the module instance.
    ///
    /// Should only be called after `acquire_module_strong`, when the dependency
    /// is no longer required.
    pub fn unrefInstanceStrong(self: *const Info) void {
        self.release_module_strong_fn(self);
    }

    /// Searches for a module by it's name.
    ///
    /// Queries a module by its unique name. The returned `Info` instance
    /// will have its reference count increased.
    pub fn findByName(ctx: Module, module: [:0]const u8, err: *?AnyError) AnyError.Error!*const Info {
        var info: *const Info = undefined;
        const result = ctx.context.vtable.module_v0.find_by_name(
            ctx.context.data,
            module.ptr,
            &info,
        );
        try AnyError.initChecked(err, result);
        return info;
    }

    /// Searches for a module by a symbol it exports.
    ///
    /// Queries the module that exported the specified symbol. The returned
    /// `Info` instance will have its reference count increased.
    pub fn findBySymbol(
        ctx: Module,
        name: [:0]const u8,
        namespace: [:0]const u8,
        version: Version,
        err: *?AnyError,
    ) AnyError.Error!*const Info {
        var info: *const Info = undefined;
        const result = ctx.context.vtable.module_v0.find_by_symbol(
            ctx.context.data,
            name.ptr,
            namespace.ptr,
            version.intoC(),
            &info,
        );
        try AnyError.initChecked(err, result);
        return info;
    }
};

/// State of a loaded module.
///
/// A module is self-contained, and may not be passed to other modules.
/// An instance is valid for as long as the owning module remains loaded.
/// Modules must not leak any resources outside it's own module, ensuring
/// that they are destroyed upon module unloading.
pub fn Instance(
    comptime ParametersT: type,
    comptime ResourcesT: type,
    comptime ImportsT: type,
    comptime ExportsT: type,
    comptime DataT: type,
) type {
    switch (@typeInfo(ParametersT)) {
        .@"struct" => |t| {
            std.debug.assert(t.layout == .@"extern");
            std.debug.assert(@alignOf(ParametersT) <= @alignOf(*OpaqueParameter));
            std.debug.assert(@sizeOf(ParametersT) % @sizeOf(*OpaqueParameter) == 0);
            for (@typeInfo(ParametersT).@"struct".fields) |field| {
                const is_opaque_parameter = field.type == *OpaqueParameter;
                const is_opaque = @typeInfo(@typeInfo(field.type).pointer.child) == .@"opaque";
                std.debug.assert(is_opaque_parameter or is_opaque);
                std.debug.assert(@typeInfo(field.type).pointer.is_const == false);
                std.debug.assert(field.alignment == @alignOf(*OpaqueParameter));
            }
        },
        .void => {},
        else => @compileError("Invalid instance parameter type"),
    }
    switch (@typeInfo(ResourcesT)) {
        .@"struct" => |t| {
            std.debug.assert(t.layout == .@"extern");
            std.debug.assert(@alignOf(ResourcesT) <= @alignOf([*:0]const u8));
            for (@typeInfo(ResourcesT).@"struct".fields) |field| {
                std.debug.assert(field.type == [*:0]const u8);
                std.debug.assert(field.alignment == @alignOf([*:0]const u8));
            }
        },
        .void => {},
        else => @compileError("Invalid instance resource type"),
    }
    switch (@typeInfo(ImportsT)) {
        .@"struct" => |t| {
            std.debug.assert(t.layout == .@"extern");
            std.debug.assert(@alignOf(ImportsT) <= @alignOf(*const anyopaque));
            std.debug.assert(@sizeOf(ImportsT) % @sizeOf(*const anyopaque) == 0);
            for (@typeInfo(ImportsT).@"struct".fields) |field| {
                std.debug.assert(@typeInfo(field.type).pointer.size != .Slice);
                std.debug.assert(@typeInfo(field.type).pointer.is_const);
                std.debug.assert(field.alignment == @alignOf([*:0]const u8));
            }
        },
        .void => {},
        else => @compileError("Invalid instance imports type"),
    }
    switch (@typeInfo(ExportsT)) {
        .@"struct" => |t| {
            std.debug.assert(t.layout == .@"extern");
            std.debug.assert(@alignOf(ExportsT) <= @alignOf(*const anyopaque));
            std.debug.assert(@sizeOf(ExportsT) % @sizeOf(*const anyopaque) == 0);
            for (@typeInfo(ExportsT).@"struct".fields) |field| {
                std.debug.assert(@typeInfo(field.type).pointer.size != .Slice);
                std.debug.assert(@typeInfo(field.type).pointer.is_const);
                std.debug.assert(field.alignment == @alignOf([*:0]const u8));
            }
        },
        .void => {},
        else => @compileError("Invalid instance exports type"),
    }

    const ParametersPtr = if (@sizeOf(ParametersT) == 0) ?*const ParametersT else *const ParametersT;
    const ResourcesPtr = if (@sizeOf(ResourcesT) == 0) ?*const ResourcesT else *const ResourcesT;
    const ImportsPtr = if (@sizeOf(ImportsT) == 0) ?*const ImportsT else *const ImportsT;
    const ExportsPtr = if (@sizeOf(ExportsT) == 0) ?*const ExportsT else *const ExportsT;
    const DataPtr = if (@sizeOf(DataT) == 0) ?*const DataT else *const DataT;

    return extern struct {
        vtable: *const Self.VTable,
        parameters: ParametersPtr,
        resources: ResourcesPtr,
        imports: ImportsPtr,
        exports: ExportsPtr,
        info: *const Info,
        ctx: c.FimoContext,
        data: DataPtr,

        const Self = @This();
        pub const Parameters = ParametersT;
        pub const Resources = ResourcesT;
        pub const Imports = ImportsT;
        pub const Exports = ExportsT;
        pub const Data = DataT;

        /// VTable of an Instance.
        ///
        /// Adding fields to the VTable is not a breaking change.
        pub const VTable = extern struct {
            query_namespace: *const fn (
                ctx: *const OpaqueInstance,
                namespace: [*:0]const u8,
                has_dependency: *bool,
                is_static: *bool,
            ) callconv(.c) c.FimoResult,
            add_namespace: *const fn (
                ctx: *const OpaqueInstance,
                namespace: [*:0]const u8,
                fut: *EnqueuedFuture(Fallible(void)),
            ) callconv(.c) c.FimoResult,
            remove_namespace: *const fn (
                ctx: *const OpaqueInstance,
                namespace: [*:0]const u8,
                fut: *EnqueuedFuture(Fallible(void)),
            ) callconv(.c) c.FimoResult,
            query_dependency: *const fn (
                ctx: *const OpaqueInstance,
                info: *const Info,
                has_dependency: *bool,
                is_static: *bool,
            ) callconv(.c) c.FimoResult,
            add_dependency: *const fn (
                ctx: *const OpaqueInstance,
                info: *const Info,
                fut: *EnqueuedFuture(Fallible(void)),
            ) callconv(.c) c.FimoResult,
            remove_dependency: *const fn (
                ctx: *const OpaqueInstance,
                info: *const Info,
                fut: *EnqueuedFuture(Fallible(void)),
            ) callconv(.c) c.FimoResult,
            load_symbol: *const fn (
                ctx: *const OpaqueInstance,
                name: [*:0]const u8,
                namespace: [*:0]const u8,
                version: c.FimoVersion,
                fut: *EnqueuedFuture(Fallible(*const anyopaque)),
            ) callconv(.c) c.FimoResult,
        };

        /// Returns the contained context without increasing the
        /// reference count.
        pub fn context(self: *const @This()) Context {
            return Context.initC(self.ctx);
        }

        /// Checks the status of a namespace from the view of the module.
        ///
        /// Checks if the module includes the namespace. In that case,
        /// the module is allowed access to the symbols in the namespace.
        /// Additionally, this function also queries whether the include
        /// is static, i.e., the include was specified by the module at
        /// load time.
        pub fn queryNamespace(
            self: *const @This(),
            namespace: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!enum { removed, added, static } {
            var has_dependency: bool = undefined;
            var is_static: bool = undefined;
            const result = self.vtable.query_namespace(
                self.castOpaque(),
                namespace.ptr,
                &has_dependency,
                &is_static,
            );
            try AnyError.initChecked(err, result);
            if (!has_dependency) return .removed;
            if (!is_static) return .added;
            return .static;
        }

        /// Includes a namespace by the module.
        ///
        /// Once included, the module gains access to the symbols
        /// of its dependencies that are exposed in said namespace.
        /// A namespace can not be included multiple times.
        pub fn addNamespace(
            self: *const @This(),
            namespace: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
            var fut: EnqueuedFuture(Fallible(void)) = undefined;
            const result = self.vtable.add_namespace(
                self.castOpaque(),
                namespace.ptr,
                &fut,
            );
            try AnyError.initChecked(err, result);
            return fut;
        }

        /// Removes a namespace include from the module.
        ///
        /// Once excluded, the caller guarantees to relinquish
        /// access to the symbols contained in said namespace.
        /// It is only possible to exclude namespaces that were
        /// manually added, whereas static namespace includes
        /// remain valid until the module is unloaded.
        pub fn removeNamespace(
            self: *const @This(),
            namespace: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
            var fut: EnqueuedFuture(Fallible(void)) = undefined;
            const result = self.vtable.remove_namespace(
                self.castOpaque(),
                namespace.ptr,
                &fut,
            );
            try AnyError.initChecked(err, result);
            return fut;
        }

        /// Checks if a module depends on another module.
        ///
        /// Checks if the specified module is a dependency of the
        /// current instance. In that case the instance is allowed
        /// to access the symbols exported by the module. Additionally,
        /// this function also queries whether the dependency is static,
        /// i.e., the dependency was specified by the module at load time.
        pub fn queryDependency(
            self: *const @This(),
            info: *const Info,
            err: *?AnyError,
        ) AnyError.Error!enum { removed, added, static } {
            var has_dependency: bool = undefined;
            var is_static: bool = undefined;
            const result = self.vtable.query_dependency(
                self.castOpaque(),
                info,
                &has_dependency,
                &is_static,
            );
            try AnyError.initChecked(err, result);
            if (!has_dependency) return .removed;
            if (!is_static) return .added;
            return .static;
        }

        /// Acquires another module as a dependency.
        ///
        /// After acquiring a module as a dependency, the module
        /// is allowed access to the symbols and protected parameters
        /// of said dependency. Trying to acquire a dependency to a
        /// module that is already a dependency, or to a module that
        /// would result in a circular dependency will result in an
        /// error.
        pub fn addDependency(
            self: *const @This(),
            info: *const Info,
            err: *?AnyError,
        ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
            var fut: EnqueuedFuture(Fallible(void)) = undefined;
            const result = self.vtable.add_dependency(
                self.castOpaque(),
                info,
                &fut,
            );
            try AnyError.initChecked(err, result);
            return fut;
        }

        /// Removes a module as a dependency.
        ///
        /// By removing a module as a dependency, the caller
        /// ensures that it does not own any references to resources
        /// originating from the former dependency, and allows for
        /// the unloading of the module. A module can only relinquish
        /// dependencies to modules that were acquired dynamically,
        /// as static dependencies remain valid until the module is
        /// unloaded.
        pub fn removeDependency(
            self: *const @This(),
            info: *const Info,
            err: *?AnyError,
        ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
            var fut: EnqueuedFuture(Fallible(void)) = undefined;
            const result = self.vtable.remove_dependency(
                self.castOpaque(),
                info,
                &fut,
            );
            try AnyError.initChecked(err, result);
            return fut;
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded
        /// module. This is useful for loading optional symbols, or
        /// for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until
        /// the module relinquishes the dependency to the module that
        /// exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbol(
            self: *const @This(),
            comptime symbol: Symbol,
            err: *?AnyError,
        ) AnyError.Error!EnqueuedFuture(Fallible(*const symbol.symbol)) {
            const s = try self.loadSymbolRaw(
                symbol.name,
                symbol.namespace,
                symbol.version,
                err,
            );
            return @bitCast(s);
        }

        /// Loads a symbol from the module subsystem.
        ///
        /// The caller can query the subsystem for a symbol of a loaded
        /// module. This is useful for loading optional symbols, or
        /// for loading symbols after the creation of a module. The
        /// symbol, if it exists, is returned, and can be used until
        /// the module relinquishes the dependency to the module that
        /// exported the symbol. This function fails, if the module
        /// containing the symbol is not a dependency of the module.
        pub fn loadSymbolRaw(
            self: *const @This(),
            name: [:0]const u8,
            namespace: [:0]const u8,
            version: Version,
            err: *?AnyError,
        ) AnyError.Error!EnqueuedFuture(Fallible(*const anyopaque)) {
            var fut: EnqueuedFuture(Fallible(*const anyopaque)) = undefined;
            const result = self.vtable.load_symbol(
                self.castOpaque(),
                name,
                namespace,
                version.intoC(),
                &fut,
            );
            try AnyError.initChecked(err, result);
            return fut;
        }

        /// Reads a module parameter with dependency read access.
        ///
        /// Reads the value of a module parameter with dependency read
        /// access. The operation fails, if the parameter does not exist,
        /// or if the parameter does not allow reading with a dependency
        /// access.
        pub fn readParameter(
            self: *const @This(),
            module: [:0]const u8,
            parameter: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!OpaqueParameter.Value {
            return OpaqueParameter.readDependency(
                self.castOpaque(),
                module,
                parameter,
                err,
            );
        }

        /// Sets a module parameter with dependency write access.
        ///
        /// Sets the value of a module parameter with dependency write
        /// access. The operation fails, if the parameter does not exist,
        /// or if the parameter does not allow writing with a dependency
        /// access.
        pub fn writeParameter(
            self: *const @This(),
            value: OpaqueParameter.Value,
            module: [:0]const u8,
            parameter: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!void {
            return OpaqueParameter.writeDependency(
                self.castOpaque(),
                value,
                module,
                parameter,
                err,
            );
        }

        /// Casts the instance pointer to an opaque instance pointer.
        pub fn castOpaque(self: *const @This()) *const OpaqueInstance {
            return @ptrCast(self);
        }
    };
}

/// Type of an opaque module instance.
pub const OpaqueInstance = Instance(
    void,
    void,
    void,
    void,
    void,
);

/// Type of a pseudo module instance.
pub const PseudoInstance = extern struct {
    instance: OpaqueInstance,

    pub const Parameters = OpaqueInstance.Parameters;
    pub const Resources = OpaqueInstance.Resources;
    pub const Imports = OpaqueInstance.Imports;
    pub const Exports = OpaqueInstance.Exports;
    pub const Data = OpaqueInstance.Data;

    /// Constructs a new pseudo module.
    ///
    /// The functions of the module subsystem require that the caller owns
    /// a reference to their own module. This is a problem, as the constructor
    /// of the context won't be assigned a module instance during bootstrapping.
    /// As a workaround, we allow for the creation of pseudo modules, i.e.,
    /// module handles without an associated module.
    pub fn init(ctx: Module, err: *?AnyError) AnyError.Error!*const PseudoInstance {
        var instance: *const PseudoInstance = undefined;
        const result = ctx.context.vtable.module_v0.pseudo_module_new(
            ctx.context.data,
            &instance,
        );
        try AnyError.initChecked(err, result);
        return instance;
    }

    /// Destroys the pseudo module.
    ///
    /// By destroying the pseudo module, the caller ensures that they
    /// relinquished all access to handles derived by the module subsystem.
    pub fn deinit(self: *const PseudoInstance, err: *?AnyError) AnyError.Error!void {
        const ctx = Context.initC(self.instance.ctx);
        const result = ctx.vtable.module_v0.unload(
            ctx.data,
            self.instance.info,
        );
        try AnyError.initChecked(err, result);
    }

    /// Checks the status of a namespace from the view of the module.
    ///
    /// Checks if the module includes the namespace. In that case,
    /// the module is allowed access to the symbols in the namespace.
    /// Additionally, this function also queries whether the include
    /// is static, i.e., the include was specified by the module at
    /// load time.
    pub fn queryNamespace(
        self: *const @This(),
        namespace: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!enum { removed, added, static } {
        return self.castOpaque().queryNamespace(
            namespace,
            err,
        );
    }

    /// Includes a namespace by the module.
    ///
    /// Once included, the module gains access to the symbols
    /// of its dependencies that are exposed in said namespace.
    /// A namespace can not be included multiple times.
    pub fn addNamespace(
        self: *const @This(),
        namespace: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        return self.castOpaque().addNamespace(
            namespace,
            err,
        );
    }

    /// Removes a namespace include from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish
    /// access to the symbols contained in said namespace.
    /// It is only possible to exclude namespaces that were
    /// manually added, whereas static namespace includes
    /// remain valid until the module is unloaded.
    pub fn removeNamespace(
        self: *const @This(),
        namespace: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        return self.castOpaque().removeNamespace(
            namespace,
            err,
        );
    }

    /// Checks if a module depends on another module.
    ///
    /// Checks if the specified module is a dependency of the
    /// current instance. In that case the instance is allowed
    /// to access the symbols exported by the module. Additionally,
    /// this function also queries whether the dependency is static,
    /// i.e., the dependency was specified by the module at load time.
    pub fn queryDependency(
        self: *const @This(),
        info: *const Info,
        err: *?AnyError,
    ) AnyError.Error!enum { removed, added, static } {
        return self.castOpaque().queryDependency(
            info,
            err,
        );
    }

    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module
    /// is allowed access to the symbols and protected parameters
    /// of said dependency. Trying to acquire a dependency to a
    /// module that is already a dependency, or to a module that
    /// would result in a circular dependency will result in an
    /// error.
    pub fn addDependency(
        self: *const @This(),
        info: *const Info,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        return self.castOpaque().addDependency(
            info,
            err,
        );
    }

    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller
    /// ensures that it does not own any references to resources
    /// originating from the former dependency, and allows for
    /// the unloading of the module. A module can only relinquish
    /// dependencies to modules that were acquired dynamically,
    /// as static dependencies remain valid until the module is
    /// unloaded.
    pub fn removeDependency(
        self: *const @This(),
        info: *const Info,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        return self.castOpaque().removeDependency(
            info,
            err,
        );
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded
    /// module. This is useful for loading optional symbols, or
    /// for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until
    /// the module relinquishes the dependency to the module that
    /// exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    pub fn loadSymbol(
        self: *const @This(),
        comptime symbol: Symbol,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(*const symbol.symbol)) {
        return self.castOpaque().loadSymbol(symbol, err);
    }

    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded
    /// module. This is useful for loading optional symbols, or
    /// for loading symbols after the creation of a module. The
    /// symbol, if it exists, is returned, and can be used until
    /// the module relinquishes the dependency to the module that
    /// exported the symbol. This function fails, if the module
    /// containing the symbol is not a dependency of the module.
    pub fn loadSymbolRaw(
        self: *const @This(),
        name: [:0]const u8,
        namespace: [:0]const u8,
        version: Version,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(*const anyopaque)) {
        return self.castOpaque().loadSymbolRaw(
            name,
            namespace,
            version,
            err,
        );
    }

    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read
    /// access. The operation fails, if the parameter does not exist,
    /// or if the parameter does not allow reading with a dependency
    /// access.
    pub fn readParameter(
        self: *const @This(),
        module: [:0]const u8,
        parameter: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!OpaqueParameter.Value {
        return self.castOpaque().readParameter(
            module,
            parameter,
            err,
        );
    }

    /// Sets a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write
    /// access. The operation fails, if the parameter does not exist,
    /// or if the parameter does not allow writing with a dependency
    /// access.
    pub fn writeParameter(
        self: *const @This(),
        value: OpaqueParameter.Value,
        module: [:0]const u8,
        parameter: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!void {
        return self.castOpaque().writeParameter(
            value,
            module,
            parameter,
            err,
        );
    }

    /// Casts the instance pointer to an opaque instance pointer.
    pub fn castOpaque(self: *const @This()) *const OpaqueInstance {
        return @ptrCast(self);
    }
};

/// Type-erased set of modules to load by the subsystem.
pub const LoadingSet = extern struct {
    data: *anyopaque,
    vtable: *const LoadingSet.VTable,

    pub const FilterOp = enum(u32) {
        skip,
        load,
    };

    /// VTable of a loading set.
    ///
    /// Adding to the VTable is not a breaking change.
    pub const VTable = extern struct {
        ref: *const fn (ctx: *anyopaque) callconv(.c) void,
        unref: *const fn (ctx: *anyopaque) callconv(.c) void,
        query_module: *const fn (
            ctx: *anyopaque,
            module: [*:0]const u8,
            fut: *EnqueuedFuture(Fallible(bool)),
        ) callconv(.c) c.FimoResult,
        query_symbol: *const fn (
            ctx: *anyopaque,
            symbol: [*:0]const u8,
            namespace: [*:0]const u8,
            version: c.FimoVersion,
            fut: *EnqueuedFuture(Fallible(bool)),
        ) callconv(.c) c.FimoResult,
        add_callback: *const fn (
            ctx: *anyopaque,
            module: [*:0]const u8,
            on_success: *const fn (info: *const Info, data: ?*anyopaque) callconv(.c) void,
            on_error: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) void,
            on_abort: ?*const fn (data: ?*anyopaque) callconv(.c) void,
            data: ?*anyopaque,
            fut: *EnqueuedFuture(Fallible(void)),
        ) callconv(.c) c.FimoResult,
        add_module: *const fn (
            ctx: *anyopaque,
            owner: *const OpaqueInstance,
            module: *const Export,
            fut: *EnqueuedFuture(Fallible(void)),
        ) callconv(.c) c.FimoResult,
        add_modules_from_path: *const fn (
            ctx: *anyopaque,
            path: c.FimoUTF8Path,
            filter_fn: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
            filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
            filter_data: ?*anyopaque,
            fut: *EnqueuedFuture(Fallible(void)),
        ) callconv(.c) c.FimoResult,
        add_modules_from_local: *const fn (
            ctx: *anyopaque,
            filter_fn: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
            filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
            filter_data: ?*anyopaque,
            iterator_fn: *const fn (
                f: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
                data: ?*anyopaque,
            ) callconv(.c) void,
            bin_ptr: *const anyopaque,
            fut: *EnqueuedFuture(Fallible(void)),
        ) callconv(.c) c.FimoResult,
        commit: *const fn (
            ctx: *anyopaque,
            fut: *EnqueuedFuture(Fallible(void)),
        ) callconv(.c) c.FimoResult,
    };

    /// Constructs a new empty set.
    ///
    /// Modules can only be loaded, if all of their dependencies can be
    /// resolved, which requires us to determine a suitable load order.
    /// A loading set is a utility to facilitate this process, by
    /// automatically computing a suitable load order for a batch of
    /// modules.
    pub fn init(ctx: Module, err: *?AnyError) AnyError.Error!EnqueuedFuture(Fallible(LoadingSet)) {
        var fut: EnqueuedFuture(Fallible(LoadingSet)) = undefined;
        const result = ctx.context.vtable.module_v0.set_new(ctx.context.data, &fut);
        try AnyError.initChecked(err, result);
        return fut;
    }

    /// Increases the reference count of the instance.
    pub fn ref(self: LoadingSet) void {
        self.vtable.ref(self.data);
    }

    /// Decreases the reference count of the instance.
    pub fn unref(self: LoadingSet) void {
        self.vtable.unref(self.data);
    }

    /// Checks whether the set contains a specific module.
    pub fn queryModule(
        self: LoadingSet,
        module: [:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(bool)) {
        var fut: EnqueuedFuture(Fallible(bool)) = undefined;
        const result = self.vtable.query_module(self.data, module, &fut);
        try AnyError.initChecked(err, result);
        return fut;
    }

    /// Checks whether the set contains a specific symbol.
    pub fn querySymbol(
        self: LoadingSet,
        name: [:0]const u8,
        namespace: [:0]const u8,
        version: Version,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(bool)) {
        var fut: EnqueuedFuture(Fallible(bool)) = undefined;
        const result = self.vtable.query_symbol(
            self.data,
            name.ptr,
            namespace.ptr,
            version.intoC(),
            &fut,
        );
        try AnyError.initChecked(err, result);
        return fut;
    }

    /// Adds a status callback to the set.
    ///
    /// Adds a callback to report a successful or failed loading of
    /// a module. The success callback wil be called if the set was able to load
    /// all requested modules, whereas the error callback will be called immediately
    /// after the failed loading of the module. Since the module set can be in a
    /// partially loaded state at the time of calling this function, the error path
    /// may be invoked immediately. The callbacks will be provided with a user-specified
    /// data pointer, which they are in charge of cleaning up. If an error occurs during the
    /// execution of the returned future, it will invoke the optional `on_abort` function.
    /// If the requested module does not exist, the returned future will return an error.
    pub fn addCallback(
        self: LoadingSet,
        module: [:0]const u8,
        obj: anytype,
        comptime callback: fn (
            status: union(enum) { ok: *const Info, err: *const Export, abort },
            data: @TypeOf(obj),
        ) void,
        comptime on_abort: ?fn (data: @TypeOf(obj)) void,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        const Ptr = @TypeOf(obj);
        std.debug.assert(@typeInfo(Ptr) == .pointer);
        std.debug.assert(@typeInfo(Ptr).pointer.size == .One);
        const Callbacks = struct {
            fn onOk(info: *const Info, data: ?*anyopaque) callconv(.c) void {
                const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                callback(.{ .ok = info }, o);
            }
            fn onErr(mod: *const Export, data: ?*anyopaque) callconv(.c) void {
                const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                callback(.{ .err = mod }, o);
            }
            fn onAbort(data: ?*anyopaque) callconv(.c) void {
                if (on_abort) |f| {
                    const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                    f(o);
                }
            }
        };
        return self.addCallbackCustom(
            module.ptr,
            @constCast(obj),
            Callbacks.onOk,
            Callbacks.onErr,
            if (on_abort) Callbacks.onAbort else null,
            err,
        );
    }

    /// Adds a status callback to the set.
    ///
    /// Adds a callback to report a successful or failed loading of
    /// a module. The success path wil be called if the set was able to load
    /// all requested modules, whereas the error path will be called immediately
    /// after the failed loading of the module. Since the module set can be in a
    /// partially loaded state at the time of calling this function, the error path
    /// may be invoked immediately. The callbacks will be provided with a user-specified
    /// data pointer, which they are in charge of cleaning up. If an error occurs during the
    /// execution of the returned future, it will invoke the optional `on_abort` function.
    /// If the requested module does not exist, the returned future will return an error.
    pub fn addCallbackCustom(
        self: LoadingSet,
        module: [*:0]const u8,
        data: ?*anyopaque,
        on_success: *const fn (info: *const Info, data: ?*anyopaque) callconv(.c) void,
        on_error: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) void,
        on_abort: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        var fut: EnqueuedFuture(Fallible(void)) = undefined;
        const result = self.vtable.add_callback(
            self.data,
            module,
            on_success,
            on_error,
            on_abort,
            data,
            &fut,
        );
        try AnyError.initChecked(err, result);
        return fut;
    }

    /// Adds a module to the set.
    ///
    /// Adds a module to the set, so that it may be loaded by a future call
    /// to `commit`. Trying to include an invalid module, a module with
    /// duplicate exports or duplicate name will result in an error. Unlike
    /// `addModulesFromPath`, this function allows for the loading of dynamic
    /// modules, i.e. modules that are created at runtime, like non-native
    /// modules, which may require a runtime to be executed in. The new module
    /// inherits a strong reference to the same binary as the caller's module.
    ///
    /// Note that the new module is not setup to automatically depend on the
    /// owner, but may prevent it from being unloaded while the set exists.
    pub fn addModule(
        self: LoadingSet,
        owner: *const OpaqueInstance,
        module: *const Export,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        var fut: EnqueuedFuture(Fallible(void)) = undefined;
        const result = self.vtable.add_module(
            self.data,
            owner,
            module,
            &fut,
        );
        try AnyError.initChecked(err, result);
        return fut;
    }

    /// Adds modules to the set.
    ///
    /// Opens up a module binary to select which modules to load.
    /// If the path points to a file, the function will try to load the file
    /// as a binary, whereas, if it points to a directory, it will try to
    /// load a file named `module.fimo_module` contained in the directory.
    /// Each exported module is then passed to the filter, along with the
    /// provided data, which can then filter which modules to load. This
    /// function may skip invalid module exports. Trying to include a module
    /// with duplicate exports or duplicate name will result in an error.
    /// This function signals an error, if the binary does not contain the
    /// symbols necessary to query the exported modules, but does not return
    /// an error, if it does not export any modules. The necessary symbols
    /// are set up automatically, if the binary was linked with the fimo
    /// library. In case of an error, no modules are appended to the set.
    pub fn addModulesFromPath(
        self: LoadingSet,
        path: Path,
        obj: anytype,
        comptime filter: fn (module: *const Export, data: @TypeOf(obj)) LoadingSet.FilterOp,
        comptime filter_deinit: ?fn (data: @TypeOf(obj)) void,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        const Ptr = @TypeOf(obj);
        std.debug.assert(@typeInfo(Ptr) == .pointer);
        std.debug.assert(@typeInfo(Ptr).pointer.size == .One);
        const Callbacks = struct {
            fn f(module: *const Export, data: ?*anyopaque) callconv(.c) bool {
                const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                return filter(module, o) == .load;
            }
            fn deinit(data: ?*anyopaque) callconv(.c) void {
                if (filter_deinit) {
                    const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                    filter_deinit(o);
                }
            }
        };
        return self.addModulesFromPathCustom(
            path,
            @constCast(obj),
            Callbacks.f,
            if (filter_deinit != null) &Callbacks.deinit else null,
            err,
        );
    }

    /// Adds modules to the set.
    ///
    /// Opens up a module binary to select which modules to load.
    /// If the path points to a file, the function will try to load the file
    /// as a binary, whereas, if it points to a directory, it will try to
    /// load a file named `module.fimo_module` contained in the directory.
    /// Each exported module is then passed to the filter, along with the
    /// provided data, which can then filter which modules to load. This
    /// function may skip invalid module exports. Trying to include a module
    /// with duplicate exports or duplicate name will result in an error.
    /// This function signals an error, if the binary does not contain the
    /// symbols necessary to query the exported modules, but does not return
    /// an error, if it does not export any modules. The necessary symbols
    /// are set up automatically, if the binary was linked with the fimo
    /// library. In case of an error, no modules are appended to the set.
    pub fn addModulesFromPathCustom(
        self: LoadingSet,
        path: Path,
        filter_data: ?*anyopaque,
        filter: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        var fut: EnqueuedFuture(Fallible(void)) = undefined;
        const result = self.vtable.add_modules_from_path(
            self.data,
            path.intoC(),
            filter,
            filter_deinit,
            filter_data,
            &fut,
        );
        try AnyError.initChecked(err, result);
        return fut;
    }

    /// Adds modules to the set.
    ///
    /// Iterates over the exported modules of the current binary. Each exported
    /// module is then passed to the filter, along with the provided data,
    /// which can then filter which modules to load. This function may skip
    /// invalid module exports. Trying to include a module with duplicate
    /// exports or duplicate name will result in an error. This function
    /// signals an error, if the binary does not contain the symbols
    /// necessary to query the exported modules, but does not return
    /// an error, if it does not export any modules. The necessary
    /// symbols are set up automatically, if the binary was linked with
    /// the fimo library. In case of an error, no modules are appended
    /// to the set.
    pub fn addModulesFromLocal(
        self: LoadingSet,
        obj: anytype,
        comptime filter: fn (module: *const Export, data: @TypeOf(obj)) LoadingSet.FilterOp,
        comptime filter_deinit: ?fn (data: @TypeOf(obj)) void,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        const Ptr = @TypeOf(obj);
        std.debug.assert(@typeInfo(Ptr) == .pointer);
        std.debug.assert(@typeInfo(Ptr).pointer.size == .One);
        const Callbacks = struct {
            fn f(module: *const Export, data: ?*anyopaque) callconv(.c) bool {
                const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                return filter(module, o) == .load;
            }
            fn deinit(data: ?*anyopaque) callconv(.c) void {
                if (filter_deinit) {
                    const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                    filter_deinit(o);
                }
            }
        };
        return self.addModulesFromLocalCustom(
            @constCast(obj),
            Callbacks.f,
            if (filter_deinit != null) &Callbacks.deinit else null,
            err,
        );
    }

    /// Adds modules to the set.
    ///
    /// Iterates over the exported modules of the current binary. Each exported
    /// module is then passed to the filter, along with the provided data,
    /// which can then filter which modules to load. This function may skip
    /// invalid module exports. Trying to include a module with duplicate
    /// exports or duplicate name will result in an error. This function
    /// signals an error, if the binary does not contain the symbols
    /// necessary to query the exported modules, but does not return
    /// an error, if it does not export any modules. The necessary
    /// symbols are set up automatically, if the binary was linked with
    /// the fimo library. In case of an error, no modules are appended
    /// to the set.
    pub fn addModulesFromLocalCustom(
        self: LoadingSet,
        filter_data: ?*anyopaque,
        filter: *const fn (module: *const Export, data: ?*anyopaque) callconv(.c) bool,
        filter_deinit: ?*const fn (data: ?*anyopaque) callconv(.c) void,
        err: *?AnyError,
    ) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        var fut: EnqueuedFuture(Fallible(void)) = undefined;
        const result = self.vtable.add_modules_from_local(
            self.data,
            filter,
            filter_deinit,
            filter_data,
            exports.ExportIter.fimo_impl_module_export_iterator,
            @ptrCast(&exports.ExportIter.fimo_impl_module_export_iterator),
            &fut,
        );
        try AnyError.initChecked(err, result);
        return fut;
    }

    /// Loads the modules contained in the set.
    ///
    /// If the returned future is successfull, the contained modules and their
    /// resources are made available to the remaining modules. Some conditions
    /// may hinder the loading of some module, like missing dependencies,
    /// duplicates, and other loading errors. In those cases, the modules will
    /// be skipped without erroring.
    ///
    /// It is possible to submit multiple concurrent commit requests, even from
    /// the same loading set. In that case, the requests will be handled
    /// atomically, in an unspecified order.
    pub fn commit(self: LoadingSet, err: *?AnyError) AnyError.Error!EnqueuedFuture(Fallible(void)) {
        var fut: EnqueuedFuture(Fallible(void)) = undefined;
        const result = self.vtable.commit(self.data, &fut);
        try AnyError.initChecked(err, result);
        return fut;
    }
};

/// VTable of the module subsystem.
///
/// Changing the VTable is a breaking change.
pub const VTable = extern struct {
    pseudo_module_new: *const fn (
        ctx: *anyopaque,
        instance: **const PseudoInstance,
    ) callconv(.c) c.FimoResult,
    set_new: *const fn (
        ctx: *anyopaque,
        fut: *EnqueuedFuture(Fallible(LoadingSet)),
    ) callconv(.c) c.FimoResult,
    find_by_name: *const fn (
        ctx: *anyopaque,
        name: [*:0]const u8,
        info: **const Info,
    ) callconv(.c) c.FimoResult,
    find_by_symbol: *const fn (
        ctx: *anyopaque,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: c.FimoVersion,
        info: **const Info,
    ) callconv(.c) c.FimoResult,
    namespace_exists: *const fn (
        ctx: *anyopaque,
        namespace: [*:0]const u8,
        exists: *bool,
    ) callconv(.c) c.FimoResult,
    unload: *const fn (
        ctx: *anyopaque,
        info: ?*const Info,
    ) callconv(.c) c.FimoResult,
    param_query: *const fn (
        ctx: *anyopaque,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
        type: *ParameterType,
        read_group: *ParameterAccessGroup,
        write_group: *ParameterAccessGroup,
    ) callconv(.c) c.FimoResult,
    param_set_public: *const fn (
        ctx: *anyopaque,
        value: *const anyopaque,
        type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) c.FimoResult,
    param_get_public: *const fn (
        ctx: *anyopaque,
        value: *anyopaque,
        type: *ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) c.FimoResult,
    param_set_dependency: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) c.FimoResult,
    param_get_dependency: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *anyopaque,
        type: *ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.c) c.FimoResult,
    param_set_private: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        type: ParameterType,
        parameter: *OpaqueParameter,
    ) callconv(.c) c.FimoResult,
    param_get_private: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *anyopaque,
        type: *ParameterType,
        parameter: *const OpaqueParameter,
    ) callconv(.c) c.FimoResult,
    param_set_inner: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        type: ParameterType,
        parameter: *OpaqueParameterData,
    ) callconv(.c) c.FimoResult,
    param_get_inner: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *anyopaque,
        type: *ParameterType,
        parameter: *const OpaqueParameterData,
    ) callconv(.c) c.FimoResult,
};

/// Checks for the presence of a namespace in the module subsystem.
///
/// A namespace exists, if at least one loaded module exports
/// one symbol in said namespace.
pub fn namespaceExists(
    self: Module,
    namespace: [:0]const u8,
    err: *?AnyError,
) AnyError.Error!bool {
    var exists: bool = undefined;
    const result = self.context.vtable.module_v0.namespace_exists(
        self.context.data,
        namespace.ptr,
        &exists,
    );
    try AnyError.initChecked(err, result);
    return exists;
}

/// Unloads a module.
///
/// If successful, this function unloads the module.
/// To succeed, the module no other module may depend on the module.
/// This function automatically unloads cleans up unreferenced modules,
/// except if they are a pseudo module.
///
/// Setting `info` to `null` only runs the cleanup of all loose modules.
pub fn unloadModule(
    self: Module,
    info: ?*const Info,
    err: *?AnyError,
) AnyError.Error!void {
    const result = self.context.vtable.module_v0.unload(
        self.context.data,
        info,
    );
    try AnyError.initChecked(err, result);
}

/// Queries the info of a module parameter.
///
/// This function can be used to query the datatype, the read access,
/// and the write access of a module parameter. This function fails,
/// if the parameter can not be found.
pub fn queryParameter(
    self: Module,
    module: [:0]const u8,
    parameter: [:0]const u8,
    err: *?AnyError,
) AnyError.Error!OpaqueParameter.Info {
    return OpaqueParameter.query(self, module, parameter, err);
}

/// Reads a module parameter with public read access.
///
/// Reads the value of a module parameter with public read access.
/// The operation fails, if the parameter does not exist, or if
/// the parameter does not allow reading with a public access.
pub fn readParameter(
    self: Module,
    module: [:0]const u8,
    parameter: [:0]const u8,
    err: *?AnyError,
) AnyError.Error!OpaqueParameter.Value {
    return OpaqueParameter.readPublic(self, module, parameter, err);
}

/// Sets a module parameter with public write access.
///
/// Sets the value of a module parameter with public write access.
/// The operation fails, if the parameter does not exist, or if
/// the parameter does not allow writing with a public access.
pub fn writeParameter(
    self: Module,
    value: OpaqueParameter.Value,
    module: [:0]const u8,
    parameter: [:0]const u8,
    err: *?AnyError,
) AnyError.Error!void {
    return OpaqueParameter.writePublic(self, value, module, parameter, err);
}
