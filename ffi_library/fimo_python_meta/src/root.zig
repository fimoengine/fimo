const std = @import("std");

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const Module = fimo_std.Context.Module;

const Root = @This();

/// Raw c definitions.
pub const c = @cImport({
    @cInclude("fimo_python_meta/loader.h");
});

/// Symbols defined by the module.
pub const symbols = struct {
    /// Info of the `run_string` symbol.
    pub const RunString = Module.Symbol{
        .name = c.FIPY_SYMBOL_NAME_RUN_STRING,
        .namespace = c.FIPY_SYMBOL_NAMESPACE,
        .version = .{
            .major = c.FIPY_SYMBOL_VERSION_MAJOR_RUN_STRING,
            .minor = c.FIPY_SYMBOL_VERSION_MINOR_RUN_STRING,
            .patch = c.FIPY_SYMBOL_VERSION_PATCH_RUN_STRING,
        },
        .T = Root.RunString,
    };
};

/// The `run_string` symbol.
pub const RunString = extern struct {
    data: ?*anyopaque,
    call_f: *const fn (
        data: ?*anyopaque,
        code: [*:0]const u8,
        home: ?[*:0]const u8,
    ) callconv(.C) fimo_std.AnyError.AnyResult,

    /// Executes the passed string in the embedded python interpreter.
    ///
    /// This function spawns a new isolated subinterpreter and executes the provided string.
    /// The interpreter only has access to the builtin Python modules. By setting `home` to a
    /// value other than `null`, the caller can append a custom path to the module search path
    /// of the new subinterpreter. This allows for the import of custom python modules.
    pub fn call(
        self: *const RunString,
        code: [:0]const u8,
        home: ?[:0]const u8,
        err: *?AnyError,
    ) AnyError.Error!void {
        try self.call_f(self.data, code.ptr, if (home) |h| h.ptr else null).intoErrorUnion(err);
    }
};
