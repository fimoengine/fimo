#include <fimo_std/internal/tracing.h>

#include <stdalign.h>
#include <stdio.h>
#include <stdlib.h>

#include <fimo_std/internal/context.h>
#include <fimo_std/memory.h>

// We use an atomic integer to track the state of a call stack.
// In the public API we expose one of the following states:
//  1. Not bound and suspended
//  2. Not bound, suspended and blocked.
//  3. Bound
//  4. Bound and suspended
//  5. Bound, suspended and blocked.
//
// Binding the stack to the current thread implies acquiring
// a mutex on the stack, and enables modifications without
// any additional synchronisation. Additionally we need to
// support operations that operate on shared and unbound
// stacks like switching and unblocking the active stack.
// To implement these functions correctly, we include an
// additional lock in the form of the locked bit (is implied
// when bound).
#define BOUND_BIT_ 1
#define SUSPENDED_BIT_ 2
#define BLOCKED_BIT_ 4
#define LOCKED_BIT_ 8

///////////////////////////////////////////////////////////////////////
//// Forward Declarations
///////////////////////////////////////////////////////////////////////

struct StackFrame_;

static FimoError stack_frame_new_(FimoTracingCallStack *call_stack, const FimoTracingSpanDesc *span_desc,
                                  FimoTracingFormat format, const void *data);
static void stack_frame_free_(struct StackFrame_ *frame);

static FimoError call_stack_new_(FimoInternalTracingContext *ctx, bool bound, FimoTracingCallStack **call_stack);
static void call_stack_free_(FimoTracingCallStack *call_stack);
static bool call_stack_can_destroy_(FimoTracingCallStack *call_stack);
static bool call_stack_is_bound_(FimoTracingCallStack *call_stack);
static bool call_stack_is_suspended_(FimoTracingCallStack *call_stack);
static bool call_stack_is_blocked_(FimoTracingCallStack *call_stack);
static bool call_stack_would_trace_(FimoTracingCallStack *call_stack, const FimoTracingMetadata *metadata);
static bool call_stack_would_trace_(FimoTracingCallStack *call_stack, const FimoTracingMetadata *metadata);
static FimoError call_stack_switch_(FimoTracingCallStack *call_stack, FimoTracingCallStack *old);
static FimoError call_stack_unblock_(FimoTracingCallStack *call_stack);
static FimoError call_stack_suspend_(FimoTracingCallStack *call_stack, bool block);
static FimoError call_stack_resume_(FimoTracingCallStack *call_stack);
static FimoError call_stack_create_span_(FimoTracingCallStack *call_stack, const FimoTracingSpanDesc *span_desc,
                                         FimoTracingSpan **span, FimoTracingFormat format, const void *data);
static FimoError call_stack_destroy_span_(FimoTracingCallStack *call_stack, FimoTracingSpan *span);
static FimoError call_stack_emit_event_(FimoTracingCallStack *call_stack, const FimoTracingEvent *event,
                                        const FimoTracingFormat format, const void *data);

struct TSSData_;

static FimoError tss_data_new_(FimoInternalTracingContext *ctx, struct TSSData_ **tss_data);
static void tss_data_free_(struct TSSData_ *tss_data);

static FimoError ctx_init_(FimoInternalTracingContext *ctx, const FimoTracingCreationConfig *options);
static void ctx_deinit_(FimoInternalTracingContext *ctx);
static FimoError ctx_create_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack **call_stack);
static FimoError ctx_destroy_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack);
static FimoError ctx_switch_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack,
                                        FimoTracingCallStack **old);
static FimoError ctx_unblock_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack);
static FimoError ctx_suspend_current_call_stack_(FimoInternalTracingContext *ctx, bool block);
static FimoError ctx_resume_current_call_stack_(FimoInternalTracingContext *ctx);
static FimoError ctx_create_span_(FimoInternalTracingContext *ctx, const FimoTracingSpanDesc *span_desc,
                                  FimoTracingSpan **span, FimoTracingFormat format, const void *data);
static FimoError ctx_destroy_span_(FimoInternalTracingContext *ctx, FimoTracingSpan *span);
static FimoError ctx_emit_event_(FimoInternalTracingContext *ctx, const FimoTracingEvent *event,
                                 const FimoTracingFormat format, const void *data);
static bool ctx_is_enabled_(FimoInternalTracingContext *ctx);
static bool ctx_is_enabled_for_thread_(FimoInternalTracingContext *ctx);
static bool ctx_would_trace_(FimoInternalTracingContext *ctx, const FimoTracingMetadata *metadata);
static FimoError ctx_register_thread_(FimoInternalTracingContext *ctx);
static FimoError ctx_unregister_thread_(FimoInternalTracingContext *ctx);
static void ctx_flush_(FimoInternalTracingContext *ctx);

