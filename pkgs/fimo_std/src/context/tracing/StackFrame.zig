const std = @import("std");

const time = @import("../../time.zig");
const ProxyTracing = @import("../proxy_context/tracing.zig");
const Tracing = @import("../tracing.zig");
const TracingError = Tracing.TracingError;
const CallStack = @import("CallStack.zig");

const Self = @This();

metadata: *const ProxyTracing.Metadata,
parent_cursor: usize,
parent_max_level: ProxyTracing.Level,
next: ?*Self = null,
previous: ?*Self,
owner: *CallStack,

pub fn init(
    owner: *CallStack,
    desc: *const ProxyTracing.SpanDesc,
    formatter: *const ProxyTracing.Formatter,
    data: ?*const anyopaque,
) *Self {
    const rest_buffer = owner.buffer[owner.cursor..];

    var written_characters: usize = undefined;
    formatter(
        rest_buffer.ptr,
        rest_buffer.len,
        data,
        &written_characters,
    );
    const message = rest_buffer[0..written_characters];
    var num_created_spans: usize = 0;

    const now = time.Time.now();
    for (owner.call_stacks.items, owner.owner.subscribers) |call_stack, subscriber| {
        subscriber.createSpan(
            now,
            desc,
            message,
            call_stack,
        );
        num_created_spans += 1;
    }

    const frame = owner.owner.allocator.create(Self) catch |e| @panic(@errorName(e));
    frame.* = .{
        .metadata = desc.metadata,
        .parent_cursor = owner.cursor,
        .parent_max_level = owner.max_level,
        .previous = owner.end_frame,
        .owner = owner,
    };

    owner.cursor += written_characters;
    owner.max_level = @enumFromInt(@min(@intFromEnum(desc.metadata.level), @intFromEnum(owner.max_level)));

    if (owner.end_frame) |end_frame| {
        end_frame.next = frame;
        owner.end_frame = frame;
    } else {
        owner.start_frame = frame;
        owner.end_frame = frame;
    }

    return frame;
}

pub fn deinit(self: *Self) void {
    const stack = self.owner;
    if (!stack.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    defer stack.mutex.unlock();
    if (stack.mutex.lock_count == 1) @panic(@errorName(error.CallStackNotBound));
    if (stack.state.blocked) @panic(@errorName(error.CallStackBlocked));
    if (stack.state.suspended) @panic(@errorName(error.CallStackSuspended));
    if (stack.end_frame != self) @panic(@errorName(error.CallStackSpanNotOnTop));
    std.debug.assert(self.next == null);

    const now = time.Time.now();
    for (self.owner.call_stacks.items, self.owner.owner.subscribers) |call_stack, subscriber| {
        subscriber.destroySpan(now, call_stack, false);
    }

    self.owner.cursor = self.parent_cursor;
    self.owner.max_level = self.parent_max_level;
    if (self.previous) |previous| {
        previous.next = null;
        self.owner.end_frame = previous;
    } else {
        self.owner.start_frame = null;
        self.owner.end_frame = null;
    }

    self.owner.owner.allocator.destroy(self);
}

pub fn deinitAbort(self: *Self) void {
    const stack = self.owner;
    if (!stack.mutex.tryLock()) @panic(@errorName(error.CallStackInUse));
    defer stack.mutex.unlock();
    if (stack.mutex.lock_count == 1) @panic(@errorName(error.CallStackNotBound));
    if (stack.state.blocked) @panic(@errorName(error.CallStackBlocked));
    if (stack.state.suspended) @panic(@errorName(error.CallStackSuspended));
    if (stack.end_frame != self) @panic(@errorName(error.CallStackSpanNotOnTop));
    std.debug.assert(self.next == null);

    self.deinitAbortUnchecked();
}

pub fn deinitAbortUnchecked(self: *Self) void {
    const now = time.Time.now();
    for (self.owner.call_stacks.items, self.owner.owner.subscribers) |call_stack, subscriber| {
        subscriber.destroySpan(now, call_stack, true);
    }

    self.owner.cursor = self.parent_cursor;
    self.owner.max_level = self.parent_max_level;
    if (self.previous) |previous| {
        previous.next = null;
        self.owner.end_frame = previous;
    } else {
        self.owner.start_frame = null;
        self.owner.end_frame = null;
    }

    self.owner.owner.allocator.destroy(self);
}

pub fn asProxySpan(self: *Self) ProxyTracing.Span {
    return .{
        .handle = self,
        .vtable = &vtable,
    };
}

// ----------------------------------------------------
// Dummy Span
// ----------------------------------------------------

const DummyVTableImpl = struct {
    fn deinit(handle: *anyopaque) callconv(.c) void {
        std.debug.assert(handle == dummy_span.handle);
    }
    fn deinitAbort(handle: *anyopaque) callconv(.c) void {
        std.debug.assert(handle == dummy_span.handle);
    }
};

pub const dummy_span = ProxyTracing.Span{
    .handle = @ptrFromInt(1),
    .vtable = &dummy_vtable,
};

const dummy_vtable = ProxyTracing.Span.VTable{
    .deinit = &DummyVTableImpl.deinit,
    .deinit_abort = &DummyVTableImpl.deinitAbort,
};

// ----------------------------------------------------
// VTable
// ----------------------------------------------------

const VTableImpl = struct {
    fn deinit(handle: *anyopaque) callconv(.c) void {
        const self: *Self = @alignCast(@ptrCast(handle));
        self.deinit();
    }
    fn deinitAbort(handle: *anyopaque) callconv(.c) void {
        const self: *Self = @alignCast(@ptrCast(handle));
        self.deinitAbort();
    }
};

const vtable = ProxyTracing.Span.VTable{
    .deinit = &VTableImpl.deinit,
    .deinit_abort = &VTableImpl.deinitAbort,
};
