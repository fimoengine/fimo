const std = @import("std");
const Thread = std.Thread;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const Context = fimo_std.Context;
const Tracing = Context.Tracing;
const Module = Context.Module;

const context = @import("context.zig");
const fimo_export = @import("fimo_export.zig");
const Instance = fimo_export.Instance;
const Futex = @import("Futex.zig");
const ParkingLot = @import("ParkingLot.zig");
const PoolMap = @import("PoolMap.zig");
const root = @import("root.zig");

const Self = @This();

allocator: Allocator,
futex: Futex,
lot: ParkingLot,
pool_map: PoolMap = .{},
instance: ?*const Module.OpaqueInstance = null,

/// Initializes a new unowned runtime instance.
pub fn init(allocator: Allocator) Self {
    return initInInstance(allocator, null);
}

/// Initializes a new runtime instance owned by the instance.
pub fn initInInstance(allocator: Allocator, instance: ?*const Instance) Self {
    return .{
        .allocator = allocator,
        .futex = .init(allocator),
        .lot = .init(allocator),
        .pool_map = .{},
        .instance = @ptrCast(instance),
    };
}

/// Deinitializes the runtime instance.
///
/// Blocks until all worker pools are joined.
pub fn deinit(self: *Self) void {
    self.pool_map.deinit(self.allocator);
    self.futex.deinit();
    self.lot.deinit();
}

/// Returns the owner instance.
pub fn getInstance(self: *Self) ?*const Instance {
    // const instance = self.instance orelse return null;
    return @ptrCast(@alignCast(self.instance));
}

/// Returns the default stack size for new worker pools.
pub fn getDefaultStackSize(self: *Self) usize {
    const min = context.StackAllocator.minStackSize();
    const max = context.StackAllocator.maxStackSize();

    if (self.getInstance()) |instance| {
        const parameters = instance.parameters();
        const default_stack_size = parameters.default_stack_size;
        const default: usize = @intCast(default_stack_size.read());
        if (default < min) return min;
        if (default > max) return max;
        return default;
    } else {
        if (fimo_export.default_stack_size < min) return min;
        if (fimo_export.default_stack_size > max) return max;
        return fimo_export.default_stack_size;
    }
}

/// Returns the default number of workers to spawn for new pools.
pub fn getDefaultWorkerCount(self: *Self) usize {
    const core_count = Thread.getCpuCount() catch 1;

    if (self.getInstance()) |instance| {
        const parameters = instance.parameters();
        const default_worker_count = parameters.default_worker_count;
        const count: usize = @intCast(default_worker_count.read());
        if (count == 0) return core_count;
        return count;
    } else {
        if (fimo_export.default_worker_count == 0) return core_count;
        return fimo_export.default_worker_count;
    }
}

/// Returns the tracing subsystem of the owner instance.
pub fn tracing(self: *Self) ?Tracing {
    const instance = self.getInstance() orelse return null;
    return instance.context().tracing();
}

/// Logs an error message.
pub fn logErr(
    self: *Self,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    if (self.tracing()) |tr| {
        tr.emitErrSimple(fmt, args, location);
    } else {
        std.log.err(fmt, args);
    }
}

/// Logs a debug message.
pub fn logDebug(
    self: *Self,
    comptime fmt: []const u8,
    args: anytype,
    location: std.builtin.SourceLocation,
) void {
    if (self.tracing()) |tr| {
        tr.emitDebugSimple(fmt, args, location);
    } else {
        std.log.debug(fmt, args);
    }
}
