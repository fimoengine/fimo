import pytest

from ..error import ErrorCode, Error, Result


def test_new():
    result = Result.new(None)
    assert result.is_ok()
    assert not result.is_error()

    result = Result.new("test error")
    assert result.is_error()
    print(result.name)
    print(result.description)


def test_error_code():
    for c in ErrorCode:
        result = Result.from_error_code(c)
        assert isinstance(result.is_ok(), bool)
        assert isinstance(result.is_error(), bool)
        if c == ErrorCode.EOK:
            assert result.is_ok()
            assert not result.is_error()
        else:
            assert not result.is_ok()
            assert result.is_error()


def test_raise_if_error():
    for c in ErrorCode:
        result = Result.from_error_code(c)
        if not result.is_error():
            result.raise_if_error()
        else:
            with pytest.raises(Error):
                result.raise_if_error()


def test_name():
    for c in ErrorCode:
        assert isinstance(c.name, str)
        print(c.name)

        result = Result.from_error_code(c)
        assert isinstance(result.name, str)
        print(result.name)


def test_description():
    for c in ErrorCode:
        assert isinstance(c.description, str)
        print(c.description)

        result = Result.from_error_code(c)
        assert isinstance(result.description, str)
        print(result.description)
