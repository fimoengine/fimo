const std = @import("std");
const atomic = std.atomic;
const Thread = std.Thread;
const Allocator = std.mem.Allocator;
const ArrayListUnmanaged = std.ArrayListUnmanaged;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;

const fimo_std = @import("fimo_std");
const Tracing = fimo_std.Context.Tracing;
const AnyError = fimo_std.AnyError;
const AnyResult = AnyError.AnyResult;
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_command_buffer = fimo_tasks_meta.command_buffer;
const MetaCommandBuffer = meta_command_buffer.OpaqueCommandBuffer;
const MetaCommandBufferHandle = meta_command_buffer.Handle;
const meta_pool = fimo_tasks_meta.pool;
const StackSize = meta_pool.StackSize;
const MetaId = meta_pool.Id;
const MetaWorker = meta_pool.Worker;
const MetaPool = meta_pool.Pool;
const MetaPoolVTable = meta_pool.VTable;

const channel = @import("channel.zig");
const SendError = channel.SendError;
const SignalMpscChannel = channel.SignalMpscChannel;
const IntrusiveMpscChannel = channel.IntrusiveMpscChannel;
const multi_receiver = channel.multi_receiver;
const UnorderedSpmcChannel = channel.UnorderedSpmcChannel;
const CommandBuffer = @import("CommandBuffer.zig");
const context = @import("context.zig");
const Stack = context.Stack;
const Runtime = @import("Runtime.zig");
const Task = @import("Task.zig");
const Worker = @import("Worker.zig");
const GlobalChannel = Worker.GlobalChannel;

// Private members.
// Only accessible via a strong reference from the pool event loop.
workers: []Worker,
global_channel: GlobalChannel,
command_buffer_count: usize = 0,
stack_allocators: []StackAllocator,
default_stack_allocator_idx: usize,
process_list_head: ?*CommandBuffer = null,
process_list_tail: ?*CommandBuffer = null,
timeouts_head: ?*PrivateMessage.Timeout = null,
waiters: AutoArrayHashMapUnmanaged(*const anyopaque, Bucket) = .empty,
private_message_queue: PrivateChannel = .empty,
enqueue_requests: IntrusiveMpscChannel(*CommandBuffer) = .empty,
signal_channel: SignalMpscChannel = .empty,

// Accessible via a weak reference.
label: []u8,
thread: Thread,
is_public: bool,
runtime: *Runtime,
allocator: Allocator,
should_join: atomic.Value(bool) = .init(false),
worker_count: atomic.Value(usize),
task_count: atomic.Value(usize) = .init(0),
ref_count: atomic.Value(usize) = .init(1),
weak_ref_count: atomic.Value(usize) = .init(1),

const Self = @This();

const Bucket = struct {
    head: ?*Task = null,
    tail: ?*Task = null,
};

pub const PrivateChannel = IntrusiveMpscChannel(*PrivateMessage);

pub const PrivateMessage = struct {
    next: ?*PrivateMessage = null,
    msg: union(enum) {
        complete: struct { is_error: bool, task: *Task },
        sleep: struct { timeout: Timeout },
        wait: struct {
            value: *const atomic.Value(u32),
            expect: u32,
            timed_out: *bool,
            task: *Task,
            timeout: ?Timeout,
        },
        wake: struct {
            value: *const atomic.Value(u32),
            max_waiters: usize,
        },
    },

    pub const Timeout = struct {
        task: *Task,
        timeout: Instant,
        next: ?*Timeout = null,
    };
};

