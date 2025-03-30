const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const MetaParkingLot = fimo_tasks_meta.sync.ParkingLot;

const ParkingLot = @import("../ParkingLot.zig");
const Worker = @import("../Worker.zig");
const receiver = @import("receiver.zig");
const GenericReceiver = receiver.ParkableReceiver;
const RecvError = receiver.RecvError;
const TimedRecvError = receiver.TimedRecvError;
const ParkError = receiver.ParkError;
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
        pub fn close(self: *Self, lot: *ParkingLot) void {
            const state = self.push_list.load(.monotonic);

            // If the channel is already closed we can skip waking up the consumers.
            if (state & closed_bit != 0) return;

            // Mark the channel as closed and wake all consumers.
            _ = self.push_list.fetchOr(closed_bit, .release);
            self.sender().broadcast(lot);
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
        pub fn trySend(self: Self, lot: *ParkingLot, msg: T) TrySendError!void {
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
                self.signal(lot);
                return;
            }
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn send(self: Self, lot: *ParkingLot, msg: T) SendError!void {
            return self.trySend(lot, msg) catch |err| switch (err) {
                TrySendError.Closed => SendError.Closed,
                else => unreachable,
            };
        }

        /// Signals one waiting receiver of the channel.
        pub fn signal(self: Self, lot: *ParkingLot) void {
            // Do nothing if there are no waiters.
            if (self.channel.push_list.load(.monotonic) & waiting_bit == 0) return;
            self.notifySlow(lot, false);
        }

        /// Signals all waiting receivers of the channel.
        pub fn broadcast(self: Self, lot: *ParkingLot) void {
            // Do nothing if there are no waiters.
            if (self.channel.push_list.load(.monotonic) & waiting_bit == 0) return;
            self.notifySlow(lot, true);
        }

        fn notifySlow(self: Self, lot: *ParkingLot, all: bool) void {
            @branchHint(.cold);

            const Validate = struct {
                all: bool,
                fn f(this: @This()) MetaParkingLot.RequeueOp {
                    return MetaParkingLot.RequeueOp{
                        .num_tasks_to_unpark = if (this.all) std.math.maxInt(usize) else 1,
                    };
                }
            };
            const Callback = struct {
                ctx: Self,
                fn f(
                    this: @This(),
                    op: MetaParkingLot.RequeueOp,
                    result: MetaParkingLot.UnparkResult,
                ) MetaParkingLot.UnparkToken {
                    _ = op;
                    // If there aren't any waiters left we clear the waiting bit.
                    if (!result.has_more_tasks) _ = this.ctx.channel.push_list.fetchAnd(
                        ~waiting_bit,
                        .monotonic,
                    );
                    return .default;
                }
            };

            _ = lot.unparkRequeue(
                &self.channel.push_list,
                &self.channel.push_list,
                Validate{ .all = all },
                Validate.f,
                Callback{ .ctx = self },
                Callback.f,
            );
        }

        fn genericTrySend(ctx: *anyopaque, lot: *ParkingLot, msg: T) TrySendError!void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            try self.trySend(lot, msg);
        }

        fn genericSend(ctx: *anyopaque, lot: *ParkingLot, msg: T) SendError!void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            try self.send(lot, msg);
        }

        fn genericSignal(ctx: *anyopaque, lot: *ParkingLot) void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            self.signal(lot);
        }

        fn genericBroadcast(ctx: *anyopaque, lot: *ParkingLot) void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            self.broadcast(lot);
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
                    .parkChannel = &genericParkChannel,
                    .parkKey = &genericParkKey,
                    .preparePark = &genericPreparePark,
                    .shouldPark = &genericShouldPark,
                    .onParkTimeout = &genericOnParkTimeout,
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
        pub fn recv(self: Self, lot: *ParkingLot) RecvError!T {
            if (try self.tryRecv()) |msg| return msg;
            return self.recvSlow(lot, null) catch |err| switch (err) {
                TimedRecvError.Closed => RecvError.Closed,
                TimedRecvError.Timeout => unreachable,
            };
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(self: Self, lot: *ParkingLot, duration: Duration) TimedRecvError!T {
            const timeout = Instant.now().addSaturating(duration);
            return self.recvUntil(lot, timeout);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(self: Self, lot: *ParkingLot, timeout: Instant) TimedRecvError!T {
            if (try self.tryRecv()) |msg| return msg;
            return self.recvSlow(lot, timeout);
        }

        fn recvSlow(self: Self, lot: *ParkingLot, timeout: ?Instant) TimedRecvError!T {
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
                self.park(lot, timeout) catch |err| switch (err) {
                    ParkError.Retry => continue,
                    ParkError.Timeout => return TimedRecvError.Timeout,
                };

                // Retry another round.
                spin_count = 0;
            }

            unreachable;
        }

        /// Returns the channel used to park the receiver.
        pub fn parkChannel(self: Self) *anyopaque {
            return self.channel;
        }

        /// Returns the key used to park the receiver.
        pub fn parkKey(self: Self) *const anyopaque {
            return &self.channel.push_list;
        }

        /// Prepares the receiver for parking.
        pub fn preparePark(self: Self, channel: *anyopaque) ParkError!void {
            std.debug.assert(@as(*anyopaque, self.channel) == channel);
            const state = self.channel.push_list.load(.monotonic);
            if (state & list_mask != 0) return;
            if (state & closed_bit != 0) return;

            // Mark the channel as having a parked task.
            if (state & waiting_bit == 0) {
                if (self.channel.push_list.cmpxchgWeak(
                    state,
                    state | waiting_bit,
                    .monotonic,
                    .monotonic,
                )) |_| return ParkError.Retry;
            }
        }

        /// Checks whether the caller should park.
        pub fn shouldPark(self: Self, channel: *anyopaque) bool {
            std.debug.assert(@as(*anyopaque, self.channel) == channel);
            return self.channel.push_list.load(.monotonic) & ~closed_bit == waiting_bit;
        }

        /// Callback to handle a timeout while parked.
        pub fn onParkTimeout(
            self: Self,
            channel: *anyopaque,
            key: *const anyopaque,
            was_last: bool,
        ) void {
            _ = key;
            std.debug.assert(@as(*anyopaque, self.channel) == channel);
            if (was_last) {
                _ = self.channel.push_list.fetchAnd(~waiting_bit, .monotonic);
            }
        }

        fn park(self: Self, lot: *ParkingLot, timeout: ?Instant) ParkError!void {
            try self.preparePark(self.channel);

            const Validation = struct {
                ctx: Self,
                fn f(this: @This()) bool {
                    return this.ctx.shouldPark(this.ctx.channel);
                }
            };
            const BeforeSleep = struct {
                fn f(this: @This()) void {
                    _ = this;
                }
            };
            const TimedOut = struct {
                ctx: Self,
                fn f(this: @This(), key: *const anyopaque, was_last: bool) void {
                    this.ctx.onParkTimeout(this.ctx.channel, key, was_last);
                }
            };
            const result = lot.park(
                self.parkKey(),
                Validation{ .ctx = self },
                Validation.f,
                BeforeSleep{},
                BeforeSleep.f,
                TimedOut{ .ctx = self },
                TimedOut.f,
                .default,
                timeout,
            );
            switch (result.type) {
                // The state changed or we were unparked, retry.
                .unparked, .invalid => {},
                // We timed out, return the timeout error.
                .timed_out => return ParkError.Timeout,
            }
        }

        fn genericTryRecv(ctx: *anyopaque) RecvError!?T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.tryRecv();
        }

        fn genericRecv(ctx: *anyopaque, lot: *ParkingLot) RecvError!T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.recv(lot);
        }

        fn genericRecvFor(ctx: *anyopaque, lot: *ParkingLot, duration: Duration) TimedRecvError!T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.recvFor(lot, duration);
        }

        fn genericRecvUntil(ctx: *anyopaque, lot: *ParkingLot, timeout: Instant) TimedRecvError!T {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.recvUntil(lot, timeout);
        }

        fn genericParkChannel(ctx: *anyopaque) *anyopaque {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.parkChannel();
        }

        fn genericParkKey(ctx: *anyopaque) *const anyopaque {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.parkKey();
        }

        fn genericPreparePark(ctx: *anyopaque, channel: *anyopaque) ParkError!void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.preparePark(channel);
        }

        fn genericShouldPark(ctx: *anyopaque, channel: *anyopaque) bool {
            const self: *Self = @ptrCast(@alignCast(ctx));
            return self.shouldPark(channel);
        }

        fn genericOnParkTimeout(
            ctx: *anyopaque,
            channel: *anyopaque,
            key: *const anyopaque,
            was_last: bool,
        ) void {
            const self: *Self = @ptrCast(@alignCast(ctx));
            self.onParkTimeout(channel, key, was_last);
        }
    };
}

