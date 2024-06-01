from __future__ import annotations

import ctypes as c
from enum import IntEnum
from abc import ABC, abstractmethod
from typing import (
    Self,
    Any,
    Optional,
    Generic,
    TypeVar,
    NewType,
    Callable,
    TYPE_CHECKING,
)

from ._enum import ABCEnum
from .version import Version
from . import error
from . import context
from . import ffi as _ffi

if TYPE_CHECKING:
    from .context import Context as _Context, ContextView as _ContextView


class ModuleInfoView(
    _ffi.FFISharable[_ffi.Pointer[_ffi.FimoModuleInfo], "ModuleInfoView"]
):
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


class ModuleInfo(
    ModuleInfoView, _ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleInfo]]
):
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
    def find_by_symbol(
        cls, ctx: _ContextView, name: str, namespace: str, version: Version
    ) -> Self:
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
        err = _ffi.fimo_module_find_by_symbol(
            ctx.ffi, name_ffi, namespace_ffi, version_ffi, c.byref(module_ffi)
        )
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


def symbol(
    *, sym_type: type[_T], name: str, namespace: Optional[str], version: Version
) -> type[Symbol[_T]]:
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


_OpaqueT = TypeVar("_OpaqueT")


class _OpaqueType(Generic[_OpaqueT], _ffi.FFITransferable[_OpaqueT]):
    def __init__(self, ffi: _OpaqueT):
        self._ffi = ffi

    def transfer_to_ffi(self) -> _OpaqueT:
        return self._ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _OpaqueT) -> Self:
        return cls(ffi)


_OpaqueParameters = NewType(
    "_OpaqueParameters", _OpaqueType[_ffi.Pointer[_ffi.FimoModuleParamTable]]
)
_OpaqueResources = NewType(
    "_OpaqueResources", _OpaqueType[_ffi.Pointer[_ffi.FimoModuleResourceTable]]
)
_OpaqueImports = NewType(
    "_OpaqueImports", _OpaqueType[_ffi.Pointer[_ffi.FimoModuleSymbolImportTable]]
)
_OpaqueExports = NewType(
    "_OpaqueExports", _OpaqueType[_ffi.Pointer[_ffi.FimoModuleSymbolExportTable]]
)
_OpaqueData = NewType("_OpaqueData", _OpaqueType[c.c_void_p])

_Parameters = TypeVar(
    "_Parameters", bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleParamTable]]
)
_Resources = TypeVar(
    "_Resources", bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleResourceTable]]
)
_Imports = TypeVar(
    "_Imports",
    bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleSymbolImportTable]],
)
_Exports = TypeVar(
    "_Exports",
    bound=_ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleSymbolExportTable]],
)
_Data = TypeVar("_Data", bound=_ffi.FFITransferable[c.c_void_p])


