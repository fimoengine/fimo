const std = @import("std");
const Futex = std.Thread.Futex;

const Self = @This();

count: std.atomic.Value(u32) = .init(0),

const is_waiting: u32 = 1 << 31;
const count_mask: u32 = ~is_waiting;

pub fn increase(self: *Self) void {
    const old = self.count.fetchAdd(1, .monotonic);
    std.debug.assert(old & count_mask != std.math.maxInt(u31));
}

pub fn decrease(self: *Self) void {
    const old = self.count.fetchSub(1, .release);
    std.debug.assert(old & count_mask != 0);
    if (old - 1 == is_waiting) Futex.wake(&self.count, 1);
}

pub fn waitUntilZero(self: *Self) void {
    var count = self.count.load(.monotonic);
    while (true) {
        if (count & count_mask == 0) {
            _ = self.count.load(.acquire);
            return;
        }
        if (count & is_waiting == 0) {
            if (self.count.cmpxchgWeak(count, count | is_waiting, .monotonic, .monotonic)) |old| {
                count = old;
                continue;
            }
            count |= is_waiting;
        }
        Futex.wait(&self.count, count);
        count = self.count.load(.monotonic);
    }
    _ = self.count.bitReset(31, .monotonic);
}
