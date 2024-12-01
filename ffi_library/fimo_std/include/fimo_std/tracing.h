#ifndef FIMO_TRACING_H
#define FIMO_TRACING_H

#include <stddef.h>
#include <stdarg.h>

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/time.h>
#include <fimo_std/utils.h>

#include <fimo_std/impl/tracing.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * A call stack.
 *
 * Each call stack represents a unit of computation, like a thread.
 * A call stack is active on only one thread at any given time. The
 * active call stack of a thread can be swapped, which is useful
 * for tracing where a `M:N` threading model is used. In that case,
 * one would create one stack for each task, and activate it when
 * the task is resumed.
 */
typedef struct FimoTracingCallStack FimoTracingCallStack;

/**
 * Possible tracing levels.
 *
 * The levels are ordered such that given two levels
 * `lvl1` and `lvl2`, where `lvl1 >= lvl2`, then an event
 * with level `lvl2` will be traced in a context where the
 * maximum tracing level is `lvl1`.
 */
typedef enum FimoTracingLevel {
    FIMO_TRACING_LEVEL_OFF = 0,
    FIMO_TRACING_LEVEL_ERROR = 1,
    FIMO_TRACING_LEVEL_WARN = 2,
    FIMO_TRACING_LEVEL_INFO = 3,
    FIMO_TRACING_LEVEL_DEBUG = 4,
    FIMO_TRACING_LEVEL_TRACE = 5,
} FimoTracingLevel;

/**
 * Metadata for a span/event.
 */
typedef struct FimoTracingMetadata {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_TRACING_METADATA`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const FimoBaseStructIn *next;
    /**
     * Name of the event.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Target of the event.
     *
     * Must not be `NULL`.
     */
    const char *target;
    /**
     * Level at which to trace the event.
     */
    FimoTracingLevel level;
    /**
     * Optional file name where the event took place.
     */
    const char *file_name;
    /**
     * Optional line number where the event took place.
     *
     * Use a negative number to indicate no line number.
     */
    FimoI32 line_number;
} FimoTracingMetadata;

/**
 * Descriptor of a new span.
 */
typedef struct FimoTracingSpanDesc {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_TRACING_SPAN_DESC`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const FimoBaseStructIn *next;
    /**
     * Metadata of the span.
     *
     * Must not be `NULL`.
     */
    const FimoTracingMetadata *metadata;
} FimoTracingSpanDesc;

/**
 * A period of time, during which events can occur.
 */
typedef struct FimoTracingSpan {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_TRACING_SPAN`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use.
     */
    FimoBaseStructOut *next;
} FimoTracingSpan;

/**
 * An event to be traced.
 */
typedef struct FimoTracingEvent {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_TRACING_EVENT`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const FimoBaseStructIn *next;
    /**
     * Metadata of the event.
     *
     * Must not be `NULL`.
     */
    const FimoTracingMetadata *metadata;
} FimoTracingEvent;

/**
 * Signature of a message formatter.
 *
 * It is not an error to format only a part of the message.
 *
 * @param arg0 destination buffer
 * @param arg1 destination buffer size
 * @param arg2 data to format
 * @param arg3 number of written bytes of the formatter
 * @return Status code.
 */
typedef FimoResult (*FimoTracingFormat)(char *, FimoUSize, const void *, FimoUSize *);

/**
 * VTable of a tracing subscriber.
 *
 * Adding/removing functionality to a subscriber through this table
 * is a breaking change, as a subscriber may be implemented from
 * outside the library.
 */
