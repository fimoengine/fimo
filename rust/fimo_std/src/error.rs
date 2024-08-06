//! Fimo error codes.

use crate::{bindings, ffi::FFITransferable};
use std::{
    ffi::{CStr, CString},
    fmt,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::Deref,
};

/// Generic error code.
#[derive(PartialEq, Eq, Hash)]
pub struct Error<T: ?Sized = *const ()>(bindings::FimoResult, PhantomData<T>);

mod __private {
    use paste::paste;

    macro_rules! new_error {
        ($code:ident, $err:ident, $($doc:literal),+) => {
            paste! {
                impl<T: ?Sized> super::Error<T> {
                    $(
                        #[doc = $doc]
                    )+
                    pub const $err: Self = Self(
                        crate::bindings::FimoResult {
                            data: std::ptr::without_provenance_mut(crate::bindings::FimoErrorCode::[<FIMO_ $code>].0 as usize),
                            // Safety: Is guaranteed to be valid.
                            vtable: unsafe{ &crate::bindings::FIMO_IMPL_RESULT_ERROR_CODE_VTABLE }
                        },
                        std::marker::PhantomData,
                    );
                }
            }
        };
    }

    impl<T: ?Sized> super::Error<T> {
        pub(super) const FFI_OK_RESULT: crate::bindings::FimoResult = crate::bindings::FimoResult {
            data: std::ptr::null_mut(),
            vtable: std::ptr::null_mut(),
        };
    }

