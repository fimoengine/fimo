from __future__ import annotations

import ctypes as c
from enum import IntEnum
from typing import Self, Any

from ._enum import ABCEnum
from . import ffi as _ffi


class ErrorCode(_ffi.FFITransferable[_ffi.FimoErrorCode], IntEnum, metaclass=ABCEnum):
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

    def transfer_to_ffi(self) -> _ffi.FimoErrorCode:
        return _ffi.FimoErrorCode(self)

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoErrorCode) -> Self:
        return cls(ffi.value)

    @classmethod
    def from_param(cls, obj):
        return cls(obj)

    @classmethod
    def from_errno(cls, errnum: int) -> Self:
        """Constructs the `ErrorCode` from an errno value"""
        error = _ffi.fimo_error_code_from_errno(c.c_int(errnum))
        return cls.transfer_from_ffi(error)

    @property
    def name(self) -> str:
        """Returns the name of the error code"""
        name = _ffi.fimo_error_code_name(_ffi.FimoErrorCode(self))
        return name.decode()

    @property
    def description(self) -> str:
        """Returns the description of the error code"""
        name = _ffi.fimo_error_code_description(_ffi.FimoErrorCode(self))
        return name.decode()


class _PyObjWrapper:
    def __init__(self, vtable: _ffi.FimoResultVTable, obj: Any) -> None:
        self.vtable = vtable
        self.obj = obj

    def __repr__(self):
        return repr(self.obj)

    def __str__(self):
        return str(self.obj)


def _py_obj_result_string_release(ptr: c._Pointer[c.c_char]) -> None:
    _ffi.fimo_free(c.cast(ptr, c.c_void_p))


_py_obj_result_string_release_ref = c.CFUNCTYPE(None, c.POINTER(c.c_char))(
    _py_obj_result_string_release
)


def _py_obj_result_release(ptr: int) -> None:
    try:
        obj = c.cast(ptr, c.py_object)
        _ffi.c_dec_ref(obj)
        del obj
    except Exception:
        pass


def _py_obj_result_error_name(ptr: int) -> _ffi.FimoResultString:
    try:
        obj = c.cast(ptr, c.py_object).value
        name = repr(obj).encode()

        error = _ffi.FimoResult()
        str_ptr = _ffi.fimo_calloc(c.c_size_t(len(name) + 1), c.byref(error))
        if _ffi.fimo_result_is_error(error):
            string = _ffi.fimo_result_error_name(error)
            _ffi.fimo_result_release(error)
            return string
        else:
            c.memmove(str_ptr, name, len(name))
            return _ffi.FimoResultString(
                c.cast(str_ptr, c.POINTER(c.c_char)),
                _py_obj_result_string_release_ref,
            )
    except Exception:
        error = _ffi.FIMO_IMPL_RESULT_INVALID_ERROR
        return _ffi.fimo_result_error_name(error)


def _py_obj_result_error_description(ptr: int) -> _ffi.FimoResultString:
    try:
        obj = c.cast(ptr, c.py_object).value
        desc = str(obj).encode()

        error = _ffi.FimoResult()
        str_ptr = _ffi.fimo_calloc(c.c_size_t(len(desc) + 1), c.byref(error))
        if _ffi.fimo_result_is_error(error):
            string = _ffi.fimo_result_error_description(error)
            _ffi.fimo_result_release(error)
            return string
        else:
            c.memmove(str_ptr, desc, len(desc))
            return _ffi.FimoResultString(
                c.cast(str_ptr, c.POINTER(c.c_char)),
                _py_obj_result_string_release_ref,
            )
    except Exception:
        error = _ffi.FIMO_IMPL_RESULT_INVALID_ERROR
        return _ffi.fimo_result_error_description(error)


