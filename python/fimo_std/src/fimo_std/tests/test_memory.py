import ctypes
import pytest

from ..error import Error
from ..memory import DefaultAllocator


def test_allocation():
    ptr = DefaultAllocator.malloc(0)
    assert isinstance(ptr, ctypes.c_void_p)
    assert not ptr

    ptr = DefaultAllocator.malloc(ctypes.sizeof(ctypes.c_longlong))
    assert isinstance(ptr, ctypes.c_void_p)
    assert ptr
    assert ptr.value % DefaultAllocator.minimum_alignment() == 0
    DefaultAllocator.free(ptr)

    ptr, size = DefaultAllocator.malloc_with_size(1339)
    assert isinstance(ptr, ctypes.c_void_p)
    assert isinstance(size, int)
    assert ptr
    assert size >= 1339
    assert ptr.value % DefaultAllocator.minimum_alignment() == 0
    DefaultAllocator.free(ptr)


def test_zero_allocation():
    ptr = DefaultAllocator.calloc(0)
    assert isinstance(ptr, ctypes.c_void_p)
    assert not ptr

    ptr = DefaultAllocator.calloc(10 * ctypes.sizeof(ctypes.c_longlong))
    assert isinstance(ptr, ctypes.c_void_p)
    assert ptr
    assert ptr.value % DefaultAllocator.minimum_alignment() == 0
    ptr = ctypes.cast(ptr, ctypes.POINTER(ctypes.c_longlong))
    for i in range(10):
        assert ptr[i] == 0
    DefaultAllocator.free_with_size(ptr, 10 * ctypes.sizeof(ctypes.c_longlong))

    ptr, size = DefaultAllocator.calloc_with_size(1339)
    assert isinstance(ptr, ctypes.c_void_p)
    assert isinstance(size, int)
    assert ptr
    assert size >= 1339
    ptr = ctypes.cast(ptr, ctypes.POINTER(ctypes.c_ubyte))
    for i in range(1339):
        assert ptr[i] == 0
    DefaultAllocator.free_with_size(ptr, size)


def test_aligned_allocation():
    # zero alignment
    with pytest.raises(Error):
        DefaultAllocator.aligned_alloc(0, 10)

    # non-power-of-two alignment
    with pytest.raises(Error):
        DefaultAllocator.aligned_alloc(17, 10)

    ptr = DefaultAllocator.aligned_alloc(256, ctypes.sizeof(ctypes.c_longlong))
    assert isinstance(ptr, ctypes.c_void_p)
    assert ptr
    assert ptr.value % 256 == 0
    DefaultAllocator.free_with_alignment_and_size(ptr, 256, ctypes.sizeof(ctypes.c_longlong))

    ptr, size = DefaultAllocator.aligned_alloc_with_size(256, 1339)
    assert isinstance(ptr, ctypes.c_void_p)
    assert isinstance(size, int)
    assert ptr
    assert ptr.value % 256 == 0
    assert size >= 1339
    DefaultAllocator.free_with_alignment_and_size(ptr, 256, size)
