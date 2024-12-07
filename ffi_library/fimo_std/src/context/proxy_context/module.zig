//! Public interface of the module subsystem.

const std = @import("std");
const builtin = @import("builtin");

const AnyError = @import("../../AnyError.zig");
const c = @import("../../c.zig");
const Path = @import("../../path.zig").Path;
const Version = @import("../../Version.zig");
const Context = @import("../proxy_context.zig");

context: Context,

const Module = @This();

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

    fn readFfi(
        instance: *const OpaqueInstance,
        value: *anyopaque,
        value_type: *ParameterType,
        self: *const OpaqueParameterData,
    ) callconv(.C) c.FimoResult {
        const ctx = Context.initC(instance.ctx);
        return ctx.vtable.module_v0.param_get_inner(
            ctx.data,
            instance,
            value,
            value_type,
            self,
        );
    }
    fn writeFfi(
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        value_type: ParameterType,
        self: *OpaqueParameterData,
    ) callconv(.C) c.FimoResult {
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
    acquire_fn: *const fn (ctx: *const Info) callconv(.C) void,
    release_fn: *const fn (ctx: *const Info) callconv(.C) void,
    is_loaded_fn: *const fn (ctx: *const Info) callconv(.C) bool,
    acquire_module_strong_fn: *const fn (ctx: *const Info) callconv(.C) c.FimoResult,
    release_module_strong_fn: *const fn (ctx: *const Info) callconv(.C) void,

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

        /// Returns the contained context without increasing the
        /// reference count.
        pub fn context(self: *const @This()) Context {
            return Context.initC(self.ctx);
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
        ) AnyError.Error!void {
            const ctx = Context.initC(self.ctx);
            const result = ctx.vtable.module_v0.namespace_include(
                ctx.data,
                self.castOpaque(),
                namespace.ptr,
            );
            try AnyError.initChecked(err, result);
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
        ) AnyError.Error!void {
            const ctx = Context.initC(self.ctx);
            const result = ctx.vtable.module_v0.namespace_exclude(
                ctx.data,
                self.castOpaque(),
                namespace.ptr,
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
        pub fn hasNamespace(
            self: *const @This(),
            namespace: [:0]const u8,
            err: *?AnyError,
        ) AnyError.Error!enum { removed, added, static } {
            var is_included: bool = undefined;
            var is_static: bool = undefined;
            const ctx = Context.initC(self.ctx);
            const result = ctx.vtable.module_v0.namespace_included(
                ctx.data,
                self.castOpaque(),
                namespace.ptr,
                &is_included,
                &is_static,
            );
            try AnyError.initChecked(err, result);
            if (!is_included) return .removed;
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
        ) AnyError.Error!void {
            const ctx = Context.initC(self.ctx);
            const result = ctx.vtable.module_v0.acquire_dependency(
                ctx.data,
                self.castOpaque(),
                info,
            );
            try AnyError.initChecked(err, result);
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
        ) AnyError.Error!void {
            const ctx = Context.initC(self.ctx);
            const result = ctx.vtable.module_v0.relinquish_dependency(
                ctx.data,
                self.castOpaque(),
                info,
            );
            try AnyError.initChecked(err, result);
        }

        /// Checks if a module depends on another module.
        ///
        /// Checks if the specified module is a dependency of the
        /// current instance. In that case the instance is allowed
        /// to access the symbols exported by the module. Additionally,
        /// this function also queries whether the dependency is static,
        /// i.e., the dependency was specified by the module at load time.
        pub fn hasDependency(
            self: *const @This(),
            info: *const Info,
            err: *?AnyError,
        ) AnyError.Error!enum { removed, added, static } {
            var is_dependency: bool = undefined;
            var is_static: bool = undefined;
            const ctx = Context.initC(self.ctx);
            const result = ctx.vtable.module_v0.has_dependency(
                ctx.data,
                self.castOpaque(),
                info,
                &is_dependency,
                &is_static,
            );
            try AnyError.initChecked(err, result);
            if (!is_dependency) return .removed;
            if (!is_static) return .added;
            return .static;
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
        ) AnyError.Error!*const symbol.symbol {
            const s = try self.loadSymbolRaw(
                symbol.name,
                symbol.namespace,
                symbol.version,
                err,
            );
            return @alignCast(@ptrCast(s));
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
        ) AnyError.Error!*const anyopaque {
            var symbol: *const anyopaque = undefined;
            const ctx = Context.initC(self.ctx);
            const result = ctx.vtable.module_v0.load_symbol(
                ctx.data,
                self.castOpaque(),
                name,
                namespace,
                version.intoC(),
                &symbol,
            );
            try AnyError.initChecked(err, result);
            return symbol;
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
    pub fn deinit(self: *const PseudoInstance, err: *?AnyError) AnyError.Error!Context {
        var out_ctx: c.FimoContext = undefined;
        const ctx = Context.initC(self.instance.ctx);
        const result = ctx.vtable.module_v0.pseudo_module_destroy(
            ctx.data,
            self,
            &out_ctx,
        );
        try AnyError.initChecked(err, result);
        return Context.initC(out_ctx);
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
    ) AnyError.Error!void {
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
    ) AnyError.Error!void {
        return self.castOpaque().removeNamespace(
            namespace,
            err,
        );
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
        return self.castOpaque().hasNamespace(
            namespace,
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
    ) AnyError.Error!void {
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
    ) AnyError.Error!void {
        return self.castOpaque().removeDependency(
            info,
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
    pub fn hasDependency(
        self: *const @This(),
        info: *const Info,
        err: *?AnyError,
    ) AnyError.Error!enum { removed, added, static } {
        return self.castOpaque().hasDependency(
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
    ) AnyError.Error!*const symbol.symbol {
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
    ) AnyError.Error!*const anyopaque {
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
pub const LoadingSet = opaque {
    /// Constructs a new empty module set.
    ///
    /// The loading of a module fails, if at least one dependency can
    /// not be satisfied, which requires the caller to manually find a
    /// suitable loading order. To facilitate the loading, we load
    /// multiple modules together, and automatically determine an
    /// appropriate load order for all modules inside the module set.
    pub fn init(ctx: Module, err: *?AnyError) AnyError.Error!*LoadingSet {
        var set: *LoadingSet = undefined;
        const result = ctx.context.vtable.module_v0.set_new(ctx.context.data, &set);
        try AnyError.initChecked(err, result);
        return set;
    }

    /// Destroys the module set without loading any modules.
    ///
    /// It is not possible to dismiss a module set that is currently being loaded.
    pub fn dismiss(self: *LoadingSet, ctx: Module, err: *?AnyError) AnyError.Error!void {
        const result = ctx.context.vtable.module_v0.set_dismiss(ctx.context.data, self);
        try AnyError.initChecked(err, result);
    }

    /// Destroys the module set and loads the modules contained in it.
    ///
    /// After successfully calling this function, the modules contained
    /// in the set are loaded, and their symbols are available to all
    /// other modules. If the construction of one module results in an
    /// error, or if a dependency can not be satisfied, this function
    /// rolls back the loading of all modules contained in the set
    /// and returns an error. It is not possible to load a module set,
    /// while another set is being loaded.
    pub fn commit(self: *LoadingSet, ctx: Module, err: *?AnyError) AnyError.Error!void {
        const result = ctx.context.vtable.module_v0.set_finish(ctx.context.data, self);
        try AnyError.initChecked(err, result);
    }

    /// Checks whether the module set contains a module.
    pub fn hasModule(self: *LoadingSet, ctx: Module, module: [:0]const u8, err: *?AnyError) AnyError.Error!bool {
        var exists: bool = undefined;
        const result = ctx.context.vtable.module_v0.set_has_module(
            ctx.context.data,
            self,
            module.ptr,
            &exists,
        );
        try AnyError.initChecked(err, result);
        return exists;
    }

    /// Checks whether the module set contains a symbol.
    pub fn hasSymbol(
        self: *LoadingSet,
        ctx: Module,
        name: [:0]const u8,
        namespace: [:0]const u8,
        version: Version,
        err: *?AnyError,
    ) AnyError.Error!bool {
        var exists: bool = undefined;
        const result = ctx.context.vtable.module_v0.set_has_symbol(
            ctx.context.data,
            self,
            name.ptr,
            namespace.ptr,
            version.intoC(),
            &exists,
        );
        try AnyError.initChecked(err, result);
        return exists;
    }

    /// Adds a status callback to the module set.
    ///
    /// Adds a set of callbacks to report a successful or failed loading of
    /// a module. The success path wil be called if the set was able to load
    /// all requested modules, whereas the error path will be called immediately
    /// after the failed loading of the module. Since the module set can be in a
    /// partially loaded state at the time of calling this function, the error path
    /// may be invoked immediately. The callbacks will be provided with a user-specified
    /// data pointer, which they are in charge of cleaning up. If the requested module
    /// not exist, this function will return an error.
    pub fn addCallback(
        self: *LoadingSet,
        ctx: Module,
        module: [:0]const u8,
        obj: anytype,
        comptime callback: fn (
            status: union(enum) { ok: *const Info, err: *const Export },
            data: @TypeOf(obj),
        ) void,
        err: *?AnyError,
    ) AnyError.Error!void {
        const Ptr = @TypeOf(obj);
        std.debug.assert(@typeInfo(Ptr) == .pointer);
        std.debug.assert(@typeInfo(Ptr).pointer.size == .One);
        const Callbacks = struct {
            fn onOk(info: *const Info, data: ?*anyopaque) callconv(.C) void {
                const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                callback(.{ .ok = info }, o);
            }
            fn onErr(mod: *const Export, data: ?*anyopaque) callconv(.C) void {
                const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                callback(.{ .err = mod }, o);
            }
        };
        return self.addCallbackCustom(
            ctx,
            module.ptr,
            @constCast(obj),
            Callbacks.onOk,
            Callbacks.onErr,
            err,
        );
    }

    /// Adds a status callback to the module set.
    ///
    /// Adds a set of callbacks to report a successful or failed loading of
    /// a module. The success path wil be called if the set was able to load
    /// all requested modules, whereas the error path will be called immediately
    /// after the failed loading of the module. Since the module set can be in a
    /// partially loaded state at the time of calling this function, the error path
    /// may be invoked immediately. The callbacks will be provided with a user-specified
    /// data pointer, which they are in charge of cleaning up. If the requested module
    /// not exist, this function will return an error.
    pub fn addCallbackCustom(
        self: *LoadingSet,
        ctx: Module,
        module: [*:0]const u8,
        data: ?*anyopaque,
        on_success: *const fn (info: *const Info, data: ?*anyopaque) callconv(.C) void,
        on_error: *const fn (module: *const Export, data: ?*anyopaque) callconv(.C) void,
        err: *?AnyError,
    ) AnyError.Error!void {
        const result = ctx.context.vtable.module_v0.set_append_callback(
            ctx.context.data,
            self,
            module,
            on_success,
            on_error,
            data,
        );
        try AnyError.initChecked(err, result);
    }

    /// Adds a freestanding module to the module set.
    ///
    /// Adds a freestanding module to the set, so that it may be loaded
    /// by a future call to `commit`. Trying to include an invalid module,
    /// a module with duplicate exports or duplicate name will result in
    /// an error. Unlike `addModulesFromPath`, this function allows for the
    /// loading of dynamic modules, i.e. modules that are created at runtime,
    /// like non-native modules, which may require a runtime to be executed
    /// in. The new module inherits a strong reference to the same binary as
    /// the caller's module.
    ///
    /// Note that the new module is not setup to automatically depend on
    /// the owner, but may prevent it from being unloaded while the set exists.
    pub fn addModuleFreestanding(
        self: *LoadingSet,
        owner: *const OpaqueInstance,
        module: *const Export,
        err: *?AnyError,
    ) AnyError.Error!void {
        const ctx = Context.initC(owner.ctx);
        const result = ctx.vtable.module_v0.set_append_freestanding_module(
            ctx.data,
            owner,
            self,
            module,
        );
        try AnyError.initChecked(err, result);
    }

    /// Adds modules to the module set.
    ///
    /// Opens up a module binary to select which modules to load.
    /// The binary path must be encoded as `UTF-8`, and point to the binary
    /// that contains the modules. If the path is `null`, it iterates over
    /// the exported modules of the current binary. Each exported module is
    /// then passed to the filter, along with the provided data, which can
    /// then filter which modules to load. This function may skip
    /// invalid module exports. Trying to include a module with duplicate
    /// exports or duplicate name will result in an error. This function
    /// signals an error, if the binary does not contain the symbols
    /// necessary to query the exported modules, but does not return
    /// an error, if it does not export any modules. The necessary
    /// symbols are setup automatically, if the binary was linked with
    /// the fimo library. In case of an error, no modules are appended
    /// to the set.
    pub fn addModulesFromPath(
        self: *LoadingSet,
        ctx: Module,
        path: ?[:0]const u8,
        obj: anytype,
        comptime filter: fn (module: *const Export, data: @TypeOf(obj)) bool,
        err: *?AnyError,
    ) AnyError.Error!void {
        const Ptr = @TypeOf(obj);
        std.debug.assert(@typeInfo(Ptr) == .pointer);
        std.debug.assert(@typeInfo(Ptr).pointer.size == .One);
        const Callbacks = struct {
            fn f(module: *const Export, data: ?*anyopaque) callconv(.C) bool {
                const o: Ptr = @alignCast(@ptrCast(@constCast(data)));
                return filter(module, o);
            }
        };
        return self.addModulesFromPathCustom(
            ctx,
            if (path) |p| p.ptr else null,
            @constCast(obj),
            Callbacks.f,
            err,
        );
    }

    /// Adds modules to the module set.
    ///
    /// Opens up a module binary to select which modules to load.
    /// The binary path must be encoded as `UTF-8`, and point to the binary
    /// that contains the modules. If the path is `null`, it iterates over
    /// the exported modules of the current binary. Each exported module is
    /// then passed to the filter, along with the provided data, which can
    /// then filter which modules to load. This function may skip
    /// invalid module exports. Trying to include a module with duplicate
    /// exports or duplicate name will result in an error. This function
    /// signals an error, if the binary does not contain the symbols
    /// necessary to query the exported modules, but does not return
    /// an error, if it does not export any modules. The necessary
    /// symbols are setup automatically, if the binary was linked with
    /// the fimo library. In case of an error, no modules are appended
    /// to the set.
    pub fn addModulesFromPathCustom(
        self: *LoadingSet,
        ctx: Module,
        path: ?[*:0]const u8,
        data: ?*anyopaque,
        filter: *const fn (module: *const Export, data: ?*anyopaque) callconv(.C) bool,
        err: *?AnyError,
    ) AnyError.Error!void {
        const result = ctx.context.vtable.module_v0.set_append_modules(
            ctx.context.data,
            self,
            path,
            filter,
            data,
            Export.ExportIter.fimo_impl_module_export_iterator,
            @ptrCast(@constCast(&Export.ExportIter.fimo_impl_module_export_iterator)),
        );
        try AnyError.initChecked(err, result);
    }
};

/// Declaration of a module export.
pub const Export = extern struct {
    id: Context.TypeId = .module_export,
    next: ?*Context.TaggedInStruct = null,
    version: c.FimoVersion = Context.context_version.intoC(),
    name: [*:0]const u8,
    description: ?[*:0]const u8 = null,
    author: ?[*:0]const u8 = null,
    license: ?[*:0]const u8 = null,
    parameters: ?[*]const Export.Parameter = null,
    parameters_count: u32 = 0,
    resources: ?[*]const Export.Resource = null,
    resources_count: u32 = 0,
    namespace_imports: ?[*]const Export.Namespace = null,
    namespace_imports_count: u32 = 0,
    symbol_imports: ?[*]const Export.SymbolImport = null,
    symbol_imports_count: u32 = 0,
    symbol_exports: ?[*]const Export.SymbolExport = null,
    symbol_exports_count: u32 = 0,
    dynamic_symbol_exports: ?[*]const Export.DynamicSymbolExport = null,
    dynamic_symbol_exports_count: u32 = 0,
    modifiers: ?[*]const Export.Modifier = null,
    modifiers_count: u32 = 0,
    module_constructor: ?*const fn (
        ctx: *const OpaqueInstance,
        set: *LoadingSet,
        data: *?*anyopaque,
    ) callconv(.C) c.FimoResult = null,
    module_destructor: ?*const fn (
        ctx: *const OpaqueInstance,
        data: ?*anyopaque,
    ) callconv(.C) void = null,

    /// Declaration of a module parameter.
    pub const Parameter = extern struct {
        type: ParameterType,
        read_group: ParameterAccessGroup = .private,
        write_group: ParameterAccessGroup = .private,
        setter: *const fn (
            ctx: *const OpaqueInstance,
            value: *const anyopaque,
            type: ParameterType,
            data: *OpaqueParameterData,
        ) callconv(.C) c.FimoResult = OpaqueParameterData.writeFfi,
        getter: *const fn (
            ctx: *const OpaqueInstance,
            value: *anyopaque,
            type: *ParameterType,
            data: *const OpaqueParameterData,
        ) callconv(.C) c.FimoResult = OpaqueParameterData.readFfi,
        name: [*:0]const u8,
        default_value: extern union {
            u8: u8,
            u16: u16,
            u32: u32,
            u64: u64,
            i8: i8,
            i16: i16,
            i32: i32,
            i64: i64,
        },
    };

    /// Declaration of a module resource.
    pub const Resource = extern struct {
        path: [*:0]const u8,
    };

    /// Declaration of a module namespace import.
    pub const Namespace = extern struct {
        name: [*:0]const u8,
    };

    /// Declaration of a module symbol import.
    pub const SymbolImport = extern struct {
        version: c.FimoVersion,
        name: [*:0]const u8,
        namespace: [*:0]const u8 = "",
    };

    /// Declaration of a static module symbol export.
    pub const SymbolExport = extern struct {
        symbol: *const anyopaque,
        version: c.FimoVersion,
        name: [*:0]const u8,
        namespace: [*:0]const u8 = "",
    };

    /// Declaration of a dynamic module symbol export.
    pub const DynamicSymbolExport = extern struct {
        constructor: *const fn (
            ctx: *const OpaqueInstance,
            symbol: **anyopaque,
        ) callconv(.C) c.FimoResult,
        destructor: *const fn (symbol: *anyopaque) callconv(.C) void,
        version: c.FimoVersion,
        name: [*:0]const u8,
        namespace: [*:0]const u8 = "",
    };

    /// A modifier declaration for a module export.
    pub const Modifier = extern struct {
        tag: enum(c.FimoModuleExportModifierKey) {
            destructor = c.FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR,
            dependency = c.FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY,
            _,
        },
        value: extern union {
            destructor: *const extern struct {
                data: ?*anyopaque,
                destructor: *const fn (ptr: ?*anyopaque) callconv(.C) void,
            },
            dependency: *const Info,
        },
    };

    const exports_section = switch (builtin.target.os.tag) {
        .macos, .ios, .watchos, .tvos, .visionos => struct {
            const start_exports = @extern(
                [*]const ?*const Export,
                .{ .name = "_start_fimo_module" },
            );
            const stop_exports = @extern(
                [*]const ?*const Export,
                .{ .name = "_stop_fimo_module" },
            );
            const export_visibility = .hidden;
            // Make shure that the section is created.
            comptime {
                asm (
                    \\.pushsection __DATA,fimo_module,regular,no_dead_strip
                    \\.align 8
                    \\.quad 0
                    \\.popsection
                    \\
                    \\.no_dead_strip __start_fimo_module
                    \\.global __start_fimo_module
                    \\__start_fimo_module = section$start$__DATA$fimo_module
                    \\
                    \\.no_dead_strip __stop_fimo_module
                    \\.global __stop_fimo_module
                    \\__stop_fimo_module = section$end$__DATA$fimo_module
                );
                exportModuleInner(null);
            }
        },
        .windows => struct {
            const a: ?*const Export = null;
            const z: ?*const Export = null;

            const start_exports: [*]const ?*const Export = @ptrCast(&a);
            const stop_exports: [*]const ?*const Export = @ptrCast(&z);
            const export_visibility = .default;

            // Create the section.
            comptime {
                @export(&a, .{
                    .name = "module_export_" ++ @typeName(@This()) ++ "start",
                    .section = "fi_mod$a",
                    .linkage = .strong,
                    .visibility = .default,
                });
                @export(&z, .{
                    .name = "module_export_" ++ @typeName(@This()) ++ "end",
                    .section = "fi_mod$z",
                    .linkage = .strong,
                    .visibility = .default,
                });
            }
        },
        else => struct {
            extern const __start_fimo_module: ?*const Export;
            extern const __stop_fimo_module: ?*const Export;

            const start_exports: [*]const ?*const Export = @ptrCast(
                &__start_fimo_module,
            );
            const stop_exports: [*]const ?*const Export = @ptrCast(
                &__stop_fimo_module,
            );
            const export_visibility = .hidden;

            // Make shure that the section is created.
            comptime {
                exportModuleInner(null);
                asm (
                    \\.pushsection .init_array,"aw",%init_array
                    \\.reloc ., BFD_RELOC_NONE, fimo_module
                    \\.popsection
                );
            }
        },
    };

    /// Iterator over all exports of the current binary.
    pub const ExportIter = struct {
        /// Iterator position. Does not necessarily point to a valid export.
        position: [*]const ?*const Export,

        /// Initializes the iterator. Does not need to be deinitialized.
        pub fn init() @This() {
            return .{
                .position = @ptrCast(exports_section.start_exports),
            };
        }

        /// Returns the next export in the export link.
        pub fn next(self: *@This()) ?*const Export {
            while (true) {
                if (self.position == exports_section.stop_exports) {
                    return null;
                }
                const element_ptr = self.position;
                self.position += 1;

                const element = element_ptr[0];
                if (element != null) {
                    return element;
                }
            }
        }

        pub export fn fimo_impl_module_export_iterator(
            inspector: *const fn (module: *const Export, data: ?*anyopaque) callconv(.C) bool,
            data: ?*anyopaque,
        ) void {
            var it = Export.ExportIter.init();
            while (it.next()) |exp| {
                if (!inspector(exp, data)) {
                    return;
                }
            }
        }
    };

    /// Creates a new unique export in the correct section.
    ///
    /// For internal use only, as the pointer should not generally be null.
    fn exportModuleInner(comptime module: ?*const Export) void {
        _ = struct {
            const data = module;
            comptime {
                @export(&data, .{
                    .name = "module_export_" ++ @typeName(@This()),
                    .section = c.FIMO_IMPL_MODULE_SECTION,
                    .linkage = .strong,
                    .visibility = exports_section.export_visibility,
                });
            }
        };
    }

    /// Builder for a module export.
    pub const Builder = struct {
        name: [:0]const u8,
        description: ?[:0]const u8 = null,
        author: ?[:0]const u8 = null,
        license: ?[:0]const u8 = null,
        parameters: []const Builder.Parameter = &.{},
        resources: []const Builder.Resource = &.{},
        namespaces: []const Export.Namespace = &.{},
        imports: []const Builder.SymbolImport = &.{},
        exports: []const Builder.SymbolExport = &.{},
        modifiers: []const Builder.Modifier = &.{},
        stateType: type = void,
        constructor: ?*const fn (
            ctx: *const OpaqueInstance,
            set: *LoadingSet,
            data: *?*anyopaque,
        ) callconv(.C) c.FimoResult = null,
        destructor: ?*const fn (
            ctx: *const OpaqueInstance,
            data: ?*anyopaque,
        ) callconv(.C) void = null,

        const Parameter = struct {
            name: []const u8,
            member_name: [:0]const u8,
            default_value: OpaqueParameter.Value,
            read_group: ParameterAccessGroup = .private,
            write_group: ParameterAccessGroup = .private,
            read: *const fn (
                ctx: *const OpaqueInstance,
                value: *anyopaque,
                type: *ParameterType,
                data: *const OpaqueParameterData,
            ) callconv(.C) c.FimoResult = OpaqueParameterData.readFfi,
            write: *const fn (
                ctx: *const OpaqueInstance,
                value: *const anyopaque,
                type: ParameterType,
                data: *OpaqueParameterData,
            ) callconv(.C) c.FimoResult = OpaqueParameterData.writeFfi,
        };

        const Resource = struct {
            name: [:0]const u8,
            path: Path,
        };

        const SymbolImport = struct {
            name: [:0]const u8,
            symbol: Symbol,
        };

        const SymbolExport = struct {
            name: [:0]const u8,
            symbol: Symbol,
            value: union(enum) {
                static: *const anyopaque,
                dynamic: struct {
                    initFn: *const fn (
                        ctx: *const OpaqueInstance,
                        symbol: **anyopaque,
                    ) callconv(.C) c.FimoResult,
                    deinitFn: *const fn (symbol: *anyopaque) callconv(.C) void,
                },
            },
        };

        const Modifier = union(enum) {
            _,
        };

        fn State(comptime T: type) type {
            if (@sizeOf(T) == 0) {
                return struct {
                    const InitFn = fn (ctx: *const OpaqueInstance, set: *LoadingSet) anyerror!void;
                    const DeinitFn = fn (ctx: *const OpaqueInstance) void;
                    fn wrapInit(comptime f: InitFn) fn (
                        ctx: *const OpaqueInstance,
                        set: *LoadingSet,
                        data: *?*anyopaque,
                    ) callconv(.C) c.FimoResult {
                        return struct {
                            fn wrapper(
                                ctx: *const OpaqueInstance,
                                set: *LoadingSet,
                                data: *?*anyopaque,
                            ) callconv(.C) c.FimoResult {
                                f(ctx, set) catch |err| {
                                    if (@errorReturnTrace()) |tr|
                                        ctx.context().tracing().emitStackTraceSimple(tr.*, @src());
                                    return AnyError.initError(err).err;
                                };
                                data.* = null;
                                return AnyError.intoCResult(null);
                            }
                        }.wrapper;
                    }
                    fn wrapDeinit(comptime f: DeinitFn) fn (
                        ctx: *const OpaqueInstance,
                        data: ?*anyopaque,
                    ) callconv(.C) void {
                        return struct {
                            fn wrapper(
                                ctx: *const OpaqueInstance,
                                data: ?*anyopaque,
                            ) callconv(.C) void {
                                std.debug.assert(data == null);
                                f(ctx);
                            }
                        }.wrapper;
                    }
                };
            } else {
                return struct {
                    const InitFn = fn (ctx: *const OpaqueInstance, set: *LoadingSet) anyerror!*T;
                    const DeinitFn = fn (ctx: *const OpaqueInstance, state: *T) void;
                    fn wrapInit(comptime f: InitFn) fn (
                        ctx: *const OpaqueInstance,
                        set: *LoadingSet,
                        data: *?*anyopaque,
                    ) callconv(.C) c.FimoResult {
                        return struct {
                            fn wrapper(
                                ctx: *const OpaqueInstance,
                                set: *LoadingSet,
                                data: *?*anyopaque,
                            ) callconv(.C) c.FimoResult {
                                data.* = f(ctx, set) catch |err| {
                                    if (@errorReturnTrace()) |tr|
                                        ctx.context().tracing().emitStackTraceSimple(tr.*, @src());
                                    return AnyError.initError(err).err;
                                };
                                return AnyError.intoCResult(null);
                            }
                        }.wrapper;
                    }
                    fn wrapDeinit(comptime f: DeinitFn) fn (
                        ctx: *const OpaqueInstance,
                        data: ?*anyopaque,
                    ) callconv(.C) void {
                        return struct {
                            fn wrapper(
                                ctx: *const OpaqueInstance,
                                data: ?*anyopaque,
                            ) callconv(.C) void {
                                const state: ?*T = @alignCast(@ptrCast(data));
                                f(ctx, state.?);
                            }
                        }.wrapper;
                    }
                };
            }
        }

        /// Initializes a new builder.
        pub fn init(comptime name: [:0]const u8) Builder {
            return .{ .name = name };
        }

        /// Adds a description to the module.
        pub fn withDescription(comptime self: Builder, comptime description: ?[:0]const u8) Builder {
            var x = self;
            x.description = description;
            return x;
        }

        /// Adds a author to the module.
        pub fn withAuthor(comptime self: Builder, comptime author: ?[:0]const u8) Builder {
            var x = self;
            x.author = author;
            return x;
        }

        /// Adds a license to the module.
        pub fn withLicense(comptime self: Builder, comptime license: ?[:0]const u8) Builder {
            var x = self;
            x.license = license;
            return x;
        }

        /// Adds a parameter to the module.
        pub fn withParameter(
            comptime self: Builder,
            comptime parameter: Builder.Parameter,
        ) Builder {
            for (self.parameters) |p| {
                if (std.mem.eql(u8, p.name, parameter.name)) @compileError(
                    std.fmt.comptimePrint("duplicate parameter name: '{s}'", .{parameter.name}),
                );
                if (std.mem.eql(u8, p.member_name, parameter.member_name)) @compileError(
                    std.fmt.comptimePrint(
                        "duplicate parameter member name: '{s}'",
                        .{parameter.member_name},
                    ),
                );
            }

            var parameters: [self.parameters.len + 1]Builder.Parameter = undefined;
            @memcpy(parameters[0..self.parameters.len], self.parameters);
            parameters[self.parameters.len] = parameter;

            var x = self;
            x.parameters = &parameters;
            return x;
        }

        /// Adds a resource path to the module.
        pub fn withResource(
            comptime self: Builder,
            comptime resource: Builder.Resource,
        ) Builder {
            for (self.resources) |res| {
                if (std.mem.eql(u8, res.name, resource.name))
                    @compileError(
                        std.fmt.comptimePrint(
                            "duplicate resource member name: '{s}'",
                            .{res.name},
                        ),
                    );
            }

            var paths: [self.resources.len + 1]Builder.Resource = undefined;
            @memcpy(paths[0..self.resources.len], self.resources);
            paths[self.resources.len] = resource;

            var x = self;
            x.resources = &paths;
            return x;
        }

        /// Adds a namespace import to the module.
        ///
        /// A namespace may be imported multiple times.
        pub fn withNamespace(
            comptime self: Builder,
            comptime name: []const u8,
        ) Builder {
            if (std.mem.eql(u8, name, "")) return self;
            for (self.namespaces) |ns| {
                if (std.mem.eql(u8, std.mem.span(ns.name), name)) return self;
            }

            var namespaces: [self.namespaces.len + 1]Export.Namespace = undefined;
            @memcpy(namespaces[0..self.namespaces.len], self.namespaces);
            namespaces[self.namespaces.len] = .{ .name = name ++ "\x00" };

            var x = self;
            x.namespaces = &namespaces;
            return x;
        }

        /// Adds an import to the module.
        ///
        /// Automatically imports the required namespace.
        pub fn withImport(
            comptime self: Builder,
            comptime import: Builder.SymbolImport,
        ) Builder {
            for (self.imports) |imp| {
                if (std.mem.eql(u8, imp.name, import.name))
                    @compileError(
                        std.fmt.comptimePrint(
                            "duplicate import member name: '{s}'",
                            .{imp.name},
                        ),
                    );
            }

            var imports: [self.imports.len + 1]Builder.SymbolImport = undefined;
            @memcpy(imports[0..self.imports.len], self.imports);
            imports[self.imports.len] = import;

            var x = self.withNamespace(import.symbol.namespace);
            x.imports = &imports;
            return x;
        }

        fn withExportInner(
            comptime self: Builder,
            comptime @"export": Builder.SymbolExport,
        ) Builder {
            for (self.exports) |exp| {
                if (std.mem.eql(u8, exp.name, @"export".name))
                    @compileError(
                        std.fmt.comptimePrint(
                            "duplicate export member name: '{s}'",
                            .{exp.name},
                        ),
                    );
            }

            var exports: [self.exports.len + 1]Builder.SymbolExport = undefined;
            @memcpy(exports[0..self.exports.len], self.exports);
            exports[self.exports.len] = @"export";

            var x = self;
            x.exports = &exports;
            return x;
        }

        /// Adds a static export to the module.
        pub fn withExport(
            comptime self: Builder,
            comptime T: Symbol,
            comptime name: [:0]const u8,
            comptime value: *const T.symbol,
        ) Builder {
            const exp = Builder.SymbolExport{
                .name = name,
                .symbol = T,
                .value = .{ .static = value },
            };
            return self.withExportInner(exp);
        }

        /// Adds a static export to the module.
        pub fn withDynamicExport(
            comptime self: Builder,
            comptime T: Symbol,
            comptime name: [:0]const u8,
            comptime initFn: fn (ctx: *const OpaqueInstance) anyerror!*T.symbol,
            comptime deinitFn: fn (symbol: *T.symbol) void,
        ) Builder {
            const initWrapped = struct {
                fn f(ctx: *const OpaqueInstance, out: **anyopaque) callconv(.C) c.FimoResult {
                    out.* = initFn(ctx) catch |err| return AnyError.initError(err).err;
                    return AnyError.intoCResult(null);
                }
            }.f;
            const deinitWrapped = struct {
                fn f(symbol: *anyopaque) callconv(.C) void {
                    return deinitFn(@alignCast(@ptrCast(symbol)));
                }
            }.f;
            const exp = Builder.SymbolExport{
                .name = name,
                .symbol = T,
                .value = .{
                    .dynamic = .{
                        .initFn = &initWrapped,
                        .deinitFn = &deinitWrapped,
                    },
                },
            };
            return self.withExportInner(exp);
        }

        fn withModifierInner(
            comptime self: Builder,
            comptime modifier: Builder.Modifier,
        ) Builder {
            var modifiers: [self.modifiers.len + 1]Builder.Modifier = undefined;
            @memcpy(modifiers[0..self.modifiers.len], self.modifiers);
            modifiers[self.modifiers.len] = modifier;

            var x = self;
            x.modifiers = &modifiers;
            return x;
        }

        /// Adds a state to the module.
        pub fn withState(
            comptime self: Builder,
            comptime T: type,
            comptime initFn: State(T).InitFn,
            comptime deinitFn: State(T).DeinitFn,
        ) Builder {
            var x = self;
            x.stateType = T;
            x.constructor = &State(T).wrapInit(initFn);
            x.destructor = &State(T).wrapDeinit(deinitFn);
            return x;
        }

        fn ParameterTable(comptime self: Builder) type {
            if (self.parameters.len == 0) return void;
            var fields: [self.parameters.len]std.builtin.Type.StructField = undefined;
            for (self.parameters, &fields) |p, *f| {
                const pType = switch (p.default_value) {
                    .u8 => u8,
                    .u16 => u16,
                    .u32 => u32,
                    .u64 => u64,
                    .i8 => i8,
                    .i16 => i16,
                    .i32 => i32,
                    .i64 => i64,
                };
                f.* = std.builtin.Type.StructField{
                    .name = p.member_name,
                    .type = *Module.Parameter(pType, OpaqueInstance),
                    .default_value = null,
                    .is_comptime = false,
                    .alignment = @alignOf(*anyopaque),
                };
            }
            const t: std.builtin.Type = .{
                .@"struct" = std.builtin.Type.Struct{
                    .layout = .@"extern",
                    .fields = &fields,
                    .decls = &.{},
                    .is_tuple = false,
                },
            };
            return @Type(t);
        }

        fn ResourceTable(comptime self: Builder) type {
            if (self.resources.len == 0) return void;
            var fields: [self.resources.len]std.builtin.Type.StructField = undefined;
            for (self.resources, &fields) |x, *f| {
                f.* = std.builtin.Type.StructField{
                    .name = x.name,
                    .type = [*:0]const u8,
                    .default_value = null,
                    .is_comptime = false,
                    .alignment = @alignOf([*:0]const u8),
                };
            }
            const t: std.builtin.Type = .{
                .@"struct" = std.builtin.Type.Struct{
                    .layout = .@"extern",
                    .fields = &fields,
                    .decls = &.{},
                    .is_tuple = false,
                },
            };
            return @Type(t);
        }

        fn ImportTable(comptime self: Builder) type {
            if (self.imports.len == 0) return void;
            var fields: [self.imports.len]std.builtin.Type.StructField = undefined;
            for (self.imports, &fields) |x, *f| {
                f.* = std.builtin.Type.StructField{
                    .name = x.name,
                    .type = *const x.symbol.symbol,
                    .default_value = null,
                    .is_comptime = false,
                    .alignment = @alignOf(*const anyopaque),
                };
            }
            const t: std.builtin.Type = .{
                .@"struct" = std.builtin.Type.Struct{
                    .layout = .@"extern",
                    .fields = &fields,
                    .decls = &.{},
                    .is_tuple = false,
                },
            };
            return @Type(t);
        }

        fn ExportsTable(comptime self: Builder) type {
            if (self.exports.len == 0) return void;
            var i: usize = 0;
            var fields: [self.exports.len]std.builtin.Type.StructField = undefined;
            for (self.exports) |x| {
                if (x.value != .static) continue;
                fields[i] = std.builtin.Type.StructField{
                    .name = x.name,
                    .type = *const x.symbol.symbol,
                    .default_value = null,
                    .is_comptime = false,
                    .alignment = @alignOf(*const anyopaque),
                };
                i += 1;
            }
            for (self.exports) |x| {
                if (x.value != .dynamic) continue;
                fields[i] = std.builtin.Type.StructField{
                    .name = x.name,
                    .type = *const x.symbol.symbol,
                    .default_value = null,
                    .is_comptime = false,
                    .alignment = @alignOf(*const anyopaque),
                };
                i += 1;
            }
            const t: std.builtin.Type = .{
                .@"struct" = std.builtin.Type.Struct{
                    .layout = .@"extern",
                    .fields = &fields,
                    .decls = &.{},
                    .is_tuple = false,
                },
            };
            return @Type(t);
        }

        fn ffiParameters(comptime self: Builder) []const Export.Parameter {
            var parameters: [self.parameters.len]Export.Parameter = undefined;
            for (self.parameters, &parameters) |src, *dst| {
                dst.* = Export.Parameter{
                    .name = src.name ++ "\x00",
                    .read_group = src.read_group,
                    .write_group = src.write_group,
                    .getter = src.read,
                    .setter = src.write,
                    .type = switch (src.default_value) {
                        .u8 => .u8,
                        .u16 => .u16,
                        .u32 => .u32,
                        .u64 => .u64,
                        .i8 => .i8,
                        .i16 => .i16,
                        .i32 => .i32,
                        .i64 => .i64,
                    },
                    .default_value = switch (src.default_value) {
                        .u8 => |v| .{ .u8 = v },
                        .u16 => |v| .{ .u16 = v },
                        .u32 => |v| .{ .u32 = v },
                        .u64 => |v| .{ .u64 = v },
                        .i8 => |v| .{ .i8 = v },
                        .i16 => |v| .{ .i16 = v },
                        .i32 => |v| .{ .i32 = v },
                        .i64 => |v| .{ .i64 = v },
                    },
                };
            }
            const parameters_c = parameters;
            return &parameters_c;
        }

        fn ffiResources(comptime self: Builder) []const Export.Resource {
            var resources: [self.resources.len]Export.Resource = undefined;
            for (self.resources, &resources) |src, *dst| {
                dst.* = Export.Resource{
                    .path = src.path.raw ++ "\x00",
                };
            }
            const resources_c = resources;
            return &resources_c;
        }

        fn ffiNamespaces(comptime self: Builder) []const Export.Namespace {
            var namespaces: [self.namespaces.len]Export.Namespace = undefined;
            for (self.namespaces, &namespaces) |src, *dst| {
                dst.* = Export.Namespace{
                    .name = src.name,
                };
            }
            const namespaces_c = namespaces;
            return &namespaces_c;
        }

        fn ffiImports(comptime self: Builder) []const Export.SymbolImport {
            var imports: [self.imports.len]Export.SymbolImport = undefined;
            for (self.imports, &imports) |src, *dst| {
                dst.* = Export.SymbolImport{
                    .name = src.symbol.name,
                    .namespace = src.symbol.namespace,
                    .version = src.symbol.version.intoC(),
                };
            }
            const imports_c = imports;
            return &imports_c;
        }

        fn ffiExports(comptime self: Builder) []const Export.SymbolExport {
            var count: usize = 0;
            for (self.exports) |exp| {
                if (exp.value != .static) continue;
                count += 1;
            }

            var i: usize = 0;
            var exports: [count]Export.SymbolExport = undefined;
            for (self.exports) |src| {
                if (src.value != .static) continue;
                exports[i] = Export.SymbolExport{
                    .symbol = src.value.static,
                    .name = src.symbol.name,
                    .namespace = src.symbol.namespace,
                    .version = src.symbol.version.intoC(),
                };
                i += 1;
            }
            const exports_c = exports;
            return &exports_c;
        }

        fn ffiDynamicExports(comptime self: Builder) []const Export.DynamicSymbolExport {
            var count: usize = 0;
            for (self.exports) |exp| {
                if (exp.value != .dynamic) continue;
                count += 1;
            }

            var i: usize = 0;
            var exports: [count]Export.DynamicSymbolExport = undefined;
            for (self.exports) |src| {
                if (src.value != .dynamic) continue;
                exports[i] = Export.DynamicSymbolExport{
                    .name = src.symbol.name,
                    .namespace = src.symbol.namespace,
                    .version = src.symbol.version.intoC(),
                    .constructor = src.value.dynamic.initFn,
                    .destructor = src.value.dynamic.deinitFn,
                };
                i += 1;
            }
            const exports_c = exports;
            return &exports_c;
        }

        /// Exports the module with the specified configuration.
        pub fn exportModule(comptime self: Builder) type {
            const parameters = self.ffiParameters();
            const resources = self.ffiResources();
            const namespaces = self.ffiNamespaces();
            const imports = self.ffiImports();
            const exports = self.ffiExports();
            const dynamic_exports = self.ffiDynamicExports();
            const exp = &Export{
                .name = self.name,
                .description = self.description orelse null,
                .author = self.author orelse null,
                .license = self.license orelse null,
                .parameters = if (parameters.len > 0) parameters.ptr else null,
                .parameters_count = parameters.len,
                .resources = if (resources.len > 0) resources.ptr else null,
                .resources_count = resources.len,
                .namespace_imports = if (namespaces.len > 0) namespaces.ptr else null,
                .namespace_imports_count = namespaces.len,
                .symbol_imports = if (imports.len > 0) imports.ptr else null,
                .symbol_imports_count = imports.len,
                .symbol_exports = if (exports.len > 0) exports.ptr else null,
                .symbol_exports_count = exports.len,
                .dynamic_symbol_exports = if (dynamic_exports.len > 0) dynamic_exports.ptr else null,
                .dynamic_symbol_exports_count = dynamic_exports.len,
                .modifiers = null,
                .modifiers_count = 0,
                .module_constructor = self.constructor,
                .module_destructor = self.destructor,
            };
            exportModuleInner(exp);

            return Instance(
                self.ParameterTable(),
                self.ResourceTable(),
                self.ImportTable(),
                self.ExportsTable(),
                self.stateType,
            );
        }
    };

    /// Runs the registered cleanup routines.
    pub fn deinit(self: *const Export) void {
        for (self.getModifiers()) |modifier| {
            switch (modifier.tag) {
                .destructor => {
                    const destructor = modifier.value.destructor;
                    destructor.destructor(destructor.data);
                },
                .dependency => {
                    const dependency = modifier.value.dependency;
                    dependency.unref();
                },
                else => @panic("Unknown modifier"),
            }
        }
    }

    /// Returns the version of the context compiled agains.
    pub fn getVersion(self: *const Export) Version {
        return Version.initC(self.version);
    }

    /// Returns the name of the export.
    pub fn getName(self: *const Export) []const u8 {
        return std.mem.span(self.name);
    }

    /// Returns the description of the export.
    pub fn getDescription(self: *const Export) ?[]const u8 {
        return if (self.description) |x| std.mem.span(x) else null;
    }

    /// Returns the author of the export.
    pub fn getAuthor(self: *const Export) ?[]const u8 {
        return if (self.author) |x| std.mem.span(x) else null;
    }

    /// Returns the license of the export.
    pub fn getLicense(self: *const Export) ?[]const u8 {
        return if (self.license) |x| std.mem.span(x) else null;
    }

    /// Returns the parameters.
    pub fn getParameters(self: *const Export) []const Export.Parameter {
        return if (self.parameters) |x| x[0..self.parameters_count] else &.{};
    }

    /// Returns the resources.
    pub fn getResources(self: *const Export) []const Export.Resource {
        return if (self.resources) |x| x[0..self.resources_count] else &.{};
    }

    /// Returns the namespace imports.
    pub fn getNamespaceImports(self: *const Export) []const Export.Namespace {
        return if (self.namespace_imports) |x| x[0..self.namespace_imports_count] else &.{};
    }

    /// Returns the symbol imports.
    pub fn getSymbolImports(self: *const Export) []const Export.SymbolImport {
        return if (self.symbol_imports) |x| x[0..self.symbol_imports_count] else &.{};
    }

    /// Returns the static symbol exports.
    pub fn getSymbolExports(self: *const Export) []const Export.SymbolExport {
        return if (self.symbol_exports) |x| x[0..self.symbol_exports_count] else &.{};
    }

    /// Returns the dynamic symbol exports.
    pub fn getDynamicSymbolExports(self: *const Export) []const Export.DynamicSymbolExport {
        return if (self.dynamic_symbol_exports) |x|
            x[0..self.dynamic_symbol_exports_count]
        else
            &.{};
    }

    /// Returns the modifiers.
    pub fn getModifiers(self: *const Export) []const Export.Modifier {
        return if (self.modifiers) |x| x[0..self.modifiers_count] else &.{};
    }
};

/// VTable of the module subsystem.
///
/// Changing the VTable is a breaking change.
pub const VTable = extern struct {
    pseudo_module_new: *const fn (
        ctx: *anyopaque,
        instance: **const PseudoInstance,
    ) callconv(.C) c.FimoResult,
    pseudo_module_destroy: *const fn (
        ctx: *anyopaque,
        instance: *const PseudoInstance,
        context: *c.FimoContext,
    ) callconv(.C) c.FimoResult,
    set_new: *const fn (ctx: *anyopaque, set: **LoadingSet) callconv(.C) c.FimoResult,
    set_has_module: *const fn (
        ctx: *anyopaque,
        set: *LoadingSet,
        module: [*:0]const u8,
        exists: *bool,
    ) callconv(.C) c.FimoResult,
    set_has_symbol: *const fn (
        ctx: *anyopaque,
        set: *LoadingSet,
        symbol: [*:0]const u8,
        namespace: [*:0]const u8,
        version: c.FimoVersion,
        exists: *bool,
    ) callconv(.C) c.FimoResult,
    set_append_callback: *const fn (
        ctx: *anyopaque,
        set: *LoadingSet,
        module: [*:0]const u8,
        on_success: *const fn (info: *const Info, data: ?*anyopaque) callconv(.C) void,
        on_error: *const fn (module: *const Export, data: ?*anyopaque) callconv(.C) void,
        data: ?*anyopaque,
    ) callconv(.C) c.FimoResult,
    set_append_freestanding_module: *const fn (
        ctx: *anyopaque,
        owner: *const OpaqueInstance,
        set: *LoadingSet,
        module: *const Export,
    ) callconv(.C) c.FimoResult,
    set_append_modules: *const fn (
        ctx: *anyopaque,
        set: *LoadingSet,
        path: ?[*:0]const u8,
        filter: *const fn (module: *const Export, data: ?*anyopaque) callconv(.C) bool,
        data: ?*anyopaque,
        module_iterator: *const fn (
            f: *const fn (module: *const Export, data: ?*anyopaque) callconv(.C) bool,
            data: ?*anyopaque,
        ) callconv(.C) void,
        module_handle: *anyopaque,
    ) callconv(.C) c.FimoResult,
    set_dismiss: *const fn (
        ctx: *anyopaque,
        set: *LoadingSet,
    ) callconv(.C) c.FimoResult,
    set_finish: *const fn (
        ctx: *anyopaque,
        set: *LoadingSet,
    ) callconv(.C) c.FimoResult,
    find_by_name: *const fn (
        ctx: *anyopaque,
        name: [*:0]const u8,
        info: **const Info,
    ) callconv(.C) c.FimoResult,
    find_by_symbol: *const fn (
        ctx: *anyopaque,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: c.FimoVersion,
        info: **const Info,
    ) callconv(.C) c.FimoResult,
    namespace_exists: *const fn (
        ctx: *anyopaque,
        namespace: [*:0]const u8,
        exists: *bool,
    ) callconv(.C) c.FimoResult,
    namespace_include: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        namespace: [*:0]const u8,
    ) callconv(.C) c.FimoResult,
    namespace_exclude: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        namespace: [*:0]const u8,
    ) callconv(.C) c.FimoResult,
    namespace_included: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        namespace: [*:0]const u8,
        is_included: *bool,
        is_static: *bool,
    ) callconv(.C) c.FimoResult,
    acquire_dependency: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        info: *const Info,
    ) callconv(.C) c.FimoResult,
    relinquish_dependency: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        info: *const Info,
    ) callconv(.C) c.FimoResult,
    has_dependency: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        info: *const Info,
        is_dependency: *bool,
        is_static: *bool,
    ) callconv(.C) c.FimoResult,
    load_symbol: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        name: [*:0]const u8,
        namespace: [*:0]const u8,
        version: c.FimoVersion,
        symbol: **const anyopaque,
    ) callconv(.C) c.FimoResult,
    unload: *const fn (
        ctx: *anyopaque,
        info: ?*const Info,
    ) callconv(.C) c.FimoResult,
    param_query: *const fn (
        ctx: *anyopaque,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
        type: *ParameterType,
        read_group: *ParameterAccessGroup,
        write_group: *ParameterAccessGroup,
    ) callconv(.C) c.FimoResult,
    param_set_public: *const fn (
        ctx: *anyopaque,
        value: *const anyopaque,
        type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult,
    param_get_public: *const fn (
        ctx: *anyopaque,
        value: *anyopaque,
        type: *ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult,
    param_set_dependency: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        type: ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult,
    param_get_dependency: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *anyopaque,
        type: *ParameterType,
        module: [*:0]const u8,
        parameter: [*:0]const u8,
    ) callconv(.C) c.FimoResult,
    param_set_private: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        type: ParameterType,
        parameter: *OpaqueParameter,
    ) callconv(.C) c.FimoResult,
    param_get_private: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *anyopaque,
        type: *ParameterType,
        parameter: *const OpaqueParameter,
    ) callconv(.C) c.FimoResult,
    param_set_inner: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *const anyopaque,
        type: ParameterType,
        parameter: *OpaqueParameterData,
    ) callconv(.C) c.FimoResult,
    param_get_inner: *const fn (
        ctx: *anyopaque,
        instance: *const OpaqueInstance,
        value: *anyopaque,
        type: *ParameterType,
        parameter: *const OpaqueParameterData,
    ) callconv(.C) c.FimoResult,
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
