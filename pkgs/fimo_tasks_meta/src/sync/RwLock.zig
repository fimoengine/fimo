//! Reader-writer lock with upgrade capabilities.
// Taken from https://github.com/Amanieu/parking_lot/blob/87ce756554e7a89808b22333563beb5d338bc572/src/raw_rwlock.rs

const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;

const task = @import("../task.zig");
const yield = task.yield;
const testing = @import("../testing.zig");
const ParkingLot = @import("ParkingLot.zig");
const ParkToken = ParkingLot.ParkToken;
const UnparkToken = ParkingLot.UnparkToken;

const Self = @This();

state: atomic.Value(usize) = .init(0),

// There is at least one task in the main queue.
const parked_bit: usize = 0b0001;
// There is a parked task holding writer_bit. writer_bit must be set.
const writer_parked_bit: usize = 0b0010;
// A reader is holding an upgradable lock. The reader count must be non-zero and
// writer_bit must not be set.
const upgradable_bit: usize = 0b0100;
// If the reader count is zero: a writer is currently holding an exclusive lock.
// Otherwise: a writer is waiting for the remaining readers to exit the lock.
const writer_bit: usize = 0b1000;
// Mask of bits used to count readers.
const readers_mask: usize = ~@as(usize, 0b1111);
// Base unit for counting readers.
const one_reader: usize = 0b10000;

// Token idicating what type of lock a queued task is trying to acquire.
const token_shared: ParkToken = @enumFromInt(one_reader);
const token_exclusive: ParkToken = @enumFromInt(writer_bit);
const token_upgradable: ParkToken = @enumFromInt(one_reader | upgradable_bit);

const token_handoff: UnparkToken = @enumFromInt(1);

/// Blocks until exclusive lock ownership is acquired.
pub fn lockExclusive(self: *Self, provider: anytype) void {
    if (self.state.cmpxchgWeak(0, writer_bit, .acquire, .monotonic)) |_| {
        self.lockExclusiveSlow(provider);
    }
}

fn lockExclusiveSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    const TryLock = struct {
        fn f(this: *Self, state: *usize) bool {
            while (true) {
                if (state.* & (writer_bit | upgradable_bit) != 0) return false;

                // Grab writer_bit if it isn't set, even if there are parked tasks.
                state.* = this.state.cmpxchgWeak(
                    state.*,
                    state.* | writer_bit,
                    .acquire,
                    .monotonic,
                ) orelse return true;
            }
        }
    };

    // Step 1: grab exclusive ownership of writer_bit.
    self.lockCommon(provider, token_shared, self, TryLock.f, writer_bit);

    // Step 2: wait for all remaining readers to exit the lock.
    self.waitForReaders(provider);
}

/// Attempts to obtain exclusive lock ownership.
/// Returns `true` if the lock is obtained, `false` otherwise.
pub fn tryLockExclusive(self: *Self) bool {
    return self.state.cmpxchgWeak(0, writer_bit, .acquire, .monotonic) == null;
}

/// Releases a held exclusive lock. Asserts the lock is held exclusively.
pub fn unlockExclusive(self: *Self, provider: anytype) void {
    if (self.state.cmpxchgWeak(writer_bit, 0, .release, .monotonic)) |_| {
        self.unlockExclusiveSlow(provider);
    }
}

fn unlockExclusiveSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    // There are tasks to unpark. Try to unpark as many as we can.
    const Callback = struct {
        ptr: *Self,
        fn f(
            this: @This(),
            new_state: usize,
            result: ParkingLot.UnparkResult,
        ) ParkingLot.UnparkToken {
            // If we are using a fair unlock then we should keep the
            // rwlock locked and hand it off to the unparked task.
            var new_s = new_state;
            if (result.unparked_tasks != 0 and result.be_fair) {
                if (result.has_more_tasks) {
                    new_s |= parked_bit;
                }
                this.ptr.state.store(new_s, .release);
                return token_handoff;
            } else {
                // Clear the parked bit if there are no more parked tasks.
                if (result.has_more_tasks) {
                    this.ptr.state.store(parked_bit, .release);
                } else {
                    this.ptr.state.store(0, .release);
                }
                return .default;
            }
        }
    };
    self.wakeParkedTasks(provider, 0, Callback{ .ptr = self }, Callback.f);
}

