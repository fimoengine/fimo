const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;

const Futex = @import("../Futex.zig");

pub const RecvError = error{Closed};
pub const TimedRecvError = error{ Closed, Timeout };
pub const WaitError = error{ Timeout, Retry };

/// A generic receiver for a channel.
pub fn Receiver(comptime T: type) type {
    return struct {
        context: *anyopaque,
        vtable: *const VTable,

        pub const VTable = struct {
            tryRecv: *const fn (ctx: *anyopaque) RecvError!?T,
            recv: *const fn (ctx: *anyopaque, futex: *Futex) RecvError!T,
            recvFor: *const fn (ctx: *anyopaque, futex: *Futex, duration: Duration) TimedRecvError!T,
            recvUntil: *const fn (ctx: *anyopaque, futex: *Futex, timeout: Instant) TimedRecvError!T,
        };

        /// Receives one message from the channel.
        ///
        /// The caller will return `null` immediately, if the channel is empty.
        /// If the channel is closed and empty, an error is returned.
        pub fn tryRecv(self: Receiver(T)) RecvError!?T {
            return self.vtable.tryRecv(self.context);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recv(self: Receiver(T), futex: *Futex) RecvError!T {
            return self.vtable.recv(self.context, futex);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(self: Receiver(T), futex: *Futex, duration: Duration) TimedRecvError!T {
            return self.vtable.recvFor(self.context, futex, duration);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(self: Receiver(T), futex: *Futex, timeout: Instant) TimedRecvError!T {
            return self.vtable.recvUntil(self.context, futex, timeout);
        }
    };
}

/// A generic receiver for a channel that can be awaited.
pub fn WaitableReceiver(comptime T: type) type {
    return struct {
        context: *anyopaque,
        vtable: *const VTable,

        pub const VTable = struct {
            tryRecv: *const fn (ctx: *anyopaque) RecvError!?T,
            recv: *const fn (ctx: *anyopaque, futex: *Futex) RecvError!T,
            recvFor: *const fn (ctx: *anyopaque, futex: *Futex, duration: Duration) TimedRecvError!T,
            recvUntil: *const fn (ctx: *anyopaque, futex: *Futex, timeout: Instant) TimedRecvError!T,

            prepareWait: *const fn (ctx: *anyopaque) WaitError!Futex.KeyExpect,
        };

        /// Receives one message from the channel.
        ///
        /// The caller will return `null` immediately, if the channel is empty.
        /// If the channel is closed and empty, an error is returned.
        pub fn tryRecv(self: WaitableReceiver(T)) RecvError!?T {
            return self.vtable.tryRecv(self.context);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recv(self: WaitableReceiver(T), futex: *Futex) RecvError!T {
            return self.vtable.recv(self.context, futex);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(
            self: WaitableReceiver(T),
            futex: *Futex,
            duration: Duration,
        ) TimedRecvError!T {
            return self.vtable.recvFor(self.context, futex, duration);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(
            self: WaitableReceiver(T),
            futex: *Futex,
            timeout: Instant,
        ) TimedRecvError!T {
            return self.vtable.recvUntil(self.context, futex, timeout);
        }

        /// Prepares the receiver for waiting.
        pub fn prepareWait(self: WaitableReceiver(T)) WaitError!Futex.KeyExpect {
            return self.vtable.prepareWait(self.context);
        }
    };
}