pub const StackAllocator = struct {
    size: usize,
    allocated: usize = 0,
    max_allocated: usize = std.math.maxInt(usize),
    waiters_head: ?*CommandBuffer = null,
    waiters_tail: ?*CommandBuffer = null,
    cold_freelist: ArrayListUnmanaged(Stack) = .empty,
    hot_freelist: ArrayListUnmanaged(Stack) = .empty,
    deallocation_cache: ArrayListUnmanaged(Stack) = .empty,

    pub const Error = error{Block} || Allocator.Error;

    pub fn allocate(self: *StackAllocator) Error!Stack {
        if (self.deallocation_cache.pop()) |stack| return stack;
        if (self.hot_freelist.pop()) |stack| return stack;
        if (self.cold_freelist.pop()) |stack| return stack;

        if (self.allocated == self.max_allocated) return error.Block;
        const stack = try Stack.init(self.size);
        self.allocated += 1;
        return stack;
    }

    pub fn deallocate(self: *StackAllocator, stack: Stack) void {
        if (self.deallocation_cache.items.len < self.deallocation_cache.capacity) {
            self.deallocation_cache.appendAssumeCapacity(stack);
        } else {
            self.deallocateStackNoCache(stack);
        }
        self.signal();
    }

    pub fn wait(self: *StackAllocator, buffer: *CommandBuffer) void {
        std.debug.assert(buffer.next == null);
        std.debug.assert(buffer.enqueue_status == .dequeued);

        if (self.waiters_tail) |tail| {
            tail.next = buffer;
        } else {
            self.waiters_head = buffer;
        }
        self.waiters_tail = buffer;
        buffer.enqueue_status = .blocked;
    }

    fn signal(self: *StackAllocator) void {
        if (self.waiters_head) |waiter| {
            const next = waiter.next;
            waiter.next = null;
            waiter.enqueue_status = .dequeued;
            self.waiters_head = next;
            if (next == null) self.waiters_tail = null;
            waiter.enqueueToPool();
        }
    }

    fn clearCache(self: *StackAllocator) void {
        while (self.deallocation_cache.pop()) |stack| self.deallocateStackNoCache(stack);
    }

    fn deallocateStackNoCache(self: *StackAllocator, stack: Stack) void {
        if (self.hot_freelist.items.len < self.hot_freelist.capacity) {
            self.hot_freelist.appendAssumeCapacity(stack);
        } else if (self.cold_freelist.items.len < self.cold_freelist.capacity) {
            self.cold_freelist.appendAssumeCapacity(stack.transitionCold());
        } else {
            stack.deinit();
            self.allocated -= 1;
        }
    }
};

pub const InitOptions = struct {
    runtime: *Runtime,
    allocator: Allocator,
    label: []const u8 = "<unlabelled>",
    stacks: []const StackOptions,
    default_stack: usize,
    worker_count: usize,
    is_public: bool,

    pub const StackOptions = struct {
        size: usize,
        preallocated: usize,
        cold: usize,
        hot: usize,
        max_allocated: usize,
    };
};

