#include <fimo_std/array_list.h>

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <fimo_std/memory.h>

#if defined(_WIN32) || defined(WIN32)
#include <intrin.h>
#endif

#if FIMO_HAS_BUILTIN(__builtin_add_overflow)
#define FIMO_ADD_OVERFLOW_(lhs, rhs, out) __builtin_add_overflow(lhs, rhs, out)
#elif defined(_WIN32) || defined(WIN32)
#define FIMO_ADD_OVERFLOW_(lhs, rhs, out) _addcarry_u64(0, lhs, rhs, out)
#else
#error "Unsupported compiler"
#endif

#if FIMO_HAS_BUILTIN(__builtin_mul_overflow)
#define FIMO_MUL_OVERFLOW_(lhs, rhs, out) __builtin_mul_overflow(lhs, rhs, out)
#elif defined(_WIN32) || defined(WIN32)
#define FIMO_MUL_OVERFLOW_(lhs, rhs, out) (_umul128(lhs, rhs, out), *out != 0)
#else
#error "Unsupported compiler"
#endif

#define MAX_BUFFER_SIZE_ SIZE_MAX >> 1

FIMO_MUST_USE
FimoArrayList fimo_array_list_new()
{
    return (FimoArrayList) {
        .elements = NULL,
        .size = 0,
        .capacity = 0,
    };
}

FIMO_MUST_USE
FimoError fimo_array_list_with_capacity(size_t capacity, size_t elem_size,
    FimoArrayList* array)
{
    capacity = fimo_next_power_of_two_u64(capacity);
    return fimo_array_list_with_capacity_exact(capacity, elem_size, array);
}

FIMO_MUST_USE
FimoError fimo_array_list_with_capacity_exact(size_t capacity, size_t elem_size,
    FimoArrayList* array)
{
    size_t buffer_size;
    if (FIMO_MUL_OVERFLOW_(capacity, elem_size, &buffer_size)) {
        return FIMO_ERANGE;
    }

    if (!array || buffer_size > MAX_BUFFER_SIZE_) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    array->elements = fimo_malloc(buffer_size, &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }
    array->capacity = capacity;
    array->size = 0;

    return FIMO_EOK;
}

void fimo_array_list_free(FimoArrayList* array, size_t elem_size)
{
    if (!array) {
        perror("invalid array pointer");
        exit(EXIT_FAILURE);
    }

    size_t buffer_size = array->capacity * elem_size;
    fimo_free_sized(array->elements, buffer_size);
}

FIMO_MUST_USE
FimoError fimo_array_list_reserve(FimoArrayList* array, size_t elem_size,
    size_t additional)
{
    if (!array) {
        return FIMO_EINVAL;
    }

    size_t new_size;
    if (FIMO_ADD_OVERFLOW_(additional, array->size, &new_size)) {
        return FIMO_ERANGE;
    }

    if (new_size <= array->capacity) {
        return FIMO_EOK;
    }
    return fimo_array_list_resize(array, elem_size, new_size);
}

FIMO_MUST_USE
FimoError fimo_array_list_reserve_exact(FimoArrayList* array, size_t elem_size,
    size_t additional)
{
    if (!array) {
        return FIMO_EINVAL;
    }

    size_t new_size;
    if (FIMO_ADD_OVERFLOW_(additional, array->size, &new_size)) {
        return FIMO_ERANGE;
    }

    if (new_size <= array->capacity) {
        return FIMO_EOK;
    }
    return fimo_array_list_resize_exact(array, elem_size, new_size);
}

FIMO_MUST_USE
FimoError fimo_array_list_resize(FimoArrayList* array, size_t elem_size,
    size_t capacity)
{
    capacity = fimo_next_power_of_two_u64(capacity);
    return fimo_array_list_resize_exact(array, elem_size, capacity);
}

