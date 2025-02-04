//! Fimo error codes.

use crate::{
    bindings,
    utils::{FFITransferable, OpaqueHandle, VTablePtr},
    handle,
};
use std::{
    ffi::{CStr, CString},
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
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
pub struct VTable<T: ?Sized> {
    pub drop: Option<unsafe extern "C" fn(handle: Option<ErrorHandle<T>>)>,
    pub error_name: unsafe extern "C" fn(handle: Option<ErrorHandle<T>>) -> ErrorString,
    pub error_description: unsafe extern "C" fn(handle: Option<ErrorHandle<T>>) -> ErrorString,
}

unsafe extern "C" {
    static FIMO_IMPL_RESULT_ERROR_CODE_VTABLE: VTable<*mut ()>;
    static FIMO_IMPL_RESULT_STATIC_STRING_VTABLE: VTable<*mut ()>;
    static FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE: VTable<*mut ()>;

    static FIMO_IMPL_RESULT_OK_NAME: ErrorString;
    static FIMO_IMPL_RESULT_OK_DESCRIPTION: ErrorString;
}

/// FFI equivalent of a `Result<(), AnyError>`.
#[repr(C)]
pub struct AnyResult<T: ?Sized + 'static = *mut ()> {
    pub handle: Option<ErrorHandle<T>>,
    pub vtable: Option<VTablePtr<'static, VTable<T>>>,
}

impl<T: ?Sized + 'static> AnyResult<T> {
    /// An `AnyResult` equivalent of an Ok(()).
    pub const OK: Self = Self {
        handle: None,
        vtable: None,
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

        Self { handle, vtable }
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
            Some(vtable) => unsafe { (vtable.error_name)(self.handle) },
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
            Some(vtable) => unsafe { (vtable.error_description)(self.handle) },
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

impl<T: ?Sized> Debug for AnyResult<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        f.debug_tuple("AnyResult").field(&&*name).finish()
    }
}

impl<T: ?Sized> Display for AnyResult<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let description = self.description();
        write!(f, "{}", description.to_string_lossy())
    }
}

impl<T: ?Sized> Default for AnyResult<T> {
    fn default() -> Self {
        Self::OK
    }
}

impl<T: ?Sized> PartialEq for AnyResult<T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.vtable == other.vtable
    }
}

impl<T: ?Sized> Eq for AnyResult<T> {}

impl<T: ?Sized> Hash for AnyResult<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
        self.vtable.hash(state);
    }
}

impl<T: ?Sized> From<AnyResult<T>> for Result<(), AnyError<T>> {
    fn from(value: AnyResult<T>) -> Self {
        value.into_result()
    }
}

impl<T: ?Sized> From<AnyError<T>> for AnyResult<T> {
    fn from(value: AnyError<T>) -> Self {
        Self::new_err(value)
    }
}

impl<T: ?Sized> From<Result<(), AnyError<T>>> for AnyResult<T> {
    fn from(value: Result<(), AnyError<T>>) -> Self {
        match value {
            Ok(_) => Default::default(),
            Err(e) => e.into(),
        }
    }
}

impl<T: ?Sized> Drop for AnyResult<T> {
    fn drop(&mut self) {
        let Some(vtable) = self.vtable else { return };
        unsafe {
            let Some(drop) = vtable.drop else { return };
            drop(self.handle);
        }
    }
}

/// Generic error value.
#[repr(C)]
#[derive(PartialEq, Eq, Hash)]
pub struct AnyError<T: ?Sized + 'static = *mut ()> {
    pub handle: Option<ErrorHandle<T>>,
    pub vtable: VTablePtr<'static, VTable<T>>,
}

mod __private {
    use super::*;
    use paste::paste;

    macro_rules! new_error {
        ($code:ident, $($doc:literal),+) => {
            paste! {
                impl<T: ?Sized> AnyError<T> {
                    $(
                        #[doc = $doc]
                    )+
                    pub const $code: Self = unsafe {
                        Self::from_error_code_unchecked(ErrorCode::$code)
                    };
                }
            }
        };
    }

