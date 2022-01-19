//! Implementation of the `Optional` type.
use std::fmt::Debug;
use std::hash::Hash;

/// An optional value.
#[repr(C, i8)]
#[derive(Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Optional<T> {
    /// Empty variant.
    None,
    /// `T` variant.
    Some(T),
}

impl<T> Optional<T> {
    /// Returns `true` if the optional contains a value.
    #[inline]
    pub const fn is_some(&self) -> bool {
        matches!(*self, Optional::Some(_))
    }

    /// Returns `true` if the optional is empty.
    #[inline]
    pub const fn is_none(&self) -> bool {
        matches!(*self, Optional::None)
    }

    /// Maps the `Optional<T>` to `Optional<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> Optional<&T> {
        match *self {
            Optional::None => Optional::None,
            Optional::Some(ref val) => Optional::Some(val),
        }
    }

    /// Maps the `Optional<T>` to `Optional<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Optional<&mut T> {
        match *self {
            Optional::None => Optional::None,
            Optional::Some(ref mut val) => Optional::Some(val),
        }
    }

    /// Returns the contained value.
    ///
    /// # Panics
    ///
    /// Panics if no value is contained with a custom panic message provided by `msg`.
    #[inline]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Optional::None => panic!("{}", msg),
            Optional::Some(val) => val,
        }
    }

    /// Returns the contained value.
    ///
    /// # Panics
    ///
    /// Panics if no value is contained.
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Optional::None => panic!("called `Optional::unwrap()` on an empty optional"),
            Optional::Some(val) => val,
        }
    }

    /// Returns the contained value or a default.
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Optional::None => default,
            Optional::Some(val) => val,
        }
    }

    /// Returns the contained value or computes it from a closure.
    #[inline]
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            Optional::None => f(),
            Optional::Some(val) => val,
        }
    }

    /// Maps an `Optional<T>` to `Optional<U>` by applying a function to the contained value.
    #[inline]
    pub fn map<U, F>(self, f: F) -> Optional<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Optional::None => Optional::None,
            Optional::Some(val) => Optional::Some(f(val)),
        }
    }

    /// Returns the application of the closure to the contained value or a default value.
    #[inline]
    pub fn map_or<U, F>(self, default: U, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Optional::None => default,
            Optional::Some(val) => f(val),
        }
    }

    /// Applies a function to the contained value (if any), or computes a default (if not).
    #[inline]
    pub fn map_or_else<U, D, F>(self, default: D, f: F) -> U
    where
        D: FnOnce() -> U,
        F: FnOnce(T) -> U,
    {
        match self {
            Optional::None => default(),
            Optional::Some(val) => f(val),
        }
    }

    /// Transforms the `Optional<T>` into a `Result<T, E>`.
    #[inline]
    pub fn ok_or<E>(self, err: E) -> crate::Result<T, E> {
        match self {
            Optional::None => crate::Result::Err(err),
            Optional::Some(x) => crate::Result::Ok(x),
        }
    }

    /// Transforms the `Optional<T>` into a `Result<T, E>` by mapping the contained value or
    /// computing an error value from a closure.
    #[inline]
    pub fn ok_or_else<E, F>(self, f: F) -> crate::Result<T, E>
    where
        F: FnOnce() -> E,
    {
        match self {
            Optional::None => crate::Result::Err(f()),
            Optional::Some(x) => crate::Result::Ok(x),
        }
    }

    /// Maps the `Optional<T>` to the native `Option<T>`.
    #[inline]
    pub fn into_rust(self) -> Option<T> {
        match self {
            Optional::None => None,
            Optional::Some(x) => Some(x),
        }
    }
}

impl<T: Clone> Clone for Optional<T> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Optional::None => Optional::None,
            Optional::Some(x) => Optional::Some(x.clone()),
        }
    }
}

impl<T> Default for Optional<T> {
    #[inline]
    fn default() -> Self {
        Optional::None
    }
}

impl<T> From<T> for Optional<T> {
    fn from(val: T) -> Self {
        Optional::Some(val)
    }
}

impl<T> From<Option<T>> for Optional<T> {
    #[inline]
    fn from(val: Option<T>) -> Self {
        match val {
            None => Optional::None,
            Some(x) => Optional::Some(x),
        }
    }
}

impl<T> From<Optional<T>> for Option<T> {
    #[inline]
    fn from(val: Optional<T>) -> Self {
        val.into_rust()
    }
}
