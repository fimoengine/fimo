import ctypes as c
from abc import ABC, abstractmethod
from typing import Self, Optional

from . import error
from . import ffi as _ffi


class ContextOption(ABC):
    """A type that can be passed to the context at creation time."""

    @abstractmethod
    def to_context_option(self) -> c.POINTER(_ffi.FimoBaseStructIn):
        """Constructs a pointer to the option."""
        pass


class ContextView(_ffi.FFISharable[_ffi.FimoContext, Self]):
    """View of the context of the fimo library."""
    from . import tracing as _tracing
    _create_key = object()

    def __init__(self, create_key: object, context: _ffi.FimoContext):
        if create_key is not ContextView._create_key:
            error.ErrorCode.EINVAL.raise_if_error()
        self._context = context

    @property
    def ffi(self) -> _ffi.FimoContext:
        return self._context

    @classmethod
    def borrow_from_ffi(cls, ffi: _ffi.FimoContext) -> Self:
        return ContextView(ContextView._create_key, ffi)

    @property
    def _as_parameter_(self) -> _ffi.FimoContext:
        return self._context

    def check_version(self) -> None:
        """Checks the compatibility of the context version.

        This function must be called upon the acquisition of a context, that
        was not created locally, e.g., when being passed a context from
        another shared library. Failure of doing so, may cause undefined
        behavior, if the context is later utilized.
        """
        err = _ffi.fimo_context_check_version(self._context)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def acquire(self) -> "Context":
        """Acquires a reference to the context.

        Increases the reference count of the context. May abort the program,
        if doing so is not possible. May only be called with a valid reference
        to the context.

        :return: New `Context`.
        """
        _ffi.fimo_context_acquire(self._context)
        return Context.transfer_from_ffi(self._context)

    def tracing(self) -> _tracing.TracingCtx:
        """Returns a reference to the tracing subsystem."""
        return self._tracing.TracingCtx(self)


class Context(ContextView, _ffi.FFITransferable[_ffi.FimoContext]):

    def __init__(self, create_key: object, context: _ffi.FimoContext):
        if create_key is not ContextView._create_key:
            error.ErrorCode.EINVAL.raise_if_error()
        super().__init__(create_key, context)

    def __del__(self):
        if self._context is not None:
            _ffi.fimo_context_release(self._context)
            self._context = None

    def transfer_to_ffi(self) -> _ffi.FimoContext:
        ctx = self.ffi
        self._context = None
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
            error.ErrorCode.transfer_from_ffi(err).raise_if_error()
            return Context.transfer_from_ffi(ctx)

        if not isinstance(options, list):
            error.ErrorCode.EINVAL.raise_if_error()

        options_array = (c.POINTER(_ffi.FimoBaseStructIn) * (len(options) + 1))()
        for i, opt in enumerate(options):
            if not isinstance(opt, ContextOption):
                error.ErrorCode.EINVAL.raise_if_error()
            ffi_opt = opt.to_context_option()
            if not isinstance(ffi_opt, c.POINTER(_ffi.FimoBaseStructIn)):
                error.ErrorCode.EINVAL.raise_if_error()
            options_array[i] = ffi_opt
        options_array[len(options)] = c.POINTER(_ffi.FimoBaseStructIn)()

        ctx = _ffi.FimoContext()
        err = _ffi.fimo_context_init(options_array, c.byref(ctx))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return Context.transfer_from_ffi(ctx)