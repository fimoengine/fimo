//! Fimo error codes.

use crate::{
    bindings, handle,
    modules::symbols::{AssertSharable, Share},
    utils::OpaqueHandle,
};
use std::{
    ffi::{CStr, CString},
    fmt::{Debug, Display, Formatter},
    marker::{PhantomData, Unsize},
    mem::{ManuallyDrop, MaybeUninit},
    ops::Deref,
};

/// Posix error codes.
#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorCode {
    /// Operation completed successfully
    Ok,
    /// Argument list too long
    E2BIG,
    /// Permission denied
    EACCES,
    /// Address already in use
    EADDRINUSE,
    /// Address not available
    EADDRNOTAVAIL,
    /// Address family not supported
    EAFNOSUPPORT,
    /// Resource temporarily unavailable
    EAGAIN,
    /// Connection already in progress
    EALREADY,
    /// Invalid exchange
    EBADE,
    /// Bad file descriptor
    EBADF,
    /// File descriptor in bad state
    EBADFD,
    /// Bad message
    EBADMSG,
    /// Invalid request descriptor
    EBADR,
    /// Invalid request code
    EBADRQC,
    /// Invalid slot
    EBADSLT,
    /// Device or resource busy
    EBUSY,
    /// Operation canceled
    ECANCELED,
    /// No child processes
    ECHILD,
    /// Channel number out of range
    ECHRNG,
    /// Communication error on send
    ECOMM,
    /// Connection aborted
    ECONNABORTED,
    /// Connection refused
    ECONNREFUSED,
    /// Connection reset
    ECONNRESET,
    /// Resource deadlock avoided
    EDEADLK,
    /// File locking deadlock error (or Resource deadlock avoided)
    EDEADLOCK,
    /// Destination address required
    EDESTADDRREQ,
    /// Mathematics argument out of domain of function
    EDOM,
    /// Disk quota exceeded
    EDQUOT,
    /// File exists
    EEXIST,
    /// Bad address
    EFAULT,
    /// File too large
    EFBIG,
    /// Host is down
    EHOSTDOWN,
    /// Host is unreachable
    EHOSTUNREACH,
    /// Memory page has hardware error
    EHWPOISON,
    /// Identifier removed
    EIDRM,
    /// Invalid or incomplete multibyte or wide character
    EILSEQ,
    /// Operation in progress
    EINPROGRESS,
    /// Interrupted function call
    EINTR,
    /// Invalid argument
    EINVAL,
    /// Input/output error
    EIO,
    /// Socket is connected
    EISCONN,
    /// Is a directory
    EISDIR,
    /// Is a named type file
    EISNAM,
    /// Key has expired
    EKEYEXPIRED,
    /// Key was rejected by service
    EKEYREJECTED,
    /// Key has been revoked
    EKEYREVOKED,
    /// Level 2 halted
    EL2HLT,
    /// Level 2 not synchronized
    EL2NSYNC,
    /// Level 3 halted
    EL3HLT,
    /// Level 3 reset
    EL3RST,
    /// Cannot access a needed shared library
    ELIBACC,
    /// Accessing a corrupted shared library
    ELIBBAD,
    /// Attempting to link in too many shared libraries
    ELIBMAX,
    /// .lib section in a.out corrupted
    ELIBSCN,
    /// Cannot exec a shared library directly
    ELIBEXEC,
    /// Link number out of range
    ELNRNG,
    /// Too many levels of symbolic links
    ELOOP,
    /// Wrong medium type
    EMEDIUMTYPE,
    /// Too many open files
    EMFILE,
    /// Too many links
    EMLINK,
    /// Message too long
    EMSGSIZE,
    /// Multihop attempted
    EMULTIHOP,
    /// Filename too long
    ENAMETOOLONG,
    /// Network is down
    ENETDOWN,
    /// Connection aborted by network
    ENETRESET,
    /// Network unreachable
    ENETUNREACH,
    /// Too many open files in system
    ENFILE,
    /// No anode
    ENOANO,
    /// No buffer space available
    ENOBUFS,
    /// The named attribute does not exist, or the process has no access to this attribute
    ENODATA,
    /// No such device
    ENODEV,
    /// No such file or directory
    ENOENT,
    /// Exec format error
    ENOEXEC,
    /// Required key not available
    ENOKEY,
    /// No locks available
    ENOLCK,
    /// Link has been severed
    ENOLINK,
    /// No medium found
    ENOMEDIUM,
    /// Not enough space/cannot allocate memory
    ENOMEM,
    /// No message of the desired type
    ENOMSG,
    /// Machine is not on the network
    ENONET,
    /// Package not installed
    ENOPKG,
    /// Protocol not available
    ENOPROTOOPT,
    /// No space left on device
    ENOSPC,
    /// No STREAM resources
    ENOSR,
    /// Not a STREAM
    ENOSTR,
    /// Function not implemented
    ENOSYS,
    /// Block device required
    ENOTBLK,
    /// The socket is not connected
    ENOTCONN,
    /// Not a directory
    ENOTDIR,
    /// Directory not empty
    ENOTEMPTY,
    /// State not recoverable
    ENOTRECOVERABLE,
    /// Not a socket
    ENOTSOCK,
    /// Operation not supported
    ENOTSUP,
    /// Inappropriate I/O control operation
    ENOTTY,
    /// Name not unique on network
    ENOTUNIQ,
    /// No such device or address
    ENXIO,
    /// Operation not supported on socket
    EOPNOTSUPP,
    /// Value too large to be stored in data type
    EOVERFLOW,
    /// Owner died
    EOWNERDEAD,
    /// Operation not permitted
    EPERM,
    /// Protocol family not supported
    EPFNOSUPPORT,
    /// Broken pipe
    EPIPE,
    /// Protocol error
    EPROTO,
    /// Protocol not supported
    EPROTONOSUPPORT,
    /// Protocol wrong type for socket
    EPROTOTYPE,
    /// Result too large
    ERANGE,
    /// Remote address changed
    EREMCHG,
    /// Object is remote
    EREMOTE,
    /// Remote I/O error
    EREMOTEIO,
    /// Interrupted system call should be restarted
    ERESTART,
    /// Operation not possible due to RF-kill
    ERFKILL,
    /// Read-only filesystem
    EROFS,
    /// Cannot send after transport endpoint shutdown
    ESHUTDOWN,
    /// Invalid seek
    ESPIPE,
    /// Socket type not supported
    ESOCKTNOSUPPORT,
    /// No such process
    ESRCH,
    /// Stale file handle
    ESTALE,
    /// Streams pipe error
    ESTRPIPE,
    /// Timer expired
    ETIME,
    /// Connection timed out
    ETIMEDOUT,
    /// Too many references: cannot splice
    ETOOMANYREFS,
    /// Text file busy
    ETXTBSY,
    /// Structure needs cleaning
    EUCLEAN,
    /// Protocol driver not attached
    EUNATCH,
    /// Too many users
    EUSERS,
    /// Operation would block
    EWOULDBLOCK,
    /// Invalid cross-device link
    EXDEV,
    /// Exchange full
    EXFULL,
}

