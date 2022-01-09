//! Error type.
use crate::object::{CoerceObject, CoerceObjectMut};
use crate::vtable::ObjectID;
use crate::{fimo_object, fimo_vtable, ObjBox, Optional, StrInner};

fimo_object! {
    /// Type-erased error type.
    // Don't generate a debug implementation, as we are gonna derive it manually.
    pub struct Error<vtable = ErrorVTable, no_debug>;
}

impl Error {
    /// Lower-level source, if it exists.
    #[inline]
    pub fn source(&self) -> Option<&Error> {
        let (ptr, vtable) = crate::object::into_raw_parts(&self.inner);
        unsafe { (vtable.source)(ptr).into_rust().map(|e| &*e) }
    }

    /// Debug formatted error info.
    #[inline]
    fn debug(&self) -> &str {
        let (ptr, vtable) = crate::object::into_raw_parts(&self.inner);
        unsafe { (vtable.debug)(ptr).into() }
    }

    /// Display formatted error info.
    #[inline]
    fn display(&self) -> &str {
        let (ptr, vtable) = crate::object::into_raw_parts(&self.inner);
        unsafe { (vtable.display)(ptr).into() }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// `Send` and `Sync` marker.
#[derive(Debug)]
pub struct SendSync;

fimo_vtable! {
    /// Error vtable.
    pub struct ErrorVTable<id = "fimo::ffi::error", marker = SendSync> {
        /// Lower-level source, if it exists.
        pub source: unsafe extern "C" fn(*const ()) -> Optional<*const Error>,
        /// Debug formatted error info.
        pub debug: unsafe extern "C" fn(*const ()) -> StrInner<false>,
        /// Display formatted error info.
        pub display: unsafe extern "C" fn(*const ()) -> StrInner<false>,
    }
}

/// Trait for casting a type to a boxed error.
pub trait ToBoxedError<B> {
    /// Boxes the type to the specified error value.
    fn to_error(self) -> B;
}

impl<'a> ToBoxedError<Box<dyn std::error::Error + Send + Sync>> for &'a str {
    fn to_error(self) -> Box<dyn std::error::Error + Send + Sync> {
        self.into()
    }
}

impl<T: std::fmt::Debug + std::fmt::Display> ToBoxedError<ObjBox<Error>> for T {
    fn to_error(self) -> ObjBox<Error> {
        let b = ObjBox::new(ErrorWrapper {
            debug: format!("{:?}", self),
            display: format!("{}", self),
        });

        ObjBox::coerce_object(b)
    }
}

#[allow(missing_debug_implementations)]
struct ErrorWrapper {
    debug: String,
    display: String,
}

impl ObjectID for ErrorWrapper {
    const OBJECT_ID: &'static str = "fimo::ffi::error::error_wrapper";
}

impl CoerceObject<ErrorVTable> for ErrorWrapper {
    fn get_vtable() -> &'static ErrorVTable {
        #[allow(improper_ctypes_definitions)]
        unsafe extern "C" fn source(_e: *const ()) -> Optional<*const Error> {
            Optional::None
        }
        unsafe extern "C" fn debug(e: *const ()) -> StrInner<false> {
            (*(e as *const ErrorWrapper)).debug.as_str().into()
        }
        unsafe extern "C" fn display(e: *const ()) -> StrInner<false> {
            (*(e as *const ErrorWrapper)).display.as_str().into()
        }

        static VTABLE: ErrorVTable = ErrorVTable::new::<ErrorWrapper>(source, debug, display);
        &VTABLE
    }
}

impl CoerceObjectMut<ErrorVTable> for ErrorWrapper {}
