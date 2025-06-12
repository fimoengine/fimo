const std = @import("std");

const intrusive_mpsc = @import("channel/intrusive_mpsc.zig");
pub const IntrusiveMpscChannel = intrusive_mpsc.IntrusiveMpscChannel;
const multi_receiver_ = @import("channel/multi_receiver.zig");
pub const MultiReceiver = multi_receiver_.MultiReceiver;
pub const multi_receiver = multi_receiver_.multi_receiver;
const receiver = @import("channel/receiver.zig");
pub const RecvError = receiver.RecvError;
pub const TimedRecvError = receiver.TimedRecvError;
pub const WaitError = receiver.WaitError;
pub const Receiver = receiver.Receiver;
const sender = @import("channel/sender.zig");
pub const TrySendError = sender.TrySendError;
pub const SendError = sender.SendError;
pub const Sender = sender.Sender;
const signal_mpsc = @import("channel/signal_mpsc.zig");
pub const SignalMpscChannel = signal_mpsc.SignalMpscChannel;
const unordered_bounded_spmc = @import("channel/unordered_bounded_spmc.zig");
pub const UnorderedBoundedSpmcChannel = unordered_bounded_spmc.UnorderedBoundedSpmcChannel;
const unordered_spmc = @import("channel/unordered_spmc.zig");
pub const UnorderedSpmcChannel = unordered_spmc.UnorderedSpmcChannel;

test {
    std.testing.refAllDeclsRecursive(intrusive_mpsc);
    std.testing.refAllDeclsRecursive(multi_receiver_);
    std.testing.refAllDeclsRecursive(receiver);
    std.testing.refAllDeclsRecursive(sender);
    std.testing.refAllDeclsRecursive(signal_mpsc);
    std.testing.refAllDeclsRecursive(unordered_bounded_spmc);
    std.testing.refAllDeclsRecursive(unordered_spmc);
}
