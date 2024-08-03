#ifndef FIMO_TIME_H
#define FIMO_TIME_H

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Number of milliseconds per second.
 */
#define FIMO_MILLIS_PER_SEC 1000

/**
 * Number of microseconds per second.
 */
#define FIMO_MICROS_PER_SEC 1000000

/**
 * Number of nanoseconds per second.
 */
#define FIMO_NANOS_PER_SEC 1000000000

/**
 * Number of nanoseconds per millisecond.
 */
#define FIMO_NANOS_PER_MILLIS 1000000

/**
 * Number of nanoseconds per second.
 */
#define FIMO_NANOS_PER_MICROS 1000

/**
 * A span of time.
 */
typedef struct FimoDuration {
    /* Number of seconds */
    FimoU64 secs;
    /* Number of nanoseconds, must be in [0, 999999999] */
    FimoU32 nanos;
} FimoDuration;

/**
 * A point in time since the unix epoch.
 */
typedef struct FimoTime {
    /* Number of seconds */
    FimoU64 secs;
    /* Number of nanoseconds, must be in [0, 999999999] */
    FimoU32 nanos;
} FimoTime;

/**
 * A monotonic point in time.
 *
 * The starting point is undefined.
 */
typedef struct FimoTimeMonotonic {
    /* Number of seconds */
    FimoU64 secs;
    /* Number of nanoseconds, must be in [0, 999999999] */
    FimoU32 nanos;
} FimoTimeMonotonic;

/**
 * Constructs a duration.
 *
 * @param seconds number of seconds
 */
#define FIMO_SECONDS(seconds)                                                                                          \
    (FimoDuration) { .secs = (seconds), .nanos = 0 }

/**
 * Constructs a duration.
 *
 * @param milli_seconds number of milliseconds
 */
#define FIMO_MILLIS(milli_seconds)                                                                                     \
    (FimoDuration) {                                                                                                   \
        .secs = (milli_seconds) / FIMO_MILLIS_PER_SEC,                                                                 \
        .nanos = ((milli_seconds) % FIMO_MILLIS_PER_SEC) * FIMO_NANOS_PER_MILLIS                                       \
    }

/**
 * Constructs a duration.
 *
 * @param micro_seconds number of microseconds
 */
#define FIMO_MICROS(micro_seconds)                                                                                     \
    (FimoDuration) {                                                                                                   \
        .secs = (micro_seconds) / FIMO_MICROS_PER_SEC,                                                                 \
        .nanos = ((micro_seconds) % FIMO_MICROS_PER_SEC) * FIMO_NANOS_PER_MICROS                                       \
    }

/**
 * Constructs a duration.
 *
 * @param nano_seconds number of nanoseconds
 */
#define FIMO_NANOS(nano_seconds)                                                                                       \
    (FimoDuration) { .secs = (nano_seconds) / FIMO_NANOS_PER_SEC, .nanos = ((nano_seconds) % FIMO_NANOS_PER_SEC) }

/**
 * Constructs a zero duration.
 */
#define FIMO_DURATION_ZERO FIMO_SECONDS(0)

/**
 * Constructs the maximum duration.
 */
#define FIMO_DURATION_MAX                                                                                              \
    (FimoDuration) { .secs = UINT64_MAX, .nanos = 999999999 }

/**
 * The UNIX epoch.
 */
#define FIMO_UNIX_EPOCH                                                                                                \
    (FimoTime) { .secs = 0, .nanos = 0 }

/**
 * Constructs the latest possible time point.
 */
#define FIMO_TIME_MAX                                                                                                  \
    (FimoTime) { .secs = UINT64_MAX, .nanos = 999999999 }

/**
 * Constructs the latest possible monotonic time point.
 */
#define FIMO_TIME_MONOTONIC_MAX                                                                                        \
    (FimoTimeMonotonic) { .secs = UINT64_MAX, .nanos = 999999999 }

