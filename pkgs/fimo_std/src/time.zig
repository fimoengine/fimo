const std = @import("std");
const builtin = @import("builtin");

const AnyError = @import("AnyError.zig");
const AnyResult = AnyError.AnyResult;

const os_ext = switch (builtin.target.os.tag) {
    .windows => struct {
        pub extern "kernel32" fn GetSystemTimeAsFileTime(
            lpSystemTimeAsFileTime: *std.os.windows.FILETIME,
        ) callconv(.winapi) void;
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

/// Redeclaration of the C-API types.
pub const compat = struct {
    pub const Duration = extern struct {
        secs: u64,
        nanos: u32,
    };
    pub const Time = extern struct {
        secs: u64,
        nanos: u32,
    };
    pub const Instant = extern struct {
        secs: u64,
        nanos: u32,
    };
};

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

    /// Initializes the object from a ffi duration.
    pub fn initC(duration: compat.Duration) Duration {
        return Duration{
            .secs = @intCast(duration.secs),
            .sub_sec_nanos = @intCast(duration.nanos),
        };
    }

    /// Casts the object to a ffi duration.
    pub fn intoC(self: Duration) compat.Duration {
        return compat.Duration{
            .secs = @intCast(self.secs),
            .nanos = @intCast(self.sub_sec_nanos),
        };
    }

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
        const seconds = @as(Nanos, @intCast(self.secs)) * nanos_per_sec;
        return seconds + self.sub_sec_nanos;
    }

    /// Returns the order of two durations.
    pub fn order(self: Duration, other: Duration) std.math.Order {
        if (self.secs < other.secs) return .lt;
        if (self.secs > other.secs) return .gt;
        if (self.sub_sec_nanos < other.sub_sec_nanos) return .lt;
        if (self.sub_sec_nanos > other.sub_sec_nanos) return .gt;
        return .eq;
    }

    /// Adds two durations.
    pub fn add(lhs: Duration, rhs: Duration) error{Overflow}!Duration {
        var seconds = try std.math.add(Seconds, lhs.secs, rhs.secs);
        var nanoseconds = try std.math.add(
            u32,
            @intCast(lhs.sub_sec_nanos),
            @intCast(rhs.sub_sec_nanos),
        );
        if (nanoseconds >= nanos_per_sec) {
            nanoseconds -= nanos_per_sec;
            seconds = try std.math.add(Seconds, seconds, 1);
        }

        return Duration{
            .secs = seconds,
            .sub_sec_nanos = @intCast(nanoseconds),
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
            nanoseconds += nanos_per_sec - rhs.sub_sec_nanos;
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

    /// Initializes the object from a ffi time.
    pub fn initC(time: compat.Time) Time {
        return Time{
            .secs = @intCast(time.secs),
            .sub_sec_nanos = @intCast(time.nanos),
        };
    }

    /// Casts the object to a ffi time.
    pub fn intoC(self: Time) compat.Time {
        return compat.Time{
            .secs = @intCast(self.secs),
            .nanos = @intCast(self.sub_sec_nanos),
        };
    }

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
                const ts = std.posix.clock_gettime(
                    .REALTIME,
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

    /// Returns the order of two time points.
    pub fn order(self: Time, other: Time) std.math.Order {
        if (self.secs < other.secs) return .lt;
        if (self.secs > other.secs) return .gt;
        if (self.sub_sec_nanos < other.sub_sec_nanos) return .lt;
        if (self.sub_sec_nanos > other.sub_sec_nanos) return .gt;
        return .eq;
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

/// A monotonically increasing point in time.
///
/// The starting point is undefined.
pub const Instant = struct {
    /// Number of seconds.
    secs: Seconds = 0,
    /// Number of nanoseconds.
    sub_sec_nanos: SubSecondNanos = 0,

    /// The latest possible time point.
    pub const Max = Instant{
        .secs = std.math.maxInt(Seconds),
        .sub_sec_nanos = nanos_per_sec - 1,
    };

    /// Initializes the object from a ffi time.
    pub fn initC(time: compat.Instant) Instant {
        return Instant{
            .secs = @intCast(time.secs),
            .sub_sec_nanos = @intCast(time.nanos),
        };
    }

    /// Casts the object to a ffi time.
    pub fn intoC(self: Instant) compat.Instant {
        return compat.Instant{
            .secs = @intCast(self.secs),
            .nanos = @intCast(self.sub_sec_nanos),
        };
    }

    /// Returns the current time.
    pub fn now() Instant {
        switch (builtin.os.tag) {
            .windows => {
                const frequency = std.os.windows.QueryPerformanceFrequency();
                const counter = std.os.windows.QueryPerformanceCounter();
                const ns = blk: {
                    // 10Mhz (1 qpc tick every 100ns) is a common enough QPF value that we can optimize on it.
                    // https://github.com/microsoft/STL/blob/785143a0c73f030238ef618890fd4d6ae2b3a3a0/stl/inc/chrono#L694-L701
                    const common_frequency = 10_000_000;
                    if (frequency == common_frequency) {
                        break :blk counter * (nanos_per_sec / common_frequency);
                    }

                    // Convert to ns using fixed point.
                    const scale = @as(u64, nanos_per_sec << 32) / @as(u32, @intCast(frequency));
                    const result = (@as(u96, counter) * scale) >> 32;
                    break :blk @as(u64, @truncate(result));
                };
                return Instant{
                    .secs = @intCast(ns / nanos_per_sec),
                    .sub_sec_nanos = @intCast(ns % nanos_per_sec),
                };
            },
            .wasi => {
                var ns: std.os.wasi.timestamp_t = undefined;
                const err = std.os.wasi.clock_time_get(.MONOTONIC, 1, &ns);
                std.debug.assert(err == .SUCCESS);
                return Instant{
                    .secs = @intCast(ns / nanos_per_sec),
                    .sub_sec_nanos = @intCast(ns % nanos_per_sec),
                };
            },
            .macos, .ios, .tvos, .watchos, .visionos => {
                const ts = std.posix.clock_gettime(
                    .UPTIME_RAW,
                ) catch @panic("UPTIME_RAW clock not supported");
                return Instant{
                    .secs = @intCast(ts.sec),
                    .sub_sec_nanos = @intCast(ts.nsec),
                };
            },
            else => {
                const ts = std.posix.clock_gettime(
                    .MONOTONIC,
                ) catch @panic("MONOTONIC clock not supported");
                return Instant{
                    .secs = @intCast(ts.sec),
                    .sub_sec_nanos = @intCast(ts.nsec),
                };
            },
        }
    }

    /// Returns the duration elapsed since a prior time point.
    pub fn elapsed(time: Instant) error{Overflow}!Duration {
        const t = Instant.now();
        return t.durationSince(time);
    }

    /// Returns the order of two time points.
    pub fn order(self: Instant, other: Instant) std.math.Order {
        if (self.secs < other.secs) return .lt;
        if (self.secs > other.secs) return .gt;
        if (self.sub_sec_nanos < other.sub_sec_nanos) return .lt;
        if (self.sub_sec_nanos > other.sub_sec_nanos) return .gt;
        return .eq;
    }

    /// Returns the difference between two time points.
    pub fn durationSince(time: Instant, earlier: Instant) error{Overflow}!Duration {
        const t_dur = Duration{ .secs = time.secs, .sub_sec_nanos = time.sub_sec_nanos };
        const e_dur = Duration{ .secs = earlier.secs, .sub_sec_nanos = earlier.sub_sec_nanos };
        return t_dur.sub(e_dur);
    }

    /// Shifts the time point forwards by the specified duration.
    pub fn add(time: Instant, duration: Duration) error{Overflow}!Instant {
        const t = Duration{ .secs = time.secs, .sub_sec_nanos = time.sub_sec_nanos };
        const d = try t.add(duration);
        return Instant{ .secs = d.secs, .sub_sec_nanos = d.sub_sec_nanos };
    }

    /// Shifts the time point forwards by the specified duration, saturating to the maximum time point.
    pub fn addSaturating(time: Instant, duration: Duration) Instant {
        return time.add(duration) catch Instant.Max;
    }

    /// Shifts the time point backwards by the specified duration.
    pub fn sub(time: Instant, duration: Duration) error{Overflow}!Instant {
        const t = Duration{ .secs = time.secs, .sub_sec_nanos = time.sub_sec_nanos };
        const d = try t.sub(duration);
        return Instant{ .secs = d.secs, .sub_sec_nanos = d.sub_sec_nanos };
    }

    /// Shifts the time point backwards by the specified duration, saturating to the zero time point.
    pub fn subSaturating(time: Instant, duration: Duration) Instant {
        return time.sub(duration) catch Instant{};
    }
};

// ----------------------------------------------------
// FFI
// ----------------------------------------------------

const ffi = struct {
    export fn fimo_duration_zero() compat.Duration {
        return Duration.Zero.intoC();
    }

    export fn fimo_duration_max() compat.Duration {
        return Duration.Max.intoC();
    }

    export fn fimo_duration_from_seconds(seconds: u64) compat.Duration {
        return Duration.initSeconds(seconds).intoC();
    }

    export fn fimo_duration_from_millis(millis: u64) compat.Duration {
        return Duration.initMillis(millis).intoC();
    }

    export fn fimo_duration_from_nanos(nanos: u64) compat.Duration {
        return Duration.initNanos(nanos).intoC();
    }

    export fn fimo_duration_is_zero(duration: *const compat.Duration) bool {
        const d = Duration.initC(duration.*);
        return d.isZero();
    }

    export fn fimo_duration_as_secs(duration: *const compat.Duration) u64 {
        return duration.secs;
    }

    export fn fimo_duration_subsec_millis(duration: *const compat.Duration) u32 {
        const d = Duration.initC(duration.*);
        return d.subSecMillis();
    }

    export fn fimo_duration_subsec_micros(duration: *const compat.Duration) u32 {
        const d = Duration.initC(duration.*);
        return d.subSecMicros();
    }

    export fn fimo_duration_subsec_nanos(duration: *const compat.Duration) u32 {
        return duration.nanos;
    }

    export fn fimo_duration_as_millis(duration: *const compat.Duration, high: ?*u32) u64 {
        const d = Duration.initC(duration.*);
        const millis = d.millis();

        if (high) |h| {
            h.* = @intCast(millis >> 64);
        }
        return @truncate(millis);
    }

    export fn fimo_duration_as_micros(duration: *const compat.Duration, high: ?*u32) u64 {
        const d = Duration.initC(duration.*);
        const micros = d.micros();

        if (high) |h| {
            h.* = @intCast(micros >> 64);
        }
        return @truncate(micros);
    }

    export fn fimo_duration_as_nanos(duration: *const compat.Duration, high: ?*u32) u64 {
        const d = Duration.initC(duration.*);
        const nanos = d.nanos();

        if (high) |h| {
            h.* = @intCast(nanos >> 64);
        }
        return @truncate(nanos);
    }

    export fn fimo_duration_cmp(lhs: *const compat.Duration, rhs: *const compat.Duration) i32 {
        const d1 = Duration.initC(lhs.*);
        const d2 = Duration.initC(rhs.*);
        return switch (d1.order(d2)) {
            .lt => -1,
            .eq => 0,
            .gt => 1,
        };
    }

    export fn fimo_duration_add(
        lhs: *const compat.Duration,
        rhs: *const compat.Duration,
        out: *compat.Duration,
    ) AnyResult {
        const d1 = Duration.initC(lhs.*);
        const d2 = Duration.initC(rhs.*);
        if (d1.add(d2)) |d| {
            out.* = d.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_duration_saturating_add(
        lhs: *const compat.Duration,
        rhs: *const compat.Duration,
    ) compat.Duration {
        const d1 = Duration.initC(lhs.*);
        const d2 = Duration.initC(rhs.*);
        return d1.addSaturating(d2).intoC();
    }

    export fn fimo_duration_sub(
        lhs: *const compat.Duration,
        rhs: *const compat.Duration,
        out: *compat.Duration,
    ) AnyResult {
        const d1 = Duration.initC(lhs.*);
        const d2 = Duration.initC(rhs.*);
        if (d1.sub(d2)) |d| {
            out.* = d.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_duration_saturating_sub(
        lhs: *const compat.Duration,
        rhs: *const compat.Duration,
    ) compat.Duration {
        const d1 = Duration.initC(lhs.*);
        const d2 = Duration.initC(rhs.*);
        return d1.subSaturating(d2).intoC();
    }

    export fn fimo_time_now() compat.Time {
        return Time.now().intoC();
    }

    export fn fimo_time_elapsed(time_point: *const compat.Time, out: *compat.Duration) AnyResult {
        const t = Time.initC(time_point.*);
        if (t.elapsed()) |dur| {
            out.* = dur.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_time_cmp(lhs: *const compat.Time, rhs: *const compat.Time) i32 {
        const d1 = Time.initC(lhs.*);
        const d2 = Time.initC(rhs.*);
        return switch (d1.order(d2)) {
            .lt => -1,
            .eq => 0,
            .gt => 1,
        };
    }

    export fn fimo_time_duration_since(
        time_point: *const compat.Time,
        earlier_time_point: *const compat.Time,
        out: *compat.Duration,
    ) AnyResult {
        const t1 = Time.initC(time_point.*);
        const t2 = Time.initC(earlier_time_point.*);
        if (t1.durationSince(t2)) |dur| {
            out.* = dur.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_time_add(
        time_point: *const compat.Time,
        duration: *const compat.Duration,
        out: *compat.Time,
    ) AnyResult {
        const t = Time.initC(time_point.*);
        const d = Duration.initC(duration.*);
        if (t.add(d)) |shifted| {
            out.* = shifted.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_time_saturating_add(
        time_point: *const compat.Time,
        duration: *const compat.Duration,
    ) compat.Time {
        const t = Time.initC(time_point.*);
        const d = Duration.initC(duration.*);
        return t.addSaturating(d).intoC();
    }

    export fn fimo_time_sub(
        time_point: *const compat.Time,
        duration: *const compat.Duration,
        out: *compat.Time,
    ) AnyResult {
        const t = Time.initC(time_point.*);
        const d = Duration.initC(duration.*);
        if (t.sub(d)) |shifted| {
            out.* = shifted.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_time_saturating_sub(
        time_point: *const compat.Time,
        duration: *const compat.Duration,
    ) compat.Time {
        const t = Time.initC(time_point.*);
        const d = Duration.initC(duration.*);
        return t.subSaturating(d).intoC();
    }

    export fn fimo_instant_now() compat.Instant {
        return Instant.now().intoC();
    }

    export fn fimo_instant_elapsed(
        time_point: *const compat.Instant,
        out: *compat.Duration,
    ) AnyResult {
        const t = Instant.initC(time_point.*);
        if (t.elapsed()) |dur| {
            out.* = dur.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_instant_cmp(lhs: *const compat.Instant, rhs: *const compat.Instant) i32 {
        const d1 = Instant.initC(lhs.*);
        const d2 = Instant.initC(rhs.*);
        return switch (d1.order(d2)) {
            .lt => -1,
            .eq => 0,
            .gt => 1,
        };
    }

    export fn fimo_instant_duration_since(
        time_point: *const compat.Instant,
        earlier_time_point: *const compat.Instant,
        out: *compat.Duration,
    ) AnyResult {
        const t1 = Instant.initC(time_point.*);
        const t2 = Instant.initC(earlier_time_point.*);
        if (t1.durationSince(t2)) |dur| {
            out.* = dur.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_instant_add(
        time_point: *const compat.Instant,
        duration: *const compat.Duration,
        out: *compat.Instant,
    ) AnyResult {
        const t = Instant.initC(time_point.*);
        const d = Duration.initC(duration.*);
        if (t.add(d)) |shifted| {
            out.* = shifted.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_instant_saturating_add(
        time_point: *const compat.Instant,
        duration: *const compat.Duration,
    ) compat.Instant {
        const t = Instant.initC(time_point.*);
        const d = Duration.initC(duration.*);
        return t.addSaturating(d).intoC();
    }

    export fn fimo_instant_sub(
        time_point: *const compat.Instant,
        duration: *const compat.Duration,
        out: *compat.Instant,
    ) AnyResult {
        const t = Instant.initC(time_point.*);
        const d = Duration.initC(duration.*);
        if (t.sub(d)) |shifted| {
            out.* = shifted.intoC();
            return AnyResult.ok;
        } else |err| return AnyError.initError(err).intoResult();
    }

    export fn fimo_instant_saturating_sub(
        time_point: *const compat.Instant,
        duration: *const compat.Duration,
    ) compat.Instant {
        const t = Instant.initC(time_point.*);
        const d = Duration.initC(duration.*);
        return t.subSaturating(d).intoC();
    }
};

comptime {
    _ = ffi;
}
