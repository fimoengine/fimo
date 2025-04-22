const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayListUnmanaged = std.ArrayListUnmanaged;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;

const command_buffer = @import("command_buffer.zig");
const Entry = command_buffer.Entry;
const Handle = command_buffer.Handle;
const CommandBuffer = command_buffer.CommandBuffer;
const OpaqueCommandBuffer = command_buffer.OpaqueCommandBuffer;
const pool = @import("pool.zig");
const StackSize = pool.StackSize;
const Worker = pool.Worker;
const Pool = pool.Pool;
const task = @import("task.zig");
const Task = task.Task;

/// Options for spawning new futures.
pub const SpawnFutureOptions = struct {
    label: ?[]const u8 = null,
    stack_size: ?StackSize = null,
    worker: ?Worker = null,
    dependencies: []const *const Handle = &.{},
};

/// An utility type to spawn single task command buffers.
pub fn Future(Result: type) type {
    return struct {
        handle: Handle,
        result: *const Result,

        const AwaitResult = switch (@typeInfo(Result)) {
            .error_set => error{Aborted} || Result,
            .error_union => |x| anyerror!x.payload,
            else => error{Aborted}!Result,
        };

        /// Deinitializes the future.
        ///
        /// If the future has not yet completed running, it will be detached.
        /// The result of the future is not cleaned up.
        pub fn deinit(self: *const @This()) void {
            self.handle.unref();
        }

        /// Awaits for the completion of the future and returns the result.
        pub fn @"await"(self: *const @This()) AwaitResult {
            return switch (self.handle.waitOn()) {
                .completed => self.result.*,
                .aborted => error.Aborted,
            };
        }

        /// Spawns a new future in the provided pool.
        pub fn spawn(
            allocator: Allocator,
            enqueuePool: Pool,
            function: anytype,
            args: std.meta.ArgsTuple(@TypeOf(function)),
            options: SpawnFutureOptions,
            err: *?AnyError,
        ) (Allocator.Error || AnyError.Error)!@This() {
            const TaskState = extern struct {
                args: [@sizeOf(@TypeOf(args))]u8 align(@alignOf(@TypeOf(args))),
                result: [@sizeOf(Result)]u8 align(@alignOf(Result)),
            };
            const State = extern struct {
                allocator: [@sizeOf(Allocator)]u8 align(@alignOf(Allocator)),
                task: Task(TaskState),
            };
            var skip_deinit = false;
            const buffer = try allocator.create(CommandBuffer(State));
            errdefer if (!skip_deinit) {
                for (buffer.entries()) |entry| entry.abort();
                buffer.deinit();
            };
            buffer.* = .{
                .on_deinit = &struct {
                    fn f(b: *CommandBuffer(State)) callconv(.c) void {
                        const al = std.mem.bytesToValue(Allocator, b.state.allocator[0..]);
                        if (b.label_) |l| al.free(l[0..b.label_len]);
                        if (b.entries_) |e| al.free(e[0..b.entries_len]);
                        al.destroy(b);
                    }
                }.f,
                .state = .{
                    .allocator = std.mem.toBytes(allocator),
                    .task = undefined,
                },
            };
            buffer.state.task = .{
                .on_start = &struct {
                    fn f(t: *Task(TaskState)) callconv(.c) void {
                        const result_ptr = std.mem.bytesAsValue(Result, t.state.result[0..]);
                        const args_ptr = std.mem.bytesAsValue(@TypeOf(args), t.state.args[0..]);
                        result_ptr.* = @call(.auto, function, args_ptr.*);
                    }
                }.f,
                .state = .{
                    .args = std.mem.toBytes(args),
                    .result = std.mem.toBytes(@as(Result, undefined)),
                },
            };

            if (options.label) |label| {
                const dupe = try allocator.dupe(u8, label);
                buffer.label_ = dupe.ptr;
                buffer.label_len = dupe.len;
            }

            var num_entries: usize = 2 + options.dependencies.len;
            if (options.stack_size != null) num_entries += 1;
            if (options.worker != null) num_entries += 1;

            const entries = try allocator.alloc(Entry, num_entries);
            for (entries) |*entry| entry.* = .{
                .tag = .abort_on_error,
                .payload = .{ .abort_on_error = false },
            };

            buffer.entries_ = entries.ptr;
            buffer.entries_len = entries.len;

            var entries_al = ArrayListUnmanaged(Entry).initBuffer(entries);
            entries_al.appendAssumeCapacity(.{
                .tag = .abort_on_error,
                .payload = .{ .abort_on_error = true },
            });
            if (options.stack_size) |stack_size| entries_al.appendAssumeCapacity(.{
                .tag = .set_min_stack_size,
                .payload = .{ .set_min_stack_size = stack_size },
            });
            if (options.worker) |worker| entries_al.appendAssumeCapacity(.{
                .tag = .select_worker,
                .payload = .{ .select_worker = worker },
            });
            for (options.dependencies) |handle| entries_al.appendAssumeCapacity(.{
                .tag = .wait_on_command_buffer,
                .payload = .{ .wait_on_command_buffer = handle.ref() },
            });
            entries_al.appendAssumeCapacity(.{
                .tag = .enqueue_task,
                .payload = .{ .enqueue_task = @ptrCast(&buffer.state.task) },
            });

            skip_deinit = true;
            const handle = try enqueuePool.enqueueCommandBuffer(@ptrCast(buffer), err);
            const result_ptr = std.mem.bytesAsValue(Result, buffer.state.task.state.result[0..]);
            return .{ .handle = handle, .result = result_ptr };
        }
    };
}