/// Error code of the operating system.
#[cfg(windows)]
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemErrorCode(pub u32);

/// Error code of the operating system.
#[cfg(not(windows))]
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemErrorCode(pub i32);

/// Handle to an error.
pub type ErrorHandle<T> = OpaqueHandle<T>;

/// Virtual function table of [`AnyResult`] and [`AnyError`].
///
/// Adding fields to the vtable is a breaking change.
#[repr(C)]
#[derive(Debug)]
pub struct VTable {
    pub drop: Option<unsafe extern "C" fn(handle: Option<ErrorHandle<dyn Share>>)>,
    pub error_name: unsafe extern "C" fn(handle: Option<ErrorHandle<dyn Share>>) -> ErrorString,
    pub error_description:
        unsafe extern "C" fn(handle: Option<ErrorHandle<dyn Share>>) -> ErrorString,
}

unsafe extern "C" {
    static FIMO_IMPL_RESULT_ERROR_CODE_VTABLE: AssertSharable<VTable>;
    static FIMO_IMPL_RESULT_STATIC_STRING_VTABLE: AssertSharable<VTable>;
    static FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE: AssertSharable<VTable>;

    static FIMO_IMPL_RESULT_OK_NAME: ErrorString;
    static FIMO_IMPL_RESULT_OK_DESCRIPTION: ErrorString;
}

/// FFI equivalent of a `Result<(), AnyError>`.
#[repr(C)]
pub struct AnyResult<T: private::Sealed + ?Sized = dyn Share> {
    pub handle: Option<ErrorHandle<T>>,
    pub vtable: Option<&'static AssertSharable<VTable>>,
    _private: PhantomData<()>,
}

impl<T: private::Sealed + ?Sized> AnyResult<T> {
    /// An `AnyResult` equivalent of an Ok(()).
    pub const OK: Self = Self {
        handle: None,
        vtable: None,
        _private: PhantomData,
    };

    /// Constructs an `Ok` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::AnyResult;
    ///
    /// let result = <AnyResult>::new_ok();
    /// assert!(result.is_ok());
    /// println!("{result:?}");
    /// ```
    pub const fn new_ok() -> Self {
        Self::OK
    }

    /// Constructs an `Err` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::{AnyResult, AnyError};
    ///
    /// let error = <AnyError>::new(0u32);
    /// let result = <AnyResult>::new_err(error);
    /// assert!(result.is_err());
    /// println!("{result:?}");
    /// ```
    pub const fn new_err(error: AnyError<T>) -> Self {
        let handle = error.handle;
        let vtable = Some(error.vtable);
        _ = ManuallyDrop::new(error);

        Self {
            handle,
            vtable,
            _private: PhantomData,
        }
    }

