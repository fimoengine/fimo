//! Utilities for formatting.

use crate::marshal::CTypeBridge;
use crate::ptr::{coerce_obj_mut, IBase};
use crate::{interface, DynObj, ObjArc, ObjBox, ObjectId};
use std::fmt::{Arguments, Debug};
use std::ops::{Deref, DerefMut};

/// Helper type for bridging the implementations of the `fmt` modules of
/// this crate and the std library.
#[repr(transparent)]
#[derive(Copy, Clone, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FmtWrapper<T: ?Sized> {
    inner: T,
}

impl<T: ?Sized> FmtWrapper<T> {
    /// Constructs a new instance of an `FmtWrapper` taking ownership of the value.
    pub const fn new(inner: T) -> Self
    where
        T: Sized,
    {
        Self { inner }
    }

    /// Constructs a new instance of an `FmtWrapper` borrowing the value.
    pub const fn new_ref(inner: &T) -> &Self {
        unsafe { &*(inner as *const _ as *const Self) }
    }

    /// Constructs a new instance of an `FmtWrapper` borrowing the value mutable.
    pub fn new_mut(inner: &mut T) -> &mut Self {
        unsafe { &mut *(inner as *mut _ as *mut Self) }
    }
}

impl<T: ?Sized> Deref for FmtWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: ?Sized> DerefMut for FmtWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: ?Sized> std::fmt::Debug for FmtWrapper<T>
where
    T: IDebug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = FormatterWrapper { fmt: f };
        let f: &mut DynObj<dyn IFormatter + '_> = coerce_obj_mut(&mut f);
        let f: &mut Formatter<'_> = unsafe { &mut *(f as *mut _ as *mut Formatter<'_>) };
        T::fmt(&self.inner, f).map_err(|_| std::fmt::Error)
    }
}

impl<T: ?Sized> std::fmt::Display for FmtWrapper<T>
where
    T: IDisplay,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = FormatterWrapper { fmt: f };
        let f: &mut DynObj<dyn IFormatter + '_> = coerce_obj_mut(&mut f);
        let f: &mut Formatter<'_> = unsafe { &mut *(f as *mut _ as *mut Formatter<'_>) };
        T::fmt(&self.inner, f).map_err(|_| std::fmt::Error)
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "2f8ffa24-1b60-43d8-bd3d-82197b2372bf",
    )]

    /// [`Debug`] equivalent for [`DynObj`] objects.
    pub frozen interface IDebug: marker IBase {
        /// Formats the value using the given formatter.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
    }
}

impl<T: IDebug + ?Sized> IDebug for ObjBox<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDebug + ?Sized> IDebug for ObjArc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDebug + ?Sized> IDebug for Box<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDebug + ?Sized> IDebug for std::sync::Arc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDebug + ?Sized> IDebug for std::rc::Rc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDebug + ?Sized> IDebug for &'_ T {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDebug + ?Sized> IDebug for &'_ mut T {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "62ceb949-1605-402a-aa8c-1acdc75dd160",
    )]

    /// [`Display`](std::fmt::Display) equivalent for [`DynObj`] objects.
    pub frozen interface IDisplay: marker IBase {
        /// Formats the value using the given formatter.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
    }
}

impl<T: IDisplay + ?Sized> IDisplay for ObjBox<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDisplay + ?Sized> IDisplay for ObjArc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDisplay + ?Sized> IDisplay for Box<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDisplay + ?Sized> IDisplay for std::sync::Arc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDisplay + ?Sized> IDisplay for std::rc::Rc<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDisplay + ?Sized> IDisplay for &'_ T {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

impl<T: IDisplay + ?Sized> IDisplay for &'_ mut T {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        (**self).fmt(f)
    }
}

