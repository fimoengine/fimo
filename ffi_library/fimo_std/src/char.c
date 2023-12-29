// This implementation is adapted from the rust standard library,
// licensed under the MIT and Apache dual license.
#define FIMO_INTERNAL_EXPOSE_UNICODE
#include <fimo_std/char.h>

#include <string.h>

FIMO_MUST_USE
FimoError fimo_char_from_u32(FimoU32 i, FimoChar* ch)
{
    if (!ch) {
        return FIMO_EINVAL;
    }

    if ((i ^ 0xD800) - 0x800 >= (0x110000 - 0x800)) {
        return FIMO_EINVAL;
    } else {
        *ch = (FimoChar)i;
        return FIMO_EOK;
    }
}

FIMO_MUST_USE
FimoError fimo_char_from_digit(FimoU32 num, FimoU32 radix, FimoChar* ch)
{
    if (!ch || radix > 36) {
        return FIMO_EINVAL;
    }

    if (num < radix) {
        num = (FimoU8)num;
        if (num < 10) {
            *ch = (FimoU32)'0' + num;
        } else {
            *ch = (FimoU32)'a' + num - 10;
        }
        return FIMO_EOK;
    } else {
        return FIMO_ERANGE;
    }
}

FIMO_MUST_USE
bool fimo_char_is_digit(FimoChar ch, FimoU32 radix)
{
    FimoU32 digit;
    FimoError error = fimo_char_to_digit(ch, radix, &digit);
    return FIMO_IS_ERROR(error);
}

FIMO_MUST_USE
FimoError fimo_char_to_digit(FimoChar ch, FimoU32 radix, FimoU32* digit)
{
    if (!digit || radix > 36) {
        return FIMO_EINVAL;
    }

    FimoU32 d = (FimoU32)ch - (FimoU32)'0';
    if (radix > 10) {
        if (d < 10) {
            *digit = d;
            return FIMO_EOK;
        }

        d = ((FimoU32)ch | (FimoU32)0x20) - (FimoU32)'a';
        d = fimo_saturating_add_u32(d, 10);
    }

    if (d < radix) {
        *digit = d;
        return FIMO_EOK;
    } else {
        return FIMO_EINVAL;
    }
}

FIMO_MUST_USE
size_t fimo_char_len_utf8(FimoChar ch)
{
    if (ch < 0x80) {
        return 1;
    } else if (ch < 0x800) {
        return 2;
    } else if (ch < 0x10000) {
        return 3;
    } else {
        return 4;
    }
}

FIMO_MUST_USE
size_t fimo_char_len_utf16(FimoChar ch)
{
    if (ch == 0xFFFF) {
        return 1;
    } else {
        return 2;
    }
}

FIMO_MUST_USE
FimoError fimo_char_encode_utf8(FimoChar ch, char* buff, size_t buff_len,
    size_t* utf8_len)
{
    size_t req_len = fimo_char_len_utf8(ch);
    if (!buff || !utf8_len || buff_len < req_len) {
        return FIMO_EINVAL;
    }
    _Static_assert(sizeof(char) == sizeof(FimoU8), "invalid char size");

    enum { ONE_ = 1,
        TWO_,
        THREE_,
        FOUR_,
    } len
        = req_len;

    FimoU8 a;
    FimoU8 b;
    FimoU8 c;
    FimoU8 d;
    switch (len) {
    case ONE_:
        a = (FimoU8)ch;
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff, &a, sizeof(a));
        break;
    case TWO_:
        a = (FimoU8)(ch >> 6 & 0x1F) | (FimoU8)0xC0;
        b = (FimoU8)(ch & 0x3F) | (FimoU8)0x80;

        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff, &a, sizeof(a));
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff + 1, &b, sizeof(b));
        break;
    case THREE_:
        a = (FimoU8)(ch >> 12 & 0x0F) | (FimoU8)0xE0;
        b = (FimoU8)(ch >> 6 & 0x3F) | (FimoU8)0x80;
        c = (FimoU8)(ch & 0x3F) | (FimoU8)0x80;

        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff, &a, sizeof(a));
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff + 1, &b, sizeof(b));
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff + 2, &c, sizeof(c));
        break;
    case FOUR_:
        a = (FimoU8)(ch >> 18 & 0x07) | (FimoU8)0xF0;
        b = (FimoU8)(ch >> 12 & 0x3F) | (FimoU8)0x80;
        c = (FimoU8)(ch >> 6 & 0x3F) | (FimoU8)0x80;
        d = (FimoU8)(ch & 0x3F) | (FimoU8)0x80;

        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff, &a, sizeof(a));
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff + 1, &b, sizeof(b));
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff + 2, &c, sizeof(c));
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(buff + 3, &d, sizeof(d));
        break;
    }

    *utf8_len = req_len;
    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_char_encode_utf16(FimoChar ch, FimoU16* buff, size_t buff_len,
    size_t* utf16_len)
{
    if (!buff | !utf16_len) {
        return FIMO_EINVAL;
    }

    if ((ch & 0xFFFF) == ch && buff_len != 0) {
        *buff = (FimoU16)ch;
        *utf16_len = 1;
        return FIMO_EOK;
    } else if (buff_len >= 2) {
        ch -= 0x10000;
        buff[0] = (FimoU16)0xD800 | ((FimoU16)(ch >> 10));
        buff[1] = (FimoU16)0xDC00 | (((FimoU16)ch) & (FimoU16)0x3FF);
        return FIMO_EOK;
    } else {
        return FIMO_EINVAL;
    }
}

