from abc import ABC, abstractmethod
from typing import Generic, TypeVar, Self
import ctypes as c
import platform
import os

FfiType = TypeVar('FfiType')
FfiTypeView = TypeVar('FfiTypeView')


class FFITransferable(ABC, Generic[FfiType]):
    """A type that can be transferred over the ffi boundary."""

    @abstractmethod
    def transfer_to_ffi(self) -> FfiType:
        """Transfers the ownership from a Python type to a ffi type."""
        pass

    @classmethod
    @abstractmethod
    def transfer_from_ffi(cls, ffi: FfiType) -> Self:
        """Assumes ownership of a ffi type."""
        pass


class FFISharable(ABC, Generic[FfiType, FfiTypeView]):
    """A type that can be shared over the ffi boundary."""

    @property
    @abstractmethod
    def ffi(self) -> FfiType:
        """Accesses the ffi type."""
        pass

    @classmethod
    @abstractmethod
    def borrow_from_ffi(cls, ffi: FfiType) -> FfiTypeView:
        """Borrows the ownership of a ffi type."""
        pass


if platform.system() == "Linux":
    _lib_name = "libfimo_std_shared.so"
elif platform.system() == "Darwin":
    _lib_name = "libfimo_std_shared.dylib"
elif platform.system() == "Windows":
    _lib_name = "fimo_std_shared.dll"
else:
    raise RuntimeError("Unsupported platform")

_lib_path = os.path.join(os.path.dirname(__file__), _lib_name)
_lib = c.CDLL(_lib_path)

c_inc_ref = c.pythonapi.Py_IncRef
c_inc_ref.argtypes = [c.py_object]
c_dec_ref = c.pythonapi.Py_DecRef
c_dec_ref.argtypes = [c.py_object]


# Header: fimo_std/error.h

class FimoError(c.c_int):
    """Posix error codes"""
    pass


FimoErrorPtr = c.POINTER(FimoError)
"""Pointer to a FimoError"""

_fimo_strerrorname = _lib.fimo_strerrorname
_fimo_strerrorname.argtypes = [FimoError, FimoErrorPtr]
_fimo_strerrorname.restype = c.c_char_p


def fimo_strerrorname(errnum: FimoError, err: FimoErrorPtr) -> bytes:
    """Get the name of the error"""
    return _fimo_strerrorname(errnum, err)


_fimo_strerrordesc = _lib.fimo_strerrordesc
_fimo_strerrordesc.argtypes = [FimoError, FimoErrorPtr]
_fimo_strerrordesc.restype = c.c_char_p


def fimo_strerrordesc(errnum: FimoError, err: FimoErrorPtr) -> bytes:
    """Get the description of the error"""
    return _fimo_strerrordesc(errnum, err)


_fimo_error_from_errno = _lib.fimo_error_from_errno
_fimo_error_from_errno.argtypes = [c.c_int]
_fimo_error_from_errno.restype = FimoError


def fimo_error_from_errno(errnum: c.c_int) -> FimoError:
    """Constructs an error code from an errno error code."""
    return _fimo_error_from_errno(errnum)


# Header: fimo_std/memory.h

FIMO_MALLOC_ALIGNMENT = 16
"""Minimum alignment of the default allocator"""


class FimoMallocBuffer(c.Structure):
    """Am allocated buffer."""
    _fields_ = [("ptr", c.c_void_p),
                ("buff_size", c.c_size_t)]


_fimo_malloc = _lib.fimo_malloc
_fimo_malloc.argtypes = [c.c_size_t, FimoErrorPtr]
_fimo_malloc.restype = c.c_void_p


def fimo_malloc(size: c.c_size_t, error: FimoErrorPtr) -> c.c_void_p:
    """Allocate memory.

    This function allocates at least `size` bytes and returns a pointer to the allocated
    memory. The memory is not initialized. If `size` is `0`, then `fimo_malloc()`
    returns `NULL`. If `error` is not a null pointer, `fimo_malloc()` writes the
    success status into the memory pointed to by `error`.

    :param size: size of the allocation
    :param error: optional pointer to an error slot

    :return: Pointer to the allocated memory
    """
    return c.c_void_p(_fimo_malloc(size, error))


_fimo_calloc = _lib.fimo_calloc
_fimo_calloc.argtypes = [c.c_size_t, FimoErrorPtr]
_fimo_calloc.restype = c.c_void_p


def fimo_calloc(size: c.c_size_t, error: FimoErrorPtr) -> c.c_void_p:
    """Zero-allocate memory.

    This function allocates at least `size` bytes and returns a pointer to the allocated
    memory. The memory is zero-initialized. If `size` is `0`, then `fimo_malloc()`
    returns `NULL`. If `error` is not a null pointer, `fimo_calloc()` writes the
    success status into the memory pointed to by `error`.

    :param size: size of the allocation
    :param error: optional pointer to an error slot

    :return: Pointer to the allocated memory
    """
    return c.c_void_p(_fimo_calloc(size, error))


_fimo_aligned_alloc = _lib.fimo_aligned_alloc
_fimo_aligned_alloc.argtypes = [c.c_size_t, c.c_size_t, FimoErrorPtr]
_fimo_aligned_alloc.restype = c.c_void_p


def fimo_aligned_alloc(alignment: c.c_size_t, size: c.c_size_t, error: FimoErrorPtr) -> c.c_void_p:
    """Allocate memory.

    This function allocates at least `size` bytes and returns a pointer to the allocated
    memory that is aligned at least as strictly as `alignment`. The memory is not initialized.
    If `size` is `0`, then `fimo_aligned_alloc()` returns `NULL` and `alignment` is ignored.
    `alignment` must be a power of two greater than `0`. If `error` is not a null pointer,
    `fimo_aligned_alloc()` writes the success status into the memory pointed to by `error`.

    :param alignment: alignment of the allocation
    :param size: size of the allocation
    :param error: optional pointer to an error slot

    :return: Pointer to the allocated memory
    """
    return c.c_void_p(_fimo_aligned_alloc(alignment, size, error))


_fimo_malloc_sized = _lib.fimo_malloc_sized
_fimo_malloc_sized.argtypes = [c.c_size_t, FimoErrorPtr]
_fimo_malloc_sized.restype = FimoMallocBuffer


def fimo_malloc_sized(size: c.c_size_t, error: FimoErrorPtr) -> FimoMallocBuffer:
    """Allocate memory.

    This function allocates at least `size` bytes and returns a pointer to the allocated
    memory, along with the usable size in bytes. The memory is not initialized. If `size`
    is `0`, then `fimo_malloc_sized()` returns `NULL`. If `error` is not a null pointer,
    `fimo_malloc_sized()` writes the success status into the memory pointed to by `error`.

    :param size: size of the allocation
    :param error: optional pointer to an error slot

    :return: Pointer to the allocated memory and usable size in bytes.
    """
    return _fimo_malloc_sized(size, error)


_fimo_calloc_sized = _lib.fimo_calloc_sized
_fimo_calloc_sized.argtypes = [c.c_size_t, FimoErrorPtr]
_fimo_calloc_sized.restype = FimoMallocBuffer


def fimo_calloc_sized(size: c.c_size_t, error: FimoErrorPtr) -> FimoMallocBuffer:
    """Zero-allocate memory.

    This function allocates at least `size` bytes and returns a pointer to the allocated
    memory, along with the usable size in bytes. The memory is zero-initialized. If `size`
    is `0`, then `fimo_calloc_sized()` returns `NULL`. If `error` is not a null pointer,
    `fimo_calloc_sized()` writes the success status into the memory pointed to by `error`.

    :param size: size of the allocation
    :param error: optional pointer to an error slot

    :return: Pointer to the allocated memory and usable size in bytes.
    """
    return _fimo_calloc_sized(size, error)


_fimo_aligned_alloc_sized = _lib.fimo_aligned_alloc_sized
_fimo_aligned_alloc_sized.argtypes = [c.c_size_t, c.c_size_t, FimoErrorPtr]
_fimo_aligned_alloc_sized.restype = FimoMallocBuffer


def fimo_aligned_alloc_sized(alignment: c.c_size_t, size: c.c_size_t, error: FimoErrorPtr) -> FimoMallocBuffer:
    """Allocate memory.

    This function allocates at least `size` bytes and returns a pointer to the allocated
    memory that is aligned at least as strictly as `alignment`, along with the usable size
    in bytes. The memory is not initialized. If `size` is `0`, then
    `fimo_aligned_alloc_sized()` returns `NULL` and `alignment` is ignored. `alignment`
    must be a power of two greater than `0`. If `error` is not a null pointer,
    `fimo_aligned_alloc_sized()` writes the success status into the memory pointed to
    by `error`.

    :param alignment: alignment of the allocation
    :param size: size of the allocation
    :param error: optional pointer to an error slot

    :return: Pointer to the allocated memory and usable size in bytes.
    """
    return _fimo_aligned_alloc_sized(alignment, size, error)


_fimo_free = _lib.fimo_free
_fimo_free.argtypes = [c.c_void_p]
_fimo_free.restype = None


def fimo_free(ptr: c.c_void_p) -> None:
    """Free allocated memory.

    Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
    no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
    allocation function, or if the space has been deallocated by a call to `fimo_free()`,
    `fimo_free_sized()` or `fimo_free_aligned_sized()`, the behavior is undefined.

    :param ptr: pointer to the memory
    """
    _fimo_free(ptr)


_fimo_free_sized = _lib.fimo_free_sized
_fimo_free_sized.argtypes = [c.c_void_p, c.c_size_t]
_fimo_free_sized.restype = None


def fimo_free_sized(ptr: c.c_void_p, size: c.c_size_t) -> None:
    """Free allocated memory.

    Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
    no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
    allocation function, or if the space has been deallocated by a call to `fimo_free()`,
    `fimo_free_sized()` or `fimo_free_aligned_sized()`, or if `size` does not match
    the size used to allocate the memory, the behavior is undefined.

    :param ptr: pointer to the memory
    :param size: size of the allocation
    """
    _fimo_free_sized(ptr, size)


_fimo_free_aligned_sized = _lib.fimo_free_aligned_sized
_fimo_free_aligned_sized.argtypes = [c.c_void_p, c.c_size_t, c.c_size_t]
_fimo_free_aligned_sized.restype = None


def fimo_free_aligned_sized(ptr: c.c_void_p, alignment: c.c_size_t, size: c.c_size_t) -> None:
    """Free allocated memory.

    Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
    no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
    allocation function, or if the space has been deallocated by a call to `fimo_free()`,
    `fimo_free_sized()` or `fimo_free_aligned_sized()`, or if `alignment` and `size`
    do not match the alignment and size used to allocate the memory, the behavior is undefined.

    :param ptr: pointer to the memory
    :param alignment: alignment of the allocation
    :param size: size of the allocation
    """
    _fimo_free_aligned_sized(ptr, alignment, size)


# Header: fimo_std/impl/integers/integers_base.h

class FimoI8(c.c_int8):
    """8-bit integer."""
    pass


class FimoI16(c.c_int16):
    """16-bit integer."""
    pass


class FimoI32(c.c_int32):
    """32-bit integer."""
    pass


class FimoI64(c.c_int64):
    """64-bit integer."""
    pass


# Not exactly the same as ctypes lacks a ptrdiff_t
class FimoISize(c.c_ssize_t):
    """Signed integer type resulting from subtracting two pointers."""
    pass


# Not exactly the same as ctypes lacks a intptr_t
class FimoIntPtr(c.c_ssize_t):
    """Signed integer type capable of containing any pointer."""
    pass


class FimoU8(c.c_uint8):
    """8-bit unsigned integer."""
    pass


class FimoU16(c.c_uint16):
    """16-bit unsigned integer."""
    pass


class FimoU32(c.c_uint32):
    """32-bit unsigned integer."""
    pass


class FimoU64(c.c_uint64):
    """64-bit unsigned integer."""
    pass


class FimoUSize(c.c_size_t):
    """Unsigned integer guaranteed to hold any array index."""
    pass


# Not exactly the same as ctypes lacks a uintptr_t
class FimoUIntPtr(c.c_size_t):
    """Unsigned integer type capable of containing any pointer."""
    pass


# Header: fimo_std/version.h

class FimoVersion(c.Structure):
    """A version specifier."""
    _fields_ = [("major", FimoU32),
                ("minor", FimoU32),
                ("patch", FimoU32),
                ("build", FimoU64)]


FimoVersionPtr = c.POINTER(FimoVersion)
"""Pointer to a `FimoVersion`"""

_fimo_version_parse_str = _lib.fimo_version_parse_str
_fimo_version_parse_str.argtypes = [c.c_char_p, c.c_size_t, FimoVersionPtr]
_fimo_version_parse_str.restype = FimoError


def fimo_version_parse_str(str: c.c_char_p, str_len: c.c_size_t, version: FimoVersionPtr) -> FimoError:
    """Parses a string into a `FimoVersion`.

    The string must be of the form "major.minor.patch" or "major.minor.patch+build".

    :param str: string to parse
    :param str_len: length of the string
    :param version: pointer to the parsed version.

    :return: Status code
    """
    return _fimo_version_parse_str(str, str_len, version)


_fimo_version_str_len = _lib.fimo_version_str_len
_fimo_version_str_len.argtypes = [FimoVersionPtr]
_fimo_version_str_len.restype = FimoUSize