    /// Returns whether the result is not an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::{AnyResult, AnyError};
    ///
    /// let result = <AnyResult>::new_ok();
    /// assert!(result.is_ok());
    ///
    /// let error = <AnyError>::new(0u32);
    /// let result = <AnyResult>::new_err(error);
    /// assert!(!result.is_ok());
    /// ```
    pub const fn is_ok(&self) -> bool {
        self.vtable.is_none()
    }

    /// Returns whether the result is an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::{AnyResult, AnyError};
    ///
    /// let result = <AnyResult>::new_ok();
    /// assert!(!result.is_err());
    ///
    /// let error = <AnyError>::new(0u32);
    /// let result = <AnyResult>::new_err(error);
    /// assert!(result.is_err());
    /// ```
    pub const fn is_err(&self) -> bool {
        self.vtable.is_some()
    }

    /// Returns a string representing the error.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::{AnyResult, AnyError};
    ///
    /// let error = <AnyError>::new(0u32);
    /// let result = <AnyResult>::new_err(error);
    /// assert_eq!(result.name().to_string_lossy(), format!("{:?}", 0u32));
    /// ```
    pub fn name(&self) -> ErrorString {
        match self.vtable {
            None => unsafe { (&raw const FIMO_IMPL_RESULT_OK_NAME).read() },
            Some(vtable) => unsafe { (vtable.error_name)(self.handle.map(OpaqueHandle::coerce)) },
        }
    }

    /// Returns a string describing the error.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::{AnyResult, AnyError};
    ///
    /// let error = <AnyError>::new(55u32);
    /// let result = <AnyResult>::new_err(error);
    /// assert_eq!(result.description().to_string_lossy(), format!("{}", 55u32));
    /// ```
    pub fn description(&self) -> ErrorString {
        match self.vtable {
            None => unsafe { (&raw const FIMO_IMPL_RESULT_OK_DESCRIPTION).read() },
            Some(vtable) => unsafe {
                (vtable.error_description)(self.handle.map(OpaqueHandle::coerce))
            },
        }
    }

    /// Constructs a [`Result`] from an `AnyResult`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::{AnyResult, AnyError};
    ///
    /// let result = <AnyResult>::new_ok();
    /// assert!(result.into_result().is_ok());
    ///
    /// let error = <AnyError>::new(1337u32);
    /// let result = <AnyResult>::new_err(error);
    /// assert!(result.into_result().is_err());
    /// ```
    pub const fn into_result(self) -> Result<(), AnyError<T>> {
        if self.vtable.is_some() {
            let this = ManuallyDrop::new(self);
            Err(unsafe { std::mem::transmute::<ManuallyDrop<Self>, AnyError<T>>(this) })
        } else {
            _ = ManuallyDrop::new(self);
            Ok(())
        }
    }

    #[doc(hidden)]
    pub fn into_error(self) -> bindings::FimoResult {
        let this = ManuallyDrop::new(self);
        unsafe { std::mem::transmute::<ManuallyDrop<Self>, bindings::FimoResult>(this) }
    }
}

impl<T: private::Sealed + ?Sized> Debug for AnyResult<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        f.debug_tuple("AnyResult").field(&&*name).finish()
    }
}

impl<T: private::Sealed + ?Sized> Display for AnyResult<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let description = self.description();
        write!(f, "{}", description.to_string_lossy())
    }
}

impl<T: private::Sealed + ?Sized> Default for AnyResult<T> {
    fn default() -> Self {
        Self::OK
    }
}

impl<T: private::Sealed + ?Sized> From<AnyResult<T>> for Result<(), AnyError<T>> {
    fn from(value: AnyResult<T>) -> Self {
        value.into_result()
    }
}

impl<T: private::Sealed + ?Sized> From<AnyError<T>> for AnyResult<T> {
    fn from(value: AnyError<T>) -> Self {
        Self::new_err(value)
    }
}

impl<T: private::Sealed + ?Sized> From<Result<(), AnyError<T>>> for AnyResult<T> {
    fn from(value: Result<(), AnyError<T>>) -> Self {
        match value {
            Ok(_) => Default::default(),
            Err(e) => e.into(),
        }
    }
}

impl<T: private::Sealed + ?Sized> Drop for AnyResult<T> {
    fn drop(&mut self) {
        let Some(vtable) = self.vtable else { return };
        unsafe {
            let Some(drop) = vtable.drop else { return };
            drop(self.handle.map(OpaqueHandle::coerce));
        }
    }
}

/// Generic error value.
#[repr(C)]
pub struct AnyError<T: private::Sealed + ?Sized = dyn Share> {
    pub handle: Option<ErrorHandle<T>>,
    pub vtable: &'static AssertSharable<VTable>,
    _private: PhantomData<()>,
}

