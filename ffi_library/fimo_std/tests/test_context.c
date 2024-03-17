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

    fimo_context_acquire(ctx);
    fimo_context_release(ctx);
    fimo_context_release(ctx);
}

int main(void)
{
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(context_test),
    };

    return cmocka_run_group_tests(tests, NULL, NULL);
}
