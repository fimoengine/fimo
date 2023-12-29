#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

#include <cmocka.h>

#include <fimo_std/char.h>

static void ascii_uppercase(void** state)
{
    (void)state;
    const char positives[] = "ABCDEFGHIJKLMNOQPRSTUVWXYZ";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_uppercase(positives[i]));
    }

    const char negatives[] = "abcdefghijklmnopqrstuvwxyz"
                             "0123456789"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_uppercase(negatives[i]));
    }
}

static void ascii_lowercase(void** state)
{
    (void)state;
    const char positives[] = "abcdefghijklmnopqrstuvwxyz";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_lowercase(positives[i]));
    }

    const char negatives[] = "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "0123456789"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_lowercase(negatives[i]));
    }
}

static void ascii_alphanumeric(void** state)
{
    (void)state;
    const char positives[] = ""
                             "abcdefghijklmnopqrstuvwxyz"
                             "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "0123456789";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_alphanumeric(positives[i]));
    }

    const char negatives[] = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_alphanumeric(negatives[i]));
    }
}

static void ascii_digit(void** state)
{
    (void)state;
    const char positives[] = ""
                             "0123456789";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_digit(positives[i]));
    }

    const char negatives[] = "abcdefghijklmnopqrstuvwxyz"
                             "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_digit(negatives[i]));
    }
}

static void ascii_octdigit(void** state)
{
    (void)state;
    const char positives[] = ""
                             "01234567";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_octdigit(positives[i]));
    }

    const char negatives[] = "89"
                             "abcdefghijklmnopqrstuvwxyz"
                             "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_octdigit(negatives[i]));
    }
}

static void ascii_hexdigit(void** state)
{
    (void)state;
    const char positives[] = ""
                             "0123456789"
                             "abcdefABCDEF";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_hexdigit(positives[i]));
    }

    const char negatives[] = "ghijklmnopqrstuvwxyz"
                             "GHIJKLMNOQPRSTUVWXYZ"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_hexdigit(negatives[i]));
    }
}

static void ascii_punctuation(void** state)
{
    (void)state;
    const char positives[] = ""
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_punctuation(positives[i]));
    }

    const char negatives[] = "abcdefghijklmnopqrstuvwxyz"
                             "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "0123456789"
                             " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_punctuation(negatives[i]));
    }
}

static void ascii_graphic(void** state)
{
    (void)state;
    const char positives[] = ""
                             "abcdefghijklmnopqrstuvwxyz"
                             "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "0123456789"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_graphic(positives[i]));
    }

    const char negatives[] = " \t\n\x0c\r"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_graphic(negatives[i]));
    }
}

static void ascii_whitespace(void** state)
{
    (void)state;
    const char positives[] = ""
                             " \t\n\x0c\r";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_whitespace(positives[i]));
    }

    const char negatives[] = "abcdefghijklmnopqrstuvwxyz"
                             "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "0123456789"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x0b\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_whitespace(negatives[i]));
    }
}

static void ascii_control(void** state)
{
    (void)state;
    const char positives[] = ""
                             "\x00\x01\x02\x03\x04\x05\x06\x07"
                             "\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"
                             "\x10\x11\x12\x13\x14\x15\x16\x17"
                             "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f"
                             "\x7f";
    for (int i = 0; i < (int)sizeof(positives) - 1; i++) {
        assert_true(fimo_char_is_ascii_control(positives[i]));
    }

    const char negatives[] = "abcdefghijklmnopqrstuvwxyz"
                             "ABCDEFGHIJKLMNOQPRSTUVWXYZ"
                             "0123456789"
                             "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
                             " ";
    for (int i = 0; i < (int)sizeof(negatives) - 1; i++) {
        assert_false(fimo_char_is_ascii_control(negatives[i]));
    }
}

int main(void)
{
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(ascii_uppercase),
        cmocka_unit_test(ascii_lowercase),
        cmocka_unit_test(ascii_alphanumeric),
        cmocka_unit_test(ascii_digit),
        cmocka_unit_test(ascii_octdigit),
        cmocka_unit_test(ascii_hexdigit),
        cmocka_unit_test(ascii_punctuation),
        cmocka_unit_test(ascii_graphic),
        cmocka_unit_test(ascii_whitespace),
        cmocka_unit_test(ascii_control),
    };

    return cmocka_run_group_tests(tests, NULL, NULL);
}
