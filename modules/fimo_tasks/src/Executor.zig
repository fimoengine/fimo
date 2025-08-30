const std = @import("std");
const debug = std.debug;
const atomic = std.atomic;
const mem = std.mem;
const Alignment = mem.Alignment;
const Allocator = mem.Allocator;
const Thread = std.Thread;
const ArrayList = std.ArrayList;
const DoublyLinkedList = std.DoublyLinkedList;
const heap = std.heap;
const ArenaAllocator = heap.ArenaAllocator;
const posix = std.posix;
const builtin = @import("builtin");

const fimo_std = @import("fimo_std");
const ctx = fimo_std.ctx;
const Status = ctx.Status;
const AnyResult = fimo_std.AnyError.AnyResult;
const tracing = fimo_std.tracing;
const CallStack = tracing.CallStack;
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const win32 = @import("win32");

const Context = @import("context.zig").Context;
const FimoTasks = @import("FimoTasks.zig");
const Futex = @import("Futex.zig");
const Stack = @import("context.zig").Stack;
const Transfer = @import("context.zig").Transfer;

fn ItemPool(Item: type) type {
    return struct {
        const Self = @This();

        const waiting_bit: usize = 0b1;
        const list_mask: usize = ~waiting_bit;

        const Node = struct {
            item: Item,
            next: ?*@This(),
        };

        const Head = packed struct(u128) {
            tag: usize = 0,
            head: usize = 0,
        };

        nodes: []Node,
        free_list: atomic.Value(Head) = .init(.{}),

        fn init(allocator: Allocator, capacity: usize) error{OutOfMemory}!Self {
            const nodes = try allocator.alloc(Node, capacity);

            var head: ?*Node = null;
            for (nodes) |*node| {
                node.next = head;
                head = node;
            }

            return .{
                .nodes = nodes,
                .free_list = .init(.{ .head = @intFromPtr(head) }),
            };
        }

        fn deinit(self: Self, allocator: Allocator) void {
            allocator.free(self.nodes);
        }

        fn create(self: *Self, futex: *Futex) *Item {
            var orig = self.free_list.load(.monotonic);
            while (true) {
                if (orig.head & list_mask == 0) {
                    @branchHint(.cold);
                    const next = Head{ .tag = orig.tag, .head = waiting_bit };
                    if (orig.head == 0) {
                        if (self.free_list.cmpxchgWeak(orig, next, .monotonic, .monotonic)) |v| {
                            orig = v;
                            continue;
                        }
                    }
                    const raw_mem: *[2]usize = @ptrCast(&self.free_list);
                    futex.wait(&raw_mem[1], @sizeOf(usize), waiting_bit, 0, null) catch {};
                    orig = self.free_list.load(.monotonic);
                    continue;
                } else {
                    orig = self.free_list.load(.acquire);
                    if (orig.head & list_mask == 0) continue;
                }
                debug.assert(orig.head & waiting_bit == 0);

                const node: *Node = @ptrFromInt(orig.head);
                const next = Head{ .tag = orig.tag +% 1, .head = @intFromPtr(node.next) };

                if (self.free_list.cmpxchgWeak(orig, next, .acquire, .monotonic)) |v| {
                    orig = v;
                    continue;
                }

                node.item = undefined;
                return &node.item;
            }
        }

        fn destroy(self: *Self, futex: *Futex, item: *Item) void {
            const node: *Node = @fieldParentPtr("item", item);
            var orig = self.free_list.load(.monotonic);
            while (true) {
                node.next = @ptrFromInt(orig.head & list_mask);
                const next = Head{ .tag = orig.tag +% 1, .head = @intFromPtr(node) };
                orig = self.free_list.cmpxchgWeak(
                    orig,
                    next,
                    .release,
                    .monotonic,
                ) orelse break;
            }
            const free_list_raw_mem: *[2]usize = @ptrCast(&self.free_list);
            if (orig.head == waiting_bit) _ = futex.wake(&free_list_raw_mem[1], 1);
        }
    };
}

