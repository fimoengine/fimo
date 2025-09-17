const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;

const Futex = @import("Futex.zig");

/// A thread-safe boolean that can be set and awaited,
/// typically to mark/await the completion of some operation.
pub const Fence = extern struct {
    state: atomic.Value(u8) = .init(unsignaled),

    pub const unsignaled: u8 = 0b00;
    pub const signaled: u8 = 0b01;
    pub const contended: u8 = 0b10;

    /// Checks if the fence is already signaled.
    pub fn isSignaled(self: *const Fence) bool {
        return self.state.load(.acquire) & signaled != 0;
    }

    /// Blocks the caller until the fence is signaled.
    pub fn wait(self: *Fence) void {
        if (!self.isSignaled()) {
            return self.waitInner(null) catch unreachable;
        }
    }

    /// Blocks the caller until the fence is signaled, or the timeout expires.
    pub fn timedWait(self: *Fence, timeout: Duration) error{Timeout}!void {
        if (!self.isSignaled()) {
            return self.waitInner(Instant.now().addSaturating(timeout));
        }
    }

    fn waitInner(self: *Fence, timeout: ?Instant) error{Timeout}!void {
        @branchHint(.cold);

        while (true) {
            const state = self.state.load(.monotonic);
            if (state & signaled != 0)
                if (self.state.load(.acquire) & signaled != 0) return;

            if (state & contended == 0) {
                if (self.state.cmpxchgWeak(
                    unsignaled,
                    contended,
                    .monotonic,
                    .monotonic,
                )) |_| continue;
            }

            if (timeout) |t|
                Futex.TypedHelper(u8).timedWait(&self.state, contended, 0, t) catch |err|
                    switch (err) {
                        error.Timeout => return error.Timeout,
                        else => {},
                    }
            else
                Futex.TypedHelper(u8).wait(&self.state, contended, 0) catch {};
        }
    }

    /// Wakes all waiters of the fence.
    pub fn signal(self: *Fence) void {
        const state = self.state.swap(signaled, .release);
        if (state & contended != 0) _ = Futex.wake(&self.state, std.math.maxInt(usize));
    }

    /// Resets the state of the fence to be unsignaled.
    ///
    /// May not be called while threads are waiting on the fence.
    pub fn reset(self: *Fence) void {
        const state = self.state.fetchAnd(~signaled, .release);
        std.debug.assert(state != (signaled | contended));
    }
};
