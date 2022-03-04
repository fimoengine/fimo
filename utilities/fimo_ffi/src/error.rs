//! Error type.
use crate::fmt::{FmtWrapper, Formatter, IDebug, IDebugVTable, IDisplay, IDisplayVTable};
use crate::ptr::{
    from_raw, into_raw, metadata, CastSuper, FetchVTable, ObjInterface, ObjMetadata, ObjectId,
    RawObj,
};
use crate::{base_interface, base_object, base_vtable, impl_upcast, DynObj, ObjBox, Optional};
use std::marker::Unsize;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr::addr_of;

base_interface! {
    /// [`Error`]: std::error::Error equivalent for [`DynObj`] objects.
    #![vtable = IErrorVTable]
    #![uuid(0x4c9db273, 0xb5f5, 0x4edf, 0x9658, 0x4739f2bd4bc5)]
    pub trait IError: (IDebug + IDisplay) {
        /// The lower-level source of this error, if any.
        fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
            None
        }
    }
}

base_vtable! {
    /// VTable for [`IError`] objects.
    #![interface = IError]
    pub struct IErrorVTable {
        /// VTable to the [`IDebug`] implementation.
        pub debug: IDebugVTable,
        /// VTable to the [`IDisplay`] implementation.
        pub display: IDisplayVTable,
        /// The lower-level source of this error, if any.
        pub source: extern "C-unwind" fn(*const ()) -> Optional<RawObj<dyn IError + 'static>>
    }
}

impl_upcast! {
    impl (IError) -> (IDebug) obj: ObjMetadata<_> {
        let vtable: &IErrorVTable = obj.vtable();
        ObjMetadata::new(&vtable.debug)
    }
}

impl_upcast! {
    impl (IError) -> (IDisplay) obj: ObjMetadata<_> {
        let vtable: &IErrorVTable = obj.vtable();
        ObjMetadata::new(&vtable.display)
    }
}

impl IErrorVTable {
    /// Constructs a new vtable for a given type.
    #[inline]
    pub const fn new_for<'a, T>() -> Self
    where
        T: IError + ObjectId + 'a,
    {
        Self::new_for_embedded::<'a, T, dyn IError>(0)
    }

    /// Constructs a new vtable for a given type and interface with a custom offset.
    #[inline]
    pub const fn new_for_embedded<'a, T, Dyn>(offset: usize) -> Self
    where
        T: IError + ObjectId + Unsize<Dyn> + 'a,
        Dyn: ObjInterface + ?Sized + 'a,
    {
        const UNINIT: MaybeUninit<IErrorVTable> = MaybeUninit::uninit();
        const UNINIT_PTR: *const IErrorVTable = UNINIT.as_ptr();
        const IDEBUG_VTABLE_PTR: *const IDebugVTable = unsafe { addr_of!((*UNINIT_PTR).debug) };
        const IDISPLAY_VTABLE_PTR: *const IDisplayVTable =
            unsafe { addr_of!((*UNINIT_PTR).display) };
        const IDEBUG_OFFSET: usize = unsafe {
            (IDEBUG_VTABLE_PTR as *const u8).offset_from(UNINIT_PTR as *const u8) as usize
        };
        const IDISPLAY_OFFSET: usize = unsafe {
            (IDISPLAY_VTABLE_PTR as *const u8).offset_from(UNINIT_PTR as *const u8) as usize
        };

        extern "C-unwind" fn source<T: IError>(
            ptr: *const (),
        ) -> Optional<RawObj<dyn IError + 'static>> {
            let t = unsafe { &*(ptr as *const T) };
            t.source().map(|s| into_raw(s)).into()
        }

        Self::new_embedded::<T, Dyn>(
            offset,
            IDebugVTable::new_for_embedded::<T, Dyn>(IDEBUG_OFFSET),
            IDisplayVTable::new_for_embedded::<T, Dyn>(IDISPLAY_OFFSET),
            source::<T> as _,
        )
    }
}

impl<'a, T: IError + ?Sized> IError for &'a T {
    #[inline]
    fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
        (*self).source()
    }
}

impl<T: IError> IError for ObjBox<T> {
    #[inline]
    fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
        (**self).source()
    }
}

impl<'a, T> IError for DynObj<T>
where
    T: CastSuper<dyn IDebug + 'a>
        + CastSuper<dyn IDisplay + 'a>
        + CastSuper<dyn IError + 'a>
        + ?Sized,
{
    #[inline]
    fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
        let vtable: &IErrorVTable = metadata(self).super_vtable::<dyn IError + 'a>();
        (vtable.source)(self as *const _ as _)
            .into_rust()
            .map(|s| unsafe { &*(from_raw(s)) })
    }
}