///////////////////////////////////////////////////////////////////////
//// Subscriber
///////////////////////////////////////////////////////////////////////

static void subscriber_free_(FimoTracingSubscriber *subscriber) {
    FIMO_DEBUG_ASSERT(subscriber)
    if (subscriber->vtable->destroy) {
        subscriber->vtable->destroy(subscriber->ptr);
    }
}

///////////////////////////////////////////////////////////////////////
//// Call Stack Frame
///////////////////////////////////////////////////////////////////////

struct FimoTracingCallStack {
    atomic_uint state;
    char *buffer;
    FimoUSize cursor;
    FimoTracingLevel max_level;
    FimoArrayList call_stacks;
    struct StackFrame_ *start_frame;
    struct StackFrame_ *end_frame;
    FimoInternalTracingContext *ctx;
};

struct StackFrame_ {
    FimoTracingSpan span;
    const FimoTracingMetadata *metadata;
    FimoArrayList spans;
    FimoUSize parent_cursor;
    FimoTracingLevel parent_max_level;
    struct StackFrame_ *next;
    struct StackFrame_ *previous;
    FimoTracingCallStack *call_stack;
};

static FimoError stack_frame_new_(FimoTracingCallStack *call_stack, const FimoTracingSpanDesc *span_desc,
                                  FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(call_stack && span_desc && format)

    FimoArrayList spans;
    FimoError error = fimo_array_list_with_capacity(fimo_array_list_len(&call_stack->ctx->subscribers), sizeof(void *),
                                                    alignof(void *), &spans);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    FimoUSize written_bytes;
    char *buffer_start = call_stack->buffer + call_stack->cursor;
    FimoUSize buffer_len = call_stack->ctx->buff_size - call_stack->cursor;

    error = format(buffer_start, buffer_len, data, &written_bytes);
    if (FIMO_IS_ERROR(error)) {
        goto cleanup;
    }

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < fimo_array_list_len(&call_stack->ctx->subscribers); i++) {
        void **stack_;
        error = fimo_array_list_get(&call_stack->call_stacks, i, sizeof(void *), (const void **)&stack_);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                    (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        void *span_;
        error = subscriber->vtable->span_create(subscriber->ptr, &current_time, span_desc, buffer_start, written_bytes,
                                                *stack_, &span_);
        if (FIMO_IS_ERROR(error)) {
            goto cleanup;
        }
    }

    struct StackFrame_ *frame = fimo_malloc(sizeof(*frame), &error);
    if (FIMO_IS_ERROR(error)) {
        goto cleanup;
    }

    *frame = (struct StackFrame_){
            .span =
                    {
                            .type = FIMO_STRUCT_TYPE_TRACING_SPAN,
                            .next = NULL,
                    },
            .metadata = span_desc->metadata,
            .spans = spans,
            .parent_cursor = call_stack->cursor,
            .parent_max_level = call_stack->max_level,
            .next = NULL,
            .previous = call_stack->end_frame,
            .call_stack = call_stack,
    };

    call_stack->cursor += written_bytes;
    call_stack->max_level =
            span_desc->metadata->level < call_stack->max_level ? span_desc->metadata->level : call_stack->max_level;

    if (call_stack->end_frame) {
        call_stack->end_frame->next = frame;
        call_stack->end_frame = frame;
    }
    else {
        call_stack->start_frame = frame;
        call_stack->end_frame = frame;
    }

    return FIMO_EOK;

cleanup:
    for (FimoUSize i = 0; !fimo_array_list_is_empty(&spans); i++) {
        void *span_;
        FimoError error_ = fimo_array_list_pop_front(&spans, sizeof(void *), &span_, NULL);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error_))

        void **stack_;
        error_ = fimo_array_list_get(&call_stack->call_stacks, i, sizeof(void *), (const void **)&stack_);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error_))

        const FimoTracingSubscriber *subscriber;
        error_ = fimo_array_list_get(&call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                     (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error_))

        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error_))
        subscriber->vtable->span_drop(subscriber->ptr, *stack_, span_);
    }

    return error;
}

static void stack_frame_free_(struct StackFrame_ *frame) {
    FIMO_DEBUG_ASSERT(frame)
    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < fimo_array_list_len(&frame->call_stack->ctx->subscribers); i++) {
        void *span_;
        FimoError error = fimo_array_list_pop_front(&frame->spans, sizeof(void *), &span_, NULL);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        void **stack_;
        error = fimo_array_list_get(&frame->call_stack->call_stacks, i, sizeof(void *), (const void **)&stack_);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&frame->call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                    (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        subscriber->vtable->span_destroy(subscriber->ptr, &current_time, *stack_, span_);
    }

    frame->call_stack->cursor = frame->parent_cursor;
    frame->call_stack->max_level = frame->parent_max_level;
    if (frame->previous != NULL) {
        frame->previous->next = NULL;
        frame->call_stack->end_frame = frame->previous;
    }
    else {
        frame->call_stack->start_frame = NULL;
        frame->call_stack->end_frame = NULL;
    }

    fimo_array_list_free(&frame->spans, sizeof(void *), alignof(void *), NULL);
    fimo_free(frame);
}