const StackPool = struct {
    allocated: []align(heap.page_size_min) u8,
    page_size: usize,
    cache: ArrayList(Stack),
    free_list: ArrayList(Stack),
    wait_queue: DoublyLinkedList = .{},

    fn init(min_stack_size: usize, stack_count: usize, cache_len: usize) error{OutOfMemory}!StackPool {
        // NOTE(gabriel):
        //
        // We allocate an additional guard page for each stack.
        const page_size = heap.pageSize();
        const platform_min_stack_size = @import("context.zig").minStackSize();
        const platform_max_stack_size = @import("context.zig").maxStackSize();
        const stack_size = mem.alignForward(
            usize,
            @min(@max(min_stack_size, platform_min_stack_size) + page_size, platform_max_stack_size),
            page_size,
        );

        const cache_size = mem.alignForward(usize, @sizeOf(Stack) * stack_count, page_size);
        const all_stacks_size = stack_count * stack_size;
        const alloc_size = cache_size + all_stacks_size;
        const allocated: []align(heap.page_size_min) u8 = switch (comptime builtin.os.tag) {
            .windows => blk: {
                const alloc = win32.system.memory.VirtualAlloc(
                    null,
                    alloc_size,
                    .{ .RESERVE = 1 },
                    .{ .PAGE_READWRITE = 1 },
                ) orelse return error.OutOfMemory;
                errdefer _ = win32.system.memory.VirtualFree(alloc, alloc_size, .RELEASE);

                _ = win32.system.memory.VirtualAlloc(
                    alloc,
                    cache_size,
                    .{ .COMMIT = 1 },
                    .{ .PAGE_READWRITE = 1 },
                ) orelse return error.OutOfMemory;

                const alloc_bytes: [*]align(heap.page_size_min) u8 = @ptrCast(@alignCast(alloc));
                break :blk alloc_bytes[0..alloc_size];
            },
            else => posix.mmap(
                null,
                alloc_size,
                posix.PROT.READ | posix.PROT.WRITE,
                .{ .TYPE = .PRIVATE, .ANONYMOUS = true },
                -1,
                0,
            ) catch return error.OutOfMemory,
        };
        errdefer switch (comptime builtin.os.tag) {
            .windows => _ = win32.system.memory.VirtualFree(allocated.ptr, alloc_size, .RELEASE),
            else => posix.munmap(allocated),
        };

        const stacks: []Stack = @ptrCast(allocated[0 .. @sizeOf(Stack) * stack_count]);
        const stacks_memory = allocated[cache_size..];
        for (stacks, 0..) |*stack, i| {
            const stack_memory: []align(heap.page_size_min) u8 = @alignCast(
                stacks_memory[i * stack_size ..][0..stack_size],
            );
            stack.memory = stack_memory;
            stack.commited_size = if (comptime builtin.os.tag == .windows) page_size else {};

            switch (comptime builtin.os.tag) {
                .windows => {
                    const commit_region = stack_memory[stack_size - (2 * page_size) ..];
                    _ = win32.system.memory.VirtualAlloc(
                        commit_region.ptr,
                        commit_region.len,
                        .{ .COMMIT = 1 },
                        .{ .PAGE_READWRITE = 1 },
                    ) orelse return error.OutOfMemory;
                    var old: win32.system.memory.PAGE_PROTECTION_FLAGS = undefined;
                    if (win32.system.memory.VirtualProtect(
                        commit_region.ptr,
                        page_size,
                        .{ .PAGE_READWRITE = 1, .PAGE_GUARD = 1 },
                        &old,
                    ) == 0) return error.OutOfMemory;
                },
                else => posix.mprotect(
                    stack_memory[0..page_size],
                    posix.PROT.NONE,
                ) catch return error.OutOfMemory,
            }
        }

        return .{
            .allocated = allocated,
            .page_size = page_size,
            .cache = .{ .items = stacks[0..cache_len], .capacity = cache_len },
            .free_list = .{ .items = stacks[cache_len..], .capacity = stack_count - cache_len },
        };
    }

    fn deinit(self: StackPool) void {
        debug.assert(self.cache.items.len == self.cache.capacity);
        debug.assert(self.free_list.items.len == self.free_list.capacity);
        debug.assert(self.wait_queue.len() == 0);
        errdefer switch (comptime builtin.os.tag) {
            .windows => win32.system.memory.VirtualFree(
                self.allocated.ptr,
                self.allocated.len,
                .RELEASE,
            ),
            else => posix.munmap(self.allocated),
        };
    }

    fn pop(self: *StackPool, waiter: *CmdBuf) ?Stack {
        if (self.cache.pop()) |s| return s;
        if (self.free_list.pop()) |s| return s;
        debug.assert(!waiter.enqueued);
        waiter.enqueued = true;
        self.wait_queue.append(&waiter.node);
        return null;
    }

    fn push(self: *StackPool, stack: Stack) ?*CmdBuf {
        self.cache.appendBounded(stack) catch {
            switch (comptime builtin.os.tag) {
                .windows => blk: {
                    if (stack.commited_size == self.page_size) {
                        self.free_list.appendAssumeCapacity(stack);
                        break :blk;
                    }

                    // NOTE(gabriel): Decommit all but the last two pages.
                    const stack_size = stack.memory.len;
                    const commit_size = stack.commited_size;
                    const commited_start = stack_size - (commit_size + self.page_size);
                    const commited_region = stack.memory[commited_start..];
                    const decommit_region = commited_region[0 .. commit_size - self.page_size];
                    const commit_region = commited_region[commit_size - self.page_size ..];
                    _ = win32.system.memory.VirtualFree(
                        decommit_region.ptr,
                        decommit_region.len,
                        .DECOMMIT,
                    );
                    var old: win32.system.memory.PAGE_PROTECTION_FLAGS = undefined;
                    _ = win32.system.memory.VirtualProtect(
                        commit_region.ptr,
                        self.page_size,
                        .{ .PAGE_READWRITE = 1, .PAGE_GUARD = 1 },
                        &old,
                    );

                    self.free_list.appendAssumeCapacity(.{
                        .memory = stack.memory,
                        .commited_size = self.page_size,
                    });
                },
                else => {
                    posix.madvise(
                        stack.memory.ptr,
                        stack.memory.len,
                        posix.MADV.DONTNEED,
                    ) catch unreachable;
                    self.free_list.appendAssumeCapacity(stack);
                },
            }
        };
        const waiter = self.wait_queue.popFirst() orelse return null;
        const cmd_buf: *CmdBuf = @alignCast(@fieldParentPtr("node", waiter));
        cmd_buf.enqueued = false;
        return cmd_buf;
    }
};

const Message = struct {
    tag: enum {
        task_to_worker,
        dealloc_cmd_buf,
        enqueue_cmd_buf,
        wake,
        task_complete,
        task_abort,
        task_sleep,
        task_wait,
        join,
    },
    next: ?*Message = null,
};

const WakeMessage = struct {
    ptr: *const atomic.Value(u32),
    msg: Message = .{ .tag = .wake },
};

const MessageQueue = struct {
    const waiting_bit: usize = 0b1;
    const list_mask: usize = ~waiting_bit;

    msg_count: atomic.Value(usize) align(atomic.cache_line) = .init(0),
    push_list: atomic.Value(usize) align(atomic.cache_line) = .init(0),
    pop_list: atomic.Value(?*Message) align(atomic.cache_line) = .init(null),

    pub fn push(self: *MessageQueue, futex: *Futex, msg: *Message) void {
        _ = self.msg_count.fetchAdd(1, .monotonic);
        debug.assert(msg.next == null);
        var orig = self.push_list.load(.monotonic);
        while (true) {
            msg.next = @ptrFromInt(orig & list_mask);
            if (self.push_list.cmpxchgWeak(orig, @intFromPtr(msg), .release, .monotonic)) |v| {
                orig = v;
                continue;
            }
            if (orig == waiting_bit) _ = futex.wake(&self.push_list, 1);
            return;
        }
    }

    fn tryPop(self: *MessageQueue) ?*Message {
        if (self.pop_list.load(.acquire)) |msg| {
            _ = self.msg_count.fetchSub(1, .monotonic);
            self.pop_list.store(msg.next, .release);
            msg.next = null;
            return msg;
        }

        const orig = self.push_list.fetchAnd(~list_mask, .acquire);
        if (orig & list_mask == 0) return null;

        var head: *Message = @ptrFromInt(orig & list_mask);
        var pop_list_head: ?*Message = null;
        while (head.next) |next| {
            head.next = pop_list_head;
            pop_list_head = head;
            head = next;
        }
        self.pop_list.store(pop_list_head, .release);
        debug.assert(head.next == null);
        _ = self.msg_count.fetchSub(1, .monotonic);
        return head;
    }

    fn pop(self: *MessageQueue, futex: *Futex, timeout: Instant) ?*Message {
        return self.tryPop() orelse self.popSlow(futex, timeout);
    }

    fn popSlow(self: *MessageQueue, futex: *Futex, timeout: Instant) ?*Message {
        @branchHint(.cold);
        var spin_count: usize = 0;
        const spin_relax_limit = 12;
        const spin_limit = spin_relax_limit + 4;

        while (true) {
            if (self.tryPop()) |msg| {
                _ = self.push_list.fetchAnd(list_mask, .monotonic);
                _ = self.msg_count.fetchSub(1, .monotonic);
                return msg;
            }

            if (spin_count < spin_limit) {
                if (spin_count < spin_relax_limit)
                    atomic.spinLoopHint()
                else
                    Thread.yield() catch {};
                spin_count += 1;
                continue;
            }

            const orig = self.push_list.load(.monotonic);
            if (orig & list_mask != 0) continue;
            if (orig & waiting_bit == 0) {
                if (self.push_list.cmpxchgWeak(orig, waiting_bit, .monotonic, .monotonic)) |_| {
                    continue;
                }
            }
            futex.wait(&self.push_list, @sizeOf(usize), waiting_bit, 0, timeout) catch |err|
                switch (err) {
                    error.Invalid => {},
                    error.Timeout => return null,
                };
            spin_count = 0;
        }
    }
};

