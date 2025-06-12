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
    pub fn close(self: *Self, futex: *Futex) void {
        const state = self.state.load(.monotonic);

        // If the channel is already closed we can skip waking up the consumers.
        if (state & closed_bit != 0) return;

        // Mark the channel as closed and wake all consumers.
        _ = self.state.fetchOr(closed_bit, .release);
        self.sender().broadcast(futex);
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
    pub fn trySend(self: Self, futex: *Futex, msg: void) TrySendError!void {
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
            self.signal(futex);
            return;
        }
    }

    /// Sends a message into the channel, blocking if necessary.
    pub fn send(self: Self, futex: *Futex, msg: void) SendError!void {
        while (true) {
            return self.trySend(futex, msg) catch |err| switch (err) {
                TrySendError.Closed => SendError.Closed,
                TrySendError.Full => continue,
                else => unreachable,
            };
        }
    }

    /// Signals one waiting receiver of the channel.
    pub fn signal(self: Self, futex: *Futex) void {
        // Do nothing if there are no waiters.
        if (self.channel.state.load(.monotonic) & waiting_bit == 0) return;
        self.notifySlow(futex, false);
    }

    /// Signals all waiting receivers of the channel.
    pub fn broadcast(self: Self, futex: *Futex) void {
        // Do nothing if there are no waiters.
        if (self.channel.state.load(.monotonic) & waiting_bit == 0) return;
        self.notifySlow(futex, true);
    }

    fn notifySlow(self: Self, futex: *Futex, all: bool) void {
        @branchHint(.cold);
        if (self.channel.state.fetchAnd(~waiting_bit, .monotonic) & waiting_bit == 0) return;
        _ = futex.wake(&self.channel.state, if (all) std.math.maxInt(usize) else 1);
    }

    fn genericTrySend(ctx: *anyopaque, futex: *Futex, msg: void) TrySendError!void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        try self.trySend(futex, msg);
    }

    fn genericSend(ctx: *anyopaque, futex: *Futex, msg: void) SendError!void {
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
                .prepareWait = &genericPrepareWait,
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
    pub fn recv(self: Self, futex: *Futex) RecvError!void {
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
    pub fn recvFor(self: Self, futex: *Futex, duration: Duration) TimedRecvError!void {
        const timeout = Instant.now().addSaturating(duration);
        return self.recvUntil(futex, timeout);
    }

    /// Receives one message from the channel.
    ///
    /// The caller will block until a message is available or the timeout is reached.
    /// If the channel is closed and empty, an error is returned.
    pub fn recvUntil(self: Self, futex: *Futex, timeout: Instant) TimedRecvError!void {
        if (try self.tryRecv()) |msg| return msg;
        return self.recvSlow(futex, timeout);
    }

    fn recvSlow(self: Self, futex: *Futex, timeout: ?Instant) TimedRecvError!void {
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
        const state = self.channel.state.load(.monotonic);
        if (state & signaled_bit != 0) return WaitError.Retry;
        if (state & closed_bit != 0) return WaitError.Retry;

        // Mark the channel as having a parked task.
        if (state & waiting_bit == 0) {
            if (self.channel.state.cmpxchgWeak(
                state,
                state | waiting_bit,
                .monotonic,
                .monotonic,
            )) |_| return WaitError.Retry;
        }

        return .{ .key = &self.channel.state, .key_size = @sizeOf(u8), .expect = waiting_bit };
    }

    fn wait(self: Self, futex: *Futex, timeout: ?Instant) WaitError!void {
        const k = try self.prepareWait();
        return futex.wait(k.key, k.key_size, k.expect, timeout) catch |err| switch (err) {
            error.Invalid => WaitError.Retry,
            error.Timeout => WaitError.Timeout,
        };
    }

    fn genericTryRecv(ctx: *anyopaque) RecvError!?void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.tryRecv();
    }

    fn genericRecv(ctx: *anyopaque, futex: *Futex) RecvError!void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.recv(futex);
    }

    fn genericRecvFor(ctx: *anyopaque, futex: *Futex, duration: Duration) TimedRecvError!void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.recvFor(futex, duration);
    }

    fn genericRecvUntil(ctx: *anyopaque, futex: *Futex, timeout: Instant) TimedRecvError!void {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.recvUntil(futex, timeout);
    }

    fn genericPrepareWait(ctx: *anyopaque) WaitError!Futex.KeyExpect {
        const self: *Self = @ptrCast(@alignCast(ctx));
        return self.prepareWait();
    }
};
