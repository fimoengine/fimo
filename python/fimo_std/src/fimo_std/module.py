from __future__ import annotations

import ctypes as c
from enum import IntEnum
from abc import ABC, abstractmethod
from typing import Self, Optional, Generic, TypeVar, TYPE_CHECKING

from .enum import ABCEnum
from .version import Version
from . import error
from . import context
from . import ffi as _ffi

if TYPE_CHECKING:
    from .context import Context as _Context, ContextView as _ContextView


class ModuleInfoView(_ffi.FFISharable[_ffi.Pointer[_ffi.FimoModuleInfo], "ModuleInfoView"]):
    """View of a `ModuleInfo`."""

    def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModuleInfo]) -> None:
        if not isinstance(ffi, c.POINTER(_ffi.FimoModuleInfo)):
            raise TypeError("`ffi` must be an instance of `FimoModuleInfo*`")
        if not ffi:
            raise ValueError("`ffi` may not be `null`")

        self._ffi: Optional[_ffi.Pointer[_ffi.FimoModuleInfo]] = ffi

    @property
    def ffi(self) -> _ffi.Pointer[_ffi.FimoModuleInfo]:
        if self._ffi is None:
            raise ValueError("the object has already been consumed")

        return self._ffi

    @classmethod
    def borrow_from_ffi(cls, ffi: _ffi.Pointer[_ffi.FimoModuleInfo]) -> ModuleInfoView:
        return ModuleInfoView(ffi)

    @property
    def _as_parameter_(self) -> _ffi.Pointer[_ffi.FimoModuleInfo]:
        return self.ffi

    @property
    def name(self) -> str:
        """Unique module name."""
        value = self.ffi.contents.name.value
        assert isinstance(value, bytes)
        return value.decode()

    @property
    def description(self) -> Optional[str]:
        """Module description."""
        value = self.ffi.contents.description.value
        assert isinstance(value, bytes) or value is None

        if value is None:
            return None
        else:
            return value.decode()

    @property
    def author(self) -> Optional[str]:
        """Module author."""
        value = self.ffi.contents.author.value
        assert isinstance(value, bytes) or value is None

        if value is None:
            return None
        else:
            return value.decode()

    @property
    def license(self) -> Optional[str]:
        """Module author."""
        value = self.ffi.contents.license.value
        assert isinstance(value, bytes) or value is None

        if value is None:
            return None
        else:
            return value.decode()

    @property
    def module_path(self) -> str:
        """Path to the module directory."""
        value = self.ffi.contents.module_path.value
        assert isinstance(value, bytes)
        return value.decode()

    def unload(self, ctx: _ContextView) -> None:
        """Unloads the module.

        If successful, this function unloads the module. To succeed, no other module may
        depend on the module. This function automatically cleans up unreferenced modules,
        except if they are a pseudo module.
        """
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")

        err = _ffi.fimo_module_unload(ctx.ffi, self.ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def is_loaded(self) -> bool:
        """Checks whether the underlying module is still loaded."""
        return _ffi.fimo_impl_module_info_is_loaded(self.ffi)

    def __enter__(self) -> Self:
        """Locks the underlying module from being unloaded.

        The module may be locked multiple times.
        """
        err = _ffi.fimo_impl_module_info_lock_unload(self.ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Unlocks the underlying module, allowing it to be unloaded again."""
        _ffi.fimo_impl_module_info_unlock_unload(self.ffi)

    def acquire(self) -> ModuleInfo:
        """Acquires the module info by increasing the reference count."""
        _ffi.fimo_impl_module_info_acquire(self.ffi)
        return ModuleInfo.transfer_from_ffi(self.ffi)

    def _consume(self) -> None:
        if self._ffi is None:
            raise ValueError("the object has already been consumed")
        self._ffi = None


class ModuleInfo(ModuleInfoView, _ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleInfo]]):
    """Public handle to a loaded module."""

    def __del__(self):
        if self._ffi is not None:
            _ffi.fimo_impl_module_info_release(self.ffi)
            self._consume()

    def transfer_to_ffi(self) -> _ffi.Pointer[_ffi.FimoModuleInfo]:
        ffi = self.ffi
        self._consume()
        return ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.Pointer[_ffi.FimoModuleInfo]) -> Self:
        return cls(ffi)

    @classmethod
    def find_by_name(cls, ctx: _ContextView, name: str) -> Self:
        """Searches for a module by its name."""
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")
        if not isinstance(name, str):
            raise TypeError("`name` must be an instance of `str`")

        module_ffi = c.POINTER(_ffi.FimoModuleInfo)()
        name_ffi = c.c_char_p(name.encode())
        err = _ffi.fimo_module_find_by_name(ctx.ffi, name_ffi, c.byref(module_ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        return cls(module_ffi)

    @classmethod
    def find_by_symbol(cls, ctx: _ContextView, name: str, namespace: str, version: Version) -> Self:
        """Searches for a module by a symbol it exports."""
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")
        if not isinstance(name, str):
            raise TypeError("`name` must be an instance of `str`")
        if not isinstance(namespace, str):
            raise TypeError("`namespace` must be an instance of `str`")
        if not isinstance(version, Version):
            raise TypeError("`version` must be an instance of `Version`")

        module_ffi = c.POINTER(_ffi.FimoModuleInfo)()
        name_ffi = c.c_char_p(name.encode())
        namespace_ffi = c.c_char_p(namespace.encode())
        version_ffi = version.transfer_to_ffi()
        err = _ffi.fimo_module_find_by_symbol(ctx.ffi, name_ffi, namespace_ffi, version_ffi, c.byref(module_ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        return cls(module_ffi)


class DependencyType(IntEnum):
    """Type of dependency between a module and a namespace."""
    Static = 0
    Dynamic = 1


_T = TypeVar("_T", bound=_ffi.FFITransferable[c.c_void_p])


class RawSymbol:
    """A type-erased symbol from the module subsystem."""

    def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModuleRawSymbol]):
        if not isinstance(ffi, c.POINTER(_ffi.FimoModuleRawSymbol)):
            raise TypeError("`ffi` must be an instance of `FimoModuleRawSymbol*`")
        if not ffi:
            raise ValueError("`ffi` may not be `null`")

        self._ffi = ffi

    def in_use(self) -> bool:
        lock = self._ffi.contents.lock
        return _ffi.fimo_impl_module_symbol_is_used(c.byref(lock))

    def __enter__(self) -> c.c_void_p:
        lock = self._ffi.contents.lock
        data = c.c_void_p(self._ffi.contents.data)
        _ffi.fimo_impl_module_symbol_acquire(c.byref(lock))

        return data

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        lock = self._ffi.contents.lock
        _ffi.fimo_impl_module_symbol_release(c.byref(lock))


class Symbol(Generic[_T], ABC):
    """A symbol from the module subsystem."""

    def __init__(self, sym: RawSymbol) -> None:
        if not isinstance(sym, RawSymbol):
            raise TypeError("`symbol` must be an instance of `RawSymbol`")

        self._sym = sym

    @staticmethod
    @abstractmethod
    def _symbol_name() -> str:
        """Name of the symbol."""
        pass

    @staticmethod
    @abstractmethod
    def _symbol_namespace() -> str:
        """Namespace of the symbol."""
        pass

    @staticmethod
    @abstractmethod
    def _symbol_version() -> Version:
        """Version of the symbol."""
        pass

    @staticmethod
    @abstractmethod
    def _symbol_type() -> type[_T]:
        """Returns the type of the symbol."""
        pass

    def __enter__(self) -> _T:
        """Locks the symbol so that it may be used."""
        sym = self._sym.__enter__()
        return self._symbol_type().transfer_from_ffi(sym)

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Unlocks the symbol."""
        self._sym.__exit__(exc_type, exc_val, exc_tb)


def symbol(*, sym_type: type[_T], name: str, namespace: Optional[str], version: Version) -> type[Symbol[_T]]:
    """Defines a new symbol type."""
    if not isinstance(sym_type, type):
        raise TypeError("`sym_type` must be an instance of `type`")
    if not isinstance(name, str):
        raise TypeError("`name` must be an instance of `str`")
    if not isinstance(namespace, str) and namespace is not None:
        raise TypeError("`namespace` must be an instance of `str` or be `None`")
    if not isinstance(version, Version):
        raise TypeError("`version` must be an instance of `Version`")

    ns = namespace if namespace is not None else ""

    class _TypedSymbol(Symbol[_T]):
        @staticmethod
        def _symbol_name() -> str:
            return name

        @staticmethod
        def _symbol_namespace() -> str:
            return ns

        @staticmethod
        def _symbol_version() -> Version:
            return version

        @staticmethod
        def _symbol_type() -> type[_T]:
            return sym_type

    return _TypedSymbol


_Parameters = TypeVar("_Parameters", bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleParamTable]])
_Resources = TypeVar("_Resources", bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleResourceTable]])
_Imports = TypeVar("_Imports", bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleSymbolImportTable]])
_Exports = TypeVar("_Exports", bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleSymbolExportTable]])
_Data = TypeVar("_Data", bound=_ffi.FFITransferable[_ffi.Pointer[c.c_void_p]])


