const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const Futex = std.Thread.Futex;

// Based on https://www.boost.org/doc/libs/1_87_0/libs/atomic/doc/html/atomic/usage_examples.html
pub fn Fifo(T: type) type {
    if (!@hasField(T, "next") or @FieldType(T, "next") != ?*T)
        @compileError("expected an element with a `next` field");
    std.debug.assert(@alignOf(T) > 2);

    return struct {
        push_list: atomic.Value(usize) align(atomic.cache_line) = .init(0),
        pop_list: atomic.Value(?*T) align(atomic.cache_line) = .init(null),
        futex_version: atomic.Value(u32) align(atomic.cache_line) = .init(0),

        const Self = @This();

        const closed_bit_position = 0;
        const waiting_bit_position = 1;

        const closed_bit: usize = 0b01;
        const waiting_bit: usize = 0b10;

        const flags_mask: usize = 0b11;
        const list_mask: usize = ~flags_mask;

        // An empty queue.
        pub const empty: Self = .{};

        fn wait(self: *Self) enum { closed, retry } {
            @branchHint(.cold);

            _ = self.push_list.bitSet(waiting_bit_position, .seq_cst);
            defer _ = self.push_list.bitReset(waiting_bit_position, .release);

            var spin_count: usize = 0;
            const spin_relax_limit = 12;
            const spin_limit = spin_relax_limit + 4;
            var state = self.push_list.load(.monotonic);
            while (true) {
                // If the list has some element we retry.
                if (state & list_mask != 0) return .retry;

                // If the channel is closed we abort, as no new elements will be inserted.
                if (state & closed_bit != 0) return .closed;

                // Try spinning a couple of times.
                if (spin_count < spin_limit) {
                    if (spin_count < spin_relax_limit)
                        atomic.spinLoopHint()
                    else
                        std.Thread.yield() catch unreachable;
                    spin_count += 1;
                    state = self.push_list.load(.monotonic);
                    continue;
                }

                // If the channel is still empty, we park the current thread.
                {
                    const expected = self.futex_version.load(.monotonic);
                    Futex.wait(&self.futex_version, expected);
                }

                // Retry another round.
                spin_count = 0;
                state = self.push_list.load(.monotonic);
            }
        }

        fn signal(self: *Self) void {
            // Check if there are any waiters.
            if (self.push_list.load(.seq_cst) & waiting_bit == 0) return;
            _ = self.futex_version.fetchAdd(1, .seq_cst);
            Futex.wake(&self.futex_version, 1);
        }

        fn broadcast(self: *Self) void {
            // Check if there are any waiters.
            if (self.push_list.load(.seq_cst) & waiting_bit == 0) return;
            _ = self.futex_version.fetchAdd(1, .seq_cst);
            Futex.wake(&self.futex_version, std.math.maxInt(u32));
        }

        fn close(self: *Self) void {
            const state = self.push_list.load(.monotonic);

            // If the channel is already closed we can skip waking up the consumers.
            if (state & closed_bit != 0) return;

            // Mark the channel as closed and wake all consumers.
            _ = self.push_list.bitSet(closed_bit_position, .release);
            self.broadcast();
        }

        fn tryRecv(self: *Self) ?*T {
            // Try to take one element from the pop list.
            if (self.pop_list.load(.acquire)) |elem| {
                self.pop_list.store(elem.next, .release);
                elem.next = null;
                return elem;
            }

            // If the pop list is empty continue with the push list.
            var state = self.push_list.load(.monotonic);
            while (true) {
                // Clear the list, leaving only the flag bits.
                const new_state = state & flags_mask;
                if (self.push_list.cmpxchgWeak(state, new_state, .acquire, .monotonic)) |new| {
                    state = new;
                } else break;
            }

            // Reverse the push list and set it as the pop list.
            var head: *T = @as(?*T, @ptrFromInt(state & list_mask)) orelse return null;
            var pop_list_head: ?*T = null;
            while (head.next) |next| {
                head.next = pop_list_head;
                pop_list_head = head;
                head = next;
            }
            self.pop_list.store(pop_list_head, .release);

            return head;
        }

        fn recv(self: *Self) ?*T {
            while (true) {
                if (self.tryRecv()) |elem| return elem;
                if (self.wait() == .closed) return null;
            }
        }

        fn send(self: *Self, value: *T) bool {
            while (true) {
                const state = self.push_list.load(.monotonic);

                // If the queue is closed we are not allowed to send any messages.
                if (state & closed_bit != 0) return false;

                // Try to swap to the new head.
                const head: ?*T = @ptrFromInt(state & list_mask);
                value.next = head;
                if (self.push_list.cmpxchgWeak(
                    state,
                    @intFromPtr(value) | (state & waiting_bit),
                    .release,
                    .monotonic,
                ) != null) continue;

                // If the head was null there may be one waiter.
                if (head == null) self.signal();
                return true;
            }
        }
    };
}

test "smoke test" {
    const Node = struct {
        next: ?*@This() = null,
        data: u32,
    };

    var channel = Fifo(Node).empty;

    var node = Node{ .data = 15 };
    try std.testing.expect(channel.tryRecv() == null);

    try std.testing.expect(channel.send(&node));
    try std.testing.expectEqual(&node, channel.recv());
    try std.testing.expect(channel.tryRecv() == null);
}

test "multiple producers" {
    const Node = struct {
        next: ?*@This() = null,
        received: bool = false,
    };

    var channel = Fifo(Node).empty;

    const num_threads = 4;
    const num_messages = 1000;

    const nodes = try std.testing.allocator.alloc(Node, num_messages);
    defer std.testing.allocator.free(nodes);

    var node_counter = atomic.Value(usize).init(0);
    var finished_counter = atomic.Value(usize).init(0);

    const Runner = struct {
        thread: std.Thread = undefined,
        channel: *Fifo(Node),
        nodes: []Node,
        node_counter: *atomic.Value(usize),
        finished_counter: *atomic.Value(usize),

        fn run(self: *@This()) void {
            while (true) {
                const idx = self.node_counter.fetchAdd(1, .monotonic);
                if (idx >= self.nodes.len) {
                    const finished = self.finished_counter.fetchAdd(1, .acquire);
                    if (finished == num_threads - 1) self.channel.close();
                    return;
                }

                const node = &self.nodes[idx];
                const sent = self.channel.send(node);
                std.debug.assert(sent);
            }

            while (self.channel.recv()) |v| {
                self.value_map[v].store(true, .monotonic);
            }
        }
    };

    var runners = [_]Runner{.{
        .channel = &channel,
        .nodes = nodes,
        .node_counter = &node_counter,
        .finished_counter = &finished_counter,
    }} ** num_threads;
    for (&runners) |*r| r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});

    while (channel.recv()) |elem| elem.received = true;
    for (runners) |r| r.thread.join();
    for (nodes) |n| try std.testing.expectEqual(true, n.received);
}
