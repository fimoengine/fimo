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
 * The array can contain at most `FIMO_ISIZE_MAX` elements.
 */
typedef struct FimoArrayList {
    void* elements;
    size_t size;
    size_t capacity;
} FimoArrayList;

/**
 * Signature of the element destructor.
 */
typedef void (*FimoArrayListDropFunc)(void* ptr);

/**
 * Signature of the move operation of an element.
 */
typedef void (*FimoArrayListMoveFunc)(void* src, void* dst);

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
 * @param elem_align alignment of one element
 * @param array resulting array
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_with_capacity(size_t capacity, size_t elem_size,
    size_t elem_align, FimoArrayList* array);

/**
 * Creates a new empty array with an exact capacity.
 *
 * The new array has a capacity of exactly `capacity` elements.
 *
 * @param capacity minimum capacity
 * @param elem_size size of one element
 * @param elem_align alignment of one element
 * @param array resulting array
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_with_capacity_exact(size_t capacity, size_t elem_size,
    size_t elem_align, FimoArrayList* array);

/**
 * Frees an array.
 *
 * @param array array to free
 * @param elem_size size of one element
 * @param elem_align alignment of one element
 * @param drop_func optional element destructor function
 */
void fimo_array_list_free(FimoArrayList* array, size_t elem_size, size_t elem_align,
    FimoArrayListDropFunc drop_func);

/**
 * Reserve capacity for at least `additional` more elements.
 *
 * @param array array to increase the capacity of
 * @param elem_size size of one element
 * @param elem_align alignment of one element
 * @param additional number of additional elements
 * @param move_func optional element move function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_reserve(FimoArrayList* array, size_t elem_size,
    size_t elem_align, size_t additional, FimoArrayListMoveFunc move_func);

/**
 * Reserve capacity for exactly `additional` more elements.
 *
 * @param array array to increase the capacity of
 * @param elem_size size of one element
 * @param elem_align alignment of one element
 * @param additional number of additional elements
 * @param move_func optional element move function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_reserve_exact(FimoArrayList* array, size_t elem_size,
    size_t elem_align, size_t additional, FimoArrayListMoveFunc move_func);

/**
 * Resizes the array to a capacity of at least `capacity` elements.
 *
 * @param array array to resize
 * @param elem_size size of one element
 * @param elem_align alignment of one element
 * @param capacity new capacity
 * @param move_func optional element move function
 * @param drop_func optional element destructor function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_set_capacity(FimoArrayList* array, size_t elem_size,
    size_t elem_align, size_t capacity, FimoArrayListMoveFunc move_func,
    FimoArrayListDropFunc drop_func);

/**
 * Resizes the array to a capacity of exactly `capacity` elements.
 *
 * @param array array to resize
 * @param elem_size size of one element
 * @param elem_align alignment of one element
 * @param capacity new capacity
 * @param move_func optional element move function
 * @param drop_func optional element destructor function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_set_capacity_exact(FimoArrayList* array, size_t elem_size,
    size_t elem_align, size_t capacity, FimoArrayListMoveFunc move_func,
    FimoArrayListDropFunc drop_func);

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
 * The element is moved into `element`.
 *
 * @param array the array to modify
 * @param elem_size size of one element
 * @param element buffer for the element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_pop_front(FimoArrayList* array,
    size_t elem_size, void* element, FimoArrayListMoveFunc move_func);

/**
 * Removes the last element of the array.
 *
 * The element is moved into `element`.
 *
 * @param array the array to modify
 * @param elem_size size of one element
 * @param element buffer for the element
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_pop_back(FimoArrayList* array,
    size_t elem_size, void* element, FimoArrayListMoveFunc move_func);

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
 * @param elem_align alignment of one element
 * @param element element to insert
 * @param move_func optional element move function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_push(FimoArrayList* array, size_t elem_size,
    size_t elem_align, void* element, FimoArrayListMoveFunc move_func);

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
    void* element, FimoArrayListMoveFunc move_func);

/**
 * Inserts an element at the specified position.
 *
 * The position `index` must be in the range `[0, len]`.
 * The element is moved into the buffer owned by the array.
 * If the element does not fit, the capacity is increased
 * in accordance.
 *
 * @param array array to modify
 * @param index insertion index
 * @param elem_size size of one element
 * @param elem_align alignment of one element
 * @param element element to insert
 * @param move_func optional element move function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_insert(FimoArrayList* array, size_t index,
    size_t elem_size, size_t elem_align, void* element,
    FimoArrayListMoveFunc move_func);

/**
 * Inserts an element at the specified position.
 *
 * The position `index` must be in the range `[0, len]`.
 * The element is moved into the buffer owned by the array.
 *
 * @param array array to modify
 * @param index insertion index
 * @param elem_size size of one element
 * @param element element to insert
 * @param move_func optional element move function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_try_insert(FimoArrayList* array, size_t index,
    size_t elem_size, void* element, FimoArrayListMoveFunc move_func);

/**
 * Removes the element at the given position from the array.
 *
 *
 * The position `index` must be in the range `[0, len - 1]`.
 * The element is moved into `element`.
 *
 * @param array array to modify
 * @param index index to remove
 * @param elem_size size of one element
 * @param element buffer for the element
 * @param move_func optional element move function
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_array_list_remove(FimoArrayList* array, size_t index,
    size_t elem_size, void* element, FimoArrayListMoveFunc move_func);

#ifdef __cplusplus
}
#endif

#endif // FIMO_ARRAY_LIST_H