const LocalQueue = struct {
    internal: MessageQueue = .{},

    fn push(self: *LocalQueue, futex: *Futex, task: *Task) void {
        debug.assert(!task.enqueued);
        task.enqueued = true;
        self.internal.push(futex, &task.msg);
    }

    fn tryPop(self: *LocalQueue) ?*Task {
        const msg = self.internal.tryPop() orelse return null;
        const task: *Task = @alignCast(@fieldParentPtr("msg", msg));
        debug.assert(task.enqueued);
        task.enqueued = false;
        return task;
    }
};

const GlobalChannel = struct {
    // NOTE(gabriel):
    //
    // The implementation uses a sum tree to identify free slots.
    // The number of nodes in a perfect binary tree is `2^depth - 1`, with `2^(depth-1)` leaf
    // nodes. Assuming 64bit `usize`, we are bounded to `2^64 - 1` nodes and `2^63` leaf nodes.
    // This leaves the MSB unused , which we employ the determine if the channel is closed.
    const closed_bit_position = @bitSizeOf(usize) - 1;
    const closed_bit: usize = 1 << closed_bit_position;
    const counter_mask: usize = ~closed_bit;

    depth: u8,
    elements: []Element,
    counters: []Counter,

    const Element = struct {
        value: ?*Task = null,
        filled: atomic.Value(bool) align(atomic.cache_line) = .init(false),
    };
    const Counter = struct {
        value: atomic.Value(usize) align(atomic.cache_line) = .init(0),
    };

    fn init(allocator: Allocator, num_tasks: usize) error{OutOfMemory}!GlobalChannel {
        const num_elements = std.math.ceilPowerOfTwo(usize, @max(1, num_tasks)) catch
            return error.OutOfMemory;
        const depth = std.math.log2(num_elements) + 1;
        if (depth >= 63) return error.OutOfMemory;

        const elements = try allocator.alloc(Element, num_elements);
        for (elements) |*c| c.* = .{};
        const counters = try allocator.alloc(Counter, (num_elements * 2) - 1);
        for (counters) |*c| c.* = .{};

        return .{
            .depth = @truncate(depth),
            .elements = elements,
            .counters = counters,
        };
    }

    fn close(self: *GlobalChannel, futex: *Futex) void {
        const root = &self.counters[0].value;
        const orig = root.fetchOr(closed_bit, .release);
        if (orig == 0) _ = futex.wake(root, std.math.maxInt(usize));
    }

    fn push(self: *GlobalChannel, futex: *Futex, task: *Task) void {
        debug.assert(!task.enqueued);

        var elem_idx: usize = 0;
        var max_elements: usize = self.elements.len >> 1;
        if (self.depth > 1) {
            for (1..self.depth - 1) |layer_idx| {
                const layer_start: usize = (@as(usize, 1) << @truncate(layer_idx)) - 1;
                const left_idx = layer_start + elem_idx;

                const left_count = self.counters[left_idx].value.load(.monotonic);
                if (left_count == max_elements) {
                    // NOTE(gabriel):
                    //
                    // First child is already full, continue with the second.
                    elem_idx = (elem_idx << 1) | 1;
                } else {
                    // NOTE(gabriel):
                    //
                    // First child has some capacity left, continue there.
                    elem_idx = (elem_idx << 1);
                }

                max_elements >>= 1;
            }
        }

        // NOTE(gabriel):
        //
        // A consumer may not be finished reading it yet, so we wait until it is consumed.
        // We know that some thread is in the process of consuming it, as the counter indicated
        // an empty slot.
        var slot = &self.elements[elem_idx];
        const spin_count_relax = 12;
        var spin_count: usize = 0;
        while (slot.filled.load(.acquire)) {
            if (spin_count < spin_count_relax)
                atomic.spinLoopHint()
            else
                std.Thread.yield() catch unreachable;
            spin_count += 1;
        }
        debug.assert(slot.value == null);
        task.enqueued = true;
        slot.value = task;
        slot.filled.store(true, .release);

        // NOTE(gabriel):
        //
        // Traverse the tree from the leaf to the root and increase the counter.
        var num_elements: usize = 0;
        var counter_index = (@as(usize, 1) << @truncate(self.depth - 1)) + elem_idx - 1;
        for (0..self.depth) |_| {
            const counter = &self.counters[counter_index];
            num_elements = counter.value.fetchAdd(1, .release);
            counter_index -%= 1;
            counter_index >>= 1;
        }

        if (num_elements == 0) _ = futex.wake(&self.counters[0].value, 1);
    }

    fn tryPop(self: *GlobalChannel, seed: usize) error{Closed}!?*Task {
        // NOTE(gabriel):
        //
        // The first coutner of the tree serves as the counter of tasks
        // in the channel. If it is `0` we can stop immediately, otherwise
        // we try to decrement it by one, reserving one element in the channel.
        {
            const root = &self.counters[0];
            var orig = root.value.load(.monotonic);
            while (true) {
                if (orig & counter_mask == 0) {
                    if (orig & closed_bit != 0) return error.Closed;
                    return null;
                }
                const new_counter = orig - 1;
                if (root.value.cmpxchgWeak(orig, new_counter, .acquire, .monotonic)) |v| {
                    orig = v;
                    continue;
                }
                break;
            }
        }

        // NOTE(gabriel):
        //
        // Traverse the tree to find the slot that belongs to us, decrementing the node count
        // along the way by one. We use the seed to determine whether to peek into the left or
        // right child node.
        var elem_idx: usize = 0;
        var search_order: usize = seed;
        if (self.depth != 0) {
            for (1..self.depth) |layer_idx| {
                const layer_start: usize = (@as(usize, 1) << @truncate(layer_idx)) - 1;
                const left_idx = layer_start + elem_idx;
                const right_idx = left_idx + 1;

                // NOTE(gabriel):
                //
                // The LSB determines if we prefer the left or right child node.
                const first_idx, const second_idx = if (search_order & 1 == 0)
                    .{ left_idx, right_idx }
                else
                    .{ right_idx, left_idx };
                const first_node = &self.counters[first_idx];
                const second_node = &self.counters[second_idx];

                while (true) {
                    // NOTE(gabriel):
                    //
                    // If the preferred node is not empty, we always choose that one.
                    const first_count = first_node.value.load(.monotonic);
                    if (first_count != 0) {
                        const new_count = first_count - 1;
                        if (first_node.value.cmpxchgWeak(first_count, new_count, .acquire, .monotonic)) |_| {
                            continue;
                        }
                        elem_idx = (elem_idx << 1) | @intFromBool(first_idx == right_idx);
                        break;
                    }

                    // NOTE(gabriel):
                    //
                    // Now that the first node is empty, we are forced to choose the second one.
                    // We can not preform an unconditional decrement, as it is possible that, in
                    // the time between the decrement of the parent node and that of the child node,
                    // the producer inserts another element into the channel. This may lead to the
                    // parent node and the first node being incremented by one. Then another consumer
                    // may consume the element accessed through the second node.
                    const second_count = second_node.value.load(.monotonic);
                    if (second_count != 0) {
                        const new_count = second_count - 1;
                        if (second_node.value.cmpxchgWeak(second_count, new_count, .acquire, .monotonic)) |_| {
                            continue;
                        }
                        elem_idx = (elem_idx << 1) | @intFromBool(second_idx == right_idx);
                        break;
                    }
                }

                // NOTE(gabriel):
                //
                // Shift the order by one to the right to discard the LSB for the current layer.
                search_order >>= 1;
            }
        }

        const slot = &self.elements[elem_idx];
        debug.assert(slot.filled.load(.acquire) == true);
        const task = slot.value.?;
        slot.value = null;
        slot.filled.store(false, .release);
        debug.assert(task.enqueued);
        task.enqueued = false;
        return task;
    }

    fn tryPopCombined(
        self: *GlobalChannel,
        local_queue: *LocalQueue,
        seed: usize,
        comptime prefer_local: bool,
    ) error{Closed}!?*Task {
        if (comptime prefer_local) {
            if (local_queue.tryPop()) |t| return t;
            return self.tryPop(seed);
        } else {
            const res = self.tryPop(seed) catch {
                return local_queue.tryPop() orelse error.Closed;
            };
            if (res) |t| return t;
            return local_queue.tryPop();
        }
    }

    fn pop(
        self: *GlobalChannel,
        local_queue: *LocalQueue,
        futex: *Futex,
        seed: usize,
        comptime prefer_local: bool,
    ) error{Closed}!*Task {
        if (try self.tryPopCombined(local_queue, seed, prefer_local)) |t| return t;
        return self.popSlow(local_queue, futex, seed, prefer_local);
    }

    fn popSlow(
        self: *GlobalChannel,
        local_queue: *LocalQueue,
        futex: *Futex,
        seed: usize,
        comptime prefer_local: bool,
    ) error{Closed}!*Task {
        @branchHint(.cold);
        const global_expect: Futex.KeyExpect = .{
            .key = &self.counters[0].value,
            .key_size = @sizeOf(*Task),
            .expect = 0,
        };
        const local_expect: Futex.KeyExpect = .{
            .key = &local_queue.internal.push_list,
            .key_size = @sizeOf(usize),
            .expect = MessageQueue.waiting_bit,
        };
        const keys: [2]Futex.KeyExpect = if (comptime prefer_local)
            .{ local_expect, global_expect }
        else
            .{ global_expect, local_expect };

        var spin_count: usize = 0;
        const spin_relax_limit = 12;
        const spin_limit = spin_relax_limit + 4;
        while (true) {
            if (try self.tryPopCombined(local_queue, seed, prefer_local)) |t| return t;
            if (spin_count < spin_limit) {
                if (spin_count < spin_relax_limit)
                    atomic.spinLoopHint()
                else
                    Thread.yield() catch {};
                spin_count += 1;
                continue;
            }

            if (local_queue.internal.push_list.load(.monotonic) == 0) {
                if (local_queue.internal.push_list.cmpxchgWeak(
                    0,
                    MessageQueue.waiting_bit,
                    .monotonic,
                    .monotonic,
                )) |_| {
                    continue;
                }
            }
            _ = futex.waitv(&keys, null) catch 0;
            spin_count = 0;
        }
    }
};

