//! Error type.
use crate::object::CoerceObjectMut;
use crate::{fimo_object, fimo_vtable, ObjBox, Optional, StrInner};
use fimo_object::object::{ObjPtrCompat, ObjectWrapper};
use fimo_object::{fimo_marker, impl_vtable, is_object};
use std::fmt::Write;

fimo_object! {
    /// Interface of an error.
    // Don't generate a debug implementation, as we are gonna derive it manually.
    pub struct IError<vtable = IErrorVTable, no_debug>;
}

impl IError {
    /// Lower-level source, if it exists.
    #[inline]
    pub fn source(&self) -> Option<&IError> {
        let (ptr, vtable) = crate::object::into_raw_parts(&self.inner);
        unsafe { (vtable.source)(ptr).into_rust().map(|e| &*e) }
    }
}

impl std::fmt::Debug for IError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut wrapper = FormatterWrapper { inner: f };
        let w = IWriter::from_object_mut_raw(wrapper.coerce_obj_mut());
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            (vtable.debug)(ptr, w)
                .into_rust()
                .map_err(|_| std::fmt::Error)
        }
    }
}

impl std::fmt::Display for IError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut wrapper = FormatterWrapper { inner: f };
        let w = IWriter::from_object_mut_raw(wrapper.coerce_obj_mut());
        let (ptr, vtable) = self.into_raw_parts();
        unsafe {
            (vtable.display)(ptr, w)
                .into_rust()
                .map_err(|_| std::fmt::Error)
        }
    }
}

fimo_marker! {
    /// `Send` and `Sync` marker.
    #[derive(Debug)]
    #![requires(Send, Sync)]
    pub marker SendSync;
}

fimo_vtable! {
    /// VTable of an [`IError`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![marker = SendSync]
    #![uuid(0xe7af13dd, 0xadfd, 0x4541, 0xa0fa, 0x173b2f200e65)]
    pub struct IErrorVTable {
        /// Lower-level source, if it exists.
        pub source: unsafe extern "C" fn(*const ()) -> Optional<*const IError>,
        /// Debug formatted error info.
        pub debug: unsafe extern "C" fn(*const (), *mut IWriter) -> crate::Result<(), WriteError>,
        /// Display formatted error info.
        pub display: unsafe extern "C" fn(*const (), *mut IWriter) -> crate::Result<(), WriteError>,
    }
}

/// The error type that is returned from writing a message into a stream.
#[derive(Copy, Clone, Debug, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct WriteError;

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt("an error occurred when writing into a stream", f)
    }
}

fimo_object! {
    /// Interface for writing into a steam/buffer.
    pub struct IWriter<vtable = IWriterVTable>;
}

impl IWriter {
    /// Writes a string into the buffer/stream.
    pub fn write_str(&mut self, s: &str) -> Result<(), WriteError> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        unsafe { (vtable.write_str)(ptr, s.into()).into_rust() }
    }

    /// Writes a character into the buffer/stream.
    pub fn write_char(&mut self, c: char) -> Result<(), WriteError> {
        let (ptr, vtable) = self.into_raw_parts_mut();
        unsafe { (vtable.write_char)(ptr, c as u32).into_rust() }
    }
}

fimo_vtable! {
    /// VTable of an [`IWriter`].
    #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
    #![uuid(0x75cff6a6, 0xf163, 0x4578, 0x8290, 0x20a4d157c338)]
    pub struct IWriterVTable {
        /// Writes a string into the buffer/stream.
        pub write_str: unsafe extern "C" fn(*mut (), StrInner<false>) -> crate::Result<(), WriteError>,
        /// Writes a character into the buffer/stream.
        pub write_char: unsafe extern "C" fn(*mut (), u32) -> crate::Result<(), WriteError>
    }
}

struct FormatterWrapper<'a, 'b> {
    inner: &'a mut std::fmt::Formatter<'b>,
}

impl std::fmt::Write for FormatterWrapper<'_, '_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.inner.write_str(s)
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.inner.write_char(c)
    }

    #[allow(clippy::needless_arbitrary_self_type)]
    fn write_fmt(self: &mut Self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.inner.write_fmt(args)
    }
}

