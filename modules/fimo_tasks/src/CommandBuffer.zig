const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const Tracing = fimo_std.Context.Tracing;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_command_buffer = fimo_tasks_meta.command_buffer;
const Entry = meta_command_buffer.Entry;
const MetaCommandBuffer = meta_command_buffer.OpaqueCommandBuffer;
const Handle = meta_command_buffer.Handle;
const meta_task = fimo_tasks_meta.task;
const MetaTask = meta_task.OpaqueTask;
const meta_pool = fimo_tasks_meta.pool;
const StackSize = meta_pool.StackSize;
const MetaWorker = meta_pool.Worker;
const MetaPool = meta_pool.Pool;

const Futex = @import("Futex.zig");
const Pool = @import("Pool.zig");
const Task = @import("Task.zig");

owner: *Pool,
entry_index: usize = 0,
completed_index: usize = 0,
buffer: *MetaCommandBuffer,
next: ?*Self = null,
waiters_head: ?*Self = null,
waiters_tail: ?*Self = null,
enqueue_status: EnqueueStatus = .dequeued,
entry_status: []EntryStatus,
has_error: bool = false,
worker: ?MetaWorker = null,
stack_size: StackSize = .default,
abort_on_error: bool = false,
processing_entries_count: usize = 0,
state: atomic.Value(u8) = .init(running),
ref_count: atomic.Value(usize) = .init(1),

const Self = @This();

const running = 0b000;
const completed = 0b001;
const aborted = 0b010;
const status_mask = 0b11;
const waiting_bit = 0b100;

const EnqueueStatus = enum {
    dequeued,
    blocked,
    will_process,
};

const EntryStatus = union(enum) {
    not_processed,
    running_task: Task,
    running_buffer: *Self,
    processed,
};

pub fn init(owner: *Pool, buffer: *MetaCommandBuffer) Allocator.Error!*Self {
    _ = owner.ref();
    errdefer owner.unref();

    const allocator = owner.allocator;
    const entry_status = try allocator.alloc(EntryStatus, buffer.entries_len);
    errdefer allocator.free(entry_status);
    for (entry_status) |*s| s.* = .not_processed;

    const self = try allocator.create(Self);
    self.* = .{
        .owner = owner,
        .buffer = buffer,
        .entry_status = entry_status,
    };

    owner.runtime.logDebug(
        "created `{*}`, label=`{s}`",
        .{ self, self.buffer.label() },
        @src(),
    );
    return self;
}

pub fn abortDeinit(self: *Self) void {
    std.debug.assert(self.entry_index == 0);
    std.debug.assert(self.completed_index == 0);
    std.debug.assert(self.next == null);
    std.debug.assert(self.waiters_head == null);
    std.debug.assert(self.waiters_tail == null);
    std.debug.assert(self.enqueue_status == .dequeued);
    std.debug.assert(self.state.load(.acquire) == running);
    std.debug.assert(self.ref_count.load(.acquire) == 1);
    self.owner.runtime.logDebug("destroying {*}", .{self}, @src());

    for (self.buffer.entries()) |entry| entry.abort();
    self.buffer.abort();
    self.buffer.deinit();

    const pool = self.owner;
    const allocator = pool.allocator;
    allocator.free(self.entry_status);
    allocator.destroy(self);
    pool.unref();
}

pub fn ref(self: *Self) *Self {
    _ = self.ref_count.fetchAdd(1, .monotonic);
    return self;
}

pub fn unref(self: *Self) void {
    if (self.ref_count.fetchSub(1, .release) != 1) return;
    _ = self.ref_count.load(.acquire);
    self.owner.runtime.logDebug("destroying {*}", .{self}, @src());

    for (self.entry_status) |entry| std.debug.assert(entry == .processed);
    const state = self.state.load(.acquire);
    std.debug.assert(state != running);
    std.debug.assert(state & waiting_bit == 0);
    self.buffer.deinit();

    const pool = self.owner;
    const allocator = pool.allocator;
    allocator.free(self.entry_status);
    allocator.destroy(self);
    pool.unref();
}

pub fn completeTask(self: *Self, task: *Task, is_error: bool) void {
    std.debug.assert(task.owner == self);
    std.debug.assert(task.entry_index < self.entry_index);
    std.debug.assert(task.entry_index >= self.completed_index);
    switch (self.entry_status[task.entry_index]) {
        .not_processed, .processed, .running_buffer => unreachable,
        // If the command is a running task, we need to do nothing, as the
        // task will notify us when it is done
        .running_task => |*t| {
            std.debug.assert(task == t);
            self.owner.runtime.logDebug(
                "completed `{*}` of `{*}`",
                .{ task, self },
                @src(),
            );
            const stack_allocator = self.owner.getStackAllocator(task.stack_size).?;
            stack_allocator.deallocate(task.stack);

            self.entry_status[task.entry_index] = .processed;
            if (is_error) self.abortFromIndex(task.entry_index);
            self.progressCompleted();
            self.enqueueToPool();
            _ = self.owner.task_count.fetchSub(1, .monotonic);
        },
    }
}

