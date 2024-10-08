const std = @import("std");
const builtin = @import("builtin");

const c = @import("c.zig");
const errors = @import("errors.zig");

const os_ext = switch (builtin.target.os.tag) {
    .windows => struct {
        pub extern "kernel32" fn GetSystemTimeAsFileTime(
            lpSystemTimeAsFileTime: *std.os.windows.FILETIME,
        ) callconv(std.os.windows.WINAPI) void;
    },
    else => struct {},
};

/// Number of milliseconds per second.
pub const millis_per_sec = 1000;

/// Number of microseconds per second.
pub const micros_per_sec = micros_per_millis * millis_per_sec;

/// Number of nanoseconds per second.
pub const nanos_per_sec = nanos_per_micros * micros_per_sec;

/// Number of microseconds per millisecond.
pub const micros_per_millis = 1000;

/// Number of nanoseconds per millisecond.
pub const nanos_per_millis = nanos_per_micros * micros_per_millis;

/// Number of nanoseconds per microsecond.
pub const nanos_per_micros = 1000;

/// Maximum number of milliseconds.
pub const max_millis = ((std.math.maxInt(Seconds) + 1) * millis_per_sec) - 1;

/// Maximum number of microseconds.
pub const max_micros = ((std.math.maxInt(Seconds) + 1) * micros_per_sec) - 1;

/// Maximum number of nanoseconds.
pub const max_nanos = ((std.math.maxInt(Seconds) + 1) * nanos_per_sec) - 1;

/// Integer type capable of storing the number of seconds in a span of time.
pub const Seconds = u64;

/// Integer type capable of storing the number of milliseconds in a span of time.
pub const Millis = std.math.IntFittingRange(0, max_millis);

/// Integer type capable of storing the number of microseconds in a span of time.
pub const Micros = std.math.IntFittingRange(0, max_micros);

/// Integer type capable of storing the number of nanoseconds in a span of time.
pub const Nanos = std.math.IntFittingRange(0, max_nanos);

/// Integer type capable of storing the number of milliseconds in a second.
pub const SubSecondMillis = std.math.IntFittingRange(0, millis_per_sec - 1);

/// Integer type capable of storing the number of microseconds in a second.
pub const SubSecondMicros = std.math.IntFittingRange(0, micros_per_sec - 1);

/// Integer type capable of storing the number of nanoseconds in a second.
pub const SubSecondNanos = std.math.IntFittingRange(0, nanos_per_sec - 1);