const TimeoutQueue = struct {
    head: ?*Task.Timeout = null,

    fn push(self: *TimeoutQueue, task: *Task) void {
        debug.assert(task.msg.tag == .task_sleep or task.msg.tag == .task_wait);

        if (self.head == null) {
            debug.assert(task.timeout.next == null);
            self.head = &task.timeout;
            return;
        }

        var link = &self.head;
        var current = self.head;
        while (current) |curr| {
            if (curr.timeout.order(task.timeout.timeout) != .gt) {
                link = &curr.next;
                current = curr.next;
                continue;
            }
            break;
        }
        task.timeout.next = current;
        link.* = &task.timeout;
    }

    fn pop(self: *TimeoutQueue, timeout: Instant) ?*Task {
        const head = self.head orelse return null;
        if (head.timeout.order(timeout) == .gt) return null;
        self.head = head.next;
        head.next = null;
        const task: *Task = @alignCast(@fieldParentPtr("timeout", head));
        return task;
    }

    fn removeTask(self: *TimeoutQueue, task: *Task) void {
        var link = &self.head;
        var current = self.head;
        while (current) |curr| {
            const t: *Task = @alignCast(@fieldParentPtr("timeout", curr));
            if (t != task) {
                link = &curr.next;
                current = curr.next;
                continue;
            }
            link.* = curr.next;
            curr.next = null;
            return;
        }

        unreachable;
    }

    fn getNextTimeout(self: *TimeoutQueue) Instant {
        const head = self.head orelse return .Max;
        return head.timeout;
    }
};

const WaitList = struct {
    slots: []?*Task,

    fn init(allocator: Allocator, num_tasks: usize) error{OutOfMemory}!WaitList {
        const slots = try allocator.alloc(?*Task, num_tasks);
        @memset(slots, null);
        return .{ .slots = slots };
    }

    fn push(self: *WaitList, task: *Task) void {
        debug.assert(task.msg.tag == .task_wait);
        const hash = std.hash.int(@intFromPtr(task.wait_state.value));
        for (0..self.slots.len) |i| {
            // NOTE(gabriel): The number of slots is a power of two.
            const slot_idx = (hash + i) & (self.slots.len - 1);
            if (self.slots[slot_idx] != null) continue;
            self.slots[slot_idx] = task;
            return;
        }

        unreachable;
    }

    fn pop(self: *WaitList, key: *const atomic.Value(u32)) ?*Task {
        const hash = std.hash.int(@intFromPtr(key));
        for (0..self.slots.len) |i| {
            // NOTE(gabriel): The number of slots is a power of two.
            const slot_idx = (hash + i) & (self.slots.len - 1);
            const slot = self.slots[slot_idx] orelse continue;
            if (slot.wait_state.value != key) continue;
            self.slots[slot_idx] = null;
            return slot;
        }

        return null;
    }
};