    new_error!(ERROR_CODE_2BIG, E2BIG, "Argument list too long");
    new_error!(ERROR_CODE_ACCES, EACCES, "Permission denied");
    new_error!(ERROR_CODE_ADDRINUSE, EADDRINUSE, "Address already in use");
    new_error!(
        ERROR_CODE_ADDRNOTAVAIL,
        EADDRNOTAVAIL,
        "Address not available"
    );
    new_error!(
        ERROR_CODE_AFNOSUPPORT,
        EAFNOSUPPORT,
        "Address family not supported"
    );
    new_error!(ERROR_CODE_AGAIN, EAGAIN, "Resource temporarily unavailable");
    new_error!(
        ERROR_CODE_ALREADY,
        EALREADY,
        "Connection already in progress"
    );
    new_error!(ERROR_CODE_BADE, EBADE, "Invalid exchange");
    new_error!(ERROR_CODE_BADF, EBADF, "Bad file descriptor");
    new_error!(ERROR_CODE_BADFD, EBADFD, "File descriptor in bad state");
    new_error!(ERROR_CODE_BADMSG, EBADMSG, "Bad message");
    new_error!(ERROR_CODE_BADR, EBADR, "Invalid request descriptor");
    new_error!(ERROR_CODE_BADRQC, EBADRQC, "Invalid request code");
    new_error!(ERROR_CODE_BADSLT, EBADSLT, "Invalid slot");
    new_error!(ERROR_CODE_BUSY, EBUSY, "Device or resource busy");
    new_error!(ERROR_CODE_CANCELED, ECANCELED, "Operation canceled");
    new_error!(ERROR_CODE_CHILD, ECHILD, "No child processes");
    new_error!(ERROR_CODE_CHRNG, ECHRNG, "Channel number out of range");
    new_error!(ERROR_CODE_COMM, ECOMM, "Communication error on send");
    new_error!(ERROR_CODE_CONNABORTED, ECONNABORTED, "Connection aborted");
    new_error!(ERROR_CODE_CONNREFUSED, ECONNREFUSED, "Connection refused");
    new_error!(ERROR_CODE_CONNRESET, ECONNRESET, "Connection reset");
    new_error!(ERROR_CODE_DEADLK, EDEADLK, "Resource deadlock avoided");
    new_error!(
        ERROR_CODE_DEADLOCK,
        EDEADLOCK,
        "File locking deadlock error (or Resource deadlock avoided)"
    );
    new_error!(
        ERROR_CODE_DESTADDRREQ,
        EDESTADDRREQ,
        "Destination address required"
    );
    new_error!(
        ERROR_CODE_DOM,
        EDOM,
        "Mathematics argument out of domain of function"
    );
    new_error!(ERROR_CODE_DQUOT, EDQUOT, "Disk quota exceeded");
    new_error!(ERROR_CODE_EXIST, EEXIST, "File exists");
    new_error!(ERROR_CODE_FAULT, EFAULT, "Bad address");
    new_error!(ERROR_CODE_FBIG, EFBIG, "File too large");
    new_error!(ERROR_CODE_HOSTDOWN, EHOSTDOWN, "Host is down");
    new_error!(ERROR_CODE_HOSTUNREACH, EHOSTUNREACH, "Host is unreachable");
    new_error!(
        ERROR_CODE_HWPOISON,
        EHWPOISON,
        "Memory page has hardware error"
    );
    new_error!(ERROR_CODE_IDRM, EIDRM, "Identifier removed");
    new_error!(
        ERROR_CODE_ILSEQ,
        EILSEQ,
        "Invalid or incomplete multibyte or wide character"
    );
    new_error!(ERROR_CODE_INPROGRESS, EINPROGRESS, "Operation in progress");
    new_error!(ERROR_CODE_INTR, EINTR, "Interrupted function call");
    new_error!(ERROR_CODE_INVAL, EINVAL, "Invalid argument");
    new_error!(ERROR_CODE_IO, EIO, "Input/output error");
    new_error!(ERROR_CODE_ISCONN, EISCONN, "Socket is connected");
    new_error!(ERROR_CODE_ISDIR, EISDIR, "Is a directory");
    new_error!(ERROR_CODE_ISNAM, EISNAM, "Is a named type file");
    new_error!(ERROR_CODE_KEYEXPIRED, EKEYEXPIRED, "Key has expired");
    new_error!(
        ERROR_CODE_KEYREJECTED,
        EKEYREJECTED,
        "Key was rejected by service"
    );
    new_error!(ERROR_CODE_KEYREVOKED, EKEYREVOKED, "Key has been revoked");
    new_error!(ERROR_CODE_L2HLT, EL2HLT, "Level 2 halted");
    new_error!(ERROR_CODE_L2NSYNC, EL2NSYNC, "Level 2 not synchronized");
    new_error!(ERROR_CODE_L3HLT, EL3HLT, "Level 3 halted");
    new_error!(ERROR_CODE_L3RST, EL3RST, "Level 3 reset");
    new_error!(
        ERROR_CODE_LIBACC,
        ELIBACC,
        "Cannot access a needed shared library"
    );
    new_error!(
        ERROR_CODE_LIBBAD,
        ELIBBAD,
        "Accessing a corrupted shared library"
    );
    new_error!(
        ERROR_CODE_LIBMAX,
        ELIBMAX,
        "Attempting to link in too many shared libraries"
    );
    new_error!(
        ERROR_CODE_LIBSCN,
        ELIBSCN,
        ".lib section in a.out corrupted"
    );
    new_error!(
        ERROR_CODE_LIBEXEC,
        ELIBEXEC,
        "Cannot exec a shared library directly"
    );
    new_error!(ERROR_CODE_LNRNG, ELNRNG, "Link number out of range");
    new_error!(ERROR_CODE_LOOP, ELOOP, "Too many levels of symbolic links");
    new_error!(ERROR_CODE_MEDIUMTYPE, EMEDIUMTYPE, "Wrong medium type");
    new_error!(ERROR_CODE_MFILE, EMFILE, "Too many open files");
    new_error!(ERROR_CODE_MLINK, EMLINK, "Too many links");
    new_error!(ERROR_CODE_MSGSIZE, EMSGSIZE, "Message too long");
    new_error!(ERROR_CODE_MULTIHOP, EMULTIHOP, "Multihop attempted");
    new_error!(ERROR_CODE_NAMETOOLONG, ENAMETOOLONG, "Filename too long");
    new_error!(ERROR_CODE_NETDOWN, ENETDOWN, "Network is down");
    new_error!(
        ERROR_CODE_NETRESET,
        ENETRESET,
        "Connection aborted by network"
    );
    new_error!(ERROR_CODE_NETUNREACH, ENETUNREACH, "Network unreachable");
    new_error!(ERROR_CODE_NFILE, ENFILE, "Too many open files in system");
    new_error!(ERROR_CODE_NOANO, ENOANO, "No anode");
    new_error!(ERROR_CODE_NOBUFS, ENOBUFS, "No buffer space available");
    new_error!(
        ERROR_CODE_NODATA,
        ENODATA,
        "The named attribute does not exist, or the process has no access to this attribute"
    );
    new_error!(ERROR_CODE_NODEV, ENODEV, "No such device");
    new_error!(ERROR_CODE_NOENT, ENOENT, "No such file or directory");
    new_error!(ERROR_CODE_NOEXEC, ENOEXEC, "Exec format error");
    new_error!(ERROR_CODE_NOKEY, ENOKEY, "Required key not available");
    new_error!(ERROR_CODE_NOLCK, ENOLCK, "No locks available");
    new_error!(ERROR_CODE_NOLINK, ENOLINK, "Link has been severed");
    new_error!(ERROR_CODE_NOMEDIUM, ENOMEDIUM, "No medium found");
    new_error!(
        ERROR_CODE_NOMEM,
        ENOMEM,
        "Not enough space/cannot allocate memory"
    );
    new_error!(ERROR_CODE_NOMSG, ENOMSG, "No message of the desired type");
    new_error!(ERROR_CODE_NONET, ENONET, "Machine is not on the network");
    new_error!(ERROR_CODE_NOPKG, ENOPKG, "Package not installed");
    new_error!(ERROR_CODE_NOPROTOOPT, ENOPROTOOPT, "Protocol not available");
    new_error!(ERROR_CODE_NOSPC, ENOSPC, "No space left on device");
    new_error!(ERROR_CODE_NOSR, ENOSR, "No STREAM resources");
    new_error!(ERROR_CODE_NOSTR, ENOSTR, "Not a STREAM");
    new_error!(ERROR_CODE_NOSYS, ENOSYS, "Function not implemented");
    new_error!(ERROR_CODE_NOTBLK, ENOTBLK, "Block device required");
    new_error!(ERROR_CODE_NOTCONN, ENOTCONN, "The socket is not connected");
    new_error!(ERROR_CODE_NOTDIR, ENOTDIR, "Not a directory");
    new_error!(ERROR_CODE_NOTEMPTY, ENOTEMPTY, "Directory not empty");
    new_error!(
        ERROR_CODE_NOTRECOVERABLE,
        ENOTRECOVERABLE,
        "State not recoverable"
    );
    new_error!(ERROR_CODE_NOTSOCK, ENOTSOCK, "Not a socket");
    new_error!(ERROR_CODE_NOTSUP, ENOTSUP, "Operation not supported");
    new_error!(
        ERROR_CODE_NOTTY,
        ENOTTY,
        "Inappropriate I/O control operation"
    );
    new_error!(ERROR_CODE_NOTUNIQ, ENOTUNIQ, "Name not unique on network");
    new_error!(ERROR_CODE_NXIO, ENXIO, "No such device or address");
    new_error!(
        ERROR_CODE_OPNOTSUPP,
        EOPNOTSUPP,
        "Operation not supported on socket"
    );
    new_error!(
        ERROR_CODE_OVERFLOW,
        EOVERFLOW,
        "Value too large to be stored in data type"
    );
    new_error!(ERROR_CODE_OWNERDEAD, EOWNERDEAD, "Owner died");
    new_error!(ERROR_CODE_PERM, EPERM, "Operation not permitted");
    new_error!(
        ERROR_CODE_PFNOSUPPORT,
        EPFNOSUPPORT,
        "Protocol family not supported"
    );
    new_error!(ERROR_CODE_PIPE, EPIPE, "Broken pipe");
    new_error!(ERROR_CODE_PROTO, EPROTO, "Protocol error");
    new_error!(
        ERROR_CODE_PROTONOSUPPORT,
        EPROTONOSUPPORT,
        "Protocol not supported"
    );
    new_error!(
        ERROR_CODE_PROTOTYPE,
        EPROTOTYPE,
        "Protocol wrong type for socket"
    );
    new_error!(ERROR_CODE_RANGE, ERANGE, "Result too large");
    new_error!(ERROR_CODE_REMCHG, EREMCHG, "Remote address changed");
    new_error!(ERROR_CODE_REMOTE, EREMOTE, "Object is remote");
    new_error!(ERROR_CODE_REMOTEIO, EREMOTEIO, "Remote I/O error");
    new_error!(
        ERROR_CODE_RESTART,
        ERESTART,
        "Interrupted system call should be restarted"
    );
    new_error!(
        ERROR_CODE_RFKILL,
        ERFKILL,
        "Operation not possible due to RF-kill"
    );
    new_error!(ERROR_CODE_ROFS, EROFS, "Read-only filesystem");
    new_error!(
        ERROR_CODE_SHUTDOWN,
        ESHUTDOWN,
        "Cannot send after transport endpoint shutdown"
    );
    new_error!(ERROR_CODE_SPIPE, ESPIPE, "Invalid seek");
    new_error!(
        ERROR_CODE_SOCKTNOSUPPORT,
        ESOCKTNOSUPPORT,
        "Socket type not supported"
    );
    new_error!(ERROR_CODE_SRCH, ESRCH, "No such process");
    new_error!(ERROR_CODE_STALE, ESTALE, "Stale file handle");
    new_error!(ERROR_CODE_STRPIPE, ESTRPIPE, "Streams pipe error");
    new_error!(ERROR_CODE_TIME, ETIME, "Timer expired");
    new_error!(
        ERROR_CODE_TOOMANYREFS,
        ETOOMANYREFS,
        "Too many references: cannot splice"
    );
    new_error!(ERROR_CODE_TXTBSY, ETXTBSY, "Text file busy");
    new_error!(ERROR_CODE_UCLEAN, EUCLEAN, "Structure needs cleaning");
    new_error!(ERROR_CODE_UNATCH, EUNATCH, "Protocol driver not attached");
    new_error!(ERROR_CODE_USERS, EUSERS, "Too many users");
    new_error!(ERROR_CODE_WOULDBLOCK, EWOULDBLOCK, "Operation would block");
    new_error!(ERROR_CODE_XDEV, EXDEV, "Invalid cross-device link");
    new_error!(ERROR_CODE_XFULL, EXFULL, "Exchange full");
}

