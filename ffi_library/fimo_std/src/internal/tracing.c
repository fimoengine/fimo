#include <fimo_std/internal/tracing.h>

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
#define FIMO_TRACING_CALL_STACK_BOUND_BIT 1
#define FIMO_TRACING_CALL_STACK_SUSPENDED_BIT 2
#define FIMO_TRACING_CALL_STACK_BLOCKED_BIT 4
#define FIMO_TRACING_CALL_STACK_LOCKED_BIT 8

typedef struct FimoInternalTracingSpan {
    void **subscriber_spans;
    const FimoTracingMetadata *metadata;
} FimoInternalTracingSpan;

typedef struct FimoInternalTracingCallStackFrame {
    struct FimoInternalTracingCallStackFrame *previous;
    struct FimoInternalTracingCallStackFrame *next;
    FimoInternalTracingSpan *span;
    FimoTracingLevel parent_max_level;
} FimoInternalTracingCallStackFrame;

typedef struct FimoInternalTracingCallStack {
    FimoInternalTracingCallStackFrame *start_frame;
    FimoInternalTracingCallStackFrame *end_frame;
    void **subscriber_call_stacks;
    FimoTracingLevel max_level;
    atomic_uint state;
} FimoInternalTracingCallStack;

typedef struct FimoInternalTracingThreadLocalData {
    FimoInternalTracingCallStack *active_call_stack;
    FimoInternalContext *context;
    char *format_buffer;
} FimoInternalTracingThreadLocalData;

static void fimo_internal_tracing_local_data_destroy_(FimoInternalTracingThreadLocalData *local_data);

static void fimo_internal_tracing_local_data_destroy_tss_dtor_(void *local_data);

FIMO_MUST_USE
static FimoError fimo_internal_tracing_local_data_create_(FimoInternalContext *context,
                                                          FimoInternalTracingThreadLocalData **local_data);

FIMO_MUST_USE
static FimoError fimo_internal_tracing_call_stack_create_(FimoInternalContext *context, bool bound,
                                                          FimoInternalTracingCallStack **call_stack);

static void fimo_internal_tracing_call_stack_destroy_(FimoInternalContext *context,
                                                      FimoInternalTracingCallStack *call_stack);

static FimoError fimo_internal_cthreads_error_to_fimo_error(const int error) {
    switch (error) {
        case thrd_success:
            return FIMO_EOK;
        case thrd_timedout:
            return FIMO_ETIMEDOUT;
        case thrd_busy:
            return FIMO_EBUSY;
        case thrd_nomem:
            return FIMO_ENOMEM;
        case thrd_error:
            return FIMO_EUNKNOWN;
        default:
            return FIMO_EUNKNOWN;
    }
}

static void fimo_internal_tracing_local_data_destroy_(FimoInternalTracingThreadLocalData *local_data) {
    if (local_data == NULL) {
        return;
    }

    atomic_fetch_sub_explicit(&local_data->context->tracing.thread_count, 1, memory_order_release);
    fimo_internal_tracing_call_stack_destroy_(local_data->context, local_data->active_call_stack);
    fimo_free(local_data->format_buffer);
    fimo_free_aligned_sized(local_data, _Alignof(FimoInternalTracingThreadLocalData),
                            sizeof(FimoInternalTracingThreadLocalData));
}

static void fimo_internal_tracing_local_data_destroy_tss_dtor_(void *local_data) {
    fimo_internal_tracing_local_data_destroy_(local_data);
}