impl<'a> From<&'_ str> for ObjBox<DynObj<dyn IError + 'a>> {
    #[inline]
    fn from(s: &'_ str) -> Self {
        From::from(String::from(s))
    }
}

impl<'a> From<&'_ str> for ObjBox<DynObj<dyn IError + Send + Sync + 'a>> {
    #[inline]
    fn from(s: &'_ str) -> Self {
        From::from(String::from(s))
    }
}

impl From<crate::String> for ObjBox<DynObj<dyn IError>> {
    #[inline]
    fn from(v: crate::String) -> Self {
        let obj: ObjBox<DynObj<dyn IError + Send + Sync>> = From::from(v);
        ObjBox::cast_super(obj)
    }
}

impl From<crate::String> for ObjBox<DynObj<dyn IError + Send + Sync>> {
    #[inline]
    fn from(v: crate::String) -> Self {
        struct StringError {
            v: crate::String,
        }

        base_object! {
            #![uuid(0x8626eb10, 0x89d3, 0x4a75, 0xb11d, 0x457e3c4daa9c)]
            impl StringError
        }

        impl IDebug for StringError {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{:?}", &self.v)
            }
        }

        impl IDisplay for StringError {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{:?}", &self.v)
            }
        }

        impl IError for StringError {
            fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
                None
            }
        }

        impl FetchVTable<dyn IError> for StringError {
            fn fetch_interface() -> &'static IErrorVTable {
                static VTABLE: IErrorVTable = IErrorVTable::new_for::<StringError>();
                &VTABLE
            }
        }

        let v = StringError { v };
        let obj = ObjBox::new(v);
        ObjBox::coerce_obj(obj)
    }
}

impl From<String> for ObjBox<DynObj<dyn IError>> {
    #[inline]
    fn from(v: String) -> Self {
        let obj: ObjBox<DynObj<dyn IError + Send + Sync>> = From::from(v);
        ObjBox::cast_super(obj)
    }
}

impl From<String> for ObjBox<DynObj<dyn IError + Send + Sync>> {
    #[inline]
    fn from(v: String) -> Self {
        struct StringError {
            v: String,
        }

        base_object! {
            #![uuid(0xb569493b, 0xe573, 0x4efa, 0x84c9, 0x8b514e861a7b)]
            impl StringError
        }

        impl IDebug for StringError {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{:?}", &self.v)
            }
        }

        impl IDisplay for StringError {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{:?}", &self.v)
            }
        }

        impl IError for StringError {
            fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
                None
            }
        }

        impl FetchVTable<dyn IError> for StringError {
            fn fetch_interface() -> &'static IErrorVTable {
                static VTABLE: IErrorVTable = IErrorVTable::new_for::<StringError>();
                &VTABLE
            }
        }

        let v = StringError { v };
        let obj = ObjBox::new(v);
        ObjBox::coerce_obj(obj)
    }
}

impl<'a, T: IError + 'a> From<T> for ObjBox<DynObj<dyn IError + 'a>> {
    #[inline]
    default fn from(v: T) -> Self {
        struct ErrorWrapper<'a> {
            e: Box<dyn IError + 'a>,
        }

        base_object! {
            #![uuid(0x30fd9f73, 0x42df, 0x41bc, 0xb462, 0x94385325cff7)]
            generic<'a> ErrorWrapper<'a> => ErrorWrapper<'_>
        }

        impl IDebug for ErrorWrapper<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{:?}", FmtWrapper::new_ref(&self.e))
            }
        }

        impl IDisplay for ErrorWrapper<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{}", FmtWrapper::new_ref(&self.e))
            }
        }

        impl IError for ErrorWrapper<'_> {
            fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
                self.e.source()
            }
        }

        impl<'a> FetchVTable<dyn IError + 'a> for ErrorWrapper<'a> {
            fn fetch_interface() -> &'static IErrorVTable {
                static VTABLE: IErrorVTable = IErrorVTable::new_for::<ErrorWrapper<'_>>();
                &VTABLE
            }
        }

        let v = ErrorWrapper { e: Box::new(v) };
        From::from(v)
    }
}

impl<'a, T: IError + FetchVTable<dyn IError + 'a> + 'a> From<T>
    for ObjBox<DynObj<dyn IError + 'a>>
{
    #[inline]
    fn from(v: T) -> Self {
        let obj = ObjBox::new(v);
        ObjBox::coerce_obj(obj)
    }
}

