//! Reader-writer lock.
// Taken from https://github.com/rust-lang/rust/blob/master/library/std/src/sys/sync/rwlock/futex.rs

const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;

const Futex = @import("Futex.zig");

const Self = @This();

// The state consists of a 30-bit reader counter, a 'readers waiting' flag, and a 'writers waiting' flag.
// Bits 0..30:
//   0: Unlocked
//   1..=0x3FFF_FFFE: Locked by N readers
//   0x3FFF_FFFF: Write locked
// Bit 30: Readers are waiting on this futex.
// Bit 31: Writers are waiting on the writer_notify futex.
state: atomic.Value(u32) = .init(0),
// The 'condition variable' to notify writers through.
// Incremented on every signal.
writer_notify: atomic.Value(u32) = .init(0),

const READ_LOCKED: u32 = 1;
const MASK: u32 = (1 << 30) - 1;
const WRITE_LOCKED: u32 = MASK;
const DOWNGRADE: u32 = READ_LOCKED -% WRITE_LOCKED;
const MAX_READERS: u32 = MASK - 1;
const READERS_WAITING: u32 = 1 << 30;
const WRITERS_WAITING: u32 = 1 << 31;

fn isUnlocked(state: u32) bool {
    return state & MASK == 0;
}

fn isWriteLocked(state: u32) bool {
    return state & MASK == WRITE_LOCKED;
}

fn hasReadersWaiting(state: u32) bool {
    return state & READERS_WAITING != 0;
}

fn hasWritersWaiting(state: u32) bool {
    return state & WRITERS_WAITING != 0;
}

fn isReadLockable(state: u32) bool {
    // This also returns false if the counter could overflow if we tried to read lock it.
    //
    // We don't allow read-locking if there's readers waiting, even if the lock is unlocked
    // and there's no writers waiting. The only situation when this happens is after unlocking,
    // at which point the unlocking thread might be waking up writers, which have priority over readers.
    // The unlocking thread will clear the readers waiting bit and wake up readers, if necessary.
    return state & MASK < MAX_READERS and !hasReadersWaiting(state) and !hasWritersWaiting(state);
}

fn isReadLockableAfterWakeup(state: u32) bool {
    // We make a special case for checking if we can read-lock _after_ a reader thread that went to
    // sleep has been woken up by a call to `downgrade`.
    //
    // `downgrade` will wake up all readers and place the lock in read mode. Thus, there should be
    // no readers waiting and the lock should be read-locked (not write-locked or unlocked).
    //
    // Note that we do not check if any writers are waiting. This is because a call to `downgrade`
    // implies that the caller wants other readers to read the value protected by the lock. If we
    // did not allow readers to acquire the lock before writers after a `downgrade`, then only the
    // original writer would be able to read the value, thus defeating the purpose of `downgrade`.
    return state & MASK < MAX_READERS and
        !hasReadersWaiting(state) and
        !isWriteLocked(state) and
        !isUnlocked(state);
}

fn hasReachedMaxReaders(state: u32) bool {
    return state & MASK == MAX_READERS;
}

/// Attempts to obtain shared lock ownership.
/// Returns `true` if the lock is obtained, `false` otherwise.
pub fn tryLockRead(self: *Self) bool {
    var state = self.state.load(.acquire);
    while (true) {
        if (!isReadLockable(state)) return false;
        state = self.state.cmpxchgWeak(
            state,
            state + READ_LOCKED,
            .acquire,
            .monotonic,
        ) orelse return true;
    }
}

/// Obtains shared lock ownership. Blocks if another thread has exclusive ownership.
/// May block if another thread is attempting to get exclusive ownership.
pub fn lockRead(self: *Self, provider: anytype) void {
    const state = self.state.load(.monotonic);
    if (!isReadLockable(state) or self.state.cmpxchgWeak(
        state,
        state + READ_LOCKED,
        .acquire,
        .monotonic,
    ) != null) self.lockReadContended(provider);
}

/// Releases a held shared lock.
pub fn unlockRead(self: *Self, provider: anytype) void {
    const state = self.state.fetchSub(READ_LOCKED, .release) - READ_LOCKED;

    // It's impossible for a reader to be waiting on a read-locked RwLock,
    // except if there is also a writer waiting.
    std.debug.assert(!hasReadersWaiting(state) or hasWritersWaiting(state));

    // Wake up a writer if we were the last reader and there's a writer waiting.
    if (isUnlocked(state) and hasWritersWaiting(state)) self.wakeWriterOrReaders(provider, state);
}