/// Obtains shared lock ownership. Blocks if another thread has exclusive ownership.
/// May block if another thread is attempting to get exclusive ownership.
pub fn lockShared(self: *Self, provider: anytype) void {
    if (!self.tryLockSharedFast()) {
        self.lockSharedSlow(provider);
    }
}

fn lockSharedSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    const TryLock = struct {
        fn f(this: *Self, state: *usize) bool {
            var spin_count: usize = 0;
            const spin_limit = 10;
            while (true) {
                // This is the same condition as tryLockSharedFast
                if (state.* & writer_bit != 0) return false;
                _ = this.state.cmpxchgWeak(
                    state.*,
                    state.* + one_reader,
                    .acquire,
                    .monotonic,
                ) orelse return true;
                if (spin_count < spin_limit) {
                    spin_count += 1;
                    for (0..(@as(usize, 1) << @truncate(spin_count))) |_| std.atomic.spinLoopHint();
                }
                state.* = this.state.load(.monotonic);
            }
        }
    };
    self.lockCommon(provider, token_shared, self, TryLock.f, writer_bit);
}

/// Attempts to obtain shared lock ownership.
/// Returns `true` if the lock is obtained, `false` otherwise.
pub fn tryLockShared(self: *Self) bool {
    if (self.tryLockSharedFast()) return true;
    return self.tryLockSharedSlow();
}

fn tryLockSharedFast(self: *Self) bool {
    const state = self.state.load(.monotonic);

    // We can't allow grabbing a shared lock if there is a writer, even if
    // the writer is still waiting for the remaining readers to exit.
    if (state & writer_bit != 0) return false;
    const new_state = std.math.add(usize, state, one_reader) catch return false;
    return self.state.cmpxchgWeak(state, new_state, .acquire, .monotonic) == null;
}

fn tryLockSharedSlow(self: *Self) bool {
    @branchHint(.cold);
    var state = self.state.load(.monotonic);
    while (true) {
        // This mirrors the condition in tryLockSharedFast
        if (state & writer_bit != 0) return false;
        state = self.state.cmpxchgWeak(
            state,
            state + one_reader,
            .acquire,
            .monotonic,
        ) orelse return true;
    }
}

/// Releases a held shared lock.
pub fn unlockShared(self: *Self, provider: anytype) void {
    const state = self.state.fetchSub(one_reader, .release);
    if (state & (readers_mask | writer_parked_bit) == (one_reader | writer_parked_bit)) {
        self.unlockSharedSlow(provider);
    }
}

fn unlockSharedSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    const Callback = struct {
        ptr: *Self,
        fn f(this: *@This(), result: ParkingLot.UnparkResult) ParkingLot.UnparkToken {
            _ = result;
            _ = this.ptr.state.fetchAnd(~writer_parked_bit, .monotonic);
            return .default;
        }
    };
    // At this point writer_parked_bit is set and reader_mask is empty. We
    // just need to wake up a potentially sleeping pending writer.
    // Using the 2nd key at addr + 1.
    _ = ParkingLot.unparkOne(
        provider,
        @ptrFromInt(@intFromPtr(self) + 1),
        Callback{ .ptr = self },
        Callback.f,
    );
}

/// Obtains upgradable lock ownership.
/// Blocks if another task has exclusive or upgradable ownership.
/// May block if another thread is attempting to get exclusive ownership.
pub fn lockUpgradable(self: *Self, provider: anytype) void {
    if (!self.tryLockUpgradableFast()) {
        self.lockUpgradableSlow(provider);
    }
}

