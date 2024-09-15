const std = @import("std");
const builtin = @import("builtin");

const c = @import("../c.zig");

const exports_section = switch (builtin.target.os.tag) {
    .macos, .ios, .watchos, .tvos, .visionos => struct {
        const start_exports = @extern(
            [*]const ?*const c.FimoModuleExport,
            .{ .name = "_start_fimo_module" },
        );
        const stop_exports = @extern(
            [*]const ?*const c.FimoModuleExport,
            .{ .name = "_stop_fimo_module" },
        );
        const export_visibility = .hidden;
        // Make shure that the section is created.
        comptime {
            asm (
                \\.global __start_fimo_module
                \\__start_fimo_module = section$start$__DATA$__fimo_module
                \\
                \\.global __stop_fimo_module
                \\__stop_fimo_module = section$end$__DATA$__fimo_module
            );
            exportModuleInner(null);
        }
    },
    .windows => struct {
        const a: ?*const c.FimoModuleExport = null;
        const z: ?*const c.FimoModuleExport = null;

        const start_exports: [*]const ?*const c.FimoModuleExport = @ptrCast(&a);
        const stop_exports: [*]const ?*const c.FimoModuleExport = @ptrCast(&z);
        const export_visibility = .global;

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
        extern const __start_fimo_module: ?*const c.FimoModuleExport;
        extern const __stop_fimo_module: ?*const c.FimoModuleExport;

        const start_exports: [*]const ?*const c.FimoModuleExport = @ptrCast(
            &__start_fimo_module,
        );
        const stop_exports: [*]const ?*const c.FimoModuleExport = @ptrCast(
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

/// Creates a new unique export in the correct section.
///
/// For internal use only, as the pointer should not generally be null.
fn exportModuleInner(comptime module: ?*const c.FimoModuleExport) void {
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

pub const ExportModuleParamAccess = enum {
    public,
    dependency,
    private,
};

pub const ExportModuleParameterInfo = struct {
    read: ExportModuleParamAccess = .private,
    write: ExportModuleParamAccess = .private,
    value: union(enum) {
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

pub fn exportModule(
    comptime name: [:0]const u8,
    comptime description: ?[:0]const u8,
    comptime author: ?[:0]const u8,
    comptime license: ?[:0]const u8,
    comptime parameters: anytype,
    comptime resources: anytype,
    comptime namespaces: anytype,
    comptime imports: anytype,
    comptime exports: anytype,
) type {
    _ = name;
    _ = description;
    _ = author;
    _ = license;
    _ = parameters;
    _ = resources;
    _ = namespaces;
    _ = imports;
    _ = exports;

    comptime {
        return extern struct {};
    }
}

/// Iterator over all exports of the current binary.
pub const ExportIter = struct {
    /// Iterator position. Does not necessarily point to a valid export.
    position: [*]const ?*const c.FimoModuleExport,

    /// Initializes the iterator. Does not need to be deinitialized.
    pub fn init() @This() {
        return .{
            .position = @ptrCast(exports_section.start_exports),
        };
    }

    /// Returns the next export in the export link.
    pub fn next(self: *@This()) ?*const c.FimoModuleExport {
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
};

export fn fimo_impl_module_export_iterator(inspector: c.FimoImplModuleInspector, data: ?*anyopaque) void {
    if (inspector) |insp| {
        var it = ExportIter.init();
        while (it.next()) |exp| {
            if (!insp(exp, data)) {
                return;
            }
        }
    }
}
