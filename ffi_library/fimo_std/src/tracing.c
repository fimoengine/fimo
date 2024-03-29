#include <fimo_std/tracing.h>

#include <stdarg.h>
#include <stdio.h>

#include <fimo_std/internal/tracing.h>

#include <fimo_std/vtable.h>

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_create(const FimoContext context, FimoTracingCallStack *call_stack) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_create(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_destroy(const FimoContext context, const FimoTracingCallStack call_stack) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_destroy(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_switch(const FimoContext context, const FimoTracingCallStack new_call_stack,
                                         FimoTracingCallStack *old_call_stack) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_switch(context.data, new_call_stack, old_call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_unblock(const FimoContext context, const FimoTracingCallStack call_stack) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_unblock(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_suspend_current(const FimoContext context, const bool block) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_suspend_current(context.data, block);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_resume_current(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_resume_current(context.data);
}

FIMO_PRINT_F_FORMAT_ATTR(4, 5)
FIMO_MUST_USE
FimoError fimo_tracing_span_create_fmt(const FimoContext context, const FimoTracingSpanDesc *span_desc,
                                       FimoTracingSpan *span, FIMO_PRINT_F_FORMAT const char *format, ...) {
    va_list vlist;
    va_start(vlist, format);
    FimoInternalTracingFmtArgs args = {.format = format, .vlist = &vlist};
    FimoError result = fimo_tracing_span_create_custom(context, span_desc, span, fimo_internal_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_tracing_span_create_custom(const FimoContext context, const FimoTracingSpanDesc *span_desc,
                                          FimoTracingSpan *span, const FimoTracingFormat format, const void *data) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.span_create(context.data, span_desc, span, format, data);
}

FIMO_MUST_USE
FimoError fimo_tracing_span_destroy(const FimoContext context, FimoTracingSpan *span) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.span_destroy(context.data, span);
}

FIMO_PRINT_F_FORMAT_ATTR(3, 4)
FIMO_MUST_USE
FimoError fimo_tracing_event_emit_fmt(const FimoContext context, const FimoTracingEvent *event,
                                      FIMO_PRINT_F_FORMAT const char *format, ...) {
    va_list vlist;
    va_start(vlist, format);
    FimoInternalTracingFmtArgs args = {.format = format, .vlist = &vlist};
    FimoError result = fimo_tracing_event_emit_custom(context, event, fimo_internal_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_tracing_event_emit_custom(const FimoContext context, const FimoTracingEvent *event,
                                         const FimoTracingFormat format, const void *data) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.event_emit(context.data, event, format, data);
}

FIMO_MUST_USE
bool fimo_tracing_is_enabled(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.is_enabled(context.data);
}

FIMO_MUST_USE
FimoError fimo_tracing_flush(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.flush(context.data);
}
