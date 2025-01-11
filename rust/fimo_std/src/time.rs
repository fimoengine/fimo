//! Time utilities.
use crate::error::{AnyError, AnyResult};
use core::ops::{Add, AddAssign, Sub, SubAssign};
use std::mem::MaybeUninit;

unsafe extern "C" {
    fn fimo_duration_from_seconds(secs: u64) -> Duration;
    fn fimo_duration_from_millis(millis: u64) -> Duration;
    fn fimo_duration_from_nanos(nanos: u64) -> Duration;
    fn fimo_duration_as_secs(duration: &Duration) -> u64;
    fn fimo_duration_subsec_millis(duration: &Duration) -> u32;
    fn fimo_duration_subsec_micros(duration: &Duration) -> u32;
    fn fimo_duration_subsec_nanos(duration: &Duration) -> u32;
    fn fimo_duration_as_millis(duration: &Duration, high: Option<&mut MaybeUninit<u32>>) -> u64;
    fn fimo_duration_as_micros(duration: &Duration, high: Option<&mut MaybeUninit<u32>>) -> u64;
    fn fimo_duration_as_nanos(duration: &Duration, high: Option<&mut MaybeUninit<u32>>) -> u64;
    fn fimo_duration_add(
        lhs: &Duration,
        rhs: &Duration,
        out: &mut MaybeUninit<Duration>,
    ) -> AnyResult;
    fn fimo_duration_saturating_add(lhs: &Duration, rhs: &Duration) -> Duration;
    fn fimo_duration_sub(
        lhs: &Duration,
        rhs: &Duration,
        out: &mut MaybeUninit<Duration>,
    ) -> AnyResult;
    fn fimo_duration_saturating_sub(lhs: &Duration, rhs: &Duration) -> Duration;
}

/// A span between to points in time.
#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Duration {
    pub secs: u64,
    // Must be in [0, 999999999]
    pub nanos: u32,
}

impl Duration {
    /// The maximum duration.
    pub const MAX: Self = Self::new(u64::MAX, 999999999);

    /// The duration of one millisecond.
    pub const SECOND: Self = Self::new(1, 0);

    /// The duration of one millisecond.
    pub const MILLISECOND: Self = Self::new(0, 1000000);

    /// The duration of one microsecond.
    pub const MICROSECOND: Self = Self::new(0, 1000);

    /// The duration of one nanosecond.
    pub const NANOSECOND: Self = Self::new(0, 1);

    /// The zero duration.
    pub const ZERO: Self = Self::new(0, 0);

    /// Creates a new `Duration` from the specified number of whole seconds and
    /// additional nanoseconds.
    ///
    /// # Panics
    ///
    /// If the number of nanoseconds is greater or equal to 1 billion (the number of
    /// nanoseconds in a second), then it will panic.
    pub const fn new(secs: u64, nanos: u32) -> Self {
        if nanos > 999999999 {
            panic!("overflow in Duration::new")
        }

        Self { secs, nanos }
    }

    /// Constructs a `Duration` from seconds.
    pub fn from_seconds(seconds: u64) -> Self {
        unsafe { fimo_duration_from_seconds(seconds) }
    }

    /// Constructs a `Duration` from milliseconds.
    pub fn from_millis(milliseconds: u64) -> Self {
        unsafe { fimo_duration_from_millis(milliseconds) }
    }

    /// Constructs a `Duration` from nanoseconds.
    pub fn from_nanos(nanoseconds: u64) -> Self {
        unsafe { fimo_duration_from_nanos(nanoseconds) }
    }

    /// Returns the number of whole seconds in the `Duration`.
    pub fn as_secs(&self) -> u64 {
        unsafe { fimo_duration_as_secs(self) }
    }

    /// Returns the fractional part in whole milliseconds.
    pub fn subsec_millis(&self) -> u32 {
        unsafe { fimo_duration_subsec_millis(self) }
    }

    /// Returns the fractional part in whole microseconds.
    pub fn subsec_micros(&self) -> u32 {
        unsafe { fimo_duration_subsec_micros(self) }
    }

    /// Returns the fractional part in whole nanoseconds.
    pub fn subsec_nanos(&self) -> u32 {
        unsafe { fimo_duration_subsec_nanos(self) }
    }

    /// Returns the whole milliseconds in a duration.
    pub fn as_millis(&self) -> u128 {
        let mut high = MaybeUninit::uninit();
        unsafe {
            let low = fimo_duration_as_millis(self, Some(&mut high));
            let high = high.assume_init();
            ((high as u128) << 64) | (low as u128)
        }
    }

    /// Returns the whole microseconds in a duration.
    pub fn as_micros(&self) -> u128 {
        let mut high = MaybeUninit::uninit();
        unsafe {
            let low = fimo_duration_as_micros(self, Some(&mut high));
            let high = high.assume_init();
            ((high as u128) << 64) | (low as u128)
        }
    }

    /// Returns the whole nanoseconds in a duration.
    pub fn as_nanos(&self) -> u128 {
        let mut high = MaybeUninit::uninit();
        unsafe {
            let low = fimo_duration_as_nanos(self, Some(&mut high));
            let high = high.assume_init();
            ((high as u128) << 64) | (low as u128)
        }
    }

    /// Returns true if this `Duration` spans no time.
    pub const fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    /// Offsets the duration point forwards.
    ///
    /// Returns `Some(d)` where `d` is the duration `self + duration` if `d` can be represented as
    /// `Duration`, `None` otherwise.
    pub fn checked_add(&self, duration: Self) -> Option<Self> {
        let mut out = MaybeUninit::uninit();
        unsafe {
            fimo_duration_add(self, &duration, &mut out)
                .into_result()
                .map(|_| out.assume_init())
                .ok()
        }
    }

