const std = @import("std");
const atomic = std.atomic;

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

const closed_bit: usize = 0b01;
const waiting_bit: usize = 0b10;

const flags_mask: usize = 0b11;
const list_mask: usize = ~flags_mask;

/// A single-producer, multi-consumer channel that uses intrusive linked lists.
///
/// This channel does not allocate memory for the elements, instead it uses the elements themselves
/// as nodes in a linked list.
// Based on https://www.boost.org/doc/libs/1_87_0/libs/atomic/doc/html/atomic/usage_examples.html
pub fn IntrusiveMpscChannel(comptime T: type) type {
    {
        const info = @typeInfo(T);
        if (info != .pointer) @compileError("IntrusiveMpscChannel only supports pointer types");
        const child = info.pointer.child;
        if (T != *child)
            @compileError("IntrusiveMpscChannel only supports non-const or optional pointer types");
        if (!@hasField(child, "next") or @FieldType(child, "next") != ?T)
            @compileError("IntrusiveMpscChannel only supports intrusive linked lists element types");
    }

    return struct {
        push_list: atomic.Value(usize) align(atomic.cache_line) = .init(0),
        pop_list: atomic.Value(?T) align(atomic.cache_line) = .init(null),

        const Self = @This();
        pub const Sender = SenderImpl(T);
        pub const Receiver = ReceiverImpl(T);

        /// An empty channel.
        pub const empty: Self = .{};

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

        /// Checks if the channel is closed.
        pub fn isClosed(self: *Self) bool {
            const state = self.push_list.load(.monotonic);
            return state & closed_bit != 0;
        }

        /// Closes the channel.
        ///
        /// After closing the channel, no more messages can be sent into it.
        /// All blocked receivers will be woken up and receive an error.
        pub fn close(self: *Self, futex: *Futex) void {
            const state = self.push_list.load(.monotonic);

            // If the channel is already closed we can skip waking up the consumers.
            if (state & closed_bit != 0) return;

            // Mark the channel as closed and wake all consumers.
            _ = self.push_list.fetchOr(closed_bit, .release);
            self.sender().signal(futex);
        }
    };
}

