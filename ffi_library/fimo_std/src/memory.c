#include <fimo_std/memory.h>

#include <errno.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32) || defined(WIN32)
#include <malloc.h>
#elif __APPLE__
#include <malloc/malloc.h>
#elif __ANDROID__
#include <malloc.h>
#elif __linux__
#include <malloc.h>
#endif // defined(_WIN32) || defined(WIN32)

#if defined(_WIN32) || defined(WIN32)
#define FIMO_MALLOC_ALIGNMENT 16
#else
#define FIMO_MALLOC_ALIGNMENT _Alignof(max_align_t)
#endif // defined(_WIN32) || defined(WIN32)

FIMO_MUST_USE void* fimo_malloc(size_t size, FimoError* error)
{
    return fimo_malloc_sized(size, error).ptr;
}

FIMO_MUST_USE void* fimo_calloc(size_t size, FimoError* error)
{
    return fimo_calloc_sized(size, error).ptr;
}

FIMO_MUST_USE void* fimo_aligned_alloc(size_t alignment, size_t size, FimoError* error)
{
    return fimo_aligned_alloc_sized(alignment, size, error).ptr;
}

FIMO_MUST_USE FimoMallocBuffer fimo_malloc_sized(size_t size, FimoError* error)
{
    return fimo_aligned_alloc_sized(FIMO_MALLOC_ALIGNMENT, size, error);
}

FIMO_MUST_USE FimoMallocBuffer fimo_calloc_sized(size_t size, FimoError* error)
{
    FimoMallocBuffer buffer = fimo_malloc_sized(size, error);
    if (!buffer.ptr) {
        return buffer;
    }

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memset(buffer.ptr, 0, buffer.buff_size);
    return buffer;
}

FIMO_MUST_USE FimoMallocBuffer fimo_aligned_alloc_sized(size_t alignment, size_t size, FimoError* error)
{
    if (size == 0 || alignment == 0 || ((alignment & (alignment - 1)) != 0)) {
        if (error) {
            *error = size == 0 ? FIMO_EOK : FIMO_EINVAL;
        }
        return (FimoMallocBuffer) { .ptr = NULL, .buff_size = 0 };
    }

#if !defined(_WIN32) && !defined(WIN32)
    // Alignment must be smaller or a multiple of sizeof(void*)
    if (alignment > sizeof(void*)) {
        alignment = (alignment + (sizeof(void*) - 1)) & ~(sizeof(void*) - 1);
    }
#endif // !defined(_WIN32) && !defined(WIN32)

    // Align to the alignment.
    size = (size + (alignment - 1)) & ~(alignment - 1);

    void* ptr;
    size_t buff_size;

#if defined(_WIN32) || defined(WIN32)
    ptr = _aligned_malloc(size, alignment);
#else
    ptr = aligned_alloc(alignment, size);
#endif // defined(_WIN32) || defined(WIN32)
    if (!ptr) {
        if (error) {
            *error = fimo_error_from_errno(errno);
        }
        return (FimoMallocBuffer) { .ptr = NULL, .buff_size = 0 };
    }

#if defined(_WIN32) || defined(WIN32)
    buff_size = _aligned_msize(ptr, alignment, 0);
    if (buff_size == (size_t)-1) {
        if (error) {
            *error = fimo_error_from_errno(errno);
        }
        _aligned_free(ptr);
        return (FimoMallocBuffer) { .ptr = NULL, .buff_size = 0 };
    }
#elif __APPLE__
    buff_size = malloc_size(ptr);
#elif __ANDROID__
    buff_size = malloc_usable_size(ptr);
#elif __linux__
    buff_size = malloc_usable_size(ptr);
#else
    buff_size = size;
#endif // defined(_WIN32) || defined(WIN32)

    if (error) {
        *error = FIMO_EOK;
    }
    return (FimoMallocBuffer) { .ptr = ptr, .buff_size = buff_size };
}

void fimo_free(void* ptr)
{
#if defined(_WIN32) || defined(WIN32)
    _aligned_free(ptr);
#else
    free(ptr);
#endif // defined(_WIN32) || defined(WIN32)
}

void fimo_free_sized(void* ptr, size_t size)
{
    (void)size;
    fimo_free(ptr);
}

void fimo_free_aligned_sized(void* ptr, size_t alignment, size_t size)
{
    (void)alignment;
    (void)size;
    fimo_free(ptr);
}
