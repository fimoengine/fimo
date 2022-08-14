//! Implementation of the `Result` type.
use crate::{marshal::CTypeBridge, Optional, ReprC, ReprRust};
use std::{
    fmt::Debug,
    ops::{FromResidual, Residual, Try},
};

/// A type that represents either success ([`Result::Ok`]) or failure ([`Result::Err`]).
#[repr(C, i8)]
#[derive(Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
#[must_use = "this `Result` may be an `Err` variant, which should be handled"]
pub enum Result<T, E> {
    /// An `Ok` variant.
    Ok(T),
    /// An `Err` variant.
    Err(E),
}

impl<T, E> Result<T, E> {
    /// Returns `true` if the result contains success.
    #[inline]
    pub const fn is_ok(&self) -> bool {
        matches!(*self, Result::Ok(_))
    }

    /// Returns `true` if the result contains failure.
    #[inline]
    pub const fn is_err(&self) -> bool {
        !self.is_ok()
    }

    /// Converts from `Result<T, E>` to `Option<T>` discarding any error.
    #[inline]
    pub fn ok(self) -> Optional<T> {
        match self {
            Result::Ok(val) => Optional::Some(val),
            Result::Err(_) => Optional::None,
        }
    }

    /// Converts from `Result<T, E>` to `Option<E>` discarding any value.
    #[inline]
    pub fn err(self) -> Optional<E> {
        match self {
            Result::Ok(_) => Optional::None,
            Result::Err(val) => Optional::Some(val),
        }
    }

    /// Converts from `&Result<T, E>` to `Result<&T, &E>`.
    #[inline]
    pub const fn as_ref(&self) -> Result<&T, &E> {
        match *self {
            Result::Ok(ref x) => Result::Ok(x),
            Result::Err(ref x) => Result::Err(x),
        }
    }

    /// Converts from `&Result<T, E>` to `Result<&mut T, &mut E>`.
    #[inline]
    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match *self {
            Result::Ok(ref mut x) => Result::Ok(x),
            Result::Err(ref mut x) => Result::Err(x),
        }
    }

    /// Maps the `Result<T, E>` to `Result<U, E>` by mapping the ok value.
    #[inline]
    pub fn map<U, F>(self, op: F) -> Result<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Result::Ok(x) => Result::Ok(op(x)),
            Result::Err(x) => Result::Err(x),
        }
    }

    /// Maps the ok value of the result by applying f or returning the default value.
    #[inline]
    pub fn map_or<U, F>(self, default: U, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Result::Ok(x) => f(x),
            Result::Err(_) => default,
        }
    }

    /// Maps the `Result<T, E>` to `U` by either applying f to the ok value or
    /// applying default to the error value.
    #[inline]
    pub fn map_or_else<U, D, F>(self, default: D, f: F) -> U
    where
        D: FnOnce(E) -> U,
        F: FnOnce(T) -> U,
    {
        match self {
            Result::Ok(x) => f(x),
            Result::Err(x) => default(x),
        }
    }

    /// Maps the `Result<T, E>` to `Result<T,F>` by mapping the error value.
    #[inline]
    pub fn map_err<F, O>(self, op: O) -> Result<T, F>
    where
        O: FnOnce(E) -> F,
    {
        match self {
            Result::Ok(x) => Result::Ok(x),
            Result::Err(x) => Result::Err(op(x)),
        }
    }

    /// Returns the contained ok value or a provided default.
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(_) => default,
        }
    }

    /// Returns the contained ok value or computes it from a closure.
    #[inline]
    pub fn unwrap_or_else<F>(self, op: F) -> T
    where
        F: FnOnce(E) -> T,
    {
        match self {
            Result::Ok(x) => x,
            Result::Err(x) => op(x),
        }
    }
}

impl<T, E> Result<T, E>
where
    E: Debug,
{
    /// Returns the contained ok value.
    ///
    /// # Panics
    ///
    /// Panics if no ok value is contained, with a panic message including the passed message,
    /// and the content of the error value.
    #[inline]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(x) => panic!("{}: {:?}", msg, x),
        }
    }

    /// Returns the contained ok value.
    ///
    /// # Panics
    ///
    /// Panics if no ok value is contained, with a panic message provided by the error value.
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Result::Ok(x) => x,
            Result::Err(x) => panic!("{:?}", x),
        }
    }
}

impl<T, E> Result<T, E>
where
    T: Debug,
{
    /// Returns the contained error value.
    ///
    /// # Panics
    ///
    /// Panics if no error value is contained, with a panic message including the passed message,
    /// and the content of the ok value.
    #[inline]
    pub fn expect_err(self, msg: &str) -> E {
        match self {
            Result::Ok(x) => panic!("{}: {:?}", msg, x),
            Result::Err(x) => x,
        }
    }

    /// Returns the contained error value.
    ///
    /// # Panics
    ///
    /// Panics if no error value is contained, with a panic message provided by the ok value.
    #[inline]
    pub fn unwrap_err(self) -> E {
        match self {
            Result::Ok(x) => panic!("{:?}", x),
            Result::Err(x) => x,
        }
    }
}

impl<T, E> const Clone for Result<T, E>
where
    T: ~const Clone,
    E: ~const Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Result::Ok(x) => Result::Ok(x.clone()),
            Result::Err(x) => Result::Err(x.clone()),
        }
    }
}

