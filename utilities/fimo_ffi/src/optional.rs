//! Implementation of the `Optional` type.
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::Destruct;
use std::ops::{FromResidual, Residual, Try};

use crate::marshal::CTypeBridge;
use crate::tuple::{ReprC, ReprRust};

/// An optional value.
///
/// # Layout
///
/// `Optional<T>` is guaranteed to have the layout of the following `C` struct:
///
/// ```c
/// #include <stdint.h>
///
/// typedef int8_t optional_disc_t;
/// const optional_disc_t OPTIONAL_DISCRIMINANT_NONE = 0;
/// const optional_disc_t OPTIONAL_DISCRIMINANT_SOME = 1;
///
/// struct optional_none_t {};
/// struct optional_some_t { T field; };
///
/// union optional_inner_t {
///     struct optional_none_t none;
///     struct optional_some_t some;
/// };
///
/// struct optional_t {
///     optional_disc_t discriminant;
///     union optional_inner_t variant;
/// };
/// ```
#[repr(C, i8)]
#[derive(Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash, CTypeBridge)]
pub enum Optional<T> {
    /// Empty variant.
    None,
    /// `T` variant.
    Some(T),
}

impl<T> Optional<T> {
    /// Returns `true` if the optional contains a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let x: Optional<u32> = Optional::Some(2);
    /// assert_eq!(x.is_some(), true);
    ///
    /// let x: Optional<u32> = Optional::None;
    /// assert_eq!(x.is_some(), false);
    /// ```
    #[inline]
    pub const fn is_some(&self) -> bool {
        matches!(*self, Optional::Some(_))
    }

    /// Returns `true` if the optional is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let x: Optional<u32> = Optional::Some(2);
    /// assert_eq!(x.is_none(), false);
    ///
    /// let x: Optional<u32> = Optional::None;
    /// assert_eq!(x.is_none(), true);
    /// ```
    #[inline]
    pub const fn is_none(&self) -> bool {
        matches!(*self, Optional::None)
    }

    /// Maps the `Optional<T>` to `Optional<&T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let text: Optional<String> = Optional::Some("Hello, world!".to_string());
    /// // First, cast `Optional<String>` to `Optional<&String>` with `as_ref`,
    /// // then consume *that* with `map`, leaving `text` on the stack.
    /// let text_length: Optional<usize> = text.as_ref().map(|s| s.len());
    /// println!("still can print text: {:?}", text);
    /// ```
    #[inline]
    pub const fn as_ref(&self) -> Optional<&T> {
        match *self {
            Optional::None => Optional::None,
            Optional::Some(ref val) => Optional::Some(val),
        }
    }

