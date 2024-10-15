#ifndef FIMO_INTERNAL_TRACING_H
#define FIMO_INTERNAL_TRACING_H

#include <stdarg.h>
#include <stdatomic.h>
#include <stddef.h>

#include <fimo_std/array_list.h>
#include <fimo_std/error.h>
#include <fimo_std/tracing.h>

#if __APPLE__
#include <tinycthread/tinycthread.h>
#else
#include <threads.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define FIMO_INTERNAL_TRACING_EMIT_(CTX, NAME, TARGET, LVL, FMT, META_VAR, EVENT_VAR, ERROR_VAR, ...)                  \
    FIMO_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FIMO_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-zero-length")                                                     \
    static const FimoTracingMetadata META_VAR = {                                                                      \
            .type = FIMO_STRUCT_TYPE_TRACING_METADATA,                                                                 \
            .next = NULL,                                                                                              \
            .name = (NAME),                                                                                            \
            .target = (TARGET),                                                                                        \
            .level = (LVL),                                                                                            \
            .file_name = __FILE__,                                                                                     \
            .line_number = __LINE__,                                                                                   \
    };                                                                                                                 \
    static const FimoTracingEvent EVENT_VAR = {                                                                        \
            .type = FIMO_STRUCT_TYPE_TRACING_EVENT,                                                                    \
            .next = NULL,                                                                                              \
            .metadata = &META_VAR,                                                                                     \
    };                                                                                                                 \
    FimoResult ERROR_VAR = fimo_internal_tracing_event_emit_fmt(CTX, &EVENT_VAR, FMT, __VA_ARGS__);                    \
    FIMO_ASSERT_FALSE(FIMO_RESULT_IS_ERROR(ERROR_VAR))                                                                 \
    FIMO_PRAGMA_GCC(GCC diagnostic pop)

/**
 * Emits a new event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param LVL event level
 * @param FMT printf format string
 * @param args printf format args
 */
#define FIMO_INTERNAL_TRACING_EMIT(CTX, NAME, TARGET, LVL, FMT, ...)                                                   \
    FIMO_INTERNAL_TRACING_EMIT_(CTX, NAME, TARGET, LVL, FMT, FIMO_VAR(_fimo_private_metadata_),                        \
                                FIMO_VAR(_fimo_private_event_), FIMO_VAR(_fimo_private_error_), __VA_ARGS__)

/**
 * Emits an error event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 * @param ARGS printf format args
 */
#define FIMO_INTERNAL_TRACING_EMIT_ERROR(CTX, NAME, TARGET, FMT, ...)                                                  \
    FIMO_INTERNAL_TRACING_EMIT(CTX, NAME, TARGET, FIMO_TRACING_LEVEL_ERROR, FMT, __VA_ARGS__)

/**
 * Emits an error event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 */
#define FIMO_INTERNAL_TRACING_EMIT_ERROR_SIMPLE(CTX, NAME, TARGET, FMT)                                                \
    FIMO_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FIMO_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-extra-args")                                                      \
    FIMO_INTERNAL_TRACING_EMIT_ERROR(CTX, NAME, TARGET, FMT, 0)                                                        \
    FIMO_PRAGMA_GCC(GCC diagnostic pop)

/**
 * Emits a warning event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 * @param ARGS printf format args
 */
#define FIMO_INTERNAL_TRACING_EMIT_WARN(CTX, NAME, TARGET, FMT, ...)                                                   \
    FIMO_INTERNAL_TRACING_EMIT(CTX, NAME, TARGET, FIMO_TRACING_LEVEL_WARN, FMT, __VA_ARGS__)

/**
 * Emits a warning event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 */
#define FIMO_INTERNAL_TRACING_EMIT_WARN_SIMPLE(CTX, NAME, TARGET, FMT)                                                 \
    FIMO_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FIMO_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-extra-args")                                                      \
    FIMO_INTERNAL_TRACING_EMIT_WARN(CTX, NAME, TARGET, FMT, 0)                                                         \
    FIMO_PRAGMA_GCC(GCC diagnostic pop)

/**
 * Emits an info event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 * @param ARGS printf format args
 */
#define FIMO_INTERNAL_TRACING_EMIT_INFO(CTX, NAME, TARGET, FMT, ...)                                                   \
    FIMO_INTERNAL_TRACING_EMIT(CTX, NAME, TARGET, FIMO_TRACING_LEVEL_INFO, FMT, __VA_ARGS__)

/**
 * Emits an info event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 */
#define FIMO_INTERNAL_TRACING_EMIT_INFO_SIMPLE(CTX, NAME, TARGET, FMT)                                                 \
    FIMO_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FIMO_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-extra-args")                                                      \
    FIMO_INTERNAL_TRACING_EMIT_INFO(CTX, NAME, TARGET, FMT, 0)                                                         \
    FIMO_PRAGMA_GCC(GCC diagnostic pop)

/**
 * Emits a debug event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 * @param ARGS printf format args
 */
#define FIMO_INTERNAL_TRACING_EMIT_DEBUG(CTX, NAME, TARGET, FMT, ...)                                                  \
    FIMO_INTERNAL_TRACING_EMIT(CTX, NAME, TARGET, FIMO_TRACING_LEVEL_DEBUG, FMT, __VA_ARGS__)

/**
 * Emits a debug event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 */
#define FIMO_INTERNAL_TRACING_EMIT_DEBUG_SIMPLE(CTX, NAME, TARGET, FMT)                                                \
    FIMO_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FIMO_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-extra-args")                                                      \
    FIMO_INTERNAL_TRACING_EMIT_DEBUG(CTX, NAME, TARGET, FMT, 0)                                                        \
    FIMO_PRAGMA_GCC(GCC diagnostic pop)