FIMO_MUST_USE
static FimoError fimo_internal_tracing_local_data_create_(FimoInternalContext *context,
                                                          FimoInternalTracingThreadLocalData **local_data) {
    FimoError error = FIMO_EOK;
    char *format_buffer = fimo_malloc(sizeof(char) * context->tracing.format_buffer_size, &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    FimoInternalTracingCallStack *call_stack = NULL;
    error = fimo_internal_tracing_call_stack_create_(context, true, &call_stack);
    if (FIMO_IS_ERROR(error)) {
        fimo_free(format_buffer);
        return error;
    }

    FimoInternalTracingThreadLocalData *data = fimo_aligned_alloc(_Alignof(FimoInternalTracingThreadLocalData),
                                                                  sizeof(FimoInternalTracingThreadLocalData), &error);
    if (FIMO_IS_ERROR(error)) {
        fimo_internal_tracing_call_stack_destroy_(context, call_stack);
        fimo_free(format_buffer);
        return error;
    }

    data->active_call_stack = call_stack;
    data->context = context;
    data->format_buffer = format_buffer;
    atomic_fetch_add_explicit(&context->tracing.thread_count, 1, memory_order_relaxed);
    *local_data = data;

    return FIMO_EOK;
}

FIMO_MUST_USE
static FimoError fimo_internal_tracing_call_stack_create_(FimoInternalContext *context, const bool bound,
                                                          FimoInternalTracingCallStack **call_stack) {
    if (call_stack == NULL || context == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    void **subscriber_call_stacks =
            fimo_aligned_alloc(_Alignof(void *), sizeof(void *) * context->tracing.subscriber_count, &error);
    if (FIMO_IS_ERROR(error)) {
        goto error;
    }

    FimoUSize initialized_call_stacks = 0;
    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < context->tracing.subscriber_count; i++) {
        error = context->tracing.subscribers->vtable->call_stack_create(context->tracing.subscribers->ptr,
                                                                        &current_time, &subscriber_call_stacks[i]);
        if (FIMO_IS_ERROR(error)) {
            goto error_call_stack_init;
        }
        initialized_call_stacks = i + 1;
    }

    FimoInternalTracingCallStack *stack = fimo_aligned_alloc(_Alignof(FimoInternalTracingCallStack *),
                                                             sizeof(FimoInternalTracingCallStack *), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_call_stack_init;
    }

    unsigned int state = FIMO_TRACING_CALL_STACK_SUSPENDED_BIT;
    if (bound) {
        state |= FIMO_TRACING_CALL_STACK_BOUND_BIT;
    }

    stack->start_frame = NULL;
    stack->end_frame = NULL;
    stack->subscriber_call_stacks = subscriber_call_stacks;
    stack->max_level = context->tracing.maximum_level;
    atomic_init(&stack->state, state);

    *call_stack = stack;

    return FIMO_EOK;

error_call_stack_init:
    for (FimoUSize i = 0; i < initialized_call_stacks; i++) {
        context->tracing.subscribers->vtable->call_stack_destroy(context->tracing.subscribers->ptr, &current_time,
                                                                 subscriber_call_stacks[i]);
    }
    fimo_free_aligned_sized(subscriber_call_stacks, _Alignof(void *),
                            sizeof(void *) * context->tracing.subscriber_count);
error:
    return error;
}

static void fimo_internal_tracing_call_stack_destroy_(FimoInternalContext *context,
                                                      FimoInternalTracingCallStack *call_stack) {
    FimoInternalTracingCallStackFrame *next_frame = NULL;
    for (FimoInternalTracingCallStackFrame *frame = call_stack->start_frame; frame; frame = next_frame) {
        next_frame = frame->next;
        fimo_free_aligned_sized(frame, _Alignof(FimoInternalTracingCallStackFrame),
                                sizeof(FimoInternalTracingCallStackFrame));
    }

    const FimoTime current_time = fimo_time_now();
    FimoUSize subscriber_count = context->tracing.subscriber_count;
    for (FimoUSize i = 0; i < subscriber_count; i++) {
        const FimoTracingSubscriber *subscriber = &context->tracing.subscribers[i];
        subscriber->vtable->call_stack_destroy(subscriber->ptr, &current_time, call_stack->subscriber_call_stacks[i]);
    }
    fimo_free_aligned_sized(call_stack->subscriber_call_stacks, _Alignof(void *), sizeof(void *) * subscriber_count);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_init(FimoInternalContext *context, const FimoTracingCreationConfig *options) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoUSize format_buffer_size = 0;
    FimoTracingLevel maximum_level = FIMO_TRACING_LEVEL_OFF;
    FimoTracingSubscriber *subscribers = NULL;
    FimoUSize subscriber_count = 0;
    if (options) {
        format_buffer_size = options->format_buffer_size;
        maximum_level = options->maximum_level;
        subscriber_count = options->subscriber_count;

        if ((subscriber_count == 0 && options->subscribers) || (subscriber_count > 0 && options->subscribers == NULL)) {
            return FIMO_EINVAL;
        }

        FimoError error = FIMO_EOK;
        subscribers = fimo_aligned_alloc(_Alignof(FimoTracingSubscriber),
                                         subscriber_count * sizeof(FimoTracingSubscriber), &error);
        if (FIMO_IS_ERROR(error)) {
            return error;
        }

        for (FimoUSize i = 0; i < subscriber_count; i++) {
            // ReSharper disable once CppDFANullDereference
            subscribers[i] = options->subscribers[i];
        }
    }

    tss_t local_data;
    const FimoError error = fimo_internal_cthreads_error_to_fimo_error(
            tss_create(&local_data, fimo_internal_tracing_local_data_destroy_tss_dtor_));
    if (FIMO_IS_ERROR(error)) {
        fimo_free_aligned_sized(subscribers, _Alignof(FimoTracingSubscriber),
                                subscriber_count * sizeof(FimoTracingSubscriber));
        return error;
    }

    context->tracing.format_buffer_size = format_buffer_size;
    context->tracing.maximum_level = maximum_level;
    context->tracing.subscribers = subscribers;
    context->tracing.subscriber_count = subscriber_count;
    context->tracing.thread_local_data = local_data;
    atomic_init(&context->tracing.thread_count, 0);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_check_destroy(FimoInternalContext *context) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    // We are only able to destroy the local data of our own thread, therefore we
    // check that there is only one thread left to clean up.
    FimoUSize registered_threads = atomic_load_explicit(&context->tracing.thread_count, memory_order_acquire);
    if (registered_threads > 1) {
        return FIMO_EBUSY;
    }

    // There are three possibilities:
    // 1. All threads are cleaned up.
    // 2. Our thread must be cleaned up.
    // 3. Another thread must be cleaned up (branch).
    void *local_data = tss_get(context->tracing.thread_local_data);
    if (registered_threads == 1 && local_data == NULL) {
        return FIMO_EBUSY;
    }

    return FIMO_EOK;
}

void fimo_internal_tracing_destroy(FimoInternalContext *context) {
    const FimoError error = fimo_internal_tracing_check_destroy(context);
    if (FIMO_IS_ERROR(error)) {
        exit(EXIT_FAILURE);
    }

    // Given the reference count of the context, we kow that there
    // are no other threads with access to the current instance.
    // Therefore it is not possible for new threads to be registered.
    // Additionally, `fimo_internal_tracing_check_destroy` checked
    // that there is at most one thread and that no other thread
    // needs to be cleaned up. All we need to do is check, if we
    // need to clean up our local data, and clean it up, and destroy
    // the tss object afterward.
    FimoInternalTracingThreadLocalData *local_data = tss_get(context->tracing.thread_local_data);
    if (local_data) {
        fimo_internal_tracing_local_data_destroy_(local_data);
    }

    // Now that we know that there are no threads left, we can delete the tss.
    tss_delete(context->tracing.thread_local_data);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_create(void *context, FimoTracingCallStack *call_stack) {
    if (context == NULL || call_stack == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        *call_stack = NULL;
        return FIMO_EOK;
    }

    FimoInternalTracingCallStack **internal_call_stack = (FimoInternalTracingCallStack **)call_stack;
    return fimo_internal_tracing_call_stack_create_(context, false, internal_call_stack);
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_destroy(void *context, const FimoTracingCallStack call_stack) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        if (call_stack) {
            return FIMO_EINVAL;
        }
        return FIMO_EOK;
    }

    // We check it here, as it is possible for the backend to
    // create a null stack, in case it is disabled.
    if (call_stack == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalTracingCallStack *internal_call_stack = call_stack;
    unsigned int state = atomic_load_explicit(&internal_call_stack->state, memory_order_acquire);
    if (state & ((unsigned int)(FIMO_TRACING_CALL_STACK_BOUND_BIT | FIMO_TRACING_CALL_STACK_BLOCKED_BIT))) {
        return FIMO_EBUSY;
    }
    if (internal_call_stack->end_frame) {
        return FIMO_EBUSY;
    }

    fimo_internal_tracing_call_stack_destroy_(ctx, internal_call_stack);
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_switch(void *context, const FimoTracingCallStack call_stack,
                                                  FimoTracingCallStack *old_call_stack) {
    if (context == NULL || old_call_stack == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        if (call_stack) {
            return FIMO_EINVAL;
        }
        *old_call_stack = NULL;
        return FIMO_EOK;
    }

    FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL) {
        if (call_stack) {
            return FIMO_EINVAL;
        }
        *old_call_stack = NULL;
        return FIMO_EOK;
    }

    // We check it here, as it is possible for the backend
    // to create a null stack, in case it is disabled.
    if (call_stack == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalTracingCallStack *active_call_stack = local_data->active_call_stack;
    unsigned int active_state = atomic_load_explicit(&active_call_stack->state, memory_order_relaxed);
    if (!(active_state & ((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT))) {
        return FIMO_EPERM;
    }

    FimoInternalTracingCallStack *internal_call_stack = call_stack;
    unsigned int expected_state = atomic_load_explicit(&internal_call_stack->state, memory_order_relaxed);
    for (;;) {
        if (expected_state & ((unsigned int)FIMO_TRACING_CALL_STACK_LOCKED_BIT)) {
            continue;
        }
        if ((expected_state & ((unsigned int)FIMO_TRACING_CALL_STACK_BOUND_BIT)) ||
            (expected_state & ((unsigned int)FIMO_TRACING_CALL_STACK_BLOCKED_BIT)) ||
            !(expected_state & ((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT))) {
            return FIMO_EPERM;
        }
        if (atomic_compare_exchange_weak_explicit(&internal_call_stack->state, &expected_state,
                                                  expected_state | ((unsigned int)FIMO_TRACING_CALL_STACK_BOUND_BIT),
                                                  memory_order_acquire, memory_order_relaxed)) {
            break;
        }
    }

    active_state &= ~((unsigned int)FIMO_TRACING_CALL_STACK_BOUND_BIT);
    atomic_store_explicit(&active_call_stack->state, active_state, memory_order_release);
    local_data->active_call_stack = internal_call_stack;
    *old_call_stack = active_call_stack;
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_unblock(void *context, const FimoTracingCallStack call_stack) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        if (call_stack) {
            return FIMO_EINVAL;
        }
        return FIMO_EOK;
    }

    // We check it here, as it is possible for the backend to
    // create a null stack, in case it is disabled.
    if (call_stack == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalTracingCallStack *internal_call_stack = call_stack;
    unsigned int expected_state = atomic_load_explicit(&internal_call_stack->state, memory_order_relaxed);
    for (;;) {
        if (expected_state & ((unsigned int)FIMO_TRACING_CALL_STACK_BOUND_BIT)) {
            return FIMO_EPERM;
        }
        if (!(expected_state & ((unsigned int)FIMO_TRACING_CALL_STACK_BLOCKED_BIT))) {
            return FIMO_EPERM;
        }

        unsigned int new_state = expected_state;
        new_state |= (unsigned int)FIMO_TRACING_CALL_STACK_LOCKED_BIT;
        new_state &= ~((unsigned int)FIMO_TRACING_CALL_STACK_BLOCKED_BIT);
        if (atomic_compare_exchange_weak_explicit(&internal_call_stack->state, &expected_state, new_state,
                                                  memory_order_release, memory_order_relaxed)) {
            break;
        }
    }

    FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < tracing->subscriber_count; i++) {
        void *subscriber_call_stack = internal_call_stack->subscriber_call_stacks[i];
        FimoTracingSubscriber subscriber = tracing->subscribers[i];
        subscriber.vtable->call_stack_unblock(subscriber.ptr, &current_time, subscriber_call_stack);
    }

    unsigned int blocked_state = expected_state & ~((unsigned int)FIMO_TRACING_CALL_STACK_BLOCKED_BIT);
    atomic_store_explicit(&internal_call_stack->state, blocked_state, memory_order_release);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_suspend_current(void *context, const bool block) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        return FIMO_EOK;
    }

    const FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL) {
        return FIMO_EOK;
    }

    FimoInternalTracingCallStack *active_call_stack = local_data->active_call_stack;
    unsigned int call_stack_state = atomic_load_explicit(&active_call_stack->state, memory_order_relaxed);
    if (call_stack_state & ((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT)) {
        return FIMO_EPERM;
    }

    call_stack_state |= (unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT;
    call_stack_state |= block ? (unsigned int)FIMO_TRACING_CALL_STACK_BLOCKED_BIT : (unsigned int)0;
    atomic_store_explicit(&active_call_stack->state, call_stack_state, memory_order_relaxed);

    FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < tracing->subscriber_count; i++) {
        void *subscriber_call_stack = active_call_stack->subscriber_call_stacks[i];
        const FimoTracingSubscriber subscriber = tracing->subscribers[i];
        subscriber.vtable->call_stack_suspend(subscriber.ptr, &current_time, subscriber_call_stack, block);
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_call_stack_resume_current(void *context) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        return FIMO_EOK;
    }

    const FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL) {
        return FIMO_EOK;
    }

    FimoInternalTracingCallStack *active_call_stack = local_data->active_call_stack;

    unsigned int state = atomic_load_explicit(&active_call_stack->state, memory_order_relaxed);
    if ((state & (unsigned int)FIMO_TRACING_CALL_STACK_BLOCKED_BIT) ||
        (!(state & ((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT)))) {
        return FIMO_EPERM;
    }

    state &= ~((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT);
    atomic_store_explicit(&active_call_stack->state, state, memory_order_relaxed);

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < tracing->subscriber_count; i++) {
        void *subscriber_call_stack = active_call_stack->subscriber_call_stacks[i];
        const FimoTracingSubscriber subscriber = tracing->subscribers[i];
        subscriber.vtable->call_stack_resume(subscriber.ptr, &current_time, subscriber_call_stack);
    }

    return FIMO_EOK;
}

FIMO_PRINT_F_FORMAT_ATTR(4, 5)
FIMO_MUST_USE
FimoError fimo_internal_tracing_span_create_fmt(void *context, const FimoTracingSpanDesc *span_desc,
                                                FimoTracingSpan *span, FIMO_PRINT_F_FORMAT const char *format, ...) {
    va_list vlist;
    va_start(vlist, format);
    FimoImplTracingFmtArgs args = {.format = format, .vlist = &vlist};
    const FimoError result =
            fimo_internal_tracing_span_create_custom(context, span_desc, span, fimo_impl_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_span_create_custom(void *context, const FimoTracingSpanDesc *span_desc,
                                                   FimoTracingSpan *span, FimoTracingFormat format, const void *data) {
    if (context == NULL || span_desc == NULL || span == NULL || format == NULL) {
        return FIMO_EINVAL;
    }

    span->span_id = NULL;
    span->next = NULL;

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level < span_desc->metadata->level || tracing->subscriber_count == 0) {
        return FIMO_EOK;
    }

    FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL) {
        return FIMO_EOK;
    }

    FimoInternalTracingCallStack *active_call_stack = local_data->active_call_stack;
    unsigned int call_stack_state = atomic_load_explicit(&active_call_stack->state, memory_order_relaxed);
    if (call_stack_state & ((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT)) {
        return FIMO_EPERM;
    }

    if (active_call_stack->max_level < span_desc->metadata->level) {
        return FIMO_EOK;
    }

    FimoUSize written_bytes = 0;
    FimoError error = format(local_data->format_buffer, ctx->tracing.format_buffer_size, data, &written_bytes);
    if (FIMO_IS_ERROR(error)) {
        goto error_format;
    }

    const FimoTracingMetadata *metadata = span_desc->metadata;
    void **subscriber_spans = fimo_aligned_alloc(_Alignof(void *), sizeof(void *) * tracing->subscriber_count, &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_subscriber_spans;
    }

    FimoInternalTracingSpan *internal_span =
            fimo_aligned_alloc(_Alignof(FimoInternalTracingSpan), sizeof(FimoInternalTracingSpan), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_internal_span;
    }

    internal_span->metadata = metadata;
    internal_span->subscriber_spans = subscriber_spans;

    FimoInternalTracingCallStackFrame *top_frame = fimo_aligned_alloc(
            _Alignof(FimoInternalTracingCallStackFrame), sizeof(FimoInternalTracingCallStackFrame), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_call_stack_frame;
    }

    top_frame->previous = active_call_stack->end_frame;
    top_frame->next = NULL;
    top_frame->span = internal_span;
    top_frame->parent_max_level = active_call_stack->max_level;

    active_call_stack->max_level = active_call_stack->max_level <= top_frame->span->metadata->level
                                           ? active_call_stack->max_level
                                           : top_frame->span->metadata->level;
    active_call_stack->end_frame = top_frame;
    if (active_call_stack->start_frame == NULL) {
        active_call_stack->start_frame = top_frame;
    }

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < tracing->subscriber_count; i++) {
        void *subscriber_call_stack = active_call_stack->subscriber_call_stacks[i];
        void **subscriber_span = &top_frame->span->subscriber_spans[i];
        const FimoTracingSubscriber subscriber = tracing->subscribers[i];
        subscriber.vtable->span_create(subscriber.ptr, &current_time, span_desc, local_data->format_buffer,
                                       written_bytes, subscriber_call_stack, subscriber_span);
    }

    return FIMO_EOK;

error_call_stack_frame:;
    fimo_free_aligned_sized(internal_span, _Alignof(FimoInternalTracingSpan), sizeof(FimoInternalTracingSpan));
error_internal_span:
    fimo_free_aligned_sized(subscriber_spans, _Alignof(void *), sizeof(void *) * tracing->subscriber_count);
error_subscriber_spans:;
error_format:
    return error;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_span_destroy(void *context, FimoTracingSpan *span) {
    if (context == NULL || span == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    FimoInternalTracingSpan *internal_span = span->span_id;
    if (internal_span == NULL || tracing->maximum_level < internal_span->metadata->level ||
        tracing->subscriber_count == 0) {
        return FIMO_EOK;
    }

    FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL) {
        return FIMO_EOK;
    }

    FimoInternalTracingCallStack *active_call_stack = local_data->active_call_stack;
    unsigned int call_stack_state = atomic_load_explicit(&active_call_stack->state, memory_order_relaxed);
    if (call_stack_state & ((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT)) {
        return FIMO_EPERM;
    }

    FimoInternalTracingCallStackFrame *top_frame = active_call_stack->end_frame;
    if (top_frame == NULL || internal_span != top_frame->span) {
        return FIMO_EPERM;
    }

    active_call_stack->max_level = top_frame->parent_max_level;
    active_call_stack->end_frame = top_frame->previous;
    if (active_call_stack->end_frame) {
        active_call_stack->end_frame->next = NULL;
    }
    else {
        active_call_stack->start_frame = NULL;
    }

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < tracing->subscriber_count; i++) {
        void *subscriber_call_stack = active_call_stack->subscriber_call_stacks[i];
        void *subscriber_span = internal_span->subscriber_spans[i];
        FimoTracingSubscriber subscriber = tracing->subscribers[i];
        subscriber.vtable->span_destroy(subscriber.ptr, &current_time, subscriber_call_stack, subscriber_span);
    }

    fimo_free_aligned_sized(internal_span->subscriber_spans, _Alignof(void *),
                            sizeof(void *) * tracing->subscriber_count);
    fimo_free_aligned_sized(internal_span, _Alignof(FimoInternalTracingSpan), sizeof(FimoInternalTracingSpan));

    fimo_free_aligned_sized(top_frame, _Alignof(FimoInternalTracingCallStackFrame),
                            sizeof(FimoInternalTracingCallStackFrame));

    return FIMO_EOK;
}

FIMO_PRINT_F_FORMAT_ATTR(3, 4)
FIMO_MUST_USE
FimoError fimo_internal_tracing_event_emit_fmt(void *context, const FimoTracingEvent *event,
                                               FIMO_PRINT_F_FORMAT const char *format, ...) {
    va_list vlist;
    va_start(vlist, format);
    FimoImplTracingFmtArgs args = {.format = format, .vlist = &vlist};
    const FimoError result = fimo_internal_tracing_event_emit_custom(context, event, fimo_impl_tracing_fmt, &args);
    va_end(vlist);
    return result;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_event_emit_custom(void *context, const FimoTracingEvent *event,
                                                  const FimoTracingFormat format, const void *data) {
    if (context == NULL || event == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level < event->metadata->level || tracing->subscriber_count == 0) {
        return FIMO_EOK;
    }

    const FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL) {
        return FIMO_EOK;
    }

    FimoInternalTracingCallStack *active_call_stack = local_data->active_call_stack;
    unsigned int call_stack_state = atomic_load_explicit(&active_call_stack->state, memory_order_relaxed);
    if (call_stack_state & ((unsigned int)FIMO_TRACING_CALL_STACK_SUSPENDED_BIT)) {
        return FIMO_EPERM;
    }

    if (active_call_stack->max_level < event->metadata->level) {
        return FIMO_EOK;
    }

    FimoUSize written_bytes = 0;
    FimoError error = format(local_data->format_buffer, ctx->tracing.format_buffer_size, data, &written_bytes);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    const FimoTime current_time = fimo_time_now();
    for (FimoUSize i = 0; i < tracing->subscriber_count; i++) {
        void *subscriber_ptr = tracing->subscribers[i].ptr;
        void *subscriber_call_stack = active_call_stack->subscriber_call_stacks[i];
        const char *formatted_msg = local_data->format_buffer;

        tracing->subscribers[i].vtable->event_emit(subscriber_ptr, &current_time, subscriber_call_stack, event,
                                                   formatted_msg, written_bytes);
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
bool fimo_internal_tracing_is_enabled(void *context) {
    if (context == NULL) {
        return false;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        return false;
    }

    const FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL) {
        return false;
    }

    return true;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_register_thread(void *context) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        return FIMO_EOK;
    }

    if (tss_get(tracing->thread_local_data)) {
        return FIMO_EPERM;
    }

    FimoInternalTracingThreadLocalData *local_data;
    const FimoError error = fimo_internal_tracing_local_data_create_(ctx, &local_data);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }
    tss_set(tracing->thread_local_data, local_data);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_unregister_thread(void *context) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;
    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF || tracing->subscriber_count == 0) {
        return FIMO_EOK;
    }

    FimoInternalTracingThreadLocalData *local_data = tss_get(tracing->thread_local_data);
    if (local_data == NULL || local_data->active_call_stack->end_frame != NULL) {
        return FIMO_EPERM;
    }

    fimo_internal_tracing_local_data_destroy_(local_data);
    tss_set(tracing->thread_local_data, NULL);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_internal_tracing_flush(void *context) {
    if (context == NULL) {
        return FIMO_EINVAL;
    }

    FimoInternalContext *ctx = context;
    FimoInternalContextTracing *tracing = &ctx->tracing;

    if (tracing->maximum_level == FIMO_TRACING_LEVEL_OFF) {
        return FIMO_EOK;
    }

    for (FimoUSize i = 0; i < tracing->subscriber_count; i++) {
        tracing->subscribers[i].vtable->flush(tracing->subscribers[i].ptr);
    }

    return FIMO_EOK;
}