pub const Task = struct {
    const WaitState = struct {
        value: *const atomic.Value(u32),
        expect: u32,
        timed_out: *bool,
    };

    const Timeout = struct {
        timeout: Instant = .Max,
        next: ?*Timeout = null,
    };

    const LocalValue = struct {
        key: ?*const anyopaque = null,
        value: ?*anyopaque = null,
        dtor: ?*const fn (?*anyopaque) callconv(.c) void = null,
    };
    const num_local_values = 128;

    owner: *CmdBuf,
    cmd_idx: usize,
    batch_idx: usize,
    task: *fimo_tasks_meta.Task,
    stack: Stack,
    enqueued: bool = false,
    bound_to_worker: bool = false,
    worker: ?*Worker,
    timeout: Timeout = .{},
    wait_state: WaitState = undefined,
    msg: Message = undefined,
    call_stack: *CallStack,
    context: Context,
    local_result: AnyResult = .ok,
    local_values: [num_local_values]LocalValue = @splat(.{}),
    spawn_list_node: DoublyLinkedList.Node = .{},

    // NOTE(gabriel):
    //
    // Some cleanup operations must occur in the context of the task,
    // while others must be done after the task has finished running.
    // So we split up the cleanup in to steps.
    fn preExit(self: *Task) void {
        for (0..num_local_values) |i| {
            const slot = self.local_values[i];
            if (slot.key == null) continue;
            if (slot.dtor) |dtor| dtor(slot.value);
            self.local_values = undefined;
        }
    }

    fn postExit(self: *Task, is_abort: bool) void {
        if (is_abort) self.call_stack.abort() else self.call_stack.finish();
        self.local_result.deinit();
        self.stack.updateFromContext(self.context);
    }

    pub fn setLocal(self: *Task, local: LocalValue) void {
        const hash = std.hash.int(@intFromPtr(local.key));
        for (0..num_local_values) |i| {
            const slot_idx = (hash + i) & (num_local_values - 1);
            if (self.local_values[slot_idx].key != null) continue;
            self.local_values[slot_idx] = local;
            return;
        }

        @panic("local value slots exhausted");
    }

    pub fn getLocal(self: *Task, key: *const anyopaque) ?*anyopaque {
        const hash = std.hash.int(@intFromPtr(key));
        for (0..num_local_values) |i| {
            const slot_idx = (hash + i) & (num_local_values - 1);
            if (self.local_values[slot_idx].key != key) continue;
            return self.local_values[slot_idx].value;
        }

        return null;
    }

    pub fn clearLocal(self: *Task, key: *const anyopaque) void {
        const hash = std.hash.int(@intFromPtr(key));
        for (0..num_local_values) |i| {
            const slot_idx = (hash + i) & (num_local_values - 1);
            if (self.local_values[slot_idx].key != key) continue;
            const slot = self.local_values[slot_idx];
            self.local_values[slot_idx] = .{};
            if (slot.dtor) |dtor| dtor(slot.value);
            return;
        }
    }
};

pub const CmdBuf = struct {
    const State = packed struct(u8) {
        has_waiter: bool = false,
        completed: bool = false,
        _: u6 = 0,
    };

    const Fence = extern struct {
        state: atomic.Value(u8) = .init(unsignaled),

        pub const unsignaled: u8 = 0b00;
        pub const signaled: u8 = 0b01;
        pub const contended: u8 = 0b10;

        fn wait(self: *Fence, futex: *Futex) void {
            while (true) {
                const state = self.state.load(.monotonic);
                if (state & signaled != 0)
                    if (self.state.load(.acquire) & signaled != 0) return;

                if (state & contended == 0) {
                    if (self.state.cmpxchgWeak(
                        unsignaled,
                        contended,
                        .monotonic,
                        .monotonic,
                    )) |_| continue;
                }
                futex.wait(&self.state, @sizeOf(u8), contended, 0, null) catch {};
            }
        }

        fn signal(self: *Fence, futex: *Futex) void {
            const state = self.state.swap(signaled, .release);
            if (state & contended != 0) _ = futex.wake(&self.state, 1);
        }
    };

    owner: *Executor,
    msg: Message,
    enqueued: bool = false,
    node: DoublyLinkedList.Node = .{},
    join_fence: ?*Fence = null,
    cmd_buf: *fimo_tasks_meta.CmdBuf,
    dropped: atomic.Value(bool) = .init(false),
    state: atomic.Value(State) = .init(.{}),

    cmd_idx: usize = 0,
    sub_cmd_idx: usize = 0,
    active_worker: ?*Worker = null,
    spawn_list: DoublyLinkedList = .{},
    cancel_requested: atomic.Value(bool) align(atomic.cache_line) = .init(false),

    fn finish(self: *CmdBuf, futex: *Futex) bool {
        const orig = self.state.swap(.{ .has_waiter = false, .completed = true }, .release);
        debug.assert(!orig.completed);
        if (orig.has_waiter) _ = futex.wake(&self.state, 1);
        if (self.dropped.swap(true, .release)) {
            _ = self.dropped.load(.acquire);
            return true;
        }
        return false;
    }

    pub fn join(self: *CmdBuf, futex: *Futex) fimo_tasks_meta.CmdBufHandle.CompletionStatus {
        return self.joinInner(futex, false);
    }

    pub fn cancel(self: *CmdBuf, futex: *Futex) void {
        _ = self.joinInner(futex, true);
    }

    fn joinInner(
        self: *CmdBuf,
        futex: *Futex,
        request_cancel: bool,
    ) fimo_tasks_meta.CmdBufHandle.CompletionStatus {
        if (request_cancel) _ = self.cancel_requested.store(true, .monotonic);
        var orig = self.state.load(.monotonic);
        while (true) {
            if (orig.completed) {
                _ = self.state.load(.acquire);
                break;
            }
            const new_state: State = .{
                .has_waiter = true,
                .completed = false,
            };
            if (!orig.has_waiter) {
                if (self.state.cmpxchgWeak(orig, new_state, .monotonic, .monotonic)) |v| {
                    orig = v;
                    continue;
                }
            }
            const expect: u8 = @bitCast(new_state);
            futex.wait(&self.state, @sizeOf(State), expect, 0, null) catch {};
        }
        const cancelled = self.cancel_requested.load(.monotonic);
        if (self.dropped.swap(true, .release)) {
            _ = self.dropped.load(.acquire);

            // NOTE(gabriel):
            //
            // The join must complete only after the comand buffer is destroyed.
            var fence: Fence = .{};
            self.join_fence = &fence;
            self.msg = .{ .tag = .dealloc_cmd_buf };
            self.owner.msg_queue.push(futex, &self.msg);
            fence.wait(futex);
        }
        if (cancelled) return .cancelled;
        return .completed;
    }

    pub fn detach(self: *CmdBuf, futex: *Futex) void {
        self.detachInner(futex, false);
    }

    pub fn cancelDetach(self: *CmdBuf, futex: *Futex) void {
        self.detachInner(futex, true);
    }

    fn detachInner(self: *CmdBuf, futex: *Futex, request_cancel: bool) void {
        if (request_cancel) _ = self.cancel_requested.store(true, .monotonic);
        var orig = self.state.load(.monotonic);
        while (true) {
            if (orig.completed) break;
            const new_state: State = .{
                .has_waiter = false,
                .completed = false,
            };
            if (self.state.cmpxchgWeak(orig, new_state, .monotonic, .monotonic)) |v| {
                orig = v;
                continue;
            }
            break;
        }
        if (self.dropped.swap(true, .release)) {
            _ = self.dropped.load(.acquire);
            self.msg = .{ .tag = .dealloc_cmd_buf };
            self.owner.msg_queue.push(futex, &self.msg);
        }
    }
};