mod __private {
    use super::*;
    use paste::paste;

    macro_rules! new_error {
        ($code:ident, $($doc:literal),+) => {
            paste! {
                impl<T: private::Sealed + ?Sized> AnyError<T> {
                    $(
                        #[doc = $doc]
                    )+
                    #[allow(non_snake_case)]
                    pub fn $code() -> Self
                    where
                        AssertSharable<ErrorCode>: Unsize<T>
                    {
                        unsafe {
                            Self::from_error_code_unchecked(ErrorCode::$code)
                        }
                    }
                }
            }
        };
    }

    impl<T: private::Sealed + ?Sized> AnyError<T> {
        pub(super) const FFI_OK_RESULT: bindings::FimoResult = bindings::FimoResult {
            data: std::ptr::null_mut(),
            vtable: std::ptr::null_mut(),
        };
    }

    new_error!(E2BIG, "Argument list too long");
    new_error!(EACCES, "Permission denied");
    new_error!(EADDRINUSE, "Address already in use");
    new_error!(EADDRNOTAVAIL, "Address not available");
    new_error!(EAFNOSUPPORT, "Address family not supported");
    new_error!(EAGAIN, "Resource temporarily unavailable");
    new_error!(EALREADY, "Connection already in progress");
    new_error!(EBADE, "Invalid exchange");
    new_error!(EBADF, "Bad file descriptor");
    new_error!(EBADFD, "File descriptor in bad state");
    new_error!(EBADMSG, "Bad message");
    new_error!(EBADR, "Invalid request descriptor");
    new_error!(EBADRQC, "Invalid request code");
    new_error!(EBADSLT, "Invalid slot");
    new_error!(EBUSY, "Device or resource busy");
    new_error!(ECANCELED, "Operation canceled");
    new_error!(ECHILD, "No child processes");
    new_error!(ECHRNG, "Channel number out of range");
    new_error!(ECOMM, "Communication error on send");
    new_error!(ECONNABORTED, "Connection aborted");
    new_error!(ECONNREFUSED, "Connection refused");
    new_error!(ECONNRESET, "Connection reset");
    new_error!(EDEADLK, "Resource deadlock avoided");
    new_error!(
        EDEADLOCK,
        "File locking deadlock error (or Resource deadlock avoided)"
    );
    new_error!(EDESTADDRREQ, "Destination address required");
    new_error!(EDOM, "Mathematics argument out of domain of function");
    new_error!(EDQUOT, "Disk quota exceeded");
    new_error!(EEXIST, "File exists");
    new_error!(EFAULT, "Bad address");
    new_error!(EFBIG, "File too large");
    new_error!(EHOSTDOWN, "Host is down");
    new_error!(EHOSTUNREACH, "Host is unreachable");
    new_error!(EHWPOISON, "Memory page has hardware error");
    new_error!(EIDRM, "Identifier removed");
    new_error!(EILSEQ, "Invalid or incomplete multibyte or wide character");
    new_error!(EINPROGRESS, "Operation in progress");
    new_error!(EINTR, "Interrupted function call");
    new_error!(EINVAL, "Invalid argument");
    new_error!(EIO, "Input/output error");
    new_error!(EISCONN, "Socket is connected");
    new_error!(EISDIR, "Is a directory");
    new_error!(EISNAM, "Is a named type file");
    new_error!(EKEYEXPIRED, "Key has expired");
    new_error!(EKEYREJECTED, "Key was rejected by service");
    new_error!(EKEYREVOKED, "Key has been revoked");
    new_error!(EL2HLT, "Level 2 halted");
    new_error!(EL2NSYNC, "Level 2 not synchronized");
    new_error!(EL3HLT, "Level 3 halted");
    new_error!(EL3RST, "Level 3 reset");
    new_error!(ELIBACC, "Cannot access a needed shared library");
    new_error!(ELIBBAD, "Accessing a corrupted shared library");
    new_error!(ELIBMAX, "Attempting to link in too many shared libraries");
    new_error!(ELIBSCN, ".lib section in a.out corrupted");
    new_error!(ELIBEXEC, "Cannot exec a shared library directly");
    new_error!(ELNRNG, "Link number out of range");
    new_error!(ELOOP, "Too many levels of symbolic links");
    new_error!(EMEDIUMTYPE, "Wrong medium type");
    new_error!(EMFILE, "Too many open files");
    new_error!(EMLINK, "Too many links");
    new_error!(EMSGSIZE, "Message too long");
    new_error!(EMULTIHOP, "Multihop attempted");
    new_error!(ENAMETOOLONG, "Filename too long");
    new_error!(ENETDOWN, "Network is down");
    new_error!(ENETRESET, "Connection aborted by network");
    new_error!(ENETUNREACH, "Network unreachable");
    new_error!(ENFILE, "Too many open files in system");
    new_error!(ENOANO, "No anode");
    new_error!(ENOBUFS, "No buffer space available");
    new_error!(
        ENODATA,
        "The named attribute does not exist, or the process has no access to this attribute"
    );
    new_error!(ENODEV, "No such device");
    new_error!(ENOENT, "No such file or directory");
    new_error!(ENOEXEC, "Exec format error");
    new_error!(ENOKEY, "Required key not available");
    new_error!(ENOLCK, "No locks available");
    new_error!(ENOLINK, "Link has been severed");
    new_error!(ENOMEDIUM, "No medium found");
    new_error!(ENOMEM, "Not enough space/cannot allocate memory");
    new_error!(ENOMSG, "No message of the desired type");
    new_error!(ENONET, "Machine is not on the network");
    new_error!(ENOPKG, "Package not installed");
    new_error!(ENOPROTOOPT, "Protocol not available");
    new_error!(ENOSPC, "No space left on device");
    new_error!(ENOSR, "No STREAM resources");
    new_error!(ENOSTR, "Not a STREAM");
    new_error!(ENOSYS, "Function not implemented");
    new_error!(ENOTBLK, "Block device required");
    new_error!(ENOTCONN, "The socket is not connected");
    new_error!(ENOTDIR, "Not a directory");
    new_error!(ENOTEMPTY, "Directory not empty");
    new_error!(ENOTRECOVERABLE, "State not recoverable");
    new_error!(ENOTSOCK, "Not a socket");
    new_error!(ENOTSUP, "Operation not supported");
    new_error!(ENOTTY, "Inappropriate I/O control operation");
    new_error!(ENOTUNIQ, "Name not unique on network");
    new_error!(ENXIO, "No such device or address");
    new_error!(EOPNOTSUPP, "Operation not supported on socket");
    new_error!(EOVERFLOW, "Value too large to be stored in data type");
    new_error!(EOWNERDEAD, "Owner died");
    new_error!(EPERM, "Operation not permitted");
    new_error!(EPFNOSUPPORT, "Protocol family not supported");
    new_error!(EPIPE, "Broken pipe");
    new_error!(EPROTO, "Protocol error");
    new_error!(EPROTONOSUPPORT, "Protocol not supported");
    new_error!(EPROTOTYPE, "Protocol wrong type for socket");
    new_error!(ERANGE, "Result too large");
    new_error!(EREMCHG, "Remote address changed");
    new_error!(EREMOTE, "Object is remote");
    new_error!(EREMOTEIO, "Remote I/O error");
    new_error!(ERESTART, "Interrupted system call should be restarted");
    new_error!(ERFKILL, "Operation not possible due to RF-kill");
    new_error!(EROFS, "Read-only filesystem");
    new_error!(ESHUTDOWN, "Cannot send after transport endpoint shutdown");
    new_error!(ESPIPE, "Invalid seek");
    new_error!(ESOCKTNOSUPPORT, "Socket type not supported");
    new_error!(ESRCH, "No such process");
    new_error!(ESTALE, "Stale file handle");
    new_error!(ESTRPIPE, "Streams pipe error");
    new_error!(ETIME, "Timer expired");
    new_error!(ETOOMANYREFS, "Too many references: cannot splice");
    new_error!(ETXTBSY, "Text file busy");
    new_error!(EUCLEAN, "Structure needs cleaning");
    new_error!(EUNATCH, "Protocol driver not attached");
    new_error!(EUSERS, "Too many users");
    new_error!(EWOULDBLOCK, "Operation would block");
    new_error!(EXDEV, "Invalid cross-device link");
    new_error!(EXFULL, "Exchange full");
}

