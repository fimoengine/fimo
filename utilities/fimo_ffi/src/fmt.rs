//! Utilities for formatting.

use crate::ptr::{
    coerce_obj_mut, from_raw_mut, into_raw_mut, metadata, CastSuper, FetchVTable, IBase,
    ObjInterface, ObjMetadata, ObjectId, RawObjMut,
};
use crate::{
    base_interface, base_object, base_vtable, impl_upcast, ConstStr, DynObj, ObjArc, ObjBox,
    Optional,
};
use std::fmt::{Arguments, Debug};
use std::marker::Unsize;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr::addr_of;

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

base_interface! {
    /// [`Debug`] equivalent for [`DynObj`] objects.
    #![vtable = IDebugVTable]
    #![uuid(0x2f8ffa24, 0x1b60, 0x43d8, 0xbd3d, 0x82197b2372bf)]
    pub trait IDebug : (IBase) {
        /// Formats the value using the given formatter.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
    }
}

base_vtable! {
    /// VTable for a [`IDebug`] object.
    #![interface = IDebug]
    pub struct IDebugVTable {
        /// Formats the value using the given formatter.
        pub fmt: extern "C-unwind" fn(
            *const (),
            RawObjMut<dyn IFormatter + '_>,
        ) -> crate::Result<(), Error>,
    }
}

impl IDebugVTable {
    /// Constructs a new vtable for a given type.
    #[inline]
    pub const fn new_for<'a, T>() -> Self
    where
        T: IDebug + ObjectId + 'a,
    {
        Self::new_for_embedded::<'a, T, dyn IDebug>(0)
    }

