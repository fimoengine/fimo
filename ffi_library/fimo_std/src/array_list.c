#include <fimo_std/array_list.h>

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <fimo_std/memory.h>

#define MAX_CAPACITY_ FIMO_ISIZE_MAX

FIMO_EXPORT
FIMO_MUST_USE
FimoArrayList fimo_array_list_new(void) {
    return (FimoArrayList){
            .elements = NULL,
            .size = 0,
            .capacity = 0,
    };
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_with_capacity(FimoUSize capacity, const FimoUSize elem_size, const FimoUSize elem_align,
                                         FimoArrayList *array) {
    FIMO_DEBUG_ASSERT(array)
    capacity = fimo_usize_next_power_of_two(capacity);
    return fimo_array_list_with_capacity_exact(capacity, elem_size, elem_align, array);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_with_capacity_exact(const FimoUSize capacity, const FimoUSize elem_size,
                                               const FimoUSize elem_align, FimoArrayList *array) {
    FIMO_DEBUG_ASSERT(array)
    if (capacity > MAX_CAPACITY_) {
        return FIMO_EINVAL;
    }

    const FimoIntOverflowCheckUSize tmp = fimo_usize_overflowing_mul(capacity, elem_size);
    if (tmp.overflow) {
        return FIMO_ERANGE;
    }
    const FimoUSize buffer_size = tmp.value;

    FimoResult error = FIMO_EOK;
    array->elements = fimo_aligned_alloc(elem_align, buffer_size, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }
    array->capacity = capacity;
    array->size = 0;

    return FIMO_EOK;
}

FIMO_EXPORT
void fimo_array_list_free(FimoArrayList *array, const FimoUSize elem_size, const FimoUSize elem_align,
                          const FimoArrayListDropFunc drop_func) {
    FIMO_DEBUG_ASSERT(array)
    if (drop_func) {
        for (FimoISize i = 0; i < (FimoISize)array->size; i++) {
            void *data = (char *)array->elements + (i * elem_size);
            drop_func(data);
        }
    }

    FimoUSize buffer_size = array->capacity * elem_size;
    fimo_free_aligned_sized(array->elements, elem_align, buffer_size);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_reserve(FimoArrayList *array, const FimoUSize elem_size, const FimoUSize elem_align,
                                   const FimoUSize additional, const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array)
    const FimoIntOverflowCheckUSize tmp = fimo_usize_overflowing_add(additional, array->size);
    if (tmp.overflow) {
        return FIMO_ERANGE;
    }
    const FimoUSize new_size = tmp.value;

    if (new_size <= array->capacity) {
        return FIMO_EOK;
    }
    return fimo_array_list_set_capacity(array, elem_size, elem_align, new_size, move_func, NULL);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_reserve_exact(FimoArrayList *array, const FimoUSize elem_size, const FimoUSize elem_align,
                                         const FimoUSize additional, const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array)
    const FimoIntOverflowCheckUSize tmp = fimo_usize_overflowing_add(additional, array->size);
    if (tmp.overflow) {
        return FIMO_ERANGE;
    }
    const FimoUSize new_size = tmp.value;

    if (new_size <= array->capacity) {
        return FIMO_EOK;
    }
    return fimo_array_list_set_capacity_exact(array, elem_size, elem_align, new_size, move_func, NULL);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_set_capacity(FimoArrayList *array, const FimoUSize elem_size, const FimoUSize elem_align,
                                        FimoUSize capacity, const FimoArrayListMoveFunc move_func,
                                        const FimoArrayListDropFunc drop_func) {
    FIMO_DEBUG_ASSERT(array)
    capacity = fimo_usize_next_power_of_two(capacity);
    return fimo_array_list_set_capacity_exact(array, elem_size, elem_align, capacity, move_func, drop_func);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_set_capacity_exact(FimoArrayList *array, const FimoUSize elem_size,
                                              const FimoUSize elem_align, const FimoUSize capacity,
                                              const FimoArrayListMoveFunc move_func,
                                              const FimoArrayListDropFunc drop_func) {
    FIMO_DEBUG_ASSERT(array)
    if (capacity > MAX_CAPACITY_) {
        return FIMO_EINVAL;
    }

    const FimoIntOverflowCheckUSize tmp = fimo_usize_overflowing_mul(capacity, elem_size);
    if (tmp.overflow) {
        return FIMO_ERANGE;
    }
    const FimoUSize buffer_size = tmp.value;

    FimoResult error = FIMO_EOK;
    void *elements = fimo_aligned_alloc(elem_align, buffer_size, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    if (array->capacity == 0) {
        array->size = 0;
        array->capacity = capacity;
        array->elements = elements;

        return FIMO_EOK;
    }

    const FimoUSize elements_to_move = (array->size < capacity) ? array->size : capacity;

    // Move over the elements `[0, capacity)`.
    if (move_func) {
        void *curr_src_ptr = array->elements;
        void *curr_dst_ptr = elements;
        for (FimoISize i = 0; i < (FimoISize)elements_to_move; i++) {
            move_func(curr_src_ptr, curr_dst_ptr);
            curr_src_ptr = ((char *)curr_src_ptr) + elem_size;
            curr_dst_ptr = ((char *)curr_dst_ptr) + elem_size;
        }
    }
    else if (elements) {
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(elements, array->elements, elements_to_move * elem_size);
    }

    // Drop the elements `[capacity, len)`.
    if (drop_func) {
        void *curr_ptr = ((char *)array->elements) + (elements_to_move * elem_size);
        for (FimoISize i = (FimoISize)capacity; i < (FimoISize)array->size; i++) {
            drop_func(curr_ptr);
            curr_ptr = ((char *)curr_ptr) + elem_size;
        }
    }
    fimo_free_aligned_sized(array->elements, elem_align, array->capacity * elem_size);

    array->size = array->size <= capacity ? array->size : capacity;
    array->capacity = capacity;
    array->elements = elements;

    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_set_len(FimoArrayList *array, const FimoUSize len) {
    FIMO_DEBUG_ASSERT(array)
    if (len > array->capacity) {
        return FIMO_EINVAL;
    }

    array->size = len;
    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_array_list_is_empty(const FimoArrayList *array) {
    FIMO_DEBUG_ASSERT(array)
    return array->size == 0;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_array_list_len(const FimoArrayList *array) {
    FIMO_DEBUG_ASSERT(array)
    return array->size;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUSize fimo_array_list_capacity(const FimoArrayList *array) {
    FIMO_DEBUG_ASSERT(array)
    return array->capacity;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_peek_front(const FimoArrayList *array, const FimoUSize elem_size, const void **element) {
    FIMO_DEBUG_ASSERT(array && element)
    return fimo_array_list_get(array, 0, elem_size, element);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_peek_back(const FimoArrayList *array, const FimoUSize elem_size, const void **element) {
    FIMO_DEBUG_ASSERT(array && element)
    return fimo_array_list_get(array, array->size - 1, elem_size, element);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_pop_front(FimoArrayList *array, const FimoUSize elem_size, void *element,
                                     const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array && element)
    return fimo_array_list_remove(array, 0, elem_size, element, move_func);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_pop_back(FimoArrayList *array, const FimoUSize elem_size, void *element,
                                    const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array && element)
    return fimo_array_list_remove(array, array->size - 1, elem_size, element, move_func);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_get(const FimoArrayList *array, const FimoUSize index, const FimoUSize elem_size,
                               const void **element) {
    FIMO_DEBUG_ASSERT(array && element)
    if (array->size <= index) {
        return FIMO_EINVAL;
    }

    const FimoUSize element_begin = index * elem_size;
    *element = ((char *)array->elements) + element_begin;

    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_push(FimoArrayList *array, const FimoUSize elem_size, const FimoUSize elem_align,
                                void *element, const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array && element)
    return fimo_array_list_insert(array, array->size, elem_size, elem_align, element, move_func);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_try_push(FimoArrayList *array, const FimoUSize elem_size, void *element,
                                    const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array && element)
    return fimo_array_list_try_insert(array, array->size, elem_size, element, move_func);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_insert(FimoArrayList *array, const FimoUSize index, const FimoUSize elem_size,
                                  const FimoUSize elem_align, void *element, const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array && element)
    if (array->size < index) {
        return FIMO_EINVAL;
    }

    if (array->size == array->capacity) {
        const FimoResult error = fimo_array_list_reserve(array, elem_size, elem_align, 1, move_func);
        if (FIMO_RESULT_IS_ERROR(error)) {
            return error;
        }
    }

    return fimo_array_list_try_insert(array, index, elem_size, element, move_func);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_try_insert(FimoArrayList *array, const FimoUSize index, const FimoUSize elem_size,
                                      void *element, const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array && element)
    if (array->size < index || array->capacity == array->size) {
        return FIMO_EINVAL;
    }
    FIMO_DEBUG_ASSERT(elem_size == 0 || array->elements != NULL)

    const FimoUSize elements_to_shift = array->size - index;
    array->size++;

    const FimoUSize element_begin = index * elem_size;
    // ReSharper disable once CppDFANullDereference
    void *element_ptr = ((char *)array->elements) + element_begin;
    void *shift_ptr = ((char *)element_ptr) + elem_size;

    if (move_func) {
        // Shift to the right.
        void *current_element_ptr = element_ptr;
        void *current_shift_ptr = shift_ptr;
        for (FimoISize i = 0; i < (FimoISize)elements_to_shift; i++) {
            move_func(current_element_ptr, current_shift_ptr);
            current_element_ptr = ((char *)current_element_ptr) + elem_size;
            current_shift_ptr = ((char *)current_shift_ptr) + elem_size;
        }

        move_func(element, element_ptr);
    }
    else if (elem_size > 0) {
        // Shift to the right.
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memmove(shift_ptr, element_ptr, elements_to_shift * elem_size);

        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(element_ptr, element, elem_size);
    }

    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_array_list_remove(FimoArrayList *array, const FimoUSize index, const FimoUSize elem_size, void *element,
                                  const FimoArrayListMoveFunc move_func) {
    FIMO_DEBUG_ASSERT(array && element)
    if (array->size <= index) {
        return FIMO_EINVAL;
    }

    array->size--;
    const FimoUSize elements_to_shift = array->size - index;

    const FimoUSize element_begin = index * elem_size;
    void *element_ptr = ((char *)array->elements) + element_begin;
    void *shift_ptr = ((char *)element_ptr) + elem_size;

    if (move_func) {
        move_func(element_ptr, element);

        // Shift to the left.
        for (FimoISize i = 0; i < (FimoISize)elements_to_shift; i++) {
            move_func(shift_ptr, element_ptr);
            element_ptr = ((char *)element_ptr) + elem_size;
            shift_ptr = ((char *)shift_ptr) + elem_size;
        }
    }
    else if (elem_size > 0) {
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(element, element_ptr, elem_size);

        // Shift to the left.
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memmove(element_ptr, shift_ptr, elements_to_shift * elem_size);
    }

    return FIMO_EOK;
}
