const ParkingLot = @import("../ParkingLot.zig");

pub const TrySendError = error{ Full, Closed, SendFailed };
pub const SendError = error{ Closed, SendFailed };

/// A generic sender for a channel.
pub fn Sender(comptime T: type) type {
    return struct {
        context: *anyopaque,
        vtable: *const VTable,

        pub const VTable = struct {
            trySend: *const fn (ctx: *anyopaque, lot: *ParkingLot, msg: T) TrySendError!void,
            send: *const fn (ctx: *anyopaque, lot: *ParkingLot, msg: T) SendError!void,
            signal: *const fn (ctx: *anyopaque, lot: *ParkingLot) void,
            broadcast: *const fn (ctx: *anyopaque, lot: *ParkingLot) void,
        };

        /// Tries to send a message into the channel without blocking.
        pub fn trySend(self: Sender(T), lot: *ParkingLot, msg: T) TrySendError!void {
            try self.vtable.trySend(self.context, lot, msg);
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn send(self: Sender(T), lot: *ParkingLot, msg: T) SendError!void {
            try self.vtable.send(self.context, lot, msg);
        }

        /// Wakes one waiting receiver of the channel.
        pub fn signal(self: Sender(T), lot: *ParkingLot) void {
            try self.vtable.signal(self.context, lot);
        }

        /// Wakes all waiting receiver of the channel.
        pub fn broadcast(self: Sender(T), lot: *ParkingLot) void {
            try self.vtable.broadcast(self.context, lot);
        }
    };
}