def fimo_version_str_len(version: FimoVersionPtr) -> FimoUSize:
    """Calculates the string length required to represent the version as a string.

    If `version` is `NULL`, this function returns `0`. The returned length is
    large enough for a call to `fimo_version_write_str` with the same version
    instance. The returned length does not include the zero-terminator.

    :param version: version to check.

    :return: Required string length
    """
    return _fimo_version_str_len(version)


_fimo_version_str_len_full = _lib.fimo_version_str_len_full
_fimo_version_str_len_full.argtypes = [FimoVersionPtr]
_fimo_version_str_len_full.restype = FimoUSize


def fimo_version_str_len_full(version: FimoVersionPtr) -> FimoUSize:
    """Calculates the string length required to represent the version as a string.

    If `version` is `NULL`, this function returns `0`. The returned length is
    large enough for a call to `fimo_version_write_str_long` with the same
    version instance. The returned length does not include the zero-terminator.

    :param version: version to check.

    :return: Required string length
    """
    return _fimo_version_str_len_full(version)


_fimo_version_write_str = _lib.fimo_version_write_str
_fimo_version_write_str.argtypes = [FimoVersionPtr, c.c_char_p, c.c_size_t, c.POINTER(c.c_size_t)]
_fimo_version_write_str.restype = FimoError


def fimo_version_write_str(version: FimoVersionPtr, str: c.c_char_p, str_len: c.c_size_t,
                           written: c.POINTER(c.c_size_t)) -> FimoError:
    """Represents the version as a string.

    Writes a string of the form "major.minor.patch" into `str`. If `str` is
    large enough to store a zero-terminator, it is appended at the end of the
    written characters. If `written` is not `NULL`, it is set to the number of
    characters written without the zero-terminator.

    :param version: version to write out
    :param str: string to parse
    :param str_len: destination string length
    :param written: pointer to the written character count

    :return: Status code
    """
    return _fimo_version_write_str(version, str, str_len, written)


_fimo_version_write_str_long = _lib.fimo_version_write_str_long
_fimo_version_write_str_long.argtypes = [FimoVersionPtr, c.c_char_p, c.c_size_t, c.POINTER(c.c_size_t)]
_fimo_version_write_str_long.restype = FimoError


def fimo_version_write_str_long(version: FimoVersionPtr, str: c.c_char_p, str_len: c.c_size_t,
                                written: c.POINTER(c.c_size_t)) -> FimoError:
    """Represents the version as a string.

    Writes a string of the form "major.minor.patch+build" into `str`. If `str`
    is large enough to store a zero-terminator, it is appended at the end of the
    written characters. If `written` is not `NULL`, it is set to the number of
    characters written without the zero-terminator.

    :param version: version to write out
    :param str: string to parse
    :param str_len: destination string length
    :param written: pointer to the written character count

    :return: Status code
    """
    return _fimo_version_write_str_long(version, str, str_len, written)


_fimo_version_cmp = _lib.fimo_version_cmp
_fimo_version_cmp.argtypes = [FimoVersionPtr, FimoVersionPtr]
_fimo_version_cmp.restype = c.c_int


def fimo_version_cmp(lhs: FimoVersionPtr, rhs: FimoVersionPtr) -> int:
    """Compares two versions.

    Returns an ordering of the two versions, without taking into consideration
    the build numbers. Returns `-1` if `lhs < rhs`, `0` if `lhs == rhs`, or
    `1` if `lhs > rhs`.

    :param lhs: first version (not `NULL`)
    :param rhs: second version (not `NULL`)

    :return: Version ordering
    """
    return _fimo_version_cmp(lhs, rhs)


_fimo_version_cmp_long = _lib.fimo_version_cmp_long
_fimo_version_cmp_long.argtypes = [FimoVersionPtr, FimoVersionPtr]
_fimo_version_cmp_long.restype = c.c_int


def fimo_version_cmp_long(lhs: FimoVersionPtr, rhs: FimoVersionPtr) -> int:
    """Compares two versions.

    Returns an ordering of the two versions, taking into consideration the build
    numbers. Returns `-1` if `lhs < rhs`, `0` if `lhs == rhs`, or `1` if `lhs > rhs`.

    :param lhs: first version (not `NULL`)
    :param rhs: second version (not `NULL`)

    :return: Version ordering
    """
    return _fimo_version_cmp_long(lhs, rhs)


_fimo_version_compatible = _lib.fimo_version_compatible
_fimo_version_compatible.argtypes = [FimoVersionPtr, FimoVersionPtr]
_fimo_version_compatible.restype = c.c_bool


def fimo_version_compatible(got: FimoVersionPtr, required: FimoVersionPtr) -> bool:
    """Checks for the compatibility of two versions.

    If `got` is compatible with `required` it indicates that an object which is
    versioned with the version `got` can be used instead of an object of the
    same type carrying the version `required`.

    The compatibility of `got` with `required` is determined by the following
    algorithm:

        1. The major versions of `got` and `required` must be equal.
        2. If the major version is `0`, the minor versions must be equal.
        3. `got >= required` without the build number.

    :param got: version to check for compatibility
    :param required: required version

    :return: `true` if is compatible, `false` otherwise.
    """
    return _fimo_version_compatible(got, required)


# Header: fimo_std/time.h

class FimoDuration(c.Structure):
    """A span of time."""
    _fields_ = [("secs", FimoU64),
                ("nanos", FimoU32)]


class FimoTime(c.Structure):
    """A point in time since the unix epoch."""
    _fields_ = [("secs", FimoU64),
                ("nanos", FimoU32)]


class FimoTimeMonotonic(c.Structure):
    """A monotonic point in time.

    The starting point is undefined.
    """
    _fields_ = [("secs", FimoU64),
                ("nanos", FimoU32)]


FIMO_DURATION_ZERO = FimoDuration(0, 0)
"""The zero duration."""

FIMO_DURATION_MAX = FimoDuration((1 << 64) - 1, 999999999)
"""The maximum duration."""

FIMO_UNIX_EPOCH = FimoTime(0, 0)
"""The UNIX epoch."""

FIMO_TIME_MAX = FimoTime((1 << 64) - 1, 999999999)
"""Largest possible time point."""

_fimo_duration_zero = _lib.fimo_duration_zero
_fimo_duration_zero.argtypes = None
_fimo_duration_zero.restype = FimoDuration


def fimo_duration_zero() -> FimoDuration:
    """Constructs the zero duration.

    :return: Zero duration.
    """
    return _fimo_duration_zero()


_fimo_duration_max = _lib.fimo_duration_max
_fimo_duration_max.argtypes = None
_fimo_duration_max.restype = FimoDuration


def fimo_duration_max() -> FimoDuration:
    """Constructs the max duration.

    :return: Max duration.
    """
    return _fimo_duration_max()


_fimo_duration_from_seconds = _lib.fimo_duration_from_seconds
_fimo_duration_from_seconds.argtypes = [FimoU64]
_fimo_duration_from_seconds.restype = FimoDuration