typedef struct FimoTracingSubscriberVTable {
    /**
     * Destroys the subscriber.
     *
     * @param arg0 pointer to the subscriber
     */
    void (*destroy)(void *);
    /**
     * Creates a new stack.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 time of the event
     * @param arg2 pointer to the new stack
     * @return Status code.
     */
    FimoResult (*call_stack_create)(void *, const FimoTime *, void **);
    /**
     * Drops an empty call stack.
     *
     * Calling this function reverts the creation of the call stack.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 the stack
     */
    void (*call_stack_drop)(void *, void *);
    /**
     * Destroys a stack.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 time of the event
     * @param arg2 the stack
     */
    void (*call_stack_destroy)(void *, const FimoTime *, void *);
    /**
     * Marks the stack as unblocked.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 time of the event
     * @param arg2 the stack
     */
    void (*call_stack_unblock)(void *, const FimoTime *, void *);
    /**
     * Marks the stack as suspended/blocked.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 time of the event
     * @param arg2 the stack
     * @param arg3 whether to block the stack
     */
    void (*call_stack_suspend)(void *, const FimoTime *, void *, bool);
    /**
     * Marks the stack as resumed.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 time of the event
     * @param arg2 the stack
     */
    void (*call_stack_resume)(void *, const FimoTime *, void *);
    /**
     * Creates a new span.
     *
     * @param arg0 pointer to the subscriber
     * @param arg2 descriptor of the span
     * @param arg3 formatted span message
     * @param arg4 length of the span message
     * @param arg5 the call stack
     * @return Status code.
     */
    FimoResult (*span_push)(void *, const FimoTime *, const FimoTracingSpanDesc *, const char *, FimoUSize, void *);
    /**
     * Drops a newly created span.
     *
     * Calling this function reverts the creation of the span.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 the call stack
     */
    void (*span_drop)(void *, void *);
    /**
     * Exits and destroys a span.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 time of the event
     * @param arg2 the call stack
     */
    void (*span_pop)(void *, const FimoTime *, void *);
    /**
     * Emits an event.
     *
     * @param arg0 pointer to the subscriber
     * @param arg1 time of the event
     * @param arg2 the call stack
     * @param arg3 the event to emit
     * @param arg4 formatted message of the event
     * @param arg5 length of the event message
     */
    void (*event_emit)(void *, const FimoTime *, void *, const FimoTracingEvent *, const char *, FimoUSize);
    /**
     * Flushes the messages of the subscriber.
     *
     * @param arg0 pointer to the subscriber
     */
    void (*flush)(void *);
} FimoTracingSubscriberVTable;

/**
 * A subscriber for tracing events.
 *
 * The main function of the tracing subsystem is managing and routing
 * tracing events to subscribers. Therefore it does not consume any
 * events on its own, which is the task of the subscribers. Subscribers
 * may utilize the events in any way they deem fit.
 */