fn lockReadContended(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    var has_slept = false;
    var state = self.spinRead();

    while (true) {
        // If we have just been woken up, first check for a `downgrade` call.
        // Otherwise, if we can read-lock it, lock it.
        if ((has_slept and isReadLockableAfterWakeup(state)) or isReadLockable(state)) {
            state = self.state.cmpxchgWeak(
                state,
                state + READ_LOCKED,
                .acquire,
                .monotonic,
            ) orelse return;
            continue;
        }

        // Check for overflow.
        if (hasReachedMaxReaders(state)) @panic("too many active read locks on RwLock");

        // Make sure the readers waiting bit is set before we go to sleep.
        if (!hasReadersWaiting(state)) {
            if (self.state.cmpxchgWeak(state, state | READERS_WAITING, .monotonic, .monotonic)) |s| {
                state = s;
                continue;
            }
        }

        // Wait for the state to change.
        Futex.TypedHelper(u32).wait(provider, &self.state, state | READERS_WAITING, 0) catch {};
        has_slept = true;

        // Spin again after waking up.
        state = self.spinRead();
    }
}

/// Attempts to obtain exclusive lock ownership.
/// Returns `true` if the lock is obtained, `false` otherwise.
pub fn tryLockWrite(self: *Self) bool {
    var state = self.state.load(.acquire);
    while (true) {
        if (!isUnlocked(state)) return false;
        state = self.state.cmpxchgWeak(
            state,
            state + WRITE_LOCKED,
            .acquire,
            .monotonic,
        ) orelse return true;
    }
}

/// Blocks until exclusive lock ownership is acquired.
pub fn lockWrite(self: *Self, provider: anytype) void {
    if (self.state.cmpxchgWeak(0, WRITE_LOCKED, .acquire, .monotonic)) |_|
        self.lockWriteContended(provider);
}

/// Releases a held exclusive lock. Asserts the lock is held exclusively.
pub fn unlockWrite(self: *Self, provider: anytype) void {
    const state = self.state.fetchSub(WRITE_LOCKED, .release) - WRITE_LOCKED;

    std.debug.assert(isUnlocked(state));

    if (hasWritersWaiting(state) or hasReadersWaiting(state))
        self.wakeWriterOrReaders(provider, state);
}

/// Downgrades a held exclusive lock to a shared lock.
pub fn downgrade(self: *Self, provider: anytype) void {
    // Removes all write bits and adds a single read bit.
    const state = self.state.fetchAdd(DOWNGRADE, .release);
    std.debug.assert(isWriteLocked(state));

    if (hasReadersWaiting(state)) {
        // Since we had the exclusive lock, nobody else can unset this bit.
        _ = self.state.fetchSub(READERS_WAITING, .monotonic);
        _ = Futex.wake(provider, &self.state, std.math.maxInt(usize));
    }
}

fn lockWriteContended(self: *Self, provider: anytype) void {
    @branchHint(.cold);
    var state = self.spinWrite();
    var other_writers_waiting: u32 = 0;

    while (true) {
        // If it's unlocked, we try to lock it.
        if (isUnlocked(state)) {
            state = self.state.cmpxchgWeak(
                state,
                state | WRITE_LOCKED | other_writers_waiting,
                .acquire,
                .monotonic,
            ) orelse return;
            continue;
        }

        // Set the waiting bit indicating that we're waiting on it.
        if (!hasWritersWaiting(state)) {
            if (self.state.cmpxchgWeak(state, state | WRITERS_WAITING, .monotonic, .monotonic)) |s| {
                state = s;
                continue;
            }
        }

        // Other writers might be waiting now too, so we should make sure
        // we keep that bit on once we manage lock it.
        other_writers_waiting = WRITERS_WAITING;

        // Examine the notification counter before we check if `state` has changed,
        // to make sure we don't miss any notifications.
        const seq = self.writer_notify.load(.acquire);

        // Don't go to sleep if the lock has become available,
        // or if the writers waiting bit is no longer set.
        state = self.state.load(.monotonic);
        if (isUnlocked(state) or !hasWritersWaiting(state)) continue;

        // Wait for the state to change.
        Futex.TypedHelper(u32).wait(provider, &self.writer_notify, seq, 0) catch {};

        // Spin again after waking up.
        state = self.spinWrite();
    }
}

