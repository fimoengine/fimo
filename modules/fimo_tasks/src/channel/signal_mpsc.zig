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
const receiver = @import("receiver.zig");
const GenericReceiver = receiver.ParkableReceiver;
const RecvError = receiver.RecvError;
const TimedRecvError = receiver.TimedRecvError;
const ParkError = receiver.ParkError;
const sender = @import("sender.zig");
const TrySendError = sender.TrySendError;
const SendError = sender.SendError;
const GenericSender = sender.Sender;

const closed_bit: u8 = 0b001;
const waiting_bit: u8 = 0b010;
const signaled_bit: u8 = 0b100;

pub const SignalMpscChannel = struct {
    state: atomic.Value(u8) = .init(0),

    const Self = @This();
    pub const Sender = SenderImpl;
    pub const Receiver = ReceiverImpl;

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
        const state = self.state.load(.monotonic);
        return state & closed_bit != 0;
    }

    /// Closes the channel.
    ///
    /// After closing the channel, no more messages can be sent into it.
    /// All blocked receivers will be woken up and receive an error.
    pub fn close(self: *Self, lot: *ParkingLot) void {
        const state = self.state.load(.monotonic);

        // If the channel is already closed we can skip waking up the consumers.
        if (state & closed_bit != 0) return;

        // Mark the channel as closed and wake all consumers.
        _ = self.state.fetchOr(closed_bit, .release);
        self.sender().broadcast(lot);
    }
};

const SenderImpl = struct {
    channel: *SignalMpscChannel,

    const Self = @This();

    /// Returns a generic sender backed by this sender.
    pub fn sender(self: *Self) GenericSender(void) {
        return GenericSender(void){
            .context = self,
            .vtable = &GenericSender(void).VTable{
                .trySend = &genericTrySend,
                .send = &genericSend,
                .signal = &genericSignal,
                .broadcast = &genericBroadcast,
            },
        };
    }

    /// Tries to send a message into the channel without blocking.
    pub fn trySend(self: Self, lot: *ParkingLot, msg: void) TrySendError!void {
        _ = msg;

        const channel = self.channel;
        while (true) {
            const state = channel.state.load(.monotonic);

            // If the queue is closed we are not allowed to send any messages.
            if (state & closed_bit != 0) return TrySendError.Closed;

            // If the channel was already signaled we are done.
            if (state & signaled_bit != 0) return TrySendError.Full;

            if (channel.state.cmpxchgWeak(
                state,
                state | signaled_bit,
                .release,
                .monotonic,
            ) != null) continue;

            // Signal the receiver that there is a new message available.
            self.signal(lot);
            return;
        }
    }

    /// Sends a message into the channel, blocking if necessary.
    pub fn send(self: Self, lot: *ParkingLot, msg: void) SendError!void {
        while (true) {
            return self.trySend(lot, msg) catch |err| switch (err) {
                TrySendError.Closed => SendError.Closed,
                TrySendError.Full => continue,
                else => unreachable,
            };
        }
    }

    /// Signals one waiting receiver of the channel.
    pub fn signal(self: Self, lot: *ParkingLot) void {
        // Do nothing if there are no waiters.
        if (self.channel.state.load(.monotonic) & waiting_bit == 0) return;
        self.notifySlow(lot, false);
    }

    /// Signals all waiting receivers of the channel.
    pub fn broadcast(self: Self, lot: *ParkingLot) void {
        // Do nothing if there are no waiters.
        if (self.channel.state.load(.monotonic) & waiting_bit == 0) return;
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
                if (!result.has_more_tasks) _ = this.ctx.channel.state.fetchAnd(
                    ~waiting_bit,
                    .monotonic,
                );
                return .default;
            }
        };

        _ = lot.unparkRequeue(
            &self.channel.state,
            &self.channel.state,
            Validate{ .all = all },
            Validate.f,
            Callback{ .ctx = self },
            Callback.f,
        );
    }

    fn genericTrySend(ctx: *anyopaque, lot: *ParkingLot, msg: void) TrySendError!void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        try self.trySend(lot, msg);
    }

    fn genericSend(ctx: *anyopaque, lot: *ParkingLot, msg: void) SendError!void {
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

const ReceiverImpl = struct {
    channel: *SignalMpscChannel,

    const Self = @This();

    /// Returns a generic receiver backed by this receiver.
    pub fn receiver(self: *Self) GenericReceiver(void) {
        return GenericReceiver(void){
            .context = self,
            .vtable = &GenericReceiver(void).VTable{
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
    pub fn tryRecv(self: Self) RecvError!?void {
        const channel = self.channel;

        const state = channel.state.fetchAnd(~signaled_bit, .acquire);
        if (state & signaled_bit != 0) return;
        if (state & closed_bit != 0) return RecvError.Closed;
        return null;
    }

    /// Receives one message from the channel.
    ///
    /// The caller will block until a message is available.
    /// If the channel is closed and empty, an error is returned.
    pub fn recv(self: Self, lot: *ParkingLot) RecvError!void {
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
    pub fn recvFor(self: Self, lot: *ParkingLot, duration: Duration) TimedRecvError!void {
        const timeout = Instant.now().addSaturating(duration);
        return self.recvUntil(lot, timeout);
    }

    /// Receives one message from the channel.
    ///
    /// The caller will block until a message is available or the timeout is reached.
    /// If the channel is closed and empty, an error is returned.
    pub fn recvUntil(self: Self, lot: *ParkingLot, timeout: Instant) TimedRecvError!void {
        if (try self.tryRecv()) |msg| return msg;
        return self.recvSlow(lot, timeout);
    }

    fn recvSlow(self: Self, lot: *ParkingLot, timeout: ?Instant) TimedRecvError!void {
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
        return &self.channel.state;
    }

    /// Prepares the receiver for parking.
    pub fn preparePark(self: Self, channel: *anyopaque) ParkError!void {
        std.debug.assert(@as(*anyopaque, self.channel) == channel);
        const state = self.channel.state.load(.monotonic);
        if (state & signaled_bit != 0) return;
        if (state & closed_bit != 0) return;

        // Mark the channel as having a parked task.
        if (state & waiting_bit == 0) {
            if (self.channel.state.cmpxchgWeak(
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
        return self.channel.state.load(.monotonic) & ~closed_bit == waiting_bit;
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
            _ = self.channel.state.fetchAnd(~waiting_bit, .monotonic);
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

    fn genericTryRecv(ctx: *anyopaque) RecvError!?void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.tryRecv();
    }

    fn genericRecv(ctx: *anyopaque, lot: *ParkingLot) RecvError!void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.recv(lot);
    }

    fn genericRecvFor(ctx: *anyopaque, lot: *ParkingLot, duration: Duration) TimedRecvError!void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.recvFor(lot, duration);
    }

    fn genericRecvUntil(ctx: *anyopaque, lot: *ParkingLot, timeout: Instant) TimedRecvError!void {
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
