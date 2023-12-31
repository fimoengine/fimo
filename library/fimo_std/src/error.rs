//! Fimo error codes.

use core::{ffi::CStr, fmt};

use crate::bindings;

/// Generic error code.
///
/// The error codes are based on the POSIX errno codes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Error(bindings::FimoError);

mod __private {
    use paste::paste;

    macro_rules! new_error {
        ($err:ident, $($doc:literal),+) => {
            paste! {
                impl super::Error {
                    $(
                        #[doc = $doc]
                    )+
                    pub const $err: Self = Self(crate::bindings::FimoError::[<FIMO_ $err>]);
                }
            }
        };
    }

    new_error!(EOK, "Operation completed successfully");
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
    new_error!(EUNKNOWN, "Unknown error");
}

impl Error {
    /// Creates an [`Error`] from an error code.
    ///
    /// In case of an invalid error code, this returns `EINVAL`.
    pub fn from_error(error: bindings::FimoError) -> Self {
        // Safety: The function is safe to be called.
        if unsafe { !bindings::fimo_is_valid_error(error) } {
            Self(bindings::FimoError::FIMO_EINVAL)
        } else {
            Self(error)
        }
    }

    /// Creates an [`Error`] from an error code.
    ///
    /// # Safety
    ///
    /// `error` must be a valid error code.
    pub unsafe fn from_error_unchecked(error: bindings::FimoError) -> Self {
        Self(error)
    }

    /// Returns the error code.
    pub fn into_error(self) -> bindings::FimoError {
        self.0
    }

    /// Constructs an [`Error`] from an errno value.
    pub fn from_errno(errnum: core::ffi::c_int) -> Self {
        // Safety: The function is safe to be called.
        unsafe { Self(crate::bindings::fimo_error_from_errno(errnum)) }
    }

    /// Returns a string representing the error, if one exists.
    pub fn name(&self) -> Option<&'static str> {
        let mut error = bindings::FimoError::FIMO_EOK;

        // Safety: The error pointer is valid.
        let ptr = unsafe { bindings::fimo_strerrorname(self.0, &mut error) };

        // Safety: The function is safe.
        if unsafe { bindings::fimo_is_error(error) } {
            None
        } else {
            // Safety: The string returned by `fimo_strerrorname` is static and `NULL`-terminated.
            let name = unsafe { CStr::from_ptr(ptr) };

            // Safety: These strings are ASCII-only.
            unsafe { Some(core::str::from_utf8_unchecked(name.to_bytes())) }
        }
    }

    /// Returns a string describing the error, if one exists.
    pub fn description(&self) -> Option<&'static str> {
        let mut error = bindings::FimoError::FIMO_EOK;

        // Safety: The error pointer is valid.
        let ptr = unsafe { bindings::fimo_strerrordesc(self.0, &mut error) };

        // Safety: The function is safe.
        if unsafe { bindings::fimo_is_error(error) } {
            None
        } else {
            // Safety: The string returned by `fimo_strerrordesc` is static and `NULL`-terminated.
            let desc = unsafe { CStr::from_ptr(ptr) };

            // Safety: These strings are ASCII-only.
            unsafe { Some(core::str::from_utf8_unchecked(desc.to_bytes())) }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(name) => f.debug_tuple(name).finish(),
            None => f.debug_tuple("Error").field(&self.0).finish(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.description() {
            Some(description) => f.write_str(description),
            None => write!(f, "Unknown error code"),
        }
    }
}

/// A [`Result`] with an [`Error`] error type.
pub type Result<T = (), E = Error> = core::result::Result<T, E>;

/// Converts a [`FimoError`](bindings::FimoError) to an error if it's greater than zero, and
/// `Ok(())` otherwise.
pub fn to_result(error: bindings::FimoError) -> Result {
    if error.0 > 0 {
        Err(Error::from_error(error))
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
///     fn my_func(error: *mut bindings::FimoError);
/// }
///
/// to_result_indirect(|error| unsafe { my_func(error) });
/// ```
pub fn to_result_indirect<T>(f: impl FnOnce(&mut bindings::FimoError) -> T) -> Result<T> {
    let mut error = Error::EOK.into_error();
    let result = f(&mut error);
    if error.0 != bindings::FimoError::FIMO_EOK.0 {
        Err(Error::from_error(error))
    } else {
        Ok(result)
    }
}
