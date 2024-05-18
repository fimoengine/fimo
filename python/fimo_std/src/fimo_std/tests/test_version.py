from ..version import Version


def test_construction():
    version = Version(0, 1, 0)
    assert isinstance(version.major(), int)
    assert isinstance(version.minor(), int)
    assert isinstance(version.patch(), int)
    assert isinstance(version.build(), int)
    assert version.major() == 0
    assert version.minor() == 1
    assert version.patch() == 0
    assert version.build() == 0

    version = Version.parse_str('2.55.7')
    assert isinstance(version, Version)
    assert version.major() == 2
    assert version.minor() == 55
    assert version.patch() == 7
    assert version.build() == 0

    version = Version.parse_str('3.4.1+99')
    assert isinstance(version, Version)
    assert version.major() == 3
    assert version.minor() == 4
    assert version.patch() == 1
    assert version.build() == 99


def test_string():
    version = Version.parse_str('3.4.1+99')
    assert version.as_str() == '3.4.1'
    assert version.as_str_long() == '3.4.1+99'
    assert str(version) == '3.4.1'
    assert repr(version) == '3.4.1+99'

    assert version.string_length() >= 5
    assert version.string_length_long() >= 8


def test_compare():
    assert Version(0, 1, 0) == Version(0, 1, 0)
    assert Version(0, 1, 0) == Version(0, 1, 0, 99)
    assert Version(0, 1, 0) < Version(0, 1, 1)
    assert Version(0, 1, 0) < Version(0, 2, 0)
    assert Version(0, 1, 0) < Version(1, 0, 0)


def test_compatible():
    assert Version(0, 1, 0).is_compatible(Version(0, 1, 0))
    assert Version(0, 1, 1).is_compatible(Version(0, 1, 0))
    assert not Version(0, 2, 1).is_compatible(Version(0, 1, 0))

    assert not Version(1, 4, 0).is_compatible(Version(1, 5, 0, 10))
    assert Version(1, 5, 0).is_compatible(Version(1, 5, 0, 10))
    assert Version(1, 5, 0, 10).is_compatible(Version(1, 5, 0, 10))
    assert Version(1, 5, 1).is_compatible(Version(1, 5, 0, 10))
    assert Version(1, 9, 1).is_compatible(Version(1, 5, 0, 10))