FIMO_MUST_USE
FimoError fimo_array_list_resize_exact(FimoArrayList* array, size_t elem_size,
    size_t capacity)
{
    size_t buffer_size;
    if (FIMO_MUL_OVERFLOW_(capacity, elem_size, &buffer_size)) {
        return FIMO_ERANGE;
    }

    if (!array || buffer_size > MAX_BUFFER_SIZE_) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    void* elements = fimo_malloc(buffer_size, &error);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    size_t old_buffer_size = array->capacity * elem_size;
    size_t min_buffer_size = (buffer_size <= old_buffer_size)
        ? buffer_size
        : old_buffer_size;

    if (old_buffer_size > 0) {
        if (elements) {
            // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
            memcpy(elements, array->elements, min_buffer_size);
        }
        fimo_free_sized(array->elements, old_buffer_size);
    }
    array->capacity = capacity;
    array->elements = elements;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_array_list_set_len(FimoArrayList* array, size_t len)
{
    if (!array || len > array->capacity) {
        return FIMO_EINVAL;
    }

    array->size = len;
    return FIMO_EOK;
}

FIMO_MUST_USE
bool fimo_array_list_is_empty(const FimoArrayList* array)
{
    if (!array) {
        perror("invalid array list");
        exit(EXIT_FAILURE);
    }

    return array->size == 0;
}

FIMO_MUST_USE
size_t fimo_array_list_len(const FimoArrayList* array)
{
    if (!array) {
        perror("invalid array list");
        exit(EXIT_FAILURE);
    }

    return array->size;
}

FIMO_MUST_USE
size_t fimo_array_list_capacity(const FimoArrayList* array)
{
    if (!array) {
        perror("invalid array list");
        exit(EXIT_FAILURE);
    }

    return array->capacity;
}

FIMO_MUST_USE
FimoError fimo_array_list_peek_front(const FimoArrayList* array,
    size_t elem_size, const void** element)
{
    (void)elem_size;
    if (!array || array->size == 0 || !element) {
        return FIMO_EINVAL;
    }

    *element = array->elements;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_array_list_peek_back(const FimoArrayList* array,
    size_t elem_size, const void** element)
{
    if (!array || array->size == 0 || !element) {
        return FIMO_EINVAL;
    }

    size_t last_element_begin = (array->size - 1) * elem_size;
    *element = ((char*)array->elements) + last_element_begin;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_array_list_pop_front(FimoArrayList* array,
    size_t elem_size, void* element)
{
    if (!array || array->size == 0 || !element) {
        return FIMO_EINVAL;
    }

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memcpy(element, array->elements, elem_size);
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memmove(array->elements, array->elements, elem_size * (--array->size));

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_array_list_pop_back(FimoArrayList* array,
    size_t elem_size, void* element)
{
    if (!array || array->size == 0 || !element) {
        return FIMO_EINVAL;
    }

    array->size--;
    size_t last_element_begin = array->size * elem_size;
    void* last_element = ((char*)array->elements) + last_element_begin;

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memcpy(element, last_element, elem_size);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_array_list_get(const FimoArrayList* array, size_t index,
    size_t elem_size, const void** element)
{
    if (!array || array->size <= index || !element) {
        return FIMO_EINVAL;
    }

    size_t element_begin = index * elem_size;
    *element = ((char*)array->elements) + element_begin;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_array_list_push(FimoArrayList* array, size_t elem_size,
    const void* element)
{
    if (!array) {
        return FIMO_EINVAL;
    }

    return fimo_array_list_insert(array, array->size, elem_size, element);
}

FIMO_MUST_USE
FimoError fimo_array_list_try_push(FimoArrayList* array, size_t elem_size,
    const void* element)
{
    if (!array) {
        return FIMO_EINVAL;
    }

    return fimo_array_list_try_insert(array, array->size, elem_size, element);
}

FIMO_MUST_USE
FimoError fimo_array_list_insert(FimoArrayList* array, size_t index,
    size_t elem_size, const void* element)
{
    if (!array || array->size < index || !element) {
        return FIMO_EINVAL;
    }

    if (array->size == array->capacity) {
        FimoError error = fimo_array_list_reserve(array, elem_size, 1);
        if (FIMO_IS_ERROR(error)) {
            return error;
        }
    }

    return fimo_array_list_try_insert(array, index, elem_size, element);
}

FIMO_MUST_USE
FimoError fimo_array_list_try_insert(FimoArrayList* array, size_t index,
    size_t elem_size, const void* element)
{
    if (!array || array->size < index
        || array->capacity == array->size || !element) {
        return FIMO_EINVAL;
    }

    size_t elements_to_shift = array->size - index;
    array->size++;

    size_t element_begin = index * elem_size;
    void* element_ptr = ((char*)array->elements) + element_begin;
    void* shift_ptr = ((char*)element_ptr) + elem_size;

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memmove(shift_ptr, element_ptr, elements_to_shift * elem_size);

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memcpy(element_ptr, element, elem_size);

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_array_list_remove(FimoArrayList* array, size_t index,
    size_t elem_size, void* element)
{
    if (!array || array->size <= index || !element) {
        return FIMO_EINVAL;
    }

    array->size--;
    size_t elements_to_shift = array->size - index;

    size_t element_begin = index * elem_size;
    void* element_ptr = ((char*)array->elements) + element_begin;
    void* shift_ptr = ((char*)element_ptr) + elem_size;

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memcpy(element, element_ptr, elem_size);

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memmove(element_ptr, shift_ptr, elements_to_shift * elem_size);

    return FIMO_EOK;
}
