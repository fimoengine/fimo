#ifndef FIMO_WORLDS_META_WORLDS_H
#define FIMO_WORLDS_META_WORLDS_H

#include <stdbool.h>

#include <fimo_std.h>
#include <fimo_tasks_meta/package.h>

#include <fimo_worlds_meta/resources.h>
#include <fimo_worlds_meta/systems.h>

#ifdef __cplusplus
extern "C" {
#endif

/// A container for resources and scheduable systems.
typedef struct FimoWorldsMeta_World FimoWorldsMeta_World;

/// Descriptor of a new world.
typedef struct FimoWorldsMeta_WorldDescriptor {
    /// Reserved. Must be null.
    const void *next;
    /// Optional label of the world.
    const char *label;
    /// Length in characters of the world label.
    FSTD_USize label_len;
    /// Executor for the world.
    ///
    /// If this value is `null`, the world will spawn a default executor.
    /// If the value is not null, the world will increase its reference count.
    const FimoTasksMeta_Pool *pool;
} FimoWorldsMeta_WorldDescriptor;

/// Initializes a new empty world.
typedef FSTD_Status (*FimoWorldsMeta_world_create)(const FimoWorldsMeta_WorldDescriptor *descriptor,
                                                   FimoWorldsMeta_World **world);

/// Destroys the world.
///
/// The world must be empty.
typedef void (*FimoWorldsMeta_world_destroy)(FimoWorldsMeta_World *world);

/// Returns the label of the world.
typedef const char *(*FimoWorldsMeta_world_get_label)(FimoWorldsMeta_World *world, FSTD_USize *len);

/// Returns a reference to the executor used by the world.
typedef FimoTasksMeta_Pool (*FimoWorldsMeta_world_get_pool)(FimoWorldsMeta_World *world);

/// Checks if the resource is instantiated in the world.
typedef bool (*FimoWorldsMeta_world_has_resource)(FimoWorldsMeta_World *world, FimoWorldsMeta_ResourceHandle handle);

/// Adds the resource to the world.
typedef FSTD_Status (*FimoWorldsMeta_world_add_resource)(FimoWorldsMeta_World *world,
                                                         FimoWorldsMeta_ResourceHandle handle, const void *value);

/// Removes the resource from the world.
typedef FSTD_Status (*FimoWorldsMeta_world_remove_resource)(FimoWorldsMeta_World *world,
                                                            FimoWorldsMeta_ResourceHandle handle, void *value);

/// Acquires a set of exclusive and shared resource references.
///
/// The pointers to the resources are written into `out_resources`, where the indices
/// `0..exclusive_handles_len` contain the resources in the `exclusive_handles` list, while the
/// indices `exclusive_handles_len..exclusive_handles_len+shared_handles_len` contain the remaining
/// resources from the `shared_handles` list.
///
/// The locks to the resources are acquired in ascending resource handle order.
/// The caller will block until all resources are locked.
typedef void(FimoWorldsMeta_world_lock_resources)(FimoWorldsMeta_World *world,
                                                  const FimoWorldsMeta_ResourceHandle *exclusive_handles,
                                                  FSTD_USize exclusive_handles_len,
                                                  const FimoWorldsMeta_ResourceHandle *shared_handles,
                                                  FSTD_USize shared_handles_len, void **resources);

/// Unlocks an exclusive resource lock.
typedef void (*FimoWorldsMeta_world_unlock_resource_exclusive)(FimoWorldsMeta_World *world,
                                                               FimoWorldsMeta_ResourceHandle handle);

/// Unlocks a shared resource lock.
typedef void (*FimoWorldsMeta_world_unlock_resource_shared)(FimoWorldsMeta_World *world,
                                                            FimoWorldsMeta_ResourceHandle handle);

/// Allocates a new buffer.
///
/// The buffer has a size of `size` and is aligned to `alignment`.
/// `ret_addr` is optionally provided as the first return address of the allocation call stack.
/// If the value is 0 it means no return address has been provided.
typedef void *(*FimoWorldsMeta_world_allocator_alloc)(FimoWorldsMeta_World *world, FSTD_USize size,
                                                      FSTD_USize alignment, FSTD_USize ret_addr);

/// Attempt to expand or shrink the memory in place.
///
/// `alignment` must equal the same value used to allocate the buffer.
/// `size` must equal the size requested from the most recent `alloc`, `resize` or `remap`.
/// A result of `true` indicates the resize was successful and the allocation now has the same
/// address but a size of `new_size`. `ret_addr` is optionally provided as the first return address
/// of the allocation call stack. If the value is 0 it means no return address has been provided.
typedef bool (*FimoWorldsMeta_world_allocator_resize)(FimoWorldsMeta_World *world, void *ptr, FSTD_USize size,
                                                      FSTD_USize alignment, FSTD_USize new_size, FSTD_USize ret_addr);

/// Attempt to expand or shrink memory, allowing relocation.
///
/// `alignment` must equal the same value used to allocate the buffer.
/// `size` must equal the size requested from the most recent `alloc`, `resize` or `remap`.
/// `ret_addr` is optionally provided as the first return address of the allocation call stack.
/// If the value is 0 it means no return address has been provided.
typedef void *(*FimoWorldsMeta_world_allocator_remap)(FimoWorldsMeta_World *world, void *ptr, FSTD_USize size,
                                                      FSTD_USize alignment, FSTD_USize new_size, FSTD_USize ret_addr);

/// Free and invalidate a region of memory.
///
/// `alignment` must equal the same value used to allocate the buffer.
/// `size` must equal the size requested from the most recent `alloc`, `resize` or `remap`.
/// `ret_addr` is optionally provided as the first return address of the allocation call stack.
/// If the value is 0 it means no return address has been provided.
typedef void *(*FimoWorldsMeta_world_allocator_free)(FimoWorldsMeta_World *world, void *ptr, FSTD_USize size,
                                                     FSTD_USize alignment, FSTD_USize ret_addr);

#ifdef __cplusplus
}
#endif

#endif // FIMO_WORLDS_META_WORLDS_H