pub fn init(options: InitOptions) !*Self {
    std.debug.assert(options.stacks.len != 0);
    std.debug.assert(options.default_stack < options.stacks.len);
    for (options.stacks) |stack| {
        std.debug.assert(stack.size != 0);
        std.debug.assert(stack.max_allocated != 0);
        std.debug.assert(stack.preallocated <= stack.max_allocated);
        std.debug.assert(stack.cold + stack.hot <= stack.max_allocated);
        std.debug.assert(stack.preallocated <= stack.cold + stack.hot);
    }
    for (options.stacks[0 .. options.stacks.len - 1], options.stacks[1..options.stacks.len]) |stack, next|
        std.debug.assert(stack.size > next.size);
    std.debug.assert(options.worker_count != 0);

    const runtime = options.runtime;
    if (runtime.getInstance()) |instance| instance.ref();
    errdefer if (runtime.getInstance()) |instance| instance.unref();

    const allocator = options.allocator;
    const self = try allocator.create(Self);
    errdefer allocator.destroy(self);

    self.* = .{
        .workers = undefined,
        .worker_count = .init(options.worker_count),
        .global_channel = undefined,
        .stack_allocators = undefined,
        .default_stack_allocator_idx = options.default_stack,
        .label = undefined,
        .thread = undefined,
        .is_public = options.is_public,
        .runtime = runtime,
        .allocator = allocator,
    };

    self.label = try allocator.dupe(u8, options.label);
    errdefer allocator.free(self.label);

    self.global_channel = try GlobalChannel.init(allocator);
    errdefer self.global_channel.deinit(allocator);

    var init_stack_allocators: usize = 0;
    self.stack_allocators = try allocator.alloc(StackAllocator, options.stacks.len);
    errdefer {
        for (self.stack_allocators[0..init_stack_allocators]) |*stack_allocator| {
            for (stack_allocator.cold_freelist.items) |stack| stack.deinit();
            for (stack_allocator.hot_freelist.items) |stack| stack.deinit();
            stack_allocator.cold_freelist.deinit(allocator);
            stack_allocator.hot_freelist.deinit(allocator);
            stack_allocator.deallocation_cache.deinit(allocator);
        }
        allocator.free(self.stack_allocators);
    }
    for (self.stack_allocators, options.stacks) |*al, stack| {
        al.* = .{
            .size = stack.size,
            .allocated = stack.preallocated,
            .max_allocated = stack.max_allocated,
        };
        al.cold_freelist = try ArrayListUnmanaged(Stack).initCapacity(allocator, stack.cold);
        al.hot_freelist = try ArrayListUnmanaged(Stack).initCapacity(allocator, stack.hot);
        al.deallocation_cache = try ArrayListUnmanaged(Stack).initCapacity(allocator, options.worker_count * 2);

        const preallocated_hot = @min(stack.hot, stack.preallocated);
        const preallocated_cold = @min(stack.cold, stack.preallocated - preallocated_hot);
        for (0..preallocated_cold) |_| {
            const s = try Stack.init(al.size);
            al.cold_freelist.appendAssumeCapacity(s);
        }
        for (0..preallocated_hot) |_| {
            const s = try Stack.init(al.size);
            al.hot_freelist.appendAssumeCapacity(s);
        }

        init_stack_allocators += 1;
    }

    // Spawn all workers.
    var running_workers: usize = 0;
    self.workers = try allocator.alloc(Worker, options.worker_count);
    errdefer {
        self.global_channel.close(&options.runtime.futex);
        for (self.workers[0..running_workers]) |*worker| {
            worker.private_queue.close(&options.runtime.futex);
            worker.thread.join();
        }
        allocator.free(self.workers);
    }
    for (self.workers, 0..self.workers.len) |*worker, i| {
        worker.* = .{
            .pool = self,
            .id = @enumFromInt(i),
            .thread = undefined,
            .pool_sx = self.private_message_queue.sender(),
            .global_rx = self.global_channel.receiver(),
            .worker_count = &self.worker_count,
            .global_task_count = &self.task_count,
        };
        worker.thread = try Thread.spawn(.{}, Worker.run, .{worker});
        worker.thread.setName("worker") catch {};
        running_workers += 1;
    }

    // Spawn the event loop thread now that all fields are initialized.
    const self_ref = self.ref();
    errdefer self_ref.unref();
    self.thread = try Thread.spawn(.{}, runEventLoop, .{self_ref});
    self.thread.setName("event loop") catch {};

    self.runtime.logDebug(
        "created `{*}`, label=`{s}`, public=`{}`",
        .{ self, self.label, self.is_public },
        @src(),
    );
    return self;
}

/// Acquires a new strong reference.
pub fn ref(self: *Self) *Self {
    const old = self.ref_count.fetchAdd(1, .monotonic);
    std.debug.assert(old != 0 and old != std.math.maxInt(usize));
    return self;
}

/// Releases a strong reference.
pub fn unref(self: *Self) void {
    const old = self.ref_count.fetchSub(1, .release);
    std.debug.assert(old != 0);
    if (old > 1) return;

    _ = self.ref_count.load(.acquire);

    if (self.runtime.getInstance()) |instance| instance.unref();
    self.unrefWeak();
}

