const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;
const fimo_tasks_meta = @import("fimo_tasks_meta");
const MetaParkingLot = fimo_tasks_meta.sync.ParkingLot;

const ParkingLot = @import("../ParkingLot.zig");
const Worker = @import("../Worker.zig");
const intrusive_mpsc = @import("intrusive_mpsc.zig");
const IntrusiveMpscChannel = intrusive_mpsc.IntrusiveMpscChannel;
const receiver = @import("receiver.zig");
const GenericReceiver = receiver.ParkableReceiver;
const RecvError = receiver.RecvError;
const TimedRecvError = receiver.TimedRecvError;
const ParkError = receiver.ParkError;
const sender = @import("sender.zig");
const TrySendError = sender.TrySendError;
const SendError = sender.SendError;
const GenericSender = sender.Sender;

/// An allocating single-producer, multiple-consumer channel.
pub fn MpscChannel(comptime T: type) type {
    return struct {
        intrusive: IntrusiveMpscChannel(*Node) = .empty,

        const Self = @This();
        const Node = struct {
            next: ?*Node = null,
            data: T,
        };
        pub const Sender = SenderImpl(T);
        pub const Receiver = ReceiverImpl(T);

        /// An empty channel.
        pub const empty: Self = .{};

        /// Returns a sender backed by this channel.
        pub fn sender(self: *Self, allocator: Allocator) Sender {
            return Sender{
                .allocator = allocator,
                .sx = self.intrusive.sender(),
            };
        }

        /// Returns a receiver backed by this channel.
        pub fn receiver(self: *Self, allocator: Allocator) Receiver {
            return Receiver{
                .allocator = allocator,
                .rx = self.intrusive.receiver(),
            };
        }

        /// Clears the channel.
        pub fn clearAndFree(self: *Self, allocator: Allocator) void {
            const rx = self.receiver(allocator);
            while (true) {
                _ = rx.tryRecv() catch break;
            }
        }

        /// Checks if the channel is closed.
        pub fn isClosed(self: *Self) bool {
            return self.intrusive.isClosed();
        }

        /// Closes the channel.
        ///
        /// After closing the channel, no more messages can be sent into it.
        /// All blocked receivers will be woken up and receive an error.
        pub fn close(self: *Self, lot: *ParkingLot) void {
            self.intrusive.close(lot);
        }
    };
}

fn SenderImpl(comptime T: type) type {
    return struct {
        allocator: Allocator,
        sx: IntrusiveMpscChannel(*Node).Sender,

        const Self = @This();
        const Node = MpscChannel(T).Node;

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
            const node = self.allocator.create(Node) catch return TrySendError.SendFailed;
            errdefer self.allocator.destroy(node);
            node.* = Node{ .data = msg };
            try self.sx.trySend(lot, node);
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn send(self: Self, lot: *ParkingLot, msg: T) SendError!void {
            const node = self.allocator.create(Node) catch return TrySendError.SendFailed;
            errdefer self.allocator.destroy(node);
            node.* = Node{ .data = msg };
            try self.sx.send(lot, node);
        }

        /// Signals one waiting receiver of the channel.
        pub fn signal(self: Self, lot: *ParkingLot) void {
            self.sx.signal(lot);
        }

        /// Signals all waiting receivers of the channel.
        pub fn broadcast(self: Self, lot: *ParkingLot) void {
            self.sx.broadcast(lot);
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
        allocator: Allocator,
        rx: IntrusiveMpscChannel(*Node).Receiver,

        const Self = @This();
        const Node = MpscChannel(T).Node;

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
            const node = (try self.rx.tryRecv()) orelse return null;
            const msg = node.data;
            self.allocator.destroy(node);
            return msg;
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recv(self: Self, lot: *ParkingLot) RecvError!T {
            const node = try self.rx.recv(lot);
            const msg = node.data;
            self.allocator.destroy(node);
            return msg;
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(self: Self, lot: *ParkingLot, duration: Duration) TimedRecvError!T {
            const node = try self.rx.recvFor(lot, duration);
            const msg = node.data;
            self.allocator.destroy(node);
            return msg;
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(self: Self, lot: *ParkingLot, timeout: Instant) TimedRecvError!T {
            const node = try self.rx.recvUntil(lot, timeout);
            const msg = node.data;
            self.allocator.destroy(node);
            return msg;
        }

        /// Returns the channel used to park the receiver.
        pub fn parkChannel(self: Self) *anyopaque {
            return self.rx.parkChannel();
        }

        /// Returns the key used to park the receiver.
        pub fn parkKey(self: Self) *const anyopaque {
            return self.rx.parkKey();
        }

        /// Prepares the receiver for parking.
        pub fn preparePark(self: Self, channel: *anyopaque) ParkError!void {
            try self.rx.preparePark(channel);
        }

        /// Checks whether the caller should park.
        pub fn shouldPark(self: Self, channel: *anyopaque) bool {
            return self.rx.shouldPark(channel);
        }

        /// Callback to handle a timeout while parked.
        pub fn onParkTimeout(
            self: Self,
            channel: *anyopaque,
            key: *const anyopaque,
            was_last: bool,
        ) void {
            self.rx.onParkTimeout(channel, key, was_last);
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

fn channelTest(
    repeats: usize,
    num_threads: usize,
    num_messages: usize,
) !void {
    var lot = ParkingLot.init(std.testing.allocator);
    defer lot.deinit();

    for (0..repeats) |_| {
        var channel = MpscChannel(usize).empty;
        const rx = channel.receiver(std.testing.allocator);

        const nodes = try std.testing.allocator.alloc(bool, num_messages);
        defer std.testing.allocator.free(nodes);

        var node_counter = atomic.Value(usize).init(0);
        var finished_counter = atomic.Value(usize).init(0);

        const Runner = struct {
            thread: std.Thread = undefined,
            sx: MpscChannel(usize).Sender,
            lot: *ParkingLot,
            num_threads: usize,
            nodes: []bool,
            node_counter: *atomic.Value(usize),
            finished_counter: *atomic.Value(usize),

            fn run(self: *@This()) void {
                while (true) {
                    const idx = self.node_counter.fetchAdd(1, .monotonic);
                    if (idx >= self.nodes.len) {
                        const finished = self.finished_counter.fetchAdd(1, .acquire);
                        if (finished == self.num_threads - 1) self.sx.sx.channel.close(self.lot);
                        return;
                    }
                    self.sx.send(self.lot, idx) catch unreachable;
                }
            }
        };

        const runners = try std.testing.allocator.alloc(Runner, num_threads);
        defer std.testing.allocator.free(runners);
        for (runners) |*r| {
            r.* = .{
                .sx = channel.sender(std.testing.allocator),
                .lot = &lot,
                .num_threads = num_threads,
                .nodes = nodes,
                .node_counter = &node_counter,
                .finished_counter = &finished_counter,
            };
            r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
        }

        while (true) {
            const idx = rx.recv(&lot) catch break;
            nodes[idx] = true;
        }
        for (runners) |r| r.thread.join();
        for (nodes) |n| try std.testing.expectEqual(true, n);
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