    /// Maps the `Optional<T>` to `Optional<&mut T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let mut x = Optional::Some(2);
    /// match x.as_mut() {
    ///     Optional::Some(v) => *v = 42,
    ///     Optional::None => {},
    /// }
    /// assert_eq!(x, Optional::Some(42));
    /// ```
    #[inline]
    pub const fn as_mut(&mut self) -> Optional<&mut T> {
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
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let x = Optional::Some("value");
    /// assert_eq!(x.expect("fruits are healthy"), "value");
    /// ```
    ///
    /// ```should_panic
    /// use fimo_ffi::Optional;
    ///
    /// let x: Optional<&str> = Optional::None;
    /// x.expect("fruits are healthy");
    /// ```
    #[inline]
    pub const fn expect(self, msg: &str) -> T {
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
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let x = Optional::Some("air");
    /// assert_eq!(x.unwrap(), "air");
    /// ```
    ///
    /// ```should_panic
    /// use fimo_ffi::Optional;
    ///
    /// let x: Optional<&str> = Optional::None;
    /// assert_eq!(x.unwrap(), "air"); // fails
    /// ```
    #[inline]
    pub const fn unwrap(self) -> T {
        match self {
            Optional::None => panic!("called `Optional::unwrap()` on an empty optional"),
            Optional::Some(val) => val,
        }
    }

    /// Returns the contained value or a default.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// assert_eq!(Optional::Some("car").unwrap_or("bike"), "car");
    /// assert_eq!(Optional::None.unwrap_or("bike"), "bike");
    /// ```
    #[inline]
    pub const fn unwrap_or(self, default: T) -> T
    where
        T: ~const Destruct,
    {
        match self {
            Optional::None => default,
            Optional::Some(val) => val,
        }
    }

    /// Returns the contained value or computes it from a closure.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let k = 10;
    /// assert_eq!(Optional::Some(4).unwrap_or_else(|| 2 * k), 4);
    /// assert_eq!(Optional::None.unwrap_or_else(|| 2 * k), 20);
    /// ```
    #[inline]
    pub const fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: ~const FnOnce() -> T + ~const Destruct,
    {
        match self {
            Optional::None => f(),
            Optional::Some(val) => val,
        }
    }

    /// Maps an `Optional<T>` to `Optional<U>` by applying a function to the contained value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let maybe_some_string = Optional::Some(String::from("Hello, World!"));
    /// // `Optional::map` takes self *by value*, consuming `maybe_some_string`
    /// let maybe_some_len = maybe_some_string.map(|s| s.len());
    ///
    /// assert_eq!(maybe_some_len, Optional::Some(13));
    /// ```
    #[inline]
    pub const fn map<U, F>(self, f: F) -> Optional<U>
    where
        F: ~const FnOnce(T) -> U + ~const Destruct,
    {
        match self {
            Optional::None => Optional::None,
            Optional::Some(val) => Optional::Some(f(val)),
        }
    }

    /// Returns the application of the closure to the contained value or a default value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let x = Optional::Some("foo");
    /// assert_eq!(x.map_or(42, |v| v.len()), 3);
    ///
    /// let x: Optional<&str> = Optional::None;
    /// assert_eq!(x.map_or(42, |v| v.len()), 42);
    /// ```
    #[inline]
    pub const fn map_or<U, F>(self, default: U, f: F) -> U
    where
        U: ~const Destruct,
        F: ~const FnOnce(T) -> U + ~const Destruct,
    {
        match self {
            Optional::None => default,
            Optional::Some(val) => f(val),
        }
    }

    /// Applies a function to the contained value (if any), or computes a default (if not).
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::Optional;
    ///
    /// let k = 21;
    ///
    /// let x = Optional::Some("foo");
    /// assert_eq!(x.map_or_else(|| 2 * k, |v| v.len()), 3);
    ///
    /// let x: Optional<&str> = Optional::None;
    /// assert_eq!(x.map_or_else(|| 2 * k, |v| v.len()), 42);
    /// ```
    #[inline]
    pub const fn map_or_else<U, D, F>(self, default: D, f: F) -> U
    where
        D: ~const FnOnce() -> U + ~const Destruct,
        F: ~const FnOnce(T) -> U + ~const Destruct,
    {
        match self {
            Optional::None => default(),
            Optional::Some(val) => f(val),
        }
    }

    /// Transforms the `Optional<T>` into a `Result<T, E>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::{Optional, Result};
    ///
    /// let x = Optional::Some("foo");
    /// assert_eq!(x.ok_or(0), Result::Ok("foo"));
    ///
    /// let x: Optional<&str> = Optional::None;
    /// assert_eq!(x.ok_or(0), Result::Err(0));
    /// ```
    #[inline]
    pub const fn ok_or<E>(self, err: E) -> crate::Result<T, E>
    where
        E: ~const Destruct,
    {
        match self {
            Optional::None => crate::Result::Err(err),
            Optional::Some(x) => crate::Result::Ok(x),
        }
    }

    /// Transforms the `Optional<T>` into a `Result<T, E>` by mapping the contained value or
    /// computing an error value from a closure.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::{Optional, Result};
    ///
    /// let x = Optional::Some("foo");
    /// assert_eq!(x.ok_or_else(|| 0), Result::Ok("foo"));
    ///
    /// let x: Optional<&str> = Optional::None;
    /// assert_eq!(x.ok_or_else(|| 0), Result::Err(0));
    /// ```
    #[inline]
    pub const fn ok_or_else<E, F>(self, f: F) -> crate::Result<T, E>
    where
        F: ~const FnOnce() -> E + ~const Destruct,
    {
        match self {
            Optional::None => crate::Result::Err(f()),
            Optional::Some(x) => crate::Result::Ok(x),
        }
    }

    /// Takes the value out of the optional, leaving a [`None`] in its place.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::{Optional, Result};
    ///
    /// let mut x = Optional::Some(2);
    /// let y = x.take();
    /// assert_eq!(x, Optional::None);
    /// assert_eq!(y, Optional::Some(2));
    ///
    /// let mut x: Optional<u32> = Optional::None;
    /// let y = x.take();
    /// assert_eq!(x, Optional::None);
    /// assert_eq!(y, Optional::None);
    /// ```
    #[inline]
    pub fn take(&mut self) -> Optional<T> {
        std::mem::take(self)
    }

    /// Replaces the actual value in the option by the value given in parameter, returning
    /// the old value if present, leaving a [`Some`] in its place without deinitializing either one.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::{Optional, Result};
    ///
    /// let mut x = Optional::Some(2);
    /// let y = x.replace(5);
    /// assert_eq!(x, Optional::Some(5));
    /// assert_eq!(y, Optional::Some(2));
    ///
    /// let mut x: Optional<u32> = Optional::None;
    /// let y = x.replace(3);
    /// assert_eq!(x, Optional::Some(3));
    /// assert_eq!(y, Optional::None);
    /// ```
    #[inline]
    pub fn replace(&mut self, value: T) -> Optional<T> {
        std::mem::replace(self, Optional::Some(value))
    }
}

impl<T: ~const Clone> const Clone for Optional<T> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Optional::None => Optional::None,
            Optional::Some(x) => Optional::Some(x.clone()),
        }
    }
}