impl Error {
    /// Creates an [`Error`] from an arbitrary value that can be formatted.
    pub fn new(value: impl fmt::Display + fmt::Debug + 'static) -> Self {
        let error = <Error>::new_error(value).into_error();
        Self(error, PhantomData)
    }
}

impl Error<dyn Send> {
    /// Creates an [`Error`] from an arbitrary value that can be formatted.
    pub fn new_send(value: impl fmt::Display + fmt::Debug + Send + 'static) -> Self {
        let error = <Error>::new_error(value).into_error();
        Self(error, PhantomData)
    }
}

impl Error<dyn Sync> {
    /// Creates an [`Error`] from an arbitrary value that can be formatted.
    pub fn new_sync(value: impl fmt::Display + fmt::Debug + Sync + 'static) -> Self {
        let error = <Error>::new_error(value).into_error();
        Self(error, PhantomData)
    }
}

impl Error<dyn Send + Sync> {
    /// Creates an [`Error`] from an arbitrary value that can be formatted.
    pub fn new_send_sync(value: impl fmt::Display + fmt::Debug + Send + Sync + 'static) -> Self {
        let error = <Error>::new_error(value).into_error();
        Self(error, PhantomData)
    }
}

impl<T: ?Sized> Error<T> {
    /// Creates an [`Error`] from a string.
    pub fn from_string(error: &'static CStr) -> Self {
        Self(
            bindings::FimoResult {
                data: error.as_ptr().cast_mut().cast(),
                // Safety: Is guaranteed to be valid.
                vtable: unsafe { &bindings::FIMO_IMPL_RESULT_STATIC_STRING_VTABLE },
            },
            PhantomData,
        )
    }

    /// Creates an [`Error`] from an error code.
    ///
    /// In case of an invalid error code, this returns `Err`.
    pub fn from_error_code(
        error: bindings::FimoErrorCode,
    ) -> Result<Self, bindings::FimoErrorCode> {
        if !is_valid_error_code(error) {
            Err(error)
        } else {
            // Safety: We checked the validity.
            unsafe { Ok(Self::from_error_code_unchecked(error)) }
        }
    }

    /// Creates an [`Error`] from an error code.
    ///
    /// # Safety
    ///
    /// `error` must be a valid error code and not
    /// [`FIMO_ERROR_CODE_OK`](bindings::FimoErrorCode::FIMO_ERROR_CODE_OK).
    pub unsafe fn from_error_code_unchecked(error: bindings::FimoErrorCode) -> Self {
        Self(
            bindings::FimoResult {
                data: std::ptr::without_provenance_mut(error.0 as usize),
                // Safety: Is guaranteed to be valid.
                vtable: unsafe { &bindings::FIMO_IMPL_RESULT_ERROR_CODE_VTABLE },
            },
            PhantomData,
        )
    }

    /// Creates an [`Error`] from a system error code.
    pub fn from_system_error(error: bindings::FimoSystemErrorCode) -> Self {
        Self(
            bindings::FimoResult {
                data: std::ptr::without_provenance_mut(error as usize),
                // Safety: Is guaranteed to be valid.
                vtable: unsafe { &bindings::FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE },
            },
            PhantomData,
        )
    }

    /// Returns the error code.
    pub fn into_error(self) -> bindings::FimoResult {
        let this = ManuallyDrop::new(self);
        this.0
    }

    /// Constructs an [`Error`] from an errno value.
    pub fn from_errno(errnum: core::ffi::c_int) -> Self {
        // Safety: The function is safe to be called.
        let errnum = unsafe { bindings::fimo_error_code_from_errno(errnum) };
        Self::from_error_code(errnum).expect("unknown error code")
    }

    /// Returns a string representing the error.
    pub fn name(&self) -> ErrorString {
        let vtable = self.vtable();
        // Safety: FFI call is always safe.
        unsafe {
            let string = vtable.v0.error_name.unwrap_unchecked()(self.0.data);
            ErrorString(string)
        }
    }

    /// Returns a string describing the error.
    pub fn description(&self) -> ErrorString {
        let vtable = self.vtable();
        // Safety: FFI call is always safe.
        unsafe {
            let string = vtable.v0.error_description.unwrap_unchecked()(self.0.data);
            ErrorString(string)
        }
    }

    fn vtable(&self) -> &bindings::FimoResultVTable {
        // Safety: All fields are guaranteed to be initialized.
        unsafe { &*self.0.vtable }
    }
}

fn is_valid_error_code(errnum: bindings::FimoErrorCode) -> bool {
    (bindings::FimoErrorCode::FIMO_ERROR_CODE_2BIG.0
        ..=bindings::FimoErrorCode::FIMO_ERROR_CODE_XFULL.0)
        .contains(&errnum.0)
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        f.debug_tuple("Error").field(&&*name).finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = self.description();
        write!(f, "{}", description.to_string_lossy())
    }
}

impl<T: ?Sized> Drop for Error<T> {
    fn drop(&mut self) {
        let vtable = self.vtable();
        if let Some(release) = vtable.v0.release {
            // Safety: FFI call is always safe.
            unsafe {
                release(self.0.data);
            }
        }
    }
}

