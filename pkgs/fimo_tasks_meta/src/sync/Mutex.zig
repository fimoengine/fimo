//! Mutex is a synchronization primitive which enforces atomic access to a
//! shared region of code known as the "critical section".
//!
//! It does this by blocking ensuring only one task is in the critical
//! section at any given point in time by blocking the others.
//!
//! Mutex can be statically initialized and is `@sizeOf(u8)` large.
//! Use `lock()` or `tryLock()` to enter the critical section and `unlock()` to leave it.
// Taken from https://github.com/rust-lang/rust/blob/master/library/std/src/sys/sync/mutex/futex.rs

const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Duration = time.Duration;
const Instant = time.Instant;

const root = @import("../root.zig");
const yield = root.yield;
const Task = root.Task;
const CmdBufCmd = root.CmdBufCmd;
const CmdBuf = root.CmdBuf;
const Executor = root.Executor;
const testing = @import("../testing.zig");
const Futex = @import("Futex.zig");

const Mutex = @This();

state: atomic.Value(u8) = .init(0),

const UNLOCKED: u8 = 0;
const LOCKED: u8 = 1; // locked, no other threads waiting
const CONTENDED: u8 = 2; // locked, and other threads waiting (contended)

/// Tries to acquire the mutex without blocking the caller's task.
///
/// Returns `false` if the calling task would have to block to acquire it.
/// Otherwise, returns `true` and the caller should `unlock()` the Mutex to release it.
pub fn tryLock(self: *Mutex) bool {
    return self.state.cmpxchgStrong(UNLOCKED, LOCKED, .acquire, .monotonic) == null;
}

/// Acquires the mutex, blocking the caller's task until it can.
///
/// Once acquired, call `unlock()` on the Mutex to release it.
pub fn lock(self: *Mutex) void {
    if (self.state.cmpxchgStrong(UNLOCKED, LOCKED, .acquire, .monotonic)) |_|
        self.lockContended(null) catch unreachable;
}

/// Tries to acquire the mutex, blocking the caller's task until it can or the timeout is reached.
///
/// Once acquired, call `unlock()` on the Mutex to release it.
pub fn timedLock(self: *Mutex, timeout: Duration) error{Timeout}!void {
    if (self.state.cmpxchgStrong(UNLOCKED, LOCKED, .acquire, .monotonic)) |_|
        try self.lockContended(Instant.now().addSaturating(timeout));
}

fn lockContended(self: *Mutex, timeout: ?Instant) error{Timeout}!void {
    @branchHint(.cold);
    // Spin first to speed things up if the lock is released quickly.
    var state = self.spin();

    // If it's unlocked now, attempt to take the lock
    // without marking it as contended.
    if (state == UNLOCKED) state = self.state.cmpxchgStrong(
        UNLOCKED,
        LOCKED,
        .acquire,
        .monotonic,
    ) orelse return;

    while (true) {
        // Put the lock in contended state.
        // We avoid an unnecessary write if it as already set to CONTENDED,
        // to be friendlier for the caches.
        if (state != CONTENDED and self.state.swap(CONTENDED, .acquire) == UNLOCKED) {
            // We changed it from UNLOCKED to CONTENDED, so we just successfully locked it.
            return;
        }

        // Wait for the futex to change state, assuming it is still CONTENDED.
        if (timeout) |t|
            Futex.TypedHelper(u8).timedWait(&self.state, CONTENDED, 0, t) catch |err| switch (err) {
                error.Timeout => return error.Timeout,
                error.Invalid => {},
            }
        else
            Futex.TypedHelper(u8).wait(&self.state, CONTENDED, 0) catch {};

        // Spin again after waking up.
        state = self.spin();
    }
}

fn spin(self: *Mutex) u8 {
    var s: u8 = 100;
    while (true) {
        // We only use `load` (and not `swap` or `compare_exchange`)
        // while spinning, to be easier on the caches.
        const state = self.state.load(.monotonic);

        // We stop spinning when the mutex is UNLOCKED,
        // but also when it's CONTENDED.
        if (state != LOCKED or s == 0) {
            return state;
        }

        atomic.spinLoopHint();
        s -= 1;
    }
}

/// Releases the mutex which was previously acquired.
pub fn unlock(self: *Mutex) void {
    if (self.state.swap(UNLOCKED, .release) == CONTENDED)
        // We only wake up one thread. When that thread locks the mutex, it
        // will mark the mutex as CONTENDED (see lockContended above),
        // which makes sure that any other waiting threads will also be
        // woken up eventually.
        self.wake();
}

fn wake(self: *Mutex) void {
    _ = Futex.wake(&self.state, 1);
}

test "Mutex: smoke test (threads)" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    var mutex: Mutex = .{};

    try std.testing.expect(mutex.tryLock());
    try std.testing.expect(!mutex.tryLock());
    mutex.unlock();

    mutex.lock();
    try std.testing.expect(!mutex.tryLock());
    mutex.unlock();
}

