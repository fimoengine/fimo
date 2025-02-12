#ifndef FIMO_TASKS_META_POOL_H
#define FIMO_TASKS_META_POOL_H

#include <stdbool.h>

#include <fimo_std/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Unique identifier of a pool.
///
/// The identifier remains valid until the pool is destroyed.
typedef FimoUSize FimoTasksMeta_PoolId;

/// Identifier of a worker thread in a pool.
typedef FimoUSize FimoTasksMeta_PoolWorker;

/// A stack size.
typedef FimoUSize FimoTasksMeta_PoolStackSize;

typedef struct FimoTasksMeta_CommandBuffer FimoTasksMeta_CommandBuffer;
typedef struct FimoTasksMeta_CommandBufferHandle FimoTasksMeta_CommandBufferHandle;

/// VTable of a pool.
typedef struct FimoTasksMeta_PoolVTable {
    /// Returns the id of the pool.
    FimoTasksMeta_PoolId(*id)(void *pool);
    /// Acquires a new reference to the pool.
    void(*acquire)(void *pool);
    /// Releases the reference to the pool.
    void(*release)(void *pool);
    /// Sends a request to stop accepting new requests.
    void(*request_close)(void *pool);
    /// Checks if the pool accepts new requests.
    bool(*accepts_requests)(void *pool);
    /// Checks if the current thread is managed by the pool.
    bool(*owns_current_thread)(void *pool);
    /// Returns the optional label of the pool.
    ///
    /// The label is not null-terminated.
    const char*(*label)(void *pool, FimoUSize *len);
    /// Writes the ids of all workers managed by the pool into the provided array.
    ///
    /// The function writes up to `len` elements and returns the number of written elements.
    FimoUSize(*workers)(void *pool, FimoTasksMeta_PoolWorker *ptr, FimoUSize len);
    /// Writes all supported stack sizes of the pool into the provided array.
    ///
    /// The function writes up to `len` elements and returns the number of written elements.
    FimoUSize(*stack_sizes)(void *pool, FimoTasksMeta_PoolStackSize *ptr, FimoUSize len);
    /// Enqueues the command buffer in the pool.
    ///
    /// The buffer must remain valid until it is deinitialized by the pool. If `handle` is not
    /// `NULL`, it will be initialized with the handle of the enqueued buffer. Otherwise, the
    /// buffer will be enqueued in a detached state.
    FimoResult(*enqueue_buffer)(
        void *pool,
        FimoTasksMeta_CommandBuffer *buffer,
        FimoTasksMeta_CommandBufferHandle *handle
    );
} FimoTasksMeta_PoolVTable;

/// A worker pool.
typedef struct FimoTasksMeta_Pool {
    void *data;
    const FimoTasksMeta_PoolVTable *vtable;
} FimoTasksMeta_Pool;

/// Stack configuration for the worker pool.
typedef struct FimoTasksMeta_PoolConfigStackConfig {
    /// Reserved for future use.
    const void *next;
    /// Size of the stack allocation.
    FimoTasksMeta_PoolStackSize size;
    /// Number of stacks to allocate at pool creation time.
    FimoUSize preallocated_count;
    /// Number of cold stacks to keep allocated.
    FimoUSize cold_count;
    /// Number of hot stacks to keep allocated.
    FimoUSize hot_count;
    /// Maximum number of allocated stacks.
    ///
    /// A value of `0` indicates no upper limit.
    FimoUSize max_allocated;
} FimoTasksMeta_PoolConfigStackConfig;

/// Configuration for the creation of a new worker pool.
typedef struct FimoTasksMeta_PoolConfig {
    /// Reserved for future use.
    const void *next;
    /// Optional label of the worker pool.
    ///
    /// Is not null-terminated.
    const char *label;
    /// Length of the label.
    FimoUSize label_len;
    /// Configuration of the stack sizes provided by the pool.
    ///
    /// The runtime chooses the most restrictive stack config available when a stack is assigned to
    /// a new task. At least one stack config must be provided.
    const FimoTasksMeta_PoolConfigStackConfig *stacks;
    /// Number of stack configs.
    FimoUSize stacks_len;
    /// Index of the default stack configuration.
    FimoUSize default_stack_index;
    /// Number of worker threads to start.
    ///
    /// A value of `0` indicates to use the default number of workers, specified by the runtime.
    FimoUSize worker_count;
    /// Indicates whether to make the pool queryable. The pool can always be acquired through the
    /// pool id.
    bool is_queryable;
} FimoTasksMeta_PoolConfig;

/// Node of a pool query list.
typedef struct FimoTasksMeta_PoolQueryNode {
    FimoTasksMeta_Pool pool;
    struct FimoTasksMeta_PoolQueryNode *next;
} FimoTasksMeta_PoolQueryNode;

/// A query of the available worker pools.
///
/// The pool references are owned by the query and are released upon calling deinit.
typedef struct FimoTasksMeta_PoolQuery {
    FimoTasksMeta_PoolQueryNode *root;
    void(*destroy)(FimoTasksMeta_PoolQueryNode *root);
} FimoTasksMeta_PoolQuery;

/// Returns the id of the current worker.
typedef bool(*FimoTasksMeta_worker_id)(FimoTasksMeta_PoolWorker *id);

/// Returns the pool managing the current thread.
typedef bool(*FimoTasksMeta_worker_pool)(FimoTasksMeta_Pool *pool);

/// Acquires a reference to the worker pool with the provided id.
typedef bool(*FimoTasksMeta_worker_pool_by_id)(
    FimoTasksMeta_PoolId id,
    FimoTasksMeta_Pool *pool
);

/// Queries all public and active worker pools managed by the runtime.
typedef FimoResult(*FimoTasksMeta_query_worker_pools)(FimoTasksMeta_PoolQuery *query);

/// Creates a new worker pool with the specified configuration.
typedef FimoResult(*FimoTasksMeta_create_worker_pool)(
    const FimoTasksMeta_PoolConfig *config,
    FimoTasksMeta_Pool *pool
);

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_META_POOL_H
