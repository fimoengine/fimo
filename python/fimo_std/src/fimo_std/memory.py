import ctypes as c
import typing

from . import error
from . import ffi as _ffi

SizedBuffer = typing.Tuple[c.c_void_p, int]
"""Type of a sized allocated buffer."""


class DefaultAllocator:
    """Default raw memory allocator."""

    @staticmethod
    def minimum_alignment() -> int:
        """Returns the minimum alignment of the allocator."""
        return _ffi.FIMO_MALLOC_ALIGNMENT

    @staticmethod
    def malloc(size: int) -> c.c_void_p:
        """Allocate memory.

        This function allocates at least `size` bytes and returns a pointer to the allocated
        memory. The memory is not initialized. If `size` is `0`, then `DefaultAllocator.malloc()`
        returns `NULL`.

        :param size: size of the allocation
        :return: Allocated pointer
        :raises Error: Buffer could not be allocated
        """
        err = _ffi.FimoError(0)
        result = _ffi.fimo_malloc(c.c_size_t(size), c.byref(err))
        error.ErrorCode(err.value).raise_if_error()
        return result

    @staticmethod
    def malloc_with_size(size: int) -> SizedBuffer:
        """Allocate memory.

        This function allocates at least `size` bytes and returns a pointer to the allocated
        memory. The memory is not initialized. If `size` is `0`, then `DefaultAllocator.malloc()`
        returns `NULL`.

        :param size: size of the allocation
        :return:
            - Allocated pointer
            - Allocated size
        :raises Error: Buffer could not be allocated
        """
        err = _ffi.FimoError(0)
        result = _ffi.fimo_malloc_sized(c.c_size_t(size), c.byref(err))
        error.ErrorCode(err.value).raise_if_error()

        ptr = result.ptr
        buff_size = result.buff_size

        return c.c_void_p(ptr), buff_size

    @staticmethod
    def calloc(size: int) -> c.c_void_p:
        """Zero-allocate memory.

        This function allocates at least `size` bytes and returns a pointer to the allocated
        memory. The memory is zero-initialized. If `size` is `0`, then `DefaultAllocator.calloc()`
        returns `NULL`.

        :param size: size of the allocation
        :return: Allocated pointer
        :raises Error: Buffer could not be allocated
        """
        err = _ffi.FimoError(0)
        result = _ffi.fimo_calloc(c.c_size_t(size), c.byref(err))
        error.ErrorCode(err.value).raise_if_error()
        return result

    @staticmethod
    def calloc_with_size(size: int) -> SizedBuffer:
        """Zero-allocate memory.

        This function allocates at least `size` bytes and returns a pointer to the allocated
        memory. The memory is zero-initialized. If `size` is `0`, then `DefaultAllocator.calloc()`
        returns `NULL`.

        :param size: size of the allocation
        :return:
            - Allocated pointer
            - Allocated size
        :raises Error: Buffer could not be allocated
        """
        err = _ffi.FimoError(0)
        result = _ffi.fimo_calloc_sized(c.c_size_t(size), c.byref(err))
        error.ErrorCode(err.value).raise_if_error()

        ptr = result.ptr
        buff_size = result.buff_size

        return c.c_void_p(ptr), buff_size

    @staticmethod
    def aligned_alloc(alignment: int, size: int) -> c.c_void_p:
        """Allocate memory.

        This function allocates at least `size` bytes and returns a pointer to the allocated
        memory, along with the usable size in bytes. The memory is not initialized. If `size`
        is `0`, then `DefaultAllocator.aligned_alloc()` returns `NULL`.

        :param alignment: alignment of the allocation
        :param size: size of the allocation
        :return: Allocated pointer
        :raises Error: Buffer could not be allocated
        """
        err = _ffi.FimoError(0)
        result = _ffi.fimo_aligned_alloc(c.c_size_t(alignment), c.c_size_t(size), c.byref(err))
        error.ErrorCode(err.value).raise_if_error()
        return result

    @staticmethod
    def aligned_alloc_with_size(alignment: int, size: int) -> SizedBuffer:
        """Allocate memory.

        This function allocates at least `size` bytes and returns a pointer to the allocated
        memory, along with the usable size in bytes. The memory is not initialized. If `size`
        is `0`, then `DefaultAllocator.aligned_alloc()` returns `NULL`.

        :param alignment: alignment of the allocation
        :param size: size of the allocation
        :return:
            - Allocated pointer
            - Allocated size
        :raises Error: Buffer could not be allocated
        """
        err = _ffi.FimoError(0)
        result = _ffi.fimo_aligned_alloc_sized(c.c_size_t(alignment), c.c_size_t(size), c.byref(err))
        error.ErrorCode(err.value).raise_if_error()

        ptr = result.ptr
        buff_size = result.buff_size

        return c.c_void_p(ptr), buff_size

    @staticmethod
    def free(ptr: c.c_void_p) -> None:
        """Free allocated memory.

        Deallocates the memory allocated by an allocation function, If `ptr` is a null pointer
        no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
        allocation function, or if the space has been deallocated by a call to `DefaultAllocator.free()`,
        `DefaultAllocator.free_with_size()` or `DefaultAllocator.free_with_alignment_and_size()`, the
        behavior is undefined.

        :param ptr: pointer to the memory
        """
        _ffi.fimo_free(ptr)

    @staticmethod
    def free_with_size(ptr: c.c_void_p, size: int) -> None:
        """Free allocated memory.

        Deallocates the memory allocated by an allocation function, If `ptr` is a null pointer
        no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
        allocation function, or if the space has been deallocated by a call to `DefaultAllocator.free()`,
        `DefaultAllocator.free_with_size()` or `DefaultAllocator.free_with_alignment_and_size()`, or
        if `size` does not match the size used to allocate the memory, the behavior is undefined.

        :param ptr: pointer to the memory
        :param size: size of the allocation
        """
        _ffi.fimo_free_sized(ptr, c.c_size_t(size))

    @staticmethod
    def free_with_alignment_and_size(ptr: c.c_void_p, alignment: int, size: int) -> None:
        """Free allocated memory.

        Deallocates the memory allocated by an allocation function, If `ptr` is a null pointer
        no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
        allocation function, or if the space has been deallocated by a call to `DefaultAllocator.free()`,
        `DefaultAllocator.free_with_size()` or `DefaultAllocator.free_with_alignment_and_size()`, or
        if `alignment` and `size` do not match the alignment and size used to allocate the memory,
        the behavior is undefined.

        :param ptr: pointer to the memory
        :param alignment: alignment of the allocation
        :param size: size of the allocation
        """
        _ffi.fimo_free_aligned_sized(ptr, c.c_size_t(alignment), c.c_size_t(size))