/// Wakes up waiting threads after unlocking.
///
/// If both are waiting, this will wake up only one writer, but will fall
/// back to waking up readers if there was no writer to wake up.
fn wakeWriterOrReaders(self: *Self, provider: anytype, s: u32) void {
    @branchHint(.cold);
    var state = s;
    if (!isUnlocked(state)) @panic("expected an unlocked RwLock");

    // The readers waiting bit might be turned on at any point now,
    // since readers will block when there's anything waiting.
    // Writers will just lock the lock though, regardless of the waiting bits,
    // so we don't have to worry about the writer waiting bit.
    //
    // If the lock gets locked in the meantime, we don't have to do
    // anything, because then the thread that locked the lock will take
    // care of waking up waiters when it unlocks.

    // If only writers are waiting, wake one of them up.
    if (state == WRITERS_WAITING) {
        state = self.state.cmpxchgWeak(state, 0, .monotonic, .monotonic) orelse {
            _ = self.wakeWriter(provider);
            return;
        };
    }

    // If both writers and readers are waiting, leave the readers waiting
    // and only wake up one writer.
    if (state == READERS_WAITING + WRITERS_WAITING) {
        if (self.state.cmpxchgWeak(state, READERS_WAITING, .monotonic, .monotonic)) |_| return;
        if (self.wakeWriter(provider)) return;
        // No writers were actually blocked on futex_wait, so we continue
        // to wake up readers instead, since we can't be sure if we notified a writer.
        state = READERS_WAITING;
    }

    // If readers are waiting, wake them all up.
    if (state == READERS_WAITING) {
        if (self.state.cmpxchgWeak(state, 0, .monotonic, .monotonic) == null)
            _ = Futex.wake(provider, &self.state, std.math.maxInt(usize));
    }
}

/// This wakes one writer and returns true if we woke up a writer that was
/// blocked on futex_wait.
///
/// If this returns false, it might still be the case that we notified a
/// writer that was about to go to sleep.
fn wakeWriter(self: *Self, provider: anytype) bool {
    _ = self.writer_notify.fetchAdd(1, .release);
    return Futex.wake(provider, &self.writer_notify, 1) != 0;
}

/// Spin for a while, but stop directly at the given condition.
fn spinUntil(self: *Self, f: fn (state: u32) bool) u32 {
    var spin: u8 = 100;
    while (true) {
        const state = self.state.load(.monotonic);
        if (f(state) or spin == 0) return state;
        std.atomic.spinLoopHint();
        spin -= 1;
    }
}

fn spinWrite(self: *Self) u32 {
    // Stop spinning when it's unlocked or when there's waiting writers, to keep things somewhat fair.
    return self.spinUntil(struct {
        fn f(state: u32) bool {
            return isUnlocked(state) or hasWritersWaiting(state);
        }
    }.f);
}

fn spinRead(self: *Self) u32 {
    // Stop spinning when it's unlocked or read locked, or when there's waiting threads.
    return self.spinUntil(struct {
        fn f(state: u32) bool {
            return !isWriteLocked(state) or hasReadersWaiting(state) or hasWritersWaiting(state);
        }
    }.f);
}

test "RwLock: smoke test (tasks)" {
    const testing = @import("../testing.zig");

    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) !void {
            _ = err;

            var rwl = Self{};

            rwl.lockWrite(ctx);
            try std.testing.expect(!rwl.tryLockWrite());
            try std.testing.expect(!rwl.tryLockRead());
            rwl.unlockWrite(ctx);

            try std.testing.expect(rwl.tryLockWrite());
            try std.testing.expect(!rwl.tryLockRead());
            rwl.unlockWrite(ctx);

            rwl.lockRead(ctx);
            try std.testing.expect(!rwl.tryLockWrite());
            try std.testing.expect(rwl.tryLockRead());
            rwl.unlockRead(ctx);
            rwl.unlockRead(ctx);

            try std.testing.expect(rwl.tryLockRead());
            try std.testing.expect(!rwl.tryLockWrite());
            try std.testing.expect(rwl.tryLockRead());
            rwl.unlockRead(ctx);
            rwl.unlockRead(ctx);

            rwl.lockWrite(ctx);
            rwl.unlockWrite(ctx);
        }
    }.f);
}