class _ModuleBase(
    Generic[_Parameters, _Resources, _Imports, _Exports, _Data],
    _ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModule]],
    _ffi.FFISharable[_ffi.Pointer[_ffi.FimoModule], "_OpaqueModule"],
    ABC,
):
    """Base class of all modules."""

    def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModule]):
        if not isinstance(ffi, c.POINTER(_ffi.FimoModule)):
            raise TypeError("`ffi` must be an instance of `FimoModule*`")
        if not ffi:
            raise ValueError("`ffi` may not be `null`")

        self._ffi: Optional[_ffi.Pointer[_ffi.FimoModule]] = ffi

    def transfer_to_ffi(self) -> _ffi.Pointer[_ffi.FimoModule]:
        ffi = self.ffi
        self._consume()
        return ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.Pointer[_ffi.FimoModule]) -> Self:
        return cls(ffi)

    @property
    def ffi(self) -> _ffi.Pointer[_ffi.FimoModule]:
        if self._ffi is None:
            raise ValueError("the object has already been consumed")

        return self._ffi

    @classmethod
    def borrow_from_ffi(cls, ffi: _ffi.Pointer[_ffi.FimoModule]) -> _OpaqueModule:
        return _OpaqueModule(ffi)

    def _consume(self) -> None:
        self._ffi = None

    @staticmethod
    @abstractmethod
    def _parameters_type() -> type[_Parameters]:
        """Returns the type of the parameter table"""
        pass

    @staticmethod
    @abstractmethod
    def _resources_type() -> type[_Resources]:
        """Returns the type of the resources table"""
        pass

    @staticmethod
    @abstractmethod
    def _imports_type() -> type[_Imports]:
        """Returns the type of the imports table"""
        pass

    @staticmethod
    @abstractmethod
    def _exports_type() -> type[_Exports]:
        """Returns the type of the exports table"""
        pass

    @staticmethod
    @abstractmethod
    def _data_type() -> type[_Data]:
        """Returns the type of the module data"""
        pass

    def parameters(self) -> _Parameters:
        """Fetches the parameter table of the module."""
        parameters_ffi: _ffi.Pointer[_ffi.FimoModuleParamTable] = (
            self.ffi.contents.parameters
        )
        return self._parameters_type().transfer_from_ffi(parameters_ffi)

    def resources(self) -> _Resources:
        """Fetches the resource path table of the module."""
        resources_ffi: _ffi.Pointer[_ffi.FimoModuleResourceTable] = (
            self.ffi.contents.resources
        )
        return self._resources_type().transfer_from_ffi(resources_ffi)

    def imports(self) -> _Imports:
        """Fetches the symbol import table of the module."""
        imports_ffi: _ffi.Pointer[_ffi.FimoModuleSymbolImportTable] = (
            self.ffi.contents.imports
        )
        return self._imports_type().transfer_from_ffi(imports_ffi)

    def exports(self) -> _Exports:
        """Fetches the symbol export table of the module."""
        exports_ffi: _ffi.Pointer[_ffi.FimoModuleSymbolExportTable] = (
            self.ffi.contents.exports
        )
        return self._exports_type().transfer_from_ffi(exports_ffi)

    def module_info(self) -> ModuleInfoView:
        """Fetches the module info."""
        module_info_ffi: _ffi.Pointer[_ffi.FimoModuleInfo] = (
            self.ffi.contents.module_info
        )
        return ModuleInfoView(module_info_ffi)

    def context(self) -> _ContextView:
        """Fetches the context of the module."""
        context_ffi: _ffi.FimoContext = self.ffi.contents.context
        return context.ContextView.borrow_from_ffi(context_ffi)

    def data(self) -> _Data:
        """Fetches the data of the module."""
        data_ffi: c.c_void_p = c.c_void_p(self.ffi.contents.module_data)
        return self._data_type().transfer_from_ffi(data_ffi)

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
        err = _ffi.fimo_module_namespace_included(
            self.ffi, namespace_ffi, c.byref(is_included_ffi), c.byref(is_static_ffi)
        )
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
        err = _ffi.fimo_module_namespace_include(self.ffi, namespace_ffi)
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
        err = _ffi.fimo_module_namespace_exclude(self.ffi, namespace_ffi)
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
        err = _ffi.fimo_module_has_dependency(
            self.ffi, module_ffi, c.byref(has_dependency_ffi), c.byref(is_static_ffi)
        )
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
        err = _ffi.fimo_module_acquire_dependency(self.ffi, module_ffi)
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
        err = _ffi.fimo_module_relinquish_dependency(self.ffi, module_ffi)
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
        err = _ffi.fimo_module_load_symbol(
            self.ffi, name_ffi, namespace_ffi, version_ffi, c.byref(symbol_ffi)
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        return RawSymbol(symbol_ffi)


class _OpaqueModule(
    _ModuleBase[
        _OpaqueParameters, _OpaqueResources, _OpaqueImports, _OpaqueExports, _OpaqueData
    ]
):
    @staticmethod
    def _parameters_type() -> type[_OpaqueParameters]:
        """Returns the type of the parameter table"""
        return _OpaqueParameters

    @staticmethod
    def _resources_type() -> type[_OpaqueResources]:
        """Returns the type of the resources table"""
        return _OpaqueResources

    @staticmethod
    def _imports_type() -> type[_OpaqueImports]:
        """Returns the type of the imports table"""
        return _OpaqueImports

    @staticmethod
    def _exports_type() -> type[_OpaqueExports]:
        """Returns the type of the exports table"""
        return _OpaqueExports

    @staticmethod
    def _data_type() -> type[_OpaqueData]:
        """Returns the type of the module data"""
        return _OpaqueData


class PseudoModule(_OpaqueModule):
    """A pseudo module.

    The functions of the module backend require that the caller owns
    a reference to their own module. This is a problem, as the constructor
    of the context won't be assigned a module instance during bootstrapping.
    As a workaround, we allow for the creation of pseudo modules, i.e.,
    module handles without an associated module.
    """

    def __init__(self, ctx: _ContextView) -> None:
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")

        module_ffi = c.POINTER(_ffi.FimoModule)()
        err = _ffi.fimo_module_pseudo_module_new(ctx.ffi, c.byref(module_ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        super().__init__(module_ffi)

    def __del__(self) -> None:
        if self._ffi is not None:
            self.destroy()

    def destroy(self) -> _Context:
        """Destroys the `PseudoModule."""
        ctx_ffi = _ffi.FimoContext()
        err = _ffi.fimo_module_pseudo_module_destroy(self.ffi, c.byref(ctx_ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        self._consume()
        return context.Context.transfer_from_ffi(ctx_ffi)


class ParameterType(
    _ffi.FFITransferable[_ffi.FimoModuleParamType], IntEnum, metaclass=ABCEnum
):
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


class ParameterAccess(
    _ffi.FFITransferable[_ffi.FimoModuleParamAccess], IntEnum, metaclass=ABCEnum
):
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


class Parameter(_ffi.FFISharable[_ffi.Pointer[_ffi.FimoModuleParam], "Parameter"]):
    """A module parameter."""

    def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModuleParam]):
        if not isinstance(ffi, c.POINTER(_ffi.FimoModuleParam)):
            raise TypeError("`ffi` must be an instance of `FimoModuleParam*`")
        if not ffi:
            raise ValueError("`ffi` may not be `null`")

        self._ffi = ffi

    @property
    def ffi(self) -> _ffi.Pointer[_ffi.FimoModuleParam]:
        return self._ffi

    @classmethod
    def borrow_from_ffi(cls, ffi: _ffi.Pointer[_ffi.FimoModuleParam]) -> Parameter:
        return Parameter(ffi)


class ParameterData(
    _ffi.FFISharable[_ffi.Pointer[_ffi.FimoModuleParamData], "ParameterData"]
):
    """A module parameter."""

    def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModuleParamData]):
        if not isinstance(ffi, c.POINTER(_ffi.FimoModuleParamData)):
            raise TypeError("`ffi` must be an instance of `FimoModuleParamData*`")
        if not ffi:
            raise ValueError("`ffi` may not be `null`")

        self._ffi = ffi

    @property
    def ffi(self) -> _ffi.Pointer[_ffi.FimoModuleParamData]:
        return self._ffi

    @classmethod
    def borrow_from_ffi(
        cls, ffi: _ffi.Pointer[_ffi.FimoModuleParamData]
    ) -> ParameterData:
        return ParameterData(ffi)


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
            raise TypeError("`parameter` must be an instance of `str`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = _ffi.FimoModuleParamType()
        module_ffi = c.c_char_p(module.encode())
        parameter_ffi = c.c_char_p(parameter.encode())

        err = _ffi.fimo_module_param_get_public(
            ctx.ffi, value_ffi_ptr, c.byref(type_ffi), module_ffi, parameter_ffi
        )
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

    @classmethod
    def read_dependency(
        cls, caller: _OpaqueModule, module: str, parameter: str
    ) -> Self:
        """Reads a module parameter with dependency read access.

        Reads the value of a module parameter with dependency read access. The operation fails, if
        the parameter does not exist, or if the parameter does not allow reading with a dependency
        access.

        :param caller: caller module
        :param module: module containing the parameter
        :param parameter: parameter name

        :return: Parameter value.
        """
        if not isinstance(caller, _OpaqueModule):
            raise TypeError("`caller` must be an instance of `_OpaqueModule`")
        if not isinstance(module, str):
            raise TypeError("`module` must be an instance of `str`")
        if not isinstance(parameter, str):
            raise TypeError("`parameter` must be an instance of `str`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = _ffi.FimoModuleParamType()
        module_ffi = c.c_char_p(module.encode())
        parameter_ffi = c.c_char_p(parameter.encode())

        err = _ffi.fimo_module_param_get_dependency(
            caller.ffi, value_ffi_ptr, c.byref(type_ffi), module_ffi, parameter_ffi
        )
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

    @classmethod
    def read_private(cls, caller: _OpaqueModule, parameter: Parameter) -> Self:
        """Reads a module parameter with private read access.

        :param caller: caller module
        :param parameter: parameter

        :return: Parameter value.
        """
        if not isinstance(caller, _OpaqueModule):
            raise TypeError("`caller` must be an instance of `_OpaqueModule`")
        if not isinstance(parameter, Parameter):
            raise TypeError("`parameter` must be an instance of `Parameter`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = _ffi.FimoModuleParamType()
        parameter_ffi = parameter.ffi

        err = _ffi.fimo_module_param_get_private(
            caller.ffi, value_ffi_ptr, c.byref(type_ffi), parameter_ffi
        )
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

    @classmethod
    def read_inner(cls, caller: _OpaqueModule, parameter: ParameterData) -> Self:
        """Reads a module parameter.

        :param caller: caller module
        :param parameter: parameter

        :return: Parameter value.
        """
        if not isinstance(caller, _OpaqueModule):
            raise TypeError("`caller` must be an instance of `_OpaqueModule`")
        if not isinstance(parameter, ParameterData):
            raise TypeError("`parameter` must be an instance of `ParameterData`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = _ffi.FimoModuleParamType()
        parameter_ffi = parameter.ffi

        err = _ffi.fimo_module_param_get_inner(
            caller.ffi, value_ffi_ptr, c.byref(type_ffi), parameter_ffi
        )
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
            raise TypeError("`parameter` must be an instance of `str`")

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

        err = _ffi.fimo_module_param_set_public(
            ctx.ffi, value_ffi_ptr, type_ffi, module_ffi, parameter_ffi
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def write_dependency(
        self, caller: _OpaqueModule, module: str, parameter: str
    ) -> None:
        """Writes a module parameter with dependency write access.

        Sets the value of a module parameter with dependency write access. The operation fails, if
        the parameter does not exist, or if the parameter does not allow writing with a dependency
        access.

        :param caller: caller module
        :param module: module containing the parameter
        :param parameter: parameter name
        """
        if not isinstance(caller, _OpaqueModule):
            raise TypeError("`caller` must be an instance of `_OpaqueModule`")
        if not isinstance(module, str):
            raise TypeError("`module` must be an instance of `str`")
        if not isinstance(parameter, str):
            raise TypeError("`parameter` must be an instance of `str`")

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

        err = _ffi.fimo_module_param_set_dependency(
            caller.ffi, value_ffi_ptr, type_ffi, module_ffi, parameter_ffi
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def write_private(self, caller: _OpaqueModule, parameter: Parameter) -> None:
        """Writes a module parameter with private write access.

        :param caller: caller module
        :param parameter: parameter
        """
        if not isinstance(caller, _OpaqueModule):
            raise TypeError("`caller` must be an instance of `_OpaqueModule`")
        if not isinstance(parameter, Parameter):
            raise TypeError("`parameter` must be an instance of `Parameter`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = self.type.transfer_to_ffi()
        parameter_ffi = parameter.ffi

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

        err = _ffi.fimo_module_param_set_private(
            caller.ffi, value_ffi_ptr, type_ffi, parameter_ffi
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def write_inner(self, caller: _OpaqueModule, parameter: ParameterData) -> None:
        """Writes a module parameter.

        :param caller: caller module
        :param parameter: parameter
        """
        if not isinstance(caller, _OpaqueModule):
            raise TypeError("`caller` must be an instance of `_OpaqueModule`")
        if not isinstance(parameter, ParameterData):
            raise TypeError("`parameter` must be an instance of `ParameterData`")

        # noinspection PyProtectedMember
        value_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        value_ffi_ptr = c.cast(c.pointer(value_ffi), c.c_void_p)
        type_ffi = self.type.transfer_to_ffi()
        parameter_ffi = parameter.ffi

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

        err = _ffi.fimo_module_param_set_inner(
            caller.ffi, value_ffi_ptr, type_ffi, parameter_ffi
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()


class _ParameterDeclaration:
    def __init__(
        self,
        name: str,
        type: ParameterType,
        read: ParameterAccess,
        write: ParameterAccess,
        setter: Callable[[_OpaqueModule, ParameterValue, ParameterData], None],
        getter: Callable[[_OpaqueModule, ParameterData], ParameterValue],
        default: int,
    ) -> None:
        self.name = name
        self.type = type
        self.read = read
        self.write = write
        self.setter = setter
        self.getter = getter
        self.default = default


_ResourceDeclaration = NewType("_ResourceDeclaration", str)
_NamespaceImportDeclaration = NewType("_NamespaceImportDeclaration", str)


class _SymbolImportDeclaration:
    def __init__(self, name: str, namespace: str, version: Version) -> None:
        self.name = name
        self.namespace = namespace
        self.version = version


class _SymbolStaticExportDeclaration:
    def __init__(
        self, name: str, namespace: str, version: Version, symbol: c.c_void_p
    ) -> None:
        self.name = name
        self.namespace = namespace
        self.version = version
        self.symbol = symbol


class _SymbolDynamicExportDeclaration:
    def __init__(
        self,
        name: str,
        namespace: str,
        version: Version,
        constructor: _ffi.FuncPointer,
        destructor: _ffi.FuncPointer,
    ) -> None:
        self.name = name
        self.namespace = namespace
        self.version = version
        self.symbol = symbol
        self.constructor = constructor
        self.destructor = destructor


class _DataBase:
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)
        self._dynamic_symbols: dict[int, c.c_void_p] = {}

    def _add_dynamic_symbol(self, sym: c.c_void_p) -> None:
        if not isinstance(sym, c.c_void_p):
            raise TypeError("`sym` must be an instance of `c_void_p`")
        if not sym:
            raise ValueError("`sym` may not be `null`")
        assert sym.value is not None

        if sym.value in self._dynamic_symbols:
            raise ValueError("`sym` is already contained")
        self._dynamic_symbols[sym.value] = sym


def module_parameter(
    *,
    name: str,
    type: ParameterType,
    read: Optional[ParameterAccess] = None,
    write: Optional[ParameterAccess] = None,
    setter: Optional[Callable[[_OpaqueModule, ParameterValue, ParameterData], None]],
    getter: Optional[Callable[[_OpaqueModule, ParameterData], ParameterValue]],
    default: Optional[int] = 0,
) -> _ParameterDeclaration:
    """Declares a new module parameter."""

    if read is None:
        read = ParameterAccess.Private
    if write is None:
        write = ParameterAccess.Private
    if setter is None:

        def default_setter(
            module: _OpaqueModule, value: ParameterValue, data: ParameterData
        ) -> None:
            return value.write_inner(module, data)

        setter = default_setter
    if getter is None:

        def default_getter(
            module: _OpaqueModule, data: ParameterData
        ) -> ParameterValue:
            return ParameterValue.read_inner(module, data)

        getter = default_getter

    if default is None:
        default = 0

    if not isinstance(name, str):
        raise TypeError("`name` must be an instance of `str`")
    if not isinstance(type, ParameterType):
        raise TypeError("`type` must be an instance of `ParameterType`")
    if not isinstance(read, ParameterAccess):
        raise TypeError("`read` must be an instance of `ParameterAccess`")
    if not isinstance(default, int):
        raise TypeError("`default` must be an instance of `int`")

    return _ParameterDeclaration(name, type, read, write, setter, getter, default)


def module_resource(*, path: str) -> _ResourceDeclaration:
    """Declares a new module resource."""

    if not isinstance(path, str):
        raise TypeError("`path` must be an instance of `str`")
    if path.startswith("/") or path.startswith("\\"):
        raise ValueError("`path` may not start with a slash")

    return _ResourceDeclaration(path)


def module_namespace_import(*, namespace: str) -> _NamespaceImportDeclaration:
    """Declares a new module namespace import."""

    if not isinstance(namespace, str):
        raise TypeError("`namespace` must be an instance of `str`")

    return _NamespaceImportDeclaration(namespace)


# noinspection PyProtectedMember
def module_symbol_import(*, symbol: type[Symbol[_T]]) -> _SymbolImportDeclaration:
    """Declares a new module symbol import."""

    if not isinstance(symbol, type):
        raise TypeError("`symbol` must be an instance of `type`")

    name = symbol._symbol_name()
    namespace = symbol._symbol_namespace()
    version = symbol._symbol_version()

    return _SymbolImportDeclaration(name, namespace, version)


# noinspection PyProtectedMember
def module_static_symbol_export(
    *, symbol: type[Symbol[_T]], obj: c._Pointer | _ffi.FuncPointer
) -> _SymbolStaticExportDeclaration:
    """Declares a new static module symbol export."""

    if not isinstance(symbol, type):
        raise TypeError("`symbol` must be an instance of `type`")

    name = symbol._symbol_name()
    namespace = symbol._symbol_namespace()
    version = symbol._symbol_version()
    obj_ffi = c.cast(obj, c.c_void_p)

    return _SymbolStaticExportDeclaration(name, namespace, version, obj_ffi)


# noinspection PyProtectedMember
def module_dynamic_symbol_export(
    *,
    symbol: type[Symbol[_T]],
    factory: Callable[[_ModuleBase], c._Pointer | _ffi.FuncPointer],
) -> _SymbolDynamicExportDeclaration:
    """Declares a new dynamic module symbol export."""

    if not isinstance(symbol, type):
        raise TypeError("`symbol` must be an instance of `type`")

    def construct_symbol(
        module: _ModuleBase, sym: _ffi.Pointer[c.c_void_p]
    ) -> _ffi.FimoError:
        try:
            obj = factory(module)
            obj_ptr = c.cast(obj, c.c_void_p)

            data = module.data()
            if not isinstance(data, _DataBase):
                raise TypeError("module data must be an instance of `_DataBase`")

            # noinspection PyProtectedMember
            data._add_dynamic_symbol(obj_ptr)
            sym[0] = obj_ptr

            return error.ErrorCode.EOK.transfer_to_ffi()
        except Exception as e:
            return error.ErrorCode.from_exception(e).transfer_to_ffi()

    def destroy_symbol(sym: _ffi.Pointer[c.c_void_p]) -> None:
        pass

    name = symbol._symbol_name()
    namespace = symbol._symbol_namespace()
    version = symbol._symbol_version()
    constructor_ffi = _ffi.FimoModuleDynamicSymbolConstructor(construct_symbol)
    destructor_ffi = _ffi.FimoModuleDynamicSymbolDestructor(destroy_symbol)

    return _SymbolDynamicExportDeclaration(
        name, namespace, version, constructor_ffi, destructor_ffi
    )


_DataT = TypeVar("_DataT")


def export_module(
    *,
    name: str,
    description: Optional[str] = None,
    author: Optional[str] = None,
    license: Optional[str] = None,
    parameters: Optional[dict[str, _ParameterDeclaration]] = None,
    resources: Optional[dict[str, _ResourceDeclaration]] = None,
    namespaces: Optional[list[_NamespaceImportDeclaration]] = None,
    imports: Optional[list[_SymbolImportDeclaration]] = None,
    exports: Optional[
        list[_SymbolStaticExportDeclaration | _SymbolDynamicExportDeclaration]
    ] = None,
    data_type: Optional[type[_DataT]] = None,
    factory: Optional[Callable[[_ModuleBase, Any, type[_DataT]], _DataT]] = None,
):
    """Exports a new module."""
    parameters_ = parameters if parameters is not None else {}
    resources_ = resources if resources is not None else {}
    namespaces_ = namespaces if namespaces is not None else []
    imports_ = imports if imports is not None else []
    exports_ = exports if exports is not None else []
    data_type_ = data_type if data_type is not None else object

    # Generate the class types for the module tables
    parameter_index_map = {key: i for i, key in enumerate(parameters_.keys())}

    class _ModuleParameterMap(
        _ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleParamTable]]
    ):
        def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModuleParamTable]) -> None:
            if not isinstance(ffi, c.POINTER(_ffi.FimoModuleParamTable)):
                raise TypeError(
                    "`ffi` must be an instance of an `FimoModuleParamTable*`"
                )
            if not ffi:
                raise ValueError("`ffi` may not be `null`")
            self._ffi: _ffi.Pointer[_ffi.Pointer[_ffi.FimoModuleParam]] = c.cast(
                ffi, c.POINTER(c.POINTER(_ffi.FimoModuleParam))
            )

        def __getattribute__(self, name: str) -> Parameter:
            if name in parameter_index_map:
                parameter_idx = parameter_index_map[name]
                parameter = self._ffi[parameter_idx]
                return Parameter(parameter)

            raise ValueError(f"invalid attribute name: {name}")

        def transfer_to_ffi(self) -> _ffi.Pointer[_ffi.FimoModuleParamTable]:
            return c.cast(self._ffi, c.POINTER(_ffi.FimoModuleParamTable))

        @classmethod
        def transfer_from_ffi(
            cls, ffi: _ffi.Pointer[_ffi.FimoModuleParamTable]
        ) -> Self:
            return cls(ffi)

    resource_index_map = {key: i for i, key in enumerate(resources_.keys())}

    class _ModuleResourceMap(
        _ffi.FFITransferable[_ffi.Pointer[_ffi.FimoModuleResourceTable]]
    ):
        def __init__(self, ffi: _ffi.Pointer[_ffi.FimoModuleResourceTable]) -> None:
            if not isinstance(ffi, c.POINTER(_ffi.FimoModuleResourceTable)):
                raise TypeError(
                    "`ffi` must be an instance of an `FimoModuleResourceTable*`"
                )
            if not ffi:
                raise ValueError("`ffi` may not be `null`")
            self._ffi: _ffi.Pointer[c.c_void_p] = c.cast(ffi, c.POINTER(c.c_void_p))

        def __getattribute__(self, name: str) -> str:
            if name in resource_index_map:
                resource_idx = parameter_index_map[name]
                resource: bytes = self._ffi[resource_idx]
                return resource.decode()

            raise ValueError(f"invalid attribute name: {name}")

        def transfer_to_ffi(self) -> _ffi.Pointer[_ffi.FimoModuleResourceTable]:
            return c.cast(self._ffi, c.POINTER(_ffi.FimoModuleResourceTable))

        @classmethod
        def transfer_from_ffi(
            cls, ffi: _ffi.Pointer[_ffi.FimoModuleResourceTable]
        ) -> Self:
            return cls(ffi)

    # We require some additional functionality for the data
    # of the module, so we create a wrapper which inherits
    # both from our internal mixin and the original class
    # passed by the user.
    data_bases = (_DataBase,) if data_type_ is object else (_DataBase, data_type_)
    _ModuleData = type(
        "_ModuleData",
        data_bases,
        {"__init__": lambda self, *args, **kwargs: _DataBase.__init__(*args, **kwargs)},
    )

    def module_data_transfer_to_ffi(self) -> c.c_void_p:
        obj_ffi = c.py_object(self)
        obj_ptr = c.c_void_p.from_buffer(obj_ffi)
        return obj_ptr

    def module_data_transfer_from_ffi(cls, ffi: c.c_void_p) -> Any:
        obj = c.cast(ffi, c.py_object).value
        if not isinstance(obj, _ModuleData):
            raise TypeError("invalid module data type")
        return obj

    _ModuleData.transfer_to_ffi = module_data_transfer_to_ffi  # type: ignore[attr-defined]
    _ModuleData.transfer_from_ffi = classmethod(module_data_transfer_from_ffi)  # type: ignore[attr-defined]

    class _Module(_ModuleBase):
        @staticmethod
        def _parameters_type() -> type[_OpaqueParameters]:
            return _OpaqueParameters

        @staticmethod
        def _resources_type() -> type[_OpaqueResources]:
            return _OpaqueResources

        @staticmethod
        def _imports_type() -> type[_OpaqueImports]:
            return _OpaqueImports

        @staticmethod
        def _exports_type() -> type[_OpaqueExports]:
            return _OpaqueExports

        @staticmethod
        def _data_type() -> type[Any]:
            return _ModuleData

    def default_factory(
        module: _ModuleBase, module_set: Any, cls: type[_DataT]
    ) -> _DataT:
        return cls()

    factory_ = factory if factory is not None else default_factory

    name_ffi = c.c_char_p(name.encode())
    description_ffi = (
        c.c_char_p(description.encode()) if description is not None else c.c_char_p()
    )
    author_ffi = c.c_char_p(author.encode()) if author is not None else c.c_char_p()
    license_ffi = c.c_char_p(license.encode()) if license is not None else c.c_char_p()

    export = _ffi.FimoModuleExport()
    export.type = _ffi.FimoStructType.FIMO_STRUCT_TYPE_MODULE_EXPORT
    export.next = c.POINTER(_ffi.FimoBaseStructIn)()
    export.export_abi = _ffi.FimoI32(0)
    export.name = name_ffi
    export.description = description_ffi
    export.author = author_ffi
    export.license = license_ffi

    parameters_ffi = (_ffi.FimoModuleParamDecl * len(parameters_))()
    parameters_count_ffi = _ffi.FimoU32(len(parameters_))
    for i, param in enumerate(parameters_.values()):
        setter: Callable[[_OpaqueModule, ParameterValue, ParameterData], None] = (
            param.setter
        )
        getter: Callable[[_OpaqueModule, ParameterData], ParameterValue] = param.getter

        def setter_wrapper(
            module_ffi: _ffi.Pointer[_ffi.FimoModule],
            value_addr: int,
            type_ffi: _ffi.FimoModuleParamType,
            data_ffi: _ffi.Pointer[_ffi.FimoModuleParamData],
        ) -> _ffi.FimoError:
            try:
                module = _OpaqueModule.borrow_from_ffi(module_ffi)
                value_ffi = c.c_void_p.from_address(value_addr)
                type = ParameterType.transfer_from_ffi(type_ffi)
                match type:
                    case ParameterType.U8:
                        value = c.cast(value_ffi, c.POINTER(_ffi.FimoU8)).contents.value
                    case ParameterType.U16:
                        value = c.cast(
                            value_ffi, c.POINTER(_ffi.FimoU16)
                        ).contents.value
                    case ParameterType.U32:
                        value = c.cast(
                            value_ffi, c.POINTER(_ffi.FimoU32)
                        ).contents.value
                    case ParameterType.U64:
                        value = c.cast(
                            value_ffi, c.POINTER(_ffi.FimoU64)
                        ).contents.value
                    case ParameterType.I8:
                        value = c.cast(value_ffi, c.POINTER(_ffi.FimoI8)).contents.value
                    case ParameterType.I16:
                        value = c.cast(
                            value_ffi, c.POINTER(_ffi.FimoI16)
                        ).contents.value
                    case ParameterType.I32:
                        value = c.cast(
                            value_ffi, c.POINTER(_ffi.FimoI32)
                        ).contents.value
                    case ParameterType.I64:
                        value = c.cast(
                            value_ffi, c.POINTER(_ffi.FimoI64)
                        ).contents.value
                    case _:
                        raise ValueError("unknown parameter type")

                parameter = ParameterValue(value, type)
                data = ParameterData.borrow_from_ffi(data_ffi)
                setter(module, parameter, data)

                return error.ErrorCode.EOK.transfer_to_ffi()
            except Exception as e:
                return error.ErrorCode.from_exception(e).transfer_to_ffi()

        def getter_wrapper(
            module_ffi: _ffi.Pointer[_ffi.FimoModule],
            value_addr: int,
            type_ffi: _ffi.Pointer[_ffi.FimoModuleParamType],
            data_ffi: _ffi.Pointer[_ffi.FimoModuleParamData],
        ) -> _ffi.FimoError:
            try:
                module = _OpaqueModule.borrow_from_ffi(module_ffi)
                value_ffi = c.c_void_p.from_address(value_addr)
                data = ParameterData.borrow_from_ffi(data_ffi)

                parameter = getter(module, data)
                type = parameter.type
                value = parameter.value

                type_ffi[0] = type.transfer_to_ffi()
                match type:
                    case ParameterType.U8:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoU8))[0] = _ffi.FimoU8(
                            value
                        )
                    case ParameterType.U16:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoU16))[0] = _ffi.FimoU16(
                            value
                        )
                    case ParameterType.U32:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoU32))[0] = _ffi.FimoU32(
                            value
                        )
                    case ParameterType.U64:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoU64))[0] = _ffi.FimoU64(
                            value
                        )
                    case ParameterType.I8:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoI8))[0] = _ffi.FimoI8(
                            value
                        )
                    case ParameterType.I16:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoI16))[0] = _ffi.FimoI16(
                            value
                        )
                    case ParameterType.I32:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoI32))[0] = _ffi.FimoI32(
                            value
                        )
                    case ParameterType.I64:
                        c.cast(value_ffi, c.POINTER(_ffi.FimoI64))[0] = _ffi.FimoI64(
                            value
                        )
                    case _:
                        raise ValueError("unknown parameter type")

                return error.ErrorCode.EOK.transfer_to_ffi()
            except Exception as e:
                return error.ErrorCode.from_exception(e).transfer_to_ffi()

        # noinspection PyProtectedMember
        default_ffi = _ffi._FimoModuleParamDeclDefaultValue()
        match param.type:
            case ParameterType.U8:
                default_ffi.u8 = _ffi.FimoU8(param.default)
            case ParameterType.U16:
                default_ffi.u16 = _ffi.FimoU16(param.default)
            case ParameterType.U32:
                default_ffi.u32 = _ffi.FimoU32(param.default)
            case ParameterType.U64:
                default_ffi.u64 = _ffi.FimoU64(param.default)
            case ParameterType.I8:
                default_ffi.i8 = _ffi.FimoI8(param.default)
            case ParameterType.I16:
                default_ffi.i16 = _ffi.FimoI16(param.default)
            case ParameterType.I32:
                default_ffi.i32 = _ffi.FimoI32(param.default)
            case ParameterType.I64:
                default_ffi.i64 = _ffi.FimoI64(param.default)

        param_ffi = _ffi.FimoModuleParamDecl()
        param_ffi.type = param.type.transfer_to_ffi()
        param_ffi.read_access = param.read.transfer_to_ffi()
        param_ffi.write_access = param.write.transfer_to_ffi()
        param_ffi.setter = _ffi.FimoModuleParamSet(setter_wrapper)
        param_ffi.getter = _ffi.FimoModuleParamGet(getter_wrapper)
        param_ffi.name = c.c_char_p(param.name.encode())
        param_ffi.default_value = default_ffi
        parameters_ffi[i] = param_ffi
    export.parameters = parameters_ffi
    export.parameters_count = parameters_count_ffi

    resources_ffi = (_ffi.FimoModuleResourceDecl * len(resources_))()
    resources_count_ffi = _ffi.FimoU32(len(resources_))
    for i, res in enumerate(resources_.values()):
        res_ffi = _ffi.FimoModuleResourceDecl()
        res_ffi.path = c.c_char_p(res.encode())
        resources_ffi[i] = res_ffi
    export.resources = resources_ffi
    export.resources_count = resources_count_ffi

    namespaces_ffi = (_ffi.FimoModuleNamespaceImport * len(namespaces_))()
    namespaces_count_ffi = _ffi.FimoU32(len(namespaces_))
    for i, ns in enumerate(namespaces_):
        ns_ffi = _ffi.FimoModuleNamespaceImport()
        ns_ffi.name = c.c_char_p(ns.encode())
        namespaces_ffi[i] = ns_ffi
    export.namespace_imports = namespaces_ffi
    export.namespace_imports_count = namespaces_count_ffi

    imports_ffi = (_ffi.FimoModuleSymbolImport * len(imports_))()
    imports_count_ffi = _ffi.FimoU32(len(imports_))
    for i, imp in enumerate(imports_):
        import_ffi = _ffi.FimoModuleSymbolImport()
        import_ffi.version = imp.version.transfer_to_ffi()
        import_ffi.name = c.c_char_p(imp.name.encode())
        import_ffi.ns = c.c_char_p(imp.namespace.encode())
        imports_ffi[i] = import_ffi
    export.symbol_imports = imports_ffi
    export.symbol_imports_count = imports_count_ffi

    export_names: set[str] = set()
    static_exports: list[_SymbolStaticExportDeclaration] = []
    dynamic_exports: list[_SymbolDynamicExportDeclaration] = []
    for exp in exports_:
        if exp.name in export_names:
            raise ValueError(f"duplicate export defined: {exp.name}")
        export_names.add(exp.name)

        if isinstance(exp, _SymbolStaticExportDeclaration):
            static_exports.append(exp)
        elif isinstance(exp, _SymbolDynamicExportDeclaration):
            dynamic_exports.append(exp)
        else:
            raise TypeError("unknown export type")

    static_exports_ffi = (_ffi.FimoModuleSymbolExport * len(static_exports))()
    static_exports_count_ffi = _ffi.FimoU32(len(static_exports))
    for i, exp in enumerate(static_exports):
        s_export_ffi = _ffi.FimoModuleSymbolExport()
        s_export_ffi.symbol = exp.symbol
        s_export_ffi.version = exp.version.transfer_to_ffi()
        s_export_ffi.name = c.c_char_p(exp.name.encode())
        s_export_ffi.ns = c.c_char_p(exp.namespace.encode())
        static_exports_ffi[i] = s_export_ffi
    export.symbol_exports = static_exports_ffi
    export.symbol_exports_count = static_exports_count_ffi

    dynamic_exports_ffi = (_ffi.FimoModuleDynamicSymbolExport * len(dynamic_exports))()
    dynamic_exports_count_ffi = _ffi.FimoU32(len(dynamic_exports))
    for i, exp in enumerate(dynamic_exports):
        d_export_ffi = _ffi.FimoModuleDynamicSymbolExport()
        d_export_ffi.constructor = exp.constructor
        d_export_ffi.destructor = exp.destructor
        d_export_ffi.version = exp.version.transfer_to_ffi()
        d_export_ffi.name = c.c_char_p(exp.name.encode())
        d_export_ffi.ns = c.c_char_p(exp.namespace.encode())
        dynamic_exports_ffi[i] = d_export_ffi
    export.dynamic_symbol_exports = dynamic_exports_ffi
    export.dynamic_symbol_exports_count = dynamic_exports_count_ffi

    export.modifiers = c.POINTER(_ffi.FimoModuleExportModifier)()
    export.modifiers_count = _ffi.FimoU32(0)

    def constructor(
        module_ffi: _ffi.Pointer[_ffi.FimoModule],
        module_set_ffi: _ffi.Pointer[_ffi.FimoModuleLoadingSet],
        data_ffi: _ffi.Pointer[c.c_void_p],
    ) -> _ffi.FimoError:
        try:
            module = _Module.borrow_from_ffi(module_ffi)

            # noinspection PyTypeChecker
            obj = factory_(module, module_set_ffi, _ModuleData)
            obj_ptr = obj.transfer_to_ffi()  # type: ignore[attr-defined]

            data_ffi[0] = obj_ptr
            _ffi.c_inc_ref(obj)

            return error.ErrorCode.EOK.transfer_to_ffi()
        except Exception as e:
            return error.ErrorCode.from_exception(e).transfer_to_ffi()

    def destructor(
        module_ffi: _ffi.Pointer[_ffi.FimoModule], data_ffi: _ffi.Pointer[c.c_void_p]
    ) -> None:
        try:
            obj = _ModuleData.transfer_from_ffi(data_ffi.contents)  # type: ignore[attr-defined]
            _ffi.c_dec_ref(obj)
        except Exception:
            assert False

    export.module_constructor = _ffi.FimoModuleConstructor(constructor)
    export.module_destructor = _ffi.FimoModuleDestructor(destructor)