/// Acquires a new weak reference.
pub fn refWeak(self: *Self) *Self {
    const old = self.weak_ref_count.fetchAdd(1, .monotonic);
    std.debug.assert(old != 0 and old != std.math.maxInt(usize));
    return self;
}

/// Releases a weak reference.
pub fn unrefWeak(self: *Self) void {
    const old = self.weak_ref_count.fetchSub(1, .release);
    std.debug.assert(old != 0);
    if (old > 1) return;

    _ = self.weak_ref_count.load(.acquire);

    // Wait until the thread is joined.
    std.debug.assert(self.signal_channel.isClosed());
    std.debug.assert(self.enqueue_requests.isClosed());
    std.debug.assert(self.private_message_queue.isClosed());
    std.debug.assert(self.workers.len == 0);
    std.debug.assert(self.command_buffer_count == 0);
    std.debug.assert(self.stack_allocators.len == 0);

    const allocator = self.allocator;
    allocator.free(self.label);
    allocator.destroy(self);
}

/// Releases a weak reference and waits for the thread to join.
pub fn unrefWeakAndJoin(self: *Self) void {
    // Wait until the thread is joined.
    self.runtime.logDebug("joining `{*}`", .{self}, @src());
    std.debug.assert(self.enqueue_requests.isClosed());
    self.thread.join();
    self.unrefWeak();
}

/// Acquires a new weak reference from a strong reference.
pub fn refWeakFromStrong(self: *Self) *Self {
    return self.refWeak();
}

/// Acquires a new strong reference from a weak reference.
pub fn refStrongFromWeak(self: *Self) bool {
    var old = self.ref_count.load(.acquire);
    while (old != 0) {
        if (self.ref_count.cmpxchgWeak(old, old + 1, .acquire, .monotonic)) |new| {
            old = new;
            continue;
        }
        return true;
    }
    return false;
}

/// Tries to acquire a new strong reference from a weak reference.
/// If this fails, it joins the pool thread and releases the weak reference.
pub fn refStrongFromWeakOrJoinAndUnref(self: *Self) bool {
    if (!self.should_join.load(.acquire) and self.refStrongFromWeak()) return true;
    self.unrefWeakAndJoin();
    return false;
}

pub fn enqueueCommandBuffer(self: *Self, buffer: *CommandBuffer) SendError!void {
    const buffer_ref = buffer.ref();
    errdefer buffer_ref.unref();
    return self.enqueue_requests.sender().send(&self.runtime.futex, buffer_ref);
}

pub fn wakeByAddress(self: *Self, value: *const atomic.Value(u32), max_waiters: usize) void {
    const allocator = self.allocator;
    const msg = allocator.create(PrivateMessage) catch @panic("oom");
    msg.* = .{ .msg = .{ .wake = .{ .value = value, .max_waiters = max_waiters } } };
    self.private_message_queue.sender().send(&self.runtime.futex, msg) catch |err| switch (err) {
        // If the private message queue is closed, we don't need to wake up anyone.
        error.Closed => allocator.destroy(msg),
        else => unreachable,
    };
}

pub fn requestClose(self: *Self) void {
    self.runtime.logDebug("`{*}` close requested", .{self}, @src());
    self.enqueue_requests.close(&self.runtime.futex);
    self.signal_channel.sender().trySend(&self.runtime.futex, {}) catch {};
}

pub fn getStackAllocator(self: *Self, size: StackSize) ?*StackAllocator {
    if (size == .default) return &self.stack_allocators[self.default_stack_allocator_idx];
    for (self.stack_allocators) |*allocator| {
        if (allocator.size >= @intFromEnum(size)) return allocator;
    }
    return null;
}