fn SenderImpl(comptime T: type) type {
    return struct {
        channel: *IntrusiveMpscChannel(T),

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
            std.debug.assert(msg.next == null);
            const channel = self.channel;
            while (true) {
                const state = channel.push_list.load(.monotonic);

                // If the queue is closed we are not allowed to send any messages.
                if (state & closed_bit != 0) return TrySendError.Closed;

                // Try to swap to the new head.
                const head: ?T = @ptrFromInt(state & list_mask);
                msg.next = head;
                if (channel.push_list.cmpxchgWeak(
                    state,
                    @intFromPtr(msg) | (state & waiting_bit),
                    .release,
                    .monotonic,
                ) != null) continue;

                // Signal the receiver that there is a new message available.
                self.signal(futex);
                return;
            }
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn send(self: Self, futex: *Futex, msg: T) SendError!void {
            return self.trySend(futex, msg) catch |err| switch (err) {
                TrySendError.Closed => SendError.Closed,
                else => unreachable,
            };
        }

        /// Signals one waiting receiver of the channel.
        pub fn signal(self: Self, futex: *Futex) void {
            // Do nothing if there are no waiters.
            if (self.channel.push_list.load(.monotonic) & waiting_bit == 0) return;
            self.signalSlow(futex);
        }

        /// Signals all waiting receivers of the channel.
        pub fn broadcast(self: Self, futex: *Futex) void {
            self.signal(futex);
        }

        fn signalSlow(self: Self, futex: *Futex) void {
            @branchHint(.cold);
            const state = self.channel.push_list.fetchAnd(~waiting_bit, .monotonic);
            if (state & waiting_bit == 0) return;
            _ = futex.wake(&self.channel.push_list, 1);
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
        channel: *IntrusiveMpscChannel(T),

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
            const channel = self.channel;

            // Try to take one element from the pop list.
            if (channel.pop_list.load(.acquire)) |msg| {
                channel.pop_list.store(msg.next, .release);
                msg.next = null;
                return msg;
            }

            // If the pop list is empty, continue by taking the push list.
            const state = channel.push_list.fetchAnd(flags_mask, .acquire);

            // Reverse the push list and append it to the pop list.
            var head: T = @as(?T, @ptrFromInt(state & list_mask)) orelse {
                if (state & closed_bit != 0) return RecvError.Closed;
                return null;
            };
            var pop_list_head: ?T = null;
            while (head.next) |next| {
                head.next = pop_list_head;
                pop_list_head = head;
                head = next;
            }
            channel.pop_list.store(pop_list_head, .release);
            std.debug.assert(head.next == null);

            return head;
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recv(self: Self, futex: *Futex) RecvError!T {
            if (try self.tryRecv()) |msg| return msg;
            return self.recvSlow(futex, null) catch |err| switch (err) {
                TimedRecvError.Closed => RecvError.Closed,
                TimedRecvError.Timeout => unreachable,
            };
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(self: Self, futex: *Futex, duration: Duration) TimedRecvError!T {
            const timeout = Instant.now().addSaturating(duration);
            return self.recvUntil(futex, timeout);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(self: Self, futex: *Futex, timeout: Instant) TimedRecvError!T {
            if (try self.tryRecv()) |msg| return msg;
            return self.recvSlow(futex, timeout);
        }

        fn recvSlow(self: Self, futex: *Futex, timeout: ?Instant) TimedRecvError!T {
            @branchHint(.cold);

            var spin_count: usize = 0;
            const spin_relax_limit = 12;
            const spin_limit = spin_relax_limit + 4;
            while (true) {
                // Try receiving a message again.
                if (self.tryRecv()) |msg| {
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
            const state = self.channel.push_list.load(.monotonic);
            if (state & list_mask != 0) return WaitError.Retry;
            if (state & closed_bit != 0) return WaitError.Retry;

            // Mark the channel as having a parked task.
            if (state & waiting_bit == 0) {
                if (self.channel.push_list.cmpxchgWeak(
                    0,
                    waiting_bit,
                    .monotonic,
                    .monotonic,
                )) |_| return WaitError.Retry;
            }

            return .{
                .key = &self.channel.push_list,
                .key_size = @sizeOf(usize),
                .expect = waiting_bit,
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

test "smoke test" {
    const Node = struct {
        next: ?*@This() = null,
        data: u32,
    };

    var futex = Futex.init(std.testing.allocator);
    defer futex.deinit();

    var channel = IntrusiveMpscChannel(*Node).empty;
    const sx = channel.sender();
    const rx = channel.receiver();

    var node = Node{ .data = 15 };
    try std.testing.expect(rx.tryRecv() catch unreachable == null);

    try sx.send(&futex, &node);
    try std.testing.expectEqual(&node, try rx.recv(&futex));
    try std.testing.expect(rx.tryRecv() catch unreachable == null);

    channel.close(&futex);
    try std.testing.expectError(SendError.Closed, sx.send(&futex, &node));
}

fn channelTest(
    repeats: usize,
    num_threads: usize,
    num_messages: usize,
) !void {
    const Node = struct {
        next: ?*@This() = null,
        received: bool = false,
    };

    var futex = Futex.init(std.testing.allocator);
    defer futex.deinit();

    for (0..repeats) |_| {
        var channel = IntrusiveMpscChannel(*Node).empty;
        const rx = channel.receiver();

        const nodes = try std.testing.allocator.alloc(Node, num_messages);
        defer std.testing.allocator.free(nodes);
        for (nodes) |*node| node.* = .{};

        var node_counter = atomic.Value(usize).init(0);
        var finished_counter = atomic.Value(usize).init(0);

        const Runner = struct {
            thread: std.Thread = undefined,
            sx: IntrusiveMpscChannel(*Node).Sender,
            futex: *Futex,
            num_threads: usize,
            nodes: []Node,
            node_counter: *atomic.Value(usize),
            finished_counter: *atomic.Value(usize),

            fn run(self: *@This()) void {
                while (true) {
                    const idx = self.node_counter.fetchAdd(1, .monotonic);
                    if (idx >= self.nodes.len) {
                        const finished = self.finished_counter.fetchAdd(1, .acquire);
                        if (finished == self.num_threads - 1) self.sx.channel.close(self.futex);
                        return;
                    }

                    const node = &self.nodes[idx];
                    self.sx.send(self.futex, node) catch unreachable;
                }
            }
        };

        const runners = try std.testing.allocator.alloc(Runner, num_threads);
        defer std.testing.allocator.free(runners);
        for (runners) |*r| {
            r.* = .{
                .sx = channel.sender(),
                .futex = &futex,
                .num_threads = num_threads,
                .nodes = nodes,
                .node_counter = &node_counter,
                .finished_counter = &finished_counter,
            };
            r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
        }

        while (true) {
            const elem = rx.recv(&futex) catch break;
            elem.received = true;
        }
        for (runners) |r| r.thread.join();
        for (nodes) |n| try std.testing.expectEqual(true, n.received);
    }
}

test "intrusive mpsc: one one" {
    try channelTest(1000, 1, 1);
}

test "intrusive mpsc: one hundred" {
    try channelTest(100, 1, 100);
}

test "intrusive mpsc: hundred hundred" {
    try channelTest(100, 100, 100);
}
