const symbols = @import("symbols.zig");

/// A key for a task-specific-storage.
///
/// A new key can be defined by casting from a stable address.
pub fn Key(comptime T: type) type {
    return opaque {
        /// Associates a value with the key for the current task.
        ///
        /// The current value associated with the key is replaced with the new value without
        /// invoking any destructor function. The destructor function is set to `dtor`, and will
        /// be invoked upon task exit. May only be called by a task.
        pub fn set(
            self: *const @This(),
            value: ?*T,
            comptime dtor: ?fn (value: ?*T) void,
        ) void {
            const Wrapper = struct {
                fn dtorWrapper(v: ?*anyopaque) callconv(.c) void {
                    if (comptime dtor) |f| f(@ptrCast(@alignCast(v)));
                }
            };
            const sym = symbols.task_local_set.getGlobal().get();
            sym(
                @ptrCast(self),
                @ptrCast(value),
                if (comptime dtor != null) &Wrapper.dtorWrapper else null,
            );
        }

        /// Returns the value associated to the key for the current task.
        ///
        /// May only be called by a task.
        pub fn get(self: *const @This()) ?*T {
            const sym = symbols.task_local_get.getGlobal().get();
            return @ptrCast(@alignCast(sym(@ptrCast(self))));
        }

        /// Clears the value of the current task associated with the key.
        ///
        /// This operation invokes the associated destructor function and sets the value to `null`.
        /// May only be called by a task.
        pub fn clear(self: *const @This()) void {
            const sym = symbols.task_local_clear.getGlobal().get();
            sym(@ptrCast(self));
        }
    };
}

/// A key with an unknown value type.
pub const OpaqueKey = Key(anyopaque);