///////////////////////////////////////////////////////////////////////
//// Call Stack
///////////////////////////////////////////////////////////////////////

static FimoError call_stack_new_(FimoInternalTracingContext *ctx, bool bound, FimoTracingCallStack **call_stack) {
    FIMO_DEBUG_ASSERT(ctx && call_stack)

    FimoError error;
    char *buffer = fimo_calloc(ctx->buff_size, &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    FimoArrayList call_stacks;
    error = fimo_array_list_with_capacity(fimo_array_list_len(&ctx->subscribers), sizeof(void *), alignof(void *),
                                          &call_stacks);
    if (FIMO_IS_ERROR(error)) {
        fimo_free(buffer);
        return error;
    }

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < fimo_array_list_len(&ctx->subscribers); i++) {
        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&ctx->subscribers, i, sizeof(FimoTracingSubscriber), (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        void *stack_;
        error = subscriber->vtable->call_stack_create(subscriber->ptr, &current_time, &stack_);
        if (FIMO_IS_ERROR(error)) {
            goto cleanup_call_stacks;
        }
        error = fimo_array_list_try_push(&call_stacks, sizeof(void *), &stack_, NULL);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
    }

    unsigned int init_state = bound ? SUSPENDED_BIT_ | BOUND_BIT_ : SUSPENDED_BIT_;

    *call_stack = fimo_malloc(sizeof(**call_stack), &error);
    if (FIMO_IS_ERROR(error)) {
        goto cleanup_call_stacks;
    }

    **call_stack = (FimoTracingCallStack){
            .state = init_state,
            .buffer = buffer,
            .cursor = 0,
            .max_level = ctx->max_level,
            .start_frame = NULL,
            .end_frame = NULL,
            .ctx = ctx,
    };

    return FIMO_EOK;

cleanup_call_stacks:
    for (FimoUSize i = 0; !fimo_array_list_is_empty(&call_stacks); i++) {
        void *stack_;
        FimoError error_ = fimo_array_list_pop_front(&call_stacks, sizeof(void *), &stack_, NULL);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error_))

        const FimoTracingSubscriber *subscriber;
        error_ = fimo_array_list_get(&ctx->subscribers, i, sizeof(FimoTracingSubscriber), (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error_))
        subscriber->vtable->call_stack_drop(subscriber->ptr, &stack_);
    }

    fimo_array_list_free(&call_stacks, sizeof(void *), alignof(void *), NULL);
    fimo_free(buffer);

    return error;
}

static void call_stack_free_(FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(call_stack && call_stack_can_destroy_(call_stack))

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; !fimo_array_list_is_empty(&call_stack->call_stacks); i++) {
        void *stack_;
        FimoError error = fimo_array_list_pop_front(&call_stack->call_stacks, sizeof(void *), &stack_, NULL);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                    (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        subscriber->vtable->call_stack_destroy(subscriber->ptr, &current_time, stack_);
    }
    fimo_array_list_free(&call_stack->call_stacks, sizeof(void *), alignof(void *), NULL);

    fimo_free(call_stack->buffer);
    fimo_free(call_stack);
}

static bool call_stack_can_destroy_(FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(call_stack)
    const unsigned int state = atomic_load_explicit(&call_stack->state, memory_order_acquire);
    return (state & (unsigned int)(BOUND_BIT_ | BLOCKED_BIT_)) == 0 && call_stack->end_frame == NULL;
}

static bool call_stack_is_bound_(FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(call_stack)
    unsigned int state = atomic_load_explicit(&call_stack->state, memory_order_relaxed);
    return (state & (unsigned int)BOUND_BIT_) != 0;
}

static bool call_stack_is_suspended_(FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(call_stack)
    unsigned int state = atomic_load_explicit(&call_stack->state, memory_order_relaxed);
    return (state & (unsigned int)SUSPENDED_BIT_) != 0;
}

static bool call_stack_is_blocked_(FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(call_stack)
    unsigned int state = atomic_load_explicit(&call_stack->state, memory_order_relaxed);
    return (state & (unsigned int)BLOCKED_BIT_) != 0;
}

static bool call_stack_would_trace_(FimoTracingCallStack *call_stack, const FimoTracingMetadata *metadata) {
    FIMO_DEBUG_ASSERT(call_stack && metadata)
    return call_stack->max_level >= metadata->level;
}

