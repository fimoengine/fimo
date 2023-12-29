#include <fimo_std/tracing.h>

#include <stdarg.h>
#include <stdio.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/internal/tracing.h>

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_create(FimoContext context,
    FimoTracingCallStack* call_stack)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_call_stack_create(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_destroy(FimoContext context,
    FimoTracingCallStack call_stack)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_call_stack_destroy(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_switch(FimoContext context,
    FimoTracingCallStack new_call_stack, FimoTracingCallStack* old_call_stack)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_call_stack_switch(context.data, new_call_stack, old_call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_unblock(FimoContext context,
    FimoTracingCallStack call_stack)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_call_stack_unblock(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_suspend_current(FimoContext context,
    bool block)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_call_stack_suspend_current(context.data, block);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_resume_current(FimoContext context)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_call_stack_resume_current(context.data);
}

FIMO_PRINT_F_FORMAT_ATTR(4, 5)
FIMO_MUST_USE
FimoError fimo_tracing_span_create_fmt(FimoContext context,
    const FimoTracingSpanDesc* span_desc, FimoTracingSpan* span,
    FIMO_PRINT_F_FORMAT const char* format, ...)
{
    va_list vlist;
    va_start(vlist, format);
    FimoInternalTracingFmtArgs args = { .format = format, .vlist = &vlist };
    FimoError result = fimo_tracing_span_create_custom(context, span_desc, span, fimo_internal_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_tracing_span_create_custom(FimoContext context,
    const FimoTracingSpanDesc* span_desc, FimoTracingSpan* span,
    FimoTracingFormat format, const void* data)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_span_create(context.data, span_desc, span, format, data);
}

FIMO_MUST_USE
FimoError fimo_tracing_span_destroy(FimoContext context, FimoTracingSpan* span)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_span_destroy(context.data, span);
}

FIMO_PRINT_F_FORMAT_ATTR(3, 4)
FIMO_MUST_USE
FimoError fimo_tracing_event_emit_fmt(FimoContext context,
    const FimoTracingEvent* event, FIMO_PRINT_F_FORMAT const char* format, ...)
{
    va_list vlist;
    va_start(vlist, format);
    FimoInternalTracingFmtArgs args = { .format = format, .vlist = &vlist };
    FimoError result = fimo_tracing_event_emit_custom(context, event, fimo_internal_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_tracing_event_emit_custom(FimoContext context,
    const FimoTracingEvent* event, FimoTracingFormat format, const void* data)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_event_emit(context.data, event, format, data);
}

FIMO_MUST_USE
bool fimo_tracing_is_enabled(FimoContext context)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_is_enabled(context.data);
}

FIMO_MUST_USE
FimoError fimo_tracing_flush(FimoContext context)
{
    const FimoInternalContextVTable* vtable = context.vtable;
    return vtable->tracing_flush(context.data);
}