impl<T> const Default for Optional<T> {
    #[inline]
    fn default() -> Self {
        Optional::None
    }
}

impl<T> const From<T> for Optional<T> {
    fn from(val: T) -> Self {
        Optional::Some(val)
    }
}

impl<T> const From<Option<T>> for Optional<T> {
    #[inline]
    fn from(val: Option<T>) -> Self {
        <Option<T> as ReprRust>::into_c(val)
    }
}

impl<T> const From<Optional<T>> for Option<T> {
    #[inline]
    fn from(val: Optional<T>) -> Self {
        <Optional<T> as ReprC>::into_rust(val)
    }
}

unsafe impl<T> const CTypeBridge for Option<T>
where
    T: ~const CTypeBridge,
{
    type Type = Optional<T::Type>;

    fn marshal(self) -> Self::Type {
        match self {
            Some(x) => Optional::Some(x.marshal()),
            None => Optional::None,
        }
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        match x {
            Optional::Some(x) => Some(T::demarshal(x)),
            Optional::None => None,
        }
    }
}

impl<T> const ReprC for Optional<T> {
    type T = Option<T>;

    #[inline]
    fn into_rust(self) -> Self::T {
        match self {
            Optional::None => None,
            Optional::Some(v) => Some(v),
        }
    }

    #[inline]
    fn from_rust(t: Self::T) -> Self {
        match t {
            Some(v) => Self::Some(v),
            None => Self::None,
        }
    }
}

impl<T> const ReprRust for Option<T> {
    type T = Optional<T>;

    #[inline]
    fn into_c(self) -> Self::T {
        <Optional<T> as ReprC>::from_rust(self)
    }

    #[inline]
    fn from_c(t: Self::T) -> Self {
        <Optional<T> as ReprC>::into_rust(t)
    }
}

impl<T> const Try for Optional<T> {
    type Output = T;

    type Residual = Optional<std::convert::Infallible>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Self::Some(output)
    }

    #[inline]
    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            Optional::None => std::ops::ControlFlow::Break(Optional::None),
            Optional::Some(v) => std::ops::ControlFlow::Continue(v),
        }
    }
}

impl<T> const FromResidual for Optional<T> {
    #[inline]
    fn from_residual(residual: Optional<std::convert::Infallible>) -> Self {
        match residual {
            Optional::None => Optional::None,
            Optional::Some(_) => unreachable!(),
        }
    }
}

impl<T> const Residual<T> for Optional<std::convert::Infallible> {
    type TryType = Optional<T>;
}