static FimoError call_stack_switch_(FimoTracingCallStack *call_stack, FimoTracingCallStack *old) {
    FIMO_DEBUG_ASSERT(call_stack && old && call_stack_is_bound_(old))
    FIMO_DEBUG_ASSERT_FALSE(call_stack == old)

    // When the call stack is not bound we must synchronize our access.
    // We do this by locking it.
    unsigned int expected = atomic_load_explicit(&call_stack->state, memory_order_relaxed);
    for (;;) {
        if (expected & (unsigned int)LOCKED_BIT_) {
            continue;
        }
        if (expected & (unsigned int)BOUND_BIT_ || expected & (unsigned int)BLOCKED_BIT_ ||
            !(expected & (unsigned int)SUSPENDED_BIT_)) {
            return FIMO_EPERM;
        }

        unsigned int state = expected | (unsigned int)BOUND_BIT_;
        if (atomic_compare_exchange_weak_explicit(&call_stack->state, &expected, state, memory_order_acquire,
                                                  memory_order_relaxed)) {
            break;
        }
    }

    unsigned int old_state = atomic_load_explicit(&old->state, memory_order_relaxed);
    old_state &= ~((unsigned int)BOUND_BIT_);
    atomic_store_explicit(&old->state, old_state, memory_order_release);

    return FIMO_EOK;
}

static FimoError call_stack_unblock_(FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(call_stack)

    // We allow unblocking a call stack that is not bound,
    // therefore we must synchronize our access to its.
    // We do this by locking it.
    unsigned int expected = atomic_load_explicit(&call_stack->state, memory_order_relaxed);
    for (;;) {
        if (expected & (unsigned int)LOCKED_BIT_) {
            continue;
        }
        if (expected & (unsigned int)BOUND_BIT_ || !(expected & (unsigned int)BLOCKED_BIT_)) {
            return FIMO_EPERM;
        }

        unsigned int state = expected | (unsigned int)LOCKED_BIT_;
        if (atomic_compare_exchange_weak_explicit(&call_stack->state, &expected, state, memory_order_acquire,
                                                  memory_order_relaxed)) {
            break;
        }
    }

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < fimo_array_list_len(&call_stack->ctx->subscribers); i++) {
        void **stack_;
        FimoError error = fimo_array_list_get(&call_stack->call_stacks, i, sizeof(void *), (const void **)&stack_);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                    (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        subscriber->vtable->call_stack_unblock(subscriber->ptr, &current_time, *stack_);
    }

    unsigned int state = expected & ~((unsigned int)BLOCKED_BIT_);
    atomic_store_explicit(&call_stack->state, state, memory_order_release);

    return FIMO_EOK;
}

static FimoError call_stack_suspend_(FimoTracingCallStack *call_stack, bool block) {
    FIMO_DEBUG_ASSERT(call_stack && call_stack_is_bound_(call_stack))
    if (call_stack_is_suspended_(call_stack)) {
        return FIMO_EPERM;
    }

    unsigned int state = atomic_load_explicit(&call_stack->state, memory_order_relaxed);
    state |= (unsigned int)SUSPENDED_BIT_;
    state |= block ? (unsigned int)BLOCKED_BIT_ : (unsigned int)0;
    atomic_store_explicit(&call_stack->state, state, memory_order_relaxed);

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < fimo_array_list_len(&call_stack->ctx->subscribers); i++) {
        void **stack_;
        FimoError error = fimo_array_list_get(&call_stack->call_stacks, i, sizeof(void *), (const void **)&stack_);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                    (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        subscriber->vtable->call_stack_suspend(subscriber->ptr, &current_time, *stack_, block);
    }

    return FIMO_EOK;
}

static FimoError call_stack_resume_(FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(call_stack && call_stack_is_bound_(call_stack))
    if (call_stack_is_blocked_(call_stack) || !call_stack_is_suspended_(call_stack)) {
        return FIMO_EPERM;
    }

    unsigned int state = atomic_load_explicit(&call_stack->state, memory_order_relaxed);
    state &= ~((unsigned int)SUSPENDED_BIT_);
    atomic_store_explicit(&call_stack->state, state, memory_order_relaxed);

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < fimo_array_list_len(&call_stack->ctx->subscribers); i++) {
        void **stack_;
        FimoError error = fimo_array_list_get(&call_stack->call_stacks, i, sizeof(void *), (const void **)&stack_);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                    (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        subscriber->vtable->call_stack_resume(subscriber->ptr, &current_time, *stack_);
    }

    return FIMO_EOK;
}

static FimoError call_stack_create_span_(FimoTracingCallStack *call_stack, const FimoTracingSpanDesc *span_desc,
                                         FimoTracingSpan **span, FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(call_stack && span_desc && span && format && call_stack_is_bound_(call_stack))
    if (call_stack_is_suspended_(call_stack)) {
        return FIMO_EPERM;
    }

    FimoError error = stack_frame_new_(call_stack, span_desc, format, data);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    FIMO_DEBUG_ASSERT(call_stack->end_frame);
    *span = &call_stack->end_frame->span;

    return FIMO_EOK;
}

