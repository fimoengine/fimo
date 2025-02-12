//! Mutex is a synchronization primitive which enforces atomic access to a
//! shared region of code known as the "critical section".
//!
//! It does this by blocking ensuring only one task is in the critical
//! section at any given point in time by blocking the others.
//!
//! Mutex can be statically initialized and is `@sizeOf(u8)` large.
//! Use `lock()` or `tryLock()` to enter the critical section and `unlock()` to leave it.

const std = @import("std");
const atomic = std.atomic;

const fimo_std = @import("fimo_std");
const AnyError = fimo_std.AnyError;
const time = fimo_std.time;
const Duration = time.Duration;
const Time = time.Time;

const task = @import("../task.zig");
const Task = task.OpaqueTask;
const TaskId = task.Id;
const testing = @import("../testing.zig");
const ParkingLot = @import("ParkingLot.zig");

const Mutex = @This();

impl: BargingLock = .{},

/// Tries to acquire the mutex without blocking the caller's task.
///
/// Returns `false` if the calling task would have to block to acquire it.
/// Otherwise, returns `true` and the caller should `unlock()` the Mutex to release it.
pub fn tryLock(self: *Mutex) bool {
    return self.impl.tryLock();
}

/// Acquires the mutex, blocking the caller's task until it can.
///
/// Once acquired, call `unlock()` or `unlockFair()` on the Mutex to release it.
pub fn lock(self: *Mutex, provider: anytype) void {
    self.impl.lock(provider);
}

/// Tries to acquire the mutex, blocking the caller's task until it can or the timeout is reached.
///
/// Once acquired, call `unlock()` or `unlockFair()` on the Mutex to release it.
pub fn timedLock(self: *Mutex, provider: anytype, timeout: Duration) error{Timeout}!void {
    self.impl.timedLock(provider, timeout);
}

/// Releases the mutex which was previously acquired.
pub fn unlock(self: *Mutex, provider: anytype) void {
    self.impl.unlock(provider);
}

/// Releases the mutex which was previously acquired with a fair unlocking mechanism.
pub fn unlockFair(self: *Mutex, provider: anytype) void {
    self.impl.unlockFair(provider);
}

