import sys
import ctypes as c
from typing import Self

from ..context import Context
from .. import tracing
from .. import time
from .. import ffi


class CallStack(ffi.FFITransferable[c.c_void_p]):
    def __init__(self):
        self.spans = []

    def __del__(self):
        print("dropping call stack")

    def transfer_to_ffi(self) -> c.c_void_p:
        ptr = c.c_void_p(id(self))
        ffi.c_inc_ref(self)
        return ptr

    @classmethod
    def transfer_from_ffi(cls, ptr: c.c_void_p) -> Self:
        obj = c.cast(ptr, c.py_object).value
        if not isinstance(obj, cls):
            raise TypeError()

        return obj

    def push_span(self, desc: tracing.SpanDescriptor, message: bytes):
        self.spans.append((desc, message.decode()))

    def pop_span(self):
        self.spans.pop()


class Subscriber(tracing.Subscriber[CallStack]):
    def __del__(self):
        print("dropping subscriber")

    def create_call_stack(self, t: time.Time) -> CallStack:
        return CallStack()

    def drop_call_stack(self, call_stack: CallStack) -> None:
        ffi.c_dec_ref(call_stack)

    def destroy_call_stack(self, t: time.Time, call_stack: CallStack) -> None:
        ffi.c_dec_ref(call_stack)

    def unblock_call_stack(self, t: time.Time, call_stack: CallStack) -> None:
        pass

    def suspend_call_stack(self, t: time.Time, call_stack: CallStack, block: bool) -> None:
        pass

    def resume_call_stack(self, t: time.Time, call_stack: CallStack) -> None:
        pass

    def create_span(self, t: time.Time, span_descriptor: tracing.SpanDescriptor, message: bytes,
                    call_stack: CallStack) -> None:
        call_stack.push_span(span_descriptor, message)

    def drop_span(self, call_stack: CallStack) -> None:
        call_stack.pop_span()

    def destroy_span(self, t: time.Time, call_stack: CallStack) -> None:
        call_stack.pop_span()

    def emit_event(self, t: time.Time, call_stack: CallStack, event: tracing.Event, message: bytes) -> None:
        match event.metadata.level:
            case tracing.Level.Error:
                event_type = "ERROR"
            case tracing.Level.Warn:
                event_type = "WARN"
            case tracing.Level.Info:
                event_type = "INFO"
            case tracing.Level.Debug:
                event_type = "DEBUG"
            case tracing.Level.Trace:
                event_type = "TRACE"
            case _:
                return

        msg = f"{event_type} {message.decode()}"
        if event.metadata.level == tracing.Level.Error:
            sys.stdout.flush()
            print(msg, file=sys.stderr)
        else:
            print(msg)

    def flush(self) -> None:
        sys.stdout.flush()


def test_enabled():
    context = Context.new_context()
    assert context.tracing().is_enabled() is False
    del context

    tracing_config = tracing.CreationConfig().with_max_level(tracing.Level.Trace).with_subscriber(
        tracing.DefaultSubscriber)
    context = Context.new_context([tracing_config])
    assert context.tracing().is_enabled() is True


def test_events():
    tracing_config = tracing.CreationConfig().with_max_level(tracing.Level.Trace).with_subscriber(
        tracing.DefaultSubscriber)
    context = Context.new_context([tracing_config])

    with tracing.ThreadAccess(context) as t:
        t.context.tracing().emit_error('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_warn('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_info('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_debug('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_trace('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().flush()


def test_span():
    tracing_config = tracing.CreationConfig().with_max_level(tracing.Level.Trace).with_subscriber(
        tracing.DefaultSubscriber)
    context = Context.new_context([tracing_config])

    with tracing.ThreadAccess(context) as t:
        with tracing.Span.error_span(t.context, "Error span: a={}, b={}, c={c}", "a", "b", c="c"):
            t.context.tracing().emit_error('Message: x={}, y={}, z={z}', 0, 1, z=2)
        with tracing.Span.warn_span(t.context, "Warn span: a={}, b={}, c={c}", "a", "b", c="c"):
            t.context.tracing().emit_warn('Message: x={}, y={}, z={z}', 0, 1, z=2)
        with tracing.Span.info_span(t.context, "Info span: a={}, b={}, c={c}", "a", "b", c="c"):
            t.context.tracing().emit_info('Message: x={}, y={}, z={z}', 0, 1, z=2)
        with tracing.Span.debug_span(t.context, "Debug span: a={}, b={}, c={c}", "a", "b", c="c"):
            t.context.tracing().emit_debug('Message: x={}, y={}, z={z}', 0, 1, z=2)
        with tracing.Span.trace_span(t.context, "Trace span: a={}, b={}, c={c}", "a", "b", c="c"):
            t.context.tracing().emit_trace('Message: x={}, y={}, z={z}', 0, 1, z=2)


def test_custom_subscriber():
    tracing_config = tracing.CreationConfig().with_max_level(tracing.Level.Trace).with_subscriber(
        Subscriber())
    context = Context.new_context([tracing_config])

    with tracing.ThreadAccess(context) as t:
        t.context.tracing().emit_error('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_warn('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_info('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_debug('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().emit_trace('Message: x={}, y={}, z={z}', 0, 1, z=2)
        t.context.tracing().flush()

    del context
    sys.stdout.flush()