def fimo_duration_from_seconds(seconds: FimoU64) -> FimoDuration:
    """Constructs a duration from seconds.

    :return: Duration.
    """
    return _fimo_duration_from_seconds(seconds)


_fimo_duration_from_millis = _lib.fimo_duration_from_millis
_fimo_duration_from_millis.argtypes = [FimoU64]
_fimo_duration_from_millis.restype = FimoDuration


def fimo_duration_from_millis(milliseconds: FimoU64) -> FimoDuration:
    """Constructs a duration from milliseconds.

    :return: Duration.
    """
    return _fimo_duration_from_millis(milliseconds)


_fimo_duration_from_nanos = _lib.fimo_duration_from_nanos
_fimo_duration_from_nanos.argtypes = [FimoU64]
_fimo_duration_from_nanos.restype = FimoDuration


def fimo_duration_from_nanos(nanoseconds: FimoU64) -> FimoDuration:
    """Constructs a duration from nanoseconds.

    :return: Duration.
    """
    return _fimo_duration_from_nanos(nanoseconds)


_fimo_duration_is_zero = _lib.fimo_duration_is_zero
_fimo_duration_is_zero.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_is_zero.restype = c.c_bool


def fimo_duration_is_zero(duration: c.POINTER(FimoDuration)) -> bool:
    """Checks if a duration is zero.

    :return: `true` if the duration is zero.
    """
    return _fimo_duration_is_zero(duration)


_fimo_duration_as_secs = _lib.fimo_duration_as_secs
_fimo_duration_as_secs.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_as_secs.restype = FimoU64


def fimo_duration_as_secs(duration: c.POINTER(FimoDuration)) -> FimoU64:
    """Returns the whole seconds in a duration.

    :return: Whole seconds.
    """
    return _fimo_duration_as_secs(duration)


_fimo_duration_subsec_millis = _lib.fimo_duration_subsec_millis
_fimo_duration_subsec_millis.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_subsec_millis.restype = FimoU32


def fimo_duration_subsec_millis(duration: c.POINTER(FimoDuration)) -> FimoU32:
    """Returns the fractional part in milliseconds.

    :return: Fractional part in whole milliseconds
    """
    return _fimo_duration_subsec_millis(duration)


_fimo_duration_subsec_micros = _lib.fimo_duration_subsec_micros
_fimo_duration_subsec_micros.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_subsec_micros.restype = FimoU32


def fimo_duration_subsec_micros(duration: c.POINTER(FimoDuration)) -> FimoU32:
    """Returns the fractional part in microseconds.

    :return: Fractional part in whole microseconds.
    """
    return _fimo_duration_subsec_micros(duration)


_fimo_duration_subsec_nanos = _lib.fimo_duration_subsec_nanos
_fimo_duration_subsec_nanos.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_subsec_nanos.restype = FimoU32


def fimo_duration_subsec_nanos(duration: c.POINTER(FimoDuration)) -> FimoU32:
    """Returns the fractional part in nanoseconds.

    :return: Fractional part in whole nanoseconds.
    """
    return _fimo_duration_subsec_nanos(duration)


_fimo_duration_as_millis = _lib.fimo_duration_as_millis
_fimo_duration_as_millis.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoU32)]
_fimo_duration_as_millis.restype = FimoU64


def fimo_duration_as_millis(duration: c.POINTER(FimoDuration), high: c.POINTER(FimoU32)) -> FimoU32:
    """Returns the whole milliseconds in a duration.

    If `high` is not null, it is set to store the overflow portion of the milliseconds.

    :return: Low part of the milliseconds.
    """
    return _fimo_duration_as_millis(duration, high)


_fimo_duration_as_micros = _lib.fimo_duration_as_micros
_fimo_duration_as_micros.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoU32)]
_fimo_duration_as_micros.restype = FimoU64


def fimo_duration_as_micros(duration: c.POINTER(FimoDuration), high: c.POINTER(FimoU32)) -> FimoU32:
    """Returns the whole microseconds in a duration.

    If `high` is not null, it is set to store the overflow portion of the microseconds.

    :return: Low part of the microseconds.
    """
    return _fimo_duration_as_micros(duration, high)


_fimo_duration_as_nanos = _lib.fimo_duration_as_nanos
_fimo_duration_as_nanos.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoU32)]
_fimo_duration_as_nanos.restype = FimoU64


def fimo_duration_as_nanos(duration: c.POINTER(FimoDuration), high: c.POINTER(FimoU32)) -> FimoU32:
    """Returns the whole nanoseconds in a duration.

    If `high` is not null, it is set to store the overflow portion of the nanoseconds.

    :return: Low part of the nanoseconds.
    """
    return _fimo_duration_as_nanos(duration, high)


_fimo_duration_add = _lib.fimo_duration_add
_fimo_duration_add.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoDuration), c.POINTER(FimoDuration)]
_fimo_duration_add.restype = FimoError


def fimo_duration_add(lhs: c.POINTER(FimoDuration), rhs: c.POINTER(FimoDuration),
                      out: c.POINTER(FimoDuration)) -> FimoError:
    """Adds two durations.

    :return: Status code.
    """
    return _fimo_duration_add(lhs, rhs, out)


_fimo_duration_saturating_add = _lib.fimo_duration_saturating_add
_fimo_duration_saturating_add.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoDuration)]
_fimo_duration_saturating_add.restype = FimoDuration


def fimo_duration_saturating_add(lhs: c.POINTER(FimoDuration), rhs: c.POINTER(FimoDuration)) -> FimoDuration:
    """Adds two durations.

    The result saturates to `FIMO_DURATION_MAX`, if an overflow occurs.

    :return: Added durations.
    """
    return _fimo_duration_saturating_add(lhs, rhs)


_fimo_duration_sub = _lib.fimo_duration_sub
_fimo_duration_sub.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoDuration), c.POINTER(FimoDuration)]
_fimo_duration_sub.restype = FimoError


def fimo_duration_sub(lhs: c.POINTER(FimoDuration), rhs: c.POINTER(FimoDuration),
                      out: c.POINTER(FimoDuration)) -> FimoError:
    """Subtracts two durations.

    :return: Status code.
    """
    return _fimo_duration_sub(lhs, rhs, out)


_fimo_duration_saturating_sub = _lib.fimo_duration_saturating_sub
_fimo_duration_saturating_sub.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoDuration)]
_fimo_duration_saturating_sub.restype = FimoDuration


def fimo_duration_saturating_sub(lhs: c.POINTER(FimoDuration), rhs: c.POINTER(FimoDuration)) -> FimoDuration:
    """Subtracts two durations.

    The result saturates to `FIMO_DURATION_ZERO`, if an overflow occurs or the resulting duration is negative.

    :return: Subtracted durations.
    """
    return _fimo_duration_saturating_sub(lhs, rhs)


