const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;

const Futex = @import("../Futex.zig");
const Worker = @import("../Worker.zig");
const receiver = @import("receiver.zig");
const GenericReceiver = receiver.WaitableReceiver;
const RecvError = receiver.RecvError;
const TimedRecvError = receiver.TimedRecvError;
const WaitError = receiver.WaitError;
const sender = @import("sender.zig");
const TrySendError = sender.TrySendError;
const SendError = sender.SendError;
const GenericSender = sender.Sender;

// The implementation uses a sum tree to identify free slots.
// The number of nodes in a perfect binary tree is `2^depth - 1`, with `2^(depth-1)` leaf
// nodes. Assuming 64bit `usize`, we are bounded to `2^64 - 1` nodes and `2^63` leaf nodes.
// This leaves the MSB unused , which we employ the determine if the channel is closed.
const closed_bit_position = @bitSizeOf(usize) - 1;
const closed_bit: usize = 1 << closed_bit_position;

const counter_mask: usize = ~closed_bit;

/// A single producer, multiple consumers unbounded channel.
///
/// The channel is unordered, meaning that the insertion order is not guaranteed to be preserved
/// by the consumers of the channel.
pub fn UnorderedBoundedSpmcChannel(comptime T: type) type {
    return struct {
        depth: u8 = 0,
        elements: []Element = &.{},
        counters: []align(atomic.cache_line) atomic.Value(usize) = &.{},
        dummy_counter: atomic.Value(usize) align(atomic.cache_line) = .init(0),
        prev: ?*Self = null,

        const Self = @This();

        pub const Sender = SenderImpl(T);
        pub const Receiver = ReceiverImpl(T);

        /// An empty channel.
        pub const empty: Self = .{};

        const Element = struct {
            value: T = undefined,
            filled: atomic.Value(bool) align(atomic.cache_line) = .init(false),
        };

        /// Initializes a new channel with the given capacity.
        pub fn initWithCapacity(allocator: Allocator, n: usize) Allocator.Error!Self {
            return initWithCapacityChained(allocator, n, null);
        }

        /// Initializes a new channel with the given capacity and a previous channel.
        pub fn initWithCapacityChained(
            allocator: Allocator,
            n: usize,
            prev: ?*Self,
        ) Allocator.Error!Self {
            if (n == 0) return empty;
            const n_ceil = std.math.ceilPowerOfTwo(usize, n) catch
                return Allocator.Error.OutOfMemory;
            const depth = std.math.log2(n_ceil) + 1;
            if (depth >= 63) return Allocator.Error.OutOfMemory;

            const elements = try allocator.alloc(Element, n_ceil);
            errdefer allocator.free(elements);
            for (elements) |*element| {
                element.* = .{};
            }

            const num_counters = (n_ceil * 2) - 1;
            const counters = try allocator.alignedAlloc(
                atomic.Value(usize),
                .fromByteUnits(atomic.cache_line),
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

        /// Deinitializes the channel and all linked channels.
        pub fn deinit(self: *Self, allocator: Allocator) void {
            var current: ?*Self = self;
            while (current) |curr| {
                allocator.free(curr.elements);
                allocator.free(curr.counters);
                current = curr.prev;
                if (curr != self) allocator.destroy(curr);
            }
            self.* = .{};
        }

        /// Returns a sender backed by this channel.
        pub fn sender(self: *Self) Sender {
            return Sender{
                .channel = self,
            };
        }

        /// Returns a receiver backed by this channel.
        pub fn receiver(self: *Self) Receiver {
            return Receiver{
                .channel = self,
            };
        }

        /// Returns the capacity of the channel.
        pub fn capacity(self: *Self) usize {
            return self.elements.len;
        }

        /// Closes the channel.
        ///
        /// After closing the channel, no more messages can be sent into it.
        /// All blocked receivers will be woken up and receive an error.
        pub fn close(self: *Self, futex: *Futex) void {
            const root = self.rootNode();
            const state = root.load(.monotonic);

            // If the channel is already closed we can skip waking up the consumers.
            if (state & closed_bit != 0) return;

            // Mark the channel as closed and wake all consumers.
            _ = root.fetchOr(closed_bit, .release);
            self.sender().broadcast(futex);
        }

        fn rootNode(self: *Self) *align(atomic.cache_line) atomic.Value(usize) {
            return if (self.elements.len == 0)
                &self.dummy_counter
            else
                &self.counters[0];
        }
    };
}

fn SenderImpl(comptime T: type) type {
    return struct {
        channel: *UnorderedBoundedSpmcChannel(T),

        const Self = @This();

        /// Returns a generic sender backed by this sender.
        pub fn sender(self: *Self) GenericSender(T) {
            return GenericSender(T){
                .context = self,
                .vtable = &GenericSender(T).VTable{
                    .trySend = &genericTrySend,
                    .send = &genericSend,
                    .signal = &genericSignal,
                    .broadcast = &genericBroadcast,
                },
            };
        }

        /// Tries to send a message into the channel without blocking.
        pub fn trySend(self: Self, futex: *Futex, msg: T) TrySendError!void {
            try self.trySendWithSeed(futex, 0, msg);
        }

        /// Tries to send a message into the channel without blocking.
        pub fn trySendWithSeed(self: Self, futex: *Futex, seed: usize, msg: T) TrySendError!void {
            // Check if the tree is full or closed.
            const channel = self.channel;
            const root = channel.rootNode();
            const root_state = root.load(.monotonic);
            if (root_state & closed_bit != 0) return TrySendError.Closed;
            if (root_state & counter_mask == channel.capacity()) return TrySendError.Full;

            // Compute the index of the next free element by traversing the tree.
            var elem_idx: usize = 0;
            var search_order: usize = seed;
            var max_elements: usize = channel.capacity() >> 1;
            if (channel.depth > 1) {
                for (1..channel.depth - 1) |layer_idx| {
                    const layer_start: usize = (@as(usize, 1) << @truncate(layer_idx)) - 1;
                    const left_idx = layer_start + elem_idx;
                    const right_idx = left_idx + 1;

                    // The LSB determines if we prefer the left or right child node.
                    const first_idx, const second_index = if (search_order & 1 == 0)
                        .{ left_idx, right_idx }
                    else
                        .{ right_idx, left_idx };

                    const first_count = channel.counters[first_idx].load(.monotonic);
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
            var element = &channel.elements[elem_idx];

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
            element.value = msg;
            element.filled.store(true, .release);

            // Traverse the tree from the leaf to the root and increase the counter.
            var num_elements: usize = 0;
            var counter_index = (@as(usize, 1) << @truncate(channel.depth - 1)) + elem_idx - 1;
            for (0..channel.depth) |_| {
                const counter = &channel.counters[counter_index];
                num_elements = counter.fetchAdd(1, .release);
                counter_index -%= 1;
                counter_index >>= 1;
            }

            self.signal(futex);
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn send(self: Self, futex: *Futex, msg: T) SendError!void {
            try self.sendWithSeed(futex, 0, msg);
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn sendWithSeed(self: Self, futex: *Futex, seed: usize, msg: T) SendError!void {
            while (true) {
                // Todo: Replace the busy wait with a more efficient mechanism.
                return self.trySendWithSeed(futex, seed, msg) catch |err| switch (err) {
                    TrySendError.Closed => SendError.Closed,
                    TrySendError.Full => continue,
                    else => unreachable,
                };
            }
        }

        /// Signals one waiting receiver of the channel.
        pub fn signal(self: Self, futex: *Futex) void {
            _ = futex.wake(self.channel.rootNode(), 1);
        }

        /// Signals all waiting receivers of the channel.
        pub fn broadcast(self: Self, futex: *Futex) void {
            _ = futex.wake(self.channel.rootNode(), std.math.maxInt(usize));
        }

        fn genericTrySend(ctx: *anyopaque, futex: *Futex, msg: T) TrySendError!void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            try self.trySend(futex, msg);
        }

        fn genericSend(ctx: *anyopaque, futex: *Futex, msg: T) SendError!void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            try self.send(futex, msg);
        }

        fn genericSignal(ctx: *anyopaque, futex: *Futex) void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            self.signal(futex);
        }

        fn genericBroadcast(ctx: *anyopaque, futex: *Futex) void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            self.broadcast(futex);
        }
    };
}

fn ReceiverImpl(comptime T: type) type {
    return struct {
        channel: *UnorderedBoundedSpmcChannel(T),

        const Self = @This();

        /// Returns a generic receiver backed by this receiver.
        pub fn receiver(self: *Self) GenericReceiver(T) {
            return GenericReceiver(T){
                .context = self,
                .vtable = &GenericReceiver(T).VTable{
                    .tryRecv = &genericTryRecv,
                    .recv = &genericRecv,
                    .recvFor = &genericRecvFor,
                    .recvUntil = &genericRecvUntil,
                    .prepareWait = &genericPrepareWait,
                },
            };
        }

        /// Receives one message from the channel.
        ///
        /// The caller will return `null` immediately, if the channel is empty.
        /// If the channel is closed and empty, an error is returned.
        pub fn tryRecv(self: Self) RecvError!?T {
            return self.tryRecvWithSeed(0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will return `null` immediately, if the channel is empty.
        /// If the channel is closed and empty, an error is returned.
        pub fn tryRecvWithSeed(self: Self, seed: usize) RecvError!?T {
            // Decrement the counter of the root node.
            const channel = self.channel;
            {
                const root = channel.rootNode();
                var state = root.load(.monotonic);
                while (true) {
                    // If the channel is empty we abort.
                    if (state & counter_mask == 0) {
                        if (state & closed_bit != 0) return RecvError.Closed;
                        return null;
                    }

                    // Try decrementing the counter by one.
                    if (root.cmpxchgWeak(state, state - 1, .acquire, .monotonic)) |new| {
                        state = new;
                        continue;
                    }

                    // Now that we decremented the counter we have acquired one slot from the channel.
                    break;
                }
            }

            // Traverse the tree to find the slot that belongs to us, decrementing the node count
            // along the way by one. We use the seed to determine whether to peek into the left or
            // right child node.
            var elem_idx: usize = 0;
            var search_order: usize = seed;
            if (channel.depth != 0) {
                for (1..channel.depth) |layer_idx| {
                    const layer_start: usize = (@as(usize, 1) << @truncate(layer_idx)) - 1;
                    const left_idx = layer_start + elem_idx;
                    const right_idx = left_idx + 1;

                    // The LSB determines if we prefer the left or right child node.
                    const first_idx, const second_idx = if (search_order & 1 == 0)
                        .{ left_idx, right_idx }
                    else
                        .{ right_idx, left_idx };
                    const first_node = &channel.counters[first_idx];
                    const second_node = &channel.counters[second_idx];

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
            const slot = &channel.elements[elem_idx];
            const value = slot.value;
            slot.filled.store(false, .release);

            return value;
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recv(self: Self, futex: *Futex) RecvError!T {
            return self.recvWithSeed(futex, 0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvWithSeed(self: Self, futex: *Futex, seed: usize) RecvError!T {
            if (try self.tryRecvWithSeed(seed)) |msg| return msg;
            return self.recvSlow(futex, seed, null) catch |err| switch (err) {
                TimedRecvError.Closed => RecvError.Closed,
                TimedRecvError.Timeout => unreachable,
            };
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(self: Self, futex: *Futex, duration: Duration) TimedRecvError!T {
            return self.recvForWithSeed(futex, duration, 0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvForWithSeed(
            self: Self,
            futex: *Futex,
            duration: Duration,
            seed: usize,
        ) TimedRecvError!T {
            const timeout = Instant.now().addSaturating(duration);
            return self.recvUntilWithSeed(futex, timeout, seed);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(self: Self, futex: *Futex, timeout: Instant) TimedRecvError!T {
            return self.recvUntilWithSeed(futex, timeout, 0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntilWithSeed(
            self: Self,
            futex: *Futex,
            timeout: Instant,
            seed: usize,
        ) TimedRecvError!T {
            if (try self.tryRecvWithSeed(seed)) |msg| return msg;
            return self.recvSlow(futex, seed, timeout);
        }

        fn recvSlow(
            self: Self,
            futex: *Futex,
            seed: usize,
            timeout: ?Instant,
        ) TimedRecvError!T {
            @branchHint(.cold);

            var spin_count: usize = 0;
            const spin_relax_limit = 12;
            const spin_limit = spin_relax_limit + 4;
            while (true) {
                // Try receiving a message again.
                if (self.tryRecvWithSeed(seed)) |msg| {
                    if (msg) |m| return m;
                } else |err| return @errorCast(err);

                // Try spinning a couple of times.
                if (spin_count < spin_limit) {
                    if (spin_count < spin_relax_limit)
                        atomic.spinLoopHint()
                    else
                        Worker.yield();
                    spin_count += 1;
                    continue;
                }

                // If the channel is still empty, park the caller.
                self.wait(futex, timeout) catch |err| switch (err) {
                    WaitError.Retry => continue,
                    WaitError.Timeout => return TimedRecvError.Timeout,
                };

                // Retry another round.
                spin_count = 0;
            }

            unreachable;
        }

        /// Prepares the receiver for parking.
        pub fn prepareWait(self: Self) WaitError!Futex.KeyExpect {
            const root = self.channel.rootNode();
            const state = root.load(.monotonic);
            if (state & counter_mask != 0) return WaitError.Retry;
            if (state & closed_bit != 0) return WaitError.Retry;

            return .{
                .key = root,
                .key_size = @sizeOf(usize),
                .expect = 0,
            };
        }

        fn wait(self: Self, futex: *Futex, timeout: ?Instant) WaitError!void {
            const k = try self.prepareWait();
            return futex.wait(k.key, k.key_size, k.expect, timeout) catch |err| switch (err) {
                error.Invalid => WaitError.Retry,
                error.Timeout => WaitError.Timeout,
            };
        }

        fn genericTryRecv(ctx: *anyopaque) RecvError!?T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.tryRecv();
        }

        fn genericRecv(ctx: *anyopaque, futex: *Futex) RecvError!T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.recv(futex);
        }

        fn genericRecvFor(ctx: *anyopaque, futex: *Futex, duration: Duration) TimedRecvError!T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.recvFor(futex, duration);
        }

        fn genericRecvUntil(ctx: *anyopaque, futex: *Futex, timeout: Instant) TimedRecvError!T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.recvUntil(futex, timeout);
        }

        fn genericPrepareWait(ctx: *anyopaque) WaitError!Futex.KeyExpect {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.prepareWait();
        }
    };
}

fn channelTest(
    repeats: usize,
    num_threads: usize,
    num_messages: usize,
    capacity: usize,
) !void {
    var futex = Futex.init(std.testing.allocator);
    defer futex.deinit();

    for (0..repeats) |_| {
        var channel = try UnorderedBoundedSpmcChannel(usize).initWithCapacity(
            std.testing.allocator,
            capacity,
        );
        defer channel.deinit(std.testing.allocator);
        const sx = channel.sender();

        const value_map = try std.testing.allocator.alignedAlloc(
            atomic.Value(bool),
            .fromByteUnits(atomic.cache_line),
            num_messages,
        );
        defer std.testing.allocator.free(value_map);

        const Runner = struct {
            thread: std.Thread = undefined,
            rx: UnorderedBoundedSpmcChannel(usize).Receiver,
            futex: *Futex,
            value_map: []align(atomic.cache_line) atomic.Value(bool),

            fn run(self: *@This()) void {
                while (true) {
                    const msg = self.rx.recv(self.futex) catch |e| switch (e) {
                        RecvError.Closed => return,
                        else => unreachable,
                    };
                    self.value_map[msg].store(true, .monotonic);
                }
            }
        };

        const runners = try std.testing.allocator.alloc(Runner, num_threads);
        defer std.testing.allocator.free(runners);
        for (runners) |*r| {
            r.* = .{
                .rx = channel.receiver(),
                .futex = &futex,
                .value_map = value_map,
            };
            r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
        }

        for (0..num_messages) |i| try sx.send(&futex, i);
        channel.close(&futex);

        for (runners) |r| r.thread.join();
        for (value_map) |v| try std.testing.expectEqual(true, v.load(.monotonic));
    }
}

test "bounded spmc: one one one" {
    try channelTest(1000, 1, 1, 1);
}

test "bounded spmc: one hundred one" {
    try channelTest(100, 1, 100, 1);
}

test "bounded spmc: one hundred twenty" {
    try channelTest(100, 1, 100, 20);
}

test "bounded spmc: hundred hundred one" {
    try channelTest(100, 100, 100, 1);
}

test "bounded spmc: hundred hundred twenty" {
    try channelTest(100, 100, 100, 20);
}

test "bounded spmc: hundred hundred hundred" {
    try channelTest(100, 100, 100, 100);
}
