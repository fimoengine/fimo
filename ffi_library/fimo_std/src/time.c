#include <fimo_std/time.h>

#if defined(_WIN32) || defined(WIN32)
#include <Windows.h>
#include <intrin.h>
#elif __unix__ || __APPLE__
#include <time.h>
#endif

#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
#define FIMO_ADD_OVERFLOW(lhs, rhs, out) __builtin_add_overflow(lhs, rhs, out)
#elif defined(_WIN32) || defined(WIN32)
#define FIMO_ADD_OVERFLOW(lhs, rhs, out) _addcarry_u64(0, lhs, rhs, out)
#else
#error "Unsupported compiler"
#endif

FIMO_MUST_USE FimoDuration fimo_duration_zero(void)
{
    return FIMO_DURATION_ZERO;
}

FIMO_MUST_USE FimoDuration fimo_duration_max(void)
{
    return FIMO_DURATION_MAX;
}

FIMO_MUST_USE FimoDuration fimo_duration_from_seconds(FimoU64 seconds)
{
    return FIMO_SECONDS(seconds);
}

FIMO_MUST_USE FimoDuration fimo_duration_from_millis(FimoU64 milliseconds)
{
    return FIMO_MILLIS(milliseconds);
}

FIMO_MUST_USE FimoDuration fimo_duration_from_nanos(FimoU64 nanoseconds)
{
    return FIMO_NANOS(nanoseconds);
}

FIMO_MUST_USE bool fimo_duration_is_zero(const FimoDuration* duration)
{
    return duration->secs == 0 && duration->nanos == 0;
}

FIMO_MUST_USE FimoU64 fimo_duration_as_secs(const FimoDuration* duration)
{
    return duration->secs;
}

FIMO_MUST_USE FimoU32 fimo_duration_subsec_millis(const FimoDuration* duration)
{
    return duration->nanos / FIMO_NANOS_PER_MILLIS;
}

FIMO_MUST_USE FimoU32 fimo_duration_subsec_micros(const FimoDuration* duration)
{
    return duration->nanos / FIMO_NANOS_PER_MICROS;
}

FIMO_MUST_USE FimoU32 fimo_duration_subsec_nanos(const FimoDuration* duration)
{
    return duration->nanos;
}

FIMO_MUST_USE FimoU64 fimo_duration_as_millis(const FimoDuration* duration, FimoU32* high)
{
#if defined(_WIN32) || defined(WIN32)
    FimoU64 high_u64 = 0;
    FimoU64 low = _umul128(duration->secs, FIMO_MILLIS_PER_SEC, &high_u64);
    high_u64 += FIMO_ADD_OVERFLOW(low, duration->nanos / FIMO_NANOS_PER_MILLIS, &low);

    if (high) {
        *high = (FimoU32)high_u64;
    }
    return low;
#else
    __extension__ unsigned __int128 nanos = ((unsigned __int128)duration->secs * FIMO_MILLIS_PER_SEC) + duration->nanos;
    __extension__ unsigned __int128 millis = nanos / FIMO_NANOS_PER_MILLIS;

    if (high) {
        *high = (FimoU32)(millis >> 64);
    }
    return (FimoU64)(millis);
#endif
}

FIMO_MUST_USE FimoU64 fimo_duration_as_micros(const FimoDuration* duration, FimoU32* high)
{
#if defined(_WIN32) || defined(WIN32)
    FimoU64 high_u64 = 0;
    FimoU64 low = _umul128(duration->secs, FIMO_MICROS_PER_SEC, &high_u64);
    high_u64 += FIMO_ADD_OVERFLOW(low, duration->nanos / FIMO_NANOS_PER_MICROS, &low);

    if (high) {
        *high = (FimoU32)high_u64;
    }
    return low;
#else
    __extension__ unsigned __int128 nanos = ((unsigned __int128)duration->secs * FIMO_MILLIS_PER_SEC) + duration->nanos;
    __extension__ unsigned __int128 micros = nanos / FIMO_NANOS_PER_MICROS;

    if (high) {
        *high = (FimoU32)(micros >> 64);
    }
    return (FimoU64)(micros);
#endif
}

FIMO_MUST_USE FimoU64 fimo_duration_as_nanos(const FimoDuration* duration, FimoU32* high)
{
#if defined(_WIN32) || defined(WIN32)
    FimoU64 high_u64 = 0;
    FimoU64 low = _umul128(duration->secs, FIMO_NANOS_PER_SEC, &high_u64);
    high_u64 += FIMO_ADD_OVERFLOW(low, duration->nanos, &low);

    if (high) {
        *high = (FimoU32)high_u64;
    }
    return low;
#else
    __extension__ unsigned __int128 nanos = ((unsigned __int128)duration->secs * FIMO_NANOS_PER_SEC) + duration->nanos;

    if (high) {
        *high = (FimoU32)(nanos >> 64);
    }
    return (FimoU64)(nanos);
#endif
}

FIMO_MUST_USE FimoError fimo_duration_add(const FimoDuration* lhs, const FimoDuration* rhs, FimoDuration* out)
{
    FimoU64 secs = 0;
    bool overflow = FIMO_ADD_OVERFLOW(lhs->secs, rhs->secs, &secs);
    if (overflow) {
        return FIMO_ERANGE;
    }

    FimoU32 nanos = lhs->nanos + rhs->nanos;
    if (nanos >= FIMO_NANOS_PER_SEC) {
        nanos -= FIMO_NANOS_PER_SEC;
        overflow = FIMO_ADD_OVERFLOW(secs, 1, &secs);
        if (overflow) {
            return FIMO_ERANGE;
        }
    }

    out->secs = secs;
    out->nanos = nanos;
    return FIMO_EOK;
}