pub fn enqueueTask(self: *Self, task: *Task, worker: ?MetaWorker) void {
    std.debug.assert(task.msg == null);
    std.debug.assert(task.next == null);
    const futex = &self.runtime.futex;
    if (worker) |w| {
        self.runtime.logDebug("`{*}` enqueueing `{*}` to `{}`", .{ self, task, w }, @src());
        std.debug.assert(task.worker == null or task.worker == worker);
        const ptr = &self.workers[@intFromEnum(w)];
        ptr.private_queue.sender().send(futex, task) catch unreachable;
    } else {
        self.runtime.logDebug("`{*}` enqueueing `{*}` to global queue", .{ self, task }, @src());
        self.global_channel.sender(self.allocator).send(futex, task) catch |e| @panic(@errorName(e));
    }
}

fn processPrivateMessage(self: *Self, msg: *PrivateMessage) void {
    switch (msg.msg) {
        .complete => self.handleComplete(msg),
        .sleep => self.handleSleep(msg),
        .wait => self.handleWait(msg),
        .wake => self.handleWake(msg),
    }
}

fn handleComplete(self: *Self, msg: *PrivateMessage) void {
    _ = self;
    const complete = msg.msg.complete;
    const task = complete.task;
    const owner = task.owner;
    owner.completeTask(task, complete.is_error);
}

fn handleSleep(self: *Self, msg: *PrivateMessage) void {
    const timeout = &msg.msg.sleep.timeout;
    std.debug.assert(timeout.next == null);
    self.runtime.logDebug("`{*}` enqueueing sleep timeout `{*}`", .{ self, timeout }, @src());

    // Insert the timeout into the timeout queue.
    if (self.timeouts_head == null) {
        self.timeouts_head = timeout;
        return;
    }

    var link = &self.timeouts_head;
    var current = self.timeouts_head;
    while (current) |curr| {
        if (curr.timeout.order(timeout.timeout) != .gt) {
            link = &curr.next;
            current = curr.next;
            continue;
        }
        break;
    }
    timeout.next = current;
    link.* = timeout;
}

fn handleWait(self: *Self, msg: *PrivateMessage) void {
    const wait = &msg.msg.wait;
    const task: *Task = wait.task;
    std.debug.assert(task.next == null);
    self.runtime.logDebug("`{*}` processing wait for `{*}`", .{ self, task }, @src());

    // Wake the task if the value does not match the expected value.
    if (wait.value.load(.acquire) != wait.expect) {
        self.runtime.logDebug("`{*}` waking `{*}`", .{ self, task }, @src());
        wait.timed_out.* = false;
        task.msg = null;
        if (task.call_stack) |cs| cs.unblock();
        self.enqueueTask(task, task.worker);
        return;
    }

    // Enqueue the waiter.
    self.runtime.logDebug(
        "`{*}` enqueuing waiter `{x}` for `{*}`, key=`{*}`",
        .{ self, @intFromPtr(wait), task, wait.value },
        @src(),
    );
    const entry = self.waiters.getOrPutValue(self.allocator, wait.value, .{}) catch @panic("oom");
    const bucket = entry.value_ptr;
    if (bucket.tail) |tail| {
        tail.next = task;
    } else {
        bucket.head = task;
    }
    bucket.tail = task;

    // Enqueue the task timeout, if it has one.
    if (wait.timeout) |*timeout| blk: {
        self.runtime.logDebug(
            "`{*}` enqueueing wait timeout `{x}`, waiter=`{x}`",
            .{ self, @intFromPtr(timeout), @intFromPtr(wait) },
            @src(),
        );

        if (self.timeouts_head == null) {
            self.timeouts_head = timeout;
            break :blk;
        }

        var link = &self.timeouts_head;
        var current = self.timeouts_head;
        while (current) |curr| {
            if (curr.timeout.order(timeout.timeout) != .gt) {
                link = &curr.next;
                current = curr.next;
                continue;
            }
            break;
        }
        timeout.next = current;
        link.* = timeout;
    }
}

