const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const Futex = std.Thread.Futex;
const Random = std.Random;

/// A single producer, multiple consumers unbounded channel.
///
/// The channel is unordered, meaning that the insertion order is not guaranteed to be preserved
/// by the consumers of the channel.
pub fn Channel(T: type) type {
    return struct {
        active_channel: atomic.Value(usize),

        const Self = @This();
        const Inner = BoundedChannel(T);

        /// Error of a `trySend` operation.
        pub const TrySendError = error{
            /// The channel is closed.
            Closed,
            /// The channel is full.
            Full,
        };

        /// Error of a `send` operation.
        pub const SendError = error{
            /// The channel is closed.
            Closed,
        } || Allocator.Error;

        // Use the first bit of the pointer to determine if the channel is closed.
        const closed_bit: usize = 1;
        const channel_mask: usize = ~closed_bit;

        /// Initializes a new channel with a capacity of 1.
        pub fn init(allocator: Allocator) Allocator.Error!Self {
            return Self.initWithCapacity(allocator, 1);
        }

        /// Initializes a new channel with a custom capacity.
        ///
        /// The capacity will be rounded up to the next power of two.
        pub fn initWithCapacity(allocator: Allocator, capacity: usize) Allocator.Error!Self {
            const channel = try allocator.create(Inner);
            errdefer allocator.destroy(channel);
            channel.* = try Inner.init(allocator, capacity);

            return Self{ .active_channel = .init(@intFromPtr(channel)) };
        }

        /// Deallocates the channel.
        ///
        /// The channel is not flushed nor closed prior.
        pub fn deinit(self: *Self, allocator: Allocator) void {
            const state = self.active_channel.load(.acquire);
            const active_channel: *Inner = @ptrFromInt(state & channel_mask);
            active_channel.deinit(allocator);
            allocator.destroy(active_channel);
        }

        /// Closes the channel.
        ///
        /// All waiting consumers are woken up.
        pub fn close(self: *Self) void {
            while (true) {
                const state = self.active_channel.load(.acquire);
                if (state & closed_bit != 0) return;

                // Mark the active channel as closed.
                if (self.active_channel.cmpxchgWeak(
                    state,
                    state | closed_bit,
                    .release,
                    .monotonic,
                ) != null) continue;
                const channel: *Inner = @ptrFromInt(state & channel_mask);
                channel.close();
            }
        }

        /// Tries to receive one element from the channel without blocking.
        pub fn tryRecv(self: *Self) ?T {
            return self.tryRecvWithSeed(0);
        }

        /// Tries to receive one element from the channel without blocking.
        ///
        /// The `seed` can be used to indicate a preferred slot in the channel.
        pub fn tryRecvWithSeed(self: *Self, seed: usize) ?T {
            while (true) {
                const state = self.active_channel.load(.acquire);

                // First try pulling an element from the active channel.
                const active_channel: *Inner = @ptrFromInt(state & channel_mask);
                if (active_channel.tryRecvWithSeed(seed)) |elem| return elem;

                // If the channel is closed it must be the active one so we abort.
                if (state & closed_bit != 0) return null;

                // If the channel is still active, we were not able to pull an element.
                if (self.active_channel.load(.monotonic) & channel_mask == state & channel_mask) return null;
            }
        }

        /// Tries to receive one element from the channel.
        ///
        /// The caller will block until it is able to reveive the element or until the channel is
        /// closed. In case the channel is closed, the function returns `null`.
        pub fn recv(self: *Self) ?T {
            return self.recvWithSeed(0);
        }

        /// Tries to receive one element from the channel.
        ///
        /// The caller will block until it is able to reveive the element or until the channel is
        /// closed. In case the channel is closed, the function returns `null`. The `seed` can be
        /// used to indicate a preferred slot in the channel.
        pub fn recvWithSeed(self: *Self, seed: usize) ?T {
            while (true) {
                const state = self.active_channel.load(.acquire);

                // First try pulling an element from the active channel.
                const active_channel: *Inner = @ptrFromInt(state & channel_mask);
                if (active_channel.recvWithSeed(seed)) |elem| return elem;

                // If the channel is closed it must be the active one so we abort.
                if (state & closed_bit != 0) return null;
            }
        }

        /// Tries to send an element through the channel without growing its capacity.
        pub fn trySend(self: *Self, value: T) TrySendError!void {
            try self.trySendWithSeed(value, 0);
        }

        /// Tries to send an element through the channel without growing its capacity.
        ///
        /// The `seed` can be used to indicate a preferred slot in the channel.
        pub fn trySendWithSeed(self: *Self, value: T, seed: usize) TrySendError!void {
            const state = self.active_channel.load(.acquire);

            // If the channel is already closed we abort and report a failure.
            if (state & closed_bit != 0) return TrySendError.Closed;

            // Try sending a value into the channel. This may only fail if it is full.
            const channel: *Inner = @ptrFromInt(state & channel_mask);
            std.debug.assert(!channel.isClosed());
            if (!channel.trySendWithSeed(value, seed)) return TrySendError.Full;
        }

        pub fn send(self: *Self, allocator: Allocator, value: T) SendError!void {
            return self.sendWithSeed(allocator, value, 0);
        }

        /// Tries to send an element through the channel.
        ///
        /// If the channel is not guaranteed to have enough space to store the element, this will
        /// try to reallocate the internal buffers to the next power of two. The `seed` can be used
        /// to indicate a preferred slot in the channel.
        pub fn sendWithSeed(
            self: *Self,
            allocator: Allocator,
            value: T,
            seed: usize,
        ) SendError!void {
            const state = self.active_channel.load(.acquire);

            // If the channel is already closed we abort and report a failure.
            if (state & closed_bit != 0) return SendError.Closed;

            // Try sending a value into the channel. This may only fail if it is full.
            const channel: *Inner = @ptrFromInt(state & channel_mask);
            std.debug.assert(!channel.isClosed());
            if (channel.trySendWithSeed(value, seed)) return;

            // Since the channel is full, we create a new channel and migrate all elements.
            const new_channel = try allocator.create(Inner);
            errdefer allocator.destroy(new_channel);

            // The capacity is automatically increased to the next power of two.
            new_channel.* = try Inner.initChained(allocator, channel.capacity() + 1, channel);

            // Set the new channel as the active one and close the old one.
            // Someone may already have closed the channel, so we check for that.
            if (self.active_channel.cmpxchgStrong(
                state,
                @intFromPtr(new_channel),
                .release,
                .monotonic,
            ) != null) return SendError.Closed;
            channel.close();

            // Migrate all elements to the new channel.
            var default_rng = Random.DefaultPrng.init(@intCast(seed));
            const rng = default_rng.random();
            while (channel.tryRecv()) |elem| {
                const inserted = new_channel.trySendWithSeed(elem, rng.int(usize));
                std.debug.assert(inserted);
            }
            const inserted = new_channel.trySendWithSeed(value, seed);
            std.debug.assert(inserted);

            return;
        }
    };
}

