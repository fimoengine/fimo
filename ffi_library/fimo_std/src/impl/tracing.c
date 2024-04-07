#include <fimo_std/impl/tracing.h>

#include <fimo_std/error.h>
#include <fimo_std/memory.h>
#include <fimo_std/tracing.h>

#include <stdio.h>

FimoError fimo_impl_tracing_fmt(char *buffer, FimoUSize buffer_size, const void *args, FimoUSize *written_size) {
    if (buffer == NULL || args == NULL || written_size == NULL) {
        return FIMO_EINVAL;
    }
    FIMO_PRAGMA_MSVC(warning(push))
    FIMO_PRAGMA_MSVC(warning(disable : 4996))
    FimoImplTracingFmtArgs *tracing_args = (FimoImplTracingFmtArgs *)args;
    int written = vsnprintf(buffer, buffer_size, tracing_args->format, *tracing_args->vlist);
    *written_size = (FimoUSize)written;
    FIMO_PRAGMA_MSVC(warning(pop))
    return FIMO_EOK;
}

///////////////////////////////////////////////////////////////////////
//// Default Subscriber
///////////////////////////////////////////////////////////////////////

#define ANSI_COLOR_RED "\x1b[31m"
#define ANSI_COLOR_GREEN "\x1b[32m"
#define ANSI_COLOR_YELLOW "\x1b[33m"
#define ANSI_COLOR_BLUE "\x1b[34m"
#define ANSI_COLOR_MAGENTA "\x1b[35m"
#define ANSI_COLOR_CYAN "\x1b[36m"
#define ANSI_COLOR_RESET "\x1b[0m"

#define ANSI_SGR_ITALIC "\033[3m"
#define ANSI_SGR_RESET "\033[0m"

#define EVENT_MESSAGE "%s: %.*s"
#define AT_FILE_PATH_STR "\t" ANSI_SGR_ITALIC "at" ANSI_SGR_RESET " %s:%d\n"
#define AT_UNKNOWN_FILE_PATH_STR "\t" ANSI_SGR_ITALIC "at" ANSI_SGR_RESET " unknown\n"
#define BACKTRACE_STR "\t" ANSI_SGR_ITALIC "in" ANSI_SGR_RESET " %s" ANSI_SGR_ITALIC " with" ANSI_SGR_RESET " %.*s\n"

#define ERROR_STR ANSI_COLOR_RED "ERROR " EVENT_MESSAGE ANSI_COLOR_RESET "\n"
#define WARN_STR ANSI_COLOR_YELLOW "WARN " EVENT_MESSAGE ANSI_COLOR_RESET "\n"
#define INFO_STR ANSI_COLOR_GREEN "INFO " EVENT_MESSAGE ANSI_COLOR_RESET "\n"
#define DEBUG_STR ANSI_COLOR_BLUE "DEBUG " EVENT_MESSAGE ANSI_COLOR_RESET "\n"
#define TRACE_STR ANSI_COLOR_MAGENTA "TRACE " EVENT_MESSAGE ANSI_COLOR_RESET "\n"

#define PRINT_BUFFER_LEN 1024
static _Thread_local char PRINT_BUFFER[PRINT_BUFFER_LEN + 1] = {0};

struct Span_ {
    struct Span_ *next;
    struct Span_ *previous;
    const FimoTracingSpanDesc *span_desc;
    const char *message;
    FimoUSize message_len;
};

struct CallStack_ {
    struct Span_ *tail;
};

FimoError fimo_impl_tracing_default_subscriber_call_stack_create(void *subscriber, const FimoTime *time, void **stack) {
    (void)PRINT_BUFFER;
    (void)subscriber;
    (void)time;

    struct CallStack_ **stack_ = (struct CallStack_ **)stack;

    FimoError error;
    *stack_ = fimo_malloc(sizeof(struct CallStack_), &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }
    **stack_ = (struct CallStack_){
            .tail = NULL,
    };

    return FIMO_EOK;
}

void fimo_impl_tracing_default_subscriber_call_stack_drop(void *subscriber, void *stack) {
    (void)subscriber;
    struct CallStack_ *stack_ = stack;
    FIMO_DEBUG_ASSERT(stack_ && stack_->tail == NULL);
    fimo_free(stack_);
}

void fimo_impl_tracing_default_subscriber_call_stack_destroy(void *subscriber, const FimoTime *time, void *stack) {
    (void)subscriber;
    (void)time;
    struct CallStack_ *stack_ = stack;
    FIMO_DEBUG_ASSERT(stack_ && stack_->tail == NULL);
    fimo_free(stack_);
}

void fimo_impl_tracing_default_subscriber_call_stack_unblock(void *subscriber, const FimoTime *time, void *stack) {
    (void)subscriber;
    (void)time;
    FIMO_DEBUG_ASSERT(stack);
}

void fimo_impl_tracing_default_subscriber_call_stack_suspend(void *subscriber, const FimoTime *time, void *stack,
                                                             bool block) {
    (void)subscriber;
    (void)time;
    (void)block;
    FIMO_DEBUG_ASSERT(stack);
}

