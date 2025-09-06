const std = @import("std");

const time = @import("../../time.zig");
const Instant = time.Instant;
const pub_tracing = @import("../../tracing.zig");
const tracing = @import("../tracing.zig");

const Self = @This();

mutex: std.Thread.Mutex.Recursive = std.Thread.Mutex.Recursive.init,
state: packed struct(u8) {
    suspended: bool = true,
    blocked: bool = false,
    _padding: u6 = 0,
} = .{},
fmt_buffer: []u8,
max_level: tracing.Level,
frames: std.MultiArrayList(Frame) = .empty,

var dummy: Self = undefined;

pub const Frame = struct {
    id: *const tracing.EventInfo,
    previous_max_level: tracing.Level,
};

/// Creates a new empty call stack.
///
/// The call stack is marked as suspended.
pub fn init() *Self {
    if (!tracing.isEnabled()) return &dummy;

    const call_stack = tracing.allocator.create(Self) catch |err| @panic(@errorName(err));
    call_stack.* = Self{
        .fmt_buffer = undefined,
        .max_level = tracing.max_level,
    };
    tracing.call_stack_count.increase();

    const now = Instant.now().intoC();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.CreateCallStack{
            .time = now,
            .stack = call_stack,
        };
        subscriber.createCallStack(event);
    }

    return call_stack;
}

/// Creates a new empty call stack.
pub fn initBound(fmt_buffer: []u8) *Self {
    const self = init();
    self.mutex.lock();
    self.state.suspended = false;
    self.fmt_buffer = fmt_buffer;

    const thread_id = std.Thread.getCurrentId();
    const now = Instant.now().intoC();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.ResumeCallStack{
            .time = now,
            .stack = self,
            .thread_id = thread_id,
        };
        subscriber.resumeCallStack(event);
    }

    return self;
}

pub fn finishBound(self: *Self) void {
    if (!self.mutex.tryLock()) @panic("call stack in use");
    if (self.state.blocked) @panic("call stack is blocked");
    if (self.frames.len != 0) @panic("call stack is not empty");

    const now = Instant.now().intoC();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.DestroyCallStack{
            .time = now,
            .stack = self,
        };
        subscriber.destroyCallStack(event);
    }

    self.frames.deinit(tracing.allocator);
    tracing.allocator.destroy(self);
    tracing.call_stack_count.decrease();
}

/// Destroys an empty call stack.
pub fn finish(self: *Self) void {
    if (!tracing.isEnabled()) return;
    if (!self.mutex.tryLock()) @panic("call stack in use");
    if (self.mutex.lock_count != 1) @panic("call stack is bound");
    if (self.state.blocked) @panic("call stack is blocked");
    if (self.frames.len != 0) @panic("call stack is not empty");

    const now = Instant.now().intoC();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.DestroyCallStack{
            .time = now,
            .stack = self,
        };
        subscriber.destroyCallStack(event);
    }

    self.frames.deinit(tracing.allocator);
    tracing.allocator.destroy(self);
    tracing.call_stack_count.decrease();
}

/// Unwinds and destroys the call stack.
pub fn abort(self: *Self) void {
    if (!tracing.isEnabled()) return;
    if (!self.mutex.tryLock()) @panic("call stack in use");
    if (self.mutex.lock_count != 1) @panic("call stack is bound");

    const now = Instant.now().intoC();
    if (self.state.blocked) {
        for (tracing.subscribers) |subscriber| {
            const event = tracing.events.UnblockCallStack{
                .time = now,
                .stack = self,
            };
            subscriber.unblockCallStack(event);
        }
    }
    while (self.frames.pop()) |_| {
        for (tracing.subscribers) |subscriber| {
            const event = tracing.events.ExitSpan{
                .time = now,
                .stack = self,
                .is_unwinding = true,
            };
            subscriber.exitSpan(event);
        }
    }
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.DestroyCallStack{
            .time = now,
            .stack = self,
        };
        subscriber.destroyCallStack(event);
    }

    self.frames.deinit(tracing.allocator);
    tracing.allocator.destroy(self);
    tracing.call_stack_count.decrease();
}

/// Switches the call stack of the current thread.
pub fn swapCurrent(self: *Self) *Self {
    if (!tracing.isEnabled()) return self;
    if (!self.mutex.tryLock()) @panic("call stack in use");
    if (self.mutex.lock_count != 1) @panic("call stack is bound");
    if (self.state.blocked) @panic("call stack is blocked");
    if (!self.state.suspended) @panic("call stack is not suspended");

    if (!tracing.isEnabledForCurrentThread()) @panic("thread not registered");
    const data = tracing.ThreadData.get().?;
    const old = data.call_stack;
    data.call_stack = self;
    self.fmt_buffer = data.fmt_buffer;

    old.fmt_buffer = undefined;
    old.mutex.unlock();
    return old;
}

/// Unblocks the blocked call stack.
pub fn unblock(self: *Self) void {
    if (!tracing.isEnabled()) return;
    if (!self.mutex.tryLock()) @panic("call stack in use");
    defer self.mutex.unlock();
    if (self.mutex.lock_count != 1) @panic("call stack is bound");
    if (!self.state.blocked) @panic("call stack is not blocked");
    std.debug.assert(self.state.suspended);

    const now = Instant.now().intoC();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.UnblockCallStack{
            .time = now,
            .stack = self,
        };
        subscriber.unblockCallStack(event);
    }

    self.state.blocked = false;
}