fn BoundedChannel(T: type) type {
    return struct {
        depth: u8 = 0,
        elements: []Element = &.{},
        counters: []align(atomic.cache_line) atomic.Value(usize) = &.{},
        waiter_count: atomic.Value(u32) align(atomic.cache_line) = .init(0),
        futex_version: atomic.Value(u32) align(atomic.cache_line) = .init(0),
        dummy_counter: atomic.Value(usize) align(atomic.cache_line) = .init(0),
        prev: ?*Self = null,

        const Self = @This();
        const empty: Self = .{};

        // The implementation uses a sum tree to identify free slots.
        // The number of nodes in a perfect binary tree is `2^depth - 1`, with `2^(depth-1)` leaf
        // nodes. Assuming 64bit `usize`, we are bounded to `2^64 - 1` nodes and `2^63` leaf nodes.
        // This leaves the MSB unused , which we employ the determine if the channel is closed.
        const is_closed_bit_position = @bitSizeOf(usize) - 1;
        const is_closed_bit: usize = 1 << is_closed_bit_position;
        const counter_mask: usize = ~is_closed_bit;

        const Element = struct {
            value: T = undefined,
            filled: atomic.Value(bool) align(atomic.cache_line) = .init(false),
        };

        fn init(allocator: Allocator, n: usize) Allocator.Error!Self {
            return Self.initChained(allocator, n, null);
        }

        fn initChained(allocator: Allocator, n: usize, prev: ?*Self) Allocator.Error!Self {
            if (n == 0) return Self{};
            const n_ceil = std.math.ceilPowerOfTwo(
                usize,
                n,
            ) catch return Allocator.Error.OutOfMemory;
            const depth = std.math.log2(n_ceil) + 1;

            const elements = try allocator.alloc(Element, n_ceil);
            errdefer allocator.free(elements);
            for (elements) |*element| {
                element.* = .{};
            }

            const num_counters = (n_ceil * 2) - 1;
            const counters = try allocator.alignedAlloc(
                atomic.Value(usize),
                atomic.cache_line,
                num_counters,
            );
            for (counters) |*counter| {
                counter.* = .init(0);
            }

            return Self{
                .depth = @truncate(depth),
                .elements = elements,
                .counters = counters,
                .prev = prev,
            };
        }

        fn deinit(self: *Self, allocator: Allocator) void {
            var current: ?*Self = self;
            while (current) |curr| {
                allocator.free(curr.elements);
                allocator.free(curr.counters);
                current = curr.prev;
                if (curr != self) allocator.destroy(curr);
            }
            self.* = .{};
        }

        fn rootNode(self: *Self) *align(atomic.cache_line) atomic.Value(usize) {
            return if (self.elements.len == 0)
                &self.dummy_counter
            else
                &self.counters[0];
        }

        fn wait(self: *Self) enum { closed, retry } {
            @branchHint(.cold);

            _ = self.waiter_count.fetchAdd(1, .seq_cst);
            defer _ = self.waiter_count.fetchSub(1, .release);

            var spin_count: usize = 0;
            const spin_relax_limit = 12;
            const spin_limit = spin_relax_limit + 4;
            const counter = self.rootNode();
            var state = counter.load(.monotonic);
            while (true) {
                // If the channel has some element we retry.
                if (state & counter_mask != 0) return .retry;

                // If the channel is closed we abort, as no new elements will be inserted.
                if (state & is_closed_bit != 0) return .closed;

                // Try spinning a couple of times.
                if (spin_count < spin_limit) {
                    if (spin_count < spin_relax_limit)
                        atomic.spinLoopHint()
                    else
                        std.Thread.yield() catch unreachable;
                    spin_count += 1;
                    state = counter.load(.monotonic);
                    continue;
                }

                // If the channel is still empty, we park the current thread.
                {
                    const expected = self.futex_version.load(.monotonic);
                    Futex.wait(&self.futex_version, expected);
                }

                // Retry another round.
                spin_count = 0;
                state = counter.load(.monotonic);
            }
        }

        fn signal(self: *Self) void {
            // Check if there are any waiters.
            if (self.waiter_count.load(.seq_cst) == 0) return;
            _ = self.futex_version.fetchAdd(1, .seq_cst);
            Futex.wake(&self.futex_version, 1);
        }

        fn broadcast(self: *Self) void {
            // Check if there are any waiters.
            if (self.waiter_count.load(.seq_cst) == 0) return;
            _ = self.futex_version.fetchAdd(1, .seq_cst);
            Futex.wake(&self.futex_version, std.math.maxInt(u32));
        }

        fn capacity(self: *Self) usize {
            return self.elements.len;
        }

        fn isClosed(self: *Self) bool {
            const root = self.rootNode();
            const root_state = root.load(.monotonic);
            return root_state & is_closed_bit != 0;
        }

        fn close(self: *Self) void {
            const root = self.rootNode();
            const root_state = root.load(.acquire);

            // If the channel is already closed we can skip waking up the consumers.
            if (root_state & is_closed_bit != 0) return;

            // Mark the channel as closed and wake all consumers.
            _ = root.bitSet(is_closed_bit_position, .release);
            self.broadcast();
        }

        fn tryRecv(self: *Self) ?T {
            return self.tryRecvWithSeed(0);
        }

        fn tryRecvWithSeed(self: *Self, seed: usize) ?T {
            // Enter a cas loop to decrement the counter of the root node.
            const root = self.rootNode();
            var state = root.load(.monotonic);
            while (true) {
                // If the channel is empty we abort.
                if (state & counter_mask == 0) return null;

                // Try decrementing the counter by one.
                if (root.cmpxchgWeak(state, state - 1, .acquire, .monotonic)) |new| {
                    state = new;
                    continue;
                }

                // Now that we decremented the counter we have acquired one slot from the channel.
                break;
            }

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

                    // The LSB determines if we prefer the left or right child node.
                    const first_idx, const second_idx = if (search_order & 1 == 0)
                        .{ left_idx, right_idx }
                    else
                        .{ right_idx, left_idx };
                    const first_node = &self.counters[first_idx];
                    const second_node = &self.counters[second_idx];

                    // Try to decrement the count at the first or second node by one.
                    blk: while (true) {
                        // If the preferred node is not empty, we always choose that one.
                        const first_count = first_node.load(.monotonic);
                        if (first_count != 0) {
                            if (first_node.cmpxchgWeak(
                                first_count,
                                first_count - 1,
                                .acquire,
                                .monotonic,
                            ) != null) continue :blk;
                            elem_idx = (elem_idx << 1) | @intFromBool(first_idx == right_idx);
                            break;
                        }

                        // Now that the first node is empty, we are forced to choose the second one.
                        // We can not preform an unconditional decrement, as it is possible that, in
                        // the time between the decrement of the parent node and that of the child node,
                        // the producer inserts another element into the channel. This may lead to the
                        // parent node and the first node being incremented by one. Then another consumer
                        // may consume the element accessed through the second node.
                        const second_count = second_node.load(.monotonic);
                        if (second_count != 0) {
                            if (second_node.cmpxchgWeak(
                                second_count,
                                second_count - 1,
                                .acquire,
                                .monotonic,
                            ) != null) continue :blk;
                            elem_idx = (elem_idx << 1) | @intFromBool(second_idx == right_idx);
                            break;
                        }
                    }

                    // Shift the order by one to the right to discard the LSB for the current layer.
                    search_order >>= 1;
                }
            }

            // With the element index we can take the value from the slot and notify that it is empty.
            const slot = &self.elements[elem_idx];
            const value = slot.value;
            slot.filled.store(false, .release);

            return value;
        }

        fn recv(self: *Self) ?T {
            return self.recvWithSeed(0);
        }

        fn recvWithSeed(self: *Self, seed: usize) ?T {
            while (true) {
                if (self.tryRecvWithSeed(seed)) |elem| return elem;
                if (self.wait() == .closed) return null;
            }
        }

        fn trySend(self: *Self, value: T) bool {
            // Always prefer the left child.
            self.trySendWithSeed(value, 0);
        }

        fn trySendWithSeed(self: *Self, value: T, seed: usize) bool {
            // Check if the tree is full or closed.
            const root = self.rootNode();
            const root_state = root.load(.monotonic);
            const is_closed = root_state & is_closed_bit != 0;
            const is_full = root_state & counter_mask == self.capacity();
            if (is_closed or is_full) return false;

            // Compute the index of the next free element by traversing the tree.
            var elem_idx: usize = 0;
            var search_order: usize = seed;
            var max_elements: usize = self.capacity() >> 1;
            if (self.depth > 1) {
                for (1..self.depth - 1) |layer_idx| {
                    const layer_start: usize = (@as(usize, 1) << @truncate(layer_idx)) - 1;
                    const left_idx = layer_start + elem_idx;
                    const right_idx = left_idx + 1;

                    // The LSB determines if we prefer the left or right child node.
                    const first_idx, const second_index = if (search_order & 1 == 0)
                        .{ left_idx, right_idx }
                    else
                        .{ right_idx, left_idx };

                    const first_count = self.counters[first_idx].load(.monotonic);
                    if (first_count == max_elements) {
                        // First child is already full, continue with the second.
                        elem_idx = (elem_idx << 1) | @intFromBool(second_index == right_idx);
                    } else {
                        // First child has some capacity left, continue there.
                        elem_idx = (elem_idx << 1) | @intFromBool(first_idx == right_idx);
                    }

                    search_order >>= 1;
                    max_elements >>= 1;
                }
            }

            // Write the value at the empty node position.
            var element = &self.elements[elem_idx];

            // A consumer may not be finished reading it yet, so we wait until it is consumed.
            // We know that some thread is in the process of consuming it, as the counter indicated
            // an empty slot.
            const spin_count_relax = 12;
            var spin_count: usize = 0;
            while (element.filled.load(.acquire)) {
                if (spin_count < spin_count_relax)
                    atomic.spinLoopHint()
                else
                    std.Thread.yield() catch unreachable;
                spin_count += 1;
            }

            // Now that the slot is truly empty, we can fill it.
            element.value = value;
            element.filled.store(true, .release);

            // Traverse the tree from the leaf to the root and increase the counter.
            var num_elements: usize = 0;
            var counter_index = (@as(usize, 1) << @truncate(self.depth - 1)) + elem_idx - 1;
            for (0..self.depth) |_| {
                const counter = &self.counters[counter_index];
                num_elements = counter.fetchAdd(1, .release);
                counter_index -%= 1;
                counter_index >>= 1;
            }

            // If the counter was empty we try to wake one consumer.
            if (num_elements == 0) self.signal();
            return true;
        }
    };
}