/// A span of time.
pub const Duration = struct {
    /// Number of seconds.
    secs: Seconds = 0,
    /// Number of nanoseconds.
    sub_sec_nanos: SubSecondNanos = 0,

    /// Zero duration.
    pub const Zero = Duration{
        .secs = 0,
        .sub_sec_nanos = 0,
    };

    /// Maximum duration.
    pub const Max = Duration{
        .secs = std.math.maxInt(u64),
        .sub_sec_nanos = nanos_per_sec - 1,
    };

    /// Constructs a duration from a number of seconds.
    pub fn initSeconds(seconds: Seconds) Duration {
        return .{
            .secs = seconds,
            .sub_sec_nanos = 0,
        };
    }

    /// Constructs a duration from a number of milliseconds.
    pub fn initMillis(milliseconds: Millis) Duration {
        std.debug.assert(milliseconds <= max_millis);
        return .{
            .secs = @intCast(milliseconds / millis_per_sec),
            .sub_sec_nanos = @intCast((milliseconds % millis_per_sec) * nanos_per_millis),
        };
    }

    /// Constructs a duration from a number of microseconds.
    pub fn initMicros(microseconds: Micros) Duration {
        std.debug.assert(microseconds <= max_micros);
        return .{
            .secs = @intCast(microseconds / micros_per_sec),
            .sub_sec_nanos = @intCast((microseconds % micros_per_sec) * nanos_per_micros),
        };
    }

    /// Constructs a duration from a number of nanoseconds.
    pub fn initNanos(nanoseconds: Nanos) Duration {
        std.debug.assert(nanoseconds <= max_nanos);
        return .{
            .secs = @intCast(nanoseconds / nanos_per_sec),
            .sub_sec_nanos = @intCast(nanoseconds % nanos_per_sec),
        };
    }

    /// Checks whether the duration is zero.
    pub fn isZero(self: Duration) bool {
        return self.secs == 0 and self.sub_sec_nanos == 0;
    }

    /// Extracts the sub-second milliseconds.
    pub fn subSecMillis(self: Duration) SubSecondMillis {
        return @intCast(self.sub_sec_nanos / nanos_per_millis);
    }

    /// Extracts the sub-second microseconds.
    pub fn subSecMicros(self: Duration) SubSecondMicros {
        return @intCast(self.sub_sec_nanos / nanos_per_micros);
    }

    /// Extracts the number of milliseconds
    pub fn millis(self: Duration) Millis {
        return @intCast(self.nanos() / nanos_per_millis);
    }

    /// Extracts the number of microseconds
    pub fn micros(self: Duration) Millis {
        return @intCast(self.nanos() / nanos_per_micros);
    }

    /// Extracts the number of nanoseconds.
    pub fn nanos(self: Duration) Nanos {
        const seconds: Nanos = @intCast(self.secs * nanos_per_sec);
        return seconds + self.sub_sec_nanos;
    }

    /// Adds two durations.
    pub fn add(lhs: Duration, rhs: Duration) error{Overflow}!Duration {
        var seconds = try std.math.add(Seconds, lhs.secs, rhs.secs);
        var nanoseconds = try std.math.add(SubSecondNanos, lhs.sub_sec_nanos, rhs.sub_sec_nanos);
        if (nanoseconds >= nanos_per_sec) {
            nanoseconds -= nanos_per_sec;
            seconds = try std.math.add(Seconds, seconds, 1);
        }

        return Duration{
            .secs = seconds,
            .sub_sec_nanos = nanoseconds,
        };
    }

    /// Adds two duration, saturating the result to the maximum possible duration.
    pub fn addSaturating(lhs: Duration, rhs: Duration) Duration {
        return lhs.add(rhs) catch Duration.Max;
    }

    /// Subtracts two durations.
    pub fn sub(lhs: Duration, rhs: Duration) error{Overflow}!Duration {
        var seconds = try std.math.sub(Seconds, lhs.secs, rhs.secs);
        var nanoseconds = lhs.sub_sec_nanos;

        if (nanoseconds >= rhs.sub_sec_nanos) {
            nanoseconds -= rhs.sub_sec_nanos;
        } else {
            seconds = try std.math.sub(Seconds, seconds, 1);
            nanoseconds += nanos_per_millis - rhs.sub_sec_nanos;
        }

        return Duration{
            .secs = seconds,
            .sub_sec_nanos = nanoseconds,
        };
    }

    /// Subtracts two duration, saturating the result to the zero duration.
    pub fn subSaturating(lhs: Duration, rhs: Duration) Duration {
        return lhs.sub(rhs) catch Duration.Zero;
    }
};

