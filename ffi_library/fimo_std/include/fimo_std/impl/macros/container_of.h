#ifndef FIMO_IMPL_MACROS_CONTAINER_OF_H
#define FIMO_IMPL_MACROS_CONTAINER_OF_H

#include <fimo_std/impl/integers/integers_base.h>
#include <fimo_std/impl/macros/inline.h>

/**
 * Return a pointer to the structure containing the member.
 *
 * @param PTR pointer to the member
 * @param TYPE type of the structure
 * @param MEMBER name of the member within the struct
 */
#define FIMO_CONTAINER_OF(PTR, TYPE, MEMBER) \
    (TYPE*)fimo_impl_macros_container_of((PTR), offsetof(TYPE, MEMBER))

/**
 * Return a const pointer to the structure containing the member.
 *
 * @param PTR pointer to the member
 * @param TYPE type of the structure
 * @param MEMBER name of the member within the struct
 */
#define FIMO_CONTAINER_OF_CONST(PTR, TYPE, MEMBER) \
    (const TYPE*)fimo_impl_macros_container_of_const((PTR), offsetof(TYPE, MEMBER))

static FIMO_INLINE_ALWAYS const void* fimo_impl_macros_container_of_const(const void* ptr, FimoUSize member_offset)
{
    const char* tmp = (const char*)ptr;
    tmp -= member_offset;
    return tmp;
}

static FIMO_INLINE_ALWAYS void* fimo_impl_macros_container_of(void* ptr, FimoUSize member_offset)
{
    return (void*)fimo_impl_macros_container_of_const(ptr, member_offset);
}

#endif // !FIMO_IMPL_MACROS_CONTAINER_OF_H