static FimoError call_stack_destroy_span_(FimoTracingCallStack *call_stack, FimoTracingSpan *span) {
    FIMO_DEBUG_ASSERT(call_stack && span && call_stack_is_bound_(call_stack))
    if (call_stack_is_suspended_(call_stack)) {
        return FIMO_EPERM;
    }

    struct StackFrame_ *top = call_stack->end_frame;
    if (top == NULL || &top->span != span) {
        return FIMO_EPERM;
    }
    stack_frame_free_(top);

    return FIMO_EOK;
}

static FimoError call_stack_emit_event_(FimoTracingCallStack *call_stack, const FimoTracingEvent *event,
                                        const FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(call_stack && event && format && call_stack_is_bound_(call_stack))
    if (call_stack_is_suspended_(call_stack)) {
        return FIMO_EPERM;
    }
    if (!call_stack_would_trace_(call_stack, event->metadata)) {
        return FIMO_EOK;
    }

    char *buffer_start = call_stack->buffer + call_stack->cursor;
    FimoUSize buffer_len = call_stack->ctx->buff_size - call_stack->cursor;

    FimoUSize written_bytes = 0;
    FimoError error = format(buffer_start, buffer_len, data, &written_bytes);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < fimo_array_list_len(&call_stack->ctx->subscribers); i++) {
        void **stack_;
        error = fimo_array_list_get(&call_stack->call_stacks, i, sizeof(void *), (const void **)&stack_);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))

        const FimoTracingSubscriber *subscriber;
        error = fimo_array_list_get(&call_stack->ctx->subscribers, i, sizeof(FimoTracingSubscriber),
                                    (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        subscriber->vtable->event_emit(subscriber->ptr, &current_time, *stack_, event, buffer_start, written_bytes);
    }

    return FIMO_EOK;
}

///////////////////////////////////////////////////////////////////////
//// Thread Specific Data
///////////////////////////////////////////////////////////////////////

struct TSSData_ {
    FimoTracingCallStack *active;
    FimoInternalTracingContext *ctx;
};

static FimoError tss_data_new_(FimoInternalTracingContext *ctx, struct TSSData_ **tss_data) {
    FIMO_DEBUG_ASSERT(ctx && tss_data)

    FimoTracingCallStack *call_stack;
    FimoError error = call_stack_new_(ctx, true, &call_stack);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    *tss_data = fimo_malloc(sizeof(**tss_data), &error);
    if (FIMO_IS_ERROR(error)) {
        call_stack_free_(call_stack);
        return error;
    }

    **tss_data = (struct TSSData_){
            .active = call_stack,
            .ctx = ctx,
    };
    atomic_fetch_add_explicit(&ctx->thread_count, 1, memory_order_acquire);

    return FIMO_EOK;
}

static void tss_data_free_(struct TSSData_ *tss_data) {
    FIMO_DEBUG_ASSERT(tss_data)

    atomic_fetch_sub_explicit(&tss_data->ctx->thread_count, 1, memory_order_release);
    call_stack_free_(tss_data->active);
    fimo_free(tss_data);
}

///////////////////////////////////////////////////////////////////////
//// Creation Config
///////////////////////////////////////////////////////////////////////

static void creation_config_cleanup_(const FimoTracingCreationConfig *options) {
    FIMO_DEBUG_ASSERT(options)
    for (FimoUSize i = 0; i < options->subscriber_count; i++) {
        subscriber_free_(&options->subscribers[i]);
    }
}

///////////////////////////////////////////////////////////////////////
//// Context
///////////////////////////////////////////////////////////////////////

static FimoError ctx_init_(FimoInternalTracingContext *ctx, const FimoTracingCreationConfig *options) {
    FIMO_DEBUG_ASSERT(ctx)
    FimoUSize format_buffer_size = 1024;
    FimoTracingLevel maximum_level = FIMO_TRACING_LEVEL_OFF;
    FimoArrayList subscribers = fimo_array_list_new();
    if (options) {
        if (options->format_buffer_size != 0) {
            format_buffer_size = options->format_buffer_size;
        }
        maximum_level = options->maximum_level;
        FimoError error = fimo_array_list_with_capacity(options->subscriber_count, sizeof(FimoTracingSubscriber),
                                                        alignof(FimoTracingSubscriber), &subscribers);
        if (FIMO_IS_ERROR(error)) {
            creation_config_cleanup_(options);
            return error;
        }

        for (FimoUSize i = 0; i < options->subscriber_count; i++) {
            error = fimo_array_list_try_push(&subscribers, sizeof(FimoTracingSubscriber), &options->subscribers[i],
                                             NULL);
            FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        }
    }

    tss_t local_data;
    if (tss_create(&local_data, (tss_dtor_t)tss_data_free_) != thrd_success) {
        fimo_array_list_free(&subscribers, sizeof(FimoTracingSubscriber), alignof(FimoTracingSubscriber),
                             (FimoArrayListDropFunc)subscriber_free_);
        return FIMO_EUNKNOWN;
    }

    *ctx = (FimoInternalTracingContext){
            .buff_size = format_buffer_size,
            .max_level = maximum_level,
            .subscribers = subscribers,
            .tss_data = local_data,
            .thread_count = 0,
    };

    return FIMO_EOK;
}