/// Possible alignments returned by [`IFormatter::align`].
#[repr(u32)]
#[derive(Debug, CTypeBridge)]
pub enum Alignment {
    /// Indication that contents should be left-aligned.
    Left,
    /// Indication that contents should be right-aligned.
    Right,
    /// Indication that contents should be center-aligned.
    Center,
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "4f94dc64-2f45-4590-9d41-1b7917510138",
    )]

    /// Type-erased configuration for formatting.
    pub frozen interface IFormatter: IWrite @ frozen version("0.0") {
        /// See [`Formatter::pad`]: std::fmt::Formatter::pad.
        fn pad(&mut self, s: &str) -> Result<(), Error>;

        /// See [`Formatter::pad_integral`]: std::fmt::Formatter::pad_integral.
        fn pad_integral(
            &mut self,
            is_nonnegative: bool,
            prefix: &str,
            buf: &str
        ) -> Result<(), Error>;

        /// See [`Formatter::fill`]: std::fmt::Formatter::fill.
        fn fill(&self) -> char;

        /// See [`Formatter::align`]: std::fmt::Formatter::align.
        fn align(&self) -> Option<Alignment>;

        /// See [`Formatter::width`]: std::fmt::Formatter::width.
        fn width(&self) -> Option<usize>;

        /// See [`Formatter::precision`]: std::fmt::Formatter::precision.
        fn precision(&self) -> Option<usize>;

        /// See [`Formatter::sign_plus`]: std::fmt::Formatter::sign_plus.
        fn sign_plus(&self) -> bool;

        /// See [`Formatter::sign_minus`]: std::fmt::Formatter::sign_minus.
        fn sign_minus(&self) -> bool;

        /// See [`Formatter::alternate`]: std::fmt::Formatter::alternate.
        fn alternate(&self) -> bool;

        /// See [`Formatter::sign_aware_zero_pad`]: std::fmt::Formatter::sign_aware_zero_pad.
        fn sign_aware_zero_pad(&self) -> bool;
    }
}

/// Wrapper around a `DynObj<dyn IFormatter + 'a>`
/// allowing the use of the [`IFormatter`] trait without
/// having it in scope.
#[repr(transparent)]
#[allow(missing_debug_implementations)]
pub struct Formatter<'a> {
    inner: DynObj<dyn IFormatter + 'a>,
}

impl<'a> Formatter<'a> {
    /// See [`IWrite::write_str`].
    #[inline]
    pub fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.inner.write_str(s)
    }

    /// See [`IWrite::write_fmt`].
    #[inline]
    pub fn write_fmt(&mut self, args: Arguments<'_>) -> Result<(), Error> {
        self.inner.write_fmt(args)
    }

    /// See [`IFormatter::pad`].
    #[inline]
    pub fn pad(&mut self, s: &str) -> Result<(), Error> {
        self.inner.pad(s)
    }

    /// See [`IFormatter::pad_integral`].
    #[inline]
    pub fn pad_integral(
        &mut self,
        is_nonnegative: bool,
        prefix: &str,
        buf: &str,
    ) -> Result<(), Error> {
        self.inner.pad_integral(is_nonnegative, prefix, buf)
    }

    /// See [`IFormatter::fill`].
    #[inline]
    pub fn fill(&self) -> char {
        self.inner.fill()
    }

    /// See [`IFormatter::align`].
    #[inline]
    pub fn align(&self) -> Option<Alignment> {
        self.inner.align()
    }

    /// See [`IFormatter::width`].
    #[inline]
    pub fn width(&self) -> Option<usize> {
        self.inner.width()
    }

    /// See [`IFormatter::precision`].
    #[inline]
    pub fn precision(&self) -> Option<usize> {
        self.inner.precision()
    }

    /// See [`IFormatter::sign_plus`].
    #[inline]
    pub fn sign_plus(&self) -> bool {
        self.inner.sign_plus()
    }

    /// See [`IFormatter::sign_minus`].
    #[inline]
    pub fn sign_minus(&self) -> bool {
        self.inner.sign_minus()
    }

    /// See [`IFormatter::alternate`].
    #[inline]
    pub fn alternate(&self) -> bool {
        self.inner.alternate()
    }

    /// See [`IFormatter::sign_aware_zero_pad`].
    #[inline]
    pub fn sign_aware_zero_pad(&self) -> bool {
        self.inner.sign_aware_zero_pad()
    }
}

impl<'a> IWrite for Formatter<'a> {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.write_str(s)
    }

    #[inline]
    fn write_char(&mut self, c: char) -> Result<(), Error> {
        self.inner.write_char(c)
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result<(), Error> {
        self.write_fmt(args)
    }
}

unsafe impl<'lt, 'a> CTypeBridge for &'lt Formatter<'a> {
    type Type = <&'lt DynObj<dyn IFormatter + 'a> as CTypeBridge>::Type;

    fn marshal(self) -> Self::Type {
        self.inner.marshal()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let inner = <&'lt DynObj<dyn IFormatter + 'a> as CTypeBridge>::demarshal(x);

        // Safety: Formatter is repr(transparent).
        &*(inner as *const _ as *const Formatter<'a>)
    }
}

unsafe impl<'lt, 'a> CTypeBridge for &'lt mut Formatter<'a> {
    type Type = <&'lt mut DynObj<dyn IFormatter + 'a> as CTypeBridge>::Type;

