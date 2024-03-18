#ifndef FIMO_CHAR_H
#define FIMO_CHAR_H

#include <fimo_std/error.h>
#include <fimo_std/impl/unicode.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * The lowest valid code point of a `FimoChar`.
 */
#define FIMO_CHAR_MIN (FimoChar)'\0'

/**
 * The highest valid code point of a `FimoChar`.
 */
#define FIMO_CHAR_MAX (FimoChar)0x10ffff

/**
 * `U+FFFD REPLACEMENT CHARACTER` (�) used to represent a
 * decoding error.
 */
#define FIMO_CHAR_REPLACEMENT_CHARACTER (FimoChar)0xFFFD

/**
 * Major version of the implemented Unicode Standard.
 */
#define FIMO_CHAR_UNICODE_VERSION_MAJOR FIMO_IMPL_UNICODE_VERSION_MAJOR

/**
 * Minor version of the implemented Unicode Standard.
 */
#define FIMO_CHAR_UNICODE_VERSION_MINOR FIMO_IMPL_UNICODE_VERSION_MINOR

/**
 * Update version of the implemented Unicode Standard.
 */
#define FIMO_CHAR_UNICODE_VERSION_UPDATE FIMO_IMPL_UNICODE_VERSION_UPDATE

/**
 * A Unicode character.
 *
 * A `FimoChar` represents a single 'Unicode scalar value'.
 * A 'Unicode scalar value' is any 'Unicode code point' other
 * than a a 'Unicode surrogate code point'. A `FimoChar` may
 * only be constructed by a valid 'Unicode scalar value'.
 */
typedef FimoU32 FimoChar;

/**
 * Number of characters that are required for mapping the
 * case of a unicode character.
 */
typedef enum FimoCharCaseMapperSize {
    FIMO_CHAR_CASE_MAPPER_SIZE_ZERO,
    FIMO_CHAR_CASE_MAPPER_SIZE_ONE,
    FIMO_CHAR_CASE_MAPPER_SIZE_TWO,
    FIMO_CHAR_CASE_MAPPER_SIZE_THREE,
} FimoCharCaseMapperSize;

/**
 * An iterator for mapping from a lowercase character to their
 * uppercase representation, or vice versa.
 *
 * Unlike ASCII characters, changing the case of a Unicode
 * character may insert or remove additional characters.
 */
typedef struct FimoCharCaseMapper {
    FimoCharCaseMapperSize size;
    union {
        struct {
            FimoU8 empty;
        } zero;
        struct {
            FimoChar c;
        } one;
        struct {
            FimoChar b;
            FimoChar c;
        } two;
        struct {
            FimoChar a;
            FimoChar b;
            FimoChar c;
        } three;
    } characters;
} FimoCharCaseMapper;

/**
 * Performs a checked conversion from a 32-Bit value to
 * a Unicode character.
 *
 * @param i 32-Bit value
 * @param ch character result
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_char_from_u32(FimoU32 i, FimoChar *ch);

/**
 * Converts a digit in the given radix to a character.
 *
 * A ‘radix’ here is sometimes also called a ‘base’. A radix of two
 * indicates a binary number, a radix of ten, decimal, and a radix
 * of sixteen, hexadecimal, to give some common values. Arbitrary
 * radices `< 36` are supported.
 *
 * @param num number
 * @param radix radic
 * @param ch resulting character
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_char_from_digit(FimoU32 num, FimoU32 radix, FimoChar *ch);

/**
 * Checks if a character is a digit in the given radix.
 *
 * A ‘radix’ here is sometimes also called a ‘base’. A radix of two
 * indicates a binary number, a radix of ten, decimal, and a radix
 * of sixteen, hexadecimal, to give some common values. Arbitrary
 * radices `< 36` are supported.
 *
 * @param ch character
 * @param radix radix
 *
 * @return `true` if the character is a digit.
 */
FIMO_MUST_USE
bool fimo_char_is_digit(FimoChar ch, FimoU32 radix);

/**
 * Converts the character to a digit in the given radix.
 *
 * A ‘radix’ here is sometimes also called a ‘base’. A radix of two
 * indicates a binary number, a radix of ten, decimal, and a radix
 * of sixteen, hexadecimal, to give some common values. Arbitrary
 * radices `< 36` are supported.
 *
 * @param ch character
 * @param radix radix
 * @param digit resulting digit
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_char_to_digit(FimoChar ch, FimoU32 radix, FimoU32 *digit);

/**
 * Returns the number of bytes that would be required to encode
 * this characters in UTF-8.
 *
 * @param ch character
 *
 * @return Number of bytes.
 */
