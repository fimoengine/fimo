//! Time utilities.
use crate::{
    bindings,
    error::{to_result_indirect_in_place, AnyError},
    ffi::{FFISharable, FFITransferable},
};
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// A span between to points in time.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Duration(bindings::FimoDuration);

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

        Self(bindings::FimoDuration { secs, nanos })
    }

    /// Constructs a `Duration` from seconds.
    pub fn from_seconds(seconds: u64) -> Self {
        // Safety: FFI call is safe.
        unsafe { Self(bindings::fimo_duration_from_seconds(seconds)) }
    }

    /// Constructs a `Duration` from milliseconds.
    pub fn from_millis(milliseconds: u64) -> Self {
        // Safety: FFI call is safe.
        unsafe { Self(bindings::fimo_duration_from_millis(milliseconds)) }
    }

    /// Constructs a `Duration` from nanoseconds.
    pub fn from_nanos(nanoseconds: u64) -> Self {
        // Safety: FFI call is safe.
        unsafe { Self(bindings::fimo_duration_from_nanos(nanoseconds)) }
    }

    /// Returns the number of whole seconds in the `Duration`.
    pub fn as_secs(&self) -> u64 {
        // Safety: FFI call is safe.
        unsafe { bindings::fimo_duration_as_secs(&self.0) }
    }

    /// Returns the fractional part in whole milliseconds.
    pub fn subsec_millis(&self) -> u32 {
        // Safety: FFI call is safe.
        unsafe { bindings::fimo_duration_subsec_millis(&self.0) }
    }

    /// Returns the fractional part in whole microseconds.
    pub fn subsec_micros(&self) -> u32 {
        // Safety: FFI call is safe.
        unsafe { bindings::fimo_duration_subsec_micros(&self.0) }
    }

    /// Returns the fractional part in whole nanoseconds.
    pub fn subsec_nanos(&self) -> u32 {
        // Safety: FFI call is safe.
        unsafe { bindings::fimo_duration_subsec_nanos(&self.0) }
    }

    /// Returns the whole milliseconds in a duration.
    pub fn as_millis(&self) -> u128 {
        let mut high = 0;
        // Safety: FFI call is safe.
        let low = unsafe { bindings::fimo_duration_as_millis(&self.0, &mut high) };
        ((high as u128) << 64) | (low as u128)
    }

    /// Returns the whole microseconds in a duration.
    pub fn as_micros(&self) -> u128 {
        let mut high = 0;
        // Safety: FFI call is safe.
        let low = unsafe { bindings::fimo_duration_as_micros(&self.0, &mut high) };
        ((high as u128) << 64) | (low as u128)
    }

    /// Returns the whole nanoseconds in a duration.
    pub fn as_nanos(&self) -> u128 {
        let mut high = 0;
        // Safety: FFI call is safe.
        let low = unsafe { bindings::fimo_duration_as_nanos(&self.0, &mut high) };
        ((high as u128) << 64) | (low as u128)
    }

    /// Returns `Some(d)` where `d` is the duration `self + duration` if `d` can be represented as
    /// `Duration`, `None` otherwise.
    pub fn checked_add(&self, duration: Self) -> Option<Self> {
        // Safety: FFI call is safe.
        let res = unsafe {
            to_result_indirect_in_place(|error, time| {
                *error = bindings::fimo_duration_add(&self.0, &duration.0, time.as_mut_ptr());
            })
        };
        match res {
            Ok(x) => Some(Self(x)),
            Err(_) => None,
        }
    }

    /// Returns `d` where `d` is the duration `self + duration` if `d` can be represented as
    /// `Duration`, `MAX` otherwise.
    pub fn saturating_add(&self, duration: Self) -> Self {
        // Safety: FFI call is safe.
        let res = unsafe { bindings::fimo_duration_saturating_add(&self.0, &duration.0) };
        Self(res)
    }

    /// Returns `Some(d)` where `d` is the duration `self - duration` if `d` can be represented as
    /// `Duration`, `None` otherwise.
    pub fn checked_sub(&self, duration: Self) -> Option<Self> {
        // Safety: FFI call is safe.
        let res = unsafe {
            to_result_indirect_in_place(|error, time| {
                *error = bindings::fimo_duration_sub(&self.0, &duration.0, time.as_mut_ptr());
            })
        };
        match res {
            Ok(x) => Some(Self(x)),
            Err(_) => None,
        }
    }

    /// Returns `d` where `d` is the duration `self - duration` if `d` can be represented as
    /// `Duration`, `ZERO` otherwise.
    pub fn saturating_sub(&self, duration: Self) -> Self {
        // Safety: FFI call is safe.
        let res = unsafe { bindings::fimo_duration_saturating_sub(&self.0, &duration.0) };
        Self(res)
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

impl FFISharable<bindings::FimoDuration> for Duration {
    type BorrowedView<'a> = Duration;

    fn share_to_ffi(&self) -> bindings::FimoDuration {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoDuration) -> Self::BorrowedView<'a> {
        Self(ffi)
    }
}