test "smoke test" {
    var channel = try Channel(u32).init(std.testing.allocator);
    defer channel.deinit(std.testing.allocator);

    try std.testing.expect(channel.tryRecv() == null);

    try channel.send(std.testing.allocator, 15);
    try std.testing.expectEqual(15, channel.recv());
    try std.testing.expect(channel.tryRecv() == null);
}

test "multiple consumers" {
    var channel = try Channel(u32).init(std.testing.allocator);
    defer channel.deinit(std.testing.allocator);
    errdefer channel.close();

    const num_threads = 4;
    const num_increments = 1000;

    const value_map = try std.testing.allocator.alignedAlloc(
        atomic.Value(bool),
        atomic.cache_line,
        num_increments,
    );
    defer std.testing.allocator.free(value_map);

    const Runner = struct {
        thread: std.Thread = undefined,
        channel: *Channel(u32),
        value_map: []align(atomic.cache_line) atomic.Value(bool),

        fn run(self: *@This()) void {
            while (self.channel.recv()) |v| {
                self.value_map[v].store(true, .monotonic);
            }
        }
    };

    var runners = [_]Runner{.{ .channel = &channel, .value_map = value_map }} ** num_threads;
    for (&runners) |*r| r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
    errdefer for (runners) |r| r.thread.join();

    for (0..num_increments) |i| try channel.send(std.testing.allocator, @truncate(i));
    channel.close();

    for (runners) |r| r.thread.join();
    for (value_map) |v| try std.testing.expectEqual(true, v.load(.monotonic));
}

test "multiple consumers fixed capacity" {
    var channel = try Channel(u32).initWithCapacity(std.testing.allocator, 40);
    defer channel.deinit(std.testing.allocator);
    errdefer channel.close();

    const num_threads = 4;
    const num_increments = 1000;

    const value_map = try std.testing.allocator.alignedAlloc(
        atomic.Value(bool),
        atomic.cache_line,
        num_increments,
    );
    defer std.testing.allocator.free(value_map);

    const Runner = struct {
        thread: std.Thread = undefined,
        channel: *Channel(u32),
        value_map: []align(atomic.cache_line) atomic.Value(bool),

        fn run(self: *@This()) void {
            while (self.channel.recv()) |v| {
                self.value_map[v].store(true, .monotonic);
            }
        }
    };

    var runners = [_]Runner{.{ .channel = &channel, .value_map = value_map }} ** num_threads;
    for (&runners) |*r| r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
    errdefer for (runners) |r| r.thread.join();

    for (0..num_increments) |i| {
        while (true) {
            channel.trySend(@truncate(i)) catch continue;
            break;
        }
    }
    channel.close();

    for (runners) |r| r.thread.join();
    for (value_map) |v| try std.testing.expectEqual(true, v.load(.monotonic));
}