pub fn processEntry(self: *Self) void {
    self.owner.runtime.logDebug("processing `{*}`", .{self}, @src());
    std.debug.assert(self.enqueue_status == .will_process);
    self.enqueue_status = .dequeued;

    const entries = self.buffer.entries();
    while (self.entry_index < entries.len) {
        const entry = entries[self.entry_index];
        switch (entry.tag) {
            .abort_on_error => self.processAbortOnError(
                entry,
            ) catch return,
            .set_min_stack_size => self.processMinStackSize(
                entry,
            ) catch return,
            .select_worker => self.processSelectWorker(
                entry,
            ) catch return,
            .select_any_worker => self.processSelectAnyWorker(
                entry,
            ) catch return,
            .enqueue_task => self.processEnqueueTask(
                entry,
            ) catch return,
            .enqueue_command_buffer => self.processEnqueueCommandBuffer(
                entry,
            ) catch return,
            .wait_on_barrier => self.processWaitOnBarrier(
                entry,
            ) catch return,
            .wait_on_command_buffer => self.processWaitOnCommandBuffer(
                entry,
            ) catch return,
            .wait_on_command_indirect => self.processWaitOnCommandIndirect(
                entry,
            ) catch return,
            else => self.processUnknown(entry),
        }
    }

    // Wait for all command buffers to complete.
    std.debug.assert(self.completed_index <= self.entry_index);
    while (self.completed_index != self.entry_index) {
        self.waitOnEntry(self.completed_index) catch return;
    }

    // Now that all tasks have completed, we mark the command buffer as completed or aborted.
    // Additionally, others may be waiting on this command buffer to complete, so we notify them.
    if (self.has_error) self.buffer.abort() else self.buffer.complete();
    self.broadcast();
    self.unref();
}

const ProcessError = error{Block};

fn processAbortOnError(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .abort_on_error);
    self.abort_on_error = entry.payload.abort_on_error;
    self.owner.runtime.logDebug(
        "`{*}` setting `abort on error` to `{}`, index=`{}`",
        .{ self, self.abort_on_error, self.entry_index },
        @src(),
    );
    self.markStatus(.processed);
}

fn processMinStackSize(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .set_min_stack_size);
    const min_stack_size = entry.payload.set_min_stack_size;
    self.owner.runtime.logDebug(
        "`{*}` setting `min stack size` to `{}`, index=`{}`",
        .{ self, min_stack_size, self.entry_index },
        @src(),
    );
    if (self.owner.getStackAllocator(min_stack_size) == null) {
        self.owner.runtime.logErr(
            "`{*}` validation failed:" ++
                " invalid stack size, entry=`{s}`, index=`{}`, stack_size=`{}`",
            .{
                self,
                @tagName(entry.tag),
                self.entry_index,
                @intFromEnum(min_stack_size),
            },
            @src(),
        );
        self.abortCurrentAndForward();
        return;
    }
    self.stack_size = min_stack_size;
    self.markStatus(.processed);
}

fn processSelectWorker(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .select_worker);
    const worker = entry.payload.select_worker;
    self.owner.runtime.logDebug(
        "`{*}` setting `worker` to `{}`, index=`{}`",
        .{ self, worker, self.entry_index },
        @src(),
    );
    if (self.owner.workers.len <= @intFromEnum(worker)) {
        self.owner.runtime.logErr(
            "`{*}` validation failed:" ++
                " invalid worker, entry=`{s}`, index=`{}`, worker=`{}`",
            .{
                self,
                @tagName(entry.tag),
                self.entry_index,
                worker,
            },
            @src(),
        );
        self.abortCurrentAndForward();
        return;
    }
    self.worker = worker;
    self.markStatus(.processed);
}

fn processSelectAnyWorker(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .select_any_worker);
    self.worker = null;
    self.owner.runtime.logDebug(
        "`{*}` setting `worker` to `any`, index=`{}`",
        .{ self, self.entry_index },
        @src(),
    );
    self.markStatus(.processed);
}