impl FFITransferable<bindings::FimoDuration> for Duration {
    fn into_ffi(self) -> bindings::FimoDuration {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoDuration) -> Self {
        Self(ffi)
    }
}

/// A point in time since the unix epoch.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Time(bindings::FimoTime);

impl Time {
    /// The unix epoch.
    pub const UNIX_EPOCH: Self = Self(bindings::FimoTime { secs: 0, nanos: 0 });

    /// Maximum time point.
    pub const MAX_TIME_POINT: Self = Self(bindings::FimoTime {
        secs: u64::MAX,
        nanos: 999999999,
    });

    /// Returns the current time.
    pub fn now() -> Self {
        // Safety: FFI call is safe.
        unsafe { Self(bindings::fimo_time_now()) }
    }

    /// Returns the duration elapsed since the time.
    ///
    /// May result in an error, if a time shift caused `self` to be in the future.
    pub fn elapsed(&self) -> Result<Duration, AnyError> {
        // Safety: FFI call is safe.
        let duration = unsafe {
            to_result_indirect_in_place(|error, duration| {
                *error = bindings::fimo_time_elapsed(&self.0, duration.as_mut_ptr());
            })?
        };
        Ok(Duration(duration))
    }

    /// Returns the difference between two time points.
    ///
    /// Returns an error if `self` is after `other`.
    pub fn duration_since(&self, other: &Self) -> Result<Duration, AnyError> {
        // Safety: FFI call is safe.
        let duration = unsafe {
            to_result_indirect_in_place(|error, duration| {
                *error =
                    bindings::fimo_time_duration_since(&self.0, &other.0, duration.as_mut_ptr());
            })?
        };
        Ok(Duration(duration))
    }

    /// Returns `Some(t)` where t is the time `self + duration` if `t` can be represented as `Time`,
    /// `None` otherwise.
    pub fn checked_add(&self, duration: Duration) -> Option<Self> {
        // Safety: FFI call is safe.
        let time = unsafe {
            to_result_indirect_in_place(|error, time| {
                *error = bindings::fimo_time_add(&self.0, &duration.0, time.as_mut_ptr());
            })
        };
        match time {
            Ok(x) => Some(Self(x)),
            Err(_) => None,
        }
    }

    /// Returns `t` where t is the time `self + duration` if `t` can be represented as `Time`,
    /// `MAX_TIME_POINT` otherwise.
    pub fn saturating_add(&self, duration: Duration) -> Self {
        // Safety: FFI call is safe.
        let time = unsafe { bindings::fimo_time_saturating_add(&self.0, &duration.0) };
        Self(time)
    }

    /// Returns `Some(t)` where t is the time `self - duration` if `t` can be represented as `Time`,
    /// `None` otherwise.
    pub fn checked_sub(&self, duration: Duration) -> Option<Self> {
        // Safety: FFI call is safe.
        let time = unsafe {
            to_result_indirect_in_place(|error, time| {
                *error = bindings::fimo_time_sub(&self.0, &duration.0, time.as_mut_ptr());
            })
        };
        match time {
            Ok(x) => Some(Self(x)),
            Err(_) => None,
        }
    }

    /// Returns `t` where t is the time `self - duration` if `t` can be represented as `Time`,
    /// `UNIX_EPOCH` otherwise.
    pub fn saturating_sub(&self, duration: Duration) -> Self {
        // Safety: FFI call is safe.
        let time = unsafe { bindings::fimo_time_saturating_sub(&self.0, &duration.0) };
        Self(time)
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

impl FFISharable<bindings::FimoTime> for Time {
    type BorrowedView<'a> = Time;

    fn share_to_ffi(&self) -> bindings::FimoTime {
        self.0
    }

    unsafe fn borrow_from_ffi<'a>(ffi: bindings::FimoTime) -> Self::BorrowedView<'a> {
        Self(ffi)
    }
}

impl FFITransferable<bindings::FimoTime> for Time {
    fn into_ffi(self) -> bindings::FimoTime {
        self.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoTime) -> Self {
        Self(ffi)
    }
}
