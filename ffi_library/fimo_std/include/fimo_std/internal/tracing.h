#ifndef FIMO_INTERNAL_TRACING_H
#define FIMO_INTERNAL_TRACING_H

#include <stdarg.h>
#include <stdatomic.h>
#include <stddef.h>

#include <fimo_std/error.h>
#include <fimo_std/tracing.h>

#include <tinycthread/tinycthread.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FIMO_INTERNAL_TRACING_EMIT_(CTX, NAME, TARGET, LVL, FMT, META_VAR, EVENT_VAR, ERROR_VAR, ...)                  \
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
    FimoError ERROR_VAR = fimo_internal_tracing_event_emit_fmt(CTX, &EVENT_VAR, FMT, __VA_ARGS__);                     \
    FIMO_ASSERT_FALSE(FIMO_IS_ERROR(ERROR_VAR))

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

typedef struct FimoInternalContext FimoInternalContext;

/**
 * Tracing backend state.
 */
typedef struct FimoInternalContextTracing {
    FimoTracingSubscriber *subscribers;
    FimoUSize subscriber_count;
    FimoUSize format_buffer_size;
    FimoTracingLevel maximum_level;
    tss_t thread_local_data;
    atomic_size_t thread_count;
} FimoInternalContextTracing;

/**
 * Argument type for the standard formatter.
 */
typedef struct FimoInternalTracingFmtArgs {
    /// `vprintf` format string.
    const char *format;
    /// `vprintf` argument list.
    va_list *vlist;
} FimoInternalTracingFmtArgs;

/**
 * Initializes the tracing backend.
 *
 * If `options` is `NULL`, the backend is initialized with the default options,
 * i.e., it is disabled.
 *
 * @param context partially initialized context
 * @param options init options
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_init(FimoInternalContext *context, const FimoTracingCreationConfig *options);

/**
 * Checks whether the backend can be destroyed.
 *
 * The backend can be destroyed, if this functions returns without producing an
 * error.
 *
 * @param context the context.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_check_destroy(FimoInternalContext *context);

/**
 * Destroys the backend.
 *
 * Calls exit if the backend can not be destroyed. The caller must ensure that they
 * are responsible for destroying the context.
 *
 * @param context the context.
 */
void fimo_internal_tracing_destroy(FimoInternalContext *context);

