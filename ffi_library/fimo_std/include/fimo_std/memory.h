#ifndef FIMO_MEMORY_H
#define FIMO_MEMORY_H

#include <stddef.h>

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * An allocated buffer.
 */
typedef struct FimoMallocBuffer {
    void* ptr;
    size_t buff_size;
} FimoMallocBuffer;

/**
 * Allocate memory.
 *
 * This function allocates at least `size` bytes and returns a pointer to the allocated
 * memory. The memory is not initialized. If `size` is `0`, then @ref fimo_malloc()
 * returns `NULL`. If `error` is not a null pointer, @ref fimo_malloc() writes the
 * success status into the memory pointed to by `error`.
 *
 * @param size size of the allocation
 * @param error optional pointer to an error slot
 *
 * @return Pointer to the allocated memory.
 *
 * @error `FIMO_EOK`: The allocation was successful.
 * @error `FIMO_ENOMEM`: Out of memory.
 */
FIMO_MUST_USE void* fimo_malloc(size_t size, FimoError* error);

/**
 * Zero-allocate memory.
 *
 * This function allocates at least `size` bytes and returns a pointer to the allocated
 * memory. The memory is zero-initialized. If `size` is `0`, then @ref fimo_malloc()
 * returns `NULL`. If `error` is not a null pointer, @ref fimo_calloc() writes the
 * success status into the memory pointed to by `error`.
 *
 * @param size size of the allocation
 * @param error optional pointer to an error slot
 *
 * @return Pointer to the allocated memory.
 *
 * @error `FIMO_EOK`: The allocation was successful.
 * @error `FIMO_ENOMEM`: Out of memory.
 */
FIMO_MUST_USE void* fimo_calloc(size_t size, FimoError* error);

/**
 * Allocate memory.
 *
 * This function allocates at least `size` bytes and returns a pointer to the allocated
 * memory that is aligned at least as strictly as `alignment`. The memory is not initialized.
 * If `size` is `0`, then @ref fimo_aligned_alloc() returns `NULL` and `alignment` is ignored.
 * `alignment` must be a power of two greater than `0`. If `error` is not a null pointer,
 * @ref fimo_aligned_alloc() writes the success status into the memory pointed to by `error`.
 *
 * @param size: size of the allocation
 * @param alignment: alignment of the allocation
 * @param error: optional pointer to an error slot
 *
 * @return Pointer to the allocated memory.
 *
 * @error `FIMO_EOK`: The allocation was successful.
 * @error `FIMO_EINVAL`: Invalid alignment received.
 * @error `FIMO_ENOMEM`: Out of memory.
 */
FIMO_MUST_USE void* fimo_aligned_alloc(size_t alignment, size_t size, FimoError* error);

/**
 * Allocate memory.
 *
 * This function allocates at least `size` bytes and returns a pointer to the allocated
 * memory, along with the usable size in bytes. The memory is not initialized. If `size`
 * is `0`, then @ref fimo_malloc_sized() returns `NULL`. If `error` is not a null pointer,
 * @ref fimo_malloc_sized() writes the success status into the memory pointed to by `error`.
 *
 * @param size: size of the allocation
 * @param error: optional pointer to an error slot
 *
 * @return Pointer to the allocated memory and usable size in bytes.
 *
 * @error `FIMO_EOK`: The allocation was successful.
 * @error `FIMO_ENOMEM`: Out of memory.
 */
FIMO_MUST_USE FimoMallocBuffer fimo_malloc_sized(size_t size, FimoError* error);

/**
 * Zero-allocate memory.
 *
 * This function allocates at least `size` bytes and returns a pointer to the allocated
 * memory, along with the usable size in bytes. The memory is zero-initialized. If `size`
 * is `0`, then @ref fimo_calloc_sized() returns `NULL`. If `error` is not a null pointer,
 * @ref fimo_calloc_sized() writes the success status into the memory pointed to by `error`.
 *
 * @param size: size of the allocation
 * @param error: optional pointer to an error slot
 *
 * @return Pointer to the allocated memory and usable size in bytes.
 *
 * @error `FIMO_EOK`: The allocation was successful.
 * @error `FIMO_ENOMEM`: Out of memory.
 */
FIMO_MUST_USE FimoMallocBuffer fimo_calloc_sized(size_t size, FimoError* error);

/**
 * Allocate memory.
 *
 * This function allocates at least `size` bytes and returns a pointer to the allocated
 * memory that is aligned at least as strictly as `alignment`, along with the usable size
 * in bytes. The memory is not initialized. If `size` is `0`, then
 * @ref fimo_aligned_alloc_sized() returns `NULL` and `alignment` is ignored. `alignment`
 * must be a power of two greater than `0`. If `error` is not a null pointer,
 * @ref fimo_aligned_alloc_sized() writes the success status into the memory pointed to
 * by `error`.
 *
 * @param size: size of the allocation
 * @param alignment: alignment of the allocation
 * @param error: optional pointer to an error slot
 *
 * @return Pointer to the allocated memory and usable size in bytes.
 *
 * @error `FIMO_EOK`: The allocation was successful.
 * @error `FIMO_EINVAL`: Invalid alignment received.
 * @error `FIMO_ENOMEM`: Out of memory.
 */
FIMO_MUST_USE FimoMallocBuffer fimo_aligned_alloc_sized(size_t alignment, size_t size, FimoError* error);

/**
 * Free allocated memory.
 *
 * Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
 * no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
 * allocation function, or if the space has been deallocated by a call to @ref fimo_free(),
 * @ref fimo_free_sized() or @ref fimo_free_aligned_sized(), the behavior is undefined.
 *
 * @param ptr: pointer to the memory
 */
void fimo_free(void* ptr);

/**
 * Free allocated memory.
 *
 * Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
 * no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
 * allocation function, or if the space has been deallocated by a call to @ref fimo_free(),
 * @ref fimo_free_sized() or @ref fimo_free_aligned_sized(), or if `size` does not match
 * the size used to allocate the memory, the behavior is undefined.
 *
 * @param ptr: pointer to the memory
 * @param size: size of the allocation
 */
void fimo_free_sized(void* ptr, size_t size);

/**
 * Free allocated memory.
 *
 * Deallocates the memory allocated by an allocation function. If `ptr` is a null pointer,
 * no action shall occur. Otherwise, if `ptr` does not match a pointer returned by the
 * allocation function, or if the space has been deallocated by a call to @ref fimo_free(),
 * @ref fimo_free_sized() or @ref fimo_free_aligned_sized(), or if `alignment` and `size`
 * do not match the alignment and size used to allocate the memory, the behavior is undefined.
 *
 * @param ptr: pointer to the memory
 * @param alignment: alignment of the allocation
 * @param size: size of the allocation
 */
void fimo_free_aligned_sized(void* ptr, size_t alignment, size_t size);

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_MEMORY_H