static void ctx_deinit_(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    FimoUSize remaining_threads = atomic_load_explicit(&ctx->thread_count, memory_order_acquire);
    FIMO_DEBUG_ASSERT_FALSE(remaining_threads > 1);

    // There are three possibilities:
    // 1. All threads are cleaned up.
    // 2. Our thread must be cleaned up.
    // 3. Another thread must be cleaned up.
    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    FIMO_DEBUG_ASSERT_FALSE(remaining_threads == 1 && local_data == NULL)
    if (local_data) {
        tss_data_free_(local_data);
    }

    // Now that we know that there are no threads left, we can delete the tss.
    tss_delete(ctx->tss_data);
    fimo_array_list_free(&ctx->subscribers, sizeof(FimoTracingSubscriber), alignof(FimoTracingSubscriber),
                         (FimoArrayListDropFunc)subscriber_free_);
}

static FimoError ctx_create_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack **call_stack) {
    FIMO_DEBUG_ASSERT(ctx && call_stack)
    if (!ctx_is_enabled_(ctx)) {
        *call_stack = NULL;
        return FIMO_EOK;
    }

    return call_stack_new_(ctx, false, call_stack);
}

static FimoError ctx_destroy_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        FIMO_DEBUG_ASSERT_FALSE(call_stack)
        return FIMO_EOK;
    }

    if (!call_stack_can_destroy_(call_stack)) {
        return FIMO_EPERM;
    }
    call_stack_free_(call_stack);
    return FIMO_EOK;
}

static FimoError ctx_switch_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack,
                                        FimoTracingCallStack **old) {
    FIMO_DEBUG_ASSERT(ctx && old)
    if (!ctx_is_enabled_(ctx)) {
        FIMO_DEBUG_ASSERT_FALSE(call_stack)
        *old = NULL;
        return FIMO_EOK;
    }
    if (call_stack == NULL) {
        return FIMO_EINVAL;
    }
    if (!ctx_is_enabled_for_thread_(ctx)) {
        return FIMO_ENOTSUP;
    }

    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    FIMO_DEBUG_ASSERT(local_data && local_data->active)
    if (local_data->active == call_stack) {
        return FIMO_EINVAL;
    }

    const FimoError error = call_stack_switch_(call_stack, local_data->active);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    *old = local_data->active;
    local_data->active = call_stack;

    return FIMO_EOK;
}

static FimoError ctx_unblock_call_stack_(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        FIMO_DEBUG_ASSERT_FALSE(call_stack)
        return FIMO_EOK;
    }
    if (call_stack == NULL) {
        return FIMO_EINVAL;
    }
    return call_stack_unblock_(call_stack);
}

static FimoError ctx_suspend_current_call_stack_(FimoInternalTracingContext *ctx, bool block) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        return FIMO_EOK;
    }
    if (!ctx_is_enabled_for_thread_(ctx)) {
        return FIMO_ENOTSUP;
    }

    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    FIMO_DEBUG_ASSERT(local_data && local_data->active)
    return call_stack_suspend_(local_data->active, block);
}

static FimoError ctx_resume_current_call_stack_(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        return FIMO_EOK;
    }
    if (!ctx_is_enabled_for_thread_(ctx)) {
        return FIMO_ENOTSUP;
    }

    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    FIMO_DEBUG_ASSERT(local_data && local_data->active)
    return call_stack_resume_(local_data->active);
}

static FimoError ctx_create_span_(FimoInternalTracingContext *ctx, const FimoTracingSpanDesc *span_desc,
                                  FimoTracingSpan **span, FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(ctx && span_desc && span && format)
    if (!ctx_is_enabled_(ctx)) {
        *span = NULL;
        return FIMO_EOK;
    }
    if (!ctx_is_enabled_for_thread_(ctx)) {
        return FIMO_ENOTSUP;
    }

    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    FIMO_DEBUG_ASSERT(local_data && local_data->active)
    return call_stack_create_span_(local_data->active, span_desc, span, format, data);
}

static FimoError ctx_destroy_span_(FimoInternalTracingContext *ctx, FimoTracingSpan *span) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        FIMO_DEBUG_ASSERT(span == NULL)
        return FIMO_EOK;
    }
    if (!ctx_is_enabled_for_thread_(ctx)) {
        return FIMO_ENOTSUP;
    }

    FIMO_DEBUG_ASSERT(span)
    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    FIMO_DEBUG_ASSERT(local_data && local_data->active)
    return call_stack_destroy_span_(local_data->active, span);
}

static FimoError ctx_emit_event_(FimoInternalTracingContext *ctx, const FimoTracingEvent *event,
                                 const FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(ctx && event && format)
    if (!ctx_would_trace_(ctx, event->metadata)) {
        return FIMO_EOK;
    }

    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    FIMO_DEBUG_ASSERT(local_data && local_data->active)
    return call_stack_emit_event_(local_data->active, event, format, data);
}

static bool ctx_is_enabled_(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return !(ctx->max_level == FIMO_TRACING_LEVEL_OFF || fimo_array_list_is_empty(&ctx->subscribers));
}

static bool ctx_is_enabled_for_thread_(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_is_enabled_(ctx) && tss_get(ctx->tss_data) != NULL;
}

static bool ctx_would_trace_(FimoInternalTracingContext *ctx, const FimoTracingMetadata *metadata) {
    FIMO_DEBUG_ASSERT(ctx && metadata)
    return ctx_is_enabled_for_thread_(ctx) && ctx->max_level >= metadata->level;
}

static FimoError ctx_register_thread_(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        return FIMO_EOK;
    }

    if (tss_get(ctx->tss_data)) {
        return FIMO_EPERM;
    }

    struct TSSData_ *local_data;
    const FimoError error = tss_data_new_(ctx, &local_data);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }
    const int result = tss_set(ctx->tss_data, local_data);
    FIMO_DEBUG_ASSERT(result == thrd_success);

    return FIMO_EOK;
}

static FimoError ctx_unregister_thread_(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        return FIMO_EOK;
    }

    struct TSSData_ *local_data = tss_get(ctx->tss_data);
    if (local_data == NULL || local_data->active->end_frame != NULL) {
        return FIMO_EPERM;
    }

    tss_data_free_(local_data);
    const int result = tss_set(ctx->tss_data, NULL);
    FIMO_DEBUG_ASSERT(result == thrd_success);

    return FIMO_EOK;
}

static void ctx_flush_(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    if (!ctx_is_enabled_(ctx)) {
        return;
    }

    for (FimoUSize i = 0; i < fimo_array_list_len(&ctx->subscribers); i++) {
        const FimoTracingSubscriber *subscriber;
        const FimoError error =
                fimo_array_list_get(&ctx->subscribers, i, sizeof(FimoTracingSubscriber), (const void **)&subscriber);
        FIMO_DEBUG_ASSERT_FALSE(FIMO_IS_ERROR(error))
        subscriber->vtable->flush(subscriber->ptr);
    }
}

///////////////////////////////////////////////////////////////////////
//// Trampoline functions
///////////////////////////////////////////////////////////////////////

FimoError fimo_internal_trampoline_tracing_call_stack_create(void *ctx, FimoTracingCallStack **call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_call_stack_create(&((FimoInternalContext *)ctx)->tracing, call_stack);
}

FimoError fimo_internal_trampoline_tracing_call_stack_destroy(void *ctx, FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_call_stack_destroy(&((FimoInternalContext *)ctx)->tracing, call_stack);
}

FimoError fimo_internal_trampoline_tracing_call_stack_switch(void *ctx, FimoTracingCallStack *call_stack,
                                                             FimoTracingCallStack **old) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_call_stack_switch(&((FimoInternalContext *)ctx)->tracing, call_stack, old);
}

FimoError fimo_internal_trampoline_tracing_call_stack_unblock(void *ctx, FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_call_stack_unblock(&((FimoInternalContext *)ctx)->tracing, call_stack);
}

FimoError fimo_internal_trampoline_tracing_call_stack_suspend_current(void *ctx, const bool block) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_call_stack_suspend_current(&((FimoInternalContext *)ctx)->tracing, block);
}

FimoError fimo_internal_trampoline_tracing_call_stack_resume_current(void *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_call_stack_resume_current(&((FimoInternalContext *)ctx)->tracing);
}

FimoError fimo_internal_trampoline_tracing_span_create(void *ctx, const FimoTracingSpanDesc *span_desc,
                                                       FimoTracingSpan **span, FimoTracingFormat format,
                                                       const void *data) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_span_create_custom(&((FimoInternalContext *)ctx)->tracing, span_desc, span, format,
                                                    data);
}

FimoError fimo_internal_trampoline_tracing_span_destroy(void *ctx, FimoTracingSpan *span) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_span_destroy(&((FimoInternalContext *)ctx)->tracing, span);
}

FimoError fimo_internal_trampoline_tracing_event_emit(void *ctx, const FimoTracingEvent *event,
                                                      FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_event_emit_custom(&((FimoInternalContext *)ctx)->tracing, event, format, data);
}