fn handleWake(self: *Self, msg: *PrivateMessage) void {
    const wake = msg.msg.wake;
    self.allocator.destroy(msg);
    self.runtime.logDebug(
        "`{*}` waking {} waiters, key=`{*}`",
        .{ self, wake.max_waiters, wake.value },
        @src(),
    );
    std.debug.assert(wake.max_waiters == 0 or wake.max_waiters == 1);
    if (wake.max_waiters == 0) return;

    // Dequeue the waiters from the bucket.
    const bucket = self.waiters.getPtr(wake.value) orelse return;

    var unparked: usize = 0;
    var link: *?*Task = &bucket.head;
    var current: ?*Task = bucket.head;
    var previous: ?*Task = null;
    while (current) |curr| {
        const curr_wait = &curr.msg.?.msg.wait;
        std.debug.assert(curr_wait.value == wake.value);

        link.* = curr.next;
        if (bucket.tail == curr) bucket.tail = previous;
        self.runtime.logDebug(
            "`{*}` waking waiter `{x}`",
            .{ self, @intFromPtr(curr_wait) },
            @src(),
        );

        // If the task registered a timeout we also dequeue it.
        if (curr_wait.timeout) |*timeout| {
            self.runtime.logDebug(
                "`{*}` removing timeout `{x}`, waiter=`{x}`",
                .{ self, @intFromPtr(timeout), @intFromPtr(curr_wait) },
                @src(),
            );
            var timeout_current = self.timeouts_head;
            var timeout_previous: ?*PrivateMessage.Timeout = null;
            while (timeout_current) |timeout_curr| {
                if (timeout_curr != timeout) {
                    timeout_current = timeout_curr.next;
                    timeout_previous = timeout_curr;
                    continue;
                }

                if (timeout_previous) |prev|
                    prev.next = timeout_curr.next
                else
                    self.timeouts_head = timeout_curr.next;
                break;
            }
        }

        link = &curr.next;
        previous = curr;
        current = curr.next;

        curr_wait.timed_out.* = false;
        curr.next = null;
        curr.msg = null;
        if (curr.call_stack) |cs| cs.unblock();
        self.enqueueTask(curr, curr.worker);

        unparked += 1;
        if (unparked == wake.max_waiters) break;
    }

    // Remove the bucket if it's empty.
    if (bucket.head == null) _ = self.waiters.swapRemove(wake.value);
}

fn handleTimeout(self: *Self, timeout: *PrivateMessage.Timeout) void {
    self.runtime.logDebug("`{*}` timeout `{x}` reached", .{ self, @intFromPtr(timeout) }, @src());

    const task = timeout.task;
    const wait = switch (task.msg.?.msg) {
        .complete, .wake => unreachable,
        .sleep => {
            self.runtime.logDebug("`{*}` waking `{*}` from sleep", .{ self, task }, @src());
            task.msg = null;
            std.debug.assert(task.next == null);
            if (task.call_stack) |cs| cs.unblock();
            self.enqueueTask(task, task.worker);
            return;
        },
        .wait => |*v| blk: {
            self.runtime.logDebug(
                "`{*}` timing out waiter `{x}`",
                .{ self, @intFromPtr(v) },
                @src(),
            );
            break :blk v;
        },
    };
    std.debug.assert(wait.timeout != null);

    // Dequeue the waiter from the bucket.
    const bucket = self.waiters.getPtr(wait.value) orelse unreachable;

    var link: *?*Task = &bucket.head;
    var current: ?*Task = bucket.head;
    var previous: ?*Task = null;
    while (current) |curr| {
        if (curr != task) {
            link = &curr.next;
            previous = curr;
            current = curr.next;
            continue;
        }

        link.* = task.next;
        if (bucket.tail == task) bucket.tail = previous;

        wait.timed_out.* = true;
        task.next = null;
        task.msg = null;
        if (task.call_stack) |cs| cs.unblock();
        self.enqueueTask(task, task.worker);
        break;
    }

    // Remove the bucket if it's empty.
    if (bucket.head == null) _ = self.waiters.swapRemove(wait.value);
}

