from __future__ import annotations
import ctypes as c
from abc import ABC, abstractmethod
from typing import Self, Optional

from . import error
from . import ffi as _ffi


class ContextOption(ABC):
    """A type that can be passed to the context at creation time."""

    @abstractmethod
    def to_context_option(self) -> _ffi.Ref[_ffi.FimoBaseStructIn]:
        """Constructs a pointer to the option."""
        pass


class ContextView(_ffi.FFISharable[_ffi.FimoContext, "ContextView"]):
    """View of the context of the fimo library."""

    from . import tracing as _tracing
    from . import module as _module

    _create_key = object()

    def __init__(self, create_key: object, context: _ffi.FimoContext):
        if create_key is not ContextView._create_key:
            raise ValueError("`create_key` must be an instance of `_create_key`")
        if not isinstance(context, _ffi.FimoContext):
            raise TypeError("`context` must be an instance of `FimoContext`")

        self._context: _ffi.FimoContext | None = context

    @property
    def ffi(self) -> _ffi.FimoContext:
        if self._context is None:
            raise ValueError("context has been consumed")

        return self._context

    @classmethod
    def borrow_from_ffi(cls, ffi: _ffi.FimoContext) -> ContextView:
        return ContextView(ContextView._create_key, ffi)

    @property
    def _as_parameter_(self) -> _ffi.FimoContext:
        return self.ffi

    def check_version(self) -> None:
        """Checks the compatibility of the context version.

        This function must be called upon the acquisition of a context, that
        was not created locally, e.g., when being passed a context from
        another shared library. Failure of doing so, may cause undefined
        behavior, if the context is later utilized.
        """
        if self._context is None:
            raise ValueError("context has been consumed")

        err = _ffi.fimo_context_check_version(self._context)
        error.Result.transfer_from_ffi(err).raise_if_error()

    def acquire(self) -> Context:
        """Acquires a reference to the context.

        Increases the reference count of the context. May abort the program,
        if doing so is not possible. May only be called with a valid reference
        to the context.

        :return: New `Context`.
        """
        if self._context is None:
            raise ValueError("context has been consumed")

        _ffi.fimo_context_acquire(self._context)
        return Context.transfer_from_ffi(self._context)

    def tracing(self) -> _tracing.TracingCtx:
        """Returns a reference to the tracing subsystem."""
        if self._context is None:
            raise ValueError("context has been consumed")

        return self._tracing.TracingCtx(self)

    def module(self) -> _module.ModuleCtx:
        """Returns a reference to the module subsystem."""
        if self._context is None:
            raise ValueError("context has been consumed")

        return self._module.ModuleCtx(self)

    def _consume(self) -> None:
        if self._context is None:
            raise ValueError("context has been consumed")

        self._context = None


class Context(ContextView, _ffi.FFITransferable[_ffi.FimoContext]):

    def __init__(self, create_key: object, context: _ffi.FimoContext):
        if create_key is not ContextView._create_key:
            error.Result.from_error_code(error.ErrorCode.EINVAL).raise_if_error()
        super().__init__(create_key, context)

    def __del__(self):
        if self._context is not None:
            _ffi.fimo_context_release(self._context)
            self._consume()

    def transfer_to_ffi(self) -> _ffi.FimoContext:
        ctx = self.ffi
        self._consume()
        return ctx

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoContext) -> Self:
        return cls(ContextView._create_key, ffi)

    @staticmethod
    def new_context(options: Optional[list] = None):
        """Initializes a new context with the given options."""
        options_type = c.POINTER(c.POINTER(_ffi.FimoBaseStructIn))

        if options is None:
            ctx = _ffi.FimoContext()
            err = _ffi.fimo_context_init(options_type(), c.byref(ctx))
            error.Result.transfer_from_ffi(err).raise_if_error()
            return Context.transfer_from_ffi(ctx)

        if not isinstance(options, list):
            error.Result.from_error_code(error.ErrorCode.EINVAL).raise_if_error()

        options_array = (c.POINTER(_ffi.FimoBaseStructIn) * (len(options) + 1))()
        for i, opt in enumerate(options):
            if not isinstance(opt, ContextOption):
                error.Result.from_error_code(error.ErrorCode.EINVAL).raise_if_error()
            ffi_opt = opt.to_context_option()
            if not isinstance(ffi_opt, c.POINTER(_ffi.FimoBaseStructIn)):
                error.Result.from_error_code(error.ErrorCode.EINVAL).raise_if_error()
            options_array[i] = ffi_opt
        options_array[len(options)] = c.POINTER(_ffi.FimoBaseStructIn)()

        ctx = _ffi.FimoContext()
        err = _ffi.fimo_context_init(options_array, c.byref(ctx))
        error.Result.transfer_from_ffi(err).raise_if_error()
        return Context.transfer_from_ffi(ctx)
