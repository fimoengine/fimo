#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

#include <cmocka.h>

#include <fimo_std/context.h>

/* A test case that does nothing and succeeds. */
static void context_test(void** state)
{
    (void)state; /* unused */
    FimoContext ctx;
    FimoError error = fimo_context_init(NULL, &ctx);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_context_check_version(ctx);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_context_destroy_strong(ctx);
    assert_true(FIMO_IS_ERROR(error));

    error = fimo_context_destroy_weak(ctx);
    assert_true(FIMO_IS_ERROR(error));

    FimoAtomicRefCount* rc = FIMO_CONTEXT_REF_COUNT(ctx);
    assert_non_null(rc);
    assert_true(fimo_refcount_atomic_is_unique(rc));

    bool destroy = fimo_decrease_strong_count_atomic(rc);
    assert_true(destroy);

    error = fimo_context_destroy_strong(ctx);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_context_destroy_weak(ctx);
    assert_false(FIMO_IS_ERROR(error));
}

int main(void)
{
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(context_test),
    };

    return cmocka_run_group_tests(tests, NULL, NULL);
}
