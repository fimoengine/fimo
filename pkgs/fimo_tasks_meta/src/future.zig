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
    /// Allocator used to spawn to future.
    ///
    /// Must outlive the future handle.
    allocator: Allocator,
    /// Label of the underlying future command buffer.
    label: ?[]const u8 = null,
    /// Minimum stack size of the future.
    stack_size: ?StackSize = null,
    /// Worker to assign the future to.
    worker: ?Worker = null,
    /// List of dependencies to wait for before starting the future.
    ///
    /// The future will be aborted if any one dependency fails.
    /// Each handle must belong to the same pool as the one the future will be spawned in.
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
            enqueuePool: Pool,
            function: anytype,
            args: std.meta.ArgsTuple(@TypeOf(function)),
            options: SpawnFutureOptions,
            err: *?AnyError,
        ) (Allocator.Error || AnyError.Error)!@This() {
            const TaskState = extern struct {
                inner: [@sizeOf(Inner)]u8 align(@alignOf(Inner)),
                const Inner = struct {
                    args: @TypeOf(args),
                    result: Result = undefined,
                };
                fn getInner(self: *@This()) *Inner {
                    return std.mem.bytesAsValue(Inner, self.inner[0..]);
                }
            };
            const State = extern struct {
                inner: [@sizeOf(Inner)]u8 align(@alignOf(Inner)),
                const Inner = struct {
                    allocator: Allocator,
                    task: Task(TaskState) = undefined,
                };
                fn getInner(self: *@This()) *Inner {
                    return std.mem.bytesAsValue(Inner, self.inner[0..]);
                }
            };

            const label_len = if (options.label) |l| l.len else 0;
            const num_entries = blk: {
                var num: usize = 2 + options.dependencies.len;
                if (options.stack_size != null) num += 1;
                if (options.worker != null) num += 1;
                break :blk num;
            };

            const label_start = @sizeOf(CommandBuffer(State));
            const entries_start = std.mem.alignForward(usize, label_start + label_len, @alignOf(Entry));
            const full_bytes_len = entries_start + (num_entries * @sizeOf(Entry));
            const alloc = try options.allocator.alignedAlloc(u8, .of(CommandBuffer(State)), full_bytes_len);

            const buffer: *CommandBuffer(State) = std.mem.bytesAsValue(
                CommandBuffer(State),
                alloc[0..@sizeOf(CommandBuffer(State))],
            );
            const label: []u8 = alloc[label_start .. label_start + label_len];
            const entries: []Entry = @alignCast(std.mem.bytesAsSlice(Entry, alloc[entries_start..]));
            std.mem.copyForwards(u8, label, options.label orelse "");

            buffer.* = .{
                .label_ = label.ptr,
                .label_len = label.len,
                .entries_ = entries.ptr,
                .entries_len = entries.len,
                .on_deinit = &struct {
                    fn f(b: *CommandBuffer(State)) callconv(.c) void {
                        const al = b.state.getInner().allocator;
                        const alloc_len = std.mem.alignForward(
                            usize,
                            @sizeOf(CommandBuffer(State)) + b.label_len,
                            @alignOf(Entry),
                        ) + (b.entries_len * @sizeOf(Entry));
                        const bytes = std.mem.asBytes(b).ptr[0..alloc_len];
                        al.free(bytes);
                    }
                }.f,
                .state = .{
                    .inner = std.mem.toBytes(State.Inner{ .allocator = options.allocator }),
                },
            };
            buffer.state.getInner().task = .{
                .on_start = &struct {
                    fn f(t: *Task(TaskState)) callconv(.c) void {
                        const inner = t.state.getInner();
                        inner.result = @call(.auto, function, inner.args);
                    }
                }.f,
                .state = .{ .inner = std.mem.toBytes(TaskState.Inner{ .args = args }) },
            };

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
                .payload = .{ .enqueue_task = @ptrCast(&buffer.state.getInner().task) },
            });

            const handle = try enqueuePool.enqueueCommandBuffer(@ptrCast(buffer), err);
            const result_ptr = &buffer.state.getInner().task.state.getInner().result;
            return .{ .handle = handle, .result = result_ptr };
        }
    };
}