fn lockUpgradableSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    const TryLock = struct {
        fn f(this: *Self, state: *usize) bool {
            var spin_count: usize = 0;
            const spin_limit = 10;
            while (true) {
                if (state.* & (writer_bit | upgradable_bit) != 0) return false;
                _ = this.state.cmpxchgWeak(
                    state.*,
                    state.* + (one_reader | upgradable_bit),
                    .acquire,
                    .monotonic,
                ) orelse return true;
                if (spin_count < spin_limit) {
                    spin_count += 1;
                    for (0..(@as(usize, 1) << @truncate(spin_count))) |_| std.atomic.spinLoopHint();
                }
                state.* = this.state.load(.monotonic);
            }
        }
    };
    self.lockCommon(provider, token_upgradable, self, TryLock.f, writer_bit | upgradable_bit);
}

/// Attempts to obtain upgradable lock ownership.
/// Returns `true` if the lock is obtained, `false` otherwise.
pub fn tryLockUpgradable(self: *Self) bool {
    if (self.tryLockUpgradableFast()) return true;
    return self.tryLockUpgradableSlow();
}

fn tryLockUpgradableFast(self: *Self) bool {
    const state = self.state.load(.monotonic);
    // We can't grab an upgradable lock if there is already a writer or upgradable reader.
    if (state & (writer_bit | upgradable_bit) != 0) return false;

    const new_state = std.math.add(usize, state, one_reader | upgradable_bit) catch return false;
    return self.state.cmpxchgWeak(state, new_state, .acquire, .monotonic) == null;
}

fn tryLockUpgradableSlow(self: *Self) bool {
    @branchHint(.cold);
    var state = self.state.load(.monotonic);
    while (true) {
        // This mirrors the condition in tryLockUpgradableFast.
        if (state & (writer_bit | upgradable_bit) != 0) return false;
        state = self.state.cmpxchgWeak(
            state,
            state + (one_reader | upgradable_bit),
            .acquire,
            .monotonic,
        ) orelse return true;
    }
}

/// Releases a held upgradable lock.
pub fn unlockUpgradable(self: *Self, provider: anytype) void {
    const state = self.state.load(.monotonic);
    if (state & parked_bit == 0) {
        if (self.state.cmpxchgWeak(
            state,
            state - (one_reader | upgradable_bit),
            .release,
            .monotonic,
        ) == null) return;
    }
    self.unlockUpgradableSlow(provider);
}

fn unlockUpgradableSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    // Just release the lock if there are no parked tasks.
    var state = self.state.load(.monotonic);
    while (state & parked_bit == 0) {
        state = self.state.cmpxchgWeak(
            state,
            state - (one_reader | upgradable_bit),
            .release,
            .monotonic,
        ) orelse return;
    }

    // There are tasks to unpark. Try to unpark as many as we can.
    const Callback = struct {
        ptr: *Self,
        fn f(
            this: @This(),
            new_state: usize,
            result: ParkingLot.UnparkResult,
        ) ParkingLot.UnparkToken {
            // If we are using a fair unlock then we should keep the
            // rwlock locked and hand it off to the unparked tasks.
            var s = this.ptr.state.load(.monotonic);
            if (result.be_fair) {
                // Fall back to normal unpark on overflow.
                while (std.math.add(
                    usize,
                    s - (one_reader | upgradable_bit),
                    new_state,
                ) catch null) |n| {
                    var ns = n;
                    if (result.has_more_tasks) {
                        ns |= parked_bit;
                    } else {
                        ns &= ~parked_bit;
                    }
                    s = this.ptr.state.cmpxchgWeak(
                        s,
                        ns,
                        .monotonic,
                        .monotonic,
                    ) orelse return token_handoff;
                }
            }

            // Otherwise just release the upgradable lock and update parked_bit.
            while (true) {
                var ns = s - (one_reader | upgradable_bit);
                if (result.has_more_tasks) {
                    ns |= parked_bit;
                } else {
                    ns &= ~parked_bit;
                }
                s = this.ptr.state.cmpxchgWeak(
                    s,
                    ns,
                    .monotonic,
                    .monotonic,
                ) orelse return .default;
            }
        }
    };
    self.wakeParkedTasks(provider, 0, Callback{ .ptr = self }, Callback.f);
}

