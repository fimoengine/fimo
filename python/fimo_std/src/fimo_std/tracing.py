from __future__ import annotations

import ctypes
import inspect
import ctypes as c
from enum import IntEnum
from abc import abstractmethod
from typing import Self, Optional, Generic, TypeVar, TYPE_CHECKING

from .time import Time
from .enum import ABCEnum
from . import error
from . import context
from . import ffi as _ffi

if TYPE_CHECKING:
    from .context import Context as _Context, ContextView as _ContextView


class Level(_ffi.FFITransferable[_ffi.FimoTracingLevel], IntEnum, metaclass=ABCEnum):
    """Available levels in the tracing subsystem."""

    Off = 0
    Error = 1
    Warn = 2
    Info = 3
    Debug = 4
    Trace = 5

    def transfer_to_ffi(self) -> _ffi.FimoTracingLevel:
        return _ffi.FimoTracingLevel(self)

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTracingLevel) -> Self:
        return cls(ffi.value)

    @classmethod
    def from_param(cls, obj):
        return cls(obj)


class Metadata(_ffi.FFITransferable[_ffi.FimoTracingMetadata]):
    """Metadata for a span/event."""

    def __init__(
        self,
        name: str,
        target: str,
        level: Level,
        file_name: Optional[str],
        line_number: Optional[int],
    ) -> None:
        if not isinstance(name, str):
            raise TypeError("`name` must be a `str`")
        if not isinstance(target, str):
            raise TypeError("`target` must be a `str`")
        if not isinstance(level, Level):
            raise TypeError("`level` must be an instance of `Level`")
        if not isinstance(file_name, str) and file_name is not None:
            raise TypeError("`file_name` must be a `str` or `None`")
        if not isinstance(line_number, int) and line_number is not None:
            raise TypeError("`line_number` must be an `int` or `None`")

        if line_number is not None and not (
            line_number >= 0 and line_number.bit_length() < 32
        ):
            raise ValueError("`line_number` must fit in a 32-bit integer")

        name_ffi: bytes = name.encode()
        target_ffi: bytes = target.encode()
        level_ffi: _ffi.FimoTracingLevel = level.transfer_to_ffi()
        file_name_ffi: Optional[bytes] = (
            file_name.encode() if file_name is not None else None
        )
        if line_number is None:
            line_number = -1

        self._metadata = _ffi.FimoTracingMetadata(
            _ffi.FimoStructType.FIMO_STRUCT_TYPE_TRACING_METADATA,
            c.POINTER(_ffi.FimoBaseStructIn)(),
            name_ffi,
            target_ffi,
            level_ffi,
            file_name_ffi,
            line_number,
        )

    def transfer_to_ffi(self) -> _ffi.FimoTracingMetadata:
        return self._metadata

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTracingMetadata) -> Self:
        name = ffi.name.decode()
        target = ffi.target.decode()
        level = Level.transfer_from_ffi(ffi.level)
        file_name = ffi.file_name.decode()
        line_number = ffi.line_number.value
        if line_number < 0:
            line_number = None
        return cls(name, target, level, file_name, line_number)

    @property
    def _as_parameter_(self) -> _ffi.FimoTracingMetadata:
        return self._metadata

    @property
    def name(self) -> str:
        """Fetches the `name` field of the `Metadata`."""
        val: bytes = self._metadata.name
        return val.decode()

    @property
    def target(self) -> str:
        """Fetches the `target` field of the `Metadata`."""
        val: bytes = self._metadata.target
        return val.decode()

    @property
    def level(self) -> Level:
        """Fetches the `level` field of the `Metadata`."""
        val: _ffi.FimoTracingLevel = self._metadata.level
        return Level.transfer_from_ffi(val)

    @property
    def file_name(self) -> Optional[str]:
        """Fetches the `file_name` field of the `Metadata`."""
        val: Optional[bytes] = self._metadata.file_name
        if val is None:
            return None
        return val.decode()

    @property
    def line_number(self) -> Optional[int]:
        """Fetches the `line_number` field of the `Metadata`."""
        val: int = self._metadata.line_number.value
        if val < 0:
            return None
        else:
            return val


