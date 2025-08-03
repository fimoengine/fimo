//! Portable user-space implementation of the linux futex API.

const std = @import("std");
const atomic = std.atomic;
const math = std.math;

const fimo_std = @import("fimo_std");
const time = fimo_std.time;
const Instant = time.Instant;
const Duration = time.Duration;

const symbols = @import("../symbols.zig");

const Futex = @This();

/// Maximum number of keys allowed for the `waitv` operation.
pub const max_waitv_key_count = 128;

/// Possible status codes of the futex symbols.
pub const Status = enum(i32) {
    Ok = 0,
    Invalid = 1,
    Timeout = 2,
    KeyError = 3,
    _,
};

/// Information required for a wait operation.
pub const KeyExpect = extern struct {
    key: *const anyopaque,
    key_size: usize,
    expect: u64,
    token: usize = 0,
};

/// Filter for a filter operation.
pub const Filter = extern struct {
    op: packed struct(usize) {
        token_op: enum(u1) { noop, deref },
        token_type: enum(u2) { u8, u16, u32, u64 },
        cmp_op: enum(u3) { eq, ne, lt, le, gt, ge },
        cmp_arg_op: enum(u1) { noop, deref },
        reserved: @Type(.{ .int = .{ .bits = @bitSizeOf(usize) - 7, .signedness = .unsigned } }) = undefined,
    },
    token_mask: usize,
    cmp_arg: usize,

    // @as(u8, token & 0) == 0
    pub const all: Filter = .{
        .op = .{
            .token_op = .noop,
            .token_type = .u8,
            .cmp_op = .eq,
            .cmp_arg_op = .noop,
        },
        .token_mask = 0,
        .cmp_arg = 0,
    };

    /// Applies the filter on a token, returning `true` if the token passes the filter.
    pub fn checkToken(self: Filter, token: usize) bool {
        const masked = token & self.token_mask;
        switch (self.op.token_type) {
            .u8 => {
                const val: u8 = if (self.op.token_op == .deref) @as(*const u8, @ptrFromInt(masked)).* else @truncate(masked);
                const cmp: u8 = if (self.op.cmp_arg_op == .deref) @as(*const u8, @ptrFromInt(self.cmp_arg)).* else @truncate(self.cmp_arg);
                return switch (self.op.cmp_op) {
                    .eq => val == cmp,
                    .ne => val != cmp,
                    .lt => val < cmp,
                    .le => val <= cmp,
                    .gt => val > cmp,
                    .ge => val >= cmp,
                };
            },
            .u16 => {
                const val: u16 = if (self.op.token_op == .deref) @as(*const u16, @ptrFromInt(masked)).* else @truncate(masked);
                const cmp: u16 = if (self.op.cmp_arg_op == .deref) @as(*const u16, @ptrFromInt(self.cmp_arg)).* else @truncate(self.cmp_arg);
                return switch (self.op.cmp_op) {
                    .eq => val == cmp,
                    .ne => val != cmp,
                    .lt => val < cmp,
                    .le => val <= cmp,
                    .gt => val > cmp,
                    .ge => val >= cmp,
                };
            },
            .u32 => {
                const val: u32 = if (self.op.token_op == .deref) @as(*const u32, @ptrFromInt(masked)).* else @truncate(masked);
                const cmp: u32 = if (self.op.cmp_arg_op == .deref) @as(*const u32, @ptrFromInt(self.cmp_arg)).* else @truncate(self.cmp_arg);
                return switch (self.op.cmp_op) {
                    .eq => val == cmp,
                    .ne => val != cmp,
                    .lt => val < cmp,
                    .le => val <= cmp,
                    .gt => val > cmp,
                    .ge => val >= cmp,
                };
            },
            .u64 => {
                const val: u64 = if (self.op.token_op == .deref) @as(*const u64, @ptrFromInt(masked)).* else @truncate(masked);
                const cmp: u64 = if (self.op.cmp_arg_op == .deref) @as(*const u64, @ptrFromInt(self.cmp_arg)).* else @truncate(self.cmp_arg);
                return switch (self.op.cmp_op) {
                    .eq => val == cmp,
                    .ne => val != cmp,
                    .lt => val < cmp,
                    .le => val <= cmp,
                    .gt => val > cmp,
                    .ge => val >= cmp,
                };
            },
        }
    }
};

/// Result of the requeue operation.
pub const RequeueResult = extern struct {
    wake_count: usize = 0,
    requeue_count: usize = 0,
};

/// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
///
/// If the value does not match, the function returns imediately with `error.Invalid`. The
/// `key_size` parameter specifies the size of the value in bytes and must be either of `1`, `2`,
/// `4` or `8`, in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or
/// `u64` respectively, and `expect` is truncated. The `token` is a user definable integer to store
/// additional metadata about the waiter, which can be utilized to controll some wake operations.
///
/// If `timeout` is reached before a wake operation wakes the task, the task will be resumed, and
/// the function returns `error.Timeout`.
pub fn timedWait(
    key: *const anyopaque,
    key_size: usize,
    expect: u64,
    token: usize,
    timeout: Instant,
) error{ Invalid, Timeout }!void {
    const t = timeout.intoC();
    const sym = symbols.futex_wait.getGlobal().get();
    return switch (sym(key, key_size, expect, token, &t)) {
        .Ok => {},
        .Invalid => error.Invalid,
        .Timeout => error.Timeout,
        else => unreachable,
    };
}

/// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
///
/// If the value does not match, the function returns imediately with `error.Invalid`. The
/// `key_size` parameter specifies the size of the value in bytes and must be either of `1`, `2`,
/// `4` or `8`, in which case `key` is treated as pointer to `u8`, `u16`, `u32`, or
/// `u64` respectively, and `expect` is truncated. The `token` is a user definable integer to store
/// additional metadata about the waiter, which can be utilized to controll some wake operations.
pub fn wait(key: *const anyopaque, key_size: usize, expect: u64, token: usize) error{Invalid}!void {
    const sym = symbols.futex_wait.getGlobal().get();
    return switch (sym(key, key_size, expect, token, null)) {
        .Ok => {},
        .Invalid => error.Invalid,
        else => unreachable,
    };
}

/// Puts the caller to sleep if all keys match their expected values.
///
/// Is a generalization of `wait` for multiple keys. At least `1` key must, and at most
/// `max_waitv_key_count` may be passed to this function. Otherwise it returns `error.KeyError`.
pub fn timedWaitv(keys: []const KeyExpect, timeout: Instant) error{ KeyError, Invalid, Timeout }!usize {
    const t = timeout.intoC();
    var wake_index: usize = undefined;
    const sym = symbols.futex_waitv.getGlobal().get();
    return switch (sym(keys.ptr, keys.len, &t, &wake_index)) {
        .Ok => wake_index,
        .Invalid => error.Invalid,
        .Timeout => error.Timeout,
        .KeyError => error.KeyError,
        else => unreachable,
    };
}

/// Puts the caller to sleep if all keys match their expected values.
///
/// Is a generalization of `wait` for multiple keys. At least `1` key must, and at most
/// `max_waitv_key_count` may be passed to this function. Otherwise it returns `error.KeyError`.
pub fn waitv(keys: []const KeyExpect) error{ KeyError, Invalid }!usize {
    var wake_index: usize = undefined;
    const sym = symbols.futex_waitv.getGlobal().get();
    return switch (sym(keys.ptr, keys.len, null, &wake_index)) {
        .Ok => wake_index,
        .Invalid => error.Invalid,
        .KeyError => error.KeyError,
        else => unreachable,
    };
}

/// Wakes at most `max_waiters` waiting on `key`.
///
/// Uses the token provided by the waiter and the `filter` to determine whether to ignore it from
/// being woken up. Returns the number of woken waiters.
pub fn wakeFilter(key: *const anyopaque, max_waiters: usize, filter: Filter) usize {
    const sym = symbols.futex_wake.getGlobal().get();
    return sym(key, max_waiters, filter);
}

/// Wakes at most `max_waiters` waiting on `key`.
///
/// Returns the number of woken waiters.
pub fn wake(key: *const anyopaque, max_waiters: usize) usize {
    return wakeFilter(key, max_waiters, .all);
}

/// Requeues waiters from `key_from` to `key_to`.
///
/// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
/// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
/// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
/// the function returns `error.Invalid`. Uses the token provided by the waiter and the `filter`
/// to determine whether to ignore it from being woken up.
pub fn requeueFilter(
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    key_size: usize,
    expect: u64,
    max_wakes: usize,
    max_requeues: usize,
    filter: Filter,
) error{Invalid}!RequeueResult {
    var result: RequeueResult = undefined;
    const sym = symbols.futex_requeue.getGlobal().get();
    return switch (sym(
        key_from,
        key_to,
        key_size,
        expect,
        max_wakes,
        max_requeues,
        filter,
        &result,
    )) {
        .Ok => result,
        .Invalid => error.Invalid,
        else => unreachable,
    };
}

/// Requeues waiters from `key_from` to `key_to`.
///
/// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
/// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
/// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
/// the function returns `error.Invalid`.
pub fn requeue(
    key_from: *const anyopaque,
    key_to: *const anyopaque,
    key_size: usize,
    expect: u64,
    max_wakes: usize,
    max_requeues: usize,
) error{Invalid}!RequeueResult {
    return requeueFilter(key_from, key_to, key_size, expect, max_wakes, max_requeues, .all);
}

pub fn TypedHelper(comptime T: type) type {
    switch (@bitSizeOf(T)) {
        8, 16, 32, 64 => {},
        else => @compileError("unsupported bit size"),
    }
    const Int = switch (@sizeOf(T)) {
        1 => u8,
        2 => u16,
        4 => u32,
        8 => u64,
        else => @compileError("unsupported byte size"),
    };

    return struct {
        pub fn timedWait(
            key: *const atomic.Value(T),
            expect: T,
            token: usize,
            timeout: Instant,
        ) error{ Invalid, Timeout }!void {
            return Futex.timedWait(key, @sizeOf(T), @as(Int, @bitCast(expect)), token, timeout);
        }
        pub fn wait(
            key: *const atomic.Value(T),
            expect: T,
            token: usize,
        ) error{Invalid}!void {
            return Futex.wait(key, @sizeOf(T), @as(Int, @bitCast(expect)), token);
        }
        pub fn requeueFilter(
            key_from: *const atomic.Value(T),
            key_to: *const anyopaque,
            expect: T,
            max_wakes: usize,
            max_requeues: usize,
            filter: Filter,
        ) error{Invalid}!RequeueResult {
            return Futex.requeueFilter(
                key_from,
                key_to,
                @sizeOf(T),
                @as(Int, @bitCast(expect)),
                max_wakes,
                max_requeues,
                filter,
            );
        }
        pub fn requeue(
            key_from: *const atomic.Value(T),
            key_to: *const anyopaque,
            expect: T,
            max_wakes: usize,
            max_requeues: usize,
        ) error{Invalid}!RequeueResult {
            return Futex.requeue(
                key_from,
                key_to,
                @sizeOf(T),
                @as(Int, @bitCast(expect)),
                max_wakes,
                max_requeues,
            );
        }
    };
}
