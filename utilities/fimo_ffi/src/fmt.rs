//! Utilities for formatting.

use crate::ptr::{
    coerce_obj_mut, from_raw_mut, into_raw_mut, metadata, CastInto, IBase, ObjInterface, RawObjMut,
};
use crate::{interface, vtable, ConstStr, DynObj, ObjArc, ObjBox, ObjectId, Optional, ReprC};
use std::fmt::{Arguments, Debug};
use std::marker::Unsize;
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

/// [`Debug`] equivalent for [`DynObj`] objects.
#[interface(
    uuid = "2f8ffa24-1b60-43d8-bd3d-82197b2372bf",
    vtable = "IDebugVTable",
    generate()
)]
pub trait IDebug: IBase {
    /// Formats the value using the given formatter.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "crate::Result<(), Error>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn fmt(
        &self,
        #[vtable_info(
            type = "RawObjMut<dyn IFormatter + '_>",
            into = "formatter_into_raw",
            from = "formatter_from_raw"
        )]
        f: &mut Formatter<'_>,
    ) -> Result<(), Error>;
}

#[inline]
fn formatter_from_raw<'a>(raw: RawObjMut<dyn IFormatter + 'a>) -> &mut Formatter<'a> {
    unsafe {
        let formatter = from_raw_mut(raw) as *mut Formatter<'a>;
        &mut *formatter
    }
}

#[inline]
fn formatter_into_raw<'a>(f: &mut Formatter<'a>) -> RawObjMut<dyn IFormatter + 'a> {
    into_raw_mut(&mut f.inner)
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

/// [`Display`](std::fmt::Display) equivalent for [`DynObj`] objects.
#[interface(
    uuid = "62ceb949-1605-402a-aa8c-1acdc75dd160",
    vtable = "IDisplayVTable",
    generate()
)]
pub trait IDisplay: IBase {
    /// Formats the value using the given formatter.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "crate::Result<(), Error>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn fmt(
        &self,
        #[vtable_info(
            type = "RawObjMut<dyn IFormatter + '_>",
            into = "formatter_into_raw",
            from = "formatter_from_raw"
        )]
        f: &mut Formatter<'_>,
    ) -> Result<(), Error>;
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
#[derive(Debug)]
pub enum Alignment {
    /// Indication that contents should be left-aligned.
    Left,
    /// Indication that contents should be right-aligned.
    Right,
    /// Indication that contents should be center-aligned.
    Center,
}

/// Type-erased configuration for formatting.
#[interface(
    uuid = "4f94dc64-2f45-4590-9d41-1b7917510138",
    vtable = "IFormatterVTable",
    generate(IWriteVTable)
)]
pub trait IFormatter: IWrite {
    /// See [`Formatter::pad`]: std::fmt::Formatter::pad.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "crate::Result<(), Error>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn pad(
        &mut self,
        #[vtable_info(type = "ConstStr<'_>", into = "Into::into", from = "Into::into")] s: &str,
    ) -> Result<(), Error>;

    /// See [`Formatter::pad_integral`]: std::fmt::Formatter::pad_integral.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "crate::Result<(), Error>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn pad_integral(
        &mut self,
        is_nonnegative: bool,
        #[vtable_info(type = "ConstStr<'_>", into = "Into::into", from = "Into::into")]
        prefix: &str,
        #[vtable_info(type = "ConstStr<'_>", into = "Into::into", from = "Into::into")] buf: &str,
    ) -> Result<(), Error>;

    /// See [`Formatter::fill`]: std::fmt::Formatter::fill.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "u32",
        into = "Into::into",
        from = "char::from_u32_unchecked"
    )]
    fn fill(&self) -> char;

    /// See [`Formatter::align`]: std::fmt::Formatter::align.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<Alignment>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn align(&self) -> Option<Alignment>;

    /// See [`Formatter::width`]: std::fmt::Formatter::width.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<usize>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn width(&self) -> Option<usize>;

    /// See [`Formatter::precision`]: std::fmt::Formatter::precision.
    #[vtable_info(
        unsafe,
        abi = r#"extern "C-unwind""#,
        return_type = "Optional<usize>",
        into = "Into::into",
        from = "Into::into"
    )]
    fn precision(&self) -> Option<usize>;

    /// See [`Formatter::sign_plus`]: std::fmt::Formatter::sign_plus.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn sign_plus(&self) -> bool;

    /// See [`Formatter::sign_minus`]: std::fmt::Formatter::sign_minus.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn sign_minus(&self) -> bool;

    /// See [`Formatter::alternate`]: std::fmt::Formatter::alternate.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn alternate(&self) -> bool;

    /// See [`Formatter::sign_aware_zero_pad`]: std::fmt::Formatter::sign_aware_zero_pad.
    #[vtable_info(unsafe, abi = r#"extern "C-unwind""#)]
    fn sign_aware_zero_pad(&self) -> bool;
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

