#ifndef FIMO_WORLDS_META_SYSTEMS_H
#define FIMO_WORLDS_META_SYSTEMS_H

#include <stdbool.h>

#include <fimo_std/fimo.h>
#include <fimo_tasks_meta/package.h>

#include <fimo_worlds_meta/errors.h>
#include <fimo_worlds_meta/jobs.h>
#include <fimo_worlds_meta/resources.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FimoWorldsMeta_World FimoWorldsMeta_World;

/// A unique identifier for a registered system.
typedef FimoUSize FimoWorldsMeta_SystemId;

/// A group of systems that can be scheduled together.
typedef struct FimoWorldsMeta_SystemGroup FimoWorldsMeta_SystemGroup;

/// Context of an instantiated system in a system group.
typedef struct FimoWorldsMeta_SystemContext FimoWorldsMeta_SystemContext;

/// Descriptor of a system dependency.
typedef struct FimoWorldsMeta_SystemDependency {
    /// System to depend on / be depended from.
    FimoWorldsMeta_SystemId system;
    /// Whether to ignore any deferred subjob of the system.
    ///
    /// If set to `true`, the system will start after the other systems `run`
    /// function is run to completion. Otherwise, the system will start after
    /// all subjobs of the system also complete their execution.
    bool ignore_deferred;
} FimoWorldsMeta_SystemDependency;

/// Descriptor of a new system.
typedef struct FimoWorldsMeta_SystemDescriptor {
    /// Reserved. Must be null.
    const void *next;
    /// Optional label of the system.
    const char *label;
    /// Length in characters of the system label.
    FimoUSize label_len;
    /// Optional array of resources to require with exclusive access.
    const FimoWorldsMeta_ResourceId *exclusive_ids;
    /// Length of the `exclusive_ids` array.
    FimoUSize exclusive_ids_len;
    /// Optional array of resources to require with shared access.
    const FimoWorldsMeta_ResourceId *shared_ids;
    /// Length of the `shared_ids` array.
    FimoUSize shared_ids_len;
    /// Optional array of systems to depend on.
    ///
    /// The system will start executing after all systems have been executed.
    const FimoWorldsMeta_SystemDependency *before;
    /// Length of the `before` array.
    FimoUSize before_len;
    /// Optional array of systems to be depended from.
    ///
    /// The systems will start executing after the new system completes its execution.
    const FimoWorldsMeta_SystemDependency *after;
    /// Length of the `after` array.
    FimoUSize after_len;

    /// Pointer to the factory for the system.
    ///
    /// The factory will be copied into the universe.
    const void *factory;
    /// Size in bytes of the factory.
    FimoUSize factory_size;
    /// Alignment in bytes of the factory. Must be a power-of-two.
    FimoUSize factory_alignment;
    /// Optional function to call when destroying the factory.
    void(*factory_destroy)(void *factory);

    /// Size in bytes of the system state.
    FimoUSize system_size;
    /// Alignment in bytes of the system state. Must be a power-of-two.
    FimoUSize system_alignment;
    /// Function called when instantiating a new system.
    ///
    /// The system is provided with a system context, that shares the same lifetime,
    /// as the system itself. The context provides additional utilities, like allocators.
    /// The state of the system must be written into the provided `system` pointer.
    /// On success, the function must return true.
    bool(*system_create)(const void *factory, FimoWorldsMeta_SystemContext *context, void *system);
    /// Optional function to call when destroying a system.
    void(*system_destroy)(void *system);
    /// Function called on each system run.
    ///
    /// The requested exclusive and shared resources are provided in the order defined by
    /// the `exclusive_ids` and `shared_ids`. Additionally, the system is provided with a
    /// pointer to an unsignaled fence. The fence may be used to spawn additional jobs from
    /// within the run function and synchronize other systems waiting on the completion of
    /// the current system. The system must signal the fence after it has completed. Failure
    /// of doing such will lead to a deadlock. The fence is guaranteed to not have any waiters
    /// until after the run function returns.
    void(*system_run)(
        void *system,
        void *const *exclusive_resources,
        void *const *shared_resources,
        FimoWorldsMeta_Fence *fence
    );
} FimoWorldsMeta_SystemDescriptor;