is_object! { #![uuid(0x86f338ae, 0x9c3d, 0x4ec7, 0xb9e4, 0x92c3deb010dd)] FormatterWrapper<'_, '_> }

impl_vtable! {
    impl mut IWriterVTable => FormatterWrapper<'_, '_> {
        unsafe extern "C" fn write_str(
            ptr: *mut (),
            s: StrInner<false>,
        ) -> crate::Result<(), WriteError> {
            let w = &mut *(ptr as *mut FormatterWrapper<'_, '_>);
            w.write_str(s.into()).map_err(|_| WriteError).into()
        }

        unsafe extern "C" fn write_char(ptr: *mut (), c: u32) -> crate::Result<(), WriteError> {
            let w = &mut *(ptr as *mut FormatterWrapper<'_, '_>);
            let c = char::from_u32_unchecked(c);
            w.write_char(c).map_err(|_| WriteError).into()
        }
    }
}

/// Trait for casting a type to a boxed error.
pub trait ToBoxedError<B> {
    /// Boxes the type to the specified error value.
    fn to_error(self) -> B;
}

/// Trait for complex errors wrapping an internal error.
pub trait InnerError: std::fmt::Debug + std::fmt::Display + Send + Sync {
    /// Returns a reference to the internal error.
    fn source(&self) -> Option<&IError>;
}

impl InnerError for &'_ str {
    #[inline]
    fn source(&self) -> Option<&IError> {
        None
    }
}

impl InnerError for String {
    #[inline]
    fn source(&self) -> Option<&IError> {
        None
    }
}

impl InnerError for IError {
    #[inline]
    fn source(&self) -> Option<&IError> {
        self.source()
    }
}

impl<T: ObjPtrCompat + InnerError + ?Sized> InnerError for ObjBox<T> {
    #[inline]
    fn source(&self) -> Option<&IError> {
        (**self).source()
    }
}

impl<T: InnerError + ?Sized> InnerError for Box<T> {
    #[inline]
    fn source(&self) -> Option<&IError> {
        (**self).source()
    }
}

impl<'a> ToBoxedError<Box<dyn std::error::Error + Send + Sync>> for &'a str {
    fn to_error(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.into()
    }
}

trait DisplayDebug: std::fmt::Display + std::fmt::Debug + Send + Sync {}

impl<T: std::fmt::Display + std::fmt::Debug + Send + Sync + ?Sized> DisplayDebug for T {}

impl<T: DisplayDebug + 'static> ToBoxedError<ObjBox<IError>> for T {
    default fn to_error(self) -> ObjBox<IError> {
        let b = ObjBox::new(SimpleErrorWrapper { e: Box::new(self) });
        ObjBox::coerce_object(b)
    }
}

impl<T: InnerError + 'static> ToBoxedError<ObjBox<IError>> for T {
    fn to_error(self) -> ObjBox<IError> {
        let b = ObjBox::new(ErrorWrapper { e: Box::new(self) });
        ObjBox::coerce_object(b)
    }
}

#[allow(missing_debug_implementations)]
struct SimpleErrorWrapper {
    e: Box<dyn DisplayDebug>,
}

is_object! { #![uuid(0xf1aa569c, 0xdf5c, 0x4eb6, 0xa726, 0x1f717e1413f8)] SimpleErrorWrapper }

impl_vtable! {
    impl mut IErrorVTable => SimpleErrorWrapper {
        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn source(_e: *const ()) -> Optional<*const IError> {
            Optional::None
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn debug(e: *const (), w: *mut IWriter) -> crate::Result<(), WriteError> {
            let e = &*(e as *const SimpleErrorWrapper);
            let w = &mut *w;

            let s = format!("{:?}", e.e);
            w.write_str(s.as_str()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn display(
            e: *const (),
            w: *mut IWriter,
        ) -> crate::Result<(), WriteError> {
            let e = &*(e as *const SimpleErrorWrapper);
            let w = &mut *w;

            let s = format!("{}", e.e);
            w.write_str(s.as_str()).into()
        }
    }
}

#[allow(missing_debug_implementations)]
struct ErrorWrapper {
    e: Box<dyn InnerError>,
}

is_object! { #![uuid(0x02c27279, 0xfdc1, 0x4e15, 0x8e6b, 0xe4e0b5be2a7b)] ErrorWrapper }

impl_vtable! {
    impl mut IErrorVTable => ErrorWrapper {
        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn source(e: *const ()) -> Optional<*const IError> {
            let e = &*(e as *const ErrorWrapper);
            e.e.source().map(|i| i as _).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn debug(e: *const (), w: *mut IWriter) -> crate::Result<(), WriteError> {
            let e = &*(e as *const ErrorWrapper);
            let w = &mut *w;

            let s = format!("{:?}", e.e);
            w.write_str(s.as_str()).into()
        }

        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn display(
            e: *const (),
            w: *mut IWriter,
        ) -> crate::Result<(), WriteError> {
            let e = &*(e as *const ErrorWrapper);
            let w = &mut *w;

            let s = format!("{}", e.e);
            w.write_str(s.as_str()).into()
        }
    }
}
