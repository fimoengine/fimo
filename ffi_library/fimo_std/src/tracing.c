#include <fimo_std/tracing.h>

#include <stdio.h>

#include <fimo_std/vtable.h>

static const FimoTracingSubscriberVTable FIMO_IMPL_TRACING_DEFAULT_SUBSCRIBER_VTABLE = {
        .destroy = NULL,
        .call_stack_create = fimo_impl_tracing_default_subscriber_call_stack_create,
        .call_stack_drop = fimo_impl_tracing_default_subscriber_call_stack_drop,
        .call_stack_destroy = fimo_impl_tracing_default_subscriber_call_stack_destroy,
        .call_stack_unblock = fimo_impl_tracing_default_subscriber_call_stack_unblock,
        .call_stack_suspend = fimo_impl_tracing_default_subscriber_call_stack_suspend,
        .call_stack_resume = fimo_impl_tracing_default_subscriber_call_stack_resume,
        .span_push = fimo_impl_tracing_default_subscriber_span_push,
        .span_drop = fimo_impl_tracing_default_subscriber_span_drop,
        .span_pop = fimo_impl_tracing_default_subscriber_span_pop,
        .event_emit = fimo_impl_tracing_default_subscriber_event_emit,
        .flush = fimo_impl_tracing_default_subscriber_flush,
};

const FimoTracingSubscriber FIMO_TRACING_DEFAULT_SUBSCRIBER = {
        .type = FIMO_STRUCT_TYPE_TRACING_SUBSCRIBER,
        .next = NULL,
        .ptr = NULL,
        .vtable = &FIMO_IMPL_TRACING_DEFAULT_SUBSCRIBER_VTABLE,
};

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_create(const FimoContext context, FimoTracingCallStack **call_stack) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_create(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_destroy(const FimoContext context, FimoTracingCallStack *call_stack) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_destroy(context.data, call_stack);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_switch(const FimoContext context, FimoTracingCallStack *call_stack,
                                         FimoTracingCallStack **old) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.call_stack_switch(context.data, call_stack, old);
}

FIMO_MUST_USE
FimoError fimo_tracing_call_stack_unblock(const FimoContext context, FimoTracingCallStack *call_stack) {
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
                                       FimoTracingSpan **span, FIMO_PRINT_F_FORMAT const char *format, ...) {
    va_list vlist;
    va_start(vlist, format);
    FimoImplTracingFmtArgs args = {.format = format, .vlist = &vlist};
    FimoError result = fimo_tracing_span_create_custom(context, span_desc, span, fimo_impl_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_tracing_span_create_custom(const FimoContext context, const FimoTracingSpanDesc *span_desc,
                                          FimoTracingSpan **span, const FimoTracingFormat format, const void *data) {
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
    FimoImplTracingFmtArgs args = {.format = format, .vlist = &vlist};
    FimoError result = fimo_tracing_event_emit_custom(context, event, fimo_impl_tracing_fmt, &args);
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
FimoError fimo_tracing_register_thread(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.register_thread(context.data);
}

FIMO_MUST_USE
FimoError fimo_tracing_unregister_thread(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.unregister_thread(context.data);
}

FIMO_MUST_USE
FimoError fimo_tracing_flush(const FimoContext context) {
    const FimoContextVTable *vtable = context.vtable;
    return vtable->tracing_v0.flush(context.data);
}