fn processEnqueueTask(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .enqueue_task);
    const stack_allocator = self.owner.getStackAllocator(self.stack_size).?;
    const stack = stack_allocator.allocate() catch |err| switch (err) {
        error.Block => {
            stack_allocator.wait(self);
            return ProcessError.Block;
        },
        Allocator.Error.OutOfMemory => {
            self.owner.runtime.logErr(
                "`{*}` validation failed:" ++
                    " stack allocation failed, entry=`{s}`, index=`{}`, stack=`{}`",
                .{
                    self,
                    @tagName(entry.tag),
                    self.entry_index,
                    @intFromEnum(self.stack_size),
                },
                @src(),
            );
            self.abortCurrentAndForward();
            return;
        },
    };

    const task_status = &self.entry_status[self.entry_index];
    const task = Task{
        .allocator = self.owner.allocator,
        .id = @enumFromInt(@intFromPtr(task_status)),
        .task = entry.payload.enqueue_task,
        .owner = self,
        .entry_index = self.entry_index,
        .stack = stack,
        .stack_size = self.stack_size,
    };
    self.markStatus(.{ .running_task = task });
    const task_ptr = &task_status.running_task;

    self.owner.runtime.logDebug(
        "`{*}` spawning `{*}`, index=`{}`",
        .{ self, task_ptr, task_ptr.entry_index },
        @src(),
    );
    _ = self.owner.task_count.fetchAdd(1, .monotonic);
    self.owner.enqueueTask(task_ptr, self.worker);
}

fn processEnqueueCommandBuffer(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .enqueue_command_buffer);
    const buffer = Self.init(self.owner, entry.payload.enqueue_command_buffer) catch {
        self.owner.runtime.logErr(
            "`{*}` validation failed:" ++
                " enqueue of command buffer failed, entry=`{s}`, index=`{}`, enqueue-buffer=`{s}`",
            .{
                self,
                @tagName(entry.tag),
                self.entry_index,
                entry.payload.enqueue_command_buffer.label(),
            },
            @src(),
        );
        self.abortCurrentAndForward();
        return;
    };
    self.owner.runtime.logDebug(
        "`{*}` spawning `{*}` to `{*}`, index=`{}`",
        .{ self, buffer, self.owner, self.entry_index },
        @src(),
    );
    self.owner.command_buffer_count += 1;
    buffer.enqueueToPool();
    self.markStatus(.{ .running_buffer = buffer.ref() });
}

fn processWaitOnBarrier(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .wait_on_barrier);
    self.owner.runtime.logDebug(
        "`{*}` waiting on barrier, index=`{}`",
        .{ self, self.entry_index },
        @src(),
    );

    // Wait for the completion of all previous commands.
    while (self.completed_index != self.entry_index) {
        try self.waitOnEntry(self.completed_index);
    }
    return self.markStatus(.processed);
}

fn processWaitOnCommandBuffer(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .wait_on_command_buffer);
    self.owner.runtime.logDebug(
        "`{*}` waiting on child command buffer, index=`{}`",
        .{ self, self.entry_index },
        @src(),
    );

    const handle = entry.payload.wait_on_command_buffer;
    if (handle.vtable != &vtable) {
        self.owner.runtime.logErr(
            "`{*}` validation failed:" ++
                " invalid handle, entry=`{s}`, index=`{}`",
            .{
                self,
                @tagName(entry.tag),
                self.entry_index,
            },
            @src(),
        );
        self.abortCurrentAndForward();
        return;
    }
    const buffer: *Self = @ptrCast(@alignCast(handle.data));
    if (buffer.owner != self.owner) {
        self.owner.runtime.logErr(
            "`{*}` validation failed:" ++
                " handle owner mismatch, entry=`{s}`, index=`{}`," ++
                " expected=`{s}`, got=`{s}`",
            .{
                self,
                @tagName(entry.tag),
                self.entry_index,
                self.owner.label,
                buffer.owner.label,
            },
            @src(),
        );
        self.abortCurrentAndForward();
        return;
    }

    // If we are waiting on another command buffer, we have to register
    // ourselves as a waiter on that buffer
    const status = try self.waitOnBuffer(buffer);
    self.markStatus(.processed);
    if (status == .aborted) self.abortForward();
}

fn processWaitOnCommandIndirect(self: *Self, entry: Entry) ProcessError!void {
    std.debug.assert(entry.tag == .wait_on_command_indirect);
    self.owner.runtime.logDebug(
        "`{*}` waiting on command, index=`{}`",
        .{ self, self.entry_index },
        @src(),
    );

    const offset = entry.payload.wait_on_command_indirect;
    if (offset == 0) return self.markStatus(.processed);
    if (offset > self.entry_index) {
        self.owner.runtime.logErr(
            "`{*}` validation failed:" ++
                " offset out of bounds, entry=`{s}`, index=`{}`, offset=`{}`",
            .{
                self,
                @tagName(entry.tag),
                self.entry_index,
                offset,
            },
            @src(),
        );
        self.abortCurrentAndForward();
        return;
    }

    // Wait on the completion of the entry.
    try self.waitOnEntry(self.entry_index - offset);
    self.markStatus(.processed);
}

