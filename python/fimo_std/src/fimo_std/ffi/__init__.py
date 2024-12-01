from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Generic, TypeVar, Self, TYPE_CHECKING
import ctypes as c
import platform
import os

from . import ctypes_patch as cpatch

if TYPE_CHECKING:
    T = TypeVar("T", bound=c._CData)
    Pointer = c._Pointer[T]
    Ref = c._Pointer[T] | c.Array[T] | c._CArgObject | None
    PtrRef = c._Pointer[c._Pointer[T]] | c.Array[c._Pointer[T]] | c._CArgObject | None
    FuncPointer = c._FuncPointer
else:
    T = TypeVar("T")

    class Pointer(Generic[T], c._Pointer): ...

    class Ref(Generic[T], c._Pointer): ...

    class PtrRef(Generic[T], c._Pointer): ...

    class FuncPointer: ...


FfiType = TypeVar("FfiType")
FfiTypeView = TypeVar("FfiTypeView")


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


class FimoErrorCode(c.c_int):
    """Posix error codes"""

    pass


if platform.system() == "Windows":
    import ctypes.wintypes

    class FimoSystemErrorCode(c.wintypes.DWORD):
        """A system error code."""

else:

    class FimoSystemErrorCode(c.c_int):  # type: ignore[no-redef]
        """A system error code."""


@cpatch.make_callback_returnable
class FimoResultString(c.Structure):
    """An owned string returned from a `FimoResult`"""

    _fields_ = [
        ("str", c.POINTER(c.c_char)),
        ("release", c.CFUNCTYPE(None, c.POINTER(c.c_char))),
    ]


class FimoResultVTableV0(c.Structure):
    """Core VTable of a `FimoResult`.

    Changing the VTable is a breaking change.
    """

    _fields_ = [
        ("release", c.CFUNCTYPE(None, c.c_void_p)),
        (
            "error_name",
            c.CFUNCTYPE(FimoResultString, c.c_void_p),
        ),
        (
            "error_description",
            c.CFUNCTYPE(FimoResultString, c.c_void_p),
        ),
    ]


class FimoResultVTable(c.Structure):
    """VTable of a `FimoResult`."""

    _fields_ = [("v0", FimoResultVTableV0)]


@cpatch.make_callback_returnable
class FimoResult(c.Structure):
    """Status of an operation."""

    _fields_ = [("data", c.c_void_p), ("vtable", c.POINTER(FimoResultVTable))]


FIMO_IMPL_RESULT_STATIC_STRING_VTABLE = FimoResultVTable.in_dll(
    _lib, "FIMO_IMPL_RESULT_STATIC_STRING_VTABLE"
)
"""VTable for a `FimoResult` containing a static string."""


FIMO_IMPL_RESULT_DYNAMIC_STRING_VTABLE = FimoResultVTable.in_dll(
    _lib, "FIMO_IMPL_RESULT_DYNAMIC_STRING_VTABLE"
)
"""VTable for a `FimoResult` containing a dynamic string."""


FIMO_IMPL_RESULT_ERROR_CODE_VTABLE = FimoResultVTable.in_dll(
    _lib, "FIMO_IMPL_RESULT_ERROR_CODE_VTABLE"
)
"""VTable for a `FimoResult` containing a `FimoErrorCode`."""


FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE = FimoResultVTable.in_dll(
    _lib, "FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE"
)
"""VTable for a `FimoResult` containing a `FimoSystemErrorCode`."""


FIMO_IMPL_RESULT_OK = FimoResult.in_dll(_lib, "FIMO_IMPL_RESULT_OK")
"""A result indicating that no error occurred."""


FIMO_IMPL_RESULT_INVALID_ERROR = FimoResult.in_dll(
    _lib, "FIMO_IMPL_RESULT_INVALID_ERROR"
)
"""A result indicating the failed construction of a `FimoResult`."""


FIMO_IMPL_RESULT_OK_NAME = FimoResultString.in_dll(_lib, "FIMO_IMPL_RESULT_OK_NAME")
"""Name of the `FIMO_IMPL_RESULT_OK` result."""


FIMO_IMPL_RESULT_OK_DESCRIPTION = FimoResultString.in_dll(
    _lib, "FIMO_IMPL_RESULT_OK_DESCRIPTION"
)
"""Description of the `FIMO_IMPL_RESULT_OK` result."""


_fimo_error_code_name = _lib.fimo_error_code_name
_fimo_error_code_name.argtypes = [FimoErrorCode]
_fimo_error_code_name.restype = c.c_char_p


def fimo_error_code_name(errnum: FimoErrorCode) -> bytes:
    """Get the name of the error code.

    In case of an unknown error this returns `"FIMO_ERROR_CODE_UNKNOWN"`.
    """
    return _fimo_error_code_name(errnum)


_fimo_error_code_description = _lib.fimo_error_code_description
_fimo_error_code_description.argtypes = [FimoErrorCode]
_fimo_error_code_description.restype = c.c_char_p


def fimo_error_code_description(errnum: FimoErrorCode) -> bytes:
    """Get the description of the error code.

    In case of an unknown error this returns `"unknown error code"`.
    """
    return _fimo_error_code_description(errnum)


_fimo_error_code_from_errno = _lib.fimo_error_code_from_errno
_fimo_error_code_from_errno.argtypes = [FimoErrorCode]
_fimo_error_code_from_errno.restype = c.c_char_p


def fimo_error_code_from_errno(errnum: c.c_int) -> FimoErrorCode:
    """Constructs an error code from an errno error code.

    Unknown errno codes translate to an invalid error code.
    """
    return _fimo_error_code_from_errno(errnum)


def fimo_result_string_release(str: FimoResultString) -> None:
    """Releases a `FimoResultString`."""
    if str.release:
        str.release(str.str)


def fimo_result_from_static_string(error: c.c_char_p) -> FimoResult:
    """Constructs a `FimoResult` from a static string."""
    if not error:
        return FIMO_IMPL_RESULT_INVALID_ERROR

    return FimoResult(
        c.cast(error, c.c_void_p), c.pointer(FIMO_IMPL_RESULT_STATIC_STRING_VTABLE)
    )


def fimo_result_from_dynamic_string(error: c.c_char_p) -> FimoResult:
    """Constructs a `FimoResult` from a dynamic string."""
    if not error:
        return FIMO_IMPL_RESULT_INVALID_ERROR

    return FimoResult(
        c.cast(error, c.c_void_p), c.pointer(FIMO_IMPL_RESULT_DYNAMIC_STRING_VTABLE)
    )


def fimo_result_from_error_code(error: FimoErrorCode) -> FimoResult:
    """Constructs a `FimoResult` from a `FimoErrorCode`."""
    value = error.value
    if value == 0:
        return FIMO_IMPL_RESULT_OK
    elif value > 127:
        return FIMO_IMPL_RESULT_INVALID_ERROR

    return FimoResult(
        c.c_void_p(error.value), c.pointer(FIMO_IMPL_RESULT_ERROR_CODE_VTABLE)
    )


def fimo_result_from_system_error_code(error: FimoSystemErrorCode) -> FimoResult:
    """Constructs a `FimoResult` from a `FimoSystemErrorCode`."""
    return FimoResult(
        c.c_void_p(error.value), c.pointer(FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE)
    )


def fimo_result_is_error(result: FimoResult) -> bool:
    """Checks whether the `FimoResult` signifies an error."""
    return bool(result.vtable)


def fimo_result_is_ok(result: FimoResult) -> bool:
    """Checks whether the `FimoResult` does not signify an error."""
    return not bool(result.vtable)


def fimo_result_release(result: FimoResult) -> None:
    """Releases the `FimoResult`."""
    if fimo_result_is_error(result):
        release = result.vtable.contents.v0.release
        if release:
            release(result.data)


def fimo_result_error_name(result: FimoResult) -> FimoResultString:
    """Get the error name contained in the `FimoResult`.

    In case `result` does not contain an error this returns `"FIMO_IMPL_RESULT_OK_NAME"`.
    """
    if fimo_result_is_ok(result):
        return FIMO_IMPL_RESULT_OK_NAME

    error_name = result.vtable.contents.v0.error_name
    return error_name(result.data)


def fimo_result_error_description(result: FimoResult) -> FimoResultString:
    """Get the error description contained in the `FimoResult`.

    In case `result` does not contain an error this returns `"FIMO_IMPL_RESULT_OK_DESCRIPTION"`.
    """
    if fimo_result_is_ok(result):
        return FIMO_IMPL_RESULT_OK_NAME

    error_description = result.vtable.contents.v0.error_description
    return error_description(result.data)


# Header: fimo_std/memory.h

FIMO_MALLOC_ALIGNMENT = 16
"""Minimum alignment of the default allocator"""


class FimoMallocBuffer(c.Structure):
    """Am allocated buffer."""

    _fields_ = [("ptr", c.c_void_p), ("buff_size", c.c_size_t)]


_fimo_malloc = _lib.fimo_malloc
_fimo_malloc.argtypes = [c.c_size_t, c.POINTER(FimoResult)]
_fimo_malloc.restype = c.c_void_p


def fimo_malloc(size: c.c_size_t, error: Ref[FimoResult]) -> c.c_void_p:
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
_fimo_calloc.argtypes = [c.c_size_t, c.POINTER(FimoResult)]
_fimo_calloc.restype = c.c_void_p


def fimo_calloc(size: c.c_size_t, error: Ref[FimoResult]) -> c.c_void_p:
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
_fimo_aligned_alloc.argtypes = [c.c_size_t, c.c_size_t, c.POINTER(FimoResult)]
_fimo_aligned_alloc.restype = c.c_void_p


def fimo_aligned_alloc(
    alignment: c.c_size_t, size: c.c_size_t, error: Ref[FimoResult]
) -> c.c_void_p:
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
_fimo_malloc_sized.argtypes = [c.c_size_t, c.POINTER(FimoResult)]
_fimo_malloc_sized.restype = FimoMallocBuffer


def fimo_malloc_sized(size: c.c_size_t, error: Ref[FimoResult]) -> FimoMallocBuffer:
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
_fimo_calloc_sized.argtypes = [c.c_size_t, c.POINTER(FimoResult)]
_fimo_calloc_sized.restype = FimoMallocBuffer


def fimo_calloc_sized(size: c.c_size_t, error: Ref[FimoResult]) -> FimoMallocBuffer:
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
_fimo_aligned_alloc_sized.argtypes = [c.c_size_t, c.c_size_t, c.POINTER(FimoResult)]
_fimo_aligned_alloc_sized.restype = FimoMallocBuffer


def fimo_aligned_alloc_sized(
    alignment: c.c_size_t, size: c.c_size_t, error: Ref[FimoResult]
) -> FimoMallocBuffer:
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


def fimo_free_aligned_sized(
    ptr: c.c_void_p, alignment: c.c_size_t, size: c.c_size_t
) -> None:
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

    _fields_ = [
        ("major", FimoU32),
        ("minor", FimoU32),
        ("patch", FimoU32),
        ("build", FimoU64),
    ]