/**
 * Creates a new empty call stack.
 *
 * If successful, the new call stack is marked as suspended, and written
 * into `call_stack`. The new call stack is not set to be the active call
 * stack.
 *
 * @param context the context
 * @param call_stack pointer to the resulting call stack
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_create(void *context, FimoTracingCallStack *call_stack);

/**
 * Destroys an empty call stack.
 *
 * Marks the completion of a task. Before calling this function, the
 * call stack must be empty, i.e., there must be no active spans on
 * the stack, and must not be active. If successful, the call stack
 * may not be used afterwards. The active call stack of the thread
 * is destroyed automatically, on thread exit or during destruction
 * of `context`. The caller must own the call stack uniquely.
 *
 * @param context the context
 * @param call_stack the call stack to destroy
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_destroy(void *context, FimoTracingCallStack call_stack);

/**
 * Switches the call stack of the current thread.
 *
 * If successful, `new_call_stack` will be used as the active call
 * stack of the calling thread. The old call stack is written into
 * `old_call_stack`, enabling the caller to switch back to it afterwards.
 * `new_call_stack` must be in a suspended, but unblocked, state and not be
 * active. The active call stack must also be in a suspended state, but may
 * also be blocked.
 *
 * @param context the context
 * @param new_call_stack new call stack
 * @param old_call_stack location to store the old call stack into
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_switch(void *context, FimoTracingCallStack call_stack,
                                                  FimoTracingCallStack *old_call_stack);

/**
 * Unblocks a blocked call stack.
 *
 * Once unblocked, the call stack may be resumed. The call stack
 * may not be active and must be marked as blocked.
 *
 * @param context the context
 * @param call_stack the call stack to unblock
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_unblock(void *context, FimoTracingCallStack call_stack);

/**
 * Marks the current call stack as being suspended.
 *
 * While suspended, the call stack can not be utilized for tracing
 * messages. The call stack optionally also be marked as being
 * blocked. In that case, the call stack must be unblocked prior
 * to resumption.
 *
 * @param context the context
 * @param block whether to mark the call stack as blocked
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_suspend_current(void *context, bool block);

/**
 * Marks the current call stack as being resumed.
 *
 * Once resumed, the context can be used to trace messages. To be
 * successful, the current call stack must be suspended and unblocked.
 *
 * @param context the context.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_resume_current(void *context);

/**
 * Creates a new span with the standard formatter and enters it.
 *
 * If successful, the newly created span is used as the context for
 * succeeding events. The message is formatted as if it were
 * formatted by a call to `snprintf`. The message may be cut of,
 * if the length exceeds the internal formatting buffer size.  The
 * contents of `span_desc` must remain valid until the span is destroyed.
 *
 * @param context the context
 * @param span_desc descriptor of the new span
 * @param span pointer to the resulting span
 * @param format formatting string
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_span_create_fmt(void *context, const FimoTracingSpanDesc *span_desc,
                                                FimoTracingSpan *span, FIMO_PRINT_F_FORMAT const char *format, ...)
        FIMO_PRINT_F_FORMAT_ATTR(4, 5);

/**
 * Creates a new span with a custom formatter and enters it.
 *
 * If successful, the newly created span is used as the context for
 * succeeding events. The backend may use a formatting buffer of a
 * fixed size. The formatter is expected to cut-of the message after
 * reaching that specified size. The contents of `span_desc` must
 * remain valid until the span is destroyed.
 *
 * @param context the context
 * @param span_desc descriptor of the new span
 * @param span pointer to the resulting span
 * @param format custom formatting function
 * @param data custom formatting data
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_span_create_custom(void *context, const FimoTracingSpanDesc *span_desc,
                                                   FimoTracingSpan *span, FimoTracingFormat format, const void *data);

/**
 * Exits and destroys a span.
 *
 * If successful, succeeding events won't occur inside the context of the
 * exited span anymore. `span` must be the span at the top of the current
 * call stack. The span may not be in use prior to a call to this function,
 * and may not be used afterwards.
 *
 * @param context the context
 * @param span the span to destroy
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_span_destroy(void *context, FimoTracingSpan *span);

/**
 * Emits a new event with the standard formatter.
 *
 * The message is formatted as if it were formatted by a call to `snprintf`.
 * The message may be cut of, if the length exceeds the internal formatting
 * buffer size.
 *
 * @param context the context
 * @param event the event to emit
 * @param format formatting string
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_event_emit_fmt(void *context, const FimoTracingEvent *event,
                                               FIMO_PRINT_F_FORMAT const char *format, ...)
        FIMO_PRINT_F_FORMAT_ATTR(3, 4);

/**
 * Emits a new event with a custom formatter.
 *
 * The backend may use a formatting buffer of a fixed size. The formatter is
 * expected to cut-of the message after reaching that specified size.
 *
 * @param context the context
 * @param event the event to emit
 * @param format custom formatting function
 * @param data custom data to format
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_event_emit_custom(void *context, const FimoTracingEvent *event,
                                                  FimoTracingFormat format, const void *data);

/**
 * Checks whether the tracing backend is enabled.
 *
 * This function can be used to check whether to call into the backend at all.
 * Calling this function is not necessary, as the remaining functions of the
 * backend are guaranteed to return default values, in case the backend is
 * disabled.
 *
 * @param context the context.
 *
 * @return `true` if the backend is enabled.
 */
FIMO_MUST_USE
bool fimo_internal_tracing_is_enabled(void *context);

/**
 * Registers the calling thread with the tracing backend.
 *
 * The tracing of the backend is opt-in on a per thread basis, where
 * unregistered threads will behave as if the backend was disabled.
 * Once registered, the calling thread gains access to the tracing
 * backend and is assigned a new empty call stack. A registered
 * thread must be unregistered from the tracing backend before the
 * context is destroyed, by terminating the tread, or by manually
 * calling `fimo_internal_tracing_unregister_thread()`.
 *
 * @param context the context
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_register_thread(void *context);

/**
 * Unregisters the calling thread from the tracing backend.
 *
 * Once unregistered, the calling thread looses access to the tracing
 * backend until it is registered again. The thread can not be unregistered
 * until the call stack is empty.
 *
 * @param context the context.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_unregister_thread(void *context);

/**
 * Flushes the streams used for tracing.
 *
 * If successful, any unwritten data is written out by the individual subscribers.
 *
 * @param context the context
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_tracing_flush(void *context);

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
FimoError fimo_internal_tracing_fmt(char *buffer, FimoUSize buffer_size, const void *args, FimoUSize *written_size);

#ifdef __cplusplus
}
#endif

#endif // FIMO_INTERNAL_TRACING_H