/**
 * Constructs the zero duration.
 *
 * @return Zero duration.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoDuration fimo_duration_zero(void);

/**
 * Constructs the max duration.
 *
 * @return Max duration.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoDuration fimo_duration_max(void);

/**
 * Constructs a duration from seconds.
 *
 * @return Duration.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoDuration fimo_duration_from_seconds(FimoU64 seconds);

/**
 * Constructs a duration from milliseconds.
 *
 * @return Duration.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoDuration fimo_duration_from_millis(FimoU64 milliseconds);

/**
 * Constructs a duration from nanoseconds.
 *
 * @return Duration.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoDuration fimo_duration_from_nanos(FimoU64 nanoseconds);

/**
 * Checks if a duration is zero.
 *
 * @param duration: duration (not `NULL`)
 *
 * @return `true` if the duration is zero.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_duration_is_zero(const FimoDuration *duration);

/**
 * Returns the whole seconds in a duration.
 *
 * @param duration: duration (not `NULL`)
 *
 * @return Whole seconds.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoU64 fimo_duration_as_secs(const FimoDuration *duration);

/**
 * Returns the fractional part in milliseconds.
 *
 * @param duration: duration (not `NULL`)
 *
 * @return Fractional part in whole milliseconds
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoU32 fimo_duration_subsec_millis(const FimoDuration *duration);

/**
 * Returns the fractional part in microseconds.
 *
 * @param duration: duration (not `NULL`)
 *
 * @return Fractional part in whole microseconds.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoU32 fimo_duration_subsec_micros(const FimoDuration *duration);

/**
 * Returns the fractional part in nanoseconds.
 *
 * @param duration: duration (not `NULL`)
 *
 * @return Fractional part in whole nanoseconds.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoU32 fimo_duration_subsec_nanos(const FimoDuration *duration);

/**
 * Returns the whole milliseconds in a duration.
 *
 * If `high` is not null, it is set to store the overflow portion of the milliseconds.
 *
 * @param duration: duration (not `NULL`)
 * @param high: high part of the milliseconds
 *
 * @return Low part of the milliseconds.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoU64 fimo_duration_as_millis(const FimoDuration *duration, FimoU32 *high);

/**
 * Returns the whole microseconds in a duration.
 *
 * If `high` is not null, it is set to store the overflow portion of the microseconds.
 *
 * @param duration: duration (not `NULL`)
 * @param high: high part of the microseconds
 *
 * @return Low part of the microseconds.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoU64 fimo_duration_as_micros(const FimoDuration *duration, FimoU32 *high);

/**
 * Returns the whole nanoseconds in a duration.
 *
 * If `high` is not null, it is set to store the overflow portion of the nanoseconds.
 *
 * @param duration: duration (not `NULL`)
 * @param high: high part of the nanoseconds
 *
 * @return Low part of the nanoseconds.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoU64 fimo_duration_as_nanos(const FimoDuration *duration, FimoU32 *high);

/**
 * Adds two durations.
 *
 * @param lhs: first duration (not `NULL`)
 * @param rhs: second duration (not `NULL`)
 * @param out: resulting duration
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The addition was successful.
 * @error `FIMO_ERANGE`: Addition would result in an overflow.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_duration_add(const FimoDuration *lhs, const FimoDuration *rhs, FimoDuration *out);

/**
 * Adds two durations.
 *
 * The result saturates to `FIMO_DURATION_MAX`, if an overflow occurs.
 *
 * @param lhs: first duration (not `NULL`)
 * @param rhs: second duration (not `NULL`)
 *
 * @return Added durations.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoDuration fimo_duration_saturating_add(const FimoDuration *lhs, const FimoDuration *rhs);

/**
 * Subtracts two durations.
 *
 * @param lhs: first duration (not `NULL`)
 * @param rhs: second duration (not `NULL`)
 * @param out: resulting duration
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The subtraction was successful.
 * @error `FIMO_ERANGE`: The subtraction would result in an overflow or a negative duration.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_duration_sub(const FimoDuration *lhs, const FimoDuration *rhs, FimoDuration *out);

/**
 * Subtracts two durations.
 *
 * The result saturates to `FIMO_DURATION_ZERO`, if an overflow occurs or the resulting duration is negative.
 *
 * @param lhs: first duration (not `NULL`)
 * @param rhs: second duration (not `NULL`)
 *
 * @return Subtracted durations.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoDuration fimo_duration_saturating_sub(const FimoDuration *lhs, const FimoDuration *rhs);

/**
 * Returns the current time.
 *
 * @return Current time.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoTime fimo_time_now(void);

/**
 * Returns the duration elapsed since a prior time point.
 *
 * @param time_point: previous time point (not `NULL`)
 * @param elapsed: resulting duration (not `NULL`)
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The operation was successful.
 * @error `FIMO_ERANGE`: A time shift caused @time_point to be in the future.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_time_elapsed(const FimoTime *time_point, FimoDuration *elapsed);

/**
 * Returns the difference between two time points.
 *
 * @param time_point: first time point (not `NULL`)
 * @param earlier_time_point: second time point (not `NULL`)
 * @param duration: resulting duration (not `NULL`)
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The operation was successful.
 * @error `FIMO_ERANGE`: The time point @time_point is after @earlier_time_point.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_time_duration_since(const FimoTime *time_point, const FimoTime *earlier_time_point,
                                    FimoDuration *duration);

/**
 * Adds a duration to a time point.
 *
 * @param time_point: time point (not `NULL`)
 * @param duration: duration (not `NULL`)
 * @param out: time point
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The addition was successful.
 * @error `FIMO_ERANGE`: The addition would result in an overflow.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_time_add(const FimoTime *time_point, const FimoDuration *duration, FimoTime *out);

/**
 * Adds a duration to a time point.
 *
 * The result saturates to %FIMO_TIME_MAX, if an overflow occurs.
 *
 * @param time_point: time point (not `NULL`)
 * @param duration: duration (not `NULL`)
 *
 * @return Shifted time point.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoTime fimo_time_saturating_add(const FimoTime *time_point, const FimoDuration *duration);

/**
 * Subtracts a duration from a time point.
 *
 * @param time_point: time point (not `NULL`)
 * @param duration: duration (not `NULL`)
 * @param out: time point
 *
 * @return Status code.
 *
 * @error `FIMO_EOK`: The subtraction was successful.
 * @error `FIMO_ERANGE`: The subtraction would result in an overflow.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_time_sub(const FimoTime *time_point, const FimoDuration *duration, FimoTime *out);

/**
 * Subtracts a duration from a time point.
 *
 * The result saturates to `FIMO_UNIX_EPOCH`, if an overflow occurs or the resulting duration is negative.
 *
 * @param time_point: time point (not `NULL`)
 * @param duration: duration (not `NULL`)
 *
 * @return Shifted time point.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoTime fimo_time_saturating_sub(const FimoTime *time_point, const FimoDuration *duration);

#ifdef __cplusplus
}
#endif

#endif // FIMO_TIME_H