/// Upgrades a held upgradable lock to an exclusive lock.
pub fn upgradeToExclusive(self: *Self, provider: anytype) void {
    const state = self.state.fetchSub((one_reader | upgradable_bit) - writer_bit, .acquire);
    if (state & readers_mask != one_reader) {
        self.upgradeToExclusiveSlow(provider);
    }
}

fn upgradeToExclusiveSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    self.waitForReaders(provider);
}

/// Attempts to upgrade a held upgradable lock to an exclusive lock without blocking.
pub fn tryUpgradeToExclusive(self: *Self) bool {
    if (self.state.cmpxchgWeak(
        one_reader | upgradable_bit,
        writer_bit,
        .acquire,
        .monotonic,
    ) == null) return true;
    return self.tryUpgradeToExclusiveSlow();
}

fn tryUpgradeToExclusiveSlow(self: *Self) bool {
    @branchHint(.cold);
    var state = self.state.load(.monotonic);
    while (true) {
        if (state & readers_mask != one_reader) return false;
        state = self.state.cmpxchgWeak(
            state,
            state - (one_reader | upgradable_bit) + writer_bit,
            .monotonic,
            .monotonic,
        ) orelse return true;
    }
}

/// Downgrades a held exclusive lock to a shared lock.
pub fn downgradeToShared(self: *Self, provider: anytype) void {
    const state = self.state.fetchAdd(one_reader - writer_bit, .release);
    if (state & parked_bit != 0) self.downgradeToSharedSlow(provider);
}

fn downgradeToSharedSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    // There are tasks to unpark. Try to unpark as many as we can.
    const Callback = struct {
        ptr: *Self,
        fn f(
            this: @This(),
            new_state: usize,
            result: ParkingLot.UnparkResult,
        ) ParkingLot.UnparkToken {
            _ = new_state;
            if (!result.has_more_tasks) {
                _ = this.ptr.state.fetchAnd(~parked_bit, .monotonic);
            }
            return .default;
        }
    };
    self.wakeParkedTasks(
        provider,
        one_reader,
        Callback{ .ptr = self },
        Callback.f,
    );
}

/// Downgrades a held exclusive lock to an upgradable lock.
pub fn downgradeToUpgradable(self: *Self, provider: anytype) void {
    const state = self.state.fetchAdd((one_reader | upgradable_bit) - writer_bit, .release);
    if (state & parked_bit != 0) self.downgradeToUpgradableSlow(provider);
}

fn downgradeToUpgradableSlow(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    // There are tasks to unpark. Try to unpark as many as we can.
    const Callback = struct {
        ptr: *Self,
        fn f(
            this: @This(),
            new_state: usize,
            result: ParkingLot.UnparkResult,
        ) ParkingLot.UnparkToken {
            _ = new_state;
            if (!result.has_more_tasks) {
                _ = this.ptr.state.fetchAnd(~parked_bit, .monotonic);
            }
            return .default;
        }
    };
    self.wakeParkedTasks(
        provider,
        one_reader | upgradable_bit,
        Callback{ .ptr = self },
        Callback.f,
    );
}

/// Downgrades a held upgradable lock to a shared lock.
pub fn downgradeUpgradableToShared(self: *Self, provider: anytype) void {
    const state = self.state.fetchSub(upgradable_bit, .monotonic);
    if (state & parked_bit != 0) self.downgradeToSharedSlow(provider);
}

