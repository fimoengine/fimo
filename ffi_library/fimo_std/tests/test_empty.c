#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

#include <cmocka.h>

/* A test case that does nothing and succeeds. */
static void ascii_control(void** state)
{
    (void)state; /* unused */
}

int main(void)
{
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(ascii_control),
    };

    return cmocka_run_group_tests(tests, NULL, NULL);
}