/// Descriptor a a new system group.
typedef struct FimoWorldsMeta_SystemGroupDescriptor {
    /// Reserved. Must be null.
    const void *next;
    /// Optional label of the system group.
    const char *label;
    /// Length in characters of the system group label.
    FimoUSize label_len;
    /// Optional executor for the system group.
    ///
    /// A null value will inherit the executor of the world.
    /// If the value is not null, the system group will increase its reference count.
    const FimoTasksMeta_Pool *pool;
    /// World to add the group to.
    FimoWorldsMeta_World *world;
} FimoWorldsMeta_SystemGroupDescriptor;

/// Known allocator strategies for a system.
typedef enum FimoWorldsMeta_SystemAllocatorStrategy : FimoI32 {
    /// An allocator that is invalidated after the system has finished executing.
    ///
    /// The memory returned by this allocator is only valid in the scope of the run function of the
    /// system for the current group generation. The allocator is not thread-safe.
    FIMO_WORLDS_META_SYSTEM_ALLOCATOR_STRATEGY_TRANSIENT,
    /// An allocator that is invalidated at the end of the current system group generation.
    ///
    /// The allocator may be utilized to spawn short lived tasks from the system, or to pass
    /// data to systems executing after the current one.
    FIMO_WORLDS_META_SYSTEM_ALLOCATOR_STRATEGY_SINGLE_GENERATION,
    /// An allocator that is invalidated after four generations.
    ///
    /// The allocator may be utilized to spawn medium-to-short lived tasks from the system, or
    /// to pass data to the systems executing in the next generations.
    FIMO_WORLDS_META_SYSTEM_ALLOCATOR_STRATEGY_MULTI_GENERATION,
    /// An allocator that is invalidated with the system.
    ///
    /// May be utilized for long-lived/persistent allocations.
    FIMO_WORLDS_META_SYSTEM_ALLOCATOR_STRATEGY_SYSTEM_PERSISTENT,
} FimoWorldsMeta_SystemAllocatorStrategy;

/// Registers a new system with the universe.
///
/// Registered resources may be added to system group of any world.
typedef FimoWorldsMeta_Error(*FimoWorldsMeta_system_register)(
    const FimoWorldsMeta_SystemDescriptor *system,
    FimoWorldsMeta_SystemId *id
);

/// Unregisters the system from the universe.
///
/// Once unregistered, the identifier is invalidated and may be reused by another system.
/// The system must not be used explicitly by any world when this method is called.
typedef void(*FimoWorldsMeta_system_unregister)(FimoWorldsMeta_SystemId id);

/// Initializes a new empty system group.
typedef FimoWorldsMeta_Error(*FimoWorldsMeta_system_group_create)(
    const FimoWorldsMeta_SystemGroupDescriptor *descriptor,
    FimoWorldsMeta_SystemGroup **group
);

/// Destroys the system group.
///
/// The caller may provide a reference to a fence via `signal`, to be notified when the group
/// has been destroyed. If no fence is provided, the caller will block until the group is
/// destroyed. Scheduled operations will be executed.
typedef void(*FimoWorldsMeta_system_group_destroy)(
    FimoWorldsMeta_SystemGroup *group,
    FimoWorldsMeta_Fence *signal
);

/// Returns the world the group is contained in.
typedef FimoWorldsMeta_World(*FimoWorldsMeta_system_group_get_world)(
    FimoWorldsMeta_SystemGroup *group
);

/// Returns the label of the system group.
typedef const char*(*FimoWorldsMeta_system_group_get_label)(
    FimoWorldsMeta_SystemGroup *group,
    FimoUSize *len
);

/// Returns a reference to the executor used by the group.
typedef FimoTasksMeta_Pool(*FimoWorldsMeta_system_group_get_pool)(
    FimoWorldsMeta_SystemGroup *group
);

/// Adds a set of systems to the group.
///
/// Already scheduled operations are not affected by the added systems.
/// The operation may add systems transitively, if the systems specify an execution order.
typedef FimoWorldsMeta_Error(*FimoWorldsMeta_system_group_add_systems)(
    FimoWorldsMeta_SystemGroup *group,
    const FimoWorldsMeta_SystemId *systems,
    FimoUSize systems_len
);