_fimo_version_parse_str = _lib.fimo_version_parse_str
_fimo_version_parse_str.argtypes = [c.c_char_p, c.c_size_t, c.POINTER(FimoVersion)]
_fimo_version_parse_str.restype = FimoResult


def fimo_version_parse_str(
    str: c.c_char_p, str_len: c.c_size_t, version: Ref[FimoVersion]
) -> FimoResult:
    """Parses a string into a `FimoVersion`.

    The string must be of the form "major.minor.patch" or "major.minor.patch+build".

    :param str: string to parse
    :param str_len: length of the string
    :param version: pointer to the parsed version.

    :return: Status code
    """
    return _fimo_version_parse_str(str, str_len, version)


_fimo_version_str_len = _lib.fimo_version_str_len
_fimo_version_str_len.argtypes = [c.POINTER(FimoVersion)]
_fimo_version_str_len.restype = FimoUSize


def fimo_version_str_len(version: Ref[FimoVersion]) -> FimoUSize:
    """Calculates the string length required to represent the version as a string.

    If `version` is `NULL`, this function returns `0`. The returned length is
    large enough for a call to `fimo_version_write_str` with the same version
    instance. The returned length does not include the zero-terminator.

    :param version: version to check.

    :return: Required string length
    """
    return _fimo_version_str_len(version)


_fimo_version_str_len_long = _lib.fimo_version_str_len_long
_fimo_version_str_len_long.argtypes = [c.POINTER(FimoVersion)]
_fimo_version_str_len_long.restype = FimoUSize


def fimo_version_str_len_long(version: Ref[FimoVersion]) -> FimoUSize:
    """Calculates the string length required to represent the version as a string.

    If `version` is `NULL`, this function returns `0`. The returned length is
    large enough for a call to `fimo_version_write_str_long` with the same
    version instance. The returned length does not include the zero-terminator.

    :param version: version to check.

    :return: Required string length
    """
    return _fimo_version_str_len_long(version)


_fimo_version_write_str = _lib.fimo_version_write_str
_fimo_version_write_str.argtypes = [
    c.POINTER(FimoVersion),
    c.c_char_p,
    c.c_size_t,
    c.POINTER(c.c_size_t),
]
_fimo_version_write_str.restype = FimoResult


def fimo_version_write_str(
    version: Ref[FimoVersion],
    str: c.c_char_p,
    str_len: c.c_size_t,
    written: Ref[c.c_size_t],
) -> FimoResult:
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
_fimo_version_write_str_long.argtypes = [
    c.POINTER(FimoVersion),
    c.c_char_p,
    c.c_size_t,
    c.POINTER(c.c_size_t),
]
_fimo_version_write_str_long.restype = FimoResult


def fimo_version_write_str_long(
    version: Ref[FimoVersion],
    str: c.c_char_p,
    str_len: c.c_size_t,
    written: Ref[c.c_size_t],
) -> FimoResult:
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
_fimo_version_cmp.argtypes = [c.POINTER(FimoVersion), c.POINTER(FimoVersion)]
_fimo_version_cmp.restype = c.c_int


def fimo_version_cmp(lhs: Ref[FimoVersion], rhs: Ref[FimoVersion]) -> int:
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
_fimo_version_cmp_long.argtypes = [c.POINTER(FimoVersion), c.POINTER(FimoVersion)]
_fimo_version_cmp_long.restype = c.c_int


def fimo_version_cmp_long(lhs: Ref[FimoVersion], rhs: Ref[FimoVersion]) -> int:
    """Compares two versions.

    Returns an ordering of the two versions, taking into consideration the build
    numbers. Returns `-1` if `lhs < rhs`, `0` if `lhs == rhs`, or `1` if `lhs > rhs`.

    :param lhs: first version (not `NULL`)
    :param rhs: second version (not `NULL`)

    :return: Version ordering
    """
    return _fimo_version_cmp_long(lhs, rhs)


_fimo_version_compatible = _lib.fimo_version_compatible
_fimo_version_compatible.argtypes = [c.POINTER(FimoVersion), c.POINTER(FimoVersion)]
_fimo_version_compatible.restype = c.c_bool


def fimo_version_compatible(got: Ref[FimoVersion], required: Ref[FimoVersion]) -> bool:
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

    _fields_ = [("secs", FimoU64), ("nanos", FimoU32)]


class FimoTime(c.Structure):
    """A point in time since the unix epoch."""

    _fields_ = [("secs", FimoU64), ("nanos", FimoU32)]


class FimoTimeMonotonic(c.Structure):
    """A monotonic point in time.

    The starting point is undefined.
    """

    _fields_ = [("secs", FimoU64), ("nanos", FimoU32)]


FIMO_DURATION_ZERO = FimoDuration(0, 0)
"""The zero duration."""

FIMO_DURATION_MAX = FimoDuration((1 << 64) - 1, 999999999)
"""The maximum duration."""

FIMO_UNIX_EPOCH = FimoTime(0, 0)
"""The UNIX epoch."""

FIMO_TIME_MAX = FimoTime((1 << 64) - 1, 999999999)
"""Largest possible time point."""

_fimo_duration_zero = _lib.fimo_duration_zero
_fimo_duration_zero.argtypes = []
_fimo_duration_zero.restype = FimoDuration


def fimo_duration_zero() -> FimoDuration:
    """Constructs the zero duration.

    :return: Zero duration.
    """
    return _fimo_duration_zero()


_fimo_duration_max = _lib.fimo_duration_max
_fimo_duration_max.argtypes = []
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


def fimo_duration_is_zero(duration: Ref[FimoDuration]) -> bool:
    """Checks if a duration is zero.

    :return: `true` if the duration is zero.
    """
    return _fimo_duration_is_zero(duration)


_fimo_duration_as_secs = _lib.fimo_duration_as_secs
_fimo_duration_as_secs.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_as_secs.restype = FimoU64


def fimo_duration_as_secs(duration: Ref[FimoDuration]) -> FimoU64:
    """Returns the whole seconds in a duration.

    :return: Whole seconds.
    """
    return _fimo_duration_as_secs(duration)


_fimo_duration_subsec_millis = _lib.fimo_duration_subsec_millis
_fimo_duration_subsec_millis.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_subsec_millis.restype = FimoU32


def fimo_duration_subsec_millis(duration: Ref[FimoDuration]) -> FimoU32:
    """Returns the fractional part in milliseconds.

    :return: Fractional part in whole milliseconds
    """
    return _fimo_duration_subsec_millis(duration)


_fimo_duration_subsec_micros = _lib.fimo_duration_subsec_micros
_fimo_duration_subsec_micros.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_subsec_micros.restype = FimoU32


def fimo_duration_subsec_micros(duration: Ref[FimoDuration]) -> FimoU32:
    """Returns the fractional part in microseconds.

    :return: Fractional part in whole microseconds.
    """
    return _fimo_duration_subsec_micros(duration)


_fimo_duration_subsec_nanos = _lib.fimo_duration_subsec_nanos
_fimo_duration_subsec_nanos.argtypes = [c.POINTER(FimoDuration)]
_fimo_duration_subsec_nanos.restype = FimoU32


def fimo_duration_subsec_nanos(duration: Ref[FimoDuration]) -> FimoU32:
    """Returns the fractional part in nanoseconds.

    :return: Fractional part in whole nanoseconds.
    """
    return _fimo_duration_subsec_nanos(duration)


_fimo_duration_as_millis = _lib.fimo_duration_as_millis
_fimo_duration_as_millis.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoU32)]
_fimo_duration_as_millis.restype = FimoU64


def fimo_duration_as_millis(duration: Ref[FimoDuration], high: Ref[FimoU32]) -> FimoU32:
    """Returns the whole milliseconds in a duration.

    If `high` is not null, it is set to store the overflow portion of the milliseconds.

    :return: Low part of the milliseconds.
    """
    return _fimo_duration_as_millis(duration, high)


_fimo_duration_as_micros = _lib.fimo_duration_as_micros
_fimo_duration_as_micros.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoU32)]
_fimo_duration_as_micros.restype = FimoU64


def fimo_duration_as_micros(duration: Ref[FimoDuration], high: Ref[FimoU32]) -> FimoU32:
    """Returns the whole microseconds in a duration.

    If `high` is not null, it is set to store the overflow portion of the microseconds.

    :return: Low part of the microseconds.
    """
    return _fimo_duration_as_micros(duration, high)


_fimo_duration_as_nanos = _lib.fimo_duration_as_nanos
_fimo_duration_as_nanos.argtypes = [c.POINTER(FimoDuration), c.POINTER(FimoU32)]
_fimo_duration_as_nanos.restype = FimoU64


def fimo_duration_as_nanos(duration: Ref[FimoDuration], high: Ref[FimoU32]) -> FimoU32:
    """Returns the whole nanoseconds in a duration.

    If `high` is not null, it is set to store the overflow portion of the nanoseconds.

    :return: Low part of the nanoseconds.
    """
    return _fimo_duration_as_nanos(duration, high)


_fimo_duration_add = _lib.fimo_duration_add
_fimo_duration_add.argtypes = [
    c.POINTER(FimoDuration),
    c.POINTER(FimoDuration),
    c.POINTER(FimoDuration),
]
_fimo_duration_add.restype = FimoResult


def fimo_duration_add(
    lhs: Ref[FimoDuration], rhs: Ref[FimoDuration], out: Ref[FimoDuration]
) -> FimoResult:
    """Adds two durations.

    :return: Status code.
    """
    return _fimo_duration_add(lhs, rhs, out)


_fimo_duration_saturating_add = _lib.fimo_duration_saturating_add
_fimo_duration_saturating_add.argtypes = [
    c.POINTER(FimoDuration),
    c.POINTER(FimoDuration),
]
_fimo_duration_saturating_add.restype = FimoDuration


def fimo_duration_saturating_add(
    lhs: Ref[FimoDuration], rhs: Ref[FimoDuration]
) -> FimoDuration:
    """Adds two durations.

    The result saturates to `FIMO_DURATION_MAX`, if an overflow occurs.

    :return: Added durations.
    """
    return _fimo_duration_saturating_add(lhs, rhs)


_fimo_duration_sub = _lib.fimo_duration_sub
_fimo_duration_sub.argtypes = [
    c.POINTER(FimoDuration),
    c.POINTER(FimoDuration),
    c.POINTER(FimoDuration),
]
_fimo_duration_sub.restype = FimoResult


def fimo_duration_sub(
    lhs: Ref[FimoDuration], rhs: Ref[FimoDuration], out: Ref[FimoDuration]
) -> FimoResult:
    """Subtracts two durations.

    :return: Status code.
    """
    return _fimo_duration_sub(lhs, rhs, out)


_fimo_duration_saturating_sub = _lib.fimo_duration_saturating_sub
_fimo_duration_saturating_sub.argtypes = [
    c.POINTER(FimoDuration),
    c.POINTER(FimoDuration),
]
_fimo_duration_saturating_sub.restype = FimoDuration