    fn marshal(self) -> Self::Type {
        self.inner.marshal()
    }

    unsafe fn demarshal(x: Self::Type) -> Self {
        let inner = <&'lt mut DynObj<dyn IFormatter + 'a> as CTypeBridge>::demarshal(x);

        // Safety: Formatter is repr(transparent).
        &mut *(inner as *mut _ as *mut Formatter<'a>)
    }
}

#[derive(ObjectId)]
#[fetch_vtable(uuid = "bfd8655b-3746-412d-a874-7af026932817", interfaces(IFormatter))]
struct FormatterWrapper<'a, 'b> {
    fmt: &'a mut std::fmt::Formatter<'b>,
}

impl<'a, 'b> IWrite for FormatterWrapper<'a, 'b> {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        std::fmt::Write::write_str(&mut self.fmt, s).map_err(|_| Error { _e: 0 })
    }

    #[inline]
    fn write_char(&mut self, c: char) -> Result<(), Error> {
        std::fmt::Write::write_char(&mut self.fmt, c).map_err(|_| Error { _e: 0 })
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result<(), Error> {
        std::fmt::Write::write_fmt(&mut self.fmt, args).map_err(|_| Error { _e: 0 })
    }
}

impl<'a, 'b> IFormatter for FormatterWrapper<'a, 'b> {
    #[inline]
    fn pad(&mut self, s: &str) -> Result<(), Error> {
        self.fmt.pad(s).map_err(|_| Error { _e: 0 })
    }

    #[inline]
    fn pad_integral(&mut self, is_nonnegative: bool, prefix: &str, buf: &str) -> Result<(), Error> {
        self.fmt
            .pad_integral(is_nonnegative, prefix, buf)
            .map_err(|_| Error { _e: 0 })
    }

    #[inline]
    fn fill(&self) -> char {
        self.fmt.fill()
    }

    #[inline]
    fn align(&self) -> Option<Alignment> {
        self.fmt.align().map(|a| match a {
            std::fmt::Alignment::Left => Alignment::Left,
            std::fmt::Alignment::Right => Alignment::Right,
            std::fmt::Alignment::Center => Alignment::Center,
        })
    }

    #[inline]
    fn width(&self) -> Option<usize> {
        self.fmt.width()
    }

    #[inline]
    fn precision(&self) -> Option<usize> {
        self.fmt.precision()
    }

    #[inline]
    fn sign_plus(&self) -> bool {
        self.fmt.sign_plus()
    }

    #[inline]
    fn sign_minus(&self) -> bool {
        self.fmt.sign_minus()
    }

    #[inline]
    fn alternate(&self) -> bool {
        self.fmt.alternate()
    }

    #[inline]
    fn sign_aware_zero_pad(&self) -> bool {
        self.fmt.sign_aware_zero_pad()
    }
}

interface! {
    #![interface_cfg(
        abi(explicit(abi = "C-unwind")),
        uuid = "fef2a45f-1309-46c2-8a83-361d51f1bf0f",
    )]

    /// [`Write`](std::fmt::Write) equivalent for [`DynObj`] objects.
    pub frozen interface IWrite: marker IBase {
        /// Writes a string slice into this writer, returning whether the write succeeded.
        fn write_str(&mut self, s: &str) -> Result<(), Error>;

        /// Writes a [`char`] into this writer, returning whether the write succeeded.
        fn write_char(&mut self, c: char) -> Result<(), Error>;

        /// Writes a multiple arguments into this writer, returning whether the write succeeded.
        #[interface_cfg(abi(explicit(abi = "Rust")), mapping(optional()))]
        fn write_fmt(&mut self, args: Arguments<'_>) -> Result<(), Error> {
            let s = std::fmt::format(args);
            self.write_str(&s)
        }
    }
}

/// The error type which is returned from formatting a message into a stream.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, CTypeBridge)]
pub struct Error {
    // C doesn't support ZSTs, instead we use the smallest possible C type.
    _e: u8,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt("an error occurred when formatting an argument", f)
    }
}

impl IDebug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{:?}", self)
    }
}

impl IDisplay for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self)
    }
}

impl std::error::Error for Error {}
impl crate::error::IError for Error {}