    /// Constructs a new vtable for a given type and interface with a custom offset.
    #[inline]
    pub const fn new_for_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: IDebug + ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjInterface + ?Sized + 'a,
    {
        extern "C-unwind" fn fmt<T: IDebug>(
            ptr: *const (),
            f: RawObjMut<dyn IFormatter + '_>,
        ) -> crate::Result<(), Error> {
            let ptr = unsafe { &*(ptr as *const T) };
            let f = unsafe { &mut *(from_raw_mut(f) as *mut Formatter<'_>) };
            ptr.fmt(f).into()
        }

        Self::new_embedded::<T, Dyn>(offset, fmt::<T> as _)
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

impl<'a, T: CastSuper<dyn IDebug + 'a> + ?Sized> IDebug for DynObj<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let vtable: &IDebugVTable = metadata(self).super_vtable();
        (vtable.fmt)(self as *const _ as _, into_raw_mut(&mut f.inner)).into_rust()
    }
}

base_interface! {
    /// [`Display`](std::fmt::Display) equivalent for [`DynObj`] objects.
    #![vtable = IDisplayVTable]
    #![uuid(0x62ceb949, 0x1605, 0x402a, 0xaa8c, 0x1acdc75dd160)]
    pub trait IDisplay : (IBase) {
        /// Formats the value using the given formatter.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
    }
}

base_vtable! {
    /// VTable for a [`IDisplay`] object.
    #![interface = IDebug]
    pub struct IDisplayVTable {
        /// Formats the value using the given formatter.
        pub fmt: extern "C-unwind" fn(
            *const (),
            RawObjMut<dyn IFormatter + '_>,
        ) -> crate::Result<(), Error>,
    }
}

impl IDisplayVTable {
    /// Constructs a new vtable for a given type.
    #[inline]
    pub const fn new_for<'a, T>() -> Self
    where
        T: IDisplay + ObjectId + 'a,
    {
        Self::new_for_embedded::<'a, T, dyn IDisplay>(0)
    }

    /// Constructs a new vtable for a given type and interface with a custom offset.
    #[inline]
    pub const fn new_for_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: IDisplay + ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjInterface + ?Sized + 'a,
    {
        extern "C-unwind" fn fmt<T: IDisplay>(
            ptr: *const (),
            f: RawObjMut<dyn IFormatter + '_>,
        ) -> crate::Result<(), Error> {
            let ptr = unsafe { &*(ptr as *const T) };
            let f = unsafe { &mut *(from_raw_mut(f) as *mut Formatter<'_>) };
            ptr.fmt(f).into()
        }

        Self::new_embedded::<T, Dyn>(offset, fmt::<T> as _)
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

impl<'a, T: CastSuper<dyn IDisplay + 'a> + ?Sized> IDisplay for DynObj<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let vtable: &IDisplayVTable = metadata(self).super_vtable();
        (vtable.fmt)(self as *const _ as _, into_raw_mut(&mut f.inner)).into_rust()
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

base_interface! {
    /// Type-erased configuration for formatting.
    #![vtable = IFormatterVTable]
    #![uuid(0x4f94dc64, 0x2f45, 0x4590, 0x9d41, 0x1b7917510138)]
    pub trait IFormatter : (IWrite) {
        /// See [`Formatter::pad`]: std::fmt::Formatter::pad.
        fn pad(&mut self, s: &str) -> Result<(), Error>;
        /// See [`Formatter::pad_integral`]: std::fmt::Formatter::pad_integral.
        fn pad_integral(&mut self, is_nonnegative: bool, prefix: &str, buf: &str) -> Result<(), Error>;
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

base_vtable! {
    /// VTable for an [`IFormatter`].
    #![interface = IFormatter]
    pub struct IFormatterVTable {
        /// VTable for the [`IWrite`] implementation.
        pub write_vtable: IWriteVTable,
        /// See [`Formatter::pad`]: std::fmt::Formatter::pad.
        pub pad: extern "C-unwind" fn(*mut (), ConstStr<'_>) -> crate::Result<(), Error>,
        /// See [`Formatter::pad_integral`]: std::fmt::Formatter::pad_integral.
        pub pad_integral: extern "C-unwind" fn(
            *mut (),
            bool,
            ConstStr<'_>,
            ConstStr<'_>
        ) -> crate::Result<(), Error>,
        /// See [`Formatter::fill`]: std::fmt::Formatter::fill.
        pub fill: extern "C-unwind" fn(*const ()) -> u32,
        /// See [`Formatter::align`]: std::fmt::Formatter::align.
        pub align: extern "C-unwind" fn(*const ()) -> Optional<Alignment>,
        /// See [`Formatter::width`]: std::fmt::Formatter::width.
        pub width: extern "C-unwind" fn(*const ()) -> Optional<usize>,
        /// See [`Formatter::precision`]: std::fmt::Formatter::precision.
        pub precision: extern "C-unwind" fn(*const ()) -> Optional<usize>,
        /// See [`Formatter::sign_plus`]: std::fmt::Formatter::sign_plus.
        pub sign_plus: extern "C-unwind" fn(*const ()) -> bool,
        /// See [`Formatter::sign_minus`]: std::fmt::Formatter::sign_minus.
        pub sign_minus: extern "C-unwind" fn(*const ()) -> bool,
        /// See [`Formatter::alternate`]: std::fmt::Formatter::alternate.
        pub alternate: extern "C-unwind" fn(*const ()) -> bool,
        /// See [`Formatter::sign_aware_zero_pad`]: std::fmt::Formatter::sign_aware_zero_pad.
        pub sign_aware_zero_pad: extern "C-unwind" fn(*const ()) -> bool,
    }
}

impl_upcast! {
    impl (IFormatter) -> (IWrite) obj: ObjMetadata<_> {
        let vtable: &IFormatterVTable = obj.vtable();
        let vtable = &vtable.write_vtable;
        ObjMetadata::new(vtable)
    }
}

impl IFormatterVTable {
    /// Constructs a new vtable for a given type.
    #[inline]
    pub const fn new_for<'a, T>() -> Self
    where
        T: IFormatter + ObjectId + 'a,
    {
        Self::new_for_embedded::<'a, T, dyn IFormatter>(0)
    }

    /// Constructs a new vtable for a given type and interface with a custom offset.
    #[inline]
    pub const fn new_for_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: IFormatter + ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjInterface + ?Sized + 'a,
    {
        extern "C-unwind" fn pad<T: IFormatter>(
            ptr: *mut (),
            s: ConstStr<'_>,
        ) -> crate::Result<(), Error> {
            let ptr = unsafe { &mut *(ptr as *mut T) };
            ptr.pad(s.into()).into()
        }
        extern "C-unwind" fn pad_internal<T: IFormatter>(
            ptr: *mut (),
            is_nonnegative: bool,
            prefix: ConstStr<'_>,
            buf: ConstStr<'_>,
        ) -> crate::Result<(), Error> {
            let ptr = unsafe { &mut *(ptr as *mut T) };
            ptr.pad_integral(is_nonnegative, prefix.into(), buf.into())
                .into()
        }
        extern "C-unwind" fn fill<T: IFormatter>(ptr: *const ()) -> u32 {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.fill() as u32
        }
        extern "C-unwind" fn align<T: IFormatter>(ptr: *const ()) -> Optional<Alignment> {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.align().into()
        }
        extern "C-unwind" fn width<T: IFormatter>(ptr: *const ()) -> Optional<usize> {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.width().into()
        }
        extern "C-unwind" fn precision<T: IFormatter>(ptr: *const ()) -> Optional<usize> {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.precision().into()
        }
        extern "C-unwind" fn sign_plus<T: IFormatter>(ptr: *const ()) -> bool {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.sign_plus()
        }
        extern "C-unwind" fn sign_minus<T: IFormatter>(ptr: *const ()) -> bool {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.sign_minus()
        }
        extern "C-unwind" fn alternate<T: IFormatter>(ptr: *const ()) -> bool {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.alternate()
        }
        extern "C-unwind" fn sign_aware_zero_pad<T: IFormatter>(ptr: *const ()) -> bool {
            let ptr = unsafe { &*(ptr as *const T) };
            ptr.sign_aware_zero_pad()
        }

        const UNINIT: MaybeUninit<IFormatterVTable> = MaybeUninit::uninit();
        const UNINIT_PTR: *const IFormatterVTable = UNINIT.as_ptr();
        const IWRITE_VTABLE_PTR: *const IWriteVTable =
            unsafe { addr_of!((*UNINIT_PTR).write_vtable) };
        const IWRITE_OFFSET: usize = unsafe {
            (IWRITE_VTABLE_PTR as *const u8).offset_from(UNINIT_PTR as *const u8) as usize
        };

        Self::new_embedded::<T, Dyn>(
            offset,
            IWriteVTable::new_for_embedded::<T, Dyn>(IWRITE_OFFSET),
            pad::<T> as _,
            pad_internal::<T> as _,
            fill::<T> as _,
            align::<T> as _,
            width::<T> as _,
            precision::<T> as _,
            sign_plus::<T> as _,
            sign_minus::<T> as _,
            alternate::<T> as _,
            sign_aware_zero_pad::<T> as _,
        )
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

struct FormatterWrapper<'a, 'b> {
    fmt: &'a mut std::fmt::Formatter<'b>,
}

base_object! {
    #![uuid(0x5567dd5f, 0xd992, 0x4460, 0xa24b, 0xa863896ef27e)]
    generic <'a, 'b> FormatterWrapper<'a, 'b> => FormatterWrapper<'_, '_>
}

impl<'a, 'b> FetchVTable<dyn IFormatter + 'a> for FormatterWrapper<'a, 'b> {
    fn fetch_interface() -> &'static IFormatterVTable {
        static VTABLE: IFormatterVTable = IFormatterVTable::new_for::<FormatterWrapper<'_, '_>>();
        &VTABLE
    }
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

impl<'a, T: CastSuper<dyn IFormatter + 'a> + ?Sized> IFormatter for DynObj<T>
where
    DynObj<T>: IWrite,
{
    #[inline]
    fn pad(&mut self, s: &str) -> Result<(), Error> {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.pad)(self as *mut _ as _, s.into()).into_rust()
    }

    #[inline]
    fn pad_integral(&mut self, is_nonnegative: bool, prefix: &str, buf: &str) -> Result<(), Error> {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.pad_integral)(
            self as *mut _ as _,
            is_nonnegative,
            prefix.into(),
            buf.into(),
        )
        .into_rust()
    }

    #[inline]
    fn fill(&self) -> char {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        let c = (vtable.fill)(self as *const _ as _);
        unsafe { char::from_u32_unchecked(c) }
    }

    #[inline]
    fn align(&self) -> Option<Alignment> {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.align)(self as *const _ as _).into_rust()
    }

    #[inline]
    fn width(&self) -> Option<usize> {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.width)(self as *const _ as _).into_rust()
    }

    #[inline]
    fn precision(&self) -> Option<usize> {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.precision)(self as *const _ as _).into_rust()
    }

    #[inline]
    fn sign_plus(&self) -> bool {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.sign_plus)(self as *const _ as _)
    }

    #[inline]
    fn sign_minus(&self) -> bool {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.sign_minus)(self as *const _ as _)
    }

    #[inline]
    fn alternate(&self) -> bool {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.alternate)(self as *const _ as _)
    }

    #[inline]
    fn sign_aware_zero_pad(&self) -> bool {
        let vtable: &IFormatterVTable = metadata(self).super_vtable::<dyn IFormatter + 'a>();
        (vtable.sign_aware_zero_pad)(self as *const _ as _)
    }
}

base_interface! {
    /// [`Write`](std::fmt::Write) equivalent for [`DynObj`] objects.
    #![vtable = IWriteVTable]
    #![uuid(0xfef2a45f, 0x1309, 0x46c2, 0x8a83, 0x361d51f1bf0f)]
    pub trait IWrite : (IBase) {
        /// Writes a string slice into this writer, returning whether the write succeeded.
        fn write_str(&mut self, s: &str) -> Result<(), Error>;
        /// Writes a [`char`] into this writer, returning whether the write succeeded.
        fn write_char(&mut self, c: char) -> Result<(), Error>;
        /// Writes a multiple arguments into this writer, returning whether the write succeeded.
        fn write_fmt(&mut self, args: Arguments<'_>) -> Result<(), Error>;
    }
}

base_vtable! {
    /// VTable for [`IWrite`] objects.
    #![interface = IWrite]
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

impl<'a, T: CastSuper<dyn IWrite + 'a> + ?Sized> IWrite for DynObj<T> {
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