bool fimo_internal_trampoline_tracing_is_enabled(void *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_is_enabled(&((FimoInternalContext *)ctx)->tracing);
}

FimoError fimo_internal_trampoline_tracing_register_thread(void *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_register_thread(&((FimoInternalContext *)ctx)->tracing);
}

FimoError fimo_internal_trampoline_tracing_unregister_thread(void *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_unregister_thread(&((FimoInternalContext *)ctx)->tracing);
}

FimoError fimo_internal_trampoline_tracing_flush(void *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return fimo_internal_tracing_flush(&((FimoInternalContext *)ctx)->tracing);
}

///////////////////////////////////////////////////////////////////////
//// Tracing Subsystem API
///////////////////////////////////////////////////////////////////////

FIMO_MUST_USE
FimoError fimo_internal_tracing_init(FimoInternalTracingContext *ctx, const FimoTracingCreationConfig *options) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_init_(ctx, options);
}

void fimo_internal_tracing_destroy(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    ctx_deinit_(ctx);
}

void fimo_internal_tracing_cleanup_options(const FimoTracingCreationConfig *options) {
    FIMO_DEBUG_ASSERT(options)
    creation_config_cleanup_(options);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_create(FimoInternalTracingContext *ctx, FimoTracingCallStack **call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    if (call_stack == NULL) {
        return FIMO_EINVAL;
    }
    return ctx_create_call_stack_(ctx, call_stack);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_destroy(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_destroy_call_stack_(ctx, call_stack);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_switch(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack,
                                                  FimoTracingCallStack **old) {
    FIMO_DEBUG_ASSERT(ctx)
    if (old == NULL) {
        return FIMO_EINVAL;
    }
    return ctx_switch_call_stack_(ctx, call_stack, old);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_unblock(FimoInternalTracingContext *ctx, FimoTracingCallStack *call_stack) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_unblock_call_stack_(ctx, call_stack);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_suspend_current(FimoInternalTracingContext *ctx, const bool block) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_suspend_current_call_stack_(ctx, block);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_resume_current(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_resume_current_call_stack_(ctx);
}

FIMO_PRINT_F_FORMAT_ATTR(4, 5)
FIMO_MUST_USE
FimoError fimo_internal_tracing_span_create_fmt(FimoInternalTracingContext *ctx, const FimoTracingSpanDesc *span_desc,
                                                FimoTracingSpan **span, FIMO_PRINT_F_FORMAT const char *format, ...) {
    FIMO_DEBUG_ASSERT(ctx)
    if (span_desc == NULL || span == NULL || format == NULL) {
        return FIMO_EINVAL;
    }

    va_list vlist;
    va_start(vlist, format);
    FimoImplTracingFmtArgs args = {.format = format, .vlist = &vlist};
    const FimoError result =
            fimo_internal_tracing_span_create_custom(ctx, span_desc, span, fimo_impl_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_span_create_custom(FimoInternalTracingContext *ctx,
                                                   const FimoTracingSpanDesc *span_desc, FimoTracingSpan **span,
                                                   FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(ctx)
    if (span_desc == NULL || span == NULL || format == NULL) {
        return FIMO_EINVAL;
    }
    return ctx_create_span_(ctx, span_desc, span, format, data);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_span_destroy(FimoInternalTracingContext *ctx, FimoTracingSpan *span) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_destroy_span_(ctx, span);
}

FIMO_PRINT_F_FORMAT_ATTR(3, 4)
FIMO_MUST_USE
FimoError fimo_internal_tracing_event_emit_fmt(FimoInternalTracingContext *ctx, const FimoTracingEvent *event,
                                               FIMO_PRINT_F_FORMAT const char *format, ...) {
    FIMO_DEBUG_ASSERT(ctx)
    if (event == NULL || format == NULL) {
        return FIMO_EINVAL;
    }

    va_list vlist;
    va_start(vlist, format);
    FimoImplTracingFmtArgs args = {.format = format, .vlist = &vlist};
    const FimoError result = fimo_internal_tracing_event_emit_custom(ctx, event, fimo_impl_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_event_emit_custom(FimoInternalTracingContext *ctx, const FimoTracingEvent *event,
                                                  const FimoTracingFormat format, const void *data) {
    FIMO_DEBUG_ASSERT(ctx)
    if (event == NULL || format == NULL) {
        return FIMO_EINVAL;
    }
    return ctx_emit_event_(ctx, event, format, data);
}

FIMO_MUST_USE
bool fimo_internal_tracing_is_enabled(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_is_enabled_(ctx);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_register_thread(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_register_thread_(ctx);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_unregister_thread(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    return ctx_unregister_thread_(ctx);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_flush(FimoInternalTracingContext *ctx) {
    FIMO_DEBUG_ASSERT(ctx)
    ctx_flush_(ctx);
    return FIMO_EOK;
}