    /// Offsets the duration point forwards.
    ///
    /// Returns `d` where `d` is the duration `self + duration` if `d` can be represented as
    /// `Duration`, `MAX` otherwise.
    pub fn saturating_add(&self, duration: Self) -> Self {
        unsafe { fimo_duration_saturating_add(self, &duration) }
    }

    /// Offsets the duration point backwards.
    ///
    /// Returns `Some(d)` where `d` is the duration `self - duration` if `d` can be represented as
    /// `Duration`, `None` otherwise.
    pub fn checked_sub(&self, duration: Self) -> Option<Self> {
        let mut out = MaybeUninit::uninit();
        unsafe {
            fimo_duration_sub(self, &duration, &mut out)
                .into_result()
                .map(|_| out.assume_init())
                .ok()
        }
    }

    /// Offsets the duration point backwards.
    ///
    /// Returns `d` where `d` is the duration `self - duration` if `d` can be represented as
    /// `Duration`, `ZERO` otherwise.
    pub fn saturating_sub(&self, duration: Self) -> Self {
        unsafe { fimo_duration_saturating_sub(self, &duration) }
    }
}

impl Add for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs).unwrap()
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.checked_add(rhs).unwrap();
    }
}

impl Sub for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs).unwrap()
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.checked_sub(rhs).unwrap();
    }
}

unsafe extern "C" {
    fn fimo_time_now() -> Time;
    fn fimo_time_elapsed(time: &Time, out: &mut MaybeUninit<Duration>) -> AnyResult;
    fn fimo_time_duration_since(
        time: &Time,
        earlier: &Time,
        out: &mut MaybeUninit<Duration>,
    ) -> AnyResult;
    fn fimo_time_add(time: &Time, duration: &Duration, out: &mut MaybeUninit<Time>) -> AnyResult;
    fn fimo_time_saturating_add(time: &Time, duration: &Duration) -> Time;
    fn fimo_time_sub(time: &Time, duration: &Duration, out: &mut MaybeUninit<Time>) -> AnyResult;
    fn fimo_time_saturating_sub(time: &Time, duration: &Duration) -> Time;
}

/// A point in time since the unix epoch.
#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Time {
    pub secs: u64,
    // Must be in [0, 999999999]
    pub nanos: u32,
}

impl Time {
    /// The unix epoch.
    pub const UNIX_EPOCH: Self = Self { secs: 0, nanos: 0 };

    /// Maximum time point.
    pub const MAX_TIME_POINT: Self = Self {
        secs: u64::MAX,
        nanos: 999999999,
    };

    /// Returns the current time.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::time::Time;
    ///
    /// let time = Time::now();
    /// ```
    pub fn now() -> Self {
        unsafe { fimo_time_now() }
    }

    /// Returns the duration elapsed since the time.
    ///
    /// May result in an error, if a time shift caused `self` to be in the future.
    pub fn elapsed(&self) -> Result<Duration, AnyError> {
        let mut out = MaybeUninit::uninit();
        unsafe {
            fimo_time_elapsed(self, &mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Returns the difference between two time points.
    ///
    /// Returns an error if `self` is after `other`.
    pub fn duration_since(&self, other: &Self) -> Result<Duration, AnyError> {
        let mut out = MaybeUninit::uninit();
        unsafe {
            fimo_time_duration_since(self, other, &mut out).into_result()?;
            Ok(out.assume_init())
        }
    }

    /// Offsets the time point forwards.
    ///
    /// Returns `Some(t)` where t is the time `self + duration` if `t` can be represented as `Time`,
    /// `None` otherwise.
    pub fn checked_add(&self, duration: Duration) -> Option<Self> {
        let mut out = MaybeUninit::uninit();
        unsafe {
            fimo_time_add(self, &duration, &mut out)
                .into_result()
                .map(|_| out.assume_init())
                .ok()
        }
    }

    /// Offsets the time point forwards.
    ///
    /// Returns `t` where t is the time `self + duration` if `t` can be represented as `Time`,
    /// `MAX_TIME_POINT` otherwise.
    pub fn saturating_add(&self, duration: Duration) -> Self {
        unsafe { fimo_time_saturating_add(self, &duration) }
    }

    /// Offsets the time point backwards.
    ///
    /// Returns `Some(t)` where t is the time `self - duration` if `t` can be represented as `Time`,
    /// `None` otherwise.
    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        let mut out = MaybeUninit::uninit();
        unsafe {
            fimo_time_sub(self, &duration, &mut out)
                .into_result()
                .map(|_| out.assume_init())
                .ok()
        }
    }

    /// Offsets the time point backwards.
    ///
    /// Returns `t` where t is the time `self - duration` if `t` can be represented as `Time`,
    /// `UNIX_EPOCH` otherwise.
    pub fn saturating_sub(&self, duration: Duration) -> Self {
        unsafe { fimo_time_saturating_sub(self, &duration) }
    }
}

impl Add<Duration> for Time {
    type Output = Time;

    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs).unwrap()
    }
}

impl AddAssign<Duration> for Time {
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.checked_add(rhs).unwrap();
    }
}

impl Sub<Duration> for Time {
    type Output = Time;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs).unwrap()
    }
}

impl SubAssign<Duration> for Time {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.checked_sub(rhs).unwrap();
    }
}
