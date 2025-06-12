const Futex = @import("../Futex.zig");

pub const TrySendError = error{ Full, Closed, SendFailed };
pub const SendError = error{ Closed, SendFailed };

/// A generic sender for a channel.
pub fn Sender(comptime T: type) type {
    return struct {
        context: *anyopaque,
        vtable: *const VTable,

        pub const VTable = struct {
            trySend: *const fn (ctx: *anyopaque, futex: *Futex, msg: T) TrySendError!void,
            send: *const fn (ctx: *anyopaque, futex: *Futex, msg: T) SendError!void,
            signal: *const fn (ctx: *anyopaque, futex: *Futex) void,
            broadcast: *const fn (ctx: *anyopaque, futex: *Futex) void,
        };

        /// Tries to send a message into the channel without blocking.
        pub fn trySend(self: Sender(T), futex: *Futex, msg: T) TrySendError!void {
            try self.vtable.trySend(self.context, futex, msg);
        }

        /// Sends a message into the channel, blocking if necessary.
        pub fn send(self: Sender(T), futex: *Futex, msg: T) SendError!void {
            try self.vtable.send(self.context, futex, msg);
        }

        /// Wakes one waiting receiver of the channel.
        pub fn signal(self: Sender(T), futex: *Futex) void {
            try self.vtable.signal(self.context, futex);
        }

        /// Wakes all waiting receiver of the channel.
        pub fn broadcast(self: Sender(T), futex: *Futex) void {
            try self.vtable.broadcast(self.context, futex);
        }
    };
}
