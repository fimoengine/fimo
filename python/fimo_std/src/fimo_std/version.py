import ctypes as c
from typing import Optional, Self

from . import error
from . import memory
from . import ffi as _ffi


class Version(_ffi.FFITransferable[_ffi.FimoVersion]):
    """A version specifier."""

    def __init__(self, major: int, minor: int, patch: int, build: Optional[int] = None):
        """Constructs a new `Version`."""
        if build is None:
            build = 0

        if not isinstance(major, int) or not 0 <= major <= 4_294_967_295:
            error.ErrorCode.EINVAL.raise_if_error()
        if not isinstance(minor, int) or not 0 <= minor <= 4_294_967_295:
            error.ErrorCode.EINVAL.raise_if_error()
        if not isinstance(patch, int) or not 0 <= patch <= 4_294_967_295:
            error.ErrorCode.EINVAL.raise_if_error()
        if not isinstance(build, int) or not 0 <= build <= 18_446_744_073_709_551_615:
            error.ErrorCode.EINVAL.raise_if_error()

        self._version = _ffi.FimoVersion(major, minor, patch, build)

    def major(self) -> int:
        """Fetches the major version specifier."""
        return self._version.major.value

    def minor(self) -> int:
        """Fetches the `minor` version specifier."""
        return self._version.minor.value

    def patch(self) -> int:
        """Fetches the `patch` version specifier."""
        return self._version.patch.value

    def build(self) -> int:
        """Fetches the `build` version specifier."""
        return self._version.build.value

    @staticmethod
    def parse_str(string: str):
        """Parses a string into a `Version`.

        The string must be of the form "major.minor.patch" or "major.minor.patch+build".

        :param string: string to parse
        :return: Parsed version.
        :raises Error: Version could not be parsed
        """
        if not isinstance(string, str):
            error.ErrorCode.EINVAL.raise_if_error()

        string_ = string.encode()
        vers = _ffi.FimoVersion()
        err = _ffi.fimo_version_parse_str(
            c.c_char_p(string_), c.c_size_t(len(string_)), c.byref(vers)
        )
        error.ErrorCode(err.value).raise_if_error()

        major = vers.major.value
        minor = vers.minor.value
        patch = vers.patch.value
        build = vers.build.value

        return Version(major, minor, patch, build)

    def string_length(self) -> int:
        """Calculates the string length required to represent the version as a string."""
        return _ffi.fimo_version_str_len(c.byref(self._version)).value

    def string_length_long(self) -> int:
        """Calculates the string length required to represent the version as a string."""
        return _ffi.fimo_version_str_len_full(c.byref(self._version)).value

    def as_str(self) -> str:
        """Returns the string representation of the version."""
        length = self.string_length() + 1
        buffer = memory.DefaultAllocator.malloc(length * c.sizeof(c.c_char))
        buffer_str = c.cast(buffer, c.c_char_p)
        err_ffi = _ffi.fimo_version_write_str(
            c.byref(self._version), buffer_str, c.c_size_t(length), None
        )
        err = error.ErrorCode(err_ffi.value)

        if err.is_error():
            memory.DefaultAllocator.free(buffer)
            err.raise_if_error()
        assert buffer_str.value is not None
        string = buffer_str.value.decode()
        memory.DefaultAllocator.free(buffer)
        return string

    def as_str_long(self) -> str:
        """Returns the string representation of the version."""
        length = self.string_length_long() + 1
        buffer = memory.DefaultAllocator.malloc(length * c.sizeof(c.c_char))
        buffer_str = c.cast(buffer, c.c_char_p)
        err_ffi = _ffi.fimo_version_write_str_long(
            c.byref(self._version), buffer_str, c.c_size_t(length), None
        )
        err = error.ErrorCode(err_ffi.value)

        if err.is_error():
            memory.DefaultAllocator.free(buffer)
            err.raise_if_error()
        assert buffer_str.value is not None
        string = buffer_str.value.decode()
        memory.DefaultAllocator.free(buffer)
        return string

    def cmp(self, other: Self) -> int:
        """Compares two versions.

        Returns an ordering of the two versions, taking into consideration the build
        numbers. Returns `-1` if `self < other`, `0` if `self == other`, or `1` if `self > other`.

        :return: Version order.
        :raises Error: `other` is not a `Version`
        """
        if not isinstance(other, Version):
            error.ErrorCode.EINTR.raise_if_error()

        return _ffi.fimo_version_cmp(c.byref(self._version), c.byref(other._version))

    def cmp_long(self, other: Self) -> int:
        """Compares two versions.

        Returns an ordering of the two versions, taking into consideration the build
        numbers. Returns `-1` if `self < other`, `0` if `self == other`, or `1` if `self > other`.

        :return: Version order.
        :raises Error: `other` is not a `Version`
        """
        if not isinstance(other, Version):
            error.ErrorCode.EINTR.raise_if_error()

        return _ffi.fimo_version_cmp_long(
            c.byref(self._version), c.byref(other._version)
        )

    def is_compatible(self, required: Self) -> bool:
        """Checks for the compatibility of two versions.

        If `self` is compatible with `required` it indicates that an object which is
        versioned with the version `self` can be used instead of an object of the
        same type carrying the version `required`.

        The compatibility of `self` with `required` is determined by the following
        algorithm:

            1. The major versions of `self` and `required` must be equal.
            2. If the major version is `0`, the minor versions must be equal.
            3. `self >= required` without the build number.

        :return: Version compatibility.
        :raises Error: `required` is not a `Version`
        """
        if not isinstance(required, Version):
            error.ErrorCode.EINTR.raise_if_error()

        return _ffi.fimo_version_compatible(
            c.byref(self._version), c.byref(required._version)
        )

    def __lt__(self, other) -> bool:
        if not isinstance(other, Version):
            return False
        return self.cmp(other) == -1

    def __le__(self, other) -> bool:
        if not isinstance(other, Version):
            return False
        return self.cmp(other) <= 0

    def __eq__(self, other) -> bool:
        if not isinstance(other, Version):
            return False
        return self.cmp(other) == 0

    def __ge__(self, other) -> bool:
        if not isinstance(other, Version):
            return False
        return self.cmp(other) >= 0

    def __gt__(self, other) -> bool:
        if not isinstance(other, Version):
            return False
        return self.cmp(other) == 1

    def __str__(self) -> str:
        return self.as_str()

    def __repr__(self) -> str:
        return self.as_str_long()

    @property
    def _as_parameter_(self) -> _ffi.FimoVersion:
        return self._version

    def transfer_to_ffi(self) -> _ffi.FimoVersion:
        return self._version

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoVersion) -> Self:
        major = ffi.major.value
        minor = ffi.minor.value
        patch = ffi.patch.value
        build = ffi.build.value
        return cls(major, minor, patch, build)
