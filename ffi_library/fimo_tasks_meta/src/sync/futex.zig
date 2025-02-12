//! Portable user-space implementation of the linux futex API.

const std = @import("std");
const atomic = std.atomic;
const math = std.math;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Time = time.Time;
const Duration = time.Duration;

const ParkingLot = @import("ParkingLot.zig");

pub fn Futex(comptime T: type) type {
    switch (@typeInfo(T)) {
        .int => {},
        else => @compileError("unsupported type for Futex, only integers are supported"),
    }

    return struct {
        pub const WakeOp = enum {};

        /// Checks if `ptr` still contains the value `expect` and if so, blocks until either:
        /// - The value at `ptr` is no longer equal to `expect`.
        /// - The caller is unblocked by a call to `wake`.
        /// - The caller is unblocked spuriously ("at random").
        /// - The caller blocks for longer than the given timeout. In which case, `error.Timeout` is returned.
        pub fn timedWait(
            provider: anytype,
            ptr: *const atomic.Value(T),
            expect: T,
            timeout: Duration,
        ) error{Timeout}!void {
            const Validation = struct {
                ptr: *const atomic.Value(T),
                expect: T,
                fn f(self: *@This()) bool {
                    return self.ptr.load(.monotonic) == self.expect;
                }
            };
            const BeforeSleep = struct {
                fn f(self: *@This()) void {
                    _ = self;
                }
            };
            const TimedOut = struct {
                fn f(self: *@This(), key: *const anyopaque, is_last: bool) void {
                    _ = is_last;
                    _ = key;
                    _ = self;
                }
            };

            const timeout_time = Time.now().addSaturating(timeout);
            const result = ParkingLot.park(
                provider,
                ptr,
                Validation{ .ptr = ptr, .expect = expect },
                Validation.f,
                BeforeSleep{},
                BeforeSleep.f,
                TimedOut{},
                TimedOut.f,
                .default,
                timeout_time,
            );
            std.debug.assert(result.token == .default);
            switch (result.type) {
                .invalid, .unparked => {},
                .timed_out => return error.Timeout,
            }
        }

        /// Checks if `ptr` still contains the value `expect` and if so, blocks until either:
        /// - The value at `ptr` is no longer equal to `expect`.
        /// - The caller is unblocked by a call to `wake`.
        /// - The caller is unblocked spuriously ("at random").
        pub fn wait(provider: anytype, ptr: *const atomic.Value(T), expect: T) void {
            const Validation = struct {
                ptr: *const atomic.Value(T),
                expect: T,
                fn f(self: *@This()) bool {
                    return self.ptr.load(.monotonic) == self.expect;
                }
            };
            const BeforeSleep = struct {
                fn f(self: *@This()) void {
                    _ = self;
                }
            };
            const TimedOut = struct {
                fn f(self: *@This(), key: *const anyopaque, is_last: bool) void {
                    _ = is_last;
                    _ = key;
                    _ = self;
                    unreachable;
                }
            };

            const result = ParkingLot.park(
                provider,
                ptr,
                Validation{ .ptr = ptr, .expect = expect },
                Validation.f,
                BeforeSleep{},
                BeforeSleep.f,
                TimedOut{},
                TimedOut.f,
                .default,
                null,
            );
            std.debug.assert(result.type != .timed_out);
            std.debug.assert(result.token == .default);
        }

        /// Unblocks at most `max_waiters` waiters.
        pub fn wake(provider: anytype, ptr: *const atomic.Value(T), max_waiters: usize) void {
            const Callback = struct {
                fn f(self: *@This()) ParkingLot.UnparkToken {
                    _ = self;
                    return .default;
                }
            };
            switch (max_waiters) {
                0 => {},
                1 => _ = ParkingLot.unparkOne(provider, ptr, Callback{}, Callback.f),
                std.math.maxInt(usize) => _ = ParkingLot.unparkAll(provider, ptr, .default),
                else => {
                    const Filter = struct {
                        unparked_waiters: usize = 0,
                        max_waiters: usize,
                        fn f(self: *@This(), token: ParkingLot.ParkToken) ParkingLot.FilterOp {
                            _ = token;
                            if (self.unparked_waiters >= self.max_waiters) return .stop;
                            self.unparked_waiters += 1;
                            return .unpark;
                        }
                    };
                    _ = ParkingLot.unparkFilter(
                        provider,
                        ptr,
                        Filter{ .max_waiters = max_waiters },
                        Filter.f,
                        Callback{},
                        Callback.f,
                    );
                },
            }
        }
    };
}