/// Marks the current call stack as being suspended.
pub fn suspendCurrent(mark_blocked: bool) void {
    if (!tracing.isEnabled()) return;
    if (!tracing.isEnabledForCurrentThread()) @panic("thread not registered");
    const data = tracing.ThreadData.get().?;
    const self = data.call_stack;
    if (self.state.suspended) @panic("call stack is suspended");

    const now = Instant.now().intoC();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.SuspendCallStack{
            .time = now,
            .stack = self,
            .mark_blocked = mark_blocked,
        };
        subscriber.suspendCallStack(event);
    }
    self.state.suspended = true;
    self.state.blocked = mark_blocked;
}

/// Marks the current call stack as being resumed.
pub fn resumeCurrent() void {
    if (!tracing.isEnabled()) return;
    if (!tracing.isEnabledForCurrentThread()) @panic("thread not registered");
    const data = tracing.ThreadData.get().?;
    const self = data.call_stack;
    if (self.state.blocked) @panic("call stack is blocked");
    if (!self.state.suspended) @panic("call stack is not suspended");

    const now = Instant.now().intoC();
    const thread_id = std.Thread.getCurrentId();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.ResumeCallStack{
            .time = now,
            .stack = self,
            .thread_id = thread_id,
        };
        subscriber.resumeCallStack(event);
    }
    self.state.suspended = false;
}

pub fn enterFrame(
    self: *Self,
    id: *const tracing.EventInfo,
    formatter: *const tracing.Formatter,
    formatter_data: *const anyopaque,
) void {
    if (!self.mutex.tryLock()) @panic("call stack in use");
    defer self.mutex.unlock();
    if (self.mutex.lock_count == 1) @panic("call stack is not bound");
    if (self.state.blocked) @panic("call stack is blocked");
    if (self.state.suspended) @panic("call stack is suspended");

    if (tracing.event_info_cache.cacheInfo(id)) for (tracing.subscribers) |subscriber| {
        const event = tracing.events.DeclareEventInfo{ .info = id };
        subscriber.declareEventInfo(event);
    };

    const now = Instant.now().intoC();
    const message_len = formatter(formatter_data, self.fmt_buffer.ptr, self.fmt_buffer.len);
    const message = self.fmt_buffer[0..message_len];
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.EnterSpan{
            .time = now,
            .stack = self,
            .span = id,
            .message = .fromSlice(message),
        };
        subscriber.enterSpan(event);
    }

    self.frames.append(
        tracing.allocator,
        .{ .id = id, .previous_max_level = self.max_level },
    ) catch |e| @panic(@errorName(e));
    const max_lvl_int = @intFromEnum(self.max_level);
    const event_lvl_int = @intFromEnum(id.level);
    self.max_level = @enumFromInt(@min(max_lvl_int, event_lvl_int));
}

pub fn exitFrame(self: *Self, id: *const tracing.EventInfo) void {
    if (!self.mutex.tryLock()) @panic("call stack in use");
    defer self.mutex.unlock();
    if (self.mutex.lock_count == 1) @panic("call stack is not bound");
    if (self.state.blocked) @panic("call stack is blocked");
    if (self.state.suspended) @panic("call stack is suspended");

    const frame = self.frames.pop() orelse @panic("span is not on top");
    if (frame.id != id) @panic("span is not on top");
    self.max_level = frame.previous_max_level;

    const now = Instant.now().intoC();
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.ExitSpan{
            .time = now,
            .stack = self,
            .is_unwinding = false,
        };
        subscriber.exitSpan(event);
    }
}

pub fn logMessage(
    self: *Self,
    info: *const tracing.EventInfo,
    formatter: *const tracing.Formatter,
    formatter_data: *const anyopaque,
) void {
    if (!self.mutex.tryLock()) @panic("call stack in use");
    defer self.mutex.unlock();
    if (self.mutex.lock_count == 1) @panic("call stack is not bound");
    if (self.state.blocked) @panic("call stack is blocked");
    if (self.state.suspended) @panic("call stack is suspended");

    const max_lvl_int = @intFromEnum(self.max_level);
    const event_lvl_int = @intFromEnum(info.level);
    if (event_lvl_int > max_lvl_int) return;

    if (tracing.event_info_cache.cacheInfo(info)) for (tracing.subscribers) |subscriber| {
        const event = tracing.events.DeclareEventInfo{ .info = info };
        subscriber.declareEventInfo(event);
    };

    const now = Instant.now().intoC();
    const message_len = formatter(formatter_data, self.fmt_buffer.ptr, self.fmt_buffer.len);
    const message = self.fmt_buffer[0..message_len];
    for (tracing.subscribers) |subscriber| {
        const event = tracing.events.LogMessage{
            .time = now,
            .stack = self,
            .info = info,
            .message = .fromSlice(message),
        };
        subscriber.logMessage(event);
    }
}