/// BargingLock from the WebKit blog post with spinning in the slow path.
const BargingLock = extern struct {
    state: atomic.Value(u8) = .init(unlocked),

    const unlocked: u8 = 0b00;
    const is_locked_bit: u8 = 0b01;
    const has_parked_bit: u8 = 0b10;

    pub const handoff_token: ParkingLot.UnparkToken = @enumFromInt(1);

    inline fn tryLock(self: *BargingLock) bool {
        var state = self.state.load(.monotonic);
        while (state & is_locked_bit == 0) {
            state = self.state.cmpxchgWeak(
                state,
                state | is_locked_bit,
                .acquire,
                .monotonic,
            ) orelse return true;
        }
        return false;
    }

    inline fn lock(self: *BargingLock, provider: anytype) void {
        if (self.state.cmpxchgWeak(unlocked, is_locked_bit, .acquire, .monotonic) != null) {
            _ = self.lockSlow(provider, null);
        }
    }

    inline fn timedLock(
        self: *BargingLock,
        provider: anytype,
        timeout: Duration,
    ) error{Timeout}!void {
        if (self.state.cmpxchgWeak(unlocked, is_locked_bit, .acquire, .monotonic) != null) {
            const timeout_time = Time.now().addSaturating(timeout);
            if (!self.lockSlow(provider, timeout_time)) return error.Timeout;
        }
    }

    inline fn unlock(self: *BargingLock, provider: anytype) void {
        if (self.state.cmpxchgWeak(is_locked_bit, unlocked, .release, .monotonic) != null) {
            self.unlockSlow(provider, false);
        }
    }

    inline fn unlockFair(self: *BargingLock, provider: anytype) void {
        if (self.state.cmpxchgWeak(is_locked_bit, unlocked, .release, .monotonic) != null) {
            self.unlockSlow(provider, true);
        }
    }

    fn lockSlow(self: *BargingLock, provider: anytype, timeout: ?Time) bool {
        @branchHint(.cold);
        // The WebKit developers observed an optimum at 40 spins for the Intel architecture.
        const spin_limit: usize = 40;

        var spin_count: usize = 0;
        var state = self.state.load(.monotonic);
        while (true) {
            // Fast path
            if (state & is_locked_bit == 0) {
                state = self.state.cmpxchgWeak(
                    state,
                    state | is_locked_bit,
                    .acquire,
                    .monotonic,
                ) orelse return true;
                continue;
            }

            // Yield to the scheduler if there is no contention.
            if (state & has_parked_bit == 0 and spin_count < spin_limit) {
                spin_count += 1;
                Task.yieldCurrent(provider);
                state = self.state.load(.monotonic);
                continue;
            }

            // Notify other tasks that we will park.
            if (state & has_parked_bit == 0) {
                if (self.state.cmpxchgWeak(state, state | has_parked_bit, .monotonic, .monotonic)) |x| {
                    state = x;
                    continue;
                }
            }

            const Validation = struct {
                ptr: *BargingLock,
                fn f(this: *@This()) bool {
                    return this.ptr.state.load(.monotonic) == is_locked_bit | has_parked_bit;
                }
            };
            const BeforeSleep = struct {
                fn f(this: *@This()) void {
                    _ = this;
                }
            };
            const TimedOut = struct {
                ptr: *BargingLock,
                fn f(this: *@This(), key: *const anyopaque, is_last: bool) void {
                    _ = key;
                    if (is_last) {
                        _ = this.ptr.state.fetchAnd(~has_parked_bit, .monotonic);
                    }
                }
            };
            const result = ParkingLot.park(
                provider,
                self,
                Validation{ .ptr = self },
                Validation.f,
                BeforeSleep{},
                BeforeSleep.f,
                TimedOut{ .ptr = self },
                TimedOut.f,
                .default,
                timeout,
            );
            switch (result.type) {
                // If the lock was passed to us by another task we are done
                .unparked => if (result.token == handoff_token) return true,
                // The state changed, retry.
                .invalid => {},
                //
                .timed_out => return false,
            }

            // Retry.
            spin_count = 0;
            state = self.state.load(.monotonic);
        }
    }

    fn unlockSlow(self: *BargingLock, provider: anytype, comptime force_fair: bool) void {
        @branchHint(.cold);

        var state = self.state.load(.monotonic);
        while (true) {
            // Fast path
            if (state == is_locked_bit) {
                state = self.state.cmpxchgWeak(
                    is_locked_bit,
                    unlocked,
                    .release,
                    .monotonic,
                ) orelse return;
                continue;
            }

            const Callback = struct {
                ptr: *BargingLock,
                fn f(this: *@This(), result: ParkingLot.UnparkResult) ParkingLot.UnparkToken {
                    // If we do a fair unlock we pass the ownership of the lock directly to
                    // the unparked task without unlocking the mutex.
                    if (result.unparked_tasks != 0 and (force_fair or result.be_fair)) {
                        if (!result.has_more_tasks) {
                            this.ptr.state.store(is_locked_bit, .monotonic);
                        }
                        return handoff_token;
                    }

                    if (result.has_more_tasks) {
                        this.ptr.state.store(has_parked_bit, .release);
                    } else {
                        this.ptr.state.store(unlocked, .release);
                    }
                    return .default;
                }
            };
            _ = ParkingLot.unparkOne(provider, self, Callback{ .ptr = self }, Callback.f);
        }
    }

    /// Implementation detail of `Condition`.
    pub inline fn markParkedILocked(self: *BargingLock) bool {
        var state = self.state.load(.monotonic);
        while (true) {
            if (state & is_locked_bit == 0) return false;
            state = self.state.cmpxchgWeak(
                state,
                state | has_parked_bit,
                .monotonic,
                .monotonic,
            ) orelse return true;
        }
    }

    /// Implementation detail of `Condition`.
    pub inline fn markParked(self: *BargingLock) void {
        _ = self.state.fetchOr(has_parked_bit, .monotonic);
    }
};

test "smoke test (threads)" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    var mutex: Mutex = .{};

    try std.testing.expect(mutex.tryLock());
    try std.testing.expect(!mutex.tryLock());
    mutex.unlock(&ctx);

    mutex.lock(&ctx);
    try std.testing.expect(!mutex.tryLock());
    mutex.unlock(&ctx);
}

test "smoke test (tasks)" {
    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) !void {
            _ = err;

            var mutex: Mutex = .{};

            try std.testing.expect(mutex.tryLock());
            try std.testing.expect(!mutex.tryLock());
            mutex.unlock(ctx);

            mutex.lock(ctx);
            try std.testing.expect(!mutex.tryLock());
            mutex.unlock(ctx);
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

test "many uncontended (threads)" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const num_threads = 4;
    const num_increments = 1000;

    const Runner = struct {
        mutex: Mutex = .{},
        thread: std.Thread = undefined,
        ctx: *const testing.TestContext,
        counter: NonAtomicCounter = .{},

        fn run(self: *@This()) void {
            var i: usize = num_increments;
            while (i > 0) : (i -= 1) {
                self.mutex.lock(self.ctx);
                defer self.mutex.unlock(self.ctx);

                self.counter.inc();
            }
        }
    };

    var runners = [_]Runner{.{ .ctx = &ctx }} ** num_threads;
    for (&runners) |*r| r.thread = try std.Thread.spawn(.{}, Runner.run, .{r});
    for (runners) |r| r.thread.join();
    for (runners) |r| try std.testing.expectEqual(r.counter.get(), num_increments);
}