impl AnyError {
    /// Creates an [`AnyError`] from an arbitrary value that can be formatted.
    pub fn new(value: impl Display + Debug + 'static) -> Self {
        unsafe { Self::new_error(value) }
    }
}

impl AnyError<dyn Send + Share> {
    /// Creates an [`AnyError`] from an arbitrary value that can be formatted.
    pub fn new_send(value: impl Display + Debug + Send + 'static) -> Self {
        unsafe { Self::new_error(value) }
    }
}

impl AnyError<dyn Sync + Share> {
    /// Creates an [`AnyError`] from an arbitrary value that can be formatted.
    pub fn new_sync(value: impl Display + Debug + Sync + 'static) -> Self {
        unsafe { Self::new_error(value) }
    }
}

impl AnyError<dyn Send + Sync + Share> {
    /// Creates an [`AnyError`] from an arbitrary value that can be formatted.
    pub fn new_send_sync(value: impl Display + Debug + Send + Sync + 'static) -> Self {
        unsafe { Self::new_error(value) }
    }
}

impl<T: private::Sealed + ?Sized> AnyError<T> {
    /// Checks whether creating a new `AnyError` with a `U` requires an allocation.
    ///
    /// As an optimization, `AnyError` supports construction without allocation under some
    /// circumstances. One of those cases is when `U` fits into a `usize`, and is aligned
    /// at most to a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fimo_std::error::AnyError;
    ///
    /// assert!(!<AnyError>::will_alloc::<u32>());
    /// assert!(!<AnyError>::will_alloc::<f64>());
    ///
    /// assert!(<AnyError>::will_alloc::<[u8; 256]>());
    /// ```
    pub const fn will_alloc<U: 'static>() -> bool {
        size_of::<U>() > size_of::<usize>() || align_of::<U>() > align_of::<usize>()
    }

    /// Creates an [`AnyError`] from a string.
    pub fn from_string(error: &'static CStr) -> Self
    where
        AssertSharable<&'static CStr>: Unsize<T>,
    {
        error.into_error()
    }

    /// Creates an [`AnyError`] from an error code.
    ///
    /// In case of an invalid error code, this returns `Err`.
    pub fn from_error_code(error: ErrorCode) -> Result<Self, ErrorCode>
    where
        AssertSharable<ErrorCode>: Unsize<T>,
    {
        match error {
            ErrorCode::Ok => Err(error),
            _ => unsafe { Ok(Self::from_error_code_unchecked(error)) },
        }
    }

    /// Creates an [`AnyError`] from an error code.
    ///
    /// # Safety
    ///
    /// `error` must be a valid error code and not [`Ok`](ErrorCode::Ok).
    pub unsafe fn from_error_code_unchecked(error: ErrorCode) -> Self
    where
        AssertSharable<ErrorCode>: Unsize<T>,
    {
        error.into_error()
    }

    /// Creates an [`AnyError`] from a system error code.
    pub fn from_system_error(error: SystemErrorCode) -> Self
    where
        AssertSharable<SystemErrorCode>: Unsize<T>,
    {
        error.into_error()
    }

    /// Returns a string representing the error.
    pub fn name(&self) -> ErrorString {
        unsafe {
            let f = self.vtable.error_name;
            f(self.handle.map(OpaqueHandle::coerce))
        }
    }

    /// Returns a string describing the error.
    pub fn description(&self) -> ErrorString {
        unsafe {
            let f = self.vtable.error_description;
            f(self.handle.map(OpaqueHandle::coerce))
        }
    }

    #[doc(hidden)]
    pub fn into_error(self) -> bindings::FimoResult {
        let this = ManuallyDrop::new(self);
        unsafe { std::mem::transmute::<ManuallyDrop<Self>, bindings::FimoResult>(this) }
    }
}