fn processUnknown(self: *Self, entry: Entry) void {
    self.owner.runtime.logErr(
        "`{*}` validation failed:" ++
            " unknown entry type, entry=`{}`, index=`{}`",
        .{
            self,
            @intFromEnum(entry.tag),
            self.entry_index,
        },
        @src(),
    );
    self.abortCurrentAndForward();
}

pub fn enqueueToPool(self: *Self) void {
    if (self.enqueue_status != .dequeued) return;

    self.owner.runtime.logDebug(
        "`{*}` enqueueing to `{*}`",
        .{ self, self.owner },
        @src(),
    );
    std.debug.assert(self.next == null);
    if (self.owner.process_list_tail) |tail| {
        tail.next = self;
    } else {
        self.owner.process_list_head = self;
    }
    self.owner.process_list_tail = self;
    self.enqueue_status = .will_process;
}

fn markStatus(self: *Self, status: EntryStatus) void {
    std.debug.assert(status != .not_processed);
    std.debug.assert(self.entry_index < self.entry_status.len);
    self.entry_status[self.entry_index] = status;
    self.entry_index += 1;
    if (status == .processed) self.progressCompleted();
}

fn progressCompleted(self: *Self) void {
    std.debug.assert(self.completed_index < self.entry_index);
    while (self.completed_index < self.entry_index) : (self.completed_index += 1) {
        switch (self.entry_status[self.completed_index]) {
            // All entries up to `entry_index` have been processed.
            .not_processed => unreachable,
            // There are still running entries so we stop.
            .running_task, .running_buffer => break,
            // Continue to the next entry.
            .processed => {},
        }
    }
}

fn abortCurrentAndForward(self: *Self) void {
    std.debug.assert(!self.has_error);
    std.debug.assert(self.entry_index < self.entry_status.len);
    std.debug.assert(self.entry_status[self.entry_index] == .not_processed);
    self.buffer.entries()[self.entry_index].abort();
    self.markStatus(.processed);
    self.abortForward();
}

fn abortFromIndex(self: *Self, index: usize) void {
    std.debug.assert(index < self.entry_status.len);
    std.debug.assert(self.entry_status[index] == .processed);

    // If the current setting is set to not abort it is impossible that
    // next entries will be aborted, as either the setting has changed between
    // the index and the current index, or the setting was false at the time of the index.
    if (!self.abort_on_error) return;

    // If we processed all entries there is nothing to abort.
    if (self.entry_index == self.entry_status.len) {
        self.has_error = true;
        return;
    }

    // Check if the abort setting changed between the index and the current index.
    const entries = self.buffer.entries();
    for (entries[index..self.entry_index]) |entry| {
        if (entry.tag == .abort_on_error and !entry.payload.abort_on_error) return;
    }

    // Now that we know that the setting did not change, we can abort all remaining entries.
    self.abortForward();
}

fn abortForward(self: *Self) void {
    if (!self.abort_on_error) return;
    const entries = self.buffer.entries();
    while (self.entry_index < self.entry_status.len) {
        const entry = entries[self.entry_index];
        switch (entry.tag) {
            .abort_on_error => {
                self.abort_on_error = entry.payload.abort_on_error;
                self.markStatus(.processed);
                if (self.abort_on_error == false) break;
            },
            else => {
                entry.abort();
                self.markStatus(.processed);
            },
        }
    }

    self.has_error = self.abort_on_error;
    std.debug.assert(!self.has_error or self.entry_index == self.entry_status.len);
}

fn broadcast(self: *Self) void {
    std.debug.assert(self.completed_index == self.entry_index);
    std.debug.assert(self.entry_index == self.entry_status.len);
    self.owner.runtime.logDebug(
        "`{*}` waking waiters",
        .{self},
        @src(),
    );

    // Mark that the command buffer has been completed or aborted.
    const new_state: u8 = if (self.has_error) aborted else completed;
    const state = self.state.swap(new_state, .release);
    std.debug.assert(state & status_mask == running);

    // If there are waiting tasks, we wake them.
    if (state & waiting_bit != 0) {
        const futex = &self.owner.runtime.futex;
        _ = futex.wake(&self.state, std.math.maxInt(usize));
    }

    // If there are waiting command buffers, we enqueue them to the pool.
    if (self.waiters_head) |head| {
        var current: ?*Self = head;
        while (current) |waiter| {
            std.debug.assert(self.enqueue_status == .blocked);
            self.owner.runtime.logDebug("waking `{*}`", .{waiter}, @src());
            waiter.enqueue_status = .will_process;
            current = waiter.next;
        }

        if (self.owner.process_list_tail) |tail| {
            tail.next = head;
        } else {
            self.owner.process_list_head = head;
        }
        self.owner.process_list_tail = self.waiters_tail;

        self.waiters_head = null;
        self.waiters_tail = null;
    }

    self.owner.command_buffer_count -= 1;
}

fn waitOnEntry(self: *Self, entry_index: usize) ProcessError!void {
    std.debug.assert(entry_index < self.entry_index);
    std.debug.assert(entry_index < self.entry_status.len);

    switch (self.entry_status[entry_index]) {
        // The command was in the past, so it has already been processed
        .not_processed => unreachable,
        // If the command is a running task, we need to do nothing, as the
        // task will notify us when it is done
        .running_task => |*task| {
            self.owner.runtime.logDebug("`{*}` blocking on `{*}`", .{ self, task }, @src());
            return error.Block;
        },
        // If we are waiting on another command buffer, we have to register
        // ourselves as a waiter on that buffer.
        .running_buffer => |buffer| {
            const status = try self.waitOnBuffer(buffer);
            buffer.unref();
            self.entry_status[entry_index] = .processed;
            if (status == .aborted) self.abortFromIndex(entry_index);
            self.progressCompleted();
        },
        .processed => self.markStatus(.processed),
    }
}

fn waitOnBuffer(self: *Self, buffer: *Self) ProcessError!Handle.CompletionStatus {
    std.debug.assert(self != buffer);
    std.debug.assert(self.owner == buffer.owner);
    std.debug.assert(self.next == null);
    std.debug.assert(self.enqueue_status == .dequeued);

    const state = self.state.load(.acquire);
    if (state & status_mask != running)
        return if (state & status_mask == completed) .completed else .aborted;

    self.owner.runtime.logDebug("`{*}` blocking on `{*}`", .{ self, buffer }, @src());
    if (buffer.waiters_tail) |tail| {
        tail.next = self;
    } else {
        buffer.waiters_head = self;
    }
    buffer.waiters_tail = self;

    self.enqueue_status = .blocked;
    return ProcessError.Block;
}

fn waitOn(self: *Self) Handle.CompletionStatus {
    self.owner.runtime.logDebug(
        "waiting on `{*}`",
        .{self},
        @src(),
    );

    const futex = &self.owner.runtime.futex;
    var state = self.state.load(.monotonic);
    while (state & status_mask == running) {
        // Flag that we are waiting on this command buffer.
        if (state & waiting_bit == 0) {
            if (self.state.cmpxchgWeak(state, state | waiting_bit, .monotonic, .monotonic)) |x| {
                state = x;
                continue;
            }
        }

        // Wait for the futex to change state, assuming it is still `running | waiting_bit`.
        futex.wait(&self.state, @sizeOf(u8), running | waiting_bit, 0, null) catch |err| switch (err) {
            error.Invalid => {},
            error.Timeout => unreachable,
        };
        state = self.state.load(.monotonic);
    }

    state = self.state.load(.acquire);
    std.debug.assert(state == completed or state == aborted);
    return if (state == completed)
        .completed
    else
        .aborted;
}

pub fn asHandle(self: *Self) Handle {
    return .{ .data = self, .vtable = &vtable };
}

const VTableImpl = struct {
    fn ref(handle: *anyopaque) callconv(.c) void {
        const this: *Self = @ptrCast(@alignCast(handle));
        _ = this.ref();
    }
    fn unref(handle: *anyopaque) callconv(.c) void {
        const this: *Self = @ptrCast(@alignCast(handle));
        this.unref();
    }
    fn owner_pool(handle: *anyopaque) callconv(.c) MetaPool {
        const this: *Self = @ptrCast(@alignCast(handle));
        return this.owner.ref().asMetaPool();
    }
    fn wait_on(handle: *anyopaque) callconv(.c) Handle.CompletionStatus {
        const this: *Self = @ptrCast(@alignCast(handle));
        return this.waitOn();
    }
};
const vtable = Handle.VTable{
    .ref = &VTableImpl.ref,
    .unref = &VTableImpl.unref,
    .owner_pool = &VTableImpl.owner_pool,
    .wait_on = &VTableImpl.wait_on,
};