fn processEnqueueRequest(self: *Self, buffer: *CommandBuffer) void {
    std.debug.assert(buffer.owner == self);
    std.debug.assert(buffer.enqueue_status == .dequeued);
    self.runtime.logDebug(
        "`{*}` spawning `{*}`",
        .{ self, buffer },
        @src(),
    );
    self.command_buffer_count += 1;
    buffer.enqueueToPool();
}

fn runEventLoop(self: *Self) void {
    const tracing = self.runtime.getTracing();
    if (tracing) |tr| tr.registerThread();
    defer if (tracing) |tr| tr.unregisterThread();

    const span = if (tracing) |tr| Tracing.Span.initTrace(
        tr,
        null,
        null,
        @src(),
        "event loop, pool=`{*}`",
        .{self},
    ) else null;
    defer if (span) |sp| sp.deinit();

    const rx = multi_receiver(&.{ *PrivateMessage, *CommandBuffer, void }, .{
        self.private_message_queue.receiver(),
        self.enqueue_requests.receiver(),
        self.signal_channel.receiver(),
    });
    const futex = &self.runtime.futex;

    const max_timeout = Instant.now().addSaturating(Duration.Max);
    var next_timeout = max_timeout;
    while (true) {
        // Wait until the next message arrives or timeout occurs.
        var curr_msg = rx.recvUntil(futex, next_timeout) catch |err| switch (err) {
            error.Timeout => blk: {
                self.runtime.logDebug("`{*}` recv timed out", .{self}, @src());
                break :blk null;
            },
            else => break,
        };

        // Clear the message queues.
        while (curr_msg) |msg| {
            switch (msg) {
                .@"0" => |v| self.processPrivateMessage(v),
                .@"1" => |v| self.processEnqueueRequest(v),
                .@"2" => {},
            }
            curr_msg = rx.tryRecv() catch null;
        }

        // Clear the cached stacks.
        for (self.stack_allocators) |*al| al.clearCache();

        // Compute the next timeout and wake timed out waiters.
        next_timeout = max_timeout;
        const now = Instant.now();
        while (self.timeouts_head) |timeout| {
            next_timeout = timeout.timeout;
            if (now.order(timeout.timeout) == .lt) break;
            self.timeouts_head = timeout.next;
            self.handleTimeout(timeout);
        }

        // Process the command buffers.
        while (self.process_list_head) |head| {
            self.process_list_head = null;
            self.process_list_tail = null;

            var current: ?*CommandBuffer = head;
            while (current) |curr| {
                current = curr.next;
                curr.next = null;
                curr.processEntry();
            }
        }

        // If there are no more command buffers and the public message queue is closed we can stop
        // the event loop.
        if (self.command_buffer_count == 0 and self.enqueue_requests.isClosed()) {
            self.runtime.logDebug("`{*}` closing", .{self}, @src());
            self.signal_channel.close(futex);
            self.private_message_queue.close(futex);
        }
    }
    self.runtime.logDebug("`{*}` terminating", .{self}, @src());

    // Join all worker threads.
    self.should_join.store(true, .release);
    std.debug.assert(self.task_count.load(.acquire) == 0);
    std.debug.assert(self.command_buffer_count == 0);
    std.debug.assert(self.process_list_head == null);
    std.debug.assert(self.process_list_tail == null);
    std.debug.assert(self.timeouts_head == null);
    std.debug.assert(self.waiters.count() == 0);
    self.global_channel.close(futex);

    for (self.workers) |*worker| {
        worker.private_queue.close(futex);
        worker.thread.join();
        std.debug.assert(worker.private_task_count.load(.acquire) == 0);
    }
    self.allocator.free(self.workers);
    self.workers = &.{};

    self.global_channel.deinit(self.allocator);
    for (self.stack_allocators) |*stack_allocator| {
        for (stack_allocator.hot_freelist.items) |stack| stack.deinit();
        for (stack_allocator.cold_freelist.items) |stack| stack.deinit();
        for (stack_allocator.deallocation_cache.items) |stack| stack.deinit();
        stack_allocator.hot_freelist.deinit(self.allocator);
        stack_allocator.cold_freelist.deinit(self.allocator);
        stack_allocator.deallocation_cache.deinit(self.allocator);
    }
    self.allocator.free(self.stack_allocators);
    self.stack_allocators = &.{};

    self.waiters.clearAndFree(self.allocator);

    self.unref();
}