impl<T: private::Sealed + ?Sized> Debug for AnyError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        f.debug_tuple("AnyError").field(&&*name).finish()
    }
}

impl<T: private::Sealed + ?Sized> Display for AnyError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let description = self.description();
        write!(f, "{}", description.to_string_lossy())
    }
}

impl<T: private::Sealed + ?Sized> Drop for AnyError<T> {
    fn drop(&mut self) {
        unsafe {
            let Some(f) = self.vtable.drop else { return };
            f(self.handle.map(|x| x.coerce()));
        }
    }
}

impl From<AnyError<dyn Send + Share>> for AnyError {
    fn from(value: AnyError<dyn Send + Share>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send + Share>, Self>(value) }
    }
}

impl From<AnyError<dyn Sync + Share>> for AnyError {
    fn from(value: AnyError<dyn Sync + Share>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Sync + Share>, Self>(value) }
    }
}

impl From<AnyError<dyn Send + Sync + Share>> for AnyError {
    fn from(value: AnyError<dyn Send + Sync + Share>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send + Sync + Share>, Self>(value) }
    }
}

impl From<AnyError<dyn Send + Sync + Share>> for AnyError<dyn Send + Share> {
    fn from(value: AnyError<dyn Send + Sync + Share>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send + Sync + Share>, Self>(value) }
    }
}

impl From<AnyError<dyn Send + Sync + Share>> for AnyError<dyn Sync + Share> {
    fn from(value: AnyError<dyn Send + Sync + Share>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send + Sync + Share>, Self>(value) }
    }
}

pub(crate) mod private {
    use std::marker::Unsize;

    use crate::modules::symbols::Share;

    pub trait Sealed: Unsize<dyn Share> + Share + 'static {}

    impl Sealed for dyn Share {}
    impl Sealed for dyn Send + Share {}
    impl Sealed for dyn Sync + Share {}
    impl Sealed for dyn Send + Sync + Share {}
}

trait ErrorSpec<E: private::Sealed + ?Sized>: Sized + 'static
where
    AssertSharable<Self>: Unsize<E>,
{
    fn into_error(self) -> AnyError<E>;
}

