#ifndef FIMO_IMPL_TRACING_H
#define FIMO_IMPL_TRACING_H

#include <stdarg.h>
#include <stdbool.h>

#include <fimo_std/error.h>
#include <fimo_std/time.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Argument type for the standard formatter.
 */
typedef struct FimoImplTracingFmtArgs {
    /// `vprintf` format string.
    const char *format;
    /// `vprintf` argument list.
    va_list *vlist;
} FimoImplTracingFmtArgs;

/**
 * Standard formatter.
 *
 * This functions acts like a call to `vsnprintf`, where the format string
 * and arguments are stored in `args`. The number of written bytes is
 * written into `written_bytes`. `args` must point to an instance of a
 * `FimoInternalTracingFmtArgs`.
 *
 * @param buffer destination buffer
 * @param buffer_size size of the buffer
 * @param args formatting args
 * @param written_size pointer to the count of written bytes
 *
 * @return Status code.
 */
FimoResult fimo_impl_tracing_fmt(char *buffer, FimoUSize buffer_size, const void *args, FimoUSize *written_size);

typedef struct FimoTracingSpanDesc FimoTracingSpanDesc;
typedef struct FimoTracingEvent FimoTracingEvent;

///////////////////////////////////////////////////////////////////////
//// Default Subscriber
///////////////////////////////////////////////////////////////////////

FimoResult fimo_impl_tracing_default_subscriber_call_stack_create(void *subscriber, const FimoTime *time, void **stack);
void fimo_impl_tracing_default_subscriber_call_stack_drop(void *subscriber, void *stack);
void fimo_impl_tracing_default_subscriber_call_stack_destroy(void *subscriber, const FimoTime *time, void *stack);
void fimo_impl_tracing_default_subscriber_call_stack_unblock(void *subscriber, const FimoTime *time, void *stack);
void fimo_impl_tracing_default_subscriber_call_stack_suspend(void *subscriber, const FimoTime *time, void *stack,
                                                             bool block);
void fimo_impl_tracing_default_subscriber_call_stack_resume(void *subscriber, const FimoTime *time, void *stack);
FimoResult fimo_impl_tracing_default_subscriber_span_push(void *subscriber, const FimoTime *time,
                                                          const FimoTracingSpanDesc *span_desc, const char *message,
                                                          FimoUSize message_len, void *stack);
void fimo_impl_tracing_default_subscriber_span_drop(void *subscriber, void *stack);
void fimo_impl_tracing_default_subscriber_span_pop(void *subscriber, const FimoTime *time, void *stack);
void fimo_impl_tracing_default_subscriber_event_emit(void *subscriber, const FimoTime *time, void *stack,
                                                     const FimoTracingEvent *event, const char *message,
                                                     FimoUSize message_len);
void fimo_impl_tracing_default_subscriber_flush(void *subscriber);

#ifdef __cplusplus
}
#endif

#endif // FIMO_IMPL_TRACING_H