def fimo_duration_saturating_sub(
    lhs: Ref[FimoDuration], rhs: Ref[FimoDuration]
) -> FimoDuration:
    """Subtracts two durations.

    The result saturates to `FIMO_DURATION_ZERO`, if an overflow occurs or the resulting duration is negative.

    :return: Subtracted durations.
    """
    return _fimo_duration_saturating_sub(lhs, rhs)


_fimo_time_now = _lib.fimo_time_now
_fimo_time_now.argtypes = []
_fimo_time_now.restype = FimoTime


def fimo_time_now() -> FimoTime:
    """Returns the current time.

    :return: Current time.
    """
    return _fimo_time_now()


_fimo_time_elapsed = _lib.fimo_time_elapsed
_fimo_time_elapsed.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration)]
_fimo_time_elapsed.restype = FimoResult


def fimo_time_elapsed(
    time_point: Ref[FimoTime], elapsed: Ref[FimoDuration]
) -> FimoResult:
    """Returns the duration elapsed since a prior time point.

    :return: Status code.
    """
    return _fimo_time_elapsed(time_point, elapsed)


_fimo_time_duration_since = _lib.fimo_time_duration_since
_fimo_time_duration_since.argtypes = [
    c.POINTER(FimoTime),
    c.POINTER(FimoTime),
    c.POINTER(FimoDuration),
]
_fimo_time_duration_since.restype = FimoResult


def fimo_time_duration_since(
    time_point: Ref[FimoTime],
    earlier_time_point: Ref[FimoTime],
    duration: Ref[FimoDuration],
) -> FimoResult:
    """Returns the difference between two time points.

    :return: Status code.
    """
    return _fimo_time_duration_since(time_point, earlier_time_point, duration)


_fimo_time_add = _lib.fimo_time_add
_fimo_time_add.argtypes = [
    c.POINTER(FimoTime),
    c.POINTER(FimoDuration),
    c.POINTER(FimoTime),
]
_fimo_time_add.restype = FimoResult


def fimo_time_add(
    time_point: Ref[FimoTime], duration: Ref[FimoDuration], out: Ref[FimoTime]
) -> FimoResult:
    """Adds a duration to a time point.

    :return: Status code.
    """
    return _fimo_time_add(time_point, duration, out)


_fimo_time_saturating_add = _lib.fimo_time_saturating_add
_fimo_time_saturating_add.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration)]
_fimo_time_saturating_add.restype = FimoTime


def fimo_time_saturating_add(
    time_point: Ref[FimoTime], duration: Ref[FimoDuration]
) -> FimoTime:
    """Adds a duration to a time point.

    The result saturates to `FIMO_TIME_MAX`, if an overflow occurs.

    :return: Status code.
    """
    return _fimo_time_saturating_add(time_point, duration)


_fimo_time_sub = _lib.fimo_time_sub
_fimo_time_sub.argtypes = [
    c.POINTER(FimoTime),
    c.POINTER(FimoDuration),
    c.POINTER(FimoTime),
]
_fimo_time_sub.restype = FimoResult


def fimo_time_sub(
    time_point: Ref[FimoTime], duration: Ref[FimoDuration], out: Ref[FimoTime]
) -> FimoResult:
    """Subtracts a duration from a time point.

    :return: Status code.
    """
    return _fimo_time_sub(time_point, duration, out)


_fimo_time_saturating_sub = _lib.fimo_time_saturating_sub
_fimo_time_saturating_sub.argtypes = [c.POINTER(FimoTime), c.POINTER(FimoDuration)]
_fimo_time_saturating_sub.restype = FimoTime


def fimo_time_saturating_sub(
    time_point: Ref[FimoTime], duration: Ref[FimoDuration]
) -> FimoTime:
    """Subtracts a duration from a time point.

    The result saturates to `FIMO_UNIX_EPOCH`, if an overflow occurs or the resulting duration is negative.

    :return: Status code.
    """
    return _fimo_time_saturating_sub(time_point, duration)


# Header: fimo_std/context.h


class FimoContext(c.Structure):
    """Context of the fimo std."""

    _fields_ = [("data", c.c_void_p), ("vtable", c.c_void_p)]


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


FimoBaseStructIn._fields_ = [
    ("type", FimoStructType),
    ("next", c.POINTER(FimoBaseStructIn)),
]


class FimoBaseStructOut(c.Structure):
    """Base structure for a pointer chain."""

    pass


FimoBaseStructOut._fields_ = [
    ("type", FimoStructType),
    ("next", c.POINTER(FimoBaseStructOut)),
]


class FimoContextVTableHeader(c.Structure):
    """Header of all VTables of a `FimoContext`, for all future versions.

    May never be changed, since we rely on it to determine whether a
    given `FimoContext` instance is compatible with the definitions
    available to us.
    """

    _fields_ = [
        ("check_version", c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoVersion)))
    ]


class FimoContextCoreVTableV0(c.Structure):
    """Core VTable of a `FimoContext`.

    Changing the VTable is a breaking change.
    """

    _fields_ = [
        ("acquire", c.CFUNCTYPE(None, c.c_void_p)),
        ("release", c.CFUNCTYPE(None, c.c_void_p)),
    ]


_CURRENT_VERSION = FimoVersion(0, 1, 0)

_fimo_context_init = _lib.fimo_context_init
_fimo_context_init.argtypes = [
    c.POINTER(c.POINTER(FimoBaseStructIn)),
    c.POINTER(FimoContext),
]
_fimo_context_init.restype = FimoResult


def fimo_context_init(
    options: PtrRef[FimoBaseStructIn], context: Ref[FimoContext]
) -> FimoResult:
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


def fimo_context_check_version(context: FimoContext) -> FimoResult:
    """Checks the compatibility of the context version.

    This function must be called upon the acquisition of a context, that
    was not created locally, e.g., when being passed a context from
    another shared library. Failure of doing so, may cause undefined
    behavior, if the context is later utilized.

    :param context: the context

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    check_version = vtable.header.check_version
    return check_version(context, c.byref(_CURRENT_VERSION))


def fimo_context_acquire(context: FimoContext) -> None:
    """Acquires a reference to the context.

    Increases the reference count of the context. May abort the program,
    if doing so is not possible. May only be called with a valid reference
    to the context.

    :param context: the context
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    acquire = vtable.core_v0.acquire
    acquire(context)


def fimo_context_release(context: FimoContext) -> None:
    """Releases a reference to the context.

    Decrements the reference count of the context. When the reference count
    reaches zero, this function also destroys the reference. May only be
    called with a valid reference to the context.

    :param context: the context
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    release = vtable.core_v0.release
    release(context)


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

    _fields_ = [
        ("type", FimoStructType),
        ("next", c.POINTER(FimoBaseStructIn)),
        ("name", c.c_char_p),
        ("target", c.c_char_p),
        ("level", FimoTracingLevel),
        ("file_name", c.c_char_p),
        ("line_number", FimoI32),
    ]


FimoTracingMetadataPtr = c.POINTER(FimoTracingMetadata)
"""Pointer to a `FimoTracingMetadata`."""


class FimoTracingSpanDesc(c.Structure):
    """Descriptor of a new span."""

    _fields_ = [
        ("type", FimoStructType),
        ("next", c.POINTER(FimoBaseStructIn)),
        ("metadata", FimoTracingMetadataPtr),
    ]


class FimoTracingSpan(c.Structure):
    """A period of time, during which events can occur."""

    _fields_ = [("type", FimoStructType), ("next", c.POINTER(FimoBaseStructOut))]


class FimoTracingEvent(c.Structure):
    """An event to be traced."""

    _fields_ = [
        ("type", FimoStructType),
        ("next", c.POINTER(FimoBaseStructIn)),
        ("metadata", FimoTracingMetadataPtr),
    ]


FimoTracingFormat = c.CFUNCTYPE(
    FimoResult, c.POINTER(c.c_char), FimoUSize, c.c_void_p, c.POINTER(FimoUSize)
)
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

    _fields_ = [
        ("destroy", c.CFUNCTYPE(None, c.c_void_p)),
        (
            "call_stack_create",
            c.CFUNCTYPE(
                FimoResult, c.c_void_p, c.POINTER(FimoTime), c.POINTER(c.c_void_p)
            ),
        ),
        ("call_stack_drop", c.CFUNCTYPE(None, c.c_void_p, c.c_void_p)),
        (
            "call_stack_destroy",
            c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p),
        ),
        (
            "call_stack_unblock",
            c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p),
        ),
        (
            "call_stack_suspend",
            c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p, c.c_bool),
        ),
        (
            "call_stack_resume",
            c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p),
        ),
        (
            "span_push",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoTime),
                c.POINTER(FimoTracingSpanDesc),
                c.POINTER(c.c_char),
                FimoUSize,
                c.c_void_p,
            ),
        ),
        ("span_drop", c.CFUNCTYPE(None, c.c_void_p, c.c_void_p)),
        ("span_pop", c.CFUNCTYPE(None, c.c_void_p, c.POINTER(FimoTime), c.c_void_p)),
        (
            "event_emit",
            c.CFUNCTYPE(
                None,
                c.c_void_p,
                c.POINTER(FimoTime),
                c.c_void_p,
                c.POINTER(FimoTracingEvent),
                c.POINTER(c.c_char),
                FimoUSize,
            ),
        ),
        ("flush", c.CFUNCTYPE(None, c.c_void_p)),
    ]


class FimoTracingSubscriber(c.Structure):
    """A subscriber for tracing events.

    The main function of the tracing backend is managing and routing
    tracing events to subscribers. Therefore, it does not consume any
    events on its own, which is the task of the subscribers. Subscribers
    may utilize the events in any way they deem fit.
    """

    _fields_ = [
        ("type", FimoStructType),
        ("next", c.POINTER(FimoBaseStructIn)),
        ("ptr", c.c_void_p),
        ("vtable", c.POINTER(FimoTracingSubscriberVTable)),
    ]


FIMO_TRACING_DEFAULT_SUBSCRIBER = FimoTracingSubscriber.in_dll(
    _lib, "FIMO_TRACING_DEFAULT_SUBSCRIBER"
)
"""Default subscriber."""


class FimoTracingCreationConfig(c.Structure):
    """Configuration for the tracing backend.

    Can be passed when creating the context.
    """

    _fields_ = [
        ("type", FimoStructType),
        ("next", c.POINTER(FimoBaseStructIn)),
        ("format_buffer_size", FimoUSize),
        ("maximum_level", FimoTracingLevel),
        ("subscribers", c.POINTER(FimoTracingSubscriber)),
        ("subscriber_count", FimoUSize),
    ]


class FimoTracingVTableV0(c.Structure):
    """VTable of the tracing subsystem.

    Changing the VTable is a breaking change.
    """

    _fields_ = [
        (
            "call_stack_create",
            c.CFUNCTYPE(
                FimoResult, c.c_void_p, c.POINTER(c.POINTER(FimoTracingCallStack))
            ),
        ),
        (
            "call_stack_destroy",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoTracingCallStack)),
        ),
        (
            "call_stack_switch",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoTracingCallStack),
                c.POINTER(c.POINTER(FimoTracingCallStack)),
            ),
        ),
        (
            "call_stack_unblock",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoTracingCallStack)),
        ),
        ("call_stack_suspend_current", c.CFUNCTYPE(FimoResult, c.c_void_p, c.c_bool)),
        ("call_stack_resume_current", c.CFUNCTYPE(FimoResult, c.c_void_p)),
        (
            "span_create",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoTracingSpanDesc),
                c.POINTER(c.POINTER(FimoTracingSpan)),
                FimoTracingFormat,
                c.c_void_p,
            ),
        ),
        (
            "span_destroy",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoTracingSpan)),
        ),
        (
            "event_emit",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoTracingEvent),
                FimoTracingFormat,
                c.c_void_p,
            ),
        ),
        ("is_enabled", c.CFUNCTYPE(c.c_bool, c.c_void_p)),
        ("register_thread", c.CFUNCTYPE(FimoResult, c.c_void_p)),
        ("unregister_thread", c.CFUNCTYPE(FimoResult, c.c_void_p)),
        ("flush", c.CFUNCTYPE(FimoResult, c.c_void_p)),
    ]


def fimo_tracing_call_stack_create(
    context: FimoContext, call_stack: PtrRef[FimoTracingCallStack]
) -> FimoResult:
    """Creates a new empty call stack.

    If successful, the new call stack is marked as suspended, and written
    into `call_stack`. The new call stack is not set to be the active call
    stack.

    :param context: the context
    :param call_stack: pointer to the resulting call stack

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    call_stack_create = vtable.tracing_v0.call_stack_create
    return call_stack_create(c.c_void_p(context.data), call_stack)