FIMO_MUST_USE
FimoUSize fimo_char_len_utf8(FimoChar ch);

/**
 * Returns the number of bytes that would be required to encode
 * this characters in UTF-16.
 *
 * @param ch character
 *
 * @return Number of bytes.
 */
FIMO_MUST_USE
FimoUSize fimo_char_len_utf16(FimoChar ch);

/**
 * Encodes the character as UTF-8 into the provided byte buffer.
 *
 * A buffer length of `4` suffices for encoding any Unicode
 * character.
 *
 * @param ch character
 * @param buff buffer to encode the character into
 * @param buff_len length of the buffer
 * @param utf8_len resulting length of the UTF-8 character
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_char_encode_utf8(FimoChar ch, char *buff, FimoUSize buff_len, FimoUSize *utf8_len);

/**
 * Encodes the character as UTF-16 into the provided buffer.
 *
 * A buffer length of `2` suffices for encoding any Unicode
 * character.
 *
 * @param ch character
 * @param buff buffer to encode the character into
 * @param buff_len length of the buffer
 * @param utf16_len resulting length of the UTF-16 character
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_char_encode_utf16(FimoChar ch, FimoU16 *buff, FimoUSize buff_len, FimoUSize *utf16_len);

/**
 * Returns whether the character has the `Alphabetic` property.
 *
 * `Alphabetic` is described in Chapter 4 (Character Properties)
 * of the Unicode Standard and specified in the Unicode Character
 * Database `DerivedCoreProperties.txt`.
 *
 * @param ch character
 *
 * @return `true` is `ch` is `Alphabetic`.
 */
FIMO_MUST_USE
bool fimo_char_is_alphabetic(FimoChar ch);

/**
 * Returns whether the character has the `Lowercase` property.
 *
 * `Lowercase` is described in Chapter 4 (Character Properties)
 * of the Unicode Standard and specified in the Unicode Character
 * Database `DerivedCoreProperties.txt`.
 *
 * @param ch character
 *
 * @return `true` is `ch` is `Lowercase`.
 */
FIMO_MUST_USE
bool fimo_char_is_lowercase(FimoChar ch);

/**
 * Returns whether the character has the `Uppercase` property.
 *
 * `Uppercase` is described in Chapter 4 (Character Properties)
 * of the Unicode Standard and specified in the Unicode Character
 * Database `DerivedCoreProperties.txt`.
 *
 * @param ch character
 *
 * @return `true` is `ch` is `Uppercase`.
 */
FIMO_MUST_USE
bool fimo_char_is_uppercase(FimoChar ch);

/**
 * Returns whether the character has the `White_Space` property.
 *
 * `White_Space` is specified in the Unicode Character Database
 * `PropList.txt`.
 *
 * @param ch character
 *
 * @return `true` is `ch` is `White_Space`.
 */
FIMO_MUST_USE
bool fimo_char_is_whitespace(FimoChar ch);

/**
 * Returns whether the character is either `Alphabetic` or `Numeric`.
 *
 * @param ch character
 *
 * @return `true` if at least one of the properties is satisfied.
 */
FIMO_MUST_USE
bool fimo_char_is_alphanumeric(FimoChar ch);

/**
 * Returns whether the character has the general category for control codes.
 *
 * Control codes (code points with the general category of Cc) are described
 * in Chapter 4 (Character Properties) of the Unicode Standard and specified
 * in the Unicode Character Database `UnicodeData.txt`.
 *
 * @param ch character
 *
 * @return `true` if the character has the general category for control codes.
 */
FIMO_MUST_USE
bool fimo_char_is_control(FimoChar ch);

/**
 * Returns whether the character has one of the general categories for numbers.
 *
 * The general categories for numbers (Nd for decimal digits, Nl for
 * letter-like numeric characters, and No for other numeric characters)
 * are specified in the Unicode Character Database `UnicodeData.txt`.
 *
 * @param ch character
 *
 * @return `true` if the character has one of the general categories for numbers.
 */
FIMO_MUST_USE
bool fimo_char_is_numeric(FimoChar ch);

/**
 * Returns an iterator that yields the lowercase mapping of this
 * character as one or more characters.
 *
 * @param ch character
 *
 * @return Character mapping iterator.
 */
FIMO_MUST_USE
FimoCharCaseMapper fimo_char_to_lowercase(FimoChar ch);

