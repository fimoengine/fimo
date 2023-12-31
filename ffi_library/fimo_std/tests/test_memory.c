#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

#include <cmocka.h>

#include <fimo_std/memory.h>

static void malloc_test(void** state)
{
    (void)state;
    FimoError error = FIMO_EOK;
    void* buff = fimo_malloc(0, &error);
    assert_null(buff);
    assert_false(FIMO_IS_ERROR(error));

    buff = fimo_malloc(sizeof(long long), &error);
    assert_non_null(buff);
    assert_false(FIMO_IS_ERROR(error));
    assert_true((uintptr_t)buff % _Alignof(max_align_t) == 0);
    fimo_free(buff);

    FimoMallocBuffer buffer = fimo_malloc_sized(1339, NULL);
    assert_non_null(buffer.ptr);
    assert_true(buffer.buff_size >= 1339);
    assert_true((uintptr_t)buffer.ptr % _Alignof(max_align_t) == 0);
    fimo_free(buffer.ptr);
}

static void calloc_test(void** state)
{
    (void)state;
    FimoError error = FIMO_EOK;
    long long* buff = fimo_calloc(0, &error);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(buff);

    buff = fimo_calloc(10 * sizeof(long long), &error);
    assert_non_null(buff);
    assert_false(FIMO_IS_ERROR(error));
    assert_true((uintptr_t)buff % _Alignof(max_align_t) == 0);
    for (int i = 0; i < 10; i++) {
        assert_true(buff[i] == 0);
    }
    fimo_free_sized(buff, 10 * sizeof(long long));

    FimoMallocBuffer buffer = fimo_calloc_sized(1339, NULL);
    assert_non_null(buffer.ptr);
    assert_true(buffer.buff_size >= 1339);
    assert_true((uintptr_t)buffer.ptr % _Alignof(max_align_t) == 0);
    buff = buffer.ptr;
    for (int i = 0; i < (int)(buffer.buff_size / sizeof(long long)); i++) {
        assert_true(buff[i] == 0);
    }
    fimo_free_sized(buffer.ptr, buffer.buff_size);
}

static void aligned_alloc_test(void** state)
{
    (void)state;
    FimoError error = FIMO_EOK;
    void* buff = fimo_aligned_alloc(0, 10, &error);
    assert_true(FIMO_IS_ERROR(error));
    assert_null(buff);

    buff = fimo_aligned_alloc(17, 10, &error);
    assert_true(FIMO_IS_ERROR(error));
    assert_null(buff);

    buff = fimo_aligned_alloc(256, 0, &error);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(buff);

    buff = fimo_aligned_alloc(256, sizeof(long long), &error);
    assert_non_null(buff);
    assert_false(FIMO_IS_ERROR(error));
    assert_true((uintptr_t)buff % 256 == 0);
    fimo_free_aligned_sized(buff, 256, sizeof(long long));

    FimoMallocBuffer buffer = fimo_aligned_alloc_sized(256, 1339, NULL);
    assert_non_null(buffer.ptr);
    assert_true(buffer.buff_size >= 1339);
    assert_true((uintptr_t)buffer.ptr % 256 == 0);
    fimo_free_aligned_sized(buffer.ptr, 256, buffer.buff_size);
}

int main(void)
{
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(malloc_test),
        cmocka_unit_test(calloc_test),
        cmocka_unit_test(aligned_alloc_test),
    };

    return cmocka_run_group_tests(tests, NULL, NULL);
}