/// Removes a system from the group.
///
/// Already scheduled systems will not be affected.
/// This operation may remove systems added transitively. The caller may provide a reference to
/// a fence via `signal`, to be notified when the system has been removed from the group.
typedef void(*FimoWorldsMeta_system_group_remove_system)(
    FimoWorldsMeta_SystemGroup *group,
    FimoWorldsMeta_SystemId id,
    FimoWorldsMeta_Fence *signal
);

/// Schedules to run all systems contained in the group.
///
/// The group will start executing after all fences in `wait_on` are signaled.
/// The caller may provide a reference to a fence via `signal`, to be notified when the group
/// has finished executing all systems.
///
/// Each schedule operation is assigned to one generation of the system group, which is an index
/// that is increased by one each time the group finishes executing all systems. Multiple generations
/// are run sequentially.
typedef FimoWorldsMeta_Error(*FimoWorldsMeta_system_group_schedule)(
    FimoWorldsMeta_SystemGroup *group,
    FimoWorldsMeta_Fence *const *wait_on,
    FimoUSize wait_on_len,
    FimoWorldsMeta_Fence *signal
);

/// Returns the group the system is contained in.
typedef FimoWorldsMeta_SystemGroup*(*FimoWorldsMeta_system_context_get_group)(
    FimoWorldsMeta_SystemContext *context
);

/// Returns the current generation of system group.
///
/// The generation is increased by one each time the group finishes executing all systems.
typedef FimoUSize(*FimoWorldsMeta_system_context_get_generation)(
    FimoWorldsMeta_SystemContext *context
);

/// Allocates a new buffer using the specified allocation strategy.
///
/// The buffer has a size of `size` and is aligned to `alignment`.
/// `ret_addr` is optionally provided as the first return address of the allocation call stack.
/// If the value is 0 it means no return address has been provided.
typedef void*(*FimoWorldsMeta_system_context_allocator_alloc)(
    FimoWorldsMeta_SystemContext *context,
    FimoWorldsMeta_SystemAllocatorStrategy strategy,
    FimoUSize size,
    FimoUSize alignment,
    FimoUSize ret_addr
);

/// Attempt to expand or shrink the memory in place.
///
/// `strategy` and `alignment` must equal the same value used to allocate the buffer.
/// `size` must equal the size requested from the most recent `alloc`, `resize` or `remap`.
/// A result of `true` indicates the resize was successful and the allocation now has the same
/// address but a size of `new_size`. `ret_addr` is optionally provided as the first return address
/// of the allocation call stack. If the value is 0 it means no return address has been provided.
typedef bool(*FimoWorldsMeta_system_context_allocator_resize)(
    FimoWorldsMeta_SystemContext *context,
    FimoWorldsMeta_SystemAllocatorStrategy strategy,
    void *ptr,
    FimoUSize size,
    FimoUSize alignment,
    FimoUSize new_size,
    FimoUSize ret_addr
);

/// Attempt to expand or shrink memory, allowing relocation.
///
/// `strategy` and `alignment` must equal the same value used to allocate the buffer.
/// `size` must equal the size requested from the most recent `alloc`, `resize` or `remap`.
/// `ret_addr` is optionally provided as the first return address of the allocation call stack.
/// If the value is 0 it means no return address has been provided.
typedef void*(*FimoWorldsMeta_system_context_allocator_remap)(
    FimoWorldsMeta_SystemContext *context,
    FimoWorldsMeta_SystemAllocatorStrategy strategy,
    void *ptr,
    FimoUSize size,
    FimoUSize alignment,
    FimoUSize new_size,
    FimoUSize ret_addr
);

/// Free and invalidate a region of memory.
///
/// `strategy` and `alignment` must equal the same value used to allocate the buffer.
/// `size` must equal the size requested from the most recent `alloc`, `resize` or `remap`.
/// `ret_addr` is optionally provided as the first return address of the allocation call stack.
/// If the value is 0 it means no return address has been provided.
typedef void*(*FimoWorldsMeta_system_context_allocator_free)(
    FimoWorldsMeta_SystemContext *context,
    FimoWorldsMeta_SystemAllocatorStrategy strategy,
    void *ptr,
    FimoUSize size,
    FimoUSize alignment,
    FimoUSize ret_addr
);

#ifdef __cplusplus
}
#endif

#endif // FIMO_WORLDS_META_SYSTEMS_H
