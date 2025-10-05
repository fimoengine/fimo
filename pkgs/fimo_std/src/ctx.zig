const std = @import("std");
const builtin = @import("builtin");

const context_version_opt = @import("context_version");

const AnyError = @import("AnyError.zig");
const AnyResult = AnyError.AnyResult;
const Inner = @import("context.zig");
const memory = @import("memory.zig");
const Arena = memory.Arena;
const Allocator = memory.Allocator;
const modules = @import("modules.zig");
const tasks = @import("tasks.zig");
const tracing = @import("tracing.zig");
const Version = @import("Version.zig");

/// Interface version compiled against.
pub const context_version = Version.initSemanticVersion(context_version_opt.version);

pub const CfgId = enum(i32) {
    _unknown = 0,
    core = 1,
    tracing = 2,
    modules = 3,
    _,
};

/// Common member of all config structures.
pub const Cfg = extern struct {
    id: CfgId,
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
    pub fn isOk(self: Status) bool {
        return @intFromEnum(self) >= 0;
    }

    /// Checks if the status indicates an error.
    pub fn isErr(self: Status) bool {
        return @intFromEnum(self) < 0;
    }

    /// Constructs an error union from the status.
    pub fn intoErrorUnion(self: Status) Error!void {
        if (self.isOk()) return;
        return switch (self) {
            .err => error.OperationFailed,
            else => error.UnknownError,
        };
    }
};

/// Error type of the context.
pub const Error = error{
    OperationFailed,
    UnknownError,
};

/// Handle to the global functions implemented by the context.
///
/// Is not intended to be instantiated outside of the current module, as it may gain additional
/// fields without being considered a breaking change.
pub const Handle = extern struct {
    get_version: *const fn () callconv(.c) Version.compat.Version,
    core_v0: CoreVTable,
    tracing_v0: tracing.VTable,
    modules_v0: modules.VTable,
    tasks_v0: tasks.VTable,

    var lock: std.Thread.Mutex = .{};
    var count: usize = 0;
    var global: ?*@This() = null;

    pub fn registerHandle(handle: *@This()) void {
        std.debug.assert(Version.initC(handle.get_version()).sattisfies(context_version));
        lock.lock();
        defer lock.unlock();

        if (global) |old| {
            std.debug.assert(old == handle and count != 0);
            count += 1;
        } else {
            std.debug.assert(count == 0);
            global = handle;
            count = 1;
        }
    }

    pub fn unregisterHandle() void {
        lock.lock();
        defer lock.unlock();

        count -= 1;
        if (count == 0) {
            std.debug.assert(global != null);
            global = null;
        }
    }

    pub fn getHandle() *@This() {
        return global orelse unreachable;
    }
};

pub const CoreCfg = extern struct {
    cfg: Cfg = .{ .id = .core },
    /// Reserve size of the global arena.
    global_arena_reserve: usize = 0,
    /// Initial commit size of the global arena.
    global_arena_commit: usize = 0,
    /// Reserve size of the per-thread scratch arenas.
    scratch_arena_reserve: usize = 0,
    /// Initial commit size of the per-thread scratch arenas.
    scratch_arena_commit: usize = 0,
};

/// Base VTable of the context.
///
/// Changing this definition is a breaking change.
pub const CoreVTable = extern struct {
    deinit: *const fn () callconv(.c) void,
    get_global_arena: *const fn () callconv(.c) *Arena,
    get_page_allocator: *const fn () callconv(.c) Allocator,
    get_scratch_arena: *const fn (conflict: ?*Arena) callconv(.c) *Arena,
    set_custom_scratch_arenas: *const fn (first: *Arena, second: *Arena) callconv(.c) void,
    unset_custom_scratch_arenas: *const fn () callconv(.c) void,
    has_error_result: *const fn () callconv(.c) bool,
    replace_result: *const fn (new: AnyResult) callconv(.c) AnyResult,
};

/// Initializes a new context with the given options.
///
/// Only one context may be initialized at any given moment.
pub fn init(options: []const *const Cfg) !void {
    try Inner.init(options);
    Handle.registerHandle(&Inner.handle);
}