pub const Worker = struct {
    threadlocal var _current: ?*Worker = null;

    executor: *Executor,
    ev_thread: Thread,
    seed: usize,
    id: fimo_tasks_meta.Worker,
    local_queue: LocalQueue = .{},
    task: ?*Task = null,
    num_tasks: usize = 0,
    context: Context = undefined,
    call_stack: *CallStack = undefined,

    pub fn currentExecutor() ?*Executor {
        const current = _current orelse return null;
        return current.executor;
    }

    pub fn currentExecutorIfInTask() ?*Executor {
        const current = _current orelse return null;
        if (current.task == null) return null;
        return current.executor;
    }

    pub fn currentTask() ?*Task {
        const current = _current orelse return null;
        return current.task;
    }

    pub fn currentId() ?fimo_tasks_meta.Worker {
        const current = _current orelse return null;
        return current.id;
    }

    pub fn yield() void {
        const worker = _current orelse {
            Thread.yield() catch {};
            return;
        };
        if (worker.task == null) {
            Thread.yield() catch {};
            return;
        }
        worker.toScheduler(.yield);
    }

    fn complete(worker: *Worker, task: *Task) noreturn {
        debug.assert(worker.task == task);
        task.preExit();
        worker.toScheduler(.complete);
        unreachable;
    }

    pub fn abort() noreturn {
        const worker = _current orelse unreachable;
        const task = worker.task orelse unreachable;
        task.preExit();
        worker.toScheduler(.abort);
        unreachable;
    }

    pub fn cancelRequested() bool {
        const worker = _current orelse return false;
        const task = worker.task orelse return false;
        return task.owner.cancel_requested.load(.monotonic);
    }

    pub fn sleep(duration: Duration) void {
        const worker = _current orelse {
            const nanos: usize = @truncate(@min(std.math.maxInt(usize), duration.nanos()));
            Thread.sleep(nanos);
            return;
        };
        const task = worker.task orelse {
            const nanos: usize = @truncate(@min(std.math.maxInt(usize), duration.nanos()));
            Thread.sleep(nanos);
            return;
        };
        task.timeout.timeout = Instant.now().addSaturating(duration);
        worker.toScheduler(.sleep);
    }

    pub fn wait(ptr: *const atomic.Value(u32), expect: u32) void {
        timedWait(ptr, expect, .Max) catch unreachable;
    }

    pub fn timedWait(
        ptr: *const atomic.Value(u32),
        expect: u32,
        timeout: Instant,
    ) error{Timeout}!void {
        const worker = _current orelse unreachable;
        const task = worker.task orelse unreachable;
        var timed_out: bool = undefined;
        task.timeout.timeout = timeout;
        task.wait_state.value = ptr;
        task.wait_state.expect = expect;
        task.wait_state.timed_out = &timed_out;
        worker.toScheduler(.wait);
        if (timed_out) return error.Timeout;
    }

    const WorkerMessage = enum {
        yield,
        complete,
        abort,
        sleep,
        wait,
    };

    fn toScheduler(self: *Worker, msg: WorkerMessage) void {
        debug.assert(self.task != null);
        const context = self.context;

        const tr = context.yieldTo(@intFromPtr(&msg));
        debug.assert(tr.data == 0);
        self.context = tr.context;
    }

    fn fetchTask(self: *Worker, futex: *Futex) ?*Task {
        const num_workers = self.executor.workers.len;
        const num_global_tasks = self.executor.task_count.load(.monotonic);
        const num_local_tasks = self.num_tasks;

        // NOTE(gabriel):
        //
        // We try to have an equal number of tasks on each worker.
        const task = if (num_global_tasks / num_workers > num_local_tasks)
            self.executor.global_channel.pop(&self.local_queue, futex, self.seed, false)
        else
            self.executor.global_channel.pop(&self.local_queue, futex, self.seed, true);
        return task catch null;
    }

    fn start(tr: Transfer) callconv(.c) noreturn {
        debug.assert(tr.data == 0);
        const worker = _current.?;
        worker.context = tr.context;

        const task = worker.task.?;
        {
            const span = tracing.spanTrace(@src());
            defer span.exit();
            task.task.run(task.task, task.batch_idx);
        }
        worker.complete(task);
    }

    fn run(self: *Worker, futex: *Futex) void {
        _current = self;
        defer _current = null;

        tracing.registerThread();
        defer tracing.unregisterThread();

        const span = tracing.spanTraceNamed(@src(), "Worker Event Loop", .{});
        defer span.exit();

        while (self.fetchTask(futex)) |task| {
            debug.assert(!task.enqueued);
            debug.assert(task.msg.tag == .task_to_worker);
            debug.assert(task.worker == null or task.worker == self);
            if (!task.bound_to_worker) {
                task.bound_to_worker = true;
                task.worker = self;
                self.num_tasks += 1;
            }

            debug.assert(self.task == null);
            self.task = task;
            CallStack.suspendCurrent(false);
            self.call_stack = task.call_stack.swapCurrent();
            CallStack.resumeCurrent();

            const old_result: AnyResult = ctx.replaceResult(task.local_result);
            const tr = task.context.yieldTo(0);
            task.local_result = ctx.replaceResult(old_result);
            task.context = tr.context;

            debug.assert(self.task == task);
            self.task = null;

            const msg: *const WorkerMessage = @ptrFromInt(tr.data);
            switch (msg.*) {
                .complete => {
                    CallStack.suspendCurrent(false);
                    task.call_stack = self.call_stack.swapCurrent();
                    CallStack.resumeCurrent();
                    task.postExit(false);
                    task.msg = .{ .tag = .task_complete };
                    debug.assert(!task.enqueued);
                    task.enqueued = true;
                    self.num_tasks -= 1;
                    self.executor.msg_queue.push(futex, &task.msg);
                },
                .abort => {
                    CallStack.suspendCurrent(false);
                    task.call_stack = self.call_stack.swapCurrent();
                    CallStack.resumeCurrent();
                    task.postExit(true);
                    task.msg = .{ .tag = .task_abort };
                    debug.assert(!task.enqueued);
                    task.enqueued = true;
                    self.num_tasks -= 1;
                    self.executor.msg_queue.push(futex, &task.msg);
                },
                .yield => {
                    CallStack.suspendCurrent(false);
                    task.call_stack = self.call_stack.swapCurrent();
                    CallStack.resumeCurrent();
                    task.msg = .{ .tag = .task_to_worker };
                    self.local_queue.push(futex, task);
                },
                .sleep => {
                    // NOTE(gabriel): A timeout behaves like a yield.
                    if (Instant.now().order(task.timeout.timeout) != .lt) {
                        CallStack.suspendCurrent(false);
                        task.call_stack = self.call_stack.swapCurrent();
                        CallStack.resumeCurrent();
                        task.msg = .{ .tag = .task_to_worker };
                        self.local_queue.push(futex, task);
                        continue;
                    }

                    CallStack.suspendCurrent(true);
                    task.call_stack = self.call_stack.swapCurrent();
                    CallStack.resumeCurrent();
                    task.msg = .{ .tag = .task_sleep };
                    debug.assert(!task.enqueued);
                    task.enqueued = true;
                    self.executor.msg_queue.push(futex, &task.msg);
                },
                .wait => {
                    // NOTE(gabriel): A timeout behaves like a yield.
                    if (Instant.now().order(task.timeout.timeout) != .lt) {
                        task.wait_state.timed_out.* = true;
                        CallStack.suspendCurrent(false);
                        task.call_stack = self.call_stack.swapCurrent();
                        CallStack.resumeCurrent();
                        task.msg = .{ .tag = .task_to_worker };
                        self.local_queue.push(futex, task);
                        continue;
                    }

                    // NOTE(gabriel): If the value changed we yield.
                    if (task.wait_state.value.load(.acquire) != task.wait_state.expect) {
                        task.wait_state.timed_out.* = false;
                        CallStack.suspendCurrent(false);
                        task.call_stack = self.call_stack.swapCurrent();
                        CallStack.resumeCurrent();
                        task.msg = .{ .tag = .task_to_worker };
                        self.local_queue.push(futex, task);
                        continue;
                    }

                    CallStack.suspendCurrent(true);
                    task.call_stack = self.call_stack.swapCurrent();
                    CallStack.resumeCurrent();
                    task.msg = .{ .tag = .task_wait };
                    debug.assert(!task.enqueued);
                    task.enqueued = true;
                    self.executor.msg_queue.push(futex, &task.msg);
                },
            }
        }
    }
};

