import time
import pytest

from ..error import Error
from ..time import Duration, Time


def test_duration_zero():
    duration = Duration.zero()
    assert isinstance(duration, Duration)
    assert duration.as_secs() == 0
    assert duration.subsec_millis() == 0
    assert duration.subsec_micros() == 0
    assert duration.subsec_nanos() == 0
    assert duration.as_millis() == 0
    assert duration.as_micros() == 0
    assert duration.as_nanos() == 0
    assert duration.is_zero()


def test_duration_max():
    duration = Duration.max()
    assert isinstance(duration, Duration)
    assert duration.as_secs() == (1 << 64) - 1
    assert duration.subsec_millis() == 999
    assert duration.subsec_micros() == 999_999
    assert duration.subsec_nanos() == 999_999_999
    assert duration.as_millis() == ((1 << 64) - 1) * 1_000 + 999
    assert duration.as_micros() == ((1 << 64) - 1) * 1_000_000 + 999999
    assert duration.as_nanos() == ((1 << 64) - 1) * 1_000_000_000 + 999999999
    assert not duration.is_zero()


def test_duration_from_seconds():
    duration = Duration.from_seconds(123_456)
    assert isinstance(duration, Duration)
    assert duration.as_secs() == 123_456
    assert duration.subsec_millis() == 0
    assert duration.subsec_micros() == 0
    assert duration.subsec_nanos() == 0
    assert duration.as_millis() == 123_456_000
    assert duration.as_micros() == 123_456_000_000
    assert duration.as_nanos() == 123_456_000_000_000
    assert not duration.is_zero()


def test_duration_from_millis():
    duration = Duration.from_millis(123_456_001)
    assert isinstance(duration, Duration)
    assert duration.as_secs() == 123_456
    assert duration.subsec_millis() == 1
    assert duration.subsec_micros() == 1_000
    assert duration.subsec_nanos() == 1_000_000
    assert duration.as_millis() == 123_456_001
    assert duration.as_micros() == 123_456_001_000
    assert duration.as_nanos() == 123_456_001_000_000
    assert not duration.is_zero()


def test_duration_from_nanos():
    duration = Duration.from_nanos(123_456_000_000_100)
    assert isinstance(duration, Duration)
    assert duration.as_secs() == 123_456
    assert duration.subsec_millis() == 0
    assert duration.subsec_micros() == 0
    assert duration.subsec_nanos() == 100
    assert duration.as_millis() == 123_456_000
    assert duration.as_micros() == 123_456_000_000
    assert duration.as_nanos() == 123_456_000_000_100
    assert not duration.is_zero()


def test_duration_add():
    a = Duration.from_millis(700)
    b = Duration.from_millis(1200)
    duration = a + b
    assert isinstance(duration, Duration)
    assert duration.as_secs() == 1
    assert duration.subsec_millis() == 900
    assert duration.subsec_micros() == 900_000
    assert duration.subsec_nanos() == 900_000_000
    assert duration.as_millis() == 1_900
    assert duration.as_micros() == 1_900_000
    assert duration.as_nanos() == 1_900_000_000
    assert not duration.is_zero()

    with pytest.raises(Error):
        Duration.max() + a


def test_duration_saturating_add():
    a = Duration.from_millis(700)
    duration = Duration.max().saturating_add(a)
    assert isinstance(duration, Duration)
    assert duration == Duration.max()
    assert not duration.is_zero()


def test_duration_sub():
    a = Duration.from_millis(2200)
    b = Duration.from_millis(700)
    duration = a - b
    assert isinstance(duration, Duration)
    assert duration.as_secs() == 1
    assert duration.subsec_millis() == 500
    assert duration.subsec_micros() == 500_000
    assert duration.subsec_nanos() == 500_000_000
    assert duration.as_millis() == 1_500
    assert duration.as_micros() == 1_500_000
    assert duration.as_nanos() == 1_500_000_000
    assert not duration.is_zero()

    with pytest.raises(Error):
        Duration.zero() - a


def test_duration_saturating_sub():
    a = Duration.from_millis(700)
    duration = Duration.zero().saturating_sub(a)
    assert isinstance(duration, Duration)
    assert duration == Duration.zero()
    assert duration.is_zero()


def test_time_now():
    now = Time.now()
    assert isinstance(now, Time)

    time.sleep(2)
    elapsed = now.elapsed()
    assert isinstance(elapsed, Duration)
    assert elapsed.as_secs() >= 2

    earlier = now
    now = Time.now()
    elapsed = now.duration_since(earlier)
    assert isinstance(elapsed, Duration)
    assert elapsed.as_secs() >= 2

    with pytest.raises(Error):
        earlier.duration_since(now)


def test_time_add():
    a = Time.now()
    b = Duration.from_seconds(2)
    t = a + b
    assert isinstance(t, Time)
    assert (
        t.duration_since(t.unix_epoch()).as_secs()
        == a.duration_since(t.unix_epoch()).as_secs() + 2
    )

    with pytest.raises(Error):
        Time.max() + b


def test_time_saturating_add():
    a = Duration.from_seconds(2)
    t = Time.max().saturating_add(a)
    assert isinstance(t, Time)
    assert t == t.max()


def test_time_sub():
    a = Time.now()
    b = Duration.from_seconds(2)
    t = a - b
    assert isinstance(t, Time)
    assert (
        t.duration_since(t.unix_epoch()).as_secs()
        == a.duration_since(t.unix_epoch()).as_secs() - 2
    )

    with pytest.raises(Error):
        Time.unix_epoch() - b


def test_time_saturating_sub():
    a = Duration.from_seconds(2)
    t = Time.unix_epoch().saturating_sub(a)
    assert isinstance(t, Time)
    assert t == t.unix_epoch()
