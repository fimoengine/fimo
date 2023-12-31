#ifndef FIMO_ARRAY_LIST_H
#define FIMO_ARRAY_LIST_H

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * A dynamically growing array of elements.
 *
 * The array can contain at most `SIZE_MAX >> 1` elements.
 */
typedef struct FimoArrayList {
    void* elements;
    size_t size;
    size_t capacity;
} FimoArrayList;

/**
 * Creates a new empty array.
 *
 * @return Empty array.
 */
FIMO_MUST_USE
FimoArrayList fimo_array_list_new(void);

/**
 * Creates a new empty array with a minimum capacity.
 *
 * The new array has a capacity of at least `capacity` elements.
 *
 * @param capacity minimum capacity
 * @param elem_size size of one element
 * @param array resulting array
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_with_capacity(size_t capacity, size_t elem_size,
    FimoArrayList* array);

/**
 * Creates a new empty array with an exact capacity.
 *
 * The new array has a capacity of exactly `capacity` elements.
 *
 * @param capacity minimum capacity
 * @param elem_size size of one element
 * @param array resulting array
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_with_capacity_exact(size_t capacity, size_t elem_size,
    FimoArrayList* array);

/**
 * Frees an array.
 *
 * @param elem_size size of one element
 * @param array array to free
 */
void fimo_array_list_free(FimoArrayList* array, size_t elem_size);

/**
 * Reserve capacity for at least `additional` more elements.
 *
 * @param array array to increase the capacity of
 * @param elem_size size of one element
 * @param additional number of additional elements
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_reserve(FimoArrayList* array, size_t elem_size,
    size_t additional);

/**
 * Reserve capacity for exactly `additional` more elements.
 *
 * @param array array to increase the capacity of
 * @param elem_size size of one element
 * @param additional number of additional elements
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_reserve_exact(FimoArrayList* array, size_t elem_size,
    size_t additional);

/**
 * Resizes the array to a capacity of at least `capacity` elements.
 *
 * @param array array to resize
 * @param elem_size size of one element
 * @param capacity new capacity
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_resize(FimoArrayList* array, size_t elem_size,
    size_t capacity);

/**
 * Resizes the array to a capacity of exactly `capacity` elements.
 *
 * @param array array to resize
 * @param elem_size size of one element
 * @param capacity new capacity
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_resize_exact(FimoArrayList* array, size_t elem_size,
    size_t capacity);

/**
 * Sets the number of elements contained in the array.
 *
 * @param array array to modify
 * @param len new number of elements
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_set_len(FimoArrayList* array, size_t len);

/**
 * Returns whether the array is empty.
 *
 * @param array array to query
 *
 * @return Array is empty.
 */
FIMO_MUST_USE
bool fimo_array_list_is_empty(const FimoArrayList* array);

/**
 * Returns the number of elements in the array.
 *
 * @param array array to query
 *
 * @return Number of elements.
 */
FIMO_MUST_USE
size_t fimo_array_list_len(const FimoArrayList* array);

/**
 * Returns the capacity in elements of the array.
 *
 * @param array array to query
 *
 * @return Array capacity.
 */
FIMO_MUST_USE
size_t fimo_array_list_capacity(const FimoArrayList* array);

/**
 * Returns a pointer to the first element in the array.
 *
 * @param array the array
 * @param elem_size size of one element
 * @param element first element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_peek_front(const FimoArrayList* array,
    size_t elem_size, const void** element);

/**
 * Returns a pointer to the last element in the array.
 *
 * @param array the array
 * @param elem_size size of one element
 * @param element last element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_peek_back(const FimoArrayList* array,
    size_t elem_size, const void** element);

/**
 * Removes the first element of the array.
 *
 * The element is copied into `element`.
 *
 * @param array the array to modify
 * @param elem_size size of one element
 * @param element buffer for the element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_pop_front(FimoArrayList* array,
    size_t elem_size, void* element);

/**
 * Removes the last element of the array.
 *
 * The element is copied into `element`.
 *
 * @param array the array to modify
 * @param elem_size size of one element
 * @param element buffer for the element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_pop_back(FimoArrayList* array,
    size_t elem_size, void* element);

/**
 * Returns a pointer to the element at position `index`.
 *
 * @param array array to query
 * @param index index of the element
 * @param elem_size size of one element
 * @param element resulting element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_get(const FimoArrayList* array, size_t index,
    size_t elem_size, const void** element);

/**
 * Pushes a new element to the end of the array.
 *
 * May reallocate the array to fit the new element.
 *
 * @param array array to modify
 * @param elem_size size of one element
 * @param element element to insert
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_push(FimoArrayList* array, size_t elem_size,
    const void* element);

/**
 * Pushes a new element to the end of the array.
 *
 * @param array array to modify
 * @param elem_size size of one element
 * @param element element to insert
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_try_push(FimoArrayList* array, size_t elem_size,
    const void* element);

/**
 * Inserts an element at the specified position.
 *
 * The position `index` must be in the range `[0, len]`.
 * The element is copied into the buffer owned by the array.
 * If the element does not fit, the capacity is increased
 * in accordance.
 *
 * @param array array to modify
 * @param index insertion index
 * @param elem_size size of one element
 * @param element element to insert
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_insert(FimoArrayList* array, size_t index,
    size_t elem_size, const void* element);

/**
 * Inserts an element at the specified position.
 *
 * The position `index` must be in the range `[0, len]`.
 * The element is copied into the buffer owned by the array.
 *
 * @param array array to modify
 * @param index insertion index
 * @param elem_size size of one element
 * @param element element to insert
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_try_insert(FimoArrayList* array, size_t index,
    size_t elem_size, const void* element);

/**
 * Removes the element at the given position from the array.
 *
 *
 * The position `index` must be in the range `[0, len - 1]`.
 * The element is copied into `element`.
 *
 * @param array array to modify
 * @param index index to remove
 * @param elem_size size of one element
 * @param element buffer for the element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_remove(FimoArrayList* array, size_t index,
    size_t elem_size, void* element);

#ifdef __cplusplus
}
#endif

#endif // FIMO_ARRAY_LIST_H