test "smoke test" {
    const Node = struct {
        next: ?*@This() = null,
        data: u32,
    };

    var lot = ParkingLot.init(std.testing.allocator);
    defer lot.deinit();

    var channel = IntrusiveMpscChannel(*Node).empty;
    const sx = channel.sender();
    const rx = channel.receiver();

    var node = Node{ .data = 15 };
    try std.testing.expect(rx.tryRecv() catch unreachable == null);

    try sx.send(&lot, &node);
    try std.testing.expectEqual(&node, try rx.recv(&lot));
    try std.testing.expect(rx.tryRecv() catch unreachable == null);

    channel.close(&lot);
    try std.testing.expectError(SendError.Closed, sx.send(&lot, &node));
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

    var lot = ParkingLot.init(std.testing.allocator);
    defer lot.deinit();

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
            lot: *ParkingLot,
            num_threads: usize,
            nodes: []Node,
            node_counter: *atomic.Value(usize),
            finished_counter: *atomic.Value(usize),

            fn run(self: *@This()) void {
                while (true) {
                    const idx = self.node_counter.fetchAdd(1, .monotonic);
                    if (idx >= self.nodes.len) {
                        const finished = self.finished_counter.fetchAdd(1, .acquire);
                        if (finished == self.num_threads - 1) self.sx.channel.close(self.lot);
                        return;
                    }

                    const node = &self.nodes[idx];
                    self.sx.send(self.lot, node) catch unreachable;
                }
            }
        };

        const runners = try std.testing.allocator.alloc(Runner, num_threads);
        defer std.testing.allocator.free(runners);
        for (runners) |*r| {
            r.* = .{
                .sx = channel.sender(),
                .lot = &lot,
                .num_threads = num_threads,
                .nodes = nodes,
                .node_counter = &node_counter,
                .finished_counter = &finished_counter,
            };
            r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
        }

        while (true) {
            const elem = rx.recv(&lot) catch break;
            elem.received = true;
        }
        for (runners) |r| r.thread.join();
        for (nodes) |n| try std.testing.expectEqual(true, n.received);
    }
}

test "one one" {
    try channelTest(1000, 1, 1);
}

test "one hundred" {
    try channelTest(100, 1, 100);
}

test "hundred hundred" {
    try channelTest(100, 100, 100);
}
