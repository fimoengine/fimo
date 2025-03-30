const std = @import("std");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const Random = std.Random;

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
const unordered_bounded_spmc = @import("unordered_bounded_spmc.zig");
const UnorderedBoundedSpmcChannel = unordered_bounded_spmc.UnorderedBoundedSpmcChannel;

const closed_bit: usize = 1;
const channel_mask: usize = ~closed_bit;

/// A single producer, multiple consumers unbounded channel.
///
/// The channel is unordered, meaning that the insertion order is not guaranteed to be preserved
/// by the consumers of the channel.
pub fn UnorderedSpmcChannel(comptime T: type) type {
    return struct {
        active_channel: atomic.Value(usize),

        const Self = @This();
        const Inner = UnorderedBoundedSpmcChannel(T);

        pub const Sender = SenderImpl(T);
        pub const Receiver = ReceiverImpl(T);

        /// Initializes a new channel with a capacity of 1.
        pub fn init(allocator: Allocator) Allocator.Error!Self {
            return Self.initWithCapacity(allocator, 1);
        }

        /// Initializes a new channel with the given capacity.
        pub fn initWithCapacity(allocator: Allocator, n: usize) Allocator.Error!Self {
            const channel = try allocator.create(Inner);
            errdefer allocator.destroy(channel);
            channel.* = try Inner.initWithCapacity(allocator, n);
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

        /// Returns a sender backed by this channel.
        pub fn sender(self: *Self, allocator: Allocator) Sender {
            return Sender{
                .channel = self,
                .allocator = allocator,
            };
        }

        /// Returns a receiver backed by this channel.
        pub fn receiver(self: *Self) Receiver {
            return Receiver{
                .channel = self,
            };
        }

        /// Closes the channel.
        ///
        /// All waiting consumers are woken up.
        pub fn close(self: *Self, lot: *ParkingLot) void {
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
                channel.close(lot);
            }
        }
    };
}

