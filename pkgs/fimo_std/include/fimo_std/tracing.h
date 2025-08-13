#ifndef FIMO_TRACING_H
#define FIMO_TRACING_H

#include <stdarg.h>
#include <stddef.h>

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/time.h>
#include <fimo_std/utils.h>

#include <fimo_std/impl/tracing.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/// Tracing levels.
///
/// The levels are ordered such that given two levels `lvl1` and `lvl2`, where `lvl1 >= lvl2`, then
/// an event with level `lvl2` will be traced in a context where the maximum tracing level is
/// `lvl1`.
typedef enum FimoTracingLevel : FimoI32 {
    FIMO_TRACING_LEVEL_OFF = 0,
    FIMO_TRACING_LEVEL_ERROR = 1,
    FIMO_TRACING_LEVEL_WARN = 2,
    FIMO_TRACING_LEVEL_INFO = 3,
    FIMO_TRACING_LEVEL_DEBUG = 4,
    FIMO_TRACING_LEVEL_TRACE = 5,
} FimoTracingLevel;

/// Basic information regarding a tracing event.
///
/// The subsystem expects instances of this struct to have a static lifetime.
typedef struct FimoTracingEventInfo {
    /// Name of the event.
    ///
    /// Must not be `NULL`.
    const char *name;
    /// Target of the event.
    ///
    /// Must not be `NULL`.
    const char *target;
    /// Scope of the event.
    ///
    /// Must not be `NULL`.
    const char *scope;
    /// Optional file name where the event took place.
    const char *file_name;
    /// Optional line number where the event took place.
    ///
    /// Use a negative number to indicate no line number.
    FimoI32 line_number;
    /// Level at which to trace the event.
    FimoTracingLevel level;
} FimoTracingEventInfo;

/// A call stack.
///
/// Each call stack represents a unit of computation, like a thread. A call stack is active on only
/// one thread at any given time. The active call stack of a thread can be swapped, which is useful
/// for tracing where a `M:N` threading model is used. In that case, one would create one stack for
/// each task, and activate it when the task is resumed.
typedef struct FimoTracingCallStack FimoTracingCallStack;

/// Type of a formatter function.
///
/// The formatter function is allowed to format only part of the message, if it would not fit into
/// the buffer.
typedef FimoUSize (*FimoTracingFormat)(char *buffer, FimoUSize buffer_len, const void *data);

/// Common header of all events.
typedef enum FimoTracingEvent : FimoU32 {
    FIMO_TRACING_EVENT_START,
    FIMO_TRACING_EVENT_FINISH,
    FIMO_TRACING_EVENT_REGISTER_THREAD,
    FIMO_TRACING_EVENT_UNREGISTER_THREAD,
    FIMO_TRACING_EVENT_CREATE_CALL_STACK,
    FIMO_TRACING_EVENT_DESTROY_CALL_STACK,
    FIMO_TRACING_EVENT_UNBLOCK_CALL_STACK,
    FIMO_TRACING_EVENT_SUSPEND_CALL_STACK,
    FIMO_TRACING_EVENT_RESUME_CALL_STACK,
    FIMO_TRACING_EVENT_ENTER_SPAN,
    FIMO_TRACING_EVENT_EXIT_SPAN,
    FIMO_TRACING_EVENT_LOG_MESSAGE,
} FimoTracingEvent;

typedef enum FimoTracingCpuArch : FimoU8 {
    FIMO_TRACING_CPU_ARCH_UNKNOWN,
    FIMO_TRACING_CPU_ARCH_X86_64,
    FIMO_TRACING_CPU_ARCH_AARCH64,
} FimoTracingCpuArch;

typedef struct FimoTracingEventStart {
    FimoTracingEvent event;
    FimoInstant time;
    FimoTime epoch;
    FimoDuration resolution;
    FimoUSize available_memory;
    FimoUSize process_id;
    FimoUSize num_cores;
    FimoTracingCpuArch cpu_arch;
    FimoU8 cpu_id;
    const char *cpu_vendor;
    FimoUSize cpu_vendor_length;
    const char *app_name;
    FimoUSize app_name_length;
    const char *host_info;
    FimoUSize host_info_length;
} FimoTracingEventStart;

typedef struct FimoTracingEventFinish {
    FimoTracingEvent event;
    FimoInstant time;
} FimoTracingEventFinish;

typedef struct FimoTracingEventRegisterThread {
    FimoTracingEvent event;
    FimoInstant time;
    FimoUSize thread_id;
} FimoTracingEventRegisterThread;

typedef struct FimoTracingEventUnregisterThread {
    FimoTracingEvent event;
    FimoInstant time;
    FimoUSize thread_id;
} FimoTracingEventUnregisterThread;

typedef struct FimoTracingEventCreateCallStack {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
} FimoTracingEventCreateCallStack;

typedef struct FimoTracingEventDestroyCallStack {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
} FimoTracingEventDestroyCallStack;

typedef struct FimoTracingEventUnblockCallStack {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
} FimoTracingEventUnblockCallStack;

typedef struct FimoTracingEventSuspendCallStack {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
    bool mark_blocked;
} FimoTracingEventSuspendCallStack;

typedef struct FimoTracingEventResumeCallStack {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
    FimoUSize thread_id;
} FimoTracingEventResumeCallStack;

typedef struct FimoTracingEventEnterSpan {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
    const FimoTracingEventInfo *span;
    const char *message;
    FimoUSize message_length;
} FimoTracingEventEnterSpan;

typedef struct FimoTracingEventExitSpan {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
    bool is_unwinding;
} FimoTracingEventExitSpan;

