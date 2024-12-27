#ifndef FIMO_ASYNC_H
#define FIMO_ASYNC_H

#include <stdalign.h>
#include <stddef.h>

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * VTable of a FimoAsyncEventLoop.
 *
 * Changing the VTable is not a breaking change.
 */
typedef struct FimoAsyncEventLoopVTable {
    /**
     * Blocks the calling thread until the event loop queue is empty.
     *
     * Signals the event loop thread to finish processing the remaining
     * tasks. The tasks may continue to enqueue new tasks. The event loop
     * may not be utilized after calling this function.
     */
    void (*join)(void *data);
    /**
     * Signals the event loop to complete the remaining tasks and exit afterwards.
     *
     * Signals the event loop thread to finish processing the remaining
     * tasks. The tasks may continue to enqueue new tasks. The caller will return
     * immediately, after which it is not allowed to access the event loop.
     */
    void (*detach)(void *data);
} FimoAsyncEventLoopVTable;

/**
 * A handle to a running event loop.
 *
 * The event loop must either be joined or detached.
 */
typedef struct FimoAsyncEventLoop {
    void *data;
    const FimoAsyncEventLoopVTable *vtable;
} FimoAsyncEventLoop;

/**
 * VTable of a FimoAsyncWaker.
 *
 * Changing the VTable is a breaking change.
 */
typedef struct FimoAsyncWakerVTableV0 FimoAsyncWakerVTableV0;

/**
 * A waker for asynchronous tasks.
 *
 * Wakers are the main building block of the async runtime, where
 * their main job is signaling that a task may make progress and may
 * therefore be polled again. A task is allowed to assume, that no
 * progress can be made, if its waker is not signaled.
 */
typedef struct FimoAsyncWaker {
    void *data;
    const FimoAsyncWakerVTableV0 *vtable;
} FimoAsyncWaker;

struct FimoAsyncWakerVTableV0 {
    /**
     * Increases the reference count of the waker.
     */
    FimoAsyncWaker (*acquire)(void *data);
    /**
     * Decreases the reference count of the waker.
     */
    void (*release)(void *data);
    /**
     * Signals the task bound to the waker and decreases the reference count.
     */
    void (*wake_release)(void *data);
    /**
     * Signals the task bound to the waker without decreasing the reference count.
     */
    void (*wake)(void *data);
    /**
     * Reserved for future extensions.
     */
    const void *next;
};

/**
 * VTable of a FimoAsyncBlockingContext.
 *
 * Changing the VTable is not a breaking change.
 */
typedef struct FimoAsyncBlockingContextVTable {
    /**
     * Releases the context.
     */
    void (*release)(void *data);
    /**
     * Returns a non-owning reference to the waker for this context.
     *
     * The waker will unblock the thread once it has been notified.
     */
    FimoAsyncWaker (*waker_ref)(void *data);
    /**
     * Blocks the current thread until it is notified by the waker.
     */
    void (*block_until_notified)(void *data);
} FimoAsyncBlockingContextVTable;

/**
 * A context that blocks the current thread until it is notified.
 *
 * The context is intended to be used by threads other than the
 * event loop thread, as they are not bound to a waker. Using this
 * context inside the event loop will result in a deadlock.
 */
typedef struct FimoAsyncBlockingContext {
    void *data;
    const FimoAsyncBlockingContextVTable *vtable;
} FimoAsyncBlockingContext;

/**
 * Defines the type of a future with the specified state and return types.
 *
 * Futures follow a simple execution model. Each future consists of three
 * main components. A state, a function to poll the future, and an optional
 * cleanup function.
 *
 * The poll function takes a pointer to the state and tries to make some
 * progress. The future may not progress if is not polled. The function
 * must either return `false`, signaling that the future has not yet been
 * completed, or return `true` and write its result in the provided pointer.
 *
 * The second parameter of the poll function is a waker for the calling
 * task. The waker is not owned by the callee, and it may not release it
 * without first acquiring it. If the poll function signals a pending
 * future, the caller is allowed to put itself in a suspended state until
 * it is notified by the waker. It is the responsibility of the poll function
 * to notify the caller through the waker, once further progress can be made.
 * Failure of doing so may result in a deadlock.
 *
 * Polling a completed future will result in undefined behavior. The future
 * may not be moved once it has been polled, as its state may be self-referential.
 *
 * @param T state type
 * @param R result type
 */