fn SenderImpl(comptime T: type) type {
    return struct {
        channel: *UnorderedSpmcChannel(T),
        allocator: Allocator,

        const Self = @This();
        const Inner = UnorderedBoundedSpmcChannel(T);

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
            try self.trySendWithSeed(lot, 0, msg);
        }

        /// Tries to send a message into the channel without blocking.
        pub fn trySendWithSeed(self: Self, lot: *ParkingLot, seed: usize, msg: T) TrySendError!void {
            while (true) {
                const state = self.channel.active_channel.load(.acquire);

                // If the channel is already closed we abort and report a failure.
                if (state & closed_bit != 0) return TrySendError.Closed;

                // Try sending a value into the channel. This may only fail if it is full.
                const channel: *Inner = @ptrFromInt(state & channel_mask);
                return channel.sender().trySendWithSeed(lot, seed, msg) catch |err| switch (err) {
                    TrySendError.Closed => unreachable,
                    else => return err,
                };
            }
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn send(self: Self, lot: *ParkingLot, msg: T) SendError!void {
            try self.sendWithSeed(lot, 0, msg);
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn sendWithSeed(self: Self, lot: *ParkingLot, seed: usize, msg: T) SendError!void {
            while (true) {
                const state = self.channel.active_channel.load(.acquire);

                // If the channel is already closed we abort and report a failure.
                if (state & closed_bit != 0) return SendError.Closed;

                // Try sending a value into the channel. This may only fail if it is full.
                const channel: *Inner = @ptrFromInt(state & channel_mask);
                return channel.sender().trySendWithSeed(lot, seed, msg) catch |err| switch (err) {
                    TrySendError.Closed, TrySendError.SendFailed => unreachable,
                    TrySendError.Full => {
                        // Since the channel is full, we create a new channel and migrate all elements.
                        const allocator = self.allocator;
                        const new_channel = allocator.create(Inner) catch return SendError.SendFailed;
                        errdefer allocator.destroy(new_channel);

                        // The capacity is automatically increased to the next power of two.
                        new_channel.* = Inner.initWithCapacityChained(
                            allocator,
                            channel.capacity() + 1,
                            channel,
                        ) catch return SendError.SendFailed;

                        // Set the new channel as the active one and close the old one.
                        // Someone may already have closed the channel, so we check for that.
                        if (self.channel.active_channel.cmpxchgStrong(
                            state,
                            @intFromPtr(new_channel),
                            .release,
                            .monotonic,
                        ) != null) return SendError.Closed;
                        channel.close(lot);

                        // Migrate all elements to the new channel.
                        var default_rng = Random.DefaultPrng.init(@intCast(seed));
                        const rng = default_rng.random();
                        while (true) {
                            const m = (channel.receiver().tryRecv() catch break) orelse break;
                            new_channel.sender().trySendWithSeed(
                                lot,
                                rng.int(usize),
                                m,
                            ) catch unreachable;
                        }
                        continue;
                    },
                };
            }
        }

        /// Signals one waiting receiver of the channel.
        pub fn signal(self: Self, lot: *ParkingLot) void {
            while (true) {
                const state = self.channel.active_channel.load(.monotonic);
                const channel: *Inner = @ptrFromInt(state & channel_mask);
                channel.sender().signal(lot);
                if (state & channel_mask == self.channel.active_channel.load(.monotonic)) return;
            }
        }

        /// Signals all waiting receivers of the channel.
        pub fn broadcast(self: Self, lot: *ParkingLot) void {
            while (true) {
                const state = self.channel.active_channel.load(.monotonic);
                const channel: *Inner = @ptrFromInt(state & channel_mask);
                channel.sender().broadcast(lot);
                if (state & channel_mask == self.channel.active_channel.load(.monotonic)) return;
            }
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
        channel: *UnorderedSpmcChannel(T),

        const Self = @This();
        const Inner = UnorderedBoundedSpmcChannel(T);

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
            return self.tryRecvWithSeed(0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will return `null` immediately, if the channel is empty.
        /// If the channel is closed and empty, an error is returned.
        pub fn tryRecvWithSeed(self: Self, seed: usize) RecvError!?T {
            while (true) {
                const state = self.channel.active_channel.load(.acquire);

                // First try pulling an element from the active channel.
                const active_channel: *Inner = @ptrFromInt(state & channel_mask);
                const message = active_channel.receiver().tryRecvWithSeed(seed) catch {
                    const current_state = self.channel.active_channel.load(.monotonic);
                    if (current_state & channel_mask == @intFromPtr(active_channel) & channel_mask)
                        return RecvError.Closed;
                    return null;
                };
                return message;
            }
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recv(self: Self, lot: *ParkingLot) RecvError!T {
            return self.recvWithSeed(lot, 0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvWithSeed(self: Self, lot: *ParkingLot, seed: usize) RecvError!T {
            if (try self.tryRecvWithSeed(seed)) |msg| return msg;
            return self.recvSlow(lot, seed, null) catch |err| switch (err) {
                TimedRecvError.Closed => RecvError.Closed,
                TimedRecvError.Timeout => unreachable,
            };
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(self: Self, lot: *ParkingLot, duration: Duration) TimedRecvError!T {
            return self.recvForWithSeed(lot, duration, 0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvForWithSeed(
            self: Self,
            lot: *ParkingLot,
            duration: Duration,
            seed: usize,
        ) TimedRecvError!T {
            const timeout = Instant.now().addSaturating(duration);
            return self.recvUntilWithSeed(lot, timeout, seed);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(self: Self, lot: *ParkingLot, timeout: Instant) TimedRecvError!T {
            return self.recvUntilWithSeed(lot, timeout, 0);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntilWithSeed(
            self: Self,
            lot: *ParkingLot,
            timeout: Instant,
            seed: usize,
        ) TimedRecvError!T {
            if (try self.tryRecvWithSeed(seed)) |msg| return msg;
            return self.recvSlow(lot, seed, timeout);
        }

        fn recvSlow(self: Self, lot: *ParkingLot, seed: usize, timeout: ?Instant) TimedRecvError!T {
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
            const state = self.channel.active_channel.load(.monotonic);
            const active: *Inner = @ptrFromInt(state & channel_mask);
            std.debug.assert(active.receiver().parkChannel() == @as(*anyopaque, active));
            return active;
        }

        /// Returns the key used to park the receiver.
        pub fn parkKey(self: Self) *const anyopaque {
            const state = self.channel.active_channel.load(.monotonic);
            const active: *Inner = @ptrFromInt(state & channel_mask);
            return active.receiver().parkKey();
        }

        /// Prepares the receiver for parking.
        pub fn preparePark(self: Self, channel: *anyopaque) ParkError!void {
            const state = self.channel.active_channel.load(.monotonic);
            if (state & closed_bit != 0) return;

            const active: *Inner = @ptrFromInt(state & channel_mask);
            const rx = active.receiver();

            if (rx.parkChannel() != channel) return;
            return rx.preparePark(channel);
        }

        /// Checks whether the caller should park.
        pub fn shouldPark(self: Self, channel: *anyopaque) bool {
            const state = self.channel.active_channel.load(.monotonic);
            if (state & closed_bit != 0) return false;

            const active: *Inner = @ptrFromInt(state & channel_mask);
            const rx = active.receiver();

            if (rx.parkChannel() != channel) return false;
            return rx.shouldPark(channel);
        }

        /// Callback to handle a timeout while parked.
        pub fn onParkTimeout(self: Self, channel: *anyopaque, key: *const anyopaque, was_last: bool) void {
            _ = self;
            const park_channel: *Inner = @ptrCast(@alignCast(channel));
            park_channel.receiver().onParkTimeout(channel, key, was_last);
        }

        fn park(self: Self, lot: *ParkingLot, timeout: ?Instant) ParkError!void {
            const channel = self.parkChannel();
            try self.preparePark(channel);

            const Validation = struct {
                ctx: Self,
                channel: *anyopaque,
                fn f(this: @This()) bool {
                    return this.ctx.shouldPark(this.channel);
                }
            };
            const BeforeSleep = struct {
                fn f(this: @This()) void {
                    _ = this;
                }
            };
            const TimedOut = struct {
                ctx: Self,
                channel: *anyopaque,
                fn f(this: @This(), key: *const anyopaque, was_last: bool) void {
                    this.ctx.onParkTimeout(this.channel, key, was_last);
                }
            };
            const result = lot.park(
                self.parkKey(),
                Validation{ .ctx = self, .channel = channel },
                Validation.f,
                BeforeSleep{},
                BeforeSleep.f,
                TimedOut{ .ctx = self, .channel = channel },
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

fn channelTest(
    repeats: usize,
    num_threads: usize,
    num_messages: usize,
) !void {
    var lot = ParkingLot.init(std.testing.allocator);
    defer lot.deinit();

    for (0..repeats) |_| {
        var channel = try UnorderedSpmcChannel(usize).init(
            std.testing.allocator,
        );
        defer channel.deinit(std.testing.allocator);
        const sx = channel.sender(std.testing.allocator);

        const value_map = try std.testing.allocator.alignedAlloc(
            atomic.Value(bool),
            atomic.cache_line,
            num_messages,
        );
        defer std.testing.allocator.free(value_map);

        const Runner = struct {
            thread: std.Thread = undefined,
            rx: UnorderedSpmcChannel(usize).Receiver,
            lot: *ParkingLot,
            value_map: []align(atomic.cache_line) atomic.Value(bool),

            fn run(self: *@This()) void {
                while (true) {
                    const msg = self.rx.recv(self.lot) catch |e| switch (e) {
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
                .lot = &lot,
                .value_map = value_map,
            };
            r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
        }

        for (0..num_messages) |i| try sx.send(&lot, i);
        channel.close(&lot);

        for (runners) |r| r.thread.join();
        for (value_map) |v| try std.testing.expectEqual(true, v.load(.monotonic));
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