test "RwLock: concurrent access (threads)" {
    const testing = @import("../testing.zig");

    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const num_writers: usize = 2;
    const num_readers: usize = 4;
    const num_writes: usize = 10000;
    const num_reads: usize = num_writes * 2;

    const Runner = struct {
        ctx: *const testing.TestContext,
        rwl: Self = .{},
        writes: usize = 0,
        reads: std.atomic.Value(usize) = std.atomic.Value(usize).init(0),

        term1: usize = 0,
        term2: usize = 0,
        term_sum: usize = 0,

        fn reader(self: *@This()) !void {
            while (true) {
                self.rwl.lockRead(self.ctx);
                defer self.rwl.unlockRead(self.ctx);

                if (self.writes >= num_writes or self.reads.load(.unordered) >= num_reads)
                    break;

                try self.check();

                _ = self.reads.fetchAdd(1, .monotonic);
            }
        }

        fn writer(self: *@This(), thread_idx: usize) !void {
            var prng = std.Random.DefaultPrng.init(thread_idx);
            var rnd = prng.random();

            while (true) {
                self.rwl.lockWrite(self.ctx);
                defer self.rwl.unlockWrite(self.ctx);

                if (self.writes >= num_writes)
                    break;

                try self.check();

                const term1 = rnd.int(usize);
                self.term1 = term1;
                try std.Thread.yield();

                const term2 = rnd.int(usize);
                self.term2 = term2;
                try std.Thread.yield();

                self.term_sum = term1 +% term2;
                self.writes += 1;
            }
        }

        fn check(self: *const @This()) !void {
            const term_sum = self.term_sum;
            try std.Thread.yield();

            const term2 = self.term2;
            try std.Thread.yield();

            const term1 = self.term1;
            try std.testing.expectEqual(term_sum, term1 +% term2);
        }
    };

    var runner = Runner{ .ctx = &ctx };
    var threads: [num_writers + num_readers]std.Thread = undefined;

    for (threads[0..num_writers], 0..) |*t, i| t.* = try std.Thread.spawn(.{}, Runner.writer, .{ &runner, i });
    for (threads[num_writers..]) |*t| t.* = try std.Thread.spawn(.{}, Runner.reader, .{&runner});

    for (threads) |t| t.join();

    try std.testing.expectEqual(num_writes, runner.writes);
}

test "RwLock: concurrent access (tasks)" {
    const task = @import("../task.zig");
    const yield = task.yield;
    const pool = @import("../pool.zig");
    const future = @import("../future.zig");
    const testing = @import("../testing.zig");

    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) !void {
            const executor = pool.Pool.current(ctx).?;
            defer executor.unref();

            const num_writers: usize = 2;
            const num_readers: usize = 4;
            const num_writes: usize = 10000;
            const num_reads: usize = num_writes * 2;

            const Runner = struct {
                ctx: *const testing.TestContext,
                rwl: Self = .{},
                writes: usize = 0,
                reads: std.atomic.Value(usize) = std.atomic.Value(usize).init(0),

                term1: usize = 0,
                term2: usize = 0,
                term_sum: usize = 0,

                fn reader(self: *@This()) void {
                    while (true) {
                        self.rwl.lockRead(self.ctx);
                        defer self.rwl.unlockRead(self.ctx);

                        if (self.writes >= num_writes or self.reads.load(.unordered) >= num_reads)
                            break;

                        self.check();

                        _ = self.reads.fetchAdd(1, .monotonic);
                    }
                }

                fn writer(self: *@This(), thread_idx: usize) void {
                    var prng = std.Random.DefaultPrng.init(thread_idx);
                    var rnd = prng.random();

                    while (true) {
                        self.rwl.lockWrite(self.ctx);
                        defer self.rwl.unlockWrite(self.ctx);

                        if (self.writes >= num_writes)
                            break;

                        self.check();

                        const term1 = rnd.int(usize);
                        self.term1 = term1;
                        yield(self.ctx);

                        const term2 = rnd.int(usize);
                        self.term2 = term2;
                        yield(self.ctx);

                        self.term_sum = term1 +% term2;
                        self.writes += 1;
                    }
                }

                fn check(self: *const @This()) void {
                    const term_sum = self.term_sum;
                    yield(self.ctx);

                    const term2 = self.term2;
                    yield(self.ctx);

                    const term1 = self.term1;
                    std.testing.expectEqual(term_sum, term1 +% term2) catch unreachable;
                }
            };

            var runner = Runner{ .ctx = ctx };
            var futures: [num_readers + num_writers]future.Future(void) = undefined;

            for (futures[0..num_writers], 0..) |*fu, i| fu.* = try future.init(
                executor,
                Runner.writer,
                .{ &runner, i },
                .{ .allocator = std.testing.allocator },
                err,
            );
            for (futures[num_writers..]) |*fu| fu.* = try future.init(
                executor,
                Runner.reader,
                .{&runner},
                .{ .allocator = std.testing.allocator },
                err,
            );

            for (futures) |fu| {
                _ = fu.@"await"() catch {};
                fu.deinit();
            }

            try std.testing.expectEqual(num_writes, runner.writes);
        }
    }.f);
}