// Safety: Is equivalent to a Box<dyn Send>.
unsafe impl<T: ?Sized + Send> Send for Error<T> {}

// Safety: Is equivalent to a Box<dyn Sync>.
unsafe impl<T: ?Sized + Sync> Sync for Error<T> {}

impl From<Error<dyn Send>> for Error {
    fn from(value: Error<dyn Send>) -> Self {
        Self(value.into_error(), PhantomData)
    }
}

impl From<Error<dyn Sync>> for Error {
    fn from(value: Error<dyn Sync>) -> Self {
        Self(value.into_error(), PhantomData)
    }
}

impl From<Error<dyn Send + Sync>> for Error {
    fn from(value: Error<dyn Send + Sync>) -> Self {
        Self(value.into_error(), PhantomData)
    }
}

impl From<Error<dyn Send + Sync>> for Error<dyn Send> {
    fn from(value: Error<dyn Send + Sync>) -> Self {
        Self(value.into_error(), PhantomData)
    }
}

impl From<Error<dyn Send + Sync>> for Error<dyn Sync> {
    fn from(value: Error<dyn Send + Sync>) -> Self {
        Self(value.into_error(), PhantomData)
    }
}

trait NewErrorSpec<T>
where
    T: 'static,
{
    fn new_error(value: T) -> Self;
}

impl<T> NewErrorSpec<T> for Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    default fn new_error(value: T) -> Self {
        fn new_string(string: String) -> bindings::FimoResultString {
            extern "C" fn release(ffi: *const std::ffi::c_char) {
                let ffi = ffi.cast_mut();
                // Safety: We know that it is a valid string.
                unsafe {
                    let _ = CString::from_raw(ffi);
                }
            }

            match CString::new(string) {
                Ok(string) => bindings::FimoResultString {
                    str_: string.into_raw(),
                    release: Some(release),
                },
                Err(_) => <Error>::from_string(c"CString::new failed")
                    .description()
                    .into_ffi(),
            }
        }

        extern "C" fn drop_inline<T>(mut ffi: *mut std::ffi::c_void) {
            let value_ptr: *mut T = std::ptr::from_mut(&mut ffi).cast();
            // Safety: The value is valid.
            unsafe { std::ptr::drop_in_place::<T>(value_ptr) };
        }
        extern "C" fn debug_inline<T: fmt::Debug>(
            ffi: *mut std::ffi::c_void,
        ) -> bindings::FimoResultString {
            let value_ptr: *const T = std::ptr::from_ref(&ffi).cast();
            // Safety: The value is valid.
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{:?}", *value))
            }
        }
        extern "C" fn display_inline<T: fmt::Display>(
            ffi: *mut std::ffi::c_void,
        ) -> bindings::FimoResultString {
            let value_ptr: *const T = std::ptr::from_ref(&ffi).cast();
            // Safety: The value is valid.
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{}", *value))
            }
        }

        // If the size and alignments match we can store the value inline.
        if size_of::<T>() <= size_of::<*mut std::ffi::c_void>()
            && align_of::<T>() <= align_of::<*mut std::ffi::c_void>()
        {
            let vtable: &'static bindings::FimoResultVTable = &const {
                bindings::FimoResultVTable {
                    v0: bindings::FimoResultVTableV0 {
                        release: if std::mem::needs_drop::<T>() {
                            Some(drop_inline::<T>)
                        } else {
                            None
                        },
                        error_name: Some(debug_inline::<T>),
                        error_description: Some(display_inline::<T>),
                    },
                }
            };

            let value = ManuallyDrop::new(value);
            // Safety: We checked that it is safe.
            let data = unsafe { std::mem::transmute_copy(&value) };
            let error = bindings::FimoResult { data, vtable };
            return Self(error, PhantomData);
        }

        extern "C" fn drop_boxed<T>(ffi: *mut std::ffi::c_void) {
            let value_ptr: *mut T = ffi.cast();
            // Safety: The value is valid.
            unsafe { drop(Box::<T>::from_raw(value_ptr)) };
        }
        extern "C" fn debug_boxed<T: fmt::Debug>(
            ffi: *mut std::ffi::c_void,
        ) -> bindings::FimoResultString {
            let value_ptr: *const T = ffi.cast();
            // Safety: The value is valid.
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{:?}", *value))
            }
        }
        extern "C" fn display_boxed<T: fmt::Display>(
            ffi: *mut std::ffi::c_void,
        ) -> bindings::FimoResultString {
            let value_ptr: *const T = ffi.cast();
            // Safety: The value is valid.
            unsafe {
                let value = &*value_ptr;
                new_string(format!("{}", *value))
            }
        }

        // Fall back to boxing the error.
        let vtable: &'static bindings::FimoResultVTable = &const {
            bindings::FimoResultVTable {
                v0: bindings::FimoResultVTableV0 {
                    release: Some(drop_boxed::<T>),
                    error_name: Some(debug_boxed::<T>),
                    error_description: Some(display_boxed::<T>),
                },
            }
        };

        let value = Box::new(value);
        let data = Box::into_raw(value).cast();
        let error = bindings::FimoResult { data, vtable };
        Self(error, PhantomData)
    }
}