_fimo_time_now = _lib.fimo_time_now
_fimo_time_now.argtypes = None
_fimo_time_now.restype = FimoTime


def fimo_time_now() -> FimoTime:
    """Returns the current time.

    :return: Current time.
    """
    return _fimo_time_now()


_fimo_time_elapsed = _lib.fimo_time_elapsed
_fimo_time_elapsed.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration)]
_fimo_time_elapsed.restype = FimoError


def fimo_time_elapsed(time_point: c.POINTER(FimoTime), elapsed: c.POINTER(FimoDuration)) -> FimoError:
    """Returns the duration elapsed since a prior time point.

    :return: Status code.
    """
    return _fimo_time_elapsed(time_point, elapsed)


_fimo_time_duration_since = _lib.fimo_time_duration_since
_fimo_time_duration_since.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoTime), c.POINTER(FimoDuration)]
_fimo_time_duration_since.restype = FimoError


def fimo_time_duration_since(time_point: c.POINTER(FimoTime), earlier_time_point: c.POINTER(FimoTime),
                             duration: c.POINTER(FimoDuration)) -> FimoError:
    """Returns the difference between two time points.

    :return: Status code.
    """
    return _fimo_time_duration_since(time_point, earlier_time_point, duration)


_fimo_time_add = _lib.fimo_time_add
_fimo_time_add.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration), c.POINTER(FimoTime)]
_fimo_time_add.restype = FimoError


def fimo_time_add(time_point: c.POINTER(FimoTime), duration: c.POINTER(FimoDuration),
                  out: c.POINTER(FimoTime)) -> FimoError:
    """Adds a duration to a time point.

    :return: Status code.
    """
    return _fimo_time_add(time_point, duration, out)


_fimo_time_saturating_add = _lib.fimo_time_saturating_add
_fimo_time_saturating_add.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration)]
_fimo_time_saturating_add.restype = FimoTime


def fimo_time_saturating_add(time_point: c.POINTER(FimoTime), duration: c.POINTER(FimoDuration)) -> FimoTime:
    """Adds a duration to a time point.

    The result saturates to `FIMO_TIME_MAX`, if an overflow occurs.

    :return: Status code.
    """
    return _fimo_time_saturating_add(time_point, duration)


_fimo_time_sub = _lib.fimo_time_sub
_fimo_time_sub.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration), c.POINTER(FimoTime)]
_fimo_time_sub.restype = FimoError


def fimo_time_sub(time_point: c.POINTER(FimoTime), duration: c.POINTER(FimoDuration),
                  out: c.POINTER(FimoTime)) -> FimoError:
    """Subtracts a duration from a time point.

    :return: Status code.
    """
    return _fimo_time_sub(time_point, duration, out)


_fimo_time_saturating_sub = _lib.fimo_time_saturating_sub
_fimo_time_saturating_sub.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration)]
_fimo_time_saturating_sub.restype = FimoTime


def fimo_time_saturating_sub(time_point: c.POINTER(FimoTime), duration: c.POINTER(FimoDuration)) -> FimoTime:
    """Subtracts a duration from a time point.

    The result saturates to `FIMO_UNIX_EPOCH`, if an overflow occurs or the resulting duration is negative.

    :return: Status code.
    """
    return _fimo_time_saturating_sub(time_point, duration)


# Header: fimo_std/context.h

class FimoContext(c.Structure):
    """Context of the fimo std."""
    _fields_ = [("data", c.c_void_p),
                ("vtable", c.c_void_p)]


FimoContextPtr = c.POINTER(FimoContext)
"""Pointer to a `FimoContext`."""


class FimoStructType(c.c_int):
    """Fimo std structure types."""
    FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG = 0
    FIMO_STRUCT_TYPE_TRACING_METADATA = 1
    FIMO_STRUCT_TYPE_TRACING_SPAN_DESC = 2
    FIMO_STRUCT_TYPE_TRACING_SPAN = 3
    FIMO_STRUCT_TYPE_TRACING_EVENT = 4
    FIMO_STRUCT_TYPE_TRACING_SUBSCRIBER = 5
    FIMO_STRUCT_TYPE_MODULE_EXPORT = 6
    FIMO_STRUCT_TYPE_MODULE_INFO = 7


class FimoBaseStructIn(c.Structure):
    """Base structure for a read-only pointer chain."""
    pass


FimoBaseStructIn._fields_ = [("type", FimoStructType),
                             ("next", c.POINTER(FimoBaseStructIn))]


class FimoBaseStructOut(c.Structure):
    """Base structure for a pointer chain."""
    pass


FimoBaseStructOut._fields_ = [("type", FimoStructType),
                              ("next", c.POINTER(FimoBaseStructOut))]


class FimoContextVTableHeader(c.Structure):
    """Header of all VTables of a `FimoContext`, for all future versions.

    May never be changed, since we rely on it to determine whether a
    given `FimoContext` instance is compatible with the definitions
    available to us.
    """
    _fields_ = [("check_version", c.CFUNCTYPE(FimoError, c.c_void_p, FimoVersionPtr))]


class FimoContextVTableV0(c.Structure):
    """Core VTable of a `FimoContext`.

    Changing the VTable is a breaking change.
    """
    _fields_ = [("acquire", c.CFUNCTYPE(None, c.c_void_p)),
                ("release", c.CFUNCTYPE(None, c.c_void_p))]


_fimo_context_init = _lib.fimo_context_init
_fimo_context_init.argtypes = [c.POINTER(c.POINTER(FimoBaseStructIn)), FimoContextPtr]
_fimo_context_init.restype = FimoError


def fimo_context_init(options: c.POINTER(c.POINTER(FimoBaseStructIn)), context: FimoContextPtr) -> FimoError:
    """Initializes a new context with the given options.

    If `options` is `NULL`, the context is initialized with the default options,
    otherwise `options` must be an array terminated with a `NULL` element. The
    initialized context is written to `context`. In case of an error, this function
    cleans up the configuration options.

    :param options: init options
    :param context: pointer to the context (not `NULL`)

    :return: Status code.
    """
    return _fimo_context_init(options, context)


_fimo_context_check_version = _lib.fimo_context_check_version
_fimo_context_check_version.argtypes = [FimoContext]
_fimo_context_check_version.restype = FimoError


def fimo_context_check_version(context: FimoContext) -> FimoError:
    """Checks the compatibility of the context version.

    This function must be called upon the acquisition of a context, that
    was not created locally, e.g., when being passed a context from
    another shared library. Failure of doing so, may cause undefined
    behavior, if the context is later utilized.

    :param context: the context

    :return: Status code.
    """
    return _fimo_context_check_version(context)


_fimo_context_acquire = _lib.fimo_context_acquire
_fimo_context_acquire.argtypes = [FimoContext]
_fimo_context_acquire.restype = None