    impl<T: ?Sized> AnyError<T> {
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

impl AnyError<dyn Send> {
    /// Creates an [`AnyError`] from an arbitrary value that can be formatted.
    pub fn new_send(value: impl Display + Debug + Send + 'static) -> Self {
        unsafe { Self::new_error(value) }
    }
}

impl AnyError<dyn Sync> {
    /// Creates an [`AnyError`] from an arbitrary value that can be formatted.
    pub fn new_sync(value: impl Display + Debug + Sync + 'static) -> Self {
        unsafe { Self::new_error(value) }
    }
}

impl AnyError<dyn Send + Sync> {
    /// Creates an [`AnyError`] from an arbitrary value that can be formatted.
    pub fn new_send_sync(value: impl Display + Debug + Send + Sync + 'static) -> Self {
        unsafe { Self::new_error(value) }
    }
}

impl<T: ?Sized> AnyError<T> {
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
    pub const fn from_string(error: &'static CStr) -> Self {
        Self {
            handle: ErrorHandle::new(error.as_ptr().cast_mut()),
            vtable: <&'static CStr as VTableHelper<T>>::VTABLE,
        }
    }

    /// Creates an [`AnyError`] from an error code.
    ///
    /// In case of an invalid error code, this returns `Err`.
    pub const fn from_error_code(error: ErrorCode) -> Result<Self, ErrorCode> {
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
    pub const unsafe fn from_error_code_unchecked(error: ErrorCode) -> Self {
        Self {
            handle: ErrorHandle::new::<()>(std::ptr::without_provenance_mut(error as usize)),
            vtable: <ErrorCode as VTableHelper<T>>::VTABLE,
        }
    }

    /// Creates an [`AnyError`] from a system error code.
    pub const fn from_system_error(error: SystemErrorCode) -> Self {
        Self {
            handle: ErrorHandle::new::<()>(std::ptr::without_provenance_mut(error.0 as usize)),
            vtable: <SystemErrorCode as VTableHelper<T>>::VTABLE,
        }
    }

    /// Returns a string representing the error.
    pub fn name(&self) -> ErrorString {
        unsafe {
            let f = self.vtable.error_name;
            f(self.handle)
        }
    }

    /// Returns a string describing the error.
    pub fn description(&self) -> ErrorString {
        unsafe {
            let f = self.vtable.error_description;
            f(self.handle)
        }
    }

    #[doc(hidden)]
    pub fn into_error(self) -> bindings::FimoResult {
        let this = ManuallyDrop::new(self);
        unsafe { std::mem::transmute::<ManuallyDrop<Self>, bindings::FimoResult>(this) }
    }
}

impl<T: ?Sized> Debug for AnyError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        f.debug_tuple("AnyError").field(&&*name).finish()
    }
}

impl<T: ?Sized> Display for AnyError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let description = self.description();
        write!(f, "{}", description.to_string_lossy())
    }
}

impl<T: ?Sized> Drop for AnyError<T> {
    fn drop(&mut self) {
        unsafe {
            let Some(f) = self.vtable.drop else { return };
            f(self.handle);
        }
    }
}

impl From<AnyError<dyn Send>> for AnyError {
    fn from(value: AnyError<dyn Send>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send>, Self>(value) }
    }
}

impl From<AnyError<dyn Sync>> for AnyError {
    fn from(value: AnyError<dyn Sync>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Sync>, Self>(value) }
    }
}

impl From<AnyError<dyn Send + Sync>> for AnyError {
    fn from(value: AnyError<dyn Send + Sync>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send + Sync>, Self>(value) }
    }
}

impl From<AnyError<dyn Send + Sync>> for AnyError<dyn Send> {
    fn from(value: AnyError<dyn Send + Sync>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send + Sync>, AnyError<dyn Send>>(value) }
    }
}

impl From<AnyError<dyn Send + Sync>> for AnyError<dyn Sync> {
    fn from(value: AnyError<dyn Send + Sync>) -> Self {
        unsafe { std::mem::transmute::<AnyError<dyn Send + Sync>, AnyError<dyn Sync>>(value) }
    }
}

