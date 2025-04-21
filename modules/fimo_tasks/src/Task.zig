const std = @import("std");
const Allocator = std.mem.Allocator;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const Tracing = fimo_std.Context.Tracing;
const CallStack = Tracing.CallStack;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_pool = fimo_tasks_meta.pool;
const StackSize = meta_pool.StackSize;
const MetaWorker = meta_pool.Worker;
const meta_task = fimo_tasks_meta.task;
const MetaId = meta_task.Id;
const MetaTask = meta_task.OpaqueTask;

const CommandBuffer = @import("CommandBuffer.zig");
const context_ = @import("context.zig");
const Context = context_.Context;
const Transfer = context_.Transfer;
const Stack = context_.Stack;
const Pool = @import("Pool.zig");
const Worker = @import("Worker.zig");

const Self = @This();

state: State = .deinit,
allocator: Allocator,
id: MetaId,
worker: ?MetaWorker = null,
task: *MetaTask,
owner: *CommandBuffer,
entry_index: usize,
stack: Stack,
stack_size: StackSize,
context: ?Context = null,
call_stack: ?CallStack = null,
local_keys: AutoArrayHashMapUnmanaged(*const anyopaque, LocalData) = .empty,
msg: ?Pool.PrivateMessage = null,
next: ?*Self = null,

pub const State = enum {
    deinit,
    init,
    running,
    stopped,
};

pub const LocalData = struct {
    value: ?*anyopaque,
    dtor: ?*const fn (value: ?*anyopaque) callconv(.c) void,
};

/// Ensures that the task is in a state that allows for a context switch.
pub fn ensureReady(self: *Self) void {
    std.debug.assert(self.state != .stopped);
    std.debug.assert(self.msg == null);
    std.debug.assert(self.next == null);
    if (self.state == .init or self.state == .running) return;
    std.debug.assert(self.context == null);
    std.debug.assert(self.call_stack == null);

    self.context = Context.init(self.stack.memory, &Worker.taskEntry);
    if (self.owner.owner.runtime.tracing()) |tracing| self.call_stack = CallStack.init(tracing);
    self.state = .init;
}

/// Runs the cleanup operations that must occur while the task is running.
pub fn exit(self: *Self, is_abort: bool) void {
    self.clearAllLocals();
    if (is_abort) self.task.abort() else self.task.complete();
    self.state = .stopped;
}

/// Runs the cleanup operations that must occur while the task is not running.
pub fn afterExit(self: *Self, is_abort: bool) void {
    std.debug.assert(self.state == .stopped);
    if (self.call_stack) |cs| if (is_abort) cs.deinitAbort() else cs.deinit();
    std.debug.assert(self.context != null);
    self.context = null;
}

/// Associates a value with the key for the current task.
pub fn setLocal(
    self: *Self,
    key: *const anyopaque,
    value: LocalData,
) Allocator.Error!void {
    try self.local_keys.put(self.allocator, key, value);
}

/// Returns the value associated to the key for the current task.
pub fn getLocal(self: *Self, key: *const anyopaque) ?*anyopaque {
    const entry = self.local_keys.get(key) orelse return null;
    return entry.value;
}

/// Clears the value of the current task associated with the key.
pub fn clearLocal(self: *Self, key: *const anyopaque) void {
    const entry = self.local_keys.fetchSwapRemove(key) orelse return;
    if (entry.value.dtor) |dtor| dtor(entry.value.value);
}

/// Clears all local data slots.
fn clearAllLocals(self: *Self) void {
    while (self.local_keys.pop()) |entry| {
        if (entry.value.dtor) |dtor| dtor(entry.value.value);
    }
    self.local_keys.clearAndFree(self.allocator);
}