const Executor = @This();

arena: ArenaAllocator,
label: []u8,
ev_thread: Thread,
workers: []Worker,
cmd_bufs: ItemPool(CmdBuf),
task_pool: ItemPool(Task),
wake_msg_pool: ItemPool(WakeMessage),
stack_pool: StackPool,
msg_queue: MessageQueue = .{},
global_channel: GlobalChannel,
timeout_queue: TimeoutQueue = .{},
wait_list: WaitList,
process_queue: DoublyLinkedList = .{},
join_requested: atomic.Value(bool) = .init(false),
task_count: atomic.Value(usize) = .init(0),
cmd_bufs_count: atomic.Value(usize) = .init(0),

pub const InitOptions = struct {
    label: ?[]const u8 = null,
    futex: *Futex,
    allocator: Allocator,
    cmd_buf_capacity: usize,
    worker_count: usize,
    max_load_factor: usize,
    stack_size: usize,
    worker_stack_cache_len: usize,
    disable_stack_cache: bool = false,
};

pub fn init(options: InitOptions) !*Executor {
    debug.assert(options.cmd_buf_capacity != 0);
    debug.assert(options.worker_count != 0);
    debug.assert(options.max_load_factor != 0);
    debug.assert(options.stack_size != 0);
    debug.assert(options.worker_stack_cache_len != 0);

    var arena: ArenaAllocator = .init(options.allocator);
    errdefer arena.deinit();
    const gpa = arena.allocator();

    const num_tasks = options.worker_count * options.max_load_factor;
    const cache_len = if (options.disable_stack_cache)
        0
    else
        options.worker_count * options.worker_stack_cache_len;
    const stack_pool: StackPool = try .init(options.stack_size, num_tasks, cache_len);
    errdefer stack_pool.deinit();

    const exe = try gpa.create(Executor);
    exe.* = .{
        .arena = undefined,
        .label = try gpa.dupe(u8, options.label orelse "<unlabelled>"),
        .ev_thread = undefined,
        .workers = try gpa.alloc(Worker, options.worker_count),
        .cmd_bufs = try .init(gpa, options.cmd_buf_capacity),
        .task_pool = try .init(gpa, num_tasks),
        .wake_msg_pool = try .init(gpa, num_tasks),
        .stack_pool = stack_pool,
        .global_channel = try .init(gpa, num_tasks),
        .wait_list = try .init(gpa, num_tasks),
    };

    var num_spawned: usize = 0;
    errdefer {
        exe.global_channel.close(options.futex);
        for (exe.workers[0..num_spawned]) |w| w.ev_thread.join();
    }
    for (exe.workers, 0..) |*w, i| {
        w.* = .{
            .executor = exe,
            .ev_thread = undefined,
            .seed = std.hash.int(i),
            .id = @enumFromInt(i),
        };
        w.ev_thread = try Thread.spawn(.{ .allocator = gpa }, Worker.run, .{ w, options.futex });
        w.ev_thread.setName("tasks worker") catch {};
        num_spawned = i;
    }

    exe.ev_thread = try Thread.spawn(.{ .allocator = gpa }, run, .{ exe, options.futex });
    exe.ev_thread.setName("executor loop") catch {};
    exe.arena = arena;
    return exe;
}

pub fn join(self: *Executor, futex: *Futex) void {
    const span = tracing.spanTraceNamed(@src(), "Join Executor", .{});
    defer span.exit();

    var msg: Message = .{ .tag = .join };
    self.msg_queue.push(futex, &msg);

    self.ev_thread.join();
    debug.assert(self.msg_queue.pop_list.load(.monotonic) == null);
    debug.assert(self.msg_queue.push_list.load(.monotonic) == 0);
    debug.assert(self.process_queue.len() == 0);
    debug.assert(self.cmd_bufs_count.load(.monotonic) == 0);
    debug.assert(self.task_count.load(.monotonic) == 0);
    debug.assert(self.cmd_bufs_count.load(.monotonic) == 0);

    self.stack_pool.deinit();
    self.arena.deinit();
}

pub fn joinRequested(self: *Executor) bool {
    return self.join_requested.load(.monotonic);
}

pub fn enqueue(self: *Executor, futex: *Futex, cmd_buf: *fimo_tasks_meta.CmdBuf) *CmdBuf {
    debug.assert(!self.joinRequested() or self.cmd_bufs_count.load(.monotonic) != 0);
    _ = self.cmd_bufs_count.fetchAdd(1, .monotonic);
    const buf = self.cmd_bufs.create(futex);
    buf.* = .{
        .owner = self,
        .msg = .{ .tag = .enqueue_cmd_buf },
        .cmd_buf = cmd_buf,
    };
    self.msg_queue.push(futex, &buf.msg);
    return buf;
}

pub fn enqueueDetached(self: *Executor, futex: *Futex, cmd_buf: *fimo_tasks_meta.CmdBuf) void {
    const buf = self.enqueue(futex, cmd_buf);
    buf.detach(futex);
}

pub fn wakeByAddress(self: *Executor, futex: *Futex, ptr: *const atomic.Value(u32)) void {
    // TODO(gabriel):
    //
    // This is _very_ ugly. It may result in recursive futex calls, as `wakeByAddress`
    // is itself called by the futex implementation.
    const msg = self.wake_msg_pool.create(futex);
    msg.* = .{ .ptr = ptr };
    self.msg_queue.push(futex, &msg.msg);
}

fn enqueueTask(self: *Executor, futex: *Futex, task: *Task) void {
    task.msg = .{ .tag = .task_to_worker };
    if (task.worker) |w| {
        w.local_queue.push(futex, task);
    } else {
        self.global_channel.push(futex, task);
    }
}

