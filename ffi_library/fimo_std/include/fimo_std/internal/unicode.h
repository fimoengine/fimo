#ifndef FIMO_INTERNAL_UNICODE_H
#define FIMO_INTERNAL_UNICODE_H
// This file is generated by src/unicode-table-generator; do not edit manually!

#include <stdbool.h>

#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

#define FIMO_INTERNAL_UNICODE_VERSION_MAJOR 15
#define FIMO_INTERNAL_UNICODE_VERSION_MINOR 1
#define FIMO_INTERNAL_UNICODE_VERSION_UPDATE 0

#ifdef FIMO_INTERNAL_EXPOSE_UNICODE

typedef FimoU32 FimoChar;

bool fimo_internal_unicode_alphabetic_lookup(FimoChar ch);
bool fimo_internal_unicode_case_ignorable_lookup(FimoChar ch);
bool fimo_internal_unicode_cased_lookup(FimoChar ch);
bool fimo_internal_unicode_cc_lookup(FimoChar ch);
bool fimo_internal_unicode_lowercase_lookup(FimoChar ch);
bool fimo_internal_unicode_n_lookup(FimoChar ch);
bool fimo_internal_unicode_uppercase_lookup(FimoChar ch);
bool fimo_internal_unicode_whitespace_lookup(FimoChar ch);

struct FimoUnicodeCharTriple {
    FimoChar ch[3];
};

struct FimoUnicodeCharTriple fimo_internal_unicode_to_lower(FimoChar ch);
struct FimoUnicodeCharTriple fimo_internal_unicode_to_upper(FimoChar ch);

#endif

#ifdef __cplusplus
}
#endif

#endif // FIMO_INTERNAL_UNICODE_H
