const std = @import("std");

const c = @import("c.zig");

pub const exports = @import("module/exports.zig");

/// A parameter of a module.
fn Parameter(comptime module: type, comptime value: type) type {
    _ = module;
    return struct {
        const Self = @This();

        /// Type stored in the parameter
        pub const ValueT = value;
    };
}

/// Generates the module parameter table.
fn ParameterTable(comptime module: type, comptime parameters: anytype) type {
    return @Type(.{
        .@"struct" = .{
            .layout = .@"extern",
            .fields = blk: {
                const src_fields = @typeInfo(@TypeOf(parameters)).@"struct".fields;
                var dst_fields: [src_fields.len]std.builtin.Type.StructField = undefined;
                for (src_fields, &dst_fields) |src, *dst| {
                    const src_type = @field(parameters, src.name);
                    dst.name = src.name;
                    dst.type = switch (src_type) {
                        i8, i16, i32, i64, u8, u16, u32, u64 => |t| *Parameter(module, t),
                        else => @compileError("invalid parameter value type " ++ @typeName(src_type)),
                    };
                    dst.default_value = null;
                    dst.is_comptime = false;
                    dst.alignment = @alignOf(*void);
                }
                break :blk &dst_fields;
            },
            .decls = &.{},
            .is_tuple = false,
        },
    });
}

/// Generates the module resource table.
fn ResourceTable(comptime resources: []const []const u8) type {
    return @Type(.{
        .@"struct" = .{
            .layout = .@"extern",
            .fields = blk: {
                var fields: [resources.len]std.builtin.Type.StructField = undefined;
                for (&fields, resources) |*field, name| {
                    field.name = name ++ "";
                    field.type = [*:0]u8;
                    field.default_value = null;
                    field.is_comptime = false;
                    field.alignment = @alignOf([*:0]u8);
                }
                break :blk &fields;
            },
            .decls = &.{},
            .is_tuple = false,
        },
    });
}

fn SymbolTable(comptime symbols: anytype) type {
    return @Type(.{
        .@"struct" = .{
            .layout = .@"extern",
            .fields = blk: {
                const src_fields = @typeInfo(@TypeOf(symbols)).@"struct".fields;
                var dst_fields: [src_fields.len]std.builtin.Type.StructField = undefined;
                for (src_fields, &dst_fields) |src, *dst| {
                    const src_type = @field(symbols, src.name);
                    dst.name = src.name;
                    dst.type = *const src_type;
                    dst.default_value = null;
                    dst.is_comptime = false;
                    dst.alignment = @alignOf(*void);
                }
                break :blk &dst_fields;
            },
            .decls = &.{},
            .is_tuple = false,
        },
    });
}

/// A module declaration.
///
/// # Note
///
/// This only provides the definition of the module type
/// and does not export it.
pub fn Module(
    comptime parameters_decl: anytype,
    comptime resource_names: []const []const u8,
    comptime imports_decl: anytype,
    comptime exports_decl: anytype,
    comptime T: type,
) type {
    return extern struct {
        module: c.FimoModule,

        const Self = @This();
        /// Type of the parameter table.
        pub const Parameters = ParameterTable(Self, parameters_decl);
        /// Type of the resource table.
        pub const Resources = ResourceTable(resource_names);
        /// Type of the import table.
        pub const Imports = SymbolTable(imports_decl);
        /// Type of the export table.
        pub const Exports = SymbolTable(exports_decl);
        /// Data type of the module state.
        pub const State = T;

        // Force the evaluation of the table functions.
        comptime {
            _ = Parameters;
            _ = Resources;
            _ = Imports;
            _ = Exports;
            _ = State;
        }

        /// Returns the parameter table of the module.
        ///
        /// # Note
        ///
        /// The pointer is guaranteed to point to a valid address, if
        /// the module declaration specified any parameters.
        pub fn parameters(self: *const Self) *const Parameters {
            const ptr: ?*const Parameters = @ptrCast(@alignCast(self.module.parameters));
            if (ptr) |p| return p;
            return @ptrFromInt(@alignOf(*const Parameters));
        }

        /// Returns the resource table of the module.
        ///
        /// # Note
        ///
        /// The pointer is guaranteed to point to a valid address, if
        /// the module declaration specified any resources.
        pub fn resources(self: *const Self) *const Resources {
            const ptr: ?*const Resources = @ptrCast(@alignCast(self.module.resources));
            if (ptr) |p| return p;
            return @ptrFromInt(@alignOf(*const Resources));
        }

        /// Returns the import table of the module.
        ///
        /// # Note
        ///
        /// The pointer is guaranteed to point to a valid address, if
        /// the module declaration specified any imports.
        pub fn imports(self: *const Self) *const Imports {
            const ptr: ?*const Imports = @ptrCast(@alignCast(self.module.imports));
            if (ptr) |p| return p;
            return @ptrFromInt(@alignOf(*const Imports));
        }

        /// Returns the export table of the module.
        ///
        /// # Note
        ///
        /// The pointer is guaranteed to point to a valid address, if
        /// the module declaration specified any exports.
        pub fn exports(self: *const Self) *const Exports {
            const ptr: ?*const Exports = @ptrCast(@alignCast(self.module.exports));
            if (ptr) |p| return p;
            return @ptrFromInt(@alignOf(*const Exports));
        }

        /// Returns the export table of the module.
        ///
        /// # Note
        ///
        /// The pointer is guaranteed to point to a valid address, if
        /// the module declaration specified any exports.
        pub fn state(self: *const Self) *const State {
            const ptr: ?*const State = @ptrCast(@alignCast(self.module.module_data));
            if (ptr) |p| return p;
            return @ptrFromInt(@alignOf(*const State));
        }

        /// Casts the module to an opaque module.
        pub fn asOpaqueModule(self: *const Self) *const OpaqueModule {
            return @ptrCast(self);
        }
    };
}

/// A type-erased module.
pub const OpaqueModule = Module(
    .{},
    &.{},
    .{},
    .{},
    anyopaque,
);

/// Info of a loaded module.
pub const ModuleInfo = extern struct {
    info: c.FimoModuleInfo,

    /// Module name.
    pub fn name(self: *const ModuleInfo) [*:0]const u8 {
        return self.info.name;
    }

    /// Module description.
    pub fn description(self: *const ModuleInfo) ?[*:0]const u8 {
        return self.info.description;
    }

    /// Module author.
    pub fn author(self: *const ModuleInfo) ?[*:0]const u8 {
        return self.info.author;
    }

    /// Module license.
    pub fn license(self: *const ModuleInfo) ?[*:0]const u8 {
        return self.info.license;
    }

    /// Path to the module directory.
    pub fn root_path(self: *const ModuleInfo) [*:0]const u8 {
        return self.info.module_path;
    }
};

test {
    const module = try std.testing.allocator.create(OpaqueModule);
    defer std.testing.allocator.destroy(module);
    const a = module.asOpaqueModule();
    std.debug.print("{*}\n", .{a});

    const p = a.parameters();
    std.debug.print("{}\n", .{p});
    const r = a.resources();
    std.debug.print("{}\n", .{r});
    const i = a.imports();
    std.debug.print("{}\n", .{i});
    const e = a.exports();
    std.debug.print("{}\n", .{e});
    const s = a.state();
    std.debug.print("{}\n", .{s});
}

// Force the inclusion of the required symbols.
comptime {
    _ = exports;
}