fn wakeParkedTasks(
    self: *Self,
    provider: anytype,
    new_state: usize,
    callback_args: anytype,
    callback: fn (@TypeOf(callback_args), usize, ParkingLot.UnparkResult) ParkingLot.UnparkToken,
) void {
    // We must wake up at least one upgrader or writer if there is one,
    // otherwise they may end up parked indefinitely since unlock_shared
    // does not call wake_parked_threads.
    const Filter = struct {
        new_state: *usize,
        fn f(this: *@This(), token: ParkToken) ParkingLot.FilterOp {
            const s = this.new_state.*;

            // If we are waking up a writer, don't wake anything else.
            if (s & writer_bit != 0) return .stop;

            // Otherwise wake *all* readers and one upgrader/writer.
            if (@intFromEnum(token) & (upgradable_bit | writer_bit) != 0 and
                s & upgradable_bit != 0)
            {
                // Skip writers and upgradable readers if we already have
                // a writer/upgradable reader.
                return .skip;
            } else {
                this.new_state.* = s + @intFromEnum(token);
                return .unpark;
            }
        }
    };
    const Callback = struct {
        new_state: *usize,
        args: @TypeOf(callback_args),
        fn f(this: *@This(), result: ParkingLot.UnparkResult) ParkingLot.UnparkToken {
            return callback(this.args, this.new_state.*, result);
        }
    };
    var new_s = new_state;
    _ = ParkingLot.unparkFilter(
        provider,
        self,
        Filter{ .new_state = &new_s },
        Filter.f,
        Callback{ .new_state = &new_s, .args = callback_args },
        Callback.f,
    );
}

fn waitForReaders(
    self: *Self,
    provider: anytype,
) void {
    // At this point writer_bit is already set, we just need to wait for the
    // remaining readers to exit the lock.
    var spin_count: usize = 0;
    const spin_yield_limit: usize = 3;
    const spin_limit: usize = 10;
    var state = self.state.load(.acquire);
    while (state & readers_mask != 0) {
        if (spin_count < spin_limit) {
            spin_count += 1;
            if (spin_count < spin_yield_limit) {
                for (0..(@as(usize, 1) << @truncate(spin_count))) |_| atomic.spinLoopHint();
            } else yield(provider);
            state = self.state.load(.acquire);
            continue;
        }

        // Set the parked bit.
        if (state & writer_parked_bit == 0) {
            if (self.state.cmpxchgWeak(
                state,
                state | writer_parked_bit,
                .acquire,
                .acquire,
            )) |s| {
                state = s;
                continue;
            }
        }

        const Validation = struct {
            ptr: *Self,
            fn f(this: *@This()) bool {
                const s = this.ptr.state.load(.monotonic);
                return (s & readers_mask != 0) and s & writer_parked_bit != 0;
            }
        };
        const BeforeSleep = struct {
            fn f(this: *@This()) void {
                _ = this;
            }
        };
        const TimedOut = struct {
            fn f(this: *@This(), key: *const anyopaque, is_last: bool) void {
                _ = this;
                _ = key;
                _ = is_last;
                unreachable;
            }
        };
        const result = ParkingLot.park(
            provider,
            @ptrFromInt(@intFromPtr(self) + 1),
            Validation{ .ptr = self },
            Validation.f,
            BeforeSleep{},
            BeforeSleep.f,
            TimedOut{},
            TimedOut.f,
            token_exclusive,
            null,
        );
        switch (result.type) {
            // We still need to re-check the state if we are unparked
            // since a previous writer timing-out could have allowed
            // another reader to sneak in before we parked.
            .unparked, .invalid => state = self.state.load(.acquire),
            // Timeout is not possible.
            .timed_out => unreachable,
        }
    }
}