typedef struct FimoTracingEventLogMessage {
    FimoTracingEvent event;
    FimoInstant time;
    void *stack;
    const FimoTracingEventInfo *info;
    const char *message;
    FimoUSize message_length;
} FimoTracingEventLogMessage;

/// A subscriber for tracing events.
///
/// The main function of the tracing subsystem is managing and routing tracing events to
/// subscribers. Therefore it does not consume any events on its own, which is the task of the
/// subscribers. Subscribers may utilize the events in any way they deem fit.
typedef struct FimoTracingSubscriber {
    /// Pointer to the subscriber (not `Null`).
    void *ptr;
    /// Event handler of the subscriber (not `Null`).
    void (*on_event)(void *data, const FimoTracingEvent *event);
} FimoTracingSubscriber;

/// Creates a new subscriber, which logs the messages to the stderr file.
FimoTracingSubscriber fimo_tracing_stderr_logger_new(void);

/// Destroys the priorly created subscriber.
void fimo_tracing_stderr_logger_destroy(FimoTracingSubscriber subscriber);

/// Configuration for the tracing subsystem.
typedef struct FimoTracingConfig {
    /// Type of the struct.
    ///
    /// Must be `FIMO_CONFIG_ID_TRACING`.
    FimoConfigId id;
    /// Length in bytes of the per-call-stack buffer used when formatting mesasges.
    FimoUSize format_buffer_size;
    /// Maximum level for which to consume tracing events.
    FimoTracingLevel maximum_level;
    /// Array of subscribers to register with the tracing subsystem.
    FimoTracingSubscriber *subscribers;
    /// Number of subscribers to register with the tracing subsystem.
    FimoUSize subscriber_count;
    /// Register the calling thread.
    bool register_thread;
    /// Name of the application (Not null).
    const char *app_name;
    /// Length in bytes of the application name.
    FimoUSize app_name_length;
} FimoTracingCreationConfig;

/// VTable of the tracing subsystem.
///
/// Changing this definition is a breaking change.
typedef struct FimoTracingVTable {
    /// Checks whether the tracing subsystem is enabled.
    ///
    /// This function can be used to check whether to call into the subsystem at all. Calling this
    /// function is not necessary, as the remaining functions of the subsystem are guaranteed to return
    /// default values, in case the subsystem is disabled.
    bool (*is_enabled)();
    /// Registers the calling thread with the tracing subsystem.
    ///
    /// The instrumentation is opt-in on a per thread basis, where unregistered threads will
    /// behave as if the subsystem was disabled. Once registered, the calling thread gains access to
    /// the tracing subsystem and is assigned a new empty call stack. A registered thread must be
    /// unregistered from the tracing subsystem before the context is destroyed, by terminating the
    /// tread, or by manually unregistering it. A registered thread may not try to register itself.
    void (*register_thread)();
    /// Unregisters the calling thread from the tracing subsystem.
    ///
    /// Once unregistered, the calling thread looses access to the tracing subsystem until it is
    /// registered again. The thread can not be unregistered until the call stack is empty.
    void (*unregister_thread)();
    /// Creates a new empty call stack.
    ///
    /// The call stack is marked as suspended.
    FimoTracingCallStack *(*create_call_stack)();
    /// Destroys a call stack.
    ///
    /// If `do_abort` is `false`, it marks the completion of a task. Before calling this function,
    /// the call stack must be empty, i.e., there must be no active spans on the stack.
    ///
    /// If `do_abort` is `true`, it marks that the task was aborted.
    ///
    /// Before calling this function,the call stack must not be active, and it may not be used
    /// afterwards. The active call stack of the thread is destroyed automatically, on thread exit
    /// or during destruction of the context.
    void (*destroy_call_stack)(FimoTracingCallStack *stack, bool do_abort);
    /// Switches the call stack of the current thread.
    ///
    /// This call stack will be used as the active call stack of the calling thread. The old call
    /// stack is returned, enabling the caller to switch back to it afterwards. This call stack
    /// must be in a suspended, but unblocked, state and not be active. The active call stack must
    /// also be in a suspended state, but may also be blocked.
    FimoTracingCallStack *(*swap_call_stack)(FimoTracingCallStack *stack);
    /// Unblocks the blocked call stack.
    ///
    /// Once unblocked, the call stack may be resumed. The call stack may not be active and must be
    /// marked as blocked.
    void (*unblock_call_stack)(FimoTracingCallStack *stack);
    /// Marks the current call stack as being suspended.
    ///
    /// While suspended, the call stack can not be utilized for tracing messages. The call stack
    /// optionally also be marked as being blocked. In that case, the call stack must be unblocked
    /// prior to resumption.
    void (*suspend_current_call_stack)(bool mark_blocked);
    /// Marks the current call stack as being resumed.
    ///
    /// Once resumed, the context can be used to trace messages. To be successful, the current call
    /// stack must be suspended and unblocked.
    void (*resume_current_call_stack)();
    /// Enters the span.
    ///
    /// Once entered, the span is used as the context for succeeding events. Each `enter` operation
    /// must be accompanied with a `exit` operation in reverse entering order. A span may be entered
    /// multiple times. The formatting function may be used to assign a name to the entered span.
    void (*enter_span)(const FimoTracingEventInfo *id, FimoTracingFormat fmt, const void *fmt_data);
    /// Exits an entered span.
    ///
    /// The events won't occur inside the context of the exited span anymore. The span must be the
    /// span at the top of the current call stack.
    void (*exit_span)(const FimoTracingEventInfo *id);
    /// Logs a message with a custom format function.
    void (*log_message)(const FimoTracingEventInfo *info, FimoTracingFormat fmt, const void *fmt_data);
} FimoTracingVTable;

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_TRACING_H