impl<'a, T: IError + Send + Sync + 'a> From<T> for ObjBox<DynObj<dyn IError + Send + Sync + 'a>> {
    #[inline]
    default fn from(v: T) -> Self {
        struct ErrorSendSync<'a> {
            e: Box<dyn IError + Send + Sync + 'a>,
        }

        base_object! {
            #![uuid(0x5c76cb7f, 0x687c, 0x45a3, 0x94b3, 0x6c164fa923a6)]
            generic<'a> ErrorSendSync<'a> => ErrorSendSync<'_>
        }

        impl IDebug for ErrorSendSync<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{:?}", FmtWrapper::new_ref(&self.e))
            }
        }

        impl IDisplay for ErrorSendSync<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
                write!(f, "{}", FmtWrapper::new_ref(&self.e))
            }
        }

        impl IError for ErrorSendSync<'_> {
            fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
                self.e.source()
            }
        }

        impl<'a> FetchVTable<dyn IError + 'a> for ErrorSendSync<'a> {
            fn fetch_interface() -> &'static IErrorVTable {
                static VTABLE: IErrorVTable = IErrorVTable::new_for::<ErrorSendSync<'_>>();
                &VTABLE
            }
        }

        let v = ErrorSendSync { e: Box::new(v) };
        From::from(v)
    }
}

impl<'a, T: IError + Send + Sync + FetchVTable<dyn IError + 'a> + 'a> From<T>
    for ObjBox<DynObj<dyn IError + Send + Sync + 'a>>
{
    #[inline]
    fn from(v: T) -> Self {
        let obj = ObjBox::new(v);
        ObjBox::coerce_obj(obj)
    }
}

/// Wraps an [`IError`] so that it implements [`Error`](std::error::Error).
///
/// # Note
///
/// We currently don't support returning the inner error and instead return [`None`].
#[repr(transparent)]
pub struct ErrorWrapper<T: IError + ?Sized> {
    inner: FmtWrapper<T>,
}

impl<T: IError + ?Sized> ErrorWrapper<T> {
    /// Constructs a new instance of an `ErrorWrapper` taking ownership of the value.
    #[inline]
    pub const fn new(inner: T) -> Self
    where
        T: Sized,
    {
        Self {
            inner: FmtWrapper::new(inner),
        }
    }

    /// Constructs a new instance of an `ErrorWrapper` borrowing the value.
    #[inline]
    pub const fn new_ref(inner: &T) -> &Self {
        unsafe { &*(FmtWrapper::new_ref(inner) as *const _ as *const Self) }
    }

    /// Constructs a new instance of an `ErrorWrapper` borrowing the value mutable.
    #[inline]
    pub fn new_mut(inner: &mut T) -> &mut Self {
        unsafe { &mut *(FmtWrapper::new_mut(inner) as *mut _ as *mut Self) }
    }
}

impl<T: IError + ?Sized> Deref for ErrorWrapper<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<T: IError + ?Sized> DerefMut for ErrorWrapper<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl<T: IError + ?Sized> std::fmt::Debug for ErrorWrapper<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self.inner)
    }
}

impl<T: IError + ?Sized> std::fmt::Display for ErrorWrapper<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

// we currently don't support returning the source error, instead
// we simply return `None`.
impl<T: IError + ?Sized> std::error::Error for ErrorWrapper<T> {}

/// Error type for modules.
#[repr(C)]
pub struct Error {
    repr: ErrorRepr,
}

impl Error {
    /// Creates a new error from a known kind of error as well as an arbitrary payload.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::error::{Error, ErrorKind};
    ///
    /// // errors can be created from strings
    /// let custom_error = Error::new(ErrorKind::Unknown, "oh no!");
    /// ```
    pub fn new(
        kind: ErrorKind,
        error: impl Into<ObjBox<DynObj<dyn IError + Send + Sync>>>,
    ) -> Error {
        Error {
            repr: ErrorRepr::Custom(Box::new(CustomError {
                kind,
                error: error.into(),
            })),
        }
    }

    /// Consumes the `Error`, returning its inner error (if any).
    ///
    /// If this [`Error`] was constructed via [`new`] then this function will
    /// return [`Some`], otherwise it will return [`None`].
    ///
    /// [`new`]: Error::new
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::error::{Error, ErrorKind, ErrorWrapper};
    ///
    /// fn print_error(err: Error) {
    ///     if let Some(inner_err) = err.into_inner() {
    ///         println!("Inner error: {}", ErrorWrapper::new_ref(&*inner_err));
    ///     } else {
    ///         println!("No inner error");
    ///     }
    /// }
    ///
    /// fn main() {
    ///     // Will print "No inner error".
    ///     print_error(ErrorKind::NotFound.into());
    ///     // Will print "Inner error: ...".
    ///     print_error(Error::new(ErrorKind::Unknown, "oh no!"));
    /// }
    /// ```
    pub fn into_inner(self) -> Option<ObjBox<DynObj<dyn IError + Send + Sync>>> {
        match self.repr {
            ErrorRepr::Simple(_) => None,
            ErrorRepr::Custom(c) => Some(c.error),
        }
    }