fn run(self: *Executor, futex: *Futex) void {
    tracing.registerThread();
    defer tracing.unregisterThread();

    const span = tracing.spanTraceNamed(@src(), "Executor Event Loop", .{});
    defer span.exit();

    while (true) {
        const timeout = self.timeout_queue.getNextTimeout();
        var opt_msg: ?*Message = self.msg_queue.pop(futex, timeout);
        while (opt_msg) |msg| : (opt_msg = self.msg_queue.tryPop()) {
            switch (msg.tag) {
                .task_to_worker => unreachable,
                .dealloc_cmd_buf => {
                    const cmd_buf: *CmdBuf = @alignCast(@fieldParentPtr("msg", msg));
                    if (cmd_buf.cmd_buf.deinit) |f| f(cmd_buf.cmd_buf);
                    const join_fence = cmd_buf.join_fence;
                    self.cmd_bufs.destroy(futex, cmd_buf);
                    _ = self.cmd_bufs_count.fetchSub(1, .monotonic);
                    if (join_fence) |f| f.signal(futex);
                },
                .enqueue_cmd_buf => {
                    const cmd_buf: *CmdBuf = @alignCast(@fieldParentPtr("msg", msg));
                    debug.assert(!cmd_buf.enqueued);
                    cmd_buf.enqueued = true;
                    self.process_queue.append(&cmd_buf.node);
                },
                .wake => {
                    const wait_msg: *WakeMessage = @alignCast(@fieldParentPtr("msg", msg));
                    if (self.wait_list.pop(wait_msg.ptr)) |task| {
                        if (task.timeout.timeout.order(.Max) != .eq) {
                            self.timeout_queue.removeTask(task);
                        }
                        task.wait_state.timed_out.* = false;
                        task.call_stack.unblock();
                        self.enqueueTask(futex, task);
                    }
                    self.wake_msg_pool.destroy(futex, wait_msg);
                },
                .task_complete, .task_abort => {
                    const task: *Task = @alignCast(@fieldParentPtr("msg", msg));
                    debug.assert(task.enqueued);
                    task.enqueued = false;
                    const cmd_buf = task.owner;
                    cmd_buf.spawn_list.remove(&task.spawn_list_node);
                    if (msg.tag == .task_abort)
                        cmd_buf.cancel_requested.store(true, .monotonic);

                    if (self.stack_pool.push(task.stack)) |waiter| {
                        debug.assert(!waiter.enqueued);
                        waiter.enqueued = true;
                        self.process_queue.append(&waiter.node);
                    }
                    self.task_pool.destroy(futex, task);
                    _ = self.task_count.fetchSub(1, .monotonic);

                    if (!cmd_buf.enqueued) {
                        cmd_buf.enqueued = true;
                        self.process_queue.append(&cmd_buf.node);
                    }
                },
                .task_sleep => {
                    const task: *Task = @alignCast(@fieldParentPtr("msg", msg));
                    debug.assert(task.enqueued);
                    task.enqueued = false;
                    self.timeout_queue.push(task);
                },
                .task_wait => {
                    const task: *Task = @alignCast(@fieldParentPtr("msg", msg));
                    debug.assert(task.enqueued);
                    task.enqueued = false;
                    if (task.wait_state.value.load(.acquire) != task.wait_state.expect) {
                        task.wait_state.timed_out.* = false;
                        task.call_stack.unblock();
                        self.enqueueTask(futex, task);
                        continue;
                    }
                    if (Instant.now().order(task.timeout.timeout) != .lt) {
                        task.wait_state.timed_out.* = true;
                        task.call_stack.unblock();
                        self.enqueueTask(futex, task);
                        continue;
                    }
                    self.wait_list.push(task);
                    if (task.timeout.timeout.order(.Max) != .eq)
                        self.timeout_queue.push(task);
                },
                .join => self.join_requested.store(true, .monotonic),
            }
        }

        const now: Instant = .now();
        while (self.timeout_queue.pop(now)) |task| {
            if (task.msg.tag == .task_wait) {
                _ = self.wait_list.pop(task.wait_state.value).?;
                task.wait_state.timed_out.* = true;
            }
            task.call_stack.unblock();
            self.enqueueTask(futex, task);
        }

        next_buf: while (self.process_queue.popFirst()) |node| {
            const cmd_buf: *CmdBuf = @alignCast(@fieldParentPtr("node", node));
            cmd_buf.enqueued = false;
            const cmds = cmd_buf.cmd_buf.cmds.get().?;
            while (cmd_buf.cmd_idx < cmds.len) : (cmd_buf.cmd_idx += 1) {
                const cmd = cmds[cmd_buf.cmd_idx];
                switch (cmd.tag) {
                    .select_worker => {
                        const worker_idx = @intFromEnum(cmd.payload.select_worker);
                        cmd_buf.active_worker = &self.workers[worker_idx];
                    },
                    .select_any_worker => cmd_buf.active_worker = null,
                    .enqueue_task => {
                        const task = cmd.payload.enqueue_task;
                        while (cmd_buf.sub_cmd_idx < task.batch_len) : (cmd_buf.sub_cmd_idx += 1) {
                            const stack = self.stack_pool.pop(cmd_buf) orelse continue :next_buf;
                            const alloc = self.task_pool.create(futex);
                            alloc.* = .{
                                .owner = cmd_buf,
                                .cmd_idx = cmd_buf.cmd_idx,
                                .batch_idx = cmd_buf.sub_cmd_idx,
                                .task = task,
                                .stack = stack,
                                .worker = cmd_buf.active_worker,
                                .call_stack = .init(),
                                .context = .init(.forStack(stack), Worker.start),
                            };
                            _ = self.task_count.fetchAdd(1, .monotonic);
                            cmd_buf.spawn_list.append(&alloc.spawn_list_node);
                            self.enqueueTask(futex, alloc);
                        }
                        cmd_buf.sub_cmd_idx = 0;
                    },
                    .wait_on_barrier => if (cmd_buf.spawn_list.first != null) continue :next_buf,
                    .wait_on_cmd_indirect => {
                        const wait_on_idx = cmd_buf.cmd_idx - cmd.payload.wait_on_cmd_indirect;
                        var current = cmd_buf.spawn_list.first;
                        while (current) |curr| {
                            const task: *Task = @alignCast(@fieldParentPtr("spawn_list_node", curr));
                            if (task.cmd_idx == wait_on_idx) continue :next_buf;
                            current = curr.next;
                        }
                    },
                    _ => unreachable,
                }
            }

            // NOTE(gabriel): Now that we processed all commands we wait until they complete.
            if (cmd_buf.spawn_list.first != null) continue;
            if (cmd_buf.cmd_buf.deinit) |f| f(cmd_buf.cmd_buf);
            if (cmd_buf.finish(futex)) {
                self.cmd_bufs.destroy(futex, cmd_buf);
                _ = self.cmd_bufs_count.fetchSub(1, .monotonic);
            }
        }

        if (self.join_requested.load(.monotonic) and
            self.cmd_bufs_count.load(.monotonic) == 0) break;
    }

    debug.assert(self.join_requested.load(.monotonic));
    debug.assert(self.task_count.load(.monotonic) == 0);
    debug.assert(self.cmd_bufs_count.load(.monotonic) == 0);
    self.global_channel.close(futex);

    for (self.workers) |*w| {
        w.ev_thread.join();
        debug.assert(w.num_tasks == 0);
    }
}