trait VTableHelper<E: ?Sized + 'static> {
    const VTABLE: VTablePtr<'static, VTable<E>>;
}

impl<E: ?Sized + 'static> VTableHelper<E> for &'static CStr {
    const VTABLE: VTablePtr<'static, VTable<E>> = const {
        unsafe {
            VTablePtr::new_unchecked((&raw const FIMO_IMPL_RESULT_STATIC_STRING_VTABLE).cast())
        }
    };
}

impl<E: ?Sized + 'static> VTableHelper<E> for ErrorCode {
    const VTABLE: VTablePtr<'static, VTable<E>> = const {
        unsafe { VTablePtr::new_unchecked((&raw const FIMO_IMPL_RESULT_ERROR_CODE_VTABLE).cast()) }
    };
}

impl<E: ?Sized + 'static> VTableHelper<E> for SystemErrorCode {
    const VTABLE: VTablePtr<'static, VTable<E>> = const {
        unsafe {
            VTablePtr::new_unchecked((&raw const FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE).cast())
        }
    };
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
    E: ?Sized + 'static,
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

        extern "C" fn drop_inline<T, E: ?Sized>(mut handle: Option<ErrorHandle<E>>) {
            let value_ptr = (&raw mut handle).cast();
            unsafe { std::ptr::drop_in_place::<T>(value_ptr) };
        }
        extern "C" fn debug_inline<T: Debug, E: ?Sized>(
            handle: Option<ErrorHandle<E>>,
        ) -> ErrorString {
            let value_ptr: *const T = (&raw const handle).cast::<T>();
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{:?}", *value))
            }
        }
        extern "C" fn display_inline<T: Display, E: ?Sized>(
            handle: Option<ErrorHandle<E>>,
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
                VTable {
                    drop: if std::mem::needs_drop::<T>() {
                        Some(drop_inline::<T, E>)
                    } else {
                        None
                    },
                    error_name: debug_inline::<T, E>,
                    error_description: display_inline::<T, E>,
                }
            };

            // Pack a `T` into a `*mut ()`.
            let handle = unsafe {
                let mut data: *mut core::ffi::c_void = std::ptr::null_mut();
                (&raw mut data).cast::<T>().write(value);
                ErrorHandle::new(data)
            };
            let vtable = VTablePtr::new(vtable);
            return Self { handle, vtable };
        }

        extern "C" fn drop_boxed<T, E: ?Sized>(handle: Option<ErrorHandle<E>>) {
            let value_ptr = handle.map_or_else(std::ptr::null_mut, |h| h.as_ptr());
            unsafe { drop(Box::<T>::from_raw(value_ptr)) };
        }
        extern "C" fn debug_boxed<T: Debug, E: ?Sized>(
            handle: Option<ErrorHandle<E>>,
        ) -> ErrorString {
            let value_ptr = handle.map_or_else(std::ptr::null, |h| h.as_ptr::<T>().cast_const());
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{:?}", *value))
            }
        }
        extern "C" fn display_boxed<T: Display, E: ?Sized>(
            handle: Option<ErrorHandle<E>>,
        ) -> ErrorString {
            let value_ptr = handle.map_or_else(std::ptr::null, |h| h.as_ptr::<T>().cast_const());
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{}", *value))
            }
        }

        // Fall back to boxing the error.
        let vtable = &const {
            VTable {
                drop: Some(drop_boxed::<T, E>),
                error_name: debug_boxed::<T, E>,
                error_description: display_boxed::<T, E>,
            }
        };

        let handle = ErrorHandle::new(Box::into_raw(Box::new(value)));
        let vtable = VTablePtr::new(vtable);
        Self { handle, vtable }
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

impl FFITransferable<bindings::FimoResultString> for ErrorString {
    fn into_ffi(self) -> bindings::FimoResultString {
        unsafe { std::mem::transmute::<Self, bindings::FimoResultString>(self) }
    }

    unsafe fn from_ffi(ffi: bindings::FimoResultString) -> Self {
        unsafe { std::mem::transmute::<bindings::FimoResultString, Self>(ffi) }
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