    /// Returns the corresponding [`ErrorKind`] of this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_ffi::error::{Error, ErrorKind};
    ///
    /// fn print_error(err: Error) {
    ///     println!("{:?}", err.kind());
    /// }
    ///
    /// fn main() {
    ///     // Will print "NotFound".
    ///     print_error(ErrorKind::NotFound.into());
    ///     // Will print "Unknown".
    ///     print_error(Error::new(ErrorKind::Unknown, "oh no!"));
    /// }
    /// ```
    pub fn kind(&self) -> ErrorKind {
        match self.repr {
            ErrorRepr::Simple(kind) => kind,
            ErrorRepr::Custom(ref c) => c.kind,
        }
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self {
            repr: ErrorRepr::Simple(kind),
        }
    }
}

impl std::fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.repr, f)
    }
}

impl std::fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.repr {
            ErrorRepr::Simple(kind) => write!(f, "{}", kind.as_str()),
            ErrorRepr::Custom(ref c) => std::fmt::Display::fmt(FmtWrapper::new_ref(&c.error), f),
        }
    }
}

impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        ErrorWrapper::new_ref(self).source()
    }
}

impl IDebug for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl IDisplay for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), crate::fmt::Error> {
        write!(f, "{}", self)
    }
}

impl IError for Error {
    fn source(&self) -> Option<&DynObj<dyn IError + 'static>> {
        match self.repr {
            ErrorRepr::Simple(_) => None,
            ErrorRepr::Custom(ref c) => c.error.source(),
        }
    }
}

/// gRPC status codes used by [`Error`].
///
/// These variants match the [gRPC status codes].
///
/// [gRPC status codes]: https://github.com/grpc/grpc/blob/master/doc/statuscodes.md#status-codes-and-their-use-in-grpc
#[repr(i8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    /// The operation was cancelled.
    Cancelled = 1,
    /// Unknown error.
    Unknown = 2,
    /// Client specified an invalid argument.
    InvalidArgument = 3,
    /// Deadline expired before operation could complete.
    DeadlineExceeded = 4,
    /// Some requested entity was not found.
    NotFound = 5,
    /// The entity that a client attempted to create already exists.
    AlreadyExists = 6,
    /// The caller does not have permission to execute the specified operation.
    PermissionDenied = 7,
    /// Some resource has been exhausted.
    ResourceExhausted = 8,
    /// The system is not in a state required for the operation's execution.
    FailedPrecondition = 9,
    /// The operation was aborted.
    Aborted = 10,
    /// The operation was attempted past the valid range.
    OutOfRange = 11,
    /// The operation is not implemented or is not supported/enabled.
    Unimplemented = 12,
    /// Internal error.
    Internal = 13,
    /// The service is currently unavailable.
    Unavailable = 14,
    /// Unrecoverable data loss or corruption.
    DataLoss = 15,
    /// The request does not have valid authentication credentials for the operation.
    Unauthenticated = 16,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::Cancelled => "cancelled",
            ErrorKind::Unknown => "unknown error",
            ErrorKind::InvalidArgument => "invalid argument specified",
            ErrorKind::DeadlineExceeded => "operation deadline exceeded",
            ErrorKind::NotFound => "entity not found",
            ErrorKind::AlreadyExists => "entity already exists",
            ErrorKind::PermissionDenied => "permission denied",
            ErrorKind::ResourceExhausted => "resource exhausted",
            ErrorKind::FailedPrecondition => "precondition failed",
            ErrorKind::Aborted => "aborted",
            ErrorKind::OutOfRange => "out of range",
            ErrorKind::Unimplemented => "unimplemented",
            ErrorKind::Internal => "internal error",
            ErrorKind::Unavailable => "unavailable",
            ErrorKind::DataLoss => "data loss",
            ErrorKind::Unauthenticated => "unauthenticated",
        }
    }
}

#[repr(C)]
enum ErrorRepr {
    Simple(ErrorKind),
    Custom(Box<CustomError>),
}

impl std::fmt::Debug for ErrorRepr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorRepr::Simple(kind) => f.debug_tuple("Kind").field(&kind).finish(),
            ErrorRepr::Custom(ref c) => std::fmt::Debug::fmt(&c, f),
        }
    }
}

#[repr(C)]
struct CustomError {
    kind: ErrorKind,
    error: ObjBox<DynObj<dyn IError + Send + Sync>>,
}

impl std::fmt::Debug for CustomError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomError")
            .field("kind", &self.kind)
            .field("error", FmtWrapper::new_ref(&self.error))
            .finish()
    }
}