fn lockCommon(
    self: *Self,
    provider: anytype,
    token: ParkToken,
    try_lock_args: anytype,
    try_lock: fn (@TypeOf(try_lock_args), *usize) bool,
    validate_flags: usize,
) void {
    var spin_count: usize = 0;
    const spin_yield_limit: usize = 3;
    const spin_limit: usize = 10;
    var state = self.state.load(.monotonic);
    while (true) {
        // Attempt to grab the lock.
        if (try_lock(try_lock_args, &state)) return;

        // If there are no parked tasks, try spinning a few times.
        if (state & (parked_bit | writer_parked_bit) == 0 and spin_count < spin_limit) {
            spin_count += 1;
            if (spin_count < spin_yield_limit) {
                for (0..(@as(usize, 1) << @truncate(spin_count))) |_| atomic.spinLoopHint();
            } else yield(provider);
            state = self.state.load(.monotonic);
            continue;
        }

        // Set the parked bit.
        if (state & parked_bit == 0) {
            if (self.state.cmpxchgWeak(state, state | parked_bit, .monotonic, .monotonic)) |s| {
                state = s;
                continue;
            }
        }

        // Park our task until we are woken up by an unlock.
        const Validation = struct {
            ptr: *Self,
            validate_flags: usize,
            fn f(this: *@This()) bool {
                const s = this.ptr.state.load(.monotonic);
                return (s & parked_bit != 0) and (s & this.validate_flags != 0);
            }
        };
        const BeforeSleep = struct {
            fn f(this: *@This()) void {
                _ = this;
            }
        };
        const TimedOut = struct {
            fn f(this: *@This(), key: *const anyopaque, is_last: bool) void {
                _ = this;
                _ = key;
                _ = is_last;
                unreachable;
            }
        };
        const result = ParkingLot.park(
            provider,
            self,
            Validation{ .ptr = self, .validate_flags = validate_flags },
            Validation.f,
            BeforeSleep{},
            BeforeSleep.f,
            TimedOut{},
            TimedOut.f,
            token,
            null,
        );
        switch (result.type) {
            // We were unparked. Return if the lock was passed to us.
            .unparked => if (result.token == token_handoff) return,
            // The validation failed, retry.
            .invalid => {},
            // Timeout is not possible.
            .timed_out => unreachable,
        }

        spin_count = 0;
        state = self.state.load(.monotonic);
    }
}

test "smoke test (threads)" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    var rwl = Self{};

    rwl.lockExclusive(ctx);
    try std.testing.expect(!rwl.tryLockExclusive());
    try std.testing.expect(!rwl.tryLockShared());
    try std.testing.expect(!rwl.tryLockUpgradable());
    rwl.unlockExclusive(ctx);

    try std.testing.expect(rwl.tryLockExclusive());
    try std.testing.expect(!rwl.tryLockShared());
    try std.testing.expect(!rwl.tryLockUpgradable());
    rwl.unlockExclusive(ctx);

    rwl.lockShared(ctx);
    try std.testing.expect(!rwl.tryLockExclusive());
    try std.testing.expect(rwl.tryLockShared());
    try std.testing.expect(rwl.tryLockUpgradable());
    rwl.unlockUpgradable(ctx);
    rwl.unlockShared(ctx);
    rwl.unlockShared(ctx);

    try std.testing.expect(rwl.tryLockShared());
    try std.testing.expect(!rwl.tryLockExclusive());
    try std.testing.expect(rwl.tryLockShared());
    try std.testing.expect(rwl.tryLockUpgradable());
    rwl.unlockUpgradable(ctx);
    rwl.unlockShared(ctx);
    rwl.unlockShared(ctx);

    rwl.lockExclusive(ctx);
    rwl.unlockExclusive(ctx);
}

test "smoke test (tasks)" {
    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) !void {
            _ = err;

            var rwl = Self{};

            rwl.lockExclusive(ctx);
            try std.testing.expect(!rwl.tryLockExclusive());
            try std.testing.expect(!rwl.tryLockShared());
            try std.testing.expect(!rwl.tryLockUpgradable());
            rwl.unlockExclusive(ctx);

            try std.testing.expect(rwl.tryLockExclusive());
            try std.testing.expect(!rwl.tryLockShared());
            try std.testing.expect(!rwl.tryLockUpgradable());
            rwl.unlockExclusive(ctx);

            rwl.lockShared(ctx);
            try std.testing.expect(!rwl.tryLockExclusive());
            try std.testing.expect(rwl.tryLockShared());
            try std.testing.expect(rwl.tryLockUpgradable());
            rwl.unlockUpgradable(ctx);
            rwl.unlockShared(ctx);
            rwl.unlockShared(ctx);

            try std.testing.expect(rwl.tryLockShared());
            try std.testing.expect(!rwl.tryLockExclusive());
            try std.testing.expect(rwl.tryLockShared());
            try std.testing.expect(rwl.tryLockUpgradable());
            rwl.unlockUpgradable(ctx);
            rwl.unlockShared(ctx);
            rwl.unlockShared(ctx);

            rwl.lockExclusive(ctx);
            rwl.unlockExclusive(ctx);
        }
    }.f);
}