def fimo_tracing_call_stack_destroy(
    context: FimoContext, call_stack: Ref[FimoTracingCallStack]
) -> FimoResult:
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
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    call_stack_destroy = vtable.tracing_v0.call_stack_destroy
    return call_stack_destroy(c.c_void_p(context.data), call_stack)


def fimo_tracing_call_stack_switch(
    context: FimoContext,
    call_stack: Ref[FimoTracingCallStack],
    old: PtrRef[FimoTracingCallStack],
) -> FimoResult:
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
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    call_stack_switch = vtable.tracing_v0.call_stack_switch
    return call_stack_switch(c.c_void_p(context.data), call_stack, old)


def fimo_tracing_call_stack_unblock(
    context: FimoContext, call_stack: Ref[FimoTracingCallStack]
) -> FimoResult:
    """Unblocks a blocked call stack.

    Once unblocked, the call stack may be resumed. The call stack
    may not be active and must be marked as blocked.

    :param context: the context
    :param call_stack: the call stack to unblock

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    call_stack_unblock = vtable.tracing_v0.call_stack_unblock
    return call_stack_unblock(c.c_void_p(context.data), call_stack)


def fimo_tracing_call_stack_suspend_current(
    context: FimoContext, block: c.c_bool
) -> FimoResult:
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
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    call_stack_suspend_current = vtable.tracing_v0.call_stack_suspend_current
    return call_stack_suspend_current(c.c_void_p(context.data), block)


def fimo_tracing_call_stack_resume_current(context: FimoContext) -> FimoResult:
    """Marks the current call stack as being resumed.

    Once resumed, the context can be used to trace messages. To be
    successful, the current call stack must be suspended and unblocked.

    This function may return `FIMO_ENOTSUP`, if the current thread is not
    registered with the subsystem.

    :param context: the context

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    call_stack_resume_current = vtable.tracing_v0.call_stack_resume_current
    return call_stack_resume_current(c.c_void_p(context.data))


def fimo_tracing_span_create_custom(
    context: FimoContext,
    span_desc: Ref[FimoTracingSpanDesc],
    span: PtrRef[FimoTracingSpan],
    format: FuncPointer,
    data: c.c_void_p,
) -> FimoResult:
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
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    span_create = vtable.tracing_v0.span_create
    return span_create(c.c_void_p(context.data), span_desc, span, format, data)


def fimo_tracing_span_destroy(
    context: FimoContext, span: Ref[FimoTracingSpan]
) -> FimoResult:
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
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    span_destroy = vtable.tracing_v0.span_destroy
    return span_destroy(c.c_void_p(context.data), span)


def fimo_tracing_event_emit_custom(
    context: FimoContext,
    event: Ref[FimoTracingEvent],
    format: FuncPointer,
    data: c.c_void_p,
) -> FimoResult:
    """Emits a new event with a custom formatter.

    The backend may use a formatting buffer of a fixed size. The formatter is
    expected to cut-of the message after reaching that specified size.

    :param context: the context
    :param event: the event to emit
    :param format: custom formatting function
    :param data: custom data to format

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    event_emit = vtable.tracing_v0.event_emit
    return event_emit(c.c_void_p(context.data), event, format, data)


def fimo_tracing_is_enabled(context: FimoContext) -> bool:
    """Checks whether the tracing subsystem is enabled.

    This function can be used to check whether to call into the subsystem at all.
    Calling this function is not necessary, as the remaining functions of the
    subsystem are guaranteed to return default values, in case the subsystem is
    disabled.

    :param context: the context

    :return: `True` if the subsystem is enabled.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    is_enabled = vtable.tracing_v0.is_enabled
    return is_enabled(c.c_void_p(context.data))


def fimo_tracing_register_thread(context: FimoContext) -> FimoResult:
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
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    register_thread = vtable.tracing_v0.register_thread
    return register_thread(c.c_void_p(context.data))


def fimo_tracing_unregister_thread(context: FimoContext) -> FimoResult:
    """Unregisters the calling thread from the tracing backend.

    Once unregistered, the calling thread looses access to the tracing
    backend until it is registered again. The thread can not be unregistered
    until the call stack is empty.

    :param context: the context

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    unregister_thread = vtable.tracing_v0.unregister_thread
    return unregister_thread(c.c_void_p(context.data))


def fimo_tracing_flush(context: FimoContext) -> FimoResult:
    """Flushes the streams used for tracing.

    If successful, any unwritten data is written out by the individual subscribers.

    :param context: the context

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    flush = vtable.tracing_v0.flush
    return flush(c.c_void_p(context.data))


# Header: fimo_std/module.h


class FimoModule(c.Structure):
    """State of a loaded module.

    A module is self-contained, and may not be passed to other modules.
    An instance of `FimoModule` is valid for as long as the owning module
    remains loaded. Modules must not leak any resources outside their own
    module, ensuring that they are destroyed upon module unloading.
    """

    pass


FimoModuleDynamicSymbolConstructor = c.CFUNCTYPE(
    FimoResult, c.POINTER(FimoModule), c.POINTER(c.c_void_p)
)
"""Constructor function for a dynamic symbol.

The constructor is in charge of constructing an instance of
a symbol. To that effect, it is provided  an instance to the
module. The resulting symbol is written into the last argument.

:param arg0: pointer to the module
:param arg1: pointer to the resulting symbol

:return: Status code.
"""

FimoModuleDynamicSymbolDestructor = c.CFUNCTYPE(None, c.c_void_p)
"""Destructor function for a dynamic symbol.

The destructor is safe to assume, that the symbol is no longer
used by any other module. During its destruction, a symbol is
not allowed to access the module backend.

:param arg0: symbol to destroy
"""


class FimoModuleLoadingSet(c.Structure):
    """Type-erased set of modules to load by the backend."""

    pass


FimoModuleConstructor = c.CFUNCTYPE(
    FimoResult,
    c.POINTER(FimoModule),
    c.POINTER(FimoModuleLoadingSet),
    c.POINTER(c.c_void_p),
)
"""Constructor function for a module.

The module constructor allows a module implementor to initialize
some module specific data at module load time. Some use cases for
module constructors are initialization of global module data, or
fetching optional symbols. Returning an error aborts the loading
of the module. Is called before the symbols of the modules are
exported/initialized.

:param arg0: pointer to the partially initialized module
:param arg1: module set that contained the module
:param arg1: pointer to the resulting module data

:return: Status code.
"""

FimoModuleDestructor = c.CFUNCTYPE(None, c.POINTER(FimoModule), c.c_void_p)
"""Destructor function for a module.

During its destruction, a module is not allowed to access the
module backend.

:param arg0: pointer to the module
:param arg1: module data to destroy
"""


class FimoModuleParamType(c.c_int):
    """Data type of module parameter."""

    FIMO_MODULE_PARAM_TYPE_U8 = c.c_int(0)
    FIMO_MODULE_PARAM_TYPE_U16 = c.c_int(1)
    FIMO_MODULE_PARAM_TYPE_U32 = c.c_int(2)
    FIMO_MODULE_PARAM_TYPE_U64 = c.c_int(3)
    FIMO_MODULE_PARAM_TYPE_I8 = c.c_int(4)
    FIMO_MODULE_PARAM_TYPE_I16 = c.c_int(5)
    FIMO_MODULE_PARAM_TYPE_I32 = c.c_int(6)
    FIMO_MODULE_PARAM_TYPE_I64 = c.c_int(7)
    pass


class FimoModuleParamAccess(c.c_int):
    """Access group for a module parameter."""

    FIMO_MODULE_PARAM_ACCESS_PUBLIC = c.c_int(0)
    FIMO_MODULE_PARAM_ACCESS_DEPENDENCY = c.c_int(1)
    FIMO_MODULE_PARAM_ACCESS_PRIVATE = c.c_int(2)
    pass


class FimoModuleParam(c.Structure):
    """A type-erased module parameter."""

    pass


class FimoModuleParamData(c.Structure):
    """A type-erased internal data type for a module parameter."""

    pass


FimoModuleParamSet = c.CFUNCTYPE(
    FimoResult,
    c.POINTER(FimoModule),
    c.c_void_p,
    FimoModuleParamType,
    c.POINTER(FimoModuleParamData),
)
"""Setter for a module parameter.

The setter can perform some validation before the parameter is set.
If the setter produces an error, the parameter won't be modified.

:param arg0: pointer to the module
:param arg1: pointer to the new value
:param arg2: type of the value
:param arg3: data of the parameter

:return: Status code.
"""

FimoModuleParamGet = c.CFUNCTYPE(
    FimoResult,
    c.POINTER(FimoModule),
    c.c_void_p,
    c.POINTER(FimoModuleParamType),
    c.POINTER(FimoModuleParamData),
)
"""Getter for a module parameter.

:param arg0: pointer to the module
:param arg1: buffer to store the value into
:param arg2: buffer to store the type of the value into
:param arg3: data of the parameter

:return: Status code.
"""