impl<T: ?Sized> FFITransferable<bindings::FimoResult> for Error<T> {
    fn into_ffi(self) -> bindings::FimoResult {
        self.into_error()
    }

    unsafe fn from_ffi(ffi: bindings::FimoResult) -> Self {
        Self(ffi, PhantomData)
    }
}

/// A string returned from an `Error`.
pub struct ErrorString(bindings::FimoResultString);

// Safety: Is only a `&CStr`.
unsafe impl Send for ErrorString {}

// Safety: Is only a `&CStr`.
unsafe impl Sync for ErrorString {}

impl Deref for ErrorString {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        // Safety: The string is always valid.
        unsafe { CStr::from_ptr(self.0.str_) }
    }
}

impl Drop for ErrorString {
    fn drop(&mut self) {
        if let Some(release) = self.0.release {
            // Safety: The function is safe to be called.
            unsafe { release(self.0.str_) }
        }
    }
}

impl FFITransferable<bindings::FimoResultString> for ErrorString {
    fn into_ffi(self) -> bindings::FimoResultString {
        let this = ManuallyDrop::new(self);
        this.0
    }

    unsafe fn from_ffi(ffi: bindings::FimoResultString) -> Self {
        Self(ffi)
    }
}

/// A [`Result`] with an [`Error`] error type.
pub type Result<T = (), E = Error> = core::result::Result<T, E>;

/// Converts a [`FimoResult`](bindings::FimoResult) to an error if it's greater than zero, and
/// `Ok(())` otherwise.
///
/// # Safety
///
/// - `error` must be properly initialized.
pub unsafe fn to_result(error: bindings::FimoResult) -> Result {
    if !error.data.is_null() {
        Err(Error(error, PhantomData))
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
    let mut error = <Error>::FFI_OK_RESULT;
    let result = f(&mut error);
    if !error.vtable.is_null() {
        Err(Error(error, PhantomData))
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
    let mut error = <Error>::FFI_OK_RESULT;

    f(&mut error, &mut data);
    if !error.vtable.is_null() {
        Err(Error(error, PhantomData))
    } else {
        // Safety: By the contract of this function the data
        // must have been initialized, if the function does
        // not return an error.
        let result = unsafe { data.assume_init() };
        Ok(result)
    }
}

impl<T: ?Sized> FFITransferable<bindings::FimoResult> for Result<(), Error<T>> {
    fn into_ffi(self) -> bindings::FimoResult {
        match self {
            Ok(_) => <Error>::FFI_OK_RESULT,
            Err(x) => x.into_error(),
        }
    }

    unsafe fn from_ffi(ffi: bindings::FimoResult) -> Self {
        if !ffi.vtable.is_null() {
            Err(Error(ffi, PhantomData))
        } else {
            Ok(())
        }
    }
}