impl<E: private::Sealed + ?Sized> ErrorSpec<E> for &'static CStr
where
    AssertSharable<Self>: Unsize<E>,
{
    fn into_error(self) -> AnyError<E> {
        let vtable = unsafe { &*(&raw const FIMO_IMPL_RESULT_STATIC_STRING_VTABLE).cast() };
        AnyError {
            handle: unsafe { ErrorHandle::new(self.as_ptr().cast_mut()) },
            vtable,
            _private: PhantomData,
        }
    }
}

impl<E: private::Sealed + ?Sized> ErrorSpec<E> for ErrorCode
where
    AssertSharable<Self>: Unsize<E>,
{
    fn into_error(self) -> AnyError<E> {
        let vtable = unsafe { &*(&raw const FIMO_IMPL_RESULT_ERROR_CODE_VTABLE).cast() };
        AnyError {
            handle: unsafe {
                ErrorHandle::new::<()>(std::ptr::without_provenance_mut(self as usize))
            },
            vtable,
            _private: PhantomData,
        }
    }
}

impl<E: private::Sealed + ?Sized> ErrorSpec<E> for SystemErrorCode
where
    AssertSharable<Self>: Unsize<E>,
{
    fn into_error(self) -> AnyError<E> {
        let vtable = unsafe { &*(&raw const FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE).cast() };
        AnyError {
            handle: unsafe {
                ErrorHandle::new::<()>(std::ptr::without_provenance_mut(self.0 as usize))
            },
            vtable,
            _private: PhantomData,
        }
    }
}

trait NewErrorSpec<T>
where
    T: 'static,
{
    unsafe fn new_error(value: T) -> Self;
}

impl<T, E> NewErrorSpec<T> for AnyError<E>
where
    T: Debug + Display + 'static,
    E: private::Sealed + ?Sized,
{
    default unsafe fn new_error(value: T) -> Self {
        fn new_string(string: String) -> ErrorString {
            extern "C" fn drop(handle: ErrorStringHandle) {
                unsafe {
                    _ = CString::from_raw(handle.as_ptr());
                }
            }

            match CString::new(string) {
                Ok(str) => ErrorString {
                    handle: unsafe { ErrorStringHandle::new_unchecked(str.into_raw()) },
                    drop: Some(drop),
                },
                Err(_) => <AnyError>::from_string(c"CString::new failed").description(),
            }
        }

        extern "C" fn drop_inline<T>(mut handle: Option<ErrorHandle<dyn Share>>) {
            let value_ptr = (&raw mut handle).cast();
            unsafe { std::ptr::drop_in_place::<T>(value_ptr) };
        }
        extern "C" fn debug_inline<T: Debug>(
            handle: Option<ErrorHandle<dyn Share>>,
        ) -> ErrorString {
            let value_ptr: *const T = (&raw const handle).cast::<T>();
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{:?}", *value))
            }
        }
        extern "C" fn display_inline<T: Display>(
            handle: Option<ErrorHandle<dyn Share>>,
        ) -> ErrorString {
            let value_ptr: *const T = (&raw const handle).cast::<T>();
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{}", *value))
            }
        }

        // If the size and alignments match we can store the value inline.
        if size_of::<T>() <= size_of::<*mut std::ffi::c_void>()
            && align_of::<T>() <= align_of::<*mut std::ffi::c_void>()
        {
            let vtable = &const {
                let vtable = VTable {
                    drop: if std::mem::needs_drop::<T>() {
                        Some(drop_inline::<T>)
                    } else {
                        None
                    },
                    error_name: debug_inline::<T>,
                    error_description: display_inline::<T>,
                };
                unsafe { AssertSharable::new(vtable) }
            };

            // Pack a `T` into a `*mut ()`.
            let handle = unsafe {
                let mut data: *mut core::ffi::c_void = std::ptr::null_mut();
                (&raw mut data).cast::<T>().write(value);
                ErrorHandle::new(data)
            };
            return Self {
                handle,
                vtable,
                _private: PhantomData,
            };
        }

        extern "C" fn drop_boxed<T>(handle: Option<ErrorHandle<dyn Share>>) {
            let value_ptr = handle.map_or_else(std::ptr::null_mut, |h| h.as_ptr());
            unsafe { drop(Box::<T>::from_raw(value_ptr)) };
        }
        extern "C" fn debug_boxed<T: Debug>(handle: Option<ErrorHandle<dyn Share>>) -> ErrorString {
            let value_ptr = handle.map_or_else(std::ptr::null, |h| h.as_ptr::<T>().cast_const());
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{:?}", *value))
            }
        }
        extern "C" fn display_boxed<T: Display>(
            handle: Option<ErrorHandle<dyn Share>>,
        ) -> ErrorString {
            let value_ptr = handle.map_or_else(std::ptr::null, |h| h.as_ptr::<T>().cast_const());
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{}", *value))
            }
        }

        // Fall back to boxing the error.
        let vtable = &const {
            let vtable = VTable {
                drop: Some(drop_boxed::<T>),
                error_name: debug_boxed::<T>,
                error_description: display_boxed::<T>,
            };
            unsafe { AssertSharable::new(vtable) }
        };

        let handle = unsafe { ErrorHandle::new(Box::into_raw(Box::new(value))) };
        Self {
            handle,
            vtable,
            _private: PhantomData,
        }
    }
}

