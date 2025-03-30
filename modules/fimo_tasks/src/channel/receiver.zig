const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;

const ParkingLot = @import("../ParkingLot.zig");

pub const RecvError = error{Closed};
pub const TimedRecvError = error{ Closed, Timeout };
pub const ParkError = error{ Timeout, Retry };

/// A generic receiver for a channel.
pub fn Receiver(comptime T: type) type {
    return struct {
        context: *anyopaque,
        vtable: *const VTable,

        pub const VTable = struct {
            tryRecv: *const fn (ctx: *anyopaque) RecvError!?T,
            recv: *const fn (ctx: *anyopaque, lot: *ParkingLot) RecvError!T,
            recvFor: *const fn (ctx: *anyopaque, lot: *ParkingLot, duration: Duration) TimedRecvError!T,
            recvUntil: *const fn (ctx: *anyopaque, lot: *ParkingLot, timeout: Instant) TimedRecvError!T,
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
        pub fn recv(self: Receiver(T), lot: *ParkingLot) RecvError!T {
            return self.vtable.recv(self.context, lot);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(self: Receiver(T), lot: *ParkingLot, duration: Duration) TimedRecvError!T {
            return self.vtable.recvFor(self.context, lot, duration);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(self: Receiver(T), lot: *ParkingLot, timeout: Instant) TimedRecvError!T {
            return self.vtable.recvUntil(self.context, lot, timeout);
        }
    };
}

/// A generic receiver for a channel that can be parked.
pub fn ParkableReceiver(comptime T: type) type {
    return struct {
        context: *anyopaque,
        vtable: *const VTable,

        pub const VTable = struct {
            tryRecv: *const fn (ctx: *anyopaque) RecvError!?T,
            recv: *const fn (ctx: *anyopaque, lot: *ParkingLot) RecvError!T,
            recvFor: *const fn (ctx: *anyopaque, lot: *ParkingLot, duration: Duration) TimedRecvError!T,
            recvUntil: *const fn (ctx: *anyopaque, lot: *ParkingLot, timeout: Instant) TimedRecvError!T,
            parkChannel: *const fn (ctx: *anyopaque) *anyopaque,
            parkKey: *const fn (ctx: *anyopaque) *const anyopaque,
            preparePark: *const fn (ctx: *anyopaque, channel: *anyopaque) ParkError!void,
            shouldPark: *const fn (ctx: *anyopaque, channel: *anyopaque) bool,
            onParkTimeout: *const fn (
                ctx: *anyopaque,
                channel: *anyopaque,
                key: *const anyopaque,
                was_last: bool,
            ) void,
        };

        /// Receives one message from the channel.
        ///
        /// The caller will return `null` immediately, if the channel is empty.
        /// If the channel is closed and empty, an error is returned.
        pub fn tryRecv(self: ParkableReceiver(T)) RecvError!?T {
            return self.vtable.tryRecv(self.context);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available.
        /// If the channel is closed and empty, an error is returned.
        pub fn recv(self: ParkableReceiver(T), lot: *ParkingLot) RecvError!T {
            return self.vtable.recv(self.context, lot);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the specified duration has elapsed.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvFor(
            self: ParkableReceiver(T),
            lot: *ParkingLot,
            duration: Duration,
        ) TimedRecvError!T {
            return self.vtable.recvFor(self.context, lot, duration);
        }

        /// Receives one message from the channel.
        ///
        /// The caller will block until a message is available or the timeout is reached.
        /// If the channel is closed and empty, an error is returned.
        pub fn recvUntil(
            self: ParkableReceiver(T),
            lot: *ParkingLot,
            timeout: Instant,
        ) TimedRecvError!T {
            return self.vtable.recvUntil(self.context, lot, timeout);
        }

        /// Returns the channel used to park the receiver.
        pub fn parkChannel(self: ParkableReceiver(T)) *anyopaque {
            return self.vtable.parkChannel(self.context);
        }

        /// Returns the key used to park the receiver.
        pub fn parkKey(self: ParkableReceiver(T)) *const anyopaque {
            return self.vtable.parkKey(self.context);
        }

        /// Prepares the receiver for parking.
        pub fn preparePark(self: ParkableReceiver(T), channel: *anyopaque) ParkError!void {
            try self.vtable.preparePark(self.context, channel);
        }

        /// Checks whether the caller should park.
        pub fn shouldPark(self: ParkableReceiver(T), channel: *anyopaque) bool {
            return self.vtable.shouldPark(self.context, channel);
        }

        /// Callback to handle a timeout while parked.
        pub fn onParkTimeout(
            self: ParkableReceiver(T),
            channel: *anyopaque,
            key: *const anyopaque,
            was_last: bool,
        ) void {
            self.vtable.onParkTimeout(self.context, channel, key, was_last);
        }
    };
}