def fimo_context_acquire(context: FimoContext) -> None:
    """Acquires a reference to the context.

    Increases the reference count of the context. May abort the program,
    if doing so is not possible. May only be called with a valid reference
    to the context.

    :param context: the context
    """
    _fimo_context_acquire(context)


_fimo_context_release = _lib.fimo_context_release
_fimo_context_release.argtypes = [FimoContext]
_fimo_context_release.restype = None


def fimo_context_release(context: FimoContext) -> None:
    """Releases a reference to the context.

    Decrements the reference count of the context. When the reference count
    reaches zero, this function also destroys the reference. May only be
    called with a valid reference to the context.

    :param context: the context
    """
    _fimo_context_release(context)


# Header: fimo_std/tracing.h

class FimoTracingCallStack(c.Structure):
    """A call stack.

    Each call stack represents a unit of computation, like a thread.
    A call stack is active on only one thread at any given time. The
    active call stack of a thread can be swapped, which is useful
    for tracing where a `M:N` threading model is used. In that case,
    one would create one stack for each task, and activate it when
    the task is resumed.
    """
    pass


class FimoTracingLevel(c.c_int):
    """Possible tracing levels.

    The levels are ordered such that given two levels `lvl1` and `lvl2`,
    where `lvl1 >= lvl2`, then an event with level `lvl2` will be traced
    in a context where the maximum tracing level is `lvl1`.
    """
    pass


class FimoTracingMetadata(c.Structure):
    """Metadata for a span/event."""
    _fields_ = [("type", FimoStructType),
                ("next", c.POINTER(FimoBaseStructIn)),
                ("name", c.c_char_p),
                ("target", c.c_char_p),
                ("level", FimoTracingLevel),
                ("file_name", c.c_char_p),
                ("line_number", FimoI32)]


FimoTracingMetadataPtr = c.POINTER(FimoTracingMetadata)
"""Pointer to a `FimoTracingMetadata`."""


class FimoTracingSpanDesc(c.Structure):
    """Descriptor of a new span."""
    _fields_ = [("type", FimoStructType),
                ("next", c.POINTER(FimoBaseStructIn)),
                ("metadata", FimoTracingMetadataPtr)]


FimoTracingSpanDescPtr = c.POINTER(FimoTracingSpanDesc)
"""Pointer to a `FimoTracingSpanDesc`."""


class FimoTracingSpan(c.Structure):
    """A period of time, during which events can occur."""
    _fields_ = [("type", FimoStructType),
                ("next", c.POINTER(FimoBaseStructOut))]


class FimoTracingEvent(c.Structure):
    """An event to be traced."""
    _fields_ = [("type", FimoStructType),
                ("next", c.POINTER(FimoBaseStructIn)),
                ("metadata", FimoTracingMetadataPtr)]


FimoTracingEventPtr = c.POINTER(FimoTracingEvent)
"""Pointer to a `FimoTracingEvent`."""

FimoTracingFormat = c.CFUNCTYPE(FimoError, c.POINTER(c.c_char), FimoUSize, c.c_void_p, c.POINTER(FimoUSize))
"""Signature of a message formatter.

It is not an error to format only a part of the message.

:param arg0: destination buffer
:param arg1: destination buffer size
:param arg2: data to format
:param arg3: number of written bytes of the formatter

:return: Status code.
"""


class FimoTracingSubscriberVTable(c.Structure):
    """VTable of a tracing subscriber.

    Adding/removing functionality to a subscriber through this table
    is a breaking change, as a subscriber may be implemented from
    outside the library.
    """
    _fields_ = [("destroy", c.CFUNCTYPE(None, c.c_void_p)),
                ("call_stack_create", c.CFUNCTYPE(FimoError, c.c_void_p, c.POINTER(FimoTime), c.POINTER(c.c_void_p))),
                ("call_stack_drop", c.CFUNCTYPE(None, c.c_void_p, c.c_void_p)),
                ("call_stack_destroy", c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p)),
                ("call_stack_unblock", c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p)),
                ("call_stack_suspend", c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p,
                                                   c.c_bool)),
                ("call_stack_resume", c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p)),
                ("span_push", c.CFUNCTYPE(FimoError, c.c_void_p, c.POINTER(FimoTime), c.POINTER(FimoTracingSpanDesc),
                                          c.POINTER(c.c_char), FimoUSize, c.c_void_p)),
                ("span_drop", c.CFUNCTYPE(None, c.c_void_p, c.c_void_p)),
                ("span_pop", c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p)),
                ("event_emit", c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p,
                                           c.POINTER(FimoTracingEvent), c.POINTER(c.c_char), FimoUSize)),
                ("flush", c.CFUNCTYPE(None, c.c_void_p))]


class FimoTracingSubscriber(c.Structure):
    """A subscriber for tracing events.

    The main function of the tracing backend is managing and routing
    tracing events to subscribers. Therefore, it does not consume any
    events on its own, which is the task of the subscribers. Subscribers
    may utilize the events in any way they deem fit.
    """
    _fields_ = [("type", FimoStructType),
                ("next", c.POINTER(FimoBaseStructIn)),
                ("ptr", c.c_void_p),
                ("vtable", c.POINTER(FimoTracingSubscriberVTable))]


FIMO_TRACING_DEFAULT_SUBSCRIBER = FimoTracingSubscriber.in_dll(_lib, "FIMO_TRACING_DEFAULT_SUBSCRIBER")
"""Default subscriber."""


class FimoTracingCreationConfig(c.Structure):
    """Configuration for the tracing backend.

    Can be passed when creating the context.
    """
    _fields_ = [("type", FimoStructType),
                ("next", c.POINTER(FimoBaseStructIn)),
                ("format_buffer_size", FimoUSize),
                ("maximum_level", FimoTracingLevel),
                ("subscribers", c.POINTER(FimoTracingSubscriber)),
                ("subscriber_count", FimoUSize)]


class FimoTracingVTableV0(c.Structure):
    """VTable of the tracing subsystem.

    Changing the VTable is a breaking change.
    """
    _fields_ = [("call_stack_create", c.CFUNCTYPE(FimoError, c.c_void_p, c.POINTER(c.POINTER(FimoTracingCallStack)))),
                ("call_stack_destroy", c.CFUNCTYPE(FimoError, c.c_void_p, c.POINTER(FimoTracingCallStack))),
                ("call_stack_switch", c.CFUNCTYPE(FimoError, c.c_void_p, c.POINTER(FimoTracingCallStack),
                                                  c.POINTER(c.POINTER(FimoTracingCallStack)))),
                ("call_stack_unblock", c.CFUNCTYPE(FimoError, c.c_void_p, c.POINTER(FimoTracingCallStack))),
                ("call_stack_suspend_current", c.CFUNCTYPE(FimoError, c.c_void_p, c.c_bool)),
                ("call_stack_resume_current", c.CFUNCTYPE(FimoError, c.c_void_p)),
                ("span_create", c.CFUNCTYPE(FimoError, c.c_void_p, FimoTracingSpanDescPtr,
                                            c.POINTER(c.POINTER(FimoTracingSpan)), FimoTracingFormat, c.c_void_p)),
                ("span_destroy", c.CFUNCTYPE(FimoError, c.c_void_p, c.POINTER(FimoTracingSpan))),
                ("event_emit", c.CFUNCTYPE(FimoError, c.c_void_p, FimoTracingEventPtr, FimoTracingFormat,
                                           c.c_void_p)),
                ("is_enabled", c.CFUNCTYPE(c.c_bool, c.c_void_p)),
                ("register_thread", c.CFUNCTYPE(FimoError, c.c_void_p)),
                ("unregister_thread", c.CFUNCTYPE(FimoError, c.c_void_p)),
                ("flush", c.CFUNCTYPE(FimoError, c.c_void_p))]