#[test]
fn no_alloc_int() {
    let x = 123456u32;
    let err = <AnyError>::new(x);
    let err_inner = unsafe { std::mem::transmute_copy(&err.handle) };
    assert_eq!(x, err_inner);
}

#[test]
fn no_alloc_enum() {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Foo {
        _A(u8),
        _B(u16),
        C(u32),
    }
    const {
        assert!(size_of::<Foo>() <= size_of::<usize>());
        assert!(align_of::<Foo>() <= align_of::<usize>());
    }

    impl Display for Foo {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(self, f)
        }
    }

    let x = Foo::C(123456789);
    let err = <AnyError>::new(x);
    let err_inner = unsafe { std::mem::transmute_copy(&err.handle) };
    assert_eq!(x, err_inner);
}

handle!(pub handle ErrorStringHandle: Send + Sync);

/// A string returned from an [`AnyError`].
#[repr(C)]
pub struct ErrorString {
    pub handle: ErrorStringHandle,
    pub drop: Option<unsafe extern "C" fn(handle: ErrorStringHandle)>,
}

impl Deref for ErrorString {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        unsafe { CStr::from_ptr(self.handle.as_ptr()) }
    }
}

impl Debug for ErrorString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &**self)
    }
}

impl Display for ErrorString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_lossy())
    }
}

impl Drop for ErrorString {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            unsafe { drop(self.handle) };
        }
    }
}

/// A [`Result`] with an [`AnyError`] error type.
pub type Result<T = (), E = AnyError> = core::result::Result<T, E>;

/// Converts a [`FimoResult`](bindings::FimoResult) to an error if it's greater than zero, and
/// `Ok(())` otherwise.
///
/// # Safety
///
/// - `error` must be properly initialized.
pub unsafe fn to_result(error: bindings::FimoResult) -> Result {
    if !error.data.is_null() {
        Err(unsafe { std::mem::transmute::<bindings::FimoResult, AnyError>(error) })
    } else {
        Ok(())
    }
}

/// Constructs a [`Result`] by calling a closure that may indicate an error.
///
/// This is usefull when calling C functions that expect a writable error pointer as one of their
/// arguments.
///
/// ```ignore
/// # use fimo_std::error:to_result_indirect;
/// # use fimo_std::bindings;
/// extern "C" {
///     fn my_func(error: *mut bindings::FimoErrorCode);
/// }
///
/// unsafe { to_result_indirect(|error| my_func(error)) };
/// ```
///
/// # Safety
///
/// - `error` must be properly initialized by `f`.
pub unsafe fn to_result_indirect<T>(f: impl FnOnce(&mut bindings::FimoResult) -> T) -> Result<T> {
    let mut error = <AnyError>::FFI_OK_RESULT;
    let result = f(&mut error);
    if !error.vtable.is_null() {
        Err(unsafe { std::mem::transmute::<bindings::FimoResult, AnyError>(error) })
    } else {
        Ok(result)
    }
}

/// Constructs a [`Result`] by calling a closure that may indicate an error.
///
/// Like [`to_result_indirect`], this is usefull when calling C functions that expect a
/// writable error pointer as one of their arguments, with the difference that the data
/// is initialized in place.
///
/// ```ignore
/// # use fimo_std::error:to_result_indirect;
/// # use fimo_std::bindings;
/// extern "C" {
///     fn my_func(error: *mut bindings::FimoResult, data: *mut u32);
/// }
///
/// unsafe {
///     to_result_indirect_in_place(|error, data| my_func(error, data.as_mut_ptr()))
/// };
/// ```
///
/// # Safety
///
/// - `f` must initialize the data or write an error in the first parameter.
pub unsafe fn to_result_indirect_in_place<T>(
    f: impl FnOnce(&mut bindings::FimoResult, &mut MaybeUninit<T>),
) -> Result<T> {
    let mut data = MaybeUninit::<T>::uninit();
    let mut error = <AnyError>::FFI_OK_RESULT;

    f(&mut error, &mut data);
    if !error.vtable.is_null() {
        Err(unsafe { std::mem::transmute::<bindings::FimoResult, AnyError>(error) })
    } else {
        // Safety: By the contract of this function the data
        // must have been initialized, if the function does
        // not return an error.
        let result = unsafe { data.assume_init() };
        Ok(result)
    }
}
