use fimo_ffi::error::ToBoxedError;
use fimo_ffi::ObjBox;

/// Result type for modules.
pub type Result<T> = fimo_ffi_core::Result<T, Error>;

/// Error type for modules.
#[repr(C)]
pub struct Error<E = ObjBox<fimo_ffi::Error>> {
    repr: ErrorRepr<E>,
}

impl<E> Error<E> {
    /// Creates a new error from a known kind of error as well as an arbitrary payload.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_module_core::{Error, ErrorKind};
    ///
    /// // errors can be created from strings
    /// let custom_error = <Error>::new(ErrorKind::Unknown, "oh no!");
    /// ```
    pub fn new(kind: ErrorKind, error: impl ToBoxedError<E>) -> Error<E> {
        Error {
            repr: ErrorRepr::Custom(Box::new(CustomError {
                kind,
                error: error.to_error(),
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
    /// use fimo_module_core::{Error, ErrorKind};
    ///
    /// fn print_error(err: Error) {
    ///     if let Some(inner_err) = err.into_inner() {
    ///         println!("Inner error: {}", inner_err);
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
    pub fn into_inner(self) -> Option<E> {
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
    /// use fimo_module_core::{Error, ErrorKind};
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

impl<E> From<ErrorKind> for Error<E> {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self {
            repr: ErrorRepr::Simple(kind),
        }
    }
}

impl<E: std::fmt::Debug> std::fmt::Debug for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.repr, f)
    }
}

impl<E: std::fmt::Display> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.repr {
            ErrorRepr::Simple(kind) => write!(f, "{}", kind.as_str()),
            ErrorRepr::Custom(ref c) => std::fmt::Display::fmt(&c.error, f),
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
enum ErrorRepr<E> {
    Simple(ErrorKind),
    Custom(Box<CustomError<E>>),
}

impl<E: std::fmt::Debug> std::fmt::Debug for ErrorRepr<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorRepr::Simple(kind) => f.debug_tuple("Kind").field(&kind).finish(),
            ErrorRepr::Custom(ref c) => std::fmt::Debug::fmt(&c, f),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct CustomError<E> {
    kind: ErrorKind,
    error: E,
}