class _FimoModuleParamDeclDefaultValue(c.Union):
    _fields_ = [
        ("u8", FimoU8),
        ("u16", FimoU16),
        ("u32", FimoU32),
        ("u64", FimoU64),
        ("i8", FimoI8),
        ("i16", FimoI16),
        ("i32", FimoI32),
        ("i64", FimoI64),
    ]


class FimoModuleParamDecl(c.Structure):
    """Declaration of a module parameter."""

    _fields_ = [
        ("type", FimoModuleParamType),
        ("read_access", FimoModuleParamAccess),
        ("write_access", FimoModuleParamAccess),
        ("setter", FimoModuleParamSet),
        ("getter", FimoModuleParamGet),
        ("name", c.c_char_p),
        ("default_value", _FimoModuleParamDeclDefaultValue),
    ]


class FimoModuleResourceDecl(c.Structure):
    """Declaration of a module resource."""

    _fields_ = [("path", c.c_char_p)]


class FimoModuleNamespaceImport(c.Structure):
    """Declaration of a module namespace import."""

    _fields_ = [("name", c.c_char_p)]


class FimoModuleSymbolImport(c.Structure):
    """Declaration of a module symbol import."""

    _fields_ = [("version", FimoVersion), ("name", c.c_char_p), ("ns", c.c_char_p)]


class FimoModuleSymbolExport(c.Structure):
    """Declaration of a static module symbol export."""

    _fields_ = [
        ("symbol", c.c_void_p),
        ("version", FimoVersion),
        ("name", c.c_char_p),
        ("ns", c.c_char_p),
    ]


class FimoModuleDynamicSymbolExport(c.Structure):
    """Declaration of a dynamic module symbol export."""

    _fields_ = [
        ("constructor", FimoModuleDynamicSymbolConstructor),
        ("destructor", FimoModuleDynamicSymbolDestructor),
        ("version", FimoVersion),
        ("name", c.c_char_p),
        ("ns", c.c_char_p),
    ]


class FimoModuleExportModifierKey(c.c_int):
    """Valid keys of `FimoModuleExportModifier`."""

    FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR = c.c_int(0)
    FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY = c.c_int(1)
    pass


class FimoModuleExportModifier(c.Structure):
    """A modifier declaration for a module export."""

    _fields_ = [("key", FimoModuleExportModifierKey), ("value", c.c_void_p)]


class FimoModuleExportModifierDestructor(c.Structure):
    """Value for the `FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR` modifier key."""

    _fields_ = [("data", c.c_void_p), ("destructor", c.CFUNCTYPE(None, c.c_void_p))]


class FimoModuleExport(c.Structure):
    """Declaration of a module export."""

    _fields_ = [
        ("type", FimoStructType),
        ("next", c.POINTER(FimoBaseStructIn)),
        ("export_abi", FimoI32),
        ("name", c.c_char_p),
        ("description", c.c_char_p),
        ("author", c.c_char_p),
        ("license", c.c_char_p),
        ("parameters", c.POINTER(FimoModuleParamDecl)),
        ("parameters_count", FimoU32),
        ("resources", c.POINTER(FimoModuleResourceDecl)),
        ("resources_count", FimoU32),
        ("namespace_imports", c.POINTER(FimoModuleNamespaceImport)),
        ("namespace_imports_count", FimoU32),
        ("symbol_imports", c.POINTER(FimoModuleSymbolImport)),
        ("symbol_imports_count", FimoU32),
        ("symbol_exports", c.POINTER(FimoModuleSymbolExport)),
        ("symbol_exports_count", FimoU32),
        ("dynamic_symbol_exports", c.POINTER(FimoModuleDynamicSymbolExport)),
        ("dynamic_symbol_exports_count", FimoU32),
        ("modifiers", c.POINTER(FimoModuleExportModifier)),
        ("modifiers_count", FimoU32),
        ("module_constructor", FimoModuleConstructor),
        ("module_destructor", FimoModuleDestructor),
    ]


class FimoModuleParamTable(c.Structure):
    """Opaque type for a parameter table of a module.

    The layout of a parameter table is equivalent to an array of
    `FimoModuleParam*`, where each entry represents one parameter
    of the module parameter declaration list.
    """

    pass


class FimoModuleResourceTable(c.Structure):
    """Opaque type for a resource path table of a module.

    The import table is equivalent to an array of `const char*`,
    where each entry represents one resource path. The resource
    paths are ordered in declaration order.
    """

    pass


class FimoModuleSymbolImportTable(c.Structure):
    """Opaque type for a symbol import table of a module.

    The import table is equivalent to an array of `const void*`,
    where each entry represents one symbol of the module symbol
    import list. The symbols are ordered in declaration order.
    """

    pass


class FimoModuleSymbolExportTable(c.Structure):
    """Opaque type for a symbol export table of a module.

    The export table is equivalent to an array of `const void*`,
    where each entry represents one symbol of the module symbol
    export list, followed by the entries of the dynamic symbol
    export list.
    """

    pass


class FimoModuleInfo(c.Structure):
    """Info of a loaded module."""

    pass


FimoModuleInfo._fields_ = [
    ("type", FimoStructType),
    ("next", c.POINTER(FimoBaseStructIn)),
    ("name", c.c_char_p),
    ("description", c.c_char_p),
    ("author", c.c_char_p),
    ("license", c.c_char_p),
    ("module_path", c.c_char_p),
    ("acquire", c.CFUNCTYPE(None, c.POINTER(FimoModuleInfo))),
    ("release", c.CFUNCTYPE(None, c.POINTER(FimoModuleInfo))),
    ("is_loaded", c.CFUNCTYPE(c.c_bool, c.POINTER(FimoModuleInfo))),
    ("acquire_module_strong", c.CFUNCTYPE(FimoResult, c.POINTER(FimoModuleInfo))),
    ("release_module_strong", c.CFUNCTYPE(None, c.POINTER(FimoModuleInfo))),
]


def fimo_module_info_acquire(info: Ref[FimoModuleInfo]) -> None:
    acquire = info.acquire
    acquire(info)


def fimo_module_info_release(info: Ref[FimoModuleInfo]) -> None:
    release = info.release
    release(info)


def fimo_module_info_is_loaded(info: Ref[FimoModuleInfo]) -> bool:
    is_loaded = info.is_loaded
    return is_loaded(info)


def fimo_module_info_acquire_module_strong(info: Ref[FimoModuleInfo]) -> FimoResult:
    acquire_module_strong = info.acquire_module_strong
    return acquire_module_strong(info)


def fimo_module_info_release_module_strong(info: Ref[FimoModuleInfo]) -> None:
    release_module_strong = info.release_module_strong
    release_module_strong(info)


FimoModule._fields_ = [
    ("parameters", c.POINTER(FimoModuleParamTable)),
    ("resources", c.POINTER(FimoModuleResourceTable)),
    ("imports", c.POINTER(FimoModuleSymbolImportTable)),
    ("exports", c.POINTER(FimoModuleSymbolExportTable)),
    ("module_info", c.POINTER(FimoModuleInfo)),
    ("context", FimoContext),
    ("module_data", c.c_void_p),
]

FimoModuleLoadingFilter = c.CFUNCTYPE(c.c_bool, c.POINTER(FimoModuleExport), c.c_void_p)
"""A filter for selection modules to load by the module backend.

The filter function is passed the module export declaration
and can then decide whether the module should be loaded by
the backend.

:param arg0: module export to inspect
:param arg1: filter data

:return: `True`, if the module should be loaded.
"""

FimoModuleLoadingSuccessCallback = c.CFUNCTYPE(
    None, c.POINTER(FimoModuleInfo), c.c_void_p
)
"""A callback for successfully loading a module.

The callback function is called when the backend was successful
in loading the requested module, making it then possible to
request symbols.

:param arg0: loaded module
:param arg1: callback data
"""

FimoModuleLoadingErrorCallback = c.CFUNCTYPE(
    None, c.POINTER(FimoModuleExport), c.c_void_p
)
"""A callback for a module loading error.

The callback function is called when the backend was not
successful in loading the requested module.

:param arg0: module that caused the error
:param arg1: callback data
"""


class FimoModuleVTableV0(c.Structure):
    """VTable of the module subsystem.

    Changing the VTable is a breaking change.
    """

    _fields_ = [
        (
            "pseudo_module_new",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(c.POINTER(FimoModule))),
        ),
        (
            "pseudo_module_destroy",
            c.CFUNCTYPE(
                FimoResult, c.c_void_p, c.POINTER(FimoModule), c.POINTER(FimoContext)
            ),
        ),
        (
            "set_new",
            c.CFUNCTYPE(
                FimoResult, c.c_void_p, c.POINTER(c.POINTER(FimoModuleLoadingSet))
            ),
        ),
        (
            "set_has_module",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModuleLoadingSet),
                c.c_char_p,
                c.POINTER(c.c_bool),
            ),
        ),
        (
            "set_has_symbol",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModuleLoadingSet),
                c.c_char_p,
                c.c_char_p,
                FimoVersion,
                c.POINTER(c.c_bool),
            ),
        ),
        (
            "set_append_callback",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModuleLoadingSet),
                c.c_char_p,
                FimoModuleLoadingSuccessCallback,
                FimoModuleLoadingErrorCallback,
                c.c_void_p,
            ),
        ),
        (
            "set_append_freestanding_module",
            c.CFUNCTYPE(
                FimoResult,
                c.POINTER(FimoModule),
                c.POINTER(FimoModuleLoadingSet),
                c.POINTER(FimoModuleExport),
            ),
        ),
        (
            "set_append_modules",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModuleLoadingSet),
                c.c_char_p,
                FimoModuleLoadingFilter,
                c.c_void_p,
                c.CFUNCTYPE(
                    None,
                    c.CFUNCTYPE(c.c_bool, c.POINTER(FimoModuleExport), c.c_void_p),
                    c.c_void_p,
                ),
                c.c_void_p,
            ),
        ),
        (
            "set_dismiss",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoModuleLoadingSet)),
        ),
        (
            "set_finish",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoModuleLoadingSet)),
        ),
        (
            "find_by_name",
            c.CFUNCTYPE(
                FimoResult, c.c_void_p, c.c_char_p, c.POINTER(c.POINTER(FimoModuleInfo))
            ),
        ),
        (
            "find_by_symbol",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.c_char_p,
                c.c_char_p,
                FimoVersion,
                c.POINTER(c.POINTER(FimoModuleInfo)),
            ),
        ),
        (
            "namespace_exists",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.c_char_p, c.POINTER(c.c_bool)),
        ),
        (
            "namespace_include",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoModule), c.c_char_p),
        ),
        (
            "namespace_exclude",
            c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoModule), c.c_char_p),
        ),
        (
            "namespace_included",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_char_p,
                c.POINTER(c.c_bool),
                c.POINTER(c.c_bool),
            ),
        ),
        (
            "acquire_dependency",
            c.CFUNCTYPE(
                FimoResult, c.c_void_p, c.POINTER(FimoModule), c.POINTER(FimoModuleInfo)
            ),
        ),
        (
            "relinquish_dependency",
            c.CFUNCTYPE(
                FimoResult, c.c_void_p, c.POINTER(FimoModule), c.POINTER(FimoModuleInfo)
            ),
        ),
        (
            "has_dependency",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.POINTER(FimoModuleInfo),
                c.POINTER(c.c_bool),
                c.POINTER(c.c_bool),
            ),
        ),
        (
            "load_symbol",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_char_p,
                c.c_char_p,
                FimoVersion,
                c.POINTER(c.c_void_p),
            ),
        ),
        ("unload", c.CFUNCTYPE(FimoResult, c.c_void_p, c.POINTER(FimoModuleInfo))),
        (
            "param_query",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.c_char_p,
                c.c_char_p,
                c.POINTER(FimoModuleParamType),
                c.POINTER(FimoModuleParamAccess),
                c.POINTER(FimoModuleParamAccess),
            ),
        ),
        (
            "param_set_public",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.c_void_p,
                FimoModuleParamType,
                c.c_char_p,
                c.c_char_p,
            ),
        ),
        (
            "param_get_public",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.c_void_p,
                c.POINTER(FimoModuleParamType),
                c.c_char_p,
                c.c_char_p,
            ),
        ),
        (
            "param_set_dependency",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_void_p,
                FimoModuleParamType,
                c.c_char_p,
                c.c_char_p,
            ),
        ),
        (
            "param_get_dependency",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_void_p,
                c.POINTER(FimoModuleParamType),
                c.c_char_p,
                c.c_char_p,
            ),
        ),
        (
            "param_set_private",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_void_p,
                FimoModuleParamType,
                c.POINTER(FimoModuleParam),
            ),
        ),
        (
            "param_get_private",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_void_p,
                c.POINTER(FimoModuleParamType),
                c.POINTER(FimoModuleParam),
            ),
        ),
        (
            "param_set_inner",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_void_p,
                FimoModuleParamType,
                c.POINTER(FimoModuleParamData),
            ),
        ),
        (
            "param_get_inner",
            c.CFUNCTYPE(
                FimoResult,
                c.c_void_p,
                c.POINTER(FimoModule),
                c.c_void_p,
                c.POINTER(FimoModuleParamType),
                c.POINTER(FimoModuleParamData),
            ),
        ),
    ]