test "Mutex: smoke test (tasks)" {
    try testing.initTestContextInTask(struct {
        fn f() !void {
            var mutex: Mutex = .{};

            try std.testing.expect(mutex.tryLock());
            try std.testing.expect(!mutex.tryLock());
            mutex.unlock();

            mutex.lock();
            try std.testing.expect(!mutex.tryLock());
            mutex.unlock();
        }
    }.f);
}

// A counter which is incremented without atomic instructions
const NonAtomicCounter = struct {
    // direct u128 could maybe use xmm ops on x86 which are atomic
    value: [2]u64 = [_]u64{ 0, 0 },

    fn get(self: NonAtomicCounter) u128 {
        return @as(u128, @bitCast(self.value));
    }

    fn inc(self: *NonAtomicCounter) void {
        for (@as([2]u64, @bitCast(self.get() + 1)), 0..) |v, i| {
            @as(*volatile u64, @ptrCast(&self.value[i])).* = v;
        }
    }
};

test "Mutex: many uncontended (threads)" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const num_threads = 4;
    const num_increments = 1000;

    const Runner = struct {
        mutex: Mutex = .{},
        thread: std.Thread = undefined,
        counter: NonAtomicCounter = .{},

        fn run(self: *@This()) void {
            var i: usize = num_increments;
            while (i > 0) : (i -= 1) {
                self.mutex.lock();
                defer self.mutex.unlock();

                self.counter.inc();
            }
        }
    };

    var runners = [_]Runner{.{}} ** num_threads;
    for (&runners) |*r| r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
    for (runners) |r| r.thread.join();
    for (runners) |r| try std.testing.expectEqual(r.counter.get(), num_increments);
}

test "Mutex: many uncontended (tasks)" {
    try testing.initTestContextInTask(struct {
        fn f() !void {
            const num_threads = 4;
            const num_increments = 1000;

            const Runner = struct {
                mutex: Mutex = .{},
                counter: NonAtomicCounter = .{},
                task: Task = .{ .run = run },

                fn run(task: *Task, idx: usize) callconv(.c) void {
                    _ = idx;
                    const self: *@This() = @alignCast(@fieldParentPtr("task", task));
                    var i: usize = num_increments;
                    while (i > 0) : (i -= 1) {
                        self.mutex.lock();
                        defer self.mutex.unlock();
                        self.counter.inc();
                    }
                }
            };
            var runners = [_]Runner{.{}} ** num_threads;
            var cmds: [num_threads]CmdBufCmd = undefined;
            for (&runners, &cmds) |*r, *cmd| cmd.* = .{
                .tag = .enqueue_task,
                .payload = .{ .enqueue_task = &r.task },
            };
            var cmd_buf = CmdBuf{ .cmds = .init(&cmds) };

            const exe = Executor.current().?;
            const handle = exe.enqueue(&cmd_buf);

            try std.testing.expectEqual(.completed, handle.join());
            for (runners) |r| try std.testing.expectEqual(r.counter.get(), num_increments);
        }
    }.f);
}

test "Mutex: many contended (threads)" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const num_threads = 4;
    const num_increments = 1000;

    const Runner = struct {
        mutex: Mutex = .{},
        counter: NonAtomicCounter = .{},

        fn run(self: *@This()) void {
            var i: usize = num_increments;
            while (i > 0) : (i -= 1) {
                // Occasionally hint to let another thread run.
                defer if (i % 100 == 0) std.Thread.yield() catch {};

                self.mutex.lock();
                defer self.mutex.unlock();

                self.counter.inc();
            }
        }
    };

    var runner = Runner{};

    var threads: [num_threads]std.Thread = undefined;
    for (&threads) |*t| t.* = try std.Thread.spawn(.{}, Runner.run, .{&runner});
    for (threads) |t| t.join();

    try std.testing.expectEqual(runner.counter.get(), num_increments * num_threads);
}

test "Mutex: many contended (tasks)" {
    try testing.initTestContextInTask(struct {
        fn f() !void {
            const num_threads = 4;
            const num_increments = 1000;

            const Runner = struct {
                mutex: Mutex = .{},
                counter: NonAtomicCounter = .{},
                task: Task = .{ .batch_len = num_threads, .run = run },

                fn run(task: *Task, idx: usize) callconv(.c) void {
                    _ = idx;
                    const self: *@This() = @alignCast(@fieldParentPtr("task", task));
                    var i: usize = num_increments;
                    while (i > 0) : (i -= 1) {
                        // Occasionally hint to let another thread run.
                        defer if (i % 100 == 0) yield();

                        self.mutex.lock();
                        defer self.mutex.unlock();

                        self.counter.inc();
                    }
                }
            };
            var runner = Runner{};
            var cmd = CmdBufCmd{
                .tag = .enqueue_task,
                .payload = .{ .enqueue_task = &runner.task },
            };
            var cmd_buf = CmdBuf{ .cmds = .init(@ptrCast(&cmd)) };

            const exe = Executor.current().?;
            const handle = exe.enqueue(&cmd_buf);

            try std.testing.expectEqual(.completed, handle.join());
            try std.testing.expectEqual(runner.counter.get(), num_increments * num_threads);
        }
    }.f);
}