/**
 * Returns an iterator that yields the uppercase mapping of this
 * character as one or more characters.
 *
 * @param ch character
 *
 * @return Character mapping iterator.
 */
FIMO_MUST_USE
FimoCharCaseMapper fimo_char_to_uppercase(FimoChar ch);

/**
 * Checks whether the character is within the ASCII range.
 *
 * @param ch character
 *
 * @return `true` if the character is within ASCII range.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii(FimoChar ch);

/**
 * Makes a copy of the value in its ASCII upper case equivalent.
 *
 * Non-ASCII letters are unchanged.
 *
 * @param ch character
 *
 * @return Modified character
 */
FIMO_MUST_USE
FimoChar fimo_char_to_ascii_uppercase(FimoChar ch);

/**
 * Makes a copy of the value in its ASCII lower case equivalent.
 *
 * Non-ASCII letters are unchanged.
 *
 * @param ch character
 *
 * @return Modified character
 */
FIMO_MUST_USE
FimoChar fimo_char_to_ascii_lowercase(FimoChar ch);

/**
 * Checks that two values are an ASCII case-insensitive match.
 *
 * @param ch character
 * @param other other character
 *
 * @return `true` if the characters are a case-insensitive match.
 */
FIMO_MUST_USE
bool fimo_char_eq_ignore_ascii_case(FimoChar ch, FimoChar other);

/**
 * Checks if the character is an ASCII alphabetic character.
 *
 * @param ch character
 *
 * @return `true` if the character is a alphabetic character.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_alphabetic(FimoChar ch);

/**
 * Checks if the character is an ASCII uppercase character.
 *
 * @param ch character
 *
 * @return `true` if the character is a uppercase character.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_uppercase(FimoChar ch);

/**
 * Checks if the character is an ASCII lowercase character.
 *
 * @param ch character
 *
 * @return `true` if the character is a lowercase character.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_lowercase(FimoChar ch);

/**
 * Checks if the character is an ASCII alphanumeric character.
 *
 * @param ch character
 *
 * @return `true` if the character is an alphanumeric character.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_alphanumeric(FimoChar ch);

/**
 * Checks if the character is an ASCII digit.
 *
 * @param ch character
 *
 * @return `true` if the character is a digit.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_digit(FimoChar ch);

/**
 * Checks if the character is an ASCII octal digit.
 *
 * @param ch character
 *
 * @return `true` if the character is an octal digit.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_octdigit(FimoChar ch);

/**
 * Checks if the character is an ASCII hexadecimal digit.
 *
 * @param ch character
 *
 * @return `true` if the character is a hexadecimal digit.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_hexdigit(FimoChar ch);

/**
 * Checks if the value is an ASCII punctuation character.
 *
 * @param ch character
 *
 * @return `true` if the character is a punctuation character.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_punctuation(FimoChar ch);

/**
 * Checks if the value is an ASCII graphic character: U+0021 ‘!’ ..= U+007E ‘~’.
 *
 * @param ch character
 *
 * @return `true` if the character is graphic.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_graphic(FimoChar ch);

/**
 * Checks if the character is an ASCII whitespace character.
 *
 * @param ch character
 *
 * @return `true` if the character is an ASCII whitespace character.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_whitespace(FimoChar ch);

/**
 * Checks if the character is an ASCII control character.
 *
 * @param ch character
 *
 * @return `true` if the character is an ASCII control character.
 */
FIMO_MUST_USE
bool fimo_char_is_ascii_control(FimoChar ch);

/**
 * Returns the length of the iterator.
 *
 * @param mapper iterator (not `NULL`)
 *
 * @return Iterator length.
 */
FIMO_MUST_USE
FimoUSize fimo_char_case_mapper_len(const FimoCharCaseMapper *mapper);

/**
 * Returns the next character in the iterator.
 *
 * @param mapper iterator (not `NULL`)
 * @param ch resulting character
 *
 * @return `true` if the iterator contained a character
 */
FIMO_MUST_USE
bool fimo_char_case_mapper_next(FimoCharCaseMapper *mapper, FimoChar *ch);

/**
 * Returns the next character in the iterator from the end.
 *
 * @param mapper iterator (not `NULL`)
 * @param ch resulting character
 *
 * @return `true` if the iterator contained a character
 */
FIMO_MUST_USE
bool fimo_char_case_mapper_next_back(FimoCharCaseMapper *mapper, FimoChar *ch);

#ifdef __cplusplus
}
#endif

#endif // FIMO_CHAR_H
