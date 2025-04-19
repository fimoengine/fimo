const std = @import("std");

const Self = @This();

count: std.atomic.Value(usize) = std.atomic.Value(usize).init(1),

/// Increase the reference count.
pub fn ref(self: *Self) void {
    _ = self.count.fetchAdd(1, .monotonic);
}

/// Decreases the reference count.
pub fn unref(self: *Self) enum { noop, deinit } {
    const count = self.count.fetchSub(1, .release);
    if (count != 1) return .noop;

    // The acquire load ensures that no other thread is using the value.
    _ = self.count.load(.acquire);
    return .deinit;
}