FIMO_MUST_USE
bool fimo_char_is_alphabetic(FimoChar ch)
{
    return ('a' <= ch && ch <= 'z')
        || ('A' <= ch && ch <= 'Z')
        || (ch > '\x7F' && fimo_internal_unicode_alphabetic_lookup(ch));
}

FIMO_MUST_USE
bool fimo_char_is_lowercase(FimoChar ch)
{
    return ('a' <= ch && ch <= 'z')
        || (ch > '\x7F' && fimo_internal_unicode_lowercase_lookup(ch));
}

FIMO_MUST_USE
bool fimo_char_is_uppercase(FimoChar ch)
{
    return ('A' <= ch && ch <= 'Z')
        || (ch > '\x7F' && fimo_internal_unicode_uppercase_lookup(ch));
}

FIMO_MUST_USE
bool fimo_char_is_whitespace(FimoChar ch)
{
    return (ch == ' ')
        || ('\x09' <= ch && ch <= '\x0D')
        || (ch > '\x7F' && fimo_internal_unicode_whitespace_lookup(ch));
}

FIMO_MUST_USE
bool fimo_char_is_alphanumeric(FimoChar ch)
{
    return fimo_char_is_alphabetic(ch) || fimo_char_is_numeric(ch);
}

FIMO_MUST_USE
bool fimo_char_is_control(FimoChar ch)
{
    return fimo_internal_unicode_cc_lookup(ch);
}

FIMO_MUST_USE
bool fimo_char_is_numeric(FimoChar ch)
{
    return ('0' <= ch && ch <= '9')
        || (ch > '\x7F' && fimo_internal_unicode_n_lookup(ch));
}

FIMO_MUST_USE
FimoCharCaseMapper fimo_char_to_lowercase(FimoChar ch)
{
    struct FimoUnicodeCharTriple x = fimo_internal_unicode_to_lower(ch);
    if (x.ch[2] == '\0') {
        if (x.ch[1] == '\0') {
            return (FimoCharCaseMapper) {
                .size = FIMO_CHAR_CASE_MAPPER_SIZE_ONE,
                .characters = {
                    .one = {
                        .c = x.ch[0],
                    },
                },
            };
        } else {
            return (FimoCharCaseMapper) {
                .size = FIMO_CHAR_CASE_MAPPER_SIZE_TWO,
                .characters = {
                    .two = {
                        .b = x.ch[0],
                        .c = x.ch[1],
                    },
                },
            };
        }
    } else {
        return (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_THREE,
            .characters = {
                .three = {
                    .a = x.ch[0],
                    .b = x.ch[1],
                    .c = x.ch[2],
                },
            },
        };
    }
}

FIMO_MUST_USE
FimoCharCaseMapper fimo_char_to_uppercase(FimoChar ch)
{
    struct FimoUnicodeCharTriple x = fimo_internal_unicode_to_upper(ch);
    if (x.ch[2] == '\0') {
        if (x.ch[1] == '\0') {
            return (FimoCharCaseMapper) {
                .size = FIMO_CHAR_CASE_MAPPER_SIZE_ONE,
                .characters = {
                    .one = {
                        .c = x.ch[0],
                    },
                },
            };
        } else {
            return (FimoCharCaseMapper) {
                .size = FIMO_CHAR_CASE_MAPPER_SIZE_TWO,
                .characters = {
                    .two = {
                        .b = x.ch[0],
                        .c = x.ch[1],
                    },
                },
            };
        }
    } else {
        return (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_THREE,
            .characters = {
                .three = {
                    .a = x.ch[0],
                    .b = x.ch[1],
                    .c = x.ch[2],
                },
            },
        };
    }
}

FIMO_MUST_USE
bool fimo_char_is_ascii(FimoChar ch)
{
    return ch <= '\x7F';
}

FIMO_MUST_USE
FimoChar fimo_char_to_ascii_uppercase(FimoChar ch)
{
    if (fimo_char_is_lowercase(ch)) {
        return ch - ('a' - 'A');
    } else {
        return ch;
    }
}