typedef struct FimoTracingSubscriber {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_TRACING_SUBSCRIBER`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const struct FimoBaseStructIn *next;
    /**
     * Pointer to the subscriber.
     */
    void *ptr;
    /**
     * Pointer to the vtable of the subscriber (not `Null`).
     */
    const FimoTracingSubscriberVTable *vtable;
} FimoTracingSubscriber;

/**
 * Default subscriber.
 */
FIMO_EXPORT
extern const FimoTracingSubscriber FIMO_TRACING_DEFAULT_SUBSCRIBER;

/**
 * Configuration for the tracing subsystem.
 *
 * Can be passed when creating the context.
 */
typedef struct FimoTracingCreationConfig {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const struct FimoBaseStructIn *next;
    /**
     * Size of the per-call-stack buffer used for formatting messages.
     */
    FimoUSize format_buffer_size;
    /**
     * Maximum level for which to consume tracing events.
     */
    FimoTracingLevel maximum_level;
    /**
     * Array of subscribers to register with the tracing subsystem.
     *
     * Must be `NULL` when there are no subscribers. The ownership
     * of the subscribers is transferred to the context.
     */
    FimoTracingSubscriber *subscribers;
    /**
     * Number of subscribers to register with the tracing subsystem.
     *
     * Must match the number of subscribers present in `subscribers`.
     */
    FimoUSize subscriber_count;
} FimoTracingCreationConfig;

/**
 * VTable of the tracing subsystem.
 *
 * Changing the VTable is a breaking change.
 */
typedef struct FimoTracingVTableV0 {
    /**
     * Creates a new empty call stack.
     *
     * If successful, the new call stack is marked as suspended, and written
     * into `call_stack`. The new call stack is not set to be the active call
     * stack.
     *
     * @param ctx the context
     * @param call_stack pointer to the resulting call stack
     *
     * @return Status code.
     */
    FimoResult (*call_stack_create)(void *ctx, FimoTracingCallStack **call_stack);
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
     * @param ctx the context
     * @param call_stack the call stack to destroy
     *
     * @return Status code.
     */
    FimoResult (*call_stack_destroy)(void *ctx, FimoTracingCallStack *call_stack);
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
     * This function may return an error, if the current thread is not
     * registered with the subsystem.
     *
     * @param ctx the context
     * @param call_stack new call stack
     * @param old location to store the old call stack into
     *
     * @return Status code.
     */
    FimoResult (*call_stack_switch)(void *ctx, FimoTracingCallStack *call_stack, FimoTracingCallStack **old);
    /**
     * Unblocks a blocked call stack.
     *
     * Once unblocked, the call stack may be resumed. The call stack
     * may not be active and must be marked as blocked.
     *
     * @param ctx the context
     * @param call_stack the call stack to unblock
     *
     * @return Status code.
     */
    FimoResult (*call_stack_unblock)(void *ctx, FimoTracingCallStack *call_stack);
    /**
     * Marks the current call stack as being suspended.
     *
     * While suspended, the call stack can not be utilized for tracing
     * messages. The call stack optionally also be marked as being
     * blocked. In that case, the call stack must be unblocked prior
     * to resumption.
     *
     * This function may return an error, if the current thread is not
     * registered with the subsystem.
     *
     * @param ctx the context
     * @param block whether to mark the call stack as blocked
     *
     * @return Status code.
     */
    FimoResult (*call_stack_suspend_current)(void *ctx, bool block);
    /**
     * Marks the current call stack as being resumed.
     *
     * Once resumed, the context can be used to trace messages. To be
     * successful, the current call stack must be suspended and unblocked.
     *
     * This function may return an error, if the current thread is not
     * registered with the subsystem.
     *
     * @param ctx the context.
     *
     * @return Status code.
     */
    FimoResult (*call_stack_resume_current)(void *ctx);
    /**
     * Creates a new span with a custom formatter and enters it.
     *
     * If successful, the newly created span is used as the context for
     * succeeding events. The subsystem may use a formatting buffer of a
     * fixed size. The formatter is expected to cut-of the message after
     * reaching that specified size. The contents of `span_desc` must
     * remain valid until the span is destroyed.
     *
     * This function may return an error, if the current thread is not
     * registered with the subsystem.
     *
     * @param ctx the context
     * @param span_desc descriptor of the new span
     * @param span pointer to the resulting span
     * @param format custom formatting function
     * @param data custom formatting data
     *
     * @return Status code.
     */
    FimoResult (*span_create)(void *ctx, const FimoTracingSpanDesc *span_desc,
                              FimoTracingSpan **span, FimoTracingFormat format, const void *data);
    /**
     * Exits and destroys a span.
     *
     * If successful, succeeding events won't occur inside the context of the
     * exited span anymore. `span` must be the span at the top of the current
     * call stack. The span may not be in use prior to a call to this function,
     * and may not be used afterwards.
     *
     * This function may return an error, if the current thread is not
     * registered with the subsystem.
     *
     * @param ctx the context
     * @param span the span to destroy
     *
     * @return Status code.
     */
    FimoResult (*span_destroy)(void *ctx, FimoTracingSpan *span);
    /**
     * Emits a new event with a custom formatter.
     *
     * The subsystem may use a formatting buffer of a fixed size. The formatter is
     * expected to cut-of the message after reaching that specified size.
     *
     * @param ctx the context
     * @param event the event to emit
     * @param format custom formatting function
     * @param data custom data to format
     *
     * @return Status code.
     */
    FimoResult (*event_emit)(void *ctx, const FimoTracingEvent *event, FimoTracingFormat format,
                             const void * data);
    /**
     * Checks whether the tracing subsystem is enabled.
     *
     * This function can be used to check whether to call into the subsystem at all.
     * Calling this function is not necessary, as the remaining functions of the
     * subsystem are guaranteed to return default values, in case the subsystem is
     * disabled.
     *
     * @param ctx the context.
     *
     * @return `true` if the subsystem is enabled.
     */
    bool (*is_enabled)(void *ctx);
    /**
     * Registers the calling thread with the tracing subsystem.
     *
     * The tracing of the subsystem is opt-in on a per thread basis, where
     * unregistered threads will behave as if the subsystem was disabled.
     * Once registered, the calling thread gains access to the tracing
     * subsystem and is assigned a new empty call stack. A registered
     * thread must be unregistered from the tracing subsystem before the
     * context is destroyed, by terminating the tread, or by manually
     * calling `fimo_tracing_unregister_thread()`.
     *
     * @param ctx the context
     *
     * @return Status code.
     */
    FimoResult (*register_thread)(void *ctx);
    /**
     * Unregisters the calling thread from the tracing subsystem.
     *
     * Once unregistered, the calling thread looses access to the tracing
     * subsystem until it is registered again. The thread can not be unregistered
     * until the call stack is empty.
     *
     * @param ctx the context.
     *
     * @return Status code.
     */
    FimoResult (*unregister_thread)(void *ctx);
    /**
     * Flushes the streams used for tracing.
     *
     * If successful, any unwritten data is written out by the individual subscribers.
     *
     * @param ctx the context
     *
     * @return Status code.
     */
    FimoResult (*flush)(void *ctx);
} FimoTracingVTableV0;

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_TRACING_H