FimoImplModuleInspector = c.CFUNCTYPE(c.c_bool, c.POINTER(FimoModuleExport), c.c_void_p)
"""Inspector function for the iterator of exported modules.

:param arg0: export declaration
:param arg1: user defined data

:return: `True`, if the iteration should continue.
"""


_fimo_impl_modules_export_list: list[Pointer[FimoModuleExport]] = []


@c.CFUNCTYPE(None, FimoImplModuleInspector, c.c_void_p)
def fimo_impl_module_export_iterator(inspector: FuncPointer, data: int) -> None:
    """Iterates over the modules exported by the current binary.

    :param inspector: inspection function.
    :param data: user defined data to pass to the inspector.
    """
    # noinspection PyBroadException
    try:
        if not inspector:
            return

        data_ptr = c.c_void_p(data)

        for export in _fimo_impl_modules_export_list:
            if not export:
                continue

            continue_iteration = inspector(export, data_ptr)
            assert isinstance(continue_iteration, bool)
            if not continue_iteration:
                break
    except Exception:
        pass


_fimo_module_pseudo_module_new = _lib.fimo_module_pseudo_module_new
_fimo_module_pseudo_module_new.argtypes = [
    FimoContext,
    c.POINTER(c.POINTER(FimoModule)),
]
_fimo_module_pseudo_module_new.restype = FimoResult


def fimo_module_pseudo_module_new(
    context: FimoContext, module: PtrRef[FimoModule]
) -> FimoResult:
    """Constructs a new pseudo module.

    The functions of the module backend require that the caller owns
    a reference to their own module. This is a problem, as the constructor
    of the context won't be assigned a module instance during bootstrapping.
    As a workaround, we allow for the creation of pseudo modules, i.e.,
    module handles without an associated module.

    :param context: the context
    :param module: resulting pseudo module

    :return: Status code.
    """
    return _fimo_module_pseudo_module_new(context, module)


_fimo_module_pseudo_module_destroy = _lib.fimo_module_pseudo_module_destroy
_fimo_module_pseudo_module_destroy.argtypes = [
    c.POINTER(FimoModule),
    c.POINTER(FimoContext),
]
_fimo_module_pseudo_module_destroy.restype = FimoResult


def fimo_module_pseudo_module_destroy(
    module: Ref[FimoModule], context: Ref[FimoContext]
) -> FimoResult:
    """Destroys an existing pseudo module.

    By destroying the pseudo module, the caller ensures that they
    relinquished all access to handles derived by the module backend.

    :param module: pseudo module to destroy
    :param context: extracted context from the module

    :return: Status code.
    """
    return _fimo_module_pseudo_module_destroy(module, context)


_fimo_module_set_new = _lib.fimo_module_set_new
_fimo_module_set_new.argtypes = [
    FimoContext,
    c.POINTER(c.POINTER(FimoModuleLoadingSet)),
]
_fimo_module_set_new.restype = FimoResult


def fimo_module_set_new(
    context: FimoContext, module_set: PtrRef[FimoModuleLoadingSet]
) -> FimoResult:
    """Constructs a new empty module set.

    The loading of a module fails, if at least one dependency can
    not be satisfied, which requires the caller to manually find a
    suitable loading order. To facilitate the loading, we load
    multiple modules together, and automatically determine an
    appropriate load order for all modules inside the module set.

    :param context: the context
    :param module_set: new module set

    :return: Status code.
    """
    return _fimo_module_set_new(context, module_set)


_fimo_module_set_has_module = _lib.fimo_module_set_has_module
_fimo_module_set_has_module.argtypes = [
    FimoContext,
    c.POINTER(FimoModuleLoadingSet),
    c.c_char_p,
    c.POINTER(c.c_bool),
]
_fimo_module_set_has_module.restype = FimoResult


def fimo_module_set_has_module(
    context: FimoContext,
    module_set: Ref[FimoModuleLoadingSet],
    name: c.c_char_p,
    has_module: Ref[c.c_bool],
) -> FimoResult:
    """Checks whether a module set contains a module.

    :param context: the context
    :param module_set: new module set
    :param name: name of the module
    :param has_module: query result

    :return: Status code.
    """
    return _fimo_module_set_has_module(context, module_set, name, has_module)


_fimo_module_set_has_symbol = _lib.fimo_module_set_has_symbol
_fimo_module_set_has_symbol.argtypes = [
    FimoContext,
    c.POINTER(FimoModuleLoadingSet),
    c.c_char_p,
    c.c_char_p,
    FimoVersion,
    c.POINTER(c.c_bool),
]
_fimo_module_set_has_symbol.restype = FimoResult


def fimo_module_set_has_symbol(
    context: FimoContext,
    module_set: Ref[FimoModuleLoadingSet],
    name: c.c_char_p,
    ns: c.c_char_p,
    version: FimoVersion,
    has_symbol: Ref[c.c_bool],
) -> FimoResult:
    """Checks whether a module set contains a symbol.

    :param context: the context
    :param module_set: new module set
    :param name: symbol name
    :param ns: namespace name
    :param version: symbol version
    :param has_symbol: query result

    :return: Status code.
    """
    return _fimo_module_set_has_symbol(
        context, module_set, name, ns, version, has_symbol
    )


_fimo_module_set_append_callback = _lib.fimo_module_set_append_callback
_fimo_module_set_append_callback.argtypes = [
    FimoContext,
    c.POINTER(FimoModuleLoadingSet),
    c.c_char_p,
    FimoModuleLoadingSuccessCallback,
    FimoModuleLoadingErrorCallback,
    c.c_void_p,
]
_fimo_module_set_append_callback.restype = FimoResult


def fimo_module_set_append_callback(
    context: FimoContext,
    module_set: Ref[FimoModuleLoadingSet],
    module_name: c.c_char_p,
    on_success: FuncPointer,
    on_error: FuncPointer,
    user_data: c.c_void_p,
) -> FimoResult:
    """Adds a status callback to the module set.

    Adds a set of callbacks to report a successful or failed loading of
    a module. The `on_success` callback wil be called if the set was
    able to load all requested modules, whereas the `on_error` callback
    will be called immediately after the failed loading of the module. Since
    the module set can be in a partially loaded state at the time of calling
    this function, the `on_error` callback may be invoked immediately. The
    callbacks will be provided with a user-specified data pointer, which they
    are in charge of cleaning up. If the requested module `module_name` does
    not exist, this function will return an error.

    :param context: the context
    :param module_set: new module set
    :param module_name: module to query
    :param on_success: success callback
    :param on_error: error callback
    :param user_data: callback user data

    :return: Status code.
    """
    return _fimo_module_set_append_callback(
        context, module_set, module_name, on_success, on_error, user_data
    )


_fimo_module_set_append_freestanding_module = (
    _lib.fimo_module_set_append_freestanding_module
)
_fimo_module_set_append_freestanding_module.argtypes = [
    c.POINTER(FimoModule),
    c.POINTER(FimoModuleLoadingSet),
    c.POINTER(FimoModuleExport),
]
_fimo_module_set_append_freestanding_module.restype = FimoResult


def fimo_module_set_append_freestanding_module(
    module: Ref[FimoModule],
    module_set: Ref[FimoModuleLoadingSet],
    module_export: Ref[FimoModuleExport],
) -> FimoResult:
    """Adds a freestanding module to the module set.

    Adds a freestanding module to the set, so that it may be loaded
    by a future call to `fimo_module_set_finish`. Trying to include
    an invalid module, a module with duplicate exports or duplicate
    name will result in an error. Unlike `fimo_module_set_append_modules`,
    this function allows for the loading of dynamic modules, i.e.
    modules that are created at runtime, like non-native modules,
    which may require a runtime to be executed in. To ensure that
    the binary of the module calling this function is not unloaded
    while the new module is instantiated, the new module inherits
    a strong reference to the same binary as the caller's module.
    Note that the new module is not setup to automatically depend
    on `module`, but may prevent it from being unloaded while
    the set exists.

    :param module: owner of the export
    :param module_set: set of modules
    :param module_export: module to append to the set

    :return: Status code.
    """
    return _fimo_module_set_append_freestanding_module(
        module, module_set, module_export
    )