/// [`Write`](std::fmt::Write) equivalent for [`DynObj`] objects.
#[interface(uuid = "fef2a45f-1309-46c2-8a83-361d51f1bf0f", vtable = "IWriteVTable")]
pub trait IWrite: IBase {
    /// Writes a string slice into this writer, returning whether the write succeeded.
    fn write_str(&mut self, s: &str) -> Result<(), Error>;

    /// Writes a [`char`] into this writer, returning whether the write succeeded.
    fn write_char(&mut self, c: char) -> Result<(), Error>;

    /// Writes a multiple arguments into this writer, returning whether the write succeeded.
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result<(), Error>;
}

// The vtable contains an `Option` so we must define it manually.
/// VTable for [`IWrite`] objects.
#[vtable(interface = "IWrite")]
#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct IWriteVTable {
    /// Writes a string slice into this writer, returning whether the write succeeded.
    pub write_str: extern "C-unwind" fn(*mut (), ConstStr<'_>) -> crate::Result<(), Error>,
    /// Writes a [`char`] into this writer, returning whether the write succeeded.
    pub write_char: extern "C-unwind" fn(*mut (), u32) -> crate::Result<(), Error>,
    /// Writes a multiple arguments into this writer, returning whether the write succeeded.
    ///
    /// # Note
    ///
    /// Implementation of this function is optional and allows for formatting without
    /// allocating additional buffers.
    #[allow(clippy::type_complexity)]
    pub write_fmt: Option<fn(*mut (), Arguments<'_>) -> crate::Result<(), Error>>,
}

impl IWriteVTable {
    /// Constructs a new vtable for a given type.
    #[inline]
    pub const fn new_for<'a, T>() -> Self
    where
        T: IWrite + ObjectId + 'a,
    {
        Self::new_for_embedded::<'a, T, dyn IWrite>(0)
    }

    /// Constructs a new vtable for a given type and interface with a custom offset.
    #[inline]
    pub const fn new_for_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: IWrite + ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjInterface + ?Sized + 'a,
    {
        extern "C-unwind" fn write_str<T: IWrite>(
            ptr: *mut (),
            s: ConstStr<'_>,
        ) -> crate::Result<(), Error> {
            let ptr = unsafe { &mut *(ptr as *mut T) };
            ptr.write_str(s.into()).into()
        }
        extern "C-unwind" fn write_char<T: IWrite>(
            ptr: *mut (),
            c: u32,
        ) -> crate::Result<(), Error> {
            let ptr = unsafe { &mut *(ptr as *mut T) };
            let c = unsafe { char::from_u32_unchecked(c) };
            ptr.write_char(c).into()
        }
        fn write_fmt<T: IWrite>(ptr: *mut (), args: Arguments<'_>) -> crate::Result<(), Error> {
            let ptr = unsafe { &mut *(ptr as *mut T) };
            ptr.write_fmt(args).into()
        }

        Self::new_embedded::<T, Dyn>(
            offset,
            write_str::<T> as _,
            write_char::<T> as _,
            Some(write_fmt::<T> as _),
        )
    }
}

impl<'a, T: CastInto<dyn IWrite + 'a> + ?Sized> IWrite for DynObj<T>
where
    DynObj<T>: IBase,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        let vtable: &IWriteVTable = metadata(self).super_vtable::<dyn IWrite + 'a>();
        (vtable.write_str)(self as *mut _ as *mut (), s.into()).into_rust()
    }

    #[inline]
    fn write_char(&mut self, c: char) -> Result<(), Error> {
        let vtable: &IWriteVTable = metadata(self).super_vtable::<dyn IWrite + 'a>();
        (vtable.write_char)(self as *mut _ as *mut (), c as u32).into_rust()
    }

    #[inline]
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result<(), Error> {
        let vtable: &IWriteVTable = metadata(self).super_vtable::<dyn IWrite + 'a>();

        // Check if the implementation supports formatting arguments.
        if let Some(write_fmt) = vtable.write_fmt {
            (write_fmt)(self as *mut _ as *mut (), args).into_rust()
        } else {
            // if it isn't the case we can manually format it into a String and then write it.
            let s = std::fmt::format(args);
            self.write_str(&s)
        }
    }
}

/// The error type which is returned from formatting a message into a stream.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