class Result(_ffi.FFITransferable[_ffi.FimoResult]):
    """Success status of an operation."""

    _create_key = object()

    def __init__(self, create_key: object, result: _ffi.FimoResult):
        if create_key is not Result._create_key:
            raise ValueError("`create_key` must be an instance of `_create_key`")
        if not isinstance(result, _ffi.FimoResult):
            raise TypeError("`result` must be an instance of `FimoResult`")

        self._result: _ffi.FimoResult | None = result

    def __del__(self):
        if self._result is not None:
            _ffi.fimo_result_release(self._result)
            self._result = None

    def transfer_to_ffi(self) -> _ffi.FimoResult:
        if self._result is None:
            raise ValueError("result has been consumed")

        result = self._result
        self._result = None
        return result

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoResult) -> Self:
        return cls(cls._create_key, ffi)

    @classmethod
    def new(cls, error: Any) -> Self:
        """Constructs a new error wrapping the `error` object.

        Passing in `None` will result in the `Ok` value.
        """
        if error is None:
            return cls.from_error_code(ErrorCode.EOK)

        # Fill the vtable
        vtable = _ffi.FimoResultVTable()
        vtable.v0.release = c.CFUNCTYPE(None, c.c_void_p)(_py_obj_result_release)
        vtable.v0.error_name = c.CFUNCTYPE(_ffi.FimoResultString, c.c_void_p)(
            _py_obj_result_error_name
        )
        vtable.v0.error_description = c.CFUNCTYPE(_ffi.FimoResultString, c.c_void_p)(
            _py_obj_result_error_description
        )

        wrapper = _PyObjWrapper(vtable, error)
        wrapper_ffi = c.py_object(wrapper)

        # Create the struct
        ffi = _ffi.FimoResult(
            c.c_void_p.from_buffer(wrapper_ffi),
            c.pointer(vtable),
        )

        # Take ownership of the object
        _ffi.c_inc_ref(wrapper)
        return cls.transfer_from_ffi(ffi)

    @classmethod
    def from_error(cls, error: "Error") -> Self:
        if not isinstance(error, Error):
            raise TypeError("`error` must be an instance of `Error`")

        # consume the result of the error
        result = error.result().transfer_to_ffi()
        return cls.transfer_from_ffi(result)

    @classmethod
    def from_error_code(cls, error_code: ErrorCode) -> Self:
        if not isinstance(error_code, ErrorCode):
            raise TypeError("`error_code` must be an instance of `ErrorCode`")

        ffi = _ffi.fimo_result_from_error_code(error_code.transfer_to_ffi())
        return cls(cls._create_key, ffi)

    @classmethod
    def from_system_error(cls, system_error: int) -> Self:
        if not isinstance(system_error, int):
            raise TypeError("`system_error` must be an instance of `int`")

        ffi = _ffi.fimo_result_from_system_error_code(
            _ffi.FimoSystemErrorCode(system_error)
        )
        return cls(cls._create_key, ffi)

    def is_error(self) -> bool:
        """Checks whether the result signifies an error."""
        if self._result is None:
            raise ValueError("result has been consumed")

        return _ffi.fimo_result_is_error(self._result)

    def is_ok(self) -> bool:
        """Checks whether the result does not signify an error."""
        if self._result is None:
            raise ValueError("result has been consumed")

        return _ffi.fimo_result_is_ok(self._result)

    def raise_if_error(self) -> None:
        """Raises an Error if the result represents an error."""
        if self.is_error():
            raise Error(self)

    @property
    def name(self) -> str:
        """Returns the error name of the result"""
        if self._result is None:
            raise ValueError("result has been consumed")

        name = _ffi.fimo_result_error_name(self._result)
        name_bytes = c.cast(name.str, c.c_char_p).value
        assert isinstance(name_bytes, bytes)
        _ffi.fimo_result_string_release(name)
        return name_bytes.decode()

    @property
    def description(self) -> str:
        """Returns the error description of the result"""
        if self._result is None:
            raise ValueError("result has been consumed")

        desc = _ffi.fimo_result_error_description(self._result)
        desc_bytes = c.cast(desc.str, c.c_char_p).value
        assert isinstance(desc_bytes, bytes)
        _ffi.fimo_result_string_release(desc)
        return desc_bytes.decode()

    def __repr__(self):
        return f"Result({self.name})"

    def __str__(self):
        return self.description


class Error(Exception):
    """An error exception"""

    def __init__(self, result: Result) -> None:
        """Initializes the Error with an error Result"""
        if not isinstance(result, Result):
            raise TypeError("`result` must be an instance of `Result`")
        if result.is_ok():
            raise ValueError("`result` does not represent an error")

        super().__init__(result.description)
        self._result = result

    def result(self) -> Result:
        """Returns the contained Result"""
        return self._result
