import pytest

from ..error import ErrorCode, Error


def test_is_valid():
    for c in ErrorCode:
        assert isinstance(c.is_valid(), bool)
        assert c.is_valid()


def test_is_error():
    for c in ErrorCode:
        assert isinstance(c.is_error(), bool)
        if c == ErrorCode.EOK:
            assert not c.is_error()
        else:
            assert c.is_error()


def test_raise_if_error():
    for c in ErrorCode:
        if not c.is_error():
            c.raise_if_error()
        else:
            with pytest.raises(Error):
                c.raise_if_error()


def test_name():
    for c in ErrorCode:
        assert isinstance(c.name(), str)
        print(c.name())


def test_description():
    for c in ErrorCode:
        assert isinstance(c.description(), str)
        print(c.description())