class SpanDescriptor(_ffi.FFITransferable[_ffi.FimoTracingSpanDesc]):
    """Descriptor of a new span."""

    def __init__(self, metadata: Metadata) -> None:
        if not isinstance(metadata, Metadata):
            raise TypeError("`metadata` must be an instance of `Metadata`")

        self._metadata = metadata
        self._desc = _ffi.FimoTracingSpanDesc(
            _ffi.FimoStructType.FIMO_STRUCT_TYPE_TRACING_SPAN_DESC,
            c.POINTER(_ffi.FimoBaseStructIn)(),
            c.pointer(metadata.transfer_to_ffi()),
        )

    def transfer_to_ffi(self) -> _ffi.FimoTracingSpanDesc:
        return self._desc

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTracingSpanDesc) -> Self:
        metadata = Metadata.transfer_from_ffi(ffi.metadata.contents)
        return cls(metadata)

    @property
    def _as_parameter_(self) -> _ffi.FimoTracingSpanDesc:
        return self._desc

    @property
    def metadata(self) -> Metadata:
        """Fetches the metadata of the descriptor."""
        return self._metadata


class Event(_ffi.FFITransferable[_ffi.FimoTracingEvent]):
    """An event to be traced."""

    def __init__(self, metadata: Metadata) -> None:
        if not isinstance(metadata, Metadata):
            raise TypeError("`metadata` must be an instance of `Metadata`")

        self._metadata = metadata
        self._event = _ffi.FimoTracingEvent(
            _ffi.FimoStructType.FIMO_STRUCT_TYPE_TRACING_EVENT,
            c.POINTER(_ffi.FimoBaseStructIn)(),
            c.pointer(metadata.transfer_to_ffi()),
        )

    def transfer_to_ffi(self) -> _ffi.FimoTracingEvent:
        return self._event

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTracingEvent) -> Self:
        metadata = Metadata.transfer_from_ffi(ffi.metadata.contents)
        return cls(metadata)

    @property
    def _as_parameter_(self) -> _ffi.FimoTracingEvent:
        return self._event

    @property
    def metadata(self) -> Metadata:
        """Fetches the metadata of the event."""
        return self._metadata


