const std = @import("std");

const c = @import("../../c.zig");
const AnyError = @import("../../AnyError.zig");
const time = @import("../../time.zig");

const Tracing = @import("../tracing.zig");
const ProxyTracing = @import("../proxy_context/tracing.zig");

const TracingError = Tracing.TracingError;
const StackFrame = @import("StackFrame.zig");

const Self = @This();

mutex: std.Thread.Mutex.Recursive = std.Thread.Mutex.Recursive.init,
state: packed struct(u8) {
    suspended: bool = true,
    blocked: bool = false,
    _padding: u6 = 0,
} = .{},
buffer: []u8,
cursor: usize = 0,
max_level: ProxyTracing.Level,
call_stacks: std.ArrayListUnmanaged(*anyopaque),
start_frame: ?*StackFrame = null,
end_frame: ?*StackFrame = null,
owner: *const Tracing,

pub fn init(
    owner: *const Tracing,
    err: *?AnyError,
) (TracingError || AnyError.Error)!*Self {
    const call_stack = try owner.allocator.create(Self);
    errdefer owner.allocator.destroy(call_stack);
    call_stack.* = Self{
        .buffer = undefined,
        .max_level = owner.max_level,
        .call_stacks = undefined,
        .owner = owner,
    };

    call_stack.buffer = try owner.allocator.alloc(u8, owner.buffer_size);
    errdefer owner.allocator.free(call_stack.buffer);

    call_stack.call_stacks = try std.ArrayListUnmanaged(*anyopaque).initCapacity(
        owner.allocator,
        owner.subscribers.len,
    );
    errdefer {
        call_stack.call_stacks.deinit(owner.allocator);
        for (call_stack.call_stacks.items, owner.subscribers) |cs, subscriber| {
            subscriber.dropCallStack(cs);
        }
    }

    const now = time.Time.now();
    for (owner.subscribers) |subscriber| {
        const cs = try subscriber.createCallStack(now, err);
        call_stack.call_stacks.appendAssumeCapacity(cs);
    }

    return call_stack;
}

pub fn deinit(self: *Self) void {
    if (!self.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    if (self.state.blocked) @panic(@errorName(error.CallStackBlocked));
    if (self.end_frame != null) @panic(@errorName(error.CallStackNotEmpty));

    const now = time.Time.now();
    for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
        subscriber.destroyCallStack(now, call_stack);
    }

    self.call_stacks.deinit(self.owner.allocator);
    self.owner.allocator.free(self.buffer);
    self.owner.allocator.destroy(self);
}

fn deinitUnbound(self: *Self) void {
    if (!self.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    if (self.mutex.lock_count != 1) @panic(@errorName(error.CallStackBound));
    self.deinit();
}

pub fn bind(self: *Self) void {
    if (!self.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    errdefer self.mutex.unlock();
    if (self.mutex.lock_count != 1) @panic(@errorName(error.CallStackBound));
    if (self.state.blocked) @panic(@errorName(error.CallStackBlocked));
    if (!self.state.suspended) @panic(@errorName(error.CallStackNotSuspended));
}

fn unbind(self: *Self) void {
    std.debug.assert(self.mutex.lock_count == 1);
    self.mutex.unlock();
}

fn unblock(self: *Self) void {
    if (!self.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    defer self.mutex.unlock();
    if (self.mutex.lock_count != 1) @panic(@errorName(error.CallStackBound));
    if (!self.state.blocked) @panic(@errorName(error.CallStackNotBlocked));
    std.debug.assert(self.state.suspended);

    const now = time.Time.now();
    for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
        subscriber.unblockCallStack(now, call_stack);
    }

    self.state.blocked = false;
}

pub fn @"suspend"(self: *Self, mark_blocked: bool) void {
    if (!self.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    defer self.mutex.unlock();
    if (self.mutex.lock_count == 1) @panic(@errorName(error.CallStackNotBound));
    if (self.state.suspended) @panic(@errorName(error.CallStackSuspended));

    const now = time.Time.now();
    for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
        subscriber.suspendCallStack(now, call_stack, mark_blocked);
    }

    self.state.suspended = true;
    self.state.blocked = mark_blocked;
}

pub fn @"resume"(self: *Self) void {
    if (!self.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    defer self.mutex.unlock();
    if (self.mutex.lock_count == 1) @panic(@errorName(error.CallStackNotBound));
    if (self.state.blocked) @panic(@errorName(error.CallStackBlocked));
    if (!self.state.suspended) @panic(@errorName(error.CallStackNotSuspended));

    const now = time.Time.now();
    for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
        subscriber.resumeCallStack(now, call_stack);
    }

    self.state.suspended = false;
}

pub fn pushSpan(
    self: *Self,
    desc: *const ProxyTracing.SpanDesc,
    formatter: *const ProxyTracing.Formatter,
    data: ?*const anyopaque,
    err: *?AnyError,
) (TracingError || AnyError.Error)!ProxyTracing.Span {
    if (!self.mutex.tryLock()) return error.CallStackInUse;
    defer self.mutex.unlock();
    if (self.mutex.lock_count == 1) return error.CallStackNotBound;
    if (self.state.blocked) return error.CallStackBlocked;
    if (self.state.suspended) return error.CallStackSuspended;

    const frame = try StackFrame.init(
        self,
        desc,
        formatter,
        data,
        err,
    );
    return frame.asProxySpan();
}

pub fn emitEvent(
    self: *Self,
    event: *const ProxyTracing.Event,
    formatter: *const ProxyTracing.Formatter,
    data: ?*const anyopaque,
    err: *?AnyError,
) (TracingError || AnyError.Error)!void {
    if (!self.mutex.tryLock()) return error.CallStackInUse;
    defer self.mutex.unlock();
    if (self.mutex.lock_count == 1) return error.CallStackNotBound;
    if (self.state.blocked) return error.CallStackBlocked;
    if (self.state.suspended) return error.CallStackSuspended;

    if (@intFromEnum(event.metadata.level) > @intFromEnum(self.max_level)) {
        return;
    }

    var written_characters: usize = undefined;
    const format_buffer = self.buffer[self.cursor..];
    const result = formatter(
        format_buffer.ptr,
        format_buffer.len,
        data,
        &written_characters,
    );
    try AnyError.initChecked(err, result);
    const message = format_buffer[0..written_characters];

    const now = time.Time.now();
    for (self.call_stacks.items, self.owner.subscribers) |call_stack, subscriber| {
        subscriber.emitEvent(now, call_stack, event, message);
    }
}

pub fn asProxy(self: *Self) ProxyTracing.CallStack {
    return .{
        .handle = self,
        .vtable = &vtable,
    };
}

// ----------------------------------------------------
// Dummy Call Stack
// ----------------------------------------------------

const DummyVTableImpl = struct {
    fn deinit(handle: *anyopaque) callconv(.c) void {
        std.debug.assert(handle == dummy_call_stack.handle);
    }
    fn replaceActive(handle: *anyopaque) callconv(.c) ProxyTracing.CallStack {
        std.debug.assert(handle == dummy_call_stack.handle);
        return dummy_call_stack;
    }
    fn unblock(handle: *anyopaque) callconv(.c) void {
        std.debug.assert(handle == dummy_call_stack.handle);
    }
};

pub const dummy_call_stack = ProxyTracing.CallStack{
    .handle = @ptrFromInt(1),
    .vtable = &dummy_vtable,
};

const dummy_vtable = ProxyTracing.CallStack.VTable{
    .deinit = &DummyVTableImpl.deinit,
    .replace_active = &DummyVTableImpl.replaceActive,
    .unblock = &DummyVTableImpl.unblock,
};

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn deinit(handle: *anyopaque) callconv(.c) void {
        const self: *Self = @alignCast(@ptrCast(handle));
        self.deinitUnbound();
    }
    fn replaceActive(handle: *anyopaque) callconv(.c) ProxyTracing.CallStack {
        const self: *Self = @alignCast(@ptrCast(handle));
        const tracing = self.owner;
        if (!tracing.isEnabledForCurrentThread()) @panic(@errorName(error.ThreadNotRegistered));

        const data = tracing.thread_data.get().?;
        if (data.call_stack == self) @panic(@errorName(error.CallStackBound));

        self.bind();
        data.call_stack.unbind();

        const old = data.call_stack;
        data.call_stack = self;
        return old.asProxy();
    }
    fn unblock(handle: *anyopaque) callconv(.c) void {
        const self: *Self = @alignCast(@ptrCast(handle));
        self.unblock();
    }
};

const vtable = ProxyTracing.CallStack.VTable{
    .deinit = &VTableImpl.deinit,
    .replace_active = &VTableImpl.replaceActive,
    .unblock = &VTableImpl.unblock,
};