def fimo_module_set_append_modules(
    context: FimoContext,
    module_set: Ref[FimoModuleLoadingSet],
    module_path: c.c_char_p,
    filter: FuncPointer,
    filter_data: c.c_void_p,
) -> FimoResult:
    """Adds modules to the module set.

    Opens up a module binary to select which modules to load.
    The binary path `module_path` must be encoded as `UTF-8`,
    and point to the binary that contains the modules.  If the
    path is `NULL`, it iterates over the exported modules of the
    current binary. Each exported module is then passed to the
    `filter`, along with the provided `filter_data`, which can
    then filter which modules to load. This function may skip
    invalid module exports. Trying to include a module with duplicate
    exports or duplicate name will result in an error. This function
    signals an error, if the binary does not contain the symbols
    necessary to query the exported modules, but does not return
    in an error, if it does not export any modules. The necessary
    symbols are set up automatically, if the binary was linked with
    the fimo library. In case of an error, no modules are appended
    to the set.

    :param context: the context
    :param module_set: set of modules
    :param module_path: path to the binary to inspect
    :param filter: filter function
    :param filter_data: custom data to pass to the filter function

    :return: Status code.
    """
    vtable_ptr = c.c_void_p(context.vtable)
    vtable = c.cast(vtable_ptr, c.POINTER(FimoContextVTable))
    module_vtable = vtable.contents.module_v0
    set_append_modules = module_vtable.set_append_modules
    return set_append_modules(
        c.c_void_p(context.data),
        module_set,
        module_path,
        filter,
        filter_data,
        fimo_impl_module_export_iterator,
        c.cast(_fimo_module_set_append_freestanding_module, c.c_void_p),
    )


_fimo_module_set_dismiss = _lib.fimo_module_set_dismiss
_fimo_module_set_dismiss.argtypes = [FimoContext, c.POINTER(FimoModuleLoadingSet)]
_fimo_module_set_dismiss.restype = FimoResult


def fimo_module_set_dismiss(
    context: FimoContext, module_set: Ref[FimoModuleLoadingSet]
) -> FimoResult:
    """Destroys the module set without loading any modules.

    It is not possible to dismiss a module set that is currently
    being loaded.

    :param context: the context
    :param module_set: the module set to destroy

    :return: Status code.
    """
    return _fimo_module_set_dismiss(context, module_set)


_fimo_module_set_finish = _lib.fimo_module_set_finish
_fimo_module_set_finish.argtypes = [FimoContext, c.POINTER(FimoModuleLoadingSet)]
_fimo_module_set_finish.restype = FimoResult


def fimo_module_set_finish(
    context: FimoContext, module_set: Ref[FimoModuleLoadingSet]
) -> FimoResult:
    """Destroys the module set and loads the modules contained in it.

    After successfully calling this function, the modules contained
    in the set are loaded, and their symbols are available to all
    other modules. If the construction of one module results in an
    error, or if a dependency can not be satisfied, this function
    rolls back the loading of all modules contained in the set
    and returns an error. It is not possible to load a module set,
    while another set is being loaded.

    :param context: the context
    :param module_set: a set of modules to load

    :return: Status code.
    """
    return _fimo_module_set_finish(context, module_set)


_fimo_module_find_by_name = _lib.fimo_module_find_by_name
_fimo_module_find_by_name.argtypes = [
    FimoContext,
    c.c_char_p,
    c.POINTER(c.POINTER(FimoModuleInfo)),
]
_fimo_module_find_by_name.restype = FimoResult


def fimo_module_find_by_name(
    context: FimoContext, name: c.c_char_p, module: PtrRef[FimoModuleInfo]
) -> FimoResult:
    """Searches for a module by its name.

    Queries a module by its unique name. The returned `FimoModuleInfo`
    will have its reference count increased.

    :param context: context
    :param name: module name
    :param module: resulting module

    :return: Status code.
    """
    return _fimo_module_find_by_name(context, name, module)


_fimo_module_find_by_symbol = _lib.fimo_module_find_by_symbol
_fimo_module_find_by_symbol.argtypes = [
    FimoContext,
    c.c_char_p,
    c.c_char_p,
    FimoVersion,
    c.POINTER(c.POINTER(FimoModuleInfo)),
]
_fimo_module_find_by_symbol.restype = FimoResult


def fimo_module_find_by_symbol(
    context: FimoContext,
    name: c.c_char_p,
    ns: c.c_char_p,
    version: FimoVersion,
    module: PtrRef[FimoModuleInfo],
) -> FimoResult:
    """Searches for a module by a symbol it exports.

    Queries the module that exported the specified symbol. The returned
    `FimoModuleInfo` will have its reference count increased.

    :param context: context
    :param name: symbol name
    :param ns: symbol namespace
    :param version: symbol version
    :param module: resulting module

    :return: Status code.
    """
    return _fimo_module_find_by_symbol(context, name, ns, version, module)


_fimo_module_namespace_exists = _lib.fimo_module_namespace_exists
_fimo_module_namespace_exists.argtypes = [FimoContext, c.c_char_p, c.POINTER(c.c_bool)]
_fimo_module_namespace_exists.restype = FimoResult


def fimo_module_namespace_exists(
    context: FimoContext, ns: c.c_char_p, exists: Ref[c.c_bool]
) -> FimoResult:
    """Checks for the presence of a namespace in the module backend.

    A namespace exists, if at least one loaded module exports
    one symbol in said namespace.

    :param context: context
    :param ns: symbol namespace
    :param exists: query result

    :return: Status code.
    """
    return _fimo_module_namespace_exists(context, ns, exists)


_fimo_module_namespace_include = _lib.fimo_module_namespace_include
_fimo_module_namespace_include.argtypes = [c.POINTER(FimoModule), c.c_char_p]
_fimo_module_namespace_include.restype = FimoResult


def fimo_module_namespace_include(
    module: Ref[FimoModule], ns: c.c_char_p
) -> FimoResult:
    """Includes a namespace by the module.

    Once included, the module gains access to the symbols
    of its dependencies that are exposed in said namespace.
    A namespace can not be included multiple times.

    :param module: module of the caller
    :param ns: namespace to include

    :return: Status code.
    """
    return _fimo_module_namespace_include(module, ns)


_fimo_module_namespace_exclude = _lib.fimo_module_namespace_exclude
_fimo_module_namespace_exclude.argtypes = [c.POINTER(FimoModule), c.c_char_p]
_fimo_module_namespace_exclude.restype = FimoResult


def fimo_module_namespace_exclude(
    module: Ref[FimoModule], ns: c.c_char_p
) -> FimoResult:
    """Removes a namespace include from the module.

    Once excluded, the caller guarantees to relinquish
    access to the symbols contained in said namespace.
    It is only possible to exclude namespaces that were
    manually added, whereas static namespace includes
    remain valid until the module is unloaded.

    :param module: module of the caller
    :param ns: namespace to exclude

    :return: Status code.
    """
    return _fimo_module_namespace_exclude(module, ns)


_fimo_module_namespace_included = _lib.fimo_module_namespace_included
_fimo_module_namespace_included.argtypes = [
    c.POINTER(FimoModule),
    c.c_char_p,
    c.POINTER(c.c_bool),
    c.POINTER(c.c_bool),
]
_fimo_module_namespace_included.restype = FimoResult


def fimo_module_namespace_included(
    module: Ref[FimoModule],
    ns: c.c_char_p,
    is_included: Ref[c.c_bool],
    is_static: Ref[c.c_bool],
) -> FimoResult:
    """Checks if a module includes a namespace.

    Checks if `module` specified that it includes the
    namespace `ns`. In that case, the module is allowed access
    to the symbols in the namespace. The result of the query
    is stored in `is_included`. Additionally, this function also
    queries whether the include is static, i.e., the include was
    specified by the module at load time. The include type is
    stored in `is_static`.

    :param module: module of the caller
    :param ns: namespace to query
    :param is_included: result of the query
    :param is_static: resulting include type

    :return: Status code.
    """
    return _fimo_module_namespace_included(module, ns, is_included, is_static)


_fimo_module_acquire_dependency = _lib.fimo_module_acquire_dependency
_fimo_module_acquire_dependency.argtypes = [
    c.POINTER(FimoModule),
    c.POINTER(FimoModuleInfo),
]
_fimo_module_acquire_dependency.restype = FimoResult


def fimo_module_acquire_dependency(
    module: Ref[FimoModule], dependency: Ref[FimoModuleInfo]
) -> FimoResult:
    """Acquires another module as a dependency.

    After acquiring a module as a dependency, the module
    is allowed access to the symbols and protected parameters
    of said dependency. Trying to acquire a dependency to a
    module that is already a dependency, or to a module that
    would result in a circular dependency will result in an
    error.

    :param module: module of the caller
    :param dependency: module to acquire as a dependency

    :return: Status code.
    """
    return _fimo_module_acquire_dependency(module, dependency)


_fimo_module_relinquish_dependency = _lib.fimo_module_relinquish_dependency
_fimo_module_relinquish_dependency.argtypes = [
    c.POINTER(FimoModule),
    c.POINTER(FimoModuleInfo),
]
_fimo_module_relinquish_dependency.restype = FimoResult


def fimo_module_relinquish_dependency(
    module: Ref[FimoModule], dependency: Ref[FimoModuleInfo]
) -> FimoResult:
    """Removes a module as a dependency.

    By removing a module as a dependency, the caller
    ensures that it does not own any references to resources
    originating from the former dependency, and allows for
    the unloading of the module. A module can only relinquish
    dependencies to modules that were acquired dynamically,
    as static dependencies remain valid until the module is
    unloaded.

    :param module: module of the caller
    :param dependency: dependency to remove

    :return: Status code.
    """
    return _fimo_module_relinquish_dependency(module, dependency)


_fimo_module_has_dependency = _lib.fimo_module_has_dependency
_fimo_module_has_dependency.argtypes = [
    c.POINTER(FimoModule),
    c.POINTER(FimoModuleInfo),
    c.POINTER(c.c_bool),
    c.POINTER(c.c_bool),
]
_fimo_module_has_dependency.restype = FimoResult


def fimo_module_has_dependency(
    module: Ref[FimoModule],
    other: Ref[FimoModuleInfo],
    has_dependency: Ref[c.c_bool],
    is_static: Ref[c.c_bool],
) -> FimoResult:
    """Checks if a module depends on another module.

    Checks if `other` is a dependency of `module`. In that
    case `module` is allowed to access the symbols exported
    by `other`. The result of the query is stored in
    `has_dependency`. Additionally, this function also
    queries whether the dependency is static, i.e., the
    dependency was set by the module backend at load time.
    The dependency type is stored in `is_static`.

    :param module: module of the caller
    :param other: other module to check as a dependency
    :param has_dependency: result of the query
    :param is_static: resulting dependency type

    :return: Status code.
    """
    return _fimo_module_has_dependency(module, other, has_dependency, is_static)


_fimo_module_load_symbol = _lib.fimo_module_load_symbol
_fimo_module_load_symbol.argtypes = [
    c.POINTER(FimoModule),
    c.c_char_p,
    c.c_char_p,
    FimoVersion,
    c.POINTER(c.c_void_p),
]
_fimo_module_load_symbol.restype = FimoResult