#define FIMO_ASYNC_FUTURE(T, R)                                 \
    struct {                                                    \
        T data;                                                 \
        bool (*poll)(T *data, FimoAsyncWaker waker, R *result); \
        void (*release)(T *data);                               \
    }

/**
 * Defines the type of an enqueued future with the specified return type.
 *
 * @param R result type
 */
#define FIMO_ASYNC_ENQUEUED_FUTURE(R) FIMO_ASYNC_FUTURE(void*, R)

/**
 * Defines a pair of a FimoResult and a T.
 */
#define FIMO_ASYNC_FALLIBLE(T)    \
    struct {                      \
        FimoResult result;        \
        T value;                  \
    }

/**
 * An enqueued future with an unknown result type.
 */
typedef FIMO_ASYNC_ENQUEUED_FUTURE(void) FimoAsyncOpaqueFuture;

/**
 * VTable of the async subsystem.
 *
 * Changing the VTable is a breaking change.
 */
typedef struct FimoAsyncVTableV0 {
    /**
     * Utilize the current thread to complete all tasks in the event loop.
     *
     * The intended purpose of this function is to complete all remaining tasks
     * before cleanup, as the context can not be destroyed until the queue is empty.
     * Upon the completion of all tasks, the funtion will return to the caller.
     *
     * @param ctx context
     */
    FimoResult (*run_to_completion)(void *ctx);
    /**
     * Initializes a new event loop.
     *
     * There can only be one event loop at a time, and it will keep
     * the context alive until it completes its execution.
     *
     * @param ctx context
     * @param loop constructed event loop
     */
    FimoResult (*start_event_loop)(void *ctx, FimoAsyncEventLoop *loop);
    /**
     * Initializes a new blocking context.
     *
     * The context provides the utilities required to await the completion
     * of a future, by blocking a waiting thread and providing a waker
     * to resume it.
     *
     * @param ctx context
     * @param context constructed context
     */
    FimoResult (*context_new_blocking)(void *ctx, FimoAsyncBlockingContext *context);
    /**
     * Enqueues a new custom future to the event loop.
     *
     * Unlike normal futures, enqueues futures may be polled immediately.
     * The future will allocate a new internal buffer to store the future
     * state and its eventual result value. The state will be copied into
     * the new buffer via a memcpy. Polling the returned future will either
     * register the calling task as a waiter, which will be notified upon
     * the completion of the future, or copy the result into the provided
     * pointer via a memcpy. The state of the future must be relocatable
     * to other threads. Releasing the constructed future does not abort
     * it. If such feature is desired, it must be implemented by the caller.
     * The caller is allowed to provide two optional cleanup functions, one
     * for the state of the future, and one for the result value. The former
     * will be called unconditionally at an appropiate time, whereas the result
     * will only be cleaned up if the caller releases the constructed future
     * before polling it to completion.
     *
     * @param ctx context
     * @param data future state to copy
     * @param data_size size in bytes of the future state
     * @param data_alignment alignment of the future state
     * @param result_size size in bytes of the future result
     * @param result_alignment alignment of the future result
     * @param poll future poll function
     * @param release_data optional cleanup function for the future state
     * @param release_result optional cleanup function for the future result
     * @param enqueued_future pollable handle to the enqueued future
     */
    FimoResult (*future_enqueue)(void *ctx, const void *data, FimoUSize data_size, FimoUSize data_alignment,
                                 FimoUSize result_size, FimoUSize result_alignment,
                                 bool (*poll)(void *data, FimoAsyncWaker waker, void *result),
                                 void (*release_data)(void *data), void (*release_result)(void *data),
                                 FimoAsyncOpaqueFuture *enqueued_future);
} FimoAsyncVTableV0;

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_ASYNC_H