/// A point in time since the unix epoch.
pub const Time = struct {
    /// Number of seconds.
    secs: Seconds = 0,
    /// Number of nanoseconds.
    sub_sec_nanos: SubSecondNanos = 0,

    /// The UNIX epoch.
    pub const UnixEpoch = Time{
        .secs = 0,
        .sub_sec_nanos = 0,
    };

    /// The latest possible time point.
    pub const Max = Time{
        .secs = std.math.maxInt(Seconds),
        .sub_sec_nanos = nanos_per_sec - 1,
    };

    /// Returns the current time.
    pub fn now() Time {
        switch (builtin.os.tag) {
            .windows => {
                // FileTime has a granularity of 100 nanoseconds and uses the NTFS/Windows epoch,
                // which is 1601-01-01.
                const epoch_adj = 11644473600 * (micros_per_sec * 10);
                var ft: std.os.windows.FILETIME = undefined;
                os_ext.GetSystemTimeAsFileTime(&ft);
                const ft64 = (@as(u64, ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
                std.debug.assert(ft64 >= epoch_adj);
                const ft_unix = ft64 - epoch_adj;
                return Time{
                    .secs = @intCast(ft_unix / (micros_per_sec * 10)),
                    .sub_sec_nanos = @intCast(ft_unix % (micros_per_sec * 10) * 100),
                };
            },
            .wasi => {
                var ns: std.os.wasi.timestamp_t = undefined;
                const err = std.os.wasi.clock_time_get(.REALTIME, 1, &ns);
                std.debug.assert(err == .SUCCESS);
                return Time{
                    .secs = @intCast(ns / nanos_per_sec),
                    .sub_sec_nanos = @intCast(ns % nanos_per_sec),
                };
            },
            else => {
                var ts: std.posix.timespec = undefined;
                std.posix.clock_gettime(
                    .REALTIME,
                    &ts,
                ) catch @panic("REALTIME clock not supported");
                return Time{
                    .secs = @intCast(ts.sec),
                    .sub_sec_nanos = @intCast(ts.nsec),
                };
            },
        }
    }

    /// Returns the duration elapsed since a prior time point.
    ///
    /// This function may fail due to clock adjustments.
    pub fn elapsed(time: Time) error{Overflow}!Duration {
        const t = Time.now();
        return t.durationSince(time);
    }

    /// Returns the difference between two time points.
    pub fn durationSince(time: Time, earlier: Time) error{Overflow}!Duration {
        const t_dur = Duration{ .secs = time.secs, .sub_sec_nanos = time.sub_sec_nanos };
        const e_dur = Duration{ .secs = earlier.secs, .sub_sec_nanos = earlier.sub_sec_nanos };
        return t_dur.sub(e_dur);
    }

    /// Shifts the time point forwards by the specified duration.
    pub fn add(time: Time, duration: Duration) error{Overflow}!Time {
        const t = Duration{ .secs = time.secs, .sub_sec_nanos = time.sub_sec_nanos };
        const d = try t.add(duration);
        return Time{ .secs = d.secs, .sub_sec_nanos = d.sub_sec_nanos };
    }

    /// Shifts the time point forwards by the specified duration, saturating to the maximum time point.
    pub fn addSaturating(time: Time, duration: Duration) Time {
        return time.add(duration) catch Time.Max;
    }

    /// Shifts the time point backwards by the specified duration.
    pub fn sub(time: Time, duration: Duration) error{Overflow}!Time {
        const t = Duration{ .secs = time.secs, .sub_sec_nanos = time.sub_sec_nanos };
        const d = try t.sub(duration);
        return Time{ .secs = d.secs, .sub_sec_nanos = d.sub_sec_nanos };
    }

    /// Shifts the time point backwards by the specified duration, saturating to the unix epoch.
    pub fn subSaturating(time: Time, duration: Duration) Time {
        return time.sub(duration) catch Time.UnixEpoch;
    }
};

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    fn cTimeToZig(t: c.FimoTime) Time {
        return Time{ .secs = @intCast(t.secs), .sub_sec_nanos = @intCast(t.nanos) };
    }
    fn zigTimeToC(t: Time) c.FimoTime {
        return c.FimoTime{ .secs = @intCast(t.secs), .nanos = @intCast(t.sub_sec_nanos) };
    }
    fn cDurationToZig(d: c.FimoDuration) Duration {
        return Duration{ .secs = @intCast(d.secs), .sub_sec_nanos = @intCast(d.nanos) };
    }
    fn zigDurationToC(d: Duration) c.FimoDuration {
        return c.FimoDuration{ .secs = @intCast(d.secs), .nanos = @intCast(d.sub_sec_nanos) };
    }

    export fn fimo_duration_zero() c.FimoDuration {
        return zigDurationToC(Duration.Zero);
    }

    export fn fimo_duration_max() c.FimoDuration {
        return zigDurationToC(Duration.Max);
    }

    export fn fimo_duration_from_seconds(seconds: u64) c.FimoDuration {
        return zigDurationToC(Duration.initSeconds(seconds));
    }

    export fn fimo_duration_from_millis(millis: u64) c.FimoDuration {
        return zigDurationToC(Duration.initMillis(millis));
    }

    export fn fimo_duration_from_nanos(nanos: u64) c.FimoDuration {
        return zigDurationToC(Duration.initNanos(nanos));
    }

    export fn fimo_duration_is_zero(duration: *const c.FimoDuration) bool {
        const d = cDurationToZig(duration.*);
        return d.isZero();
    }

    export fn fimo_duration_as_secs(duration: *const c.FimoDuration) u64 {
        return duration.secs;
    }

    export fn fimo_duration_subsec_millis(duration: *const c.FimoDuration) u32 {
        const d = cDurationToZig(duration.*);
        return d.subSecMillis();
    }

    export fn fimo_duration_subsec_micros(duration: *const c.FimoDuration) u32 {
        const d = cDurationToZig(duration.*);
        return d.subSecMicros();
    }

    export fn fimo_duration_subsec_nanos(duration: *const c.FimoDuration) u32 {
        return duration.nanos;
    }

    export fn fimo_duration_as_millis(
        duration: *const c.FimoDuration,
        high: ?*u32,
    ) u64 {
        const d = cDurationToZig(duration.*);
        const millis = d.millis();

        if (high) |h| {
            h.* = @intCast(millis >> 64);
        }
        return @truncate(millis);
    }

    export fn fimo_duration_as_micros(
        duration: *const c.FimoDuration,
        high: ?*u32,
    ) u64 {
        const d = cDurationToZig(duration.*);
        const micros = d.micros();

        if (high) |h| {
            h.* = @intCast(micros >> 64);
        }
        return @truncate(micros);
    }

    export fn fimo_duration_as_nanos(
        duration: *const c.FimoDuration,
        high: ?*u32,
    ) u64 {
        const d = cDurationToZig(duration.*);
        const nanos = d.nanos();

        if (high) |h| {
            h.* = @intCast(nanos >> 64);
        }
        return @truncate(nanos);
    }

    export fn fimo_duration_add(
        lhs: *const c.FimoDuration,
        rhs: *const c.FimoDuration,
        out: *c.FimoDuration,
    ) c.FimoResult {
        const d1 = cDurationToZig(lhs.*);
        const d2 = cDurationToZig(rhs.*);
        if (d1.add(d2)) |d| {
            out.* = zigDurationToC(d);
            return errors.Error.intoCResult(null);
        } else |err| return errors.Error.initError(err).err;
    }

    export fn fimo_duration_saturating_add(
        lhs: *const c.FimoDuration,
        rhs: *const c.FimoDuration,
    ) c.FimoDuration {
        const d1 = cDurationToZig(lhs.*);
        const d2 = cDurationToZig(rhs.*);
        return zigDurationToC(d1.addSaturating(d2));
    }

    export fn fimo_duration_sub(
        lhs: *const c.FimoDuration,
        rhs: *const c.FimoDuration,
        out: *c.FimoDuration,
    ) c.FimoResult {
        const d1 = cDurationToZig(lhs.*);
        const d2 = cDurationToZig(rhs.*);
        if (d1.sub(d2)) |d| {
            out.* = zigDurationToC(d);
            return errors.Error.intoCResult(null);
        } else |err| return errors.Error.initError(err).err;
    }

    export fn fimo_duration_saturating_sub(
        lhs: *const c.FimoDuration,
        rhs: *const c.FimoDuration,
    ) c.FimoDuration {
        const d1 = cDurationToZig(lhs.*);
        const d2 = cDurationToZig(rhs.*);
        return zigDurationToC(d1.subSaturating(d2));
    }

    export fn fimo_time_now() c.FimoTime {
        return zigTimeToC(Time.now());
    }

    export fn fimo_time_elapsed(
        time_point: *const c.FimoTime,
        out: *c.FimoDuration,
    ) c.FimoResult {
        const t = cTimeToZig(time_point.*);
        if (t.elapsed()) |dur| {
            out.* = zigDurationToC(dur);
            return errors.Error.intoCResult(null);
        } else |err| return errors.Error.initError(err).err;
    }

    export fn fimo_time_duration_since(
        time_point: *const c.FimoTime,
        earlier_time_point: *const c.FimoTime,
        out: *c.FimoDuration,
    ) c.FimoResult {
        const t1 = cTimeToZig(time_point.*);
        const t2 = cTimeToZig(earlier_time_point.*);
        if (t1.durationSince(t2)) |dur| {
            out.* = zigDurationToC(dur);
            return errors.Error.intoCResult(null);
        } else |err| return errors.Error.initError(err).err;
    }

    export fn fimo_time_add(
        time_point: *const c.FimoTime,
        duration: *const c.FimoDuration,
        out: *c.FimoTime,
    ) c.FimoResult {
        const t = cTimeToZig(time_point.*);
        const d = cDurationToZig(duration.*);
        if (t.add(d)) |shifted| {
            out.* = zigTimeToC(shifted);
            return errors.Error.intoCResult(null);
        } else |err| return errors.Error.initError(err).err;
    }

    export fn fimo_time_saturating_add(
        time_point: *const c.FimoTime,
        duration: *const c.FimoDuration,
    ) c.FimoTime {
        const t = cTimeToZig(time_point.*);
        const d = cDurationToZig(duration.*);
        return zigTimeToC(t.addSaturating(d));
    }

    export fn fimo_time_sub(
        time_point: *const c.FimoTime,
        duration: *const c.FimoDuration,
        out: *c.FimoTime,
    ) c.FimoResult {
        const t = cTimeToZig(time_point.*);
        const d = cDurationToZig(duration.*);
        if (t.sub(d)) |shifted| {
            out.* = zigTimeToC(shifted);
            return errors.Error.intoCResult(null);
        } else |err| return errors.Error.initError(err).err;
    }

    export fn fimo_time_saturating_sub(
        time_point: *const c.FimoTime,
        duration: *const c.FimoDuration,
    ) c.FimoTime {
        const t = cTimeToZig(time_point.*);
        const d = cDurationToZig(duration.*);
        return zigTimeToC(t.subSaturating(d));
    }
};

comptime {
    _ = ffi;
}