_fimo_tracing_call_stack_create = _lib.fimo_tracing_call_stack_create
_fimo_tracing_call_stack_create.argtypes = [FimoContext, c.POINTER(c.POINTER(FimoTracingCallStack))]
_fimo_tracing_call_stack_create.restype = FimoError


def fimo_tracing_call_stack_create(context: FimoContext,
                                   call_stack: c.POINTER(c.POINTER(FimoTracingCallStack))) -> FimoError:
    """Creates a new empty call stack.

    If successful, the new call stack is marked as suspended, and written
    into `call_stack`. The new call stack is not set to be the active call
    stack.

    :param context: the context
    :param call_stack: pointer to the resulting call stack

    :return: Status code.
    """
    return _fimo_tracing_call_stack_create(context, call_stack)


_fimo_tracing_call_stack_destroy = _lib.fimo_tracing_call_stack_destroy
_fimo_tracing_call_stack_destroy.argtypes = [FimoContext, c.POINTER(FimoTracingCallStack)]
_fimo_tracing_call_stack_destroy.restype = FimoError


def fimo_tracing_call_stack_destroy(context: FimoContext, call_stack: c.POINTER(FimoTracingCallStack)) -> FimoError:
    """Destroys an empty call stack.

    Marks the completion of a task. Before calling this function, the
    call stack must be empty, i.e., there must be no active spans on
    the stack, and must not be active. If successful, the call stack
    may not be used afterward. The active call stack of the thread
    is destroyed automatically, on thread exit or during destruction
    of `context`. The caller must own the call stack uniquely.

    :param context: the context
    :param call_stack: the call stack to destroy

    :return: Status code.
    """
    return _fimo_tracing_call_stack_destroy(context, call_stack)


_fimo_tracing_call_stack_switch = _lib.fimo_tracing_call_stack_switch
_fimo_tracing_call_stack_switch.argtypes = [FimoContext, c.POINTER(FimoTracingCallStack),
                                            c.POINTER(c.POINTER(FimoTracingCallStack))]
_fimo_tracing_call_stack_switch.restype = FimoError


def fimo_tracing_call_stack_switch(context: FimoContext, call_stack: c.POINTER(FimoTracingCallStack),
                                   old: c.POINTER(c.POINTER(FimoTracingCallStack))) -> FimoError:
    """Switches the call stack of the current thread.

    If successful, `new_call_stack` will be used as the active call
    stack of the calling thread. The old call stack is written into
    `old`, enabling the caller to switch back to it afterward.
    `call_stack` must be in a suspended, but unblocked, state and not be
    active. The active call stack must also be in a suspended state, but may
    also be blocked.

    This function may return `FIMO_ENOTSUP`, if the current thread is not
    registered with the subsystem.

    :param context: the context
    :param call_stack: new call stack
    :param old: location to store the old call stack into

    :return: Status code.
    """
    return _fimo_tracing_call_stack_switch(context, call_stack, old)


_fimo_tracing_call_stack_unblock = _lib.fimo_tracing_call_stack_unblock
_fimo_tracing_call_stack_unblock.argtypes = [FimoContext, c.POINTER(FimoTracingCallStack)]
_fimo_tracing_call_stack_unblock.restype = FimoError


def fimo_tracing_call_stack_unblock(context: FimoContext, call_stack: c.POINTER(FimoTracingCallStack)) -> FimoError:
    """Unblocks a blocked call stack.

    Once unblocked, the call stack may be resumed. The call stack
    may not be active and must be marked as blocked.

    :param context: the context
    :param call_stack: the call stack to unblock

    :return: Status code.
    """
    return _fimo_tracing_call_stack_unblock(context, call_stack)


_fimo_tracing_call_stack_suspend_current = _lib.fimo_tracing_call_stack_suspend_current
_fimo_tracing_call_stack_suspend_current.argtypes = [FimoContext, c.c_bool]
_fimo_tracing_call_stack_suspend_current.restype = FimoError


def fimo_tracing_call_stack_suspend_current(context: FimoContext, block: c.c_bool) -> FimoError:
    """Marks the current call stack as being suspended.

    While suspended, the call stack can not be utilized for tracing
    messages. The call stack optionally also be marked as being
    blocked. In that case, the call stack must be unblocked prior
    to resumption.

    This function may return `FIMO_ENOTSUP`, if the current thread is not
    registered with the subsystem.

    :param context: the context
    :param block: whether to mark the call stack as blocked

    :return: Status code.
    """
    return _fimo_tracing_call_stack_suspend_current(context, block)


_fimo_tracing_call_stack_resume_current = _lib.fimo_tracing_call_stack_resume_current
_fimo_tracing_call_stack_resume_current.argtypes = [FimoContext]
_fimo_tracing_call_stack_resume_current.restype = FimoError


def fimo_tracing_call_stack_resume_current(context: FimoContext) -> FimoError:
    """Marks the current call stack as being resumed.

    Once resumed, the context can be used to trace messages. To be
    successful, the current call stack must be suspended and unblocked.

    This function may return `FIMO_ENOTSUP`, if the current thread is not
    registered with the subsystem.

    :param context: the context

    :return: Status code.
    """
    return _fimo_tracing_call_stack_resume_current(context)


_fimo_tracing_span_create_fmt = _lib.fimo_tracing_span_create_fmt
_fimo_tracing_span_create_fmt.argtypes = [FimoContext, FimoTracingSpanDescPtr, c.POINTER(c.POINTER(FimoTracingSpan)),
                                          c.c_char_p]
_fimo_tracing_span_create_fmt.restype = FimoError