/**
 * Emits a trace event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 * @param ARGS printf format args
 */
#define FIMO_INTERNAL_TRACING_EMIT_TRACE(CTX, NAME, TARGET, FMT, ...)                                                  \
    FIMO_INTERNAL_TRACING_EMIT(CTX, NAME, TARGET, FIMO_TRACING_LEVEL_TRACE, FMT, __VA_ARGS__)

/**
 * Emits a trace event using the default formatter.
 *
 * @param CTX the context
 * @param NAME event name
 * @param TARGET event target
 * @param FMT printf format string
 */
#define FIMO_INTERNAL_TRACING_EMIT_TRACE_SIMPLE(CTX, NAME, TARGET, FMT)                                                \
    FIMO_PRAGMA_GCC(GCC diagnostic push)                                                                               \
    FIMO_PRAGMA_GCC(GCC diagnostic ignored "-Wformat-extra-args")                                                      \
    FIMO_INTERNAL_TRACING_EMIT_TRACE(CTX, NAME, TARGET, FMT, 0)                                                        \
    FIMO_PRAGMA_GCC(GCC diagnostic pop)

/**
 * Tracing backend state.
 */
typedef struct FimoInternalTracingContext FimoInternalTracingContext;

///////////////////////////////////////////////////////////////////////
//// Trampoline functions
///////////////////////////////////////////////////////////////////////

FimoResult fimo_internal_trampoline_tracing_call_stack_create(void *ctx, FimoTracingCallStack **call_stack);
FimoResult fimo_internal_trampoline_tracing_call_stack_destroy(void *ctx, FimoTracingCallStack *call_stack);
FimoResult fimo_internal_trampoline_tracing_call_stack_switch(void *ctx, FimoTracingCallStack *call_stack,
                                                              FimoTracingCallStack **old);
FimoResult fimo_internal_trampoline_tracing_call_stack_unblock(void *ctx, FimoTracingCallStack *call_stack);
FimoResult fimo_internal_trampoline_tracing_call_stack_suspend_current(void *ctx, bool block);
FimoResult fimo_internal_trampoline_tracing_call_stack_resume_current(void *ctx);
FimoResult fimo_internal_trampoline_tracing_span_create(void *ctx, const FimoTracingSpanDesc *span_desc,
                                                        FimoTracingSpan **span, FimoTracingFormat format,
                                                        const void *data);
FimoResult fimo_internal_trampoline_tracing_span_destroy(void *ctx, FimoTracingSpan *span);
FimoResult fimo_internal_trampoline_tracing_event_emit(void *ctx, const FimoTracingEvent *event,
                                                       FimoTracingFormat format, const void *data);
bool fimo_internal_trampoline_tracing_is_enabled(void *ctx);
FimoResult fimo_internal_trampoline_tracing_register_thread(void *ctx);
FimoResult fimo_internal_trampoline_tracing_unregister_thread(void *ctx);
FimoResult fimo_internal_trampoline_tracing_flush(void *ctx);

///////////////////////////////////////////////////////////////////////
//// Tracing Subsystem API
///////////////////////////////////////////////////////////////////////


FimoInternalTracingContext* fimo_internal_tracing_alloc(void);
void fimo_internal_tracing_dealloc(FimoInternalTracingContext *ctx);

/**
 * Initializes the tracing backend.
 *
 * If `options` is `NULL`, the backend is initialized with the default options,
 * i.e., it is disabled.
 *
 * @param ctx partially initialized context
 * @param options init options
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoResult fimo_internal_tracing_init(FimoInternalTracingContext *ctx, const FimoTracingCreationConfig *options);

/**
 * Destroys the backend.
 *
 * Calls exit if the backend can not be destroyed. The caller must ensure that they
 * are responsible for destroying the context.
 *
 * @param ctx the context.
 */
void fimo_internal_tracing_destroy(FimoInternalTracingContext *ctx);

/**
 * Cleans up the resources specified in the options.
 *
 * @param options init options
 */
void fimo_internal_tracing_cleanup_options(const FimoTracingCreationConfig *options);

/**
 * Emits a new event with a custom formatter.
 *
 * The backend may use a formatting buffer of a fixed size. The formatter is
 * expected to cut-of the message after reaching that specified size.
 *
 * @param ctx the context
 * @param event the event to emit
 * @param format custom formatting function
 * @param data custom data to format
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoResult fimo_internal_tracing_event_emit_custom(FimoInternalTracingContext *ctx, const FimoTracingEvent *event,
                                                   FimoTracingFormat format, const void *data);

/**
 * Emits a new event with the standard formatter.
 *
 * The message is formatted as if it were formatted by a call to `snprintf`.
 * The message may be cut of, if the length exceeds the internal formatting
 * buffer size.
 *
 * @param ctx the context
 * @param event the event to emit
 * @param format formatting string
 * @param ... format arguments
 *
 * @return Status code.
 */
static
FIMO_MUST_USE
FIMO_INLINE_ALWAYS
FIMO_PRINT_F_FORMAT_ATTR(3, 4)
FimoResult fimo_internal_tracing_event_emit_fmt(FimoInternalTracingContext *ctx, const FimoTracingEvent *event,
                                       			FIMO_PRINT_F_FORMAT const char *format, ...) {
    va_list vlist;
    va_start(vlist, format);
    FimoImplTracingFmtArgs args = {.format = format, .vlist = &vlist};
    FimoResult result = fimo_internal_tracing_event_emit_custom(ctx, event, fimo_impl_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

#ifdef __cplusplus
}
#endif

#endif // FIMO_INTERNAL_TRACING_H