class CallStack:
    """RAII wrapper of a tracing call stack."""

    def __init__(self, ctx: _ContextView) -> None:
        """Creates a new empty call stack.

        If successful, the new call stack is marked as suspended. The new call stack is not set to
        be the active call stack.
        """
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")

        stack = c.POINTER(_ffi.FimoTracingCallStack)()
        err = _ffi.fimo_tracing_call_stack_create(ctx.ffi, c.byref(stack))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        self._ctx: Optional[_Context] = ctx.acquire()
        self._stack: Optional[_ffi.Ref[_ffi.FimoTracingCallStack]] = stack

    def __del__(self) -> None:
        if self._ctx is not None:
            err = _ffi.fimo_tracing_call_stack_destroy(self._ctx.ffi, self._stack)
            error.ErrorCode.transfer_from_ffi(err).raise_if_error()
            self._stack = None
            self._ctx = None

    def switch(self) -> None:
        """Switches the call stack of the current thread.

        If successful, the call stack will be used as the active call stack of the calling thread.
        The old call stack will be written into `self`, such that another call to `self.switch()`
        restores the original call stack. The call stack must be in a suspended, but unblocked,
        state. The active call stack must also be in a suspended state, but may also be blocked.
        """
        if self._ctx is None:
            raise ValueError("The call stack has already been destroyed")

        stack = c.POINTER(_ffi.FimoTracingCallStack)()
        err = _ffi.fimo_tracing_call_stack_switch(
            self._ctx.ffi, self._stack, c.byref(stack)
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        self._stack = stack

    def unblock(self) -> None:
        """Unblocks the blocked call stack.

        Once unblocked, the call stack may be resumed. The call stack may not be active and must
        be marked as blocked.
        """
        if self._ctx is None:
            raise ValueError("The call stack has already been destroyed")

        err = _ffi.fimo_tracing_call_stack_unblock(self._ctx.ffi, self._stack)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    @staticmethod
    def suspend_current(ctx: _ContextView, block: bool) -> None:
        """Marks the current call stack as being suspended.

        While suspended, the call stack can not be utilized for tracing messages. The call stack
        may optionally also be marked as being blocked. In that case, the call stack must be
        unblocked prior to resumption.
        """
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")
        if not isinstance(block, bool):
            raise TypeError("`block` must be an instance of `bool`")

        err = _ffi.fimo_tracing_call_stack_suspend_current(ctx.ffi, c.c_bool(block))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    @staticmethod
    def resume_current(ctx: _ContextView) -> None:
        """Marks the current call stack as being resumed.

        Once resumed, the context can be used to trace messages. To be successful, the current
        call stack must be suspended and unblocked.
        """
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")

        err = _ffi.fimo_tracing_call_stack_resume_current(ctx.ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()


class ThreadAccess:
    """RAII access provider to the tracing subsystem for a thread."""

    def __init__(self, ctx: _ContextView) -> None:
        """Registers the calling thread with the tracing subsystem.

        The tracing of the subsystem is opt-in on a per-thread basis, where unregistered threads
        will behave as if the backend was disabled. Once registered, the calling thread gains access
        to the tracing subsystem and is assigned a new empty call stack.
        """
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")

        self._ctx: Optional[_Context] = ctx.acquire()
        err = _ffi.fimo_tracing_register_thread(self._ctx.ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def __del__(self) -> None:
        if self._ctx is not None:
            self.unregister()

    @property
    def context(self) -> _ContextView:
        """Returns the `ContextView`."""
        if self._ctx is None:
            raise ValueError("the object has already been consumed")

        return self._ctx

    def unregister(self) -> None:
        """Unregisters the calling thread from the tracing subsystem.

        Once unregistered, the calling thread looses access to the tracing subsystem until it is
        registered again. The thread can not be unregistered until the call stack is empty.
        """
        err = _ffi.fimo_tracing_unregister_thread(self.context.ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        self._ctx = None

    def __enter__(self) -> Self:
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self._ctx is not None:
            self.unregister()


class _FormatArgs:
    def __init__(self, msg: str, *args, **kwargs) -> None:
        self.msg = msg
        self.args = args
        self.kwargs = kwargs


def _format_msg(
    buffer: c._Pointer[c.c_char],
    buffer_len: _ffi.FimoUSize,
    args: int,
    written: c._Pointer[_ffi.FimoUSize],
) -> error.ErrorCode:
    try:
        args = c.cast(args, c.POINTER(c.py_object)).contents.value
        if not isinstance(args, _FormatArgs):
            raise TypeError("`args` must be an instance of `_FormatArgs`")

        msg = args.msg.format(*args.args, **args.kwargs).encode()

        msg_len = min(buffer_len.value, len(msg))
        msg_slice = msg[:msg_len]
        c.memmove(buffer, msg_slice, msg_len * c.sizeof(c.c_char))
        written[0] = msg_len
        return error.ErrorCode.EOK
    except Exception as e:
        return error.ErrorCode.from_exception(e)


class Span:
    """RAII wrapper of a span in the tracing subsystem."""

    def __init__(
        self, ctx: _ContextView, descriptor: SpanDescriptor, msg: str, *args, **kwargs
    ) -> None:
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")
        if not isinstance(descriptor, SpanDescriptor):
            raise TypeError("`descriptor` must be an instance of `SpanDescriptor`")
        if not isinstance(msg, str):
            raise TypeError("`msg` must be a `str`")

        ctx_ffi = ctx.ffi
        desc_ffi = c.pointer(descriptor.transfer_to_ffi())
        span_ffi = c.POINTER(_ffi.FimoTracingSpan)()
        format_ffi = _ffi.FimoTracingFormat(_format_msg)
        data_ffi = c.cast(
            c.pointer(c.py_object(_FormatArgs(msg, *args, **kwargs))), c.c_void_p
        )

        err = _ffi.fimo_tracing_span_create_custom(
            ctx_ffi, desc_ffi, c.byref(span_ffi), format_ffi, data_ffi
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

        self._ctx: Optional[_Context] = ctx.acquire()
        self._span: Optional[c._Pointer[_ffi.FimoTracingSpan]] = span_ffi

    def __del__(self) -> None:
        if self._ctx is not None:
            self._destroy()

    def _destroy(self) -> None:
        err = _ffi.fimo_tracing_span_destroy(self.context.ffi, self._span)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        self._span = None
        self._ctx = None

    @property
    def context(self) -> _ContextView:
        """Returns the `ContextView`."""
        if self._ctx is None:
            raise ValueError("the `Span` has been consumed")
        return self._ctx

    def __enter__(self) -> Self:
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self._ctx is not None:
            self._destroy()

    @classmethod
    def _new_span(
        cls, ctx: _ContextView, level: Level, msg: str, *args, **kwargs
    ) -> Self:
        curr_frame = inspect.currentframe()
        caller_frame = inspect.getouterframes(curr_frame, 3)[2]

        module_name = caller_frame.frame.f_globals["__name__"]
        func_name = caller_frame.function
        file_name = caller_frame.filename
        line_number = caller_frame.lineno

        metadata = Metadata(module_name, func_name, level, file_name, line_number)
        descriptor = SpanDescriptor(metadata)
        return cls(ctx, descriptor, msg, *args, **kwargs)

    @classmethod
    def error_span(cls, ctx: _ContextView, msg: str, *args, **kwargs) -> Self:
        """Creates a new error span."""
        return cls._new_span(ctx, Level.Error, msg, *args, **kwargs)

    @classmethod
    def warn_span(cls, ctx: _ContextView, msg: str, *args, **kwargs) -> Self:
        """Creates a new warn span."""
        return cls._new_span(ctx, Level.Warn, msg, *args, **kwargs)

    @classmethod
    def info_span(cls, ctx: _ContextView, msg: str, *args, **kwargs) -> Self:
        """Creates a new info span."""
        return cls._new_span(ctx, Level.Info, msg, *args, **kwargs)

    @classmethod
    def debug_span(cls, ctx: _ContextView, msg: str, *args, **kwargs) -> Self:
        """Creates a new debug span."""
        return cls._new_span(ctx, Level.Debug, msg, *args, **kwargs)

    @classmethod
    def trace_span(cls, ctx: _ContextView, msg: str, *args, **kwargs) -> Self:
        """Creates a new trace span."""
        return cls._new_span(ctx, Level.Trace, msg, *args, **kwargs)


class _SubscriberWrapper:
    def __init__(
        self, vtable: _ffi.FimoTracingSubscriberVTable, obj: Subscriber
    ) -> None:
        self.vtable = vtable
        self.obj = obj


def _subscriber_destroy(ptr: int) -> None:
    try:
        obj = c.cast(ptr, c.py_object)
        _ffi.c_dec_ref(obj)
        del obj
    except Exception:
        pass


def _subscriber_call_stack_create(
    ptr: int,
    time_ffi: c._Pointer[_ffi.FimoTime],
    call_stack_ffi: c._Pointer[c.c_void_p],
) -> error.ErrorCode:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        call_stack = obj.obj.create_call_stack(time)

        call_stack_ffi[0] = call_stack.transfer_to_ffi()
        return error.ErrorCode.EOK
    except Exception as e:
        return error.ErrorCode.from_exception(e)


def _subscriber_call_stack_drop(ptr: int, call_stack_ffi: int) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.drop_call_stack(call_stack)
        del call_stack
    except Exception:
        pass


def _subscriber_call_stack_destroy(
    ptr: int, time_ffi: c._Pointer[_ffi.FimoTime], call_stack_ffi: int
) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.destroy_call_stack(time, call_stack)
        del call_stack
    except Exception:
        pass


def _subscriber_call_stack_unblock(
    ptr: int, time_ffi: c._Pointer[_ffi.FimoTime], call_stack_ffi: int
) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.unblock_call_stack(time, call_stack)
    except Exception:
        pass


def _subscriber_call_stack_suspend(
    ptr: int, time_ffi: c._Pointer[_ffi.FimoTime], call_stack_ffi: int, block: bool
) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.suspend_call_stack(time, call_stack, block)
    except Exception:
        pass


def _subscriber_call_stack_resume(
    ptr: int, time_ffi: c._Pointer[_ffi.FimoTime], call_stack_ffi: int
) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.resume_call_stack(time, call_stack)
    except Exception:
        pass


def _subscriber_span_push(
    ptr: int,
    time_ffi: c._Pointer[_ffi.FimoTime],
    span_descriptor_ffi: c._Pointer[_ffi.FimoTracingSpanDesc],
    message_ffi: c._Pointer[c.c_char],
    message_len: _ffi.FimoUSize,
    call_stack_ffi: int,
) -> error.ErrorCode:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        span_descriptor = SpanDescriptor.transfer_from_ffi(span_descriptor_ffi[0])
        message = message_ffi[: message_len.value]
        assert isinstance(message, bytes)
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.create_span(time, span_descriptor, message, call_stack)
        return error.ErrorCode.EOK
    except Exception as e:
        return error.ErrorCode.from_exception(e)


def _subscriber_span_drop(ptr: int, call_stack_ffi: int) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.drop_span(call_stack)
    except Exception:
        pass


def _subscriber_span_pop(
    ptr: int, time_ffi: c._Pointer[_ffi.FimoTime], call_stack_ffi: int
) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.destroy_span(time, call_stack)
    except Exception:
        pass


def _subscriber_event_emit(
    ptr: int,
    time_ffi: c._Pointer[_ffi.FimoTime],
    call_stack_ffi: int,
    event_ffi: c._Pointer[_ffi.FimoTracingEvent],
    message_ffi: c._Pointer[c.c_char],
    message_len: _ffi.FimoUSize,
) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        time = Time.transfer_from_ffi(time_ffi[0])
        event = Event.transfer_from_ffi(event_ffi[0])
        message = message_ffi[: message_len.value]
        assert isinstance(message, bytes)
        call_stack = c.cast(call_stack_ffi, c.py_object).value

        obj.obj.emit_event(time, call_stack, event, message)
    except Exception as e:
        pass


def _subscriber_flush(ptr: int) -> None:
    try:
        obj: _SubscriberWrapper = c.cast(ptr, c.py_object).value
        obj.obj.flush()
    except Exception:
        pass


SubscriberCallStack = TypeVar(
    "SubscriberCallStack", bound=_ffi.FFITransferable[c.c_void_p]
)


class Subscriber(
    Generic[SubscriberCallStack], _ffi.FFITransferable[_ffi.FimoTracingSubscriber]
):
    """Interface of a tracing subscriber."""

    def transfer_to_ffi(self) -> _ffi.FimoTracingSubscriber:
        # Fill the vtable
        vtable = _ffi.FimoTracingSubscriberVTable()
        vtable.destroy = c.CFUNCTYPE(None, c.c_void_p)(_subscriber_destroy)
        vtable.call_stack_create = c.CFUNCTYPE(
            _ffi.FimoError, c.c_void_p, c.POINTER(_ffi.FimoTime), c.POINTER(c.c_void_p)
        )(_subscriber_call_stack_create)
        vtable.call_stack_drop = c.CFUNCTYPE(None, c.c_void_p, c.c_void_p)(
            _subscriber_call_stack_drop
        )
        vtable.call_stack_destroy = c.CFUNCTYPE(
            None, c.c_void_p, c.POINTER(_ffi.FimoTime), c.c_void_p
        )(_subscriber_call_stack_destroy)
        vtable.call_stack_unblock = c.CFUNCTYPE(
            None, c.c_void_p, c.POINTER(_ffi.FimoTime), c.c_void_p
        )(_subscriber_call_stack_unblock)
        vtable.call_stack_suspend = c.CFUNCTYPE(
            None, c.c_void_p, c.POINTER(_ffi.FimoTime), c.c_void_p, c.c_bool
        )(_subscriber_call_stack_suspend)
        vtable.call_stack_resume = c.CFUNCTYPE(
            None, c.c_void_p, c.POINTER(_ffi.FimoTime), c.c_void_p
        )(_subscriber_call_stack_resume)
        vtable.span_push = c.CFUNCTYPE(
            _ffi.FimoError,
            c.c_void_p,
            c.POINTER(_ffi.FimoTime),
            c.POINTER(_ffi.FimoTracingSpanDesc),
            c.POINTER(c.c_char),
            _ffi.FimoUSize,
            c.c_void_p,
        )(_subscriber_span_push)
        vtable.span_drop = c.CFUNCTYPE(None, c.c_void_p, c.c_void_p)(
            _subscriber_span_drop
        )
        vtable.span_pop = c.CFUNCTYPE(
            None, c.c_void_p, c.POINTER(_ffi.FimoTime), c.c_void_p
        )(_subscriber_span_pop)
        vtable.event_emit = c.CFUNCTYPE(
            None,
            c.c_void_p,
            c.POINTER(_ffi.FimoTime),
            c.c_void_p,
            c.POINTER(_ffi.FimoTracingEvent),
            c.POINTER(c.c_char),
            _ffi.FimoUSize,
        )(_subscriber_event_emit)
        vtable.flush = c.CFUNCTYPE(None, c.c_void_p)(_subscriber_flush)

        # Since we create the vtable dynamically we must take ownership of it
        class Wrapper:
            def __init__(
                self, vtable: _ffi.FimoTracingSubscriberVTable, obj: Subscriber
            ) -> None:
                self.vtable = vtable
                self.obj = obj

        wrapper = Wrapper(vtable, self)
        wrapper_ffi = ctypes.py_object(wrapper)

        # Create the struct
        subscriber = _ffi.FimoTracingSubscriber(
            _ffi.FimoStructType.FIMO_STRUCT_TYPE_TRACING_SUBSCRIBER,
            c.POINTER(_ffi.FimoBaseStructIn)(),
            c.c_void_p.from_buffer(wrapper_ffi),
            c.pointer(vtable),
        )

        # Since the subscriber will be passed to a C-interface it must take
        # ownership of the object. We do this by increasing the reference count
        _ffi.c_inc_ref(wrapper)
        return subscriber

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTracingSubscriber) -> Self:
        ptr = c.c_void_p(ffi.ptr)
        obj = c.cast(ptr, c.py_object).value
        if not isinstance(obj, cls):
            raise TypeError("`ffi` does not point to an instance of the `Subscriber`")

        return obj

    @abstractmethod
    def create_call_stack(self, time: Time) -> SubscriberCallStack:
        """Creates a new call stack."""
        pass

    @abstractmethod
    def drop_call_stack(self, call_stack: SubscriberCallStack) -> None:
        """Drops the call stack without tracing anything."""
        pass

    @abstractmethod
    def destroy_call_stack(self, time: Time, call_stack: SubscriberCallStack) -> None:
        """Destroys the call stack."""
        pass

    @abstractmethod
    def unblock_call_stack(self, time: Time, call_stack: SubscriberCallStack) -> None:
        """Marks the call stack as being unblocked."""
        pass

    @abstractmethod
    def suspend_call_stack(
        self, time: Time, call_stack: SubscriberCallStack, block: bool
    ) -> None:
        """Marks the stack as being suspended/blocked."""
        pass

    @abstractmethod
    def resume_call_stack(self, time: Time, call_stack: SubscriberCallStack) -> None:
        """Marks the stack as being resumed."""
        pass

    @abstractmethod
    def create_span(
        self,
        time: Time,
        span_descriptor: SpanDescriptor,
        message: bytes,
        call_stack: SubscriberCallStack,
    ) -> None:
        """Creates a new span."""
        pass

    @abstractmethod
    def drop_span(self, call_stack: SubscriberCallStack) -> None:
        """Drops the span without tracing anything."""
        pass

    @abstractmethod
    def destroy_span(self, time: Time, call_stack: SubscriberCallStack) -> None:
        """Exits and destroys a span."""
        pass

    @abstractmethod
    def emit_event(
        self, time: Time, call_stack: SubscriberCallStack, event: Event, message: bytes
    ) -> None:
        """Emits an event."""
        pass

    @abstractmethod
    def flush(self) -> None:
        """Flushes the messages of the `Subscriber`."""
        pass


class FfiSubscriberCallStack(_ffi.FFITransferable[c.c_void_p]):
    """A ffi subscriber call stack."""

    def __init__(self, ffi: c.c_void_p) -> None:
        if not isinstance(ffi, c.c_void_p):
            raise TypeError("`ffi` must be a void pointer.")
        if not ffi:
            raise ValueError("`ffi` may not be null.")

        self._ffi: Optional[c.c_void_p] = ffi

    def transfer_to_ffi(self) -> c.c_void_p:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberCallStack` has been consumed")

        ffi, self._ffi = self._ffi, None
        return ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: c.c_void_p) -> Self:
        return cls(ffi)


class FfiSubscriberView(
    Subscriber[FfiSubscriberCallStack],
    _ffi.FFISharable[_ffi.FimoTracingSubscriber, "FfiSubscriberView"],
):
    """A non-owning view to a ffi subscriber."""

    def __init__(self, ffi: _ffi.FimoTracingSubscriber) -> None:
        if not isinstance(ffi, _ffi.FimoTracingSubscriber):
            raise TypeError("`ffi` must be an instance of a `FimoTracingSubscriber`.")

        self._ffi: Optional[_ffi.FimoTracingSubscriber] = ffi

    def transfer_to_ffi(self) -> _ffi.FimoTracingSubscriber:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        return self._ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTracingSubscriber) -> Self:
        return cls(ffi)

    @property
    def ffi(self) -> _ffi.FimoTracingSubscriber:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        return self._ffi

    @classmethod
    def borrow_from_ffi(cls, ffi: _ffi.FimoTracingSubscriber) -> Self:
        return cls(ffi)

    def _consume(self) -> None:
        self._ffi = None

    def create_call_stack(self, time: Time) -> FfiSubscriberCallStack:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        ffi = c.c_void_p()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.call_stack_create
        err = ffi_fn(ptr, c.byref(time_ffi), c.byref(ffi))
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()
        return FfiSubscriberCallStack.transfer_from_ffi(ffi)

    def drop_call_stack(self, call_stack: FfiSubscriberCallStack) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        call_stack_ffi = call_stack.transfer_to_ffi()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.call_stack_drop
        ffi_fn(ptr, call_stack_ffi)

    def destroy_call_stack(
        self, time: Time, call_stack: FfiSubscriberCallStack
    ) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        call_stack_ffi = call_stack.transfer_to_ffi()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.call_stack_destroy
        ffi_fn(ptr, c.byref(time_ffi), call_stack_ffi)

    def unblock_call_stack(
        self, time: Time, call_stack: FfiSubscriberCallStack
    ) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        call_stack_ffi = call_stack.transfer_to_ffi()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.call_stack_unblock
        ffi_fn(ptr, c.byref(time_ffi), call_stack_ffi)

    def suspend_call_stack(
        self, time: Time, call_stack: FfiSubscriberCallStack, block: bool
    ) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        call_stack_ffi = call_stack.transfer_to_ffi()
        block_ffi = c.c_bool(block)

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.call_stack_suspend
        ffi_fn(ptr, c.byref(time_ffi), call_stack_ffi, block_ffi)

    def resume_call_stack(self, time: Time, call_stack: FfiSubscriberCallStack) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        call_stack_ffi = call_stack.transfer_to_ffi()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.call_stack_resume
        ffi_fn(ptr, c.byref(time_ffi), call_stack_ffi)

    def create_span(
        self,
        time: Time,
        span_descriptor: SpanDescriptor,
        message: bytes,
        call_stack: FfiSubscriberCallStack,
    ) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        span_descriptor_ffi = span_descriptor.transfer_to_ffi()
        message_ffi = c.cast(c.c_char_p(message), c.POINTER(c.c_char))
        message_len = _ffi.FimoUSize(len(message))
        call_stack_ffi = call_stack.transfer_to_ffi()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.span_push
        err = ffi_fn(
            ptr,
            c.byref(time_ffi),
            c.byref(span_descriptor_ffi),
            message_ffi,
            message_len,
            call_stack_ffi,
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def drop_span(self, call_stack: FfiSubscriberCallStack) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        call_stack_ffi = call_stack.transfer_to_ffi()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.span_drop
        ffi_fn(ptr, call_stack_ffi)

    def destroy_span(self, time: Time, call_stack: FfiSubscriberCallStack) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        call_stack_ffi = call_stack.transfer_to_ffi()

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.span_pop
        ffi_fn(ptr, c.byref(time_ffi), call_stack_ffi)

    def emit_event(
        self,
        time: Time,
        call_stack: FfiSubscriberCallStack,
        event: Event,
        message: bytes,
    ) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        time_ffi = time.transfer_to_ffi()
        call_stack_ffi = call_stack.transfer_to_ffi()
        event_ffi = event.transfer_to_ffi()
        message_ffi = c.cast(c.c_char_p(message), c.POINTER(c.c_char))
        message_len = _ffi.FimoUSize(len(message))

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.event_emit
        ffi_fn(
            ptr,
            c.byref(time_ffi),
            call_stack_ffi,
            c.byref(event_ffi),
            message_ffi,
            message_len,
        )

    def flush(self) -> None:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        ptr = self._ffi.ptr
        ffi_fn = self._ffi.vtable.contents.flush
        ffi_fn(ptr)


class FfiSubscriber(FfiSubscriberView):
    """A ffi subscriber."""

    def __init__(self, ffi: _ffi.FimoTracingSubscriber) -> None:
        super().__init__(ffi)

    def __del__(self) -> None:
        if self._ffi is None:
            return

        ptr = self._ffi.ptr
        destroy_fn = self._ffi.vtable.contents.destroy
        destroy_fn(ptr)
        self._consume()

    def transfer_to_ffi(self) -> _ffi.FimoTracingSubscriber:
        if self._ffi is None:
            raise ValueError("the `FfiSubscriberView` has been consumed")

        ffi = self._ffi
        self._consume()
        return ffi

    @classmethod
    def transfer_from_ffi(cls, ffi: _ffi.FimoTracingSubscriber) -> Self:
        return cls(ffi)


DefaultSubscriber = FfiSubscriberView.transfer_from_ffi(
    _ffi.FIMO_TRACING_DEFAULT_SUBSCRIBER
)
"""Default subscriber."""


class TracingCtx:
    """Entry point to the tracing subsystem."""

    def __init__(self, ctx: _ContextView) -> None:
        if not isinstance(ctx, context.ContextView):
            raise TypeError("`ctx` must be an instance of `ContextView`")

        self._ctx = ctx

    @property
    def context(self) -> _ContextView:
        """Returns the `ContextView`."""
        return self._ctx

    def emit_event(self, event: Event, msg: str, *args, **kwargs) -> None:
        """Emits a new event.

        The message may be cut of, if the length exceeds the internal formatting
        buffer size.
        """
        if not isinstance(event, Event):
            raise TypeError("`event` must be an instance of `Event`")
        if not isinstance(msg, str):
            raise TypeError("`msg` must be a `str`")

        format_args = c.cast(
            c.pointer(c.py_object(_FormatArgs(msg, *args, **kwargs))), c.c_void_p
        )
        format_func = _ffi.FimoTracingFormat(_format_msg)
        err = _ffi.fimo_tracing_event_emit_custom(
            self._ctx.ffi, c.byref(event.transfer_to_ffi()), format_func, format_args
        )
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()

    def _emit_event(self, level: Level, msg: str, *args, **kwargs):
        curr_frame = inspect.currentframe()
        caller_frame = inspect.getouterframes(curr_frame, 3)[2]

        module_name = caller_frame.frame.f_globals["__name__"]
        func_name = caller_frame.function
        file_name = caller_frame.filename
        line_number = caller_frame.lineno

        metadata = Metadata(module_name, func_name, level, file_name, line_number)
        event = Event(metadata)
        self.emit_event(event, msg, *args, **kwargs)

    def emit_error(self, msg: str, *args, **kwargs) -> None:
        """Emits an error event."""
        self._emit_event(Level.Error, msg, *args, **kwargs)

    def emit_warn(self, msg: str, *args, **kwargs) -> None:
        """Emits a warn event."""
        self._emit_event(Level.Warn, msg, *args, **kwargs)

    def emit_info(self, msg: str, *args, **kwargs) -> None:
        """Emits an info event."""
        self._emit_event(Level.Info, msg, *args, **kwargs)

    def emit_debug(self, msg: str, *args, **kwargs) -> None:
        """Emits a debug event."""
        self._emit_event(Level.Debug, msg, *args, **kwargs)

    def emit_trace(self, msg: str, *args, **kwargs) -> None:
        """Emits a trace event."""
        self._emit_event(Level.Trace, msg, *args, **kwargs)

    def is_enabled(self) -> bool:
        """Checks whether the tracing subsystem is enabled.

        This function can be used to check whether to call into the subsystem at all.
        Calling this function is not necessary, as the remaining functions of the
        backend are guaranteed to return default values, in case the backend is
        disabled.

        :return: `True` if the subsystem is enabled.
        """
        return _ffi.fimo_tracing_is_enabled(self._ctx.ffi)

    def flush(self) -> None:
        """Flushes the streams used for tracing.

        If successful, any unwritten data is written out by the individual subscribers.
        """
        err = _ffi.fimo_tracing_flush(self._ctx.ffi)
        error.ErrorCode.transfer_from_ffi(err).raise_if_error()


class CreationConfig(context.ContextOption):
    """Configuration of the tracing subsystem."""

    def __init__(self) -> None:
        self._format_buffer_len = 0
        self._max_level = Level.Off
        self._subscribers: list[Subscriber] = []

    def with_format_buffer_length(self, format_buffer_length: Optional[int]) -> Self:
        """Sets the format-buffer length of the tracing subsystem."""
        if (
            not isinstance(format_buffer_length, int)
            and format_buffer_length is not None
        ):
            raise TypeError("`format_buffer_length` must be an `int` or `None`.")
        self._format_buffer_len = (
            0 if format_buffer_length is None else format_buffer_length
        )
        return self

    def with_max_level(self, level: Optional[Level]) -> Self:
        """Sets the maximum level of the tracing subsystem."""
        if not isinstance(level, int) and level is not None:
            raise TypeError("`level` must be an instance of `Level` or `None`.")
        self._max_level = Level.Off if level is None else level
        return self

    def with_subscriber(self, subscriber: Subscriber) -> Self:
        """Appends a subscriber to the config."""
        if not isinstance(subscriber, Subscriber):
            raise TypeError("`subscriber` must be a subclass of `Subscriber`.")
        if subscriber in self._subscribers:
            raise ValueError("`subscriber` is already contained")
        self._subscribers.append(subscriber)
        return self

    def to_context_option(self) -> c._Pointer[_ffi.FimoBaseStructIn]:
        format_buffer_size_ffi = _ffi.FimoUSize(self._format_buffer_len)
        max_level_ffi = self._max_level.transfer_to_ffi()
        subscribers_ffi = (_ffi.FimoTracingSubscriber * len(self._subscribers))()
        subscriber_count = _ffi.FimoUSize(len(self._subscribers))

        for i, subscriber in enumerate(self._subscribers):
            subscribers_ffi[i] = subscriber.transfer_to_ffi()

        config = _ffi.FimoTracingCreationConfig(
            _ffi.FimoStructType.FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG,
            c.POINTER(_ffi.FimoBaseStructIn)(),
            format_buffer_size_ffi,
            max_level_ffi,
            subscribers_ffi,
            subscriber_count,
        )
        config_ptr = c.pointer(config)
        return c.cast(config_ptr, c.POINTER(_ffi.FimoBaseStructIn))
