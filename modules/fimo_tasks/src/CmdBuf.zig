const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const tracing = fimo_std.tracing;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const PWorker = fimo_tasks_meta.Worker;
const PTask = fimo_tasks_meta.Task;
const PCmdBufCmd = fimo_tasks_meta.CmdBufCmd;
const PCmdBuf = fimo_tasks_meta.CmdBuf;
const PCmdBufHandle = fimo_tasks_meta.CmdBufHandle;

const FimoTasks = @import("FimoTasks.zig");
const Futex = @import("Futex.zig");
const Pool = @import("Pool.zig");
const Task = @import("Task.zig");

owner: *Pool,
entry_index: usize = 0,
completed_index: usize = 0,
buffer: *PCmdBuf,
next: ?*Self = null,
waiters_head: ?*Self = null,
waiters_tail: ?*Self = null,
enqueue_status: EnqueueStatus = .dequeued,
entry_status: []EntryStatus,
has_error: bool = false,
worker: ?PWorker = null,
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
    processed,
};

pub fn init(owner: *Pool, buffer: *PCmdBuf) Allocator.Error!*Self {
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

    tracing.logDebug(@src(), "created `{*}`, label=`{s}`", .{ self, self.buffer.label() });
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
    tracing.logDebug(@src(), "destroying {*}", .{self});

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
    tracing.logDebug(@src(), "destroying {*}", .{self});

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
            tracing.logDebug(@src(), "completed `{*}` of `{*}`", .{ task, self });
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
    tracing.logDebug(@src(), "processing `{*}`", .{self});
    std.debug.assert(self.enqueue_status == .will_process);
    self.enqueue_status = .dequeued;

    const entries = self.buffer.entries();
    while (self.entry_index < entries.len) {
        const entry = entries[self.entry_index];
        switch (entry.tag) {
            .select_worker => self.processSelectWorker(
                entry,
            ) catch return,
            .select_any_worker => self.processSelectAnyWorker(
                entry,
            ) catch return,
            .enqueue_task => self.processEnqueueTask(
                entry,
            ) catch return,
            .wait_on_barrier => self.processWaitOnBarrier(
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

fn processSelectWorker(self: *Self, entry: PCmdBufCmd) ProcessError!void {
    std.debug.assert(entry.tag == .select_worker);
    const worker = entry.payload.select_worker;
    tracing.logDebug(
        @src(),
        "`{*}` setting `worker` to `{}`, index=`{}`",
        .{ self, worker, self.entry_index },
    );
    if (self.owner.workers.len <= @intFromEnum(worker)) {
        tracing.logErr(@src(), "`{*}` validation failed:" ++
            " invalid worker, entry=`{s}`, index=`{}`, worker=`{}`", .{
            self,
            @tagName(entry.tag),
            self.entry_index,
            worker,
        });
        self.abortCurrentAndForward();
        return;
    }
    self.worker = worker;
    self.markStatus(.processed);
}

fn processSelectAnyWorker(self: *Self, entry: PCmdBufCmd) ProcessError!void {
    std.debug.assert(entry.tag == .select_any_worker);
    self.worker = null;
    tracing.logDebug(@src(), "`{*}` setting `worker` to `any`, index=`{}`", .{
        self,
        self.entry_index,
    });
    self.markStatus(.processed);
}

fn processEnqueueTask(self: *Self, entry: PCmdBufCmd) ProcessError!void {
    std.debug.assert(entry.tag == .enqueue_task);
    const stack_allocator = self.owner.getStackAllocator(self.stack_size).?;
    const stack = stack_allocator.allocate() catch |err| switch (err) {
        error.Block => {
            stack_allocator.wait(self);
            return ProcessError.Block;
        },
        Allocator.Error.OutOfMemory => {
            tracing.logErr(@src(), "`{*}` validation failed:" ++
                " stack allocation failed, entry=`{s}`, index=`{}`, stack=`{}`", .{
                self,
                @tagName(entry.tag),
                self.entry_index,
                @intFromEnum(self.stack_size),
            });
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

    tracing.logDebug(@src(), "`{*}` spawning `{*}`, index=`{}`", .{
        self,
        task_ptr,
        task_ptr.entry_index,
    });
    _ = self.owner.task_count.fetchAdd(1, .monotonic);
    self.owner.enqueueTask(task_ptr, self.worker);
}

fn processWaitOnBarrier(self: *Self, entry: PCmdBufCmd) ProcessError!void {
    std.debug.assert(entry.tag == .wait_on_barrier);
    tracing.logDebug(@src(), "`{*}` waiting on barrier, index=`{}`", .{ self, self.entry_index });

    // Wait for the completion of all previous commands.
    while (self.completed_index != self.entry_index) {
        try self.waitOnEntry(self.completed_index);
    }
    return self.markStatus(.processed);
}

fn processWaitOnCommandIndirect(self: *Self, entry: PCmdBufCmd) ProcessError!void {
    std.debug.assert(entry.tag == .wait_on_command_indirect);
    tracing.logDebug(@src(), "`{*}` waiting on command, index=`{}`", .{ self, self.entry_index });

    const offset = entry.payload.wait_on_command_indirect;
    if (offset == 0) return self.markStatus(.processed);
    if (offset > self.entry_index) {
        tracing.logErr(@src(), "`{*}` validation failed:" ++
            " offset out of bounds, entry=`{s}`, index=`{}`, offset=`{}`", .{
            self,
            @tagName(entry.tag),
            self.entry_index,
            offset,
        });
        self.abortCurrentAndForward();
        return;
    }

    // Wait on the completion of the entry.
    try self.waitOnEntry(self.entry_index - offset);
    self.markStatus(.processed);
}

fn processUnknown(self: *Self, entry: PCmdBufCmd) void {
    tracing.logErr(@src(), "`{*}` validation failed:" ++
        " unknown entry type, entry=`{}`, index=`{}`", .{
        self,
        @intFromEnum(entry.tag),
        self.entry_index,
    });
    @panic("unknown cmd");
}

pub fn enqueueToPool(self: *Self) void {
    if (self.enqueue_status != .dequeued) return;

    tracing.logDebug(@src(), "`{*}` enqueueing to `{*}`", .{ self, self.owner });
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
    tracing.logDebug(@src(), "`{*}` waking waiters", .{self});

    // Mark that the command buffer has been completed or aborted.
    const new_state: u8 = if (self.has_error) aborted else completed;
    const state = self.state.swap(new_state, .release);
    std.debug.assert(state & status_mask == running);

    // If there are waiting tasks, we wake them.
    if (state & waiting_bit != 0) {
        const futex = &FimoTasks.get().futex;
        _ = futex.wake(&self.state, std.math.maxInt(usize));
    }

    // If there are waiting command buffers, we enqueue them to the pool.
    if (self.waiters_head) |head| {
        var current: ?*Self = head;
        while (current) |waiter| {
            std.debug.assert(self.enqueue_status == .blocked);
            tracing.logDebug(@src(), "waking `{*}`", .{waiter});
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
            tracing.logDebug(@src(), "`{*}` blocking on `{*}`", .{ self, task });
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

    tracing.logDebug(@src(), "`{*}` blocking on `{*}`", .{ self, buffer });
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
    tracing.logDebug(@src(), "waiting on `{*}`", .{self});

    const futex = &FimoTasks.get().futex;
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