def fimo_tracing_span_create_fmt(context: FimoContext, span_desc: FimoTracingSpanDescPtr,
                                 span: c.POINTER(c.POINTER(FimoTracingSpan)), format: c.c_char_p, *args) -> FimoError:
    """Creates a new span with the standard formatter and enters it.

    If successful, the newly created span is used as the context for
    succeeding events. The message is formatted as if it were
    formatted by a call to `snprintf`. The message may be cut of,
    if the length exceeds the internal formatting buffer size.  The
    contents of `span_desc` must remain valid until the span is destroyed.

    This function may return `FIMO_ENOTSUP`, if the current thread is not
    registered with the subsystem.

    :param context: the context
    :param span_desc: descriptor of the new span
    :param span: pointer to the resulting span
    :param format: formatting string
    :param args: format args

    :return: Status code.
    """
    return _fimo_tracing_span_create_fmt(context, span_desc, span, format, *args)


_fimo_tracing_span_create_custom = _lib.fimo_tracing_span_create_custom
_fimo_tracing_span_create_custom.argtypes = [FimoContext, FimoTracingSpanDescPtr, c.POINTER(c.POINTER(FimoTracingSpan)),
                                             FimoTracingFormat, c.c_void_p]
_fimo_tracing_span_create_custom.restype = FimoError


def fimo_tracing_span_create_custom(context: FimoContext, span_desc: FimoTracingSpanDescPtr,
                                    span: c.POINTER(c.POINTER(FimoTracingSpan)), format: FimoTracingFormat,
                                    data: c.c_void_p) -> FimoError:
    """Creates a new span with a custom formatter and enters it.

    If successful, the newly created span is used as the context for
    succeeding events. The backend may use a formatting buffer of a
    fixed size. The formatter is expected to cut-of the message after
    reaching that specified size. The contents of `span_desc` must
    remain valid until the span is destroyed.

    This function may return `FIMO_ENOTSUP`, if the current thread is not
    registered with the subsystem.

    :param context: the context
    :param span_desc: descriptor of the new span
    :param span: pointer to the resulting span
    :param format: custom formatting function
    :param data: custom formatting data

    :return: Status code.
    """
    return _fimo_tracing_span_create_custom(context, span_desc, span, format, data)


_fimo_tracing_span_destroy = _lib.fimo_tracing_span_destroy
_fimo_tracing_span_destroy.argtypes = [FimoContext, c.POINTER(FimoTracingSpan)]
_fimo_tracing_span_destroy.restype = FimoError


def fimo_tracing_span_destroy(context: FimoContext, span: c.POINTER(FimoTracingSpan)) -> FimoError:
    """Exits and destroys a span.

    If successful, succeeding events won't occur inside the context of the
    exited span anymore. `span` must be the span at the top of the current
    call stack. The span may not be in use prior to a call to this function,
    and may not be used afterward.

    This function may return `FIMO_ENOTSUP`, if the current thread is not
    registered with the subsystem.

    :param context: the context
    :param span: the span to destroy

    :return: Status code.
    """
    return _fimo_tracing_span_destroy(context, span)


_fimo_tracing_event_emit_fmt = _lib.fimo_tracing_event_emit_fmt
_fimo_tracing_event_emit_fmt.argtypes = [FimoContext, FimoTracingEventPtr, c.c_char_p]
_fimo_tracing_event_emit_fmt.restype = FimoError


def fimo_tracing_event_emit_fmt(context: FimoContext, event: FimoTracingEventPtr, format: c.c_char_p,
                                *args) -> FimoError:
    """Emits a new event with the standard formatter.

    The message is formatted as if it were formatted by a call to `snprintf`.
    The message may be cut of, if the length exceeds the internal formatting
    buffer size.

    :param context: the context
    :param event: the event to emit
    :param format: formatting string
    :param args: format args

    :return: Status code.
    """
    return _fimo_tracing_event_emit_fmt(context, event, format, *args)


_fimo_tracing_event_emit_custom = _lib.fimo_tracing_event_emit_custom
_fimo_tracing_event_emit_custom.argtypes = [FimoContext, FimoTracingEventPtr, FimoTracingFormat, c.c_void_p]
_fimo_tracing_event_emit_custom.restype = FimoError


def fimo_tracing_event_emit_custom(context: FimoContext, event: FimoTracingEventPtr, format: FimoTracingFormat,
                                   data: c.c_void_p) -> FimoError:
    """Emits a new event with a custom formatter.

    The backend may use a formatting buffer of a fixed size. The formatter is
    expected to cut-of the message after reaching that specified size.

    :param context: the context
    :param event: the event to emit
    :param format: custom formatting function
    :param data: custom data to format

    :return: Status code.
    """
    return _fimo_tracing_event_emit_custom(context, event, format, data)


_fimo_tracing_is_enabled = _lib.fimo_tracing_is_enabled
_fimo_tracing_is_enabled.argtypes = [FimoContext]
_fimo_tracing_is_enabled.restype = c.c_bool


def fimo_tracing_is_enabled(context: FimoContext) -> bool:
    """Checks whether the tracing subsystem is enabled.

    This function can be used to check whether to call into the subsystem at all.
    Calling this function is not necessary, as the remaining functions of the
    subsystem are guaranteed to return default values, in case the subsystem is
    disabled.

    :param context: the context

    :return: `True` if the subsystem is enabled.
    """
    return _fimo_tracing_is_enabled(context)


_fimo_tracing_register_thread = _lib.fimo_tracing_register_thread
_fimo_tracing_register_thread.argtypes = [FimoContext]
_fimo_tracing_register_thread.restype = FimoError


def fimo_tracing_register_thread(context: FimoContext) -> FimoError:
    """Registers the calling thread with the tracing backend.

    The tracing of the backend is opt-in on a per-thread basis, where
    unregistered threads will behave as if the backend was disabled.
    Once registered, the calling thread gains access to the tracing
    backend and is assigned a new empty call stack. A registered
    thread must be unregistered from the tracing backend before the
    context is destroyed, by terminating the tread, or by manually
    calling `fimo_tracing_unregister_thread()`.

    :param context: the context

    :return: Status code.
    """
    return _fimo_tracing_register_thread(context)


_fimo_tracing_unregister_thread = _lib.fimo_tracing_unregister_thread
_fimo_tracing_unregister_thread.argtypes = [FimoContext]
_fimo_tracing_unregister_thread.restype = FimoError


def fimo_tracing_unregister_thread(context: FimoContext) -> FimoError:
    """Unregisters the calling thread from the tracing backend.

    Once unregistered, the calling thread looses access to the tracing
    backend until it is registered again. The thread can not be unregistered
    until the call stack is empty.

    :param context: the context

    :return: Status code.
    """
    return _fimo_tracing_unregister_thread(context)


_fimo_tracing_flush = _lib.fimo_tracing_flush
_fimo_tracing_flush.argtypes = [FimoContext]
_fimo_tracing_flush.restype = FimoError


def fimo_tracing_flush(context: FimoContext) -> FimoError:
    """Flushes the streams used for tracing.

    If successful, any unwritten data is written out by the individual subscribers.

    :param context: the context

    :return: Status code.
    """
    return _fimo_tracing_flush(context)