void fimo_impl_tracing_default_subscriber_call_stack_resume(void *subscriber, const FimoTime *time, void *stack) {
    (void)subscriber;
    (void)time;
    FIMO_DEBUG_ASSERT(stack);
}

FimoError fimo_impl_tracing_default_subscriber_span_push(void *subscriber, const FimoTime *time,
                                                         const FimoTracingSpanDesc *span_desc, const char *message,
                                                         const FimoUSize message_len, void *stack) {
    (void)subscriber;
    (void)time;

    struct CallStack_ *stack_ = stack;
    FIMO_DEBUG_ASSERT(stack_);

    FimoError error;
    struct Span_ *span = fimo_malloc(sizeof(struct Span_), &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }
    *span = (struct Span_){
            .next = NULL,
            .previous = stack_->tail,
            .span_desc = span_desc,
            .message = message,
            .message_len = message_len,
    };
    stack_->tail = span;

    return FIMO_EOK;
}

void fimo_impl_tracing_default_subscriber_span_drop(void *subscriber, void *stack) {
    (void)subscriber;
    struct CallStack_ *stack_ = stack;
    FIMO_DEBUG_ASSERT(stack_ && stack_->tail);
    struct Span_ *new_tail = stack_->tail->previous;
    fimo_free(stack_->tail);
    stack_->tail = new_tail;
}

void fimo_impl_tracing_default_subscriber_span_pop(void *subscriber, const FimoTime *time, void *stack) {
    (void)subscriber;
    (void)time;
    struct CallStack_ *stack_ = stack;
    FIMO_DEBUG_ASSERT(stack_ && stack_->tail);
    struct Span_ *new_tail = stack_->tail->previous;
    fimo_free(stack_->tail);
    stack_->tail = new_tail;
}

void fimo_impl_tracing_default_subscriber_event_emit(void *subscriber, const FimoTime *time, void *stack,
                                                     const FimoTracingEvent *event, const char *message,
                                                     FimoUSize message_len) {
    (void)subscriber;
    (void)time;
    struct CallStack_ *stack_ = stack;
    FIMO_DEBUG_ASSERT(stack_);

    FIMO_PRAGMA_MSVC(warning(push))
    FIMO_PRAGMA_MSVC(warning(disable : 4996))

    bool is_error = false;
    FimoUSize written_bytes = 0;
    switch (event->metadata->level) {
        case FIMO_TRACING_LEVEL_OFF:
            break;
        case FIMO_TRACING_LEVEL_ERROR:
            is_error = true;
            written_bytes += snprintf(PRINT_BUFFER, PRINT_BUFFER_LEN - written_bytes, ERROR_STR, event->metadata->name,
                                      (int)message_len, message);
            break;
        case FIMO_TRACING_LEVEL_WARN:
            written_bytes += snprintf(PRINT_BUFFER, PRINT_BUFFER_LEN - written_bytes, WARN_STR, event->metadata->name,
                                      (int)message_len, message);
            break;
        case FIMO_TRACING_LEVEL_INFO:
            written_bytes += snprintf(PRINT_BUFFER, PRINT_BUFFER_LEN - written_bytes, INFO_STR, event->metadata->name,
                                      (int)message_len, message);
            break;
        case FIMO_TRACING_LEVEL_DEBUG:
            written_bytes += snprintf(PRINT_BUFFER, PRINT_BUFFER_LEN - written_bytes, DEBUG_STR, event->metadata->name,
                                      (int)message_len, message);
            break;
        case FIMO_TRACING_LEVEL_TRACE:
            written_bytes += snprintf(PRINT_BUFFER, PRINT_BUFFER_LEN - written_bytes, TRACE_STR, event->metadata->name,
                                      (int)message_len, message);
            break;
    }

    if (event->metadata->file_name != NULL) {
        written_bytes += snprintf(PRINT_BUFFER + written_bytes, PRINT_BUFFER_LEN - written_bytes, AT_FILE_PATH_STR,
                                  event->metadata->file_name, event->metadata->line_number);
    }
    else {
        written_bytes +=
                snprintf(PRINT_BUFFER + written_bytes, PRINT_BUFFER_LEN - written_bytes, AT_UNKNOWN_FILE_PATH_STR);
    }

    for (const struct Span_ *current = stack_->tail; current != NULL && written_bytes < PRINT_BUFFER_LEN;
         current = current->previous) {
        written_bytes += snprintf(PRINT_BUFFER + written_bytes, PRINT_BUFFER_LEN - written_bytes, BACKTRACE_STR,
                                  current->span_desc->metadata->name, (int)current->message_len, current->message);
    }

    FIMO_PRAGMA_MSVC(warning(pop))

    if (is_error) {
        fflush(stdout);
        fputs(PRINT_BUFFER, stderr);
    }
    else {
        fputs(PRINT_BUFFER, stdout);
    }
}

void fimo_impl_tracing_default_subscriber_flush(void *subscriber) {
    (void)subscriber;
    fflush(stdout);
}
