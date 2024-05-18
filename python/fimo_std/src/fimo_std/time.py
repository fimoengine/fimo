import ctypes as c
from typing import Self

from . import error
from . import ffi as _ffi


class Duration(_ffi.FFITransferable[_ffi.FimoDuration]):
    """A span of time."""

    def __init__(self, secs: int, nanos: int) -> None:
        if not isinstance(secs, int):
            raise TypeError("`secs` must be an `int`")
        if not isinstance(nanos, int):
            raise TypeError("`nanos` must be an `int`")
        if secs < 0 or secs.bit_length() > 64:
            raise ValueError("`secs` must fit in an 64-bit unsigned integer")
        if not 0 <= nanos <= 999_999_999:
            raise ValueError("`nanos` be in the range [0, 999_999_999]")

        self._ffi = _ffi.FimoDuration(secs, nanos)

    def transfer_to_ffi(self) -> _ffi.FimoDuration:
        return self._ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoDuration) -> Self:
        secs = ffi.secs.value
        nanos = ffi.nanos.value
        return cls(secs, nanos)

    @classmethod
    def zero(cls) -> Self:
        """Constructs the zero duration."""
        return cls.transfer_from_ffi(_ffi.FIMO_DURATION_ZERO)

    @classmethod
    def max(cls) -> Self:
        """Constructs the max duration."""
        return cls.transfer_from_ffi(_ffi.FIMO_DURATION_MAX)

    @classmethod
    def from_seconds(cls, seconds: int) -> Self:
        """Constructs a duration from seconds."""
        if not isinstance(seconds, int):
            raise TypeError("`seconds` must be an `int`")
        if seconds < 0 or seconds.bit_length() > 64:
            raise ValueError("`seconds` must fit in an 64-bit unsigned integer")

        ffi = _ffi.fimo_duration_from_seconds(_ffi.FimoU64(seconds))
        return cls.transfer_from_ffi(ffi)

    @classmethod
    def from_millis(cls, milliseconds: int) -> Self:
        """Constructs a duration from milliseconds."""
        if not isinstance(milliseconds, int):
            raise TypeError("`seconds` must be an `int`")
        if milliseconds < 0 or milliseconds.bit_length() > 64:
            raise ValueError("`seconds` must fit in an 64-bit unsigned integer")

        ffi = _ffi.fimo_duration_from_millis(_ffi.FimoU64(milliseconds))
        return cls.transfer_from_ffi(ffi)

    @classmethod
    def from_nanos(cls, nanoseconds: int) -> Self:
        """Constructs a duration from nanoseconds."""
        if not isinstance(nanoseconds, int):
            raise TypeError("`seconds` must be an `int`")
        if nanoseconds < 0 or nanoseconds.bit_length() > 64:
            raise ValueError("`seconds` must fit in an 64-bit unsigned integer")

        ffi = _ffi.fimo_duration_from_nanos(_ffi.FimoU64(nanoseconds))
        return cls.transfer_from_ffi(ffi)

    def is_zero(self) -> bool:
        """Checks if a duration is zero.

        :return: `True` if the duration is zero.
        """
        return _ffi.fimo_duration_is_zero(c.byref(self._ffi))

    def as_secs(self) -> int:
        """Returns the whole seconds in a duration.

        :return: Whole seconds.
        """
        return _ffi.fimo_duration_as_secs(c.byref(self._ffi)).value

    def subsec_millis(self) -> int:
        """Returns the fractional part in milliseconds.

        :return: Fractional part in whole milliseconds
        """
        return _ffi.fimo_duration_subsec_millis(c.byref(self._ffi)).value

    def subsec_micros(self) -> int:
        """Returns the fractional part in microseconds.

        :return: Fractional part in whole microseconds
        """
        return _ffi.fimo_duration_subsec_micros(c.byref(self._ffi)).value

    def subsec_nanos(self) -> int:
        """Returns the fractional part in nanoseconds.

        :return: Fractional part in whole nanoseconds
        """
        return _ffi.fimo_duration_subsec_nanos(c.byref(self._ffi)).value

    def as_millis(self) -> int:
        """Returns the whole milliseconds in a duration.

        :return: Whole milliseconds.
        """
        high = _ffi.FimoU32(0)
        low = _ffi.fimo_duration_as_millis(c.byref(self._ffi), c.byref(high))

        return (high.value << 64) | low.value

    def as_micros(self) -> int:
        """Returns the whole microseconds in a duration.

        :return: Whole microseconds.
        """
        high = _ffi.FimoU32(0)
        low = _ffi.fimo_duration_as_micros(c.byref(self._ffi), c.byref(high))

        return (high.value << 64) | low.value

    def as_nanos(self) -> int:
        """Returns the whole microseconds in a duration.

        :return: Whole microseconds.
        """
        high = _ffi.FimoU32(0)
        low = _ffi.fimo_duration_as_nanos(c.byref(self._ffi), c.byref(high))

        return (high.value << 64) | low.value

    def __add__(self, other: Self) -> Self:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        ffi = _ffi.FimoDuration()
        err = _ffi.fimo_duration_add(c.byref(self._ffi), c.byref(other._ffi), c.byref(ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return Duration.transfer_from_ffi(ffi)

    def saturating_add(self, other: Self) -> Self:
        """Adds two durations.

        The result saturates to `Duration.max()`, if an overflow occurs.

        :return: Added durations.
        :raises TypeError: `other` is not a `Duration` instance.
        """
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        ffi = _ffi.fimo_duration_saturating_add(c.byref(self._ffi), c.byref(other._ffi))
        return Duration.transfer_from_ffi(ffi)

    def __sub__(self, other: Self) -> Self:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        ffi = _ffi.FimoDuration()
        err = _ffi.fimo_duration_sub(c.byref(self._ffi), c.byref(other._ffi), c.byref(ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return Duration.transfer_from_ffi(ffi)

    def saturating_sub(self, other: Self) -> Self:
        """Subtracts two durations.

        The result saturates to `Duration.zero()`, if an overflow occurs or the resulting duration is negative.

        :return: Subtracted durations.
        :raises TypeError: `other` is not a `Duration` instance.
        """
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        ffi = _ffi.fimo_duration_saturating_sub(c.byref(self._ffi), c.byref(other._ffi))
        return Duration.transfer_from_ffi(ffi)

    def __lt__(self, other: Self) -> bool:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        return self.as_nanos() < other.as_nanos()

    def __le__(self, other: Self) -> bool:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        return self.as_nanos() <= other.as_nanos()

    def __eq__(self, other: Self) -> bool:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        return self.as_nanos() == other.as_nanos()

    def __ge__(self, other: Self) -> bool:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        return self.as_nanos() >= other.as_nanos()

    def __gt__(self, other: Self) -> bool:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        return self.as_nanos() > other.as_nanos()


class Time(_ffi.FFITransferable[_ffi.FimoTime]):
    """A point in time since the unix epoch."""

    def __init__(self, secs: int, nanos: int) -> None:
        if not isinstance(secs, int):
            raise TypeError("`secs` must be an `int`")
        if not isinstance(nanos, int):
            raise TypeError("`nanos` must be an `int`")
        if secs < 0 or secs.bit_length() > 64:
            raise ValueError("`secs` must fit in an 64-bit unsigned integer")
        if not 0 <= nanos <= 999_999_999:
            raise ValueError("`nanos` be in the range [0, 999_999_999]")

        self._ffi = _ffi.FimoTime(secs, nanos)

    def transfer_to_ffi(self) -> _ffi.FimoTime:
        return self._ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTime) -> Self:
        secs = ffi.secs.value
        nanos = ffi.nanos.value
        return cls(secs, nanos)

    @classmethod
    def unix_epoch(cls) -> Self:
        """Constructs the UNIX epoch.

        :return: UNIX epoch.
        """
        return cls.transfer_from_ffi(_ffi.FIMO_UNIX_EPOCH)

    @classmethod
    def max(cls) -> Self:
        """Constructs the latest possible time point.

        :return: Latest time point.
        """
        return cls.transfer_from_ffi(_ffi.FIMO_TIME_MAX)

    @classmethod
    def now(cls) -> Self:
        """Constructs the current time.

        :return: Current time.
        """
        ffi = _ffi.fimo_time_now()
        return cls.transfer_from_ffi(ffi)

    def elapsed(self) -> Duration:
        """Returns the duration elapsed since the time point.

        :return: Elapsed time.
        """
        ffi = _ffi.FimoDuration()
        err = _ffi.fimo_time_elapsed(c.byref(self._ffi), c.byref(ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return Duration.transfer_from_ffi(ffi)

    def duration_since(self, earlier: Self) -> Duration:
        """Returns the difference between two time points.

        :raises Error: `earlier` is after `self`.
        :return: Difference between the two time points.
        :raises TypeError: `earlier` is not a `Time` instance.
        """
        if not isinstance(earlier, Time):
            raise TypeError("`earlier` must be a `Time`")

        ffi = _ffi.FimoDuration()
        err = _ffi.fimo_time_duration_since(c.byref(self._ffi), c.byref(earlier._ffi), c.byref(ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return Duration.transfer_from_ffi(ffi)

    def __add__(self, other: Duration) -> Self:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        ffi = _ffi.FimoTime()
        other_ffi = other.transfer_to_ffi()
        err = _ffi.fimo_time_add(c.byref(self._ffi), c.byref(other_ffi), c.byref(ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return Time.transfer_from_ffi(ffi)

    def saturating_add(self, other: Duration) -> Self:
        """Adds two durations.

        The result saturates to `Time.max()`, if an overflow occurs.

        :return: Shifted time point.
        :raises TypeError: `other` is not a `Duration` instance.
        """
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        other_ffi = other.transfer_to_ffi()
        ffi = _ffi.fimo_time_saturating_add(c.byref(self._ffi), c.byref(other_ffi))
        return Time.transfer_from_ffi(ffi)

    def __sub__(self, other: Duration) -> Self:
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        ffi = _ffi.FimoTime()
        other_ffi = other.transfer_to_ffi()
        err = _ffi.fimo_time_sub(c.byref(self._ffi), c.byref(other_ffi), c.byref(ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return Time.transfer_from_ffi(ffi)

    def saturating_sub(self, other: Duration) -> Self:
        """Subtracts two durations.

        The result saturates to `Time.unix_epoch()`, if an overflow occurs or the resulting time is negative.

        :return: Shifted time point.
        :raises TypeError: `other` is not a `Duration` instance.
        """
        if not isinstance(other, Duration):
            raise TypeError("`other` must be a `Duration`")

        other_ffi = other.transfer_to_ffi()
        ffi = _ffi.fimo_time_saturating_sub(c.byref(self._ffi), c.byref(other_ffi))
        return Time.transfer_from_ffi(ffi)

    def __lt__(self, other: Self) -> bool:
        if not isinstance(other, Time):
            raise TypeError("`other` must be a `Time`")

        return self.duration_since(Time.unix_epoch()) < other.duration_since(Time.unix_epoch())

    def __le__(self, other: Self) -> bool:
        if not isinstance(other, Time):
            raise TypeError("`other` must be a `Time`")

        return self.duration_since(Time.unix_epoch()) <= other.duration_since(Time.unix_epoch())

    def __eq__(self, other: Self) -> bool:
        if not isinstance(other, Time):
            raise TypeError("`other` must be a `Time`")

        return self.duration_since(Time.unix_epoch()) == other.duration_since(Time.unix_epoch())

    def __ge__(self, other: Self) -> bool:
        if not isinstance(other, Time):
            raise TypeError("`other` must be a `Time`")

        return self.duration_since(Time.unix_epoch()) >= other.duration_since(Time.unix_epoch())

    def __gt__(self, other: Self) -> bool:
        if not isinstance(other, Time):
            raise TypeError("`other` must be a `Time`")

        return self.duration_since(Time.unix_epoch()) > other.duration_since(Time.unix_epoch())
