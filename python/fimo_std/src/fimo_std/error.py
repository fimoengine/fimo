import ctypes as c
from enum import IntEnum
from typing import Self

from .enum import ABCEnum
from . import ffi as _ffi


class ErrorCode(_ffi.FFITransferable[_ffi.FimoError], IntEnum, metaclass=ABCEnum):
    """Error codes."""
    EOK = 0
    """Operation completed successfully"""
    E2BIG = 1
    """Argument list too long"""
    EACCES = 2
    """Permission denied"""
    EADDRINUSE = 3
    """Address already in use"""
    EADDRNOTAVAIL = 4
    """Address not available"""
    EAFNOSUPPORT = 5
    """Address family not supported"""
    EAGAIN = 6
    """Resource temporarily unavailable"""
    EALREADY = 7
    """Connection already in progress"""
    EBADE = 8
    """Invalid exchange"""
    EBADF = 9
    """Bad file descriptor"""
    EBADFD = 10
    """File descriptor in bad state"""
    EBASMSG = 11
    """Bad message"""
    EBADR = 12
    """Invalid request descriptor"""
    EBADRQC = 13
    """Invalid request code"""
    EBADSLT = 14
    """Invalid slot"""
    EBUSY = 15
    """Device or resource busy"""
    ECANCELED = 16
    """Operation canceled"""
    ECHILD = 17
    """No child processes"""
    ECHRNG = 18
    """Channel number out of range"""
    ECOMM = 19
    """Communication error on send"""
    ECONNABORTED = 20
    """Connection aborted"""
    ECONNREFUSED = 21
    """Connection refused"""
    ECONNRESET = 22
    """Connection reset"""
    EDEADLK = 23
    """Resource deadlock avoided"""
    EDEADLOCK = 24
    """File locking deadlock error (or Resource deadlock avoided)"""
    EDESTADDRREQ = 25
    """Destination address required"""
    EDOM = 26
    """Mathematics argument out of domain of function"""
    EDQUOT = 27
    """Disk quota exceeded"""
    EEXIST = 28
    """File exists"""
    EFAULT = 29
    """Bad address"""
    EFBIG = 30
    """File too large"""
    EHOSTDOWN = 31
    """Host is down"""
    EHOSTUNREACH = 32
    """Host is unreachable"""
    EHWPOISON = 33
    """Memory page has hardware error"""
    EIDRM = 34
    """Identifier removed"""
    EILSEQ = 35
    """Invalid or incomplete multibyte or wide character"""
    EINPROGRESS = 36
    """Operation in progress"""
    EINTR = 37
    """Interrupted function call"""
    EINVAL = 38
    """Invalid argument"""
    EIO = 39
    """Input/output error"""
    EISCONN = 40
    """Socket is connected"""
    EISDIR = 41
    """Is a directory"""
    EISNAM = 42
    """Is a named type file"""
    EKEYEXPIRED = 43
    """Key has expired"""
    EKEYREJECTED = 44
    """Key was rejected by service"""
    EKEYREVOKED = 45
    """Key has been revoked"""
    EL2HLT = 46
    """Level 2 halted"""
    EL2NSYNC = 47
    """Level 2 not synchronized"""
    EL3HLT = 48
    """Level 3 halted"""
    EL3RST = 49
    """Level 3 reset"""
    ELIBACC = 50
    """Cannot access a needed shared library"""
    ELIBBAD = 51
    """Accessing a corrupted shared library"""
    ELIBMAX = 52
    """Attempting to link in too many shared libraries"""
    ELIBSCN = 53
    """.lib section in a.out corrupted"""
    ELIBEXEC = 54
    """Cannot exec a shared library directly"""
    ELNRNG = 55
    """Link number out of range"""
    ELOOP = 56
    """Too many levels of symbolic links"""
    EMEDIUMTYPE = 57
    """Wrong medium type"""
    EMFILE = 58
    """Too many open files"""
    EMLINK = 59
    """Too many links"""
    EMSGSIZE = 60
    """Message too long"""
    EMULTIHOP = 61
    """Multihop attempted"""
    ENAMETOOLONG = 62
    """Filename too long"""
    ENETDOWN = 63
    """Network is down"""
    ENETRESET = 64
    """Connection aborted by network"""
    ENETUNREACH = 65
    """Network unreachable"""
    ENFILE = 66
    """Too many open files in system"""
    ENOANO = 67
    """No anode"""
    ENOBUFS = 68
    """No buffer space available"""
    ENODATA = 69
    """The named attribute does not exist, or the process has no access to this attribute"""
    ENODEV = 70
    """No such device"""
    ENOENT = 71
    """No such file or directory"""
    ENOEXEC = 72
    """Exec format error"""
    ENOKEY = 73
    """Required key not available"""
    ENOLCK = 74
    """No locks available"""
    ENOLINK = 75
    """Link has been severed"""
    ENOMEDIUM = 76
    """No medium found"""
    ENOMEM = 77
    """Not enough space/cannot allocate memory"""
    ENOMSG = 78
    """No message of the desired type"""
    ENONET = 79
    """Machine is not on the network"""
    ENOPKG = 80
    """Package not installed"""
    ENOPROTOOPT = 81
    """Protocol not available"""
    ENOSPC = 82
    """No space left on device"""
    ENOSR = 83
    """No STREAM resources"""
    ENOSTR = 84
    """Not a STREAM"""
    ENOSYS = 85
    """Function not implemented"""
    ENOTBLK = 86
    """Block device required"""
    ENOTCONN = 87
    """The socket is not connected"""
    ENOTDIR = 88
    """Not a directory"""
    ENOTEMPTY = 89
    """Directory not empty"""
    ENOTRECOVERABLE = 90
    """State not recoverable"""
    ENOTSOCK = 91
    """Not a socket"""
    ENOTSUP = 92
    """Operation not supported"""
    ENOTTY = 93
    """Inappropriate I/O control operation"""
    ENOTUNIQ = 94
    """Name not unique on network"""
    ENXIO = 95
    """No such device or address"""
    EOPNOTSUPP = 96
    """Operation not supported on socket"""
    EOVERFLOW = 97
    """Value too large to be stored in data type"""
    EOWNERDEAD = 98
    """Owner died"""
    EPERM = 99
    """Operation not permitted"""
    EPFNOSUPPORT = 100
    """Protocol family not supported"""
    EPIPE = 101
    """Broken pipe"""
    EPROTO = 102
    """Protocol error"""
    EPROTONOSUPPORT = 103
    """Protocol not supported"""
    EPROTOTYPE = 104
    """Protocol wrong type for socket"""
    ERANGE = 105
    """Result too large"""
    EREMCHG = 106
    """Remote address changed"""
    EREMOTE = 107
    """Object is remote"""
    EREMOTEIO = 108
    """Remote I/O error"""
    ERESTART = 109
    """Interrupted system call should be restarted"""
    ERFKILL = 110
    """Operation not possible due to RF-kill"""
    EROFS = 111
    """Read-only filesystem"""
    ESHUTDOWN = 112
    """Cannot send after transport endpoint shutdown"""
    ESPIPE = 113
    """Invalid seek"""
    ESOCKTNOSUPPORT = 114
    """Socket type not supported"""
    ESRCH = 115
    """No such process"""
    ESTALE = 116
    """Stale file handle"""
    ESTRPIPE = 117
    """Streams pipe error"""
    ETIME = 118
    """Timer expired"""
    ETIMEDOUT = 119
    """Connection timed out"""
    ETOOMANYREFS = 120
    """Too many references: cannot splice"""
    ETXTBSY = 121
    """Text file busy"""
    EUCLEAN = 122
    """Structure needs cleaning"""
    EUNATCH = 123
    """Protocol driver not attached"""
    EUSERS = 124
    """Too many users"""
    EWOULDBLOCK = 125
    """Operation would block"""
    EXDEV = 126
    """Invalid cross-device link"""
    EXFULL = 127
    """Exchange full"""
    EUNKNOWN = 128
    """Unknown error"""

    def transfer_to_ffi(self) -> _ffi.FimoError:
        return _ffi.FimoError(self)

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoError) -> Self:
        return cls(ffi.value)

    @classmethod
    def from_param(cls, obj):
        return cls(obj)

    @classmethod
    def from_errno(cls, errnum: int) -> Self:
        """Constructs the `ErrorCode` from an errno value"""
        error = _ffi.fimo_error_from_errno(c.c_int(errnum))
        return cls.transfer_from_ffi(error)

    @classmethod
    def from_exception(cls, exception: Exception) -> Self:
        """Constructs the `ErrorCode` from a python exception."""
        if isinstance(exception, Error):
            return cls(exception.error_code())
        elif isinstance(exception, MemoryError):
            return cls(ErrorCode.ENOMEM)
        elif isinstance(exception, MemoryError):
            return cls(ErrorCode.ENOMEM)
        elif isinstance(exception, NotImplementedError):
            return cls(ErrorCode.ENOSYS)
        elif isinstance(exception, OSError):
            return cls.from_errno(exception.errno)
        elif isinstance(exception, TypeError):
            return cls(ErrorCode.EINVAL)
        elif isinstance(exception, ValueError):
            return cls(ErrorCode.EINVAL)
        else:
            return cls(ErrorCode.EUNKNOWN)

    def is_valid(self) -> bool:
        """Checks if an error number is valid."""
        return ErrorCode.EOK <= self <= ErrorCode.EUNKNOWN

    def is_error(self) -> bool:
        """Checks if the error code represents an error."""
        return self.is_valid() and self != ErrorCode.EOK

    def raise_if_error(self) -> None:
        """Raises an Error if the error code represents an error."""
        if self.is_error():
            raise Error(self)

    @property
    def name(self) -> str:
        """Returns the name of the error code"""
        error = _ffi.FimoError(0)
        name = _ffi.fimo_strerrorname(_ffi.FimoError(self), c.byref(error))
        error_code = ErrorCode(error.value)
        error_code.raise_if_error()
        return name.decode()

    @property
    def description(self) -> str:
        """Returns the description of the error code"""
        error = _ffi.FimoError(0)
        name = _ffi.fimo_strerrordesc(_ffi.FimoError(self), c.byref(error))
        error_code = ErrorCode(error.value)
        error_code.raise_if_error()
        return name.decode()


class Error(Exception):
    """An error exception"""

    def __init__(self, code: ErrorCode) -> None:
        """Initializes the Error with an ErrorCode"""
        if code is None or not isinstance(code, ErrorCode):
            code = ErrorCode.EUNKNOWN

        super().__init__(code.description)
        self._error_code = code

    def error_code(self) -> ErrorCode:
        """Returns the contained ErrorCode"""
        return self._error_code