def fimo_module_load_symbol(
    module: Ref[FimoModule],
    name: c.c_char_p,
    ns: c.c_char_p,
    version: FimoVersion,
    symbol: PtrRef[c.c_void_p],
) -> FimoResult:
    """Loads a symbol from the module backend.

    The caller can query the backend for a symbol of a loaded
    module. This is useful for loading optional symbols, or
    for loading symbols after the creation of a module. The
    symbol, if it exists, is written into `symbol`, and can
    be used until the module relinquishes the dependency to
    the module that exported the symbol. This function fails,
    if the module containing the symbol is not a dependency
    of the module.

    :param module: module that requires the symbol
    :param name: symbol name
    :param ns: symbol namespace
    :param version: symbol version
    :param symbol: resulting symbol

    :return: Status code.
    """
    return _fimo_module_load_symbol(module, name, ns, version, symbol)


_fimo_module_unload = _lib.fimo_module_unload
_fimo_module_unload.argtypes = [FimoContext, c.POINTER(FimoModuleInfo)]
_fimo_module_unload.restype = FimoResult


def fimo_module_unload(context: FimoContext, module: Ref[FimoModuleInfo]) -> FimoResult:
    """Unloads a module.

    If successful, this function unloads the module `module`.
    To succeed, the module no other module may depend on the module.
    This function automatically unloads cleans up unreferenced modules,
    except if they are a pseudo module.

    Setting `module` to `NULL` only runs the cleanup of all loose modules.

    :param context: the context
    :param module: module to unload

    :return: Status code.
    """
    return _fimo_module_unload(context, module)


_fimo_module_param_query = _lib.fimo_module_param_query
_fimo_module_param_query.argtypes = [
    FimoContext,
    c.c_char_p,
    c.c_char_p,
    c.POINTER(FimoModuleParamType),
    c.POINTER(FimoModuleParamAccess),
    c.POINTER(FimoModuleParamAccess),
]
_fimo_module_param_query.restype = FimoResult


def fimo_module_param_query(
    context: FimoContext,
    module_name: c.c_char_p,
    param: c.c_char_p,
    type: Ref[FimoModuleParamType],
    read: Ref[FimoModuleParamAccess],
    write: Ref[FimoModuleParamAccess],
) -> FimoResult:
    """Queries the info of a module parameter.

    This function can be used to query the datatype, the read access,
    and the write access of a module parameter. This function fails,
    if the parameter can not be found.

    :param context: context
    :param module_name: name of the module containing the parameter
    :param param: parameter to query
    :param type: queried parameter datatype
    :param read: queried parameter read access
    :param write: queried parameter write access

    :return: Status code.
    """
    return _fimo_module_param_query(context, module_name, param, type, read, write)


_fimo_module_param_set_public = _lib.fimo_module_param_set_public
_fimo_module_param_set_public.argtypes = [
    FimoContext,
    c.c_void_p,
    FimoModuleParamType,
    c.c_char_p,
    c.c_char_p,
]
_fimo_module_param_set_public.restype = FimoResult


def fimo_module_param_set_public(
    context: FimoContext,
    value: c.c_void_p,
    type: FimoModuleParamType,
    module_name: c.c_char_p,
    param: c.c_char_p,
) -> FimoResult:
    """Sets a module parameter with public write access.

    Sets the value of a module parameter with public write access.
    The operation fails, if the parameter does not exist, or if
    the parameter does not allow writing with a public access.
    The caller must ensure that `value` points to an instance of
    the same datatype as the parameter in question.

    :param context: context
    :param value: pointer to the value to store
    :param type: type of the value
    :param module_name: name of the module containing the parameter
    :param param: name of the parameter

    :return: Status code.
    """
    return _fimo_module_param_set_public(context, value, type, module_name, param)


_fimo_module_param_get_public = _lib.fimo_module_param_get_public
_fimo_module_param_get_public.argtypes = [
    FimoContext,
    c.c_void_p,
    c.POINTER(FimoModuleParamType),
    c.c_char_p,
    c.c_char_p,
]
_fimo_module_param_get_public.restype = FimoResult


def fimo_module_param_get_public(
    context: FimoContext,
    value: c.c_void_p,
    type: Ref[FimoModuleParamType],
    module_name: c.c_char_p,
    param: c.c_char_p,
) -> FimoResult:
    """Reads a module parameter with public read access.

    Reads the value of a module parameter with public read access.
    The operation fails, if the parameter does not exist, or if
    the parameter does not allow reading with a public access.
    The caller must ensure that `value` points to an instance of
    the same datatype as the parameter in question.

    :param context: context
    :param value: pointer where to store the value
    :param type: buffer where to store the type of the parameter
    :param module_name: name of the module containing the parameter
    :param param: name of the parameter

    :return: Status code.
    """
    return _fimo_module_param_get_public(context, value, type, module_name, param)


_fimo_module_param_set_dependency = _lib.fimo_module_param_set_dependency
_fimo_module_param_set_dependency.argtypes = [
    c.POINTER(FimoModule),
    c.c_void_p,
    FimoModuleParamType,
    c.c_char_p,
    c.c_char_p,
]
_fimo_module_param_set_dependency.restype = FimoResult


def fimo_module_param_set_dependency(
    module: Ref[FimoModule],
    value: c.c_void_p,
    type: FimoModuleParamType,
    module_name: c.c_char_p,
    param: c.c_char_p,
) -> FimoResult:
    """Sets a module parameter with dependency write access.

    Sets the value of a module parameter with dependency write
    access. The operation fails, if the parameter does not exist,
    or if the parameter does not allow writing with a dependency
    access. The caller must ensure that `value` points to an
    instance of the same datatype as the parameter in question.

    :param module: module of the caller
    :param value: pointer to the value to store
    :param type: type of the value
    :param module_name: name of the module containing the parameter
    :param param: name of the parameter

    :return: Status code.
    """
    return _fimo_module_param_set_dependency(module, value, type, module_name, param)


_fimo_module_param_get_dependency = _lib.fimo_module_param_get_dependency
_fimo_module_param_get_dependency.argtypes = [
    c.POINTER(FimoModule),
    c.c_void_p,
    c.POINTER(FimoModuleParamType),
    c.c_char_p,
    c.c_char_p,
]
_fimo_module_param_get_dependency.restype = FimoResult


def fimo_module_param_get_dependency(
    module: Ref[FimoModule],
    value: c.c_void_p,
    type: Ref[FimoModuleParamType],
    module_name: c.c_char_p,
    param: c.c_char_p,
) -> FimoResult:
    """Reads a module parameter with dependency read access.

    Reads the value of a module parameter with dependency read
    access. The operation fails, if the parameter does not exist,
    or if the parameter does not allow reading with a dependency
    access. The caller must ensure that `value` points to an
    instance of the same datatype as the parameter in question.

    :param module: module of the caller
    :param value: pointer where to store the value
    :param type: buffer where to store the type of the parameter
    :param module_name: name of the module containing the parameter
    :param param: name of the parameter

    :return: Status code.
    """
    return _fimo_module_param_get_dependency(module, value, type, module_name, param)


_fimo_module_param_set_private = _lib.fimo_module_param_set_private
_fimo_module_param_set_private.argtypes = [
    c.POINTER(FimoModule),
    c.c_void_p,
    FimoModuleParamType,
    c.POINTER(FimoModuleParam),
]
_fimo_module_param_set_private.restype = FimoResult


def fimo_module_param_set_private(
    module: Ref[FimoModule],
    value: c.c_void_p,
    type: FimoModuleParamType,
    param: Ref[FimoModuleParam],
) -> FimoResult:
    """Setter for a module parameter.

    If the setter produces an error, the parameter won't be modified.

    :param module: module of the caller
    :param value: value to write
    :param type: type of the value
    :param param: parameter to write

    :return: Status code.
    """
    return _fimo_module_param_set_private(module, value, type, param)


_fimo_module_param_get_private = _lib.fimo_module_param_get_private
_fimo_module_param_get_private.argtypes = [
    c.POINTER(FimoModule),
    c.c_void_p,
    c.POINTER(FimoModuleParamType),
    c.POINTER(FimoModuleParam),
]
_fimo_module_param_get_private.restype = FimoResult


def fimo_module_param_get_private(
    module: Ref[FimoModule],
    value: c.c_void_p,
    type: Ref[FimoModuleParamType],
    param: Ref[FimoModuleParam],
) -> FimoResult:
    """Getter for a module parameter.

    :param module: module of the caller
    :param value: buffer where to store the parameter
    :param type: buffer where to store the type of the parameter
    :param param: parameter to load

    :return: Status code.
    """
    return _fimo_module_param_get_private(module, value, type, param)


_fimo_module_param_set_inner = _lib.fimo_module_param_set_inner
_fimo_module_param_set_inner.argtypes = [
    c.POINTER(FimoModule),
    c.c_void_p,
    FimoModuleParamType,
    c.POINTER(FimoModuleParamData),
]
_fimo_module_param_set_inner.restype = FimoResult


def fimo_module_param_set_inner(
    module: Ref[FimoModule],
    value: c.c_void_p,
    type: FimoModuleParamType,
    param: Ref[FimoModuleParamData],
) -> FimoResult:
    """Internal setter for a module parameter.

    If the setter produces an error, the parameter won't be modified.

    :param module: module of the caller
    :param value: value to write
    :param type: type of the value
    :param param: parameter to write

    :return: Status code.
    """
    return _fimo_module_param_set_inner(module, value, type, param)


_fimo_module_param_get_inner = _lib.fimo_module_param_get_inner
_fimo_module_param_get_inner.argtypes = [
    c.POINTER(FimoModule),
    c.c_void_p,
    c.POINTER(FimoModuleParamType),
    c.POINTER(FimoModuleParamData),
]
_fimo_module_param_get_inner.restype = FimoResult


def fimo_module_param_get_inner(
    module: Ref[FimoModule],
    value: c.c_void_p,
    type: Ref[FimoModuleParamType],
    param: Ref[FimoModuleParamData],
) -> FimoResult:
    """Internal getter for a module parameter.

    :param module: module of the caller
    :param value: buffer where to store the parameter
    :param type: buffer where to store the type of the parameter
    :param param: parameter to load

    :return: Status code.
    """
    return _fimo_module_param_get_inner(module, value, type, param)


# Header: fimo_std/vtable.h


class FimoContextVTable(c.Structure):
    """VTable of a `FimoContext`.

    The abi of this type is semi-stable, where given two compatible
    versions `v1` and `v2` with `v1 <= v2`, a pointer to the vtable
    in `v2`, i.e., `FimoContextVTable_v2*` can be cast to a pointer
    to the vtable in version `v1`, or `FimoContextVTable_v1*`. To
    that end, we are allowed to add new fields to this struct and
    restricting the alignment. Further, to detect a version mismatch,
    we require that `FimoContextVTableHeader` is always the first
    member of the VTable.
    """

    _fields_ = [
        ("header", FimoContextVTableHeader),
        ("core_v0", FimoContextCoreVTableV0),
        ("tracing_v0", FimoTracingVTableV0),
        ("module_v0", FimoModuleVTableV0),
    ]