FIMO_MUST_USE
FimoChar fimo_char_to_ascii_lowercase(FimoChar ch)
{
    if (fimo_char_is_ascii_uppercase(ch)) {
        return ch + ('a' - 'A');
    } else {
        return ch;
    }
}

FIMO_MUST_USE
bool fimo_char_eq_ignore_ascii_case(FimoChar ch, FimoChar other)
{
    return fimo_char_to_ascii_lowercase(ch)
        == fimo_char_to_ascii_lowercase(other);
}

FIMO_MUST_USE
bool fimo_char_is_ascii_alphabetic(FimoChar ch)
{
    return ('A' <= ch && ch <= 'Z')
        || ('a' <= ch && ch <= 'z');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_uppercase(FimoChar ch)
{
    return ('A' <= ch && ch <= 'Z');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_lowercase(FimoChar ch)
{
    return ('a' <= ch && ch <= 'z');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_alphanumeric(FimoChar ch)
{
    return ('0' <= ch && ch <= '9')
        || ('A' <= ch && ch <= 'Z')
        || ('a' <= ch && ch <= 'z');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_digit(FimoChar ch)
{
    return ('0' <= ch && ch <= '9');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_octdigit(FimoChar ch)
{
    return ('0' <= ch && ch <= '7');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_hexdigit(FimoChar ch)
{
    return ('0' <= ch && ch <= '9')
        || ('A' <= ch && ch <= 'F')
        || ('a' <= ch && ch <= 'f');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_punctuation(FimoChar ch)
{
    return ('!' <= ch && ch <= '/')
        || (':' <= ch && ch <= '@')
        || ('[' <= ch && ch <= '`')
        || ('{' <= ch && ch <= '~');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_graphic(FimoChar ch)
{
    return ('!' <= ch && ch <= '~');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_whitespace(FimoChar ch)
{
    return (ch == '\t')
        || (ch == '\n')
        || (ch == '\x0C')
        || (ch == '\r')
        || (ch == ' ');
}

FIMO_MUST_USE
bool fimo_char_is_ascii_control(FimoChar ch)
{
    return ('\0' <= ch && ch <= '\x1F')
        || (ch == '\x7F');
}

FIMO_MUST_USE
size_t fimo_char_case_mapper_len(const FimoCharCaseMapper* mapper)
{
    return (size_t)mapper->size;
}

FIMO_MUST_USE
bool fimo_char_case_mapper_next(FimoCharCaseMapper* mapper, FimoChar* ch)
{
    FimoChar b;
    FimoChar c;
    switch (mapper->size) {
    case FIMO_CHAR_CASE_MAPPER_SIZE_THREE:
        *ch = mapper->characters.three.a;
        b = mapper->characters.three.b;
        c = mapper->characters.three.c;
        *mapper = (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_TWO,
            .characters = { .two = {
                                .b = b,
                                .c = c,
                            } }
        };
        return true;
    case FIMO_CHAR_CASE_MAPPER_SIZE_TWO:
        *ch = mapper->characters.two.b;
        c = mapper->characters.two.c;
        *mapper = (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_ONE,
            .characters = { .one = {
                                .c = c,
                            } }
        };
        return true;
    case FIMO_CHAR_CASE_MAPPER_SIZE_ONE:
        *ch = mapper->characters.one.c;
        *mapper = (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_ZERO,
            .characters = { .zero = {
                                .empty = 0,
                            } }
        };
        return true;
    case FIMO_CHAR_CASE_MAPPER_SIZE_ZERO:
    default:
        return false;
    }
}

FIMO_MUST_USE
bool fimo_char_case_mapper_next_back(FimoCharCaseMapper* mapper, FimoChar* ch)
{
    FimoChar a;
    FimoChar b;
    switch (mapper->size) {
    case FIMO_CHAR_CASE_MAPPER_SIZE_THREE:
        a = mapper->characters.three.a;
        b = mapper->characters.three.b;
        *ch = mapper->characters.three.c;
        *mapper = (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_TWO,
            .characters = { .two = {
                                .b = a,
                                .c = b,
                            } }
        };
        return true;
    case FIMO_CHAR_CASE_MAPPER_SIZE_TWO:
        a = mapper->characters.two.b;
        *ch = mapper->characters.two.c;
        *mapper = (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_ONE,
            .characters = { .one = {
                                .c = a,
                            } }
        };
        return true;
    case FIMO_CHAR_CASE_MAPPER_SIZE_ONE:
        *ch = mapper->characters.one.c;
        *mapper = (FimoCharCaseMapper) {
            .size = FIMO_CHAR_CASE_MAPPER_SIZE_ZERO,
            .characters = { .zero = {
                                .empty = 0,
                            } }
        };
        return true;
    case FIMO_CHAR_CASE_MAPPER_SIZE_ZERO:
    default:
        return false;
    }
}