FIMO_MUST_USE FimoDuration fimo_duration_saturating_add(const FimoDuration* lhs, const FimoDuration* rhs)
{
    FimoDuration result;
    FimoError error = fimo_duration_add(lhs, rhs, &result);
    if (FIMO_IS_ERROR(error)) {
        return FIMO_DURATION_MAX;
    } else {
        return result;
    }
}

FIMO_MUST_USE FimoError fimo_duration_sub(const FimoDuration* lhs, const FimoDuration* rhs, FimoDuration* out)
{
    if (lhs->secs < rhs->secs) {
        return FIMO_ERANGE;
    }

    FimoU64 secs = lhs->secs - rhs->secs;
    FimoU32 nanos = lhs->nanos;

    if (nanos >= rhs->nanos) {
        nanos -= rhs->nanos;
    } else if (secs >= 1) {
        secs -= 1;
        nanos += FIMO_NANOS_PER_SEC - rhs->nanos;
    } else {
        return FIMO_ERANGE;
    }

    out->secs = secs;
    out->nanos = nanos;
    return FIMO_EOK;
}

FIMO_MUST_USE FimoDuration fimo_duration_saturating_sub(const FimoDuration* lhs, const FimoDuration* rhs)
{
    FimoDuration result;
    FimoError error = fimo_duration_sub(lhs, rhs, &result);
    if (FIMO_IS_ERROR(error)) {
        return FIMO_DURATION_ZERO;
    } else {
        return result;
    }
}

FIMO_MUST_USE FimoTime fimo_time_now(void)
{
#if defined(_WIN32) || defined(WIN32)
    FILETIME filetime;
    GetSystemTimePreciseAsFileTime(&filetime);

    ULARGE_INTEGER large_integer = (ULARGE_INTEGER) { .u = { .LowPart = filetime.dwLowDateTime, .HighPart = filetime.dwHighDateTime } };
    FimoU64 time_utc = large_integer.QuadPart;
    FimoU64 time_unix = time_utc - ((FimoU64)11644473600 * (FimoU64)FIMO_MICROS_PER_SEC * 10LL); // Offset from Jan. 1, 1601 to Jan. 1, 1970
    return (FimoTime) { .secs = time_unix / (FIMO_MICROS_PER_SEC * 10), .nanos = (time_unix % (FIMO_MICROS_PER_SEC * 10) * 100) };
#elif __unix__ || __APPLE__
    struct timespec tp;
    clock_gettime(CLOCK_REALTIME, &tp);
    return (FimoTime) { .secs = tp.tv_sec, .nanos = tp.tv_nsec };
#endif
}

FIMO_MUST_USE FimoError fimo_time_elapsed(const FimoTime* time_point, FimoDuration* elapsed)
{
    FimoTime now = fimo_time_now();
    return fimo_time_duration_since(&now, time_point, elapsed);
}

FIMO_MUST_USE FimoError fimo_time_duration_since(const FimoTime* time_point, const FimoTime* earlier_time_point, FimoDuration* duration)
{
    if (time_point->secs < earlier_time_point->secs) {
        return FIMO_ERANGE;
    }

    FimoU64 secs = time_point->secs - earlier_time_point->secs;
    FimoU32 nanos = time_point->nanos;

    if (nanos >= earlier_time_point->nanos) {
        nanos -= earlier_time_point->nanos;
    } else if (secs >= 1) {
        secs -= 1;
        nanos += FIMO_NANOS_PER_SEC - earlier_time_point->nanos;
    } else {
        return FIMO_ERANGE;
    }

    duration->secs = secs;
    duration->nanos = nanos;
    return FIMO_EOK;
}

FIMO_MUST_USE FimoError fimo_time_add(const FimoTime* time_point, const FimoDuration* duration, FimoTime* out)
{
    FimoU64 secs = 0;
    bool overflow = FIMO_ADD_OVERFLOW(time_point->secs, duration->secs, &secs);
    if (overflow) {
        return FIMO_ERANGE;
    }

    FimoU32 nanos = time_point->nanos + duration->nanos;
    if (nanos >= FIMO_NANOS_PER_SEC) {
        nanos -= FIMO_NANOS_PER_SEC;
        overflow = FIMO_ADD_OVERFLOW(secs, 1, &secs);
        if (overflow) {
            return FIMO_ERANGE;
        }
    }

    out->secs = secs;
    out->nanos = nanos;
    return FIMO_EOK;
}

FIMO_MUST_USE FimoTime fimo_time_saturating_add(const FimoTime* time_point, const FimoDuration* duration)
{
    FimoTime result;
    FimoError error = fimo_time_add(time_point, duration, &result);
    if (FIMO_IS_ERROR(error)) {
        return FIMO_TIME_MAX;
    } else {
        return result;
    }
}

FIMO_MUST_USE FimoError fimo_time_sub(const FimoTime* time_point, const FimoDuration* duration, FimoTime* out)
{
    if (time_point->secs < duration->secs) {
        return FIMO_ERANGE;
    }

    FimoU64 secs = time_point->secs - duration->secs;
    FimoU32 nanos = time_point->nanos;

    if (nanos >= duration->nanos) {
        nanos -= duration->nanos;
    } else if (secs >= 1) {
        secs -= 1;
        nanos += FIMO_NANOS_PER_SEC - duration->nanos;
    } else {
        return FIMO_ERANGE;
    }

    out->secs = secs;
    out->nanos = nanos;
    return FIMO_EOK;
}

FIMO_MUST_USE FimoTime fimo_time_saturating_sub(const FimoTime* time_point, const FimoDuration* duration)
{
    FimoTime result;
    FimoError error = fimo_time_sub(time_point, duration, &result);
    if (FIMO_IS_ERROR(error)) {
        return FIMO_UNIX_EPOCH;
    } else {
        return result;
    }
}