impl<T, E> const From<std::result::Result<T, E>> for Result<T, E> {
    #[inline]
    fn from(val: std::result::Result<T, E>) -> Self {
        match val {
            Ok(x) => Result::Ok(x),
            Err(x) => Result::Err(x),
        }
    }
}

impl<T, E> const From<Result<T, E>> for std::result::Result<T, E> {
    #[inline]
    fn from(val: Result<T, E>) -> Self {
        val.into_rust()
    }
}

unsafe impl<T, E> const CTypeBridge for std::result::Result<T, E>
where
    T: ~const CTypeBridge,
    E: ~const CTypeBridge,
{
    type Type = Result<T::Type, E::Type>;

    fn marshal(self) -> Self::Type {
        match self {
            Ok(x) => Result::Ok(x.marshal()),
            Err(x) => Result::Err(x.marshal()),
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        match x {
            Result::Ok(x) => Ok(T::demarshal(x)),
            Result::Err(x) => Err(E::demarshal(x)),
        }
    }
}

impl<T, E> const ReprC for Result<T, E> {
    type T = std::result::Result<T, E>;

    #[inline]
    fn into_rust(self) -> Self::T {
        match self {
            Result::Ok(v) => Ok(v),
            Result::Err(e) => Err(e),
        }
    }

    #[inline]
    fn from_rust(t: Self::T) -> Self {
        match t {
            Ok(v) => Result::Ok(v),
            Err(e) => Result::Err(e),
        }
    }
}

impl<T, E> const ReprRust for std::result::Result<T, E> {
    type T = Result<T, E>;

    #[inline]
    fn into_c(self) -> Self::T {
        <Result<T, E> as ReprC>::from_rust(self)
    }

    #[inline]
    fn from_c(t: Self::T) -> Self {
        <Result<T, E> as ReprC>::into_rust(t)
    }
}

impl<T, E> const Try for Result<T, E> {
    type Output = T;

    type Residual = Result<std::convert::Infallible, E>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Self::Ok(output)
    }

    #[inline]
    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            Result::Ok(v) => std::ops::ControlFlow::Continue(v),
            Result::Err(e) => std::ops::ControlFlow::Break(Result::Err(e)),
        }
    }
}

impl<T, E> const FromResidual for Result<T, E> {
    #[inline]
    fn from_residual(residual: Result<std::convert::Infallible, E>) -> Self {
        match residual {
            Result::Ok(_) => unreachable!(),
            Result::Err(e) => Result::Err(e),
        }
    }
}

impl<T, E> const Residual<T> for Result<std::convert::Infallible, E> {
    type TryType = Result<T, E>;
}

#[cfg(test)]
mod tests {
    use super::Result;
    use crate::Optional;

    #[test]
    fn new_ok_test() {
        let val: Result<i32, i32> = Result::Ok(15);
        assert!(val.is_ok());
        assert_eq!(val.unwrap(), 15);
    }

    #[test]
    fn new_err_test() {
        let val: Result<bool, i32> = Result::Err(15);
        assert!(val.is_err());
        assert_eq!(val.err(), Optional::Some(15));
    }

    #[test]
    fn is_ok_test() {
        let val: Result<i32, i32> = Result::Ok(15);
        assert!(val.is_ok());
    }

    #[test]
    fn is_err_test() {
        let val: Result<bool, i32> = Result::Err(15);
        assert!(val.is_err());
    }

    #[test]
    fn as_ref_test() {
        let val: Result<i32, bool> = Result::Ok(15);
        let val_ref = val.as_ref();
        assert_eq!(val_ref.ok(), Optional::Some(&15));
        let val: Result<i32, bool> = Result::Err(false);
        let val_ref = val.as_ref();
        assert_eq!(val_ref.err(), Optional::Some(&false));
    }

    #[test]
    fn map_test() {
        let ok_val: Result<i32, bool> = Result::Ok(0);
        let err_val: Result<i32, bool> = Result::Err(false);
        let map_func = |i: i32| i + 1;

        let ok_map = ok_val.map(map_func);
        let err_map = err_val.map(map_func);

        assert_eq!(ok_map.ok(), Optional::Some(1));
        assert_eq!(err_map.err(), Optional::Some(false));
    }

    #[test]
    fn map_or_test() {
        let ok_val: Result<i32, bool> = Result::Ok(0);
        let err_val: Result<i32, bool> = Result::Err(false);
        let map_func = |i: i32| i + 1;

        let ok_map = ok_val.map_or(10, map_func);
        let err_map = err_val.map_or(10, map_func);

        assert_eq!(ok_map, 1);
        assert_eq!(err_map, 10);
    }

    #[test]
    fn map_or_else_test() {
        let ok_val: Result<i32, bool> = Result::Ok(0);
        let err_val: Result<i32, bool> = Result::Err(false);
        let map_ok_func = |i: i32| i > 0;
        let map_err_func = |i: bool| !i;

        let ok_map = ok_val.map_or_else(map_err_func, map_ok_func);
        let err_map = err_val.map_or_else(map_err_func, map_ok_func);

        assert!(!ok_map);
        assert!(err_map);
    }
}