test "many uncontended (tasks)" {
    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) !void {
            const num_threads = 4;
            const num_increments = 1000;

            var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
            defer arena.deinit();
            const allocator = arena.allocator();

            const Runner = struct {
                mutex: Mutex = .{},
                counter: NonAtomicCounter = .{},
                ctx: *const testing.TestContext,
            };
            var runners = [_]Runner{.{ .ctx = ctx }} ** num_threads;

            var tasks = blk: {
                const config = task.BuilderConfig(*Runner){
                    .on_start = struct {
                        fn f(t: *task.Task(*Runner)) void {
                            var i: usize = num_increments;
                            while (i > 0) : (i -= 1) {
                                t.state.mutex.lock(t.state.ctx);
                                defer t.state.mutex.unlock(t.state.ctx);
                                t.state.counter.inc();
                            }
                        }
                    }.f,
                };
                var tasks_: [num_threads]task.Task(*Runner) = undefined;
                for (&tasks_, &runners) |*t, *r| {
                    const b = task.Builder(config){ .state = r };
                    t.* = b.build();
                }
                break :blk tasks_;
            };

            var buffer = blk: {
                const command_buffer = @import("../command_buffer.zig");
                const config = command_buffer.BuilderConfig(void){};
                var buff = command_buffer.Builder(config){ .state = {} };
                for (&tasks) |*t| try buff.enqueueTask(allocator, @ptrCast(t));
                break :blk buff.build();
            };
            errdefer buffer.abort();

            const p = @import("../pool.zig").Pool.current(ctx).?;
            defer p.unref();

            const handle = try p.enqueueCommandBuffer(&buffer, err);
            defer handle.unref();
            try std.testing.expectEqual(.completed, handle.waitOn());
            for (runners) |r| try std.testing.expectEqual(r.counter.get(), num_increments);
        }
    }.f);
}

test "many contended (threads)" {
    var ctx = try testing.initTestContext();
    defer ctx.deinit();

    const num_threads = 4;
    const num_increments = 1000;

    const Runner = struct {
        mutex: Mutex = .{},
        ctx: *const testing.TestContext,
        counter: NonAtomicCounter = .{},

        fn run(self: *@This()) void {
            var i: usize = num_increments;
            while (i > 0) : (i -= 1) {
                // Occasionally hint to let another thread run.
                defer if (i % 100 == 0) std.Thread.yield() catch {};

                self.mutex.lock(self.ctx);
                defer self.mutex.unlock(self.ctx);

                self.counter.inc();
            }
        }
    };

    var runner = Runner{ .ctx = &ctx };

    var threads: [num_threads]std.Thread = undefined;
    for (&threads) |*t| t.* = try std.Thread.spawn(.{}, Runner.run, .{&runner});
    for (threads) |t| t.join();

    try std.testing.expectEqual(runner.counter.get(), num_increments * num_threads);
}

test "many contended (tasks)" {
    try testing.initTestContextInTask(struct {
        fn f(ctx: *const testing.TestContext, err: *?AnyError) !void {
            const num_threads = 4;
            const num_increments = 1000;

            var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
            defer arena.deinit();
            const allocator = arena.allocator();

            const Runner = struct {
                mutex: Mutex = .{},
                ctx: *const testing.TestContext,
                counter: NonAtomicCounter = .{},

                fn run(self: *@This()) void {
                    var i: usize = num_increments;
                    while (i > 0) : (i -= 1) {
                        // Occasionally hint to let another thread run.
                        defer if (i % 100 == 0) Task.yieldCurrent(self.ctx);

                        self.mutex.lock(self.ctx);
                        defer self.mutex.unlock(self.ctx);

                        self.counter.inc();
                    }
                }
            };
            var runner = Runner{ .ctx = ctx };

            var tasks = blk: {
                const config = task.BuilderConfig(*Runner){
                    .on_start = struct {
                        fn f(self: *task.Task(*Runner)) void {
                            self.state.run();
                        }
                    }.f,
                };
                const builder = task.Builder(config){ .state = &runner };
                var tasks_: [num_threads]task.Task(*Runner) = undefined;
                for (&tasks_) |*t| t.* = builder.build();
                break :blk tasks_;
            };

            var buffer = blk: {
                const command_buffer = @import("../command_buffer.zig");
                const config = command_buffer.BuilderConfig(void){};
                var buff = command_buffer.Builder(config){ .state = {} };
                for (&tasks) |*t| try buff.enqueueTask(allocator, @ptrCast(t));
                break :blk buff.build();
            };
            errdefer buffer.abort();

            const p = @import("../pool.zig").Pool.current(ctx).?;
            defer p.unref();

            const handle = try p.enqueueCommandBuffer(&buffer, err);
            defer handle.unref();
            try std.testing.expectEqual(.completed, handle.waitOn());
            try std.testing.expectEqual(runner.counter.get(), num_increments * num_threads);
        }
    }.f);
}