class _ModuleBase(Generic[_Parameters, _Resources, _Imports, _Exports, _Data],
                  _ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModule]], ABC):
    """Base class of all modules."""

    def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModule]):
        if not isinstance(ffi, c.POINTER(_ffi.FimoModule)):
            raise TypeError("`ffi` must be an instance of `FimoModule*`")
        if not ffi:
            raise ValueError("`ffi` may not be `null`")

        self._ffi: _ffi.Pointer[_ffi.FimoModule] = ffi

    def transfer_to_ffi(self) -> _ffi.Pointer[_ffi.FimoModule]:
        return self._ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.Pointer[_ffi.FimoModule]) -> Self:
        return cls(ffi)

    @abstractmethod
    def parameters(self) -> _Parameters:
        """Fetches the parameter table of the module."""
        pass

    @abstractmethod
    def resources(self) -> _Resources:
        """Fetches the resource path table of the module."""
        pass

    @abstractmethod
    def imports(self) -> _Imports:
        """Fetches the symbol import table of the module."""
        pass

    @abstractmethod
    def exports(self) -> _Exports:
        """Fetches the symbol export table of the module."""
        pass

    def module_info(self) -> ModuleInfoView:
        """Fetches the module info."""
        module_info_ffi = self._ffi.contents.module_info
        return ModuleInfoView(module_info_ffi)

    def context(self) -> _ContextView:
        """Fetches the context of the module."""
        context_ffi = self._ffi.contents.context
        return context.ContextView.borrow_from_ffi(context_ffi)

    @abstractmethod
    def data(self) -> _Data:
        """Fetches the data of the module."""
        pass

    def has_namespace_dependency(self, namespace: str) -> Optional[DependencyType]:
        """Checks if a module includes a namespace.

        Checks if the module specified that it includes the namespace `namespace`. In that case, the
        module is allowed access to the symbols in the namespace.
        """
        if not isinstance(namespace, str):
            raise TypeError("`namespace` must be an instance of `str`")

        namespace_ffi = c.c_char_p(namespace.encode())
        is_included_ffi = c.c_bool(False)
        is_static_ffi = c.c_bool(False)
        err = _ffi.fimo_module_namespace_included(self._ffi, namespace_ffi, c.byref(is_included_ffi),
                                                  c.byref(is_static_ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        if not is_included_ffi.value:
            return None
        elif is_static_ffi.value:
            return DependencyType.Static
        else:
            return DependencyType.Dynamic

    def include_namespace(self, namespace: str) -> None:
        """Includes a namespace by the module.

        Once included, the module gains access to the symbols of its dependencies that are exposed
        in said namespace. A namespace can not be included multiple times.
        """
        if not isinstance(namespace, str):
            raise TypeError("`namespace` must be an instance of `str`")

        namespace_ffi = c.c_char_p(namespace.encode())
        err = _ffi.fimo_module_namespace_include(self._ffi, namespace_ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def exclude_namespace(self, namespace: str) -> None:
        """Removes a namespace from the module.

        Once excluded, the caller guarantees to relinquish access to the symbols contained in said
        namespace. It is only possible to exclude namespaces that were manually added, whereas
        static namespace includes remain valid until the module is unloaded.
        """
        if not isinstance(namespace, str):
            raise TypeError("`namespace` must be an instance of `str`")

        namespace_ffi = c.c_char_p(namespace.encode())
        err = _ffi.fimo_module_namespace_exclude(self._ffi, namespace_ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def has_dependency(self, module: ModuleInfoView) -> Optional[DependencyType]:
        """Checks if a module depends on another module.

        Checks if `module` is a dependency of the current instance. In that case the instance is
        allowed to access the symbols exported by `module`.
        """
        if not isinstance(module, ModuleInfoView):
            raise TypeError("`module` must be an instance of `ModuleInfoView`")

        module_ffi = module.ffi
        has_dependency_ffi = c.c_bool(False)
        is_static_ffi = c.c_bool(False)
        err = _ffi.fimo_module_has_dependency(self._ffi, module_ffi, c.byref(has_dependency_ffi),
                                              c.byref(is_static_ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        if not has_dependency_ffi.value:
            return None
        elif is_static_ffi.value:
            return DependencyType.Static
        else:
            return DependencyType.Dynamic

    def acquire_dependency(self, module: ModuleInfoView) -> None:
        """Acquires another module as a dependency.

        After acquiring a module as a dependency, the module is allowed access to the symbols and
        protected parameters of said dependency. Trying to acquire a dependency to a module that is
        already a dependency, or to a module that would result in a circular dependency will result
        in an error.
        """
        if not isinstance(module, ModuleInfoView):
            raise TypeError("`module` must be an instance of `ModuleInfoView`")

        module_ffi = module.ffi
        err = _ffi.fimo_module_acquire_dependency(self._ffi, module_ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def remove_dependency(self, module: ModuleInfoView) -> None:
        """Removes a module as a dependency.

        By removing a module as a dependency, the caller ensures that it does not own any references
        to resources originating from the former dependency, and allows for the unloading of the
        module. A module can only relinquish dependencies to modules that were acquired dynamically,
        as static dependencies remain valid until the module is unloaded.
        """
        if not isinstance(module, ModuleInfoView):
            raise TypeError("`module` must be an instance of `ModuleInfoView`")

        module_ffi = module.ffi
        err = _ffi.fimo_module_relinquish_dependency(self._ffi, module_ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    # noinspection PyProtectedMember
    def load_symbol(self, sym_type: type[Symbol[_T]]) -> Symbol[_T]:
        """Loads a symbol from the module subsystem.

        The caller can query the backend for a symbol of a loaded module. This is useful for loading
        optional symbols, or for loading symbols after the creation of a module. The symbol, if it
        exists, is returned, and can be used until the module relinquishes the dependency to the
        module that exported the symbol. This function fails, if the module containing the symbol is
        not a dependency of the module, or if the module has not included the required namespace.
        """
        if not isinstance(sym_type, type):
            raise TypeError("`sym_type` must be an instance of `type`")

        name = sym_type._symbol_name()
        namespace = sym_type._symbol_namespace()
        version = sym_type._symbol_version()
        symbol = self.load_raw_symbol(name, namespace, version)
        return sym_type(symbol)

    def load_raw_symbol(self, name: str, namespace: str, version: Version) -> RawSymbol:
        """Loads a symbol from the module subsystem.

        The caller can query the backend for a symbol of a loaded module. This is useful for loading
        optional symbols, or for loading symbols after the creation of a module. The symbol, if it
        exists, is returned, and can be used until the module relinquishes the dependency to the
        module that exported the symbol. This function fails, if the module containing the symbol is
        not a dependency of the module, or if the module has not included the required namespace.
        """
        if not isinstance(name, str):
            raise TypeError("`name` must be an instance of `str`")
        if not isinstance(namespace, str):
            raise TypeError("`namespace` must be an instance of `str`")
        if not isinstance(version, Version):
            raise TypeError("`version` must be an instance of `Version`")

        name_ffi = c.c_char_p(name.encode())
        namespace_ffi = c.c_char_p(namespace.encode())
        version_ffi = version.transfer_to_ffi()
        symbol_ffi = c.POINTER(_ffi.FimoModuleRawSymbol)()
        err = _ffi.fimo_module_load_symbol(self._ffi, name_ffi, namespace_ffi, version_ffi, c.byref(symbol_ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        return RawSymbol(symbol_ffi)


class ParameterType(_ffi.FFITransferable[_ffi.FimoModuleParamType], IntEnum, metaclass=ABCEnum):
    """Datatype of a module parameter."""
    U8 = 0
    U16 = 1
    U32 = 2
    U64 = 3
    I8 = 4
    I16 = 5
    I32 = 6
    I64 = 7

    def transfer_to_ffi(self) -> _ffi.FimoModuleParamType:
        return _ffi.FimoModuleParamType(self)

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoModuleParamType) -> Self:
        return cls(ffi.value)

    @classmethod
    def from_param(cls, obj):
        return cls(obj)


class ParameterAccess(_ffi.FFITransferable[_ffi.FimoModuleParamAccess], IntEnum, metaclass=ABCEnum):
    """Access group of a module parameter."""
    Public = 0
    Dependency = 1
    Private = 2

    def transfer_to_ffi(self) -> _ffi.FimoModuleParamAccess:
        return _ffi.FimoModuleParamAccess(self)

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoModuleParamAccess) -> Self:
        return cls(ffi.value)

    @classmethod
    def from_param(cls, obj):
        return cls(obj)


class ParameterValue:
    """Value of a module parameter."""

    def __init__(self, value: int, type: ParameterType) -> None:
        if not isinstance(value, int):
            raise TypeError("`value` must be an `int`")
        if not isinstance(type, ParameterType):
            raise TypeError("`type` must be an instance of `ParameterType`")

        self._value = value
        self._type = type

    @property
    def value(self) -> int:
        return self._value

    @property
    def type(self) -> ParameterType:
        return self._type

    @classmethod
    def read_public(cls, ctx: _ContextView, module: str, parameter: str) -> Self:
        """Reads a module parameter with public read access

        Reads the value of a module parameter with public read access. The operation fails, if
        the parameter does not exist, or if the parameter does not allow reading with a public
        access.

        :param ctx: context
        :param module: module containing the parameter
        :param parameter: parameter name

        :return: Parameter value.
        """
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")
        if not isinstance(module, str):
            raise TypeError("`module` must be an instance of `str`")
        if not isinstance(parameter, str):
            raise TypeError("`module` must be an instance of `str`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = _ffi.FimoModuleParamType()
        module_ffi = c.c_char_p(module.encode())
        parameter_ffi = c.c_char_p(parameter.encode())

        err = _ffi.fimo_module_param_get_public(ctx.ffi, value_ffi_ptr, c.byref(type_ffi), module_ffi, parameter_ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        type = ParameterType.transfer_from_ffi(type_ffi)

        match type:
            case ParameterType.U8:
                return cls(value_ffi.u8.value, type)
            case ParameterType.U16:
                return cls(value_ffi.u16.value, type)
            case ParameterType.U32:
                return cls(value_ffi.u32.value, type)
            case ParameterType.U64:
                return cls(value_ffi.u64.value, type)
            case ParameterType.I8:
                return cls(value_ffi.i8.value, type)
            case ParameterType.I16:
                return cls(value_ffi.i16.value, type)
            case ParameterType.I32:
                return cls(value_ffi.i32.value, type)
            case ParameterType.I64:
                return cls(value_ffi.i64.value, type)
            case _:
                raise ValueError("unknown parameter type")

    def write_public(self, ctx: _ContextView, module: str, parameter: str) -> None:
        """Writes a module parameter with public write access.

        Sets the value of a module parameter with public write access. The operation fails, if
        the parameter does not exist, or if the parameter does not allow writing with a public
        access.

        :param ctx: context
        :param module: module containing the parameter
        :param parameter: parameter name
        """
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")
        if not isinstance(module, str):
            raise TypeError("`module` must be an instance of `str`")
        if not isinstance(parameter, str):
            raise TypeError("`module` must be an instance of `str`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = self.type.transfer_to_ffi()
        module_ffi = c.c_char_p(module.encode())
        parameter_ffi = c.c_char_p(parameter.encode())

        match self.type:
            case ParameterType.U8:
                value_ffi.u8 = _ffi.FimoU8(self.value)
            case ParameterType.U16:
                value_ffi.u16 = _ffi.FimoU16(self.value)
            case ParameterType.U32:
                value_ffi.u32 = _ffi.FimoU32(self.value)
            case ParameterType.U64:
                value_ffi.u64 = _ffi.FimoU64(self.value)
            case ParameterType.I8:
                value_ffi.i8 = _ffi.FimoI8(self.value)
            case ParameterType.I16:
                value_ffi.i16 = _ffi.FimoI16(self.value)
            case ParameterType.I32:
                value_ffi.i32 = _ffi.FimoI32(self.value)
            case ParameterType.I64:
                value_ffi.i64 = _ffi.FimoI64(self.value)

        err = _ffi.fimo_module_param_set_public(ctx.ffi, value_ffi_ptr, type_ffi, module_ffi, parameter_ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