pub fn asMetaPool(self: *Self) MetaPool {
    return .{ .data = self, .vtable = &vtable };
}

const VTableImpl = struct {
    fn id(handle: *anyopaque) callconv(.c) MetaId {
        const this: *Self = @ptrCast(@alignCast(handle));
        return @enumFromInt(@intFromPtr(this));
    }
    fn ref(handle: *anyopaque) callconv(.c) void {
        const this: *Self = @ptrCast(@alignCast(handle));
        _ = this.ref();
    }
    fn unref(handle: *anyopaque) callconv(.c) void {
        const this: *Self = @ptrCast(@alignCast(handle));
        this.unref();
    }
    fn requestClose(handle: *anyopaque) callconv(.c) void {
        const this: *Self = @ptrCast(@alignCast(handle));
        this.requestClose();
    }
    fn acceptsRequests(handle: *anyopaque) callconv(.c) bool {
        const this: *Self = @ptrCast(@alignCast(handle));
        return !this.enqueue_requests.isClosed();
    }
    fn ownsCurrentThread(handle: *anyopaque) callconv(.c) bool {
        const this: *Self = @ptrCast(@alignCast(handle));
        return Worker.currentPool() == this;
    }
    fn label(handle: *anyopaque, len: *usize) callconv(.c) ?[*]const u8 {
        const this: *Self = @ptrCast(@alignCast(handle));
        len.* = this.label.len;
        return this.label.ptr;
    }
    fn workers(handle: *anyopaque, ptr: ?[*]MetaWorker, len: usize) callconv(.c) usize {
        const this: *Self = @ptrCast(@alignCast(handle));
        if (ptr) |w| {
            for (w[0..@min(len, this.workers.len)], 0..) |*worker, i| {
                worker.* = @enumFromInt(i);
            }
        }
        return this.workers.len;
    }
    fn stack_sizes(handle: *anyopaque, ptr: ?[*]StackSize, len: usize) callconv(.c) usize {
        const this: *Self = @ptrCast(@alignCast(handle));
        if (ptr) |w| {
            const min_len = @min(len, this.stack_allocators.len);
            for (w[0..min_len], this.stack_allocators[0..min_len]) |*size, *all| {
                size.* = @enumFromInt(all.size);
            }
        }
        return this.stack_allocators.len;
    }
    fn enqueueBuffer(
        handle: *anyopaque,
        buffer: *MetaCommandBuffer,
        buff_handle: ?*MetaCommandBufferHandle,
    ) callconv(.c) AnyResult {
        const this: *Self = @ptrCast(@alignCast(handle));
        const b = CommandBuffer.init(this, buffer) catch |err| {
            for (buffer.entries()) |entry| entry.abort();
            buffer.abort();
            buffer.deinit();
            return AnyError.initError(err).intoResult();
        };
        this.enqueueCommandBuffer(b) catch |err| {
            b.abortDeinit();
            return AnyError.initError(err).intoResult();
        };
        if (buff_handle) |h| {
            h.* = b.asHandle();
        } else b.unref();
        return AnyResult.ok;
    }
};
const vtable = MetaPoolVTable{
    .id = &VTableImpl.id,
    .ref = &VTableImpl.ref,
    .unref = &VTableImpl.unref,
    .request_close = &VTableImpl.requestClose,
    .accepts_requests = &VTableImpl.acceptsRequests,
    .owns_current_thread = &VTableImpl.ownsCurrentThread,
    .label = &VTableImpl.label,
    .workers = &VTableImpl.workers,
    .stack_sizes = &VTableImpl.stack_sizes,
    .enqueue_buffer = &VTableImpl.enqueueBuffer,
};