/// Deinitializes the global context.
///
/// May block until all resources owned by the context are shut down.
pub fn deinit() void {
    const handle = Handle.getHandle();
    handle.core_v0.deinit();
    Handle.unregisterHandle();
    std.debug.assert(Handle.global == null);
}

/// Checks whether the context is initialized.
pub fn isInit() bool {
    return Handle.global != null;
}

/// Returns the version of the initialized context.
///
/// May differ from the one specified during compilation.
pub fn getVersion() Version {
    const handle = Handle.getHandle();
    return Version.initC(handle.get_version());
}

/// Returns the global arena shared by all threads.
pub fn getGlobalArena() *Arena {
    const handle = Handle.getHandle();
    return handle.core_v0.get_global_arena();
}

/// Returns an allocator for allocating buffers at the page granularity of the current system.
///
/// The allocator is backed by the global arena. The allocator implementation is allowed to
/// request chunks larger than one page size from the arena. The minimum alignment of an
/// allocation is the page size.
pub fn getPageAllocator() Allocator {
    const handle = Handle.getHandle();
    return handle.core_v0.get_page_allocator();
}

/// Returns the scratch arena for the current thread.
///
/// The scratch arena will be initialized the first time the thread calls this function.
/// The arena is owned by the calling thread and will be invalidated on thread exit
/// or after the context is deinitialized.
pub fn getScratchArena(conflict: ?*Arena) *Arena {
    const handle = Handle.getHandle();
    return handle.core_v0.get_scratch_arena(conflict);
}

/// Sets a custom scratch arena set for the calling thread.
///
/// A usecase for this function is exposing custom arenas for worker threads in a job system.
/// Extending the size of the preconfigured scratch arenas is not an indented usecase.
/// May only be called if no custom arenas are currently set.
pub fn setCustomScratchArenas(first: *Arena, second: *Arena) void {
    const handle = Handle.getHandle();
    return handle.core_v0.set_custom_scratch_arenas(first, second);
}

/// Removes the custom scratch arenas for the calling thread.
///
/// After this call `getScratchArena` will return one of the arenas managed by the context.
/// A custom arena pair must have been set previously.
pub fn unsetCustomScratchArenas() void {
    const handle = Handle.getHandle();
    return handle.core_v0.unset_custom_scratch_arenas();
}

/// Checks whether the context has an error stored for the current thread.
pub fn hasErrorResult() bool {
    const handle = Handle.getHandle();
    return handle.core_v0.has_error_result();
}

/// Replaces the thread-local result stored in the context with a new one.
///
/// The old result is returned.
pub fn replaceResult(new: AnyResult) AnyResult {
    const handle = Handle.getHandle();
    return handle.core_v0.replace_result(new);
}

/// Swaps out the thread-local result with the `ok` result.
pub fn takeResult() AnyResult {
    return replaceResult(.ok);
}

/// Clears the thread-local result.
pub fn clearResult() void {
    takeResult().deinit();
}

/// Sets the thread-local result, destroying the old one.
pub fn setResult(new: AnyResult) void {
    replaceResult(new).deinit();
}

test "context: local error" {
    try init(&.{});
    var is_init = true;
    errdefer if (is_init) deinit();

    try std.testing.expect(!hasErrorResult());
    try std.testing.expectEqual(
        replaceResult(.initErr(AnyError.initError(error.FfiError))),
        AnyResult.ok,
    );
    try std.testing.expect(hasErrorResult());

    const Runner = struct {
        thread: std.Thread,
        err: ?anyerror = null,

        var event = std.Thread.ResetEvent{};

        fn run(self: *@This()) !void {
            errdefer |err| self.err = err;
            try std.testing.expect(!hasErrorResult());
            try std.testing.expectEqual(
                replaceResult(.initErr(AnyError.initError(error.FfiError))),
                AnyResult.ok,
            );
            try std.testing.expect(hasErrorResult());
            event.set();
        }
    };
    var runner = Runner{ .thread = undefined };
    runner.thread = try std.Thread.spawn(.{}, Runner.run, .{&runner});

    // Deinit the context first to test whether it blocks until the resource
    // is cleaned up by the thread.
    Runner.event.wait();
    deinit();
    is_init = false;

    runner.thread.join();
    if (runner.err) |err| return err;
}
