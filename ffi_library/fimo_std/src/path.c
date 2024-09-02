#include <fimo_std/path.h>

#if _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#define PATH_SEPARATOR '\\'
#define PATH_SEPARATOR_STR "\\"
#else
#define PATH_SEPARATOR '/'
#define PATH_SEPARATOR_STR "/"
#endif

#include <string.h>

#include <fimo_std/memory.h>

///////////////////////////////////////////////////////////////////////
//// UTF-8 Validation
///////////////////////////////////////////////////////////////////////

// Copyright (c) 2008-2009 Bjoern Hoehrmann <bjoern@hoehrmann.de>
// See http://bjoern.hoehrmann.de/utf-8/decoder/dfa/ for details.

#define UTF8_ACCEPT 0
#define UTF8_REJECT 1

static const uint8_t utf8d[] = {
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0, // 00..1f
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0, // 20..3f
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0, // 40..5f
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
        0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0, // 60..7f
        1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,
        9,   9,   9,   9,   9,   9,   9,   9,   9,   9,   9,   9,   9,   9,   9,   9, // 80..9f
        7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,
        7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7,   7, // a0..bf
        8,   8,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,
        2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2,   2, // c0..df
        0xa, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x3, 0x4, 0x3, 0x3, // e0..ef
        0xb, 0x6, 0x6, 0x6, 0x5, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, 0x8, // f0..ff
        0x0, 0x1, 0x2, 0x3, 0x5, 0x8, 0x7, 0x1, 0x1, 0x1, 0x4, 0x6, 0x1, 0x1, 0x1, 0x1, // s0..s0
        1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,
        1,   0,   1,   1,   1,   1,   1,   0,   1,   0,   1,   1,   1,   1,   1,   1, // s1..s2
        1,   2,   1,   1,   1,   1,   1,   2,   1,   2,   1,   1,   1,   1,   1,   1,
        1,   1,   1,   1,   1,   1,   1,   2,   1,   1,   1,   1,   1,   1,   1,   1, // s3..s4
        1,   2,   1,   1,   1,   1,   1,   1,   1,   2,   1,   1,   1,   1,   1,   1,
        1,   1,   1,   1,   1,   1,   1,   3,   1,   3,   1,   1,   1,   1,   1,   1, // s5..s6
        1,   3,   1,   1,   1,   1,   1,   3,   1,   3,   1,   1,   1,   1,   1,   1,
        1,   3,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1,   1, // s7..s8
};

static uint32_t decode_(uint32_t *state, uint32_t *codep, uint32_t byte) {
    uint32_t type = utf8d[byte];

    *codep = (*state != UTF8_ACCEPT) ? (byte & 0x3fu) | (*codep << 6) : (0xff >> type) & (byte);

    *state = utf8d[256 + *state * 16 + type];
    return *state;
}

static bool is_valid_utf8(const char *str) {
    uint32_t codepoint;
    uint32_t state = 0;

    for (; *str; ++str) {
        FimoU8 byte = *str;
        decode_(&state, &codepoint, byte);

        if (state == UTF8_REJECT) {
            return false;
        }
    }

    return true;
}

///////////////////////////////////////////////////////////////////////
//// Path Iterator
///////////////////////////////////////////////////////////////////////

// Derived from the Rust project, licensed as MIT and Apache License (Version 2.0).

// Forward declarations.
static bool is_separator_(char c);
static bool is_separator_verbatim_(char c);
#if _WIN32
static const char *next_separator_(const char *string, FimoUSize length);
static const char *next_separator_verbatim_(const char *string, FimoUSize length);
#endif
static bool parse_prefix_(FimoUTF8Path path, FimoUTF8PathPrefix *prefix);
static bool prefix_is_verbatim_(FimoUTF8PathPrefix prefix);
static bool prefix_is_drive_(FimoUTF8PathPrefix prefix);
static bool prefix_has_implicit_root_(FimoUTF8PathPrefix prefix);
static FimoUSize prefix_length_(FimoUTF8PathPrefix prefix);
static bool has_root_separator_(FimoUTF8Path path, bool has_prefix, FimoUTF8PathPrefix prefix);

static bool it_prefix_len_(const FimoUTF8PathComponentIterator *it);
static bool it_prefix_is_verbatim_(const FimoUTF8PathComponentIterator *it);
static FimoUSize it_prefix_remaining_(const FimoUTF8PathComponentIterator *it);
static FimoUSize it_len_before_body_(const FimoUTF8PathComponentIterator *it);
static bool it_finished_(const FimoUTF8PathComponentIterator *it);
static bool it_is_separator_(const FimoUTF8PathComponentIterator *it, char c);
static bool it_has_root_(const FimoUTF8PathComponentIterator *it);
static bool it_include_current_dir_(const FimoUTF8PathComponentIterator *it);
static bool it_parse_single_component_(const FimoUTF8PathComponentIterator *it, FimoUTF8Path slice,
                                       FimoUTF8PathComponent *component);
static bool it_find_next_separator_(const FimoUTF8PathComponentIterator *it, FimoUSize *position);
static bool it_find_next_separator_back_(const FimoUTF8PathComponentIterator *it, FimoUSize *position);
static bool it_parse_next_component_(const FimoUTF8PathComponentIterator *it, FimoUSize *parsed_bytes,
                                     FimoUTF8PathComponent *component);
static bool it_parse_next_component_back_(const FimoUTF8PathComponentIterator *it, FimoUSize *parsed_bytes,
                                          FimoUTF8PathComponent *component);
static void it_trim_left_(FimoUTF8PathComponentIterator *it);
static void it_trim_right_(FimoUTF8PathComponentIterator *it);
static FimoUTF8Path it_as_path_(const FimoUTF8PathComponentIterator *it);
static bool it_next_component_(FimoUTF8PathComponentIterator *it, FimoUTF8PathComponent *component);
static bool it_next_component_back_(FimoUTF8PathComponentIterator *it, FimoUTF8PathComponent *component);

// Implementation.

static bool is_separator_(char c) {
#if _WIN32
    return c == PATH_SEPARATOR || c == '/';
#else
    return c == PATH_SEPARATOR;
#endif
}

static bool is_separator_verbatim_(char c) { return c == PATH_SEPARATOR; }

#if _WIN32
static const char *next_separator_(const char *string, FimoUSize length) {
    for (FimoUSize i = 0; i < length; ++string, ++i) {
        if (is_separator_(*string)) {
            return string;
        }
    }

    return NULL;
}

static const char *next_separator_verbatim_(const char *string, FimoUSize length) {
    return memchr(string, PATH_SEPARATOR, length);
}
#endif

static bool parse_prefix_(FimoUTF8Path path, FimoUTF8PathPrefix *prefix) {
    FIMO_DEBUG_ASSERT(path.path && prefix)
#if _WIN32
    // Verbatim prefix `\\?\...`
    if (path.length >= 4 && strncmp(path.path, "\\\\?\\", 4) == 0) {
        FimoUTF8Path rest = {.path = path.path + 4, .length = path.length - 4};

        // UNC prefix `hostname\share_name`.
        if (rest.length >= 4 && strncmp(rest.path, "UNC\\", 4) == 0) {
            rest.path += 4;
            rest.length -= 4;

            // Separate hostname and share name.
            const char *separator_pos = next_separator_verbatim_(rest.path, rest.length);
            if (separator_pos == NULL) {
                *prefix = (FimoUTF8PathPrefix){
                        .type = FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC,
                        .data = {.verbatim_unc =
                                         {
                                                 .hostname = rest,
                                                 .share_name = {.path = "", .length = 0},
                                         }},
                };
            }
            else {
                FimoUSize hostname_length = separator_pos - rest.path;
                FimoUTF8Path hostname = {.path = rest.path, .length = hostname_length};
                rest.path += hostname_length + 1;
                rest.length -= hostname_length + 1;

                separator_pos = next_separator_verbatim_(rest.path, rest.length);
                if (separator_pos == NULL) {
                    *prefix = (FimoUTF8PathPrefix){
                            .type = FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC,
                            .data = {.verbatim_unc =
                                             {
                                                     .hostname = hostname,
                                                     .share_name = rest,
                                             }},
                    };
                }
                else {
                    FimoUSize share_name_length = separator_pos - rest.path;
                    FimoUTF8Path share_name = {.path = rest.path, .length = share_name_length};
                    *prefix = (FimoUTF8PathPrefix){
                            .type = FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC,
                            .data = {.verbatim_unc =
                                             {
                                                     .hostname = hostname,
                                                     .share_name = share_name,
                                             }},
                    };
                }
            }

            return true;
        }

        // Drive prefix `C:`.
        if (rest.length >= 2 && rest.path[1] == ':') {
            *prefix = (FimoUTF8PathPrefix){
                    .type = FIMO_UTF8_PATH_PREFIX_VERBATIM_DISK,
                    .data = {.verbatim_disk = rest.path[0]},
            };
            return true;
        }

        // Normal prefix.
        const char *separator_pos = next_separator_verbatim_(rest.path, rest.length);
        if (separator_pos == NULL) {
            *prefix = (FimoUTF8PathPrefix){
                    .type = FIMO_UTF8_PATH_PREFIX_VERBATIM,
                    .data = {.verbatim = rest},
            };
        }
        else {
            FimoUSize prefix_length = separator_pos - rest.path;
            *prefix = (FimoUTF8PathPrefix){
                    .type = FIMO_UTF8_PATH_PREFIX_VERBATIM,
                    .data = {.verbatim = {.path = rest.path, .length = prefix_length}},
            };
        }

        return true;
    }

    // Device NS `\\.\NS`.
    if (path.length >= 4 && is_separator_(path.path[0]) && is_separator_(path.path[1]) && path.path[2] == '.' &&
        is_separator_(path.path[3])) {
        FimoUTF8Path rest = {.path = path.path + 4, .length = path.length - 4};
        const char *separator_pos = next_separator_(rest.path, rest.length);
        if (separator_pos == NULL) {
            *prefix = (FimoUTF8PathPrefix){
                    .type = FIMO_UTF8_PATH_PREFIX_DEVICE_NS,
                    .data = {.device_ns = rest},
            };
        }
        else {
            FimoUSize ns_length = separator_pos - rest.path;
            *prefix = (FimoUTF8PathPrefix){
                    .type = FIMO_UTF8_PATH_PREFIX_DEVICE_NS,
                    .data = {.device_ns = {.path = rest.path, .length = ns_length}},
            };
        }

        return true;
    }

    // UNC `\\hostname\share_name`
    if (path.length >= 2 && is_separator_(path.path[0]) && is_separator_(path.path[1])) {
        FimoUTF8Path rest = {.path = path.path + 2, .length = path.length - 2};

        // Separate hostname and share name.
        const char *separator_pos = next_separator_(rest.path, rest.length);
        if (separator_pos == NULL) {
            *prefix = (FimoUTF8PathPrefix){
                    .type = FIMO_UTF8_PATH_PREFIX_UNC,
                    .data = {.unc =
                                     {
                                             .hostname = rest,
                                             .share_name = {.path = "", .length = 0},
                                     }},
            };
        }
        else {
            FimoUSize hostname_length = separator_pos - rest.path;
            FimoUTF8Path hostname = {.path = rest.path, .length = hostname_length};
            rest.path += hostname_length + 1;
            rest.length -= hostname_length + 1;

            separator_pos = next_separator_(rest.path, rest.length);
            if (separator_pos == NULL) {
                *prefix = (FimoUTF8PathPrefix){
                        .type = FIMO_UTF8_PATH_PREFIX_UNC,
                        .data = {.unc =
                                         {
                                                 .hostname = hostname,
                                                 .share_name = rest,
                                         }},
                };
            }
            else {
                FimoUSize share_name_length = separator_pos - rest.path;
                FimoUTF8Path share_name = {.path = rest.path, .length = share_name_length};
                *prefix = (FimoUTF8PathPrefix){
                        .type = FIMO_UTF8_PATH_PREFIX_UNC,
                        .data = {.unc =
                                         {
                                                 .hostname = hostname,
                                                 .share_name = share_name,
                                         }},
                };
            }
        }

        return false;
    }

    // Disk `C:`.
    if (path.length >= 2 && path.path[1] == ':') {
        *prefix = (FimoUTF8PathPrefix){
                .type = FIMO_UTF8_PATH_PREFIX_DISK,
                .data = {.disk = path.path[0]},
        };
        return true;
    }

    return false;
#else
    (void)path;
    (void)prefix;
    return false;
#endif
}

static bool prefix_is_verbatim_(FimoUTF8PathPrefix prefix) {
    switch (prefix.type) {
        case FIMO_UTF8_PATH_PREFIX_VERBATIM:
        case FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC:
        case FIMO_UTF8_PATH_PREFIX_VERBATIM_DISK:
            return true;
        case FIMO_UTF8_PATH_PREFIX_DEVICE_NS:
        case FIMO_UTF8_PATH_PREFIX_UNC:
        case FIMO_UTF8_PATH_PREFIX_DISK:
            return false;
    }

    return false;
}

static bool prefix_is_drive_(FimoUTF8PathPrefix prefix) { return prefix.type == FIMO_UTF8_PATH_PREFIX_DISK; }

static bool prefix_has_implicit_root_(FimoUTF8PathPrefix prefix) { return prefix.type == FIMO_UTF8_PATH_PREFIX_DISK; }

static FimoUSize prefix_length_(FimoUTF8PathPrefix prefix) {
    switch (prefix.type) {
        case FIMO_UTF8_PATH_PREFIX_VERBATIM:
            return prefix.data.verbatim.length + 4;
        case FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC:
            if (prefix.data.verbatim_unc.share_name.length == 0) {
                return prefix.data.verbatim_unc.hostname.length + 8;
            }
            return prefix.data.verbatim_unc.hostname.length + prefix.data.verbatim_unc.share_name.length + 9;
        case FIMO_UTF8_PATH_PREFIX_VERBATIM_DISK:
            return 6;
        case FIMO_UTF8_PATH_PREFIX_DEVICE_NS:
            return prefix.data.device_ns.length + 4;
        case FIMO_UTF8_PATH_PREFIX_UNC:
            if (prefix.data.unc.share_name.length == 0) {
                return prefix.data.unc.hostname.length + 2;
            }
            return prefix.data.unc.hostname.length + prefix.data.unc.share_name.length + 3;
        case FIMO_UTF8_PATH_PREFIX_DISK:
            return 2;
    }

    return 0;
}

static bool has_root_separator_(FimoUTF8Path path, bool has_prefix, FimoUTF8PathPrefix prefix) {
    if (!has_prefix) {
        return path.length >= 1 && is_separator_(path.path[0]);
    }

#if _WIN32
    FimoUSize prefix_length = prefix_length_(prefix);
    if (path.length > prefix_length) {
        return (prefix_is_verbatim_(prefix) && is_separator_verbatim_(path.path[prefix_length])) ||
               is_separator_(path.path[prefix_length]);
    }
#else
    (void)prefix;
#endif
    return false;
}

static bool it_prefix_len_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    if (it->has_prefix) {
        return prefix_length_(it->prefix);
    }
    return 0;
}

static bool it_prefix_is_verbatim_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    if (it->has_prefix) {
        return prefix_is_verbatim_(it->prefix);
    }
    return false;
}

static FimoUSize it_prefix_remaining_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    if (it->front == FIMO_UTF8_PATH_COMPONENT_ITER_STATE_PREFIX) {
        return it_prefix_len_(it);
    }
    return 0;
}

static FimoUSize it_len_before_body_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    FimoUSize root = it->front <= FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR && it->has_root_separator ? 1 : 0;
    FimoUSize cur_dir =
            it->front <= FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR && it_include_current_dir_(it) ? 1 : 0;
    return it_prefix_remaining_(it) + root + cur_dir;
}

static bool it_finished_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    return it->front == FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE ||
           it->back == FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE || it->front > it->back;
}

static bool it_is_separator_(const FimoUTF8PathComponentIterator *it, char c) {
    FIMO_DEBUG_ASSERT(it)
    if (it_prefix_is_verbatim_(it)) {
        return is_separator_verbatim_(c);
    }
    return is_separator_(c);
}

static bool it_has_root_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    if (it->has_root_separator) {
        return true;
    }
    if (it->has_prefix) {
        return prefix_has_implicit_root_(it->prefix);
    }
    return false;
}

static bool it_include_current_dir_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    if (it_has_root_(it)) {
        return false;
    }
    FimoUSize prefix_remaining = it_prefix_remaining_(it);
    FimoUTF8Path rest = {.path = it->current.path + prefix_remaining, .length = it->current.length - prefix_remaining};
    if (rest.length == 1) {
        return rest.path[0] == '.';
    }
    if (rest.length >= 2 && rest.path[0] == '.') {
        return it_is_separator_(it, rest.path[1]);
    }
    return false;
}

static bool it_parse_single_component_(const FimoUTF8PathComponentIterator *it, FimoUTF8Path slice,
                                       FimoUTF8PathComponent *component) {
    FIMO_DEBUG_ASSERT(it && slice.path && component)
    if (slice.length == 0) {
        return false;
    }

    if (slice.length == 1 && slice.path[0] == '.') {
        if (it_prefix_is_verbatim_(it)) {
            *component = (FimoUTF8PathComponent){
                    .type = FIMO_UTF8_PATH_COMPONENT_CUR_DIR,
                    .data = {.cur_dir = 0},
            };
            return true;
        }

        return false;
    }

    if (slice.length == 2 && slice.path[0] == '.' && slice.path[1] == '.') {
        *component = (FimoUTF8PathComponent){
                .type = FIMO_UTF8_PATH_COMPONENT_PARENT_DIR,
                .data = {.parent_dir = 0},
        };
        return true;
    }

    *component = (FimoUTF8PathComponent){
            .type = FIMO_UTF8_PATH_COMPONENT_NORMAL,
            .data = {.normal = slice},
    };
    return true;
}

static bool it_find_next_separator_(const FimoUTF8PathComponentIterator *it, FimoUSize *position) {
    FIMO_DEBUG_ASSERT(it && position)
    for (FimoUSize i = 0; i < it->current.length; i++) {
        if (it_is_separator_(it, it->current.path[i])) {
            *position = i;
            return true;
        }
    }
    return false;
}

static bool it_find_next_separator_back_(const FimoUTF8PathComponentIterator *it, FimoUSize *position) {
    FIMO_DEBUG_ASSERT(it && position)
    FimoUSize start = it_len_before_body_(it);
    FimoUTF8Path offset_path = it->current;
    offset_path.path += start;
    offset_path.length -= start;

    for (FimoUSize i = 0; i < offset_path.length; i++) {
        if (it_is_separator_(it, offset_path.path[offset_path.length - i - 1])) {
            *position = offset_path.length - i - 1;
            return true;
        }
    }
    return false;
}

static bool it_parse_next_component_(const FimoUTF8PathComponentIterator *it, FimoUSize *parsed_bytes,
                                     FimoUTF8PathComponent *component) {
    FIMO_DEBUG_ASSERT(it && it->front == FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY && parsed_bytes && component)
    FimoUSize separator_position = 0;
    bool has_separator = it_find_next_separator_(it, &separator_position);

    FimoUSize extra = has_separator ? 1 : 0;
    FimoUTF8Path slice = it->current;
    slice.length = has_separator ? separator_position : slice.length;

    *parsed_bytes = extra + slice.length;
    return it_parse_single_component_(it, slice, component);
}

static bool it_parse_next_component_back_(const FimoUTF8PathComponentIterator *it, FimoUSize *parsed_bytes,
                                          FimoUTF8PathComponent *component) {
    FIMO_DEBUG_ASSERT(it && it->back == FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY && parsed_bytes && component)
    FimoUSize start = it_len_before_body_(it);
    FimoUSize separator_position = 0;
    bool has_separator = it_find_next_separator_back_(it, &separator_position);

    FimoUSize extra = has_separator ? 1 : 0;
    FimoUTF8Path slice = it->current;
    slice.path += has_separator ? start + separator_position + 1 : start;
    slice.length -= has_separator ? start + separator_position + 1 : start;

    *parsed_bytes = extra + slice.length;
    return it_parse_single_component_(it, slice, component);
}

static void it_trim_left_(FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    while (it->current.length > 0) {
        FimoUSize parsed_bytes;
        FimoUTF8PathComponent parsed_component;
        bool has_component = it_parse_next_component_(it, &parsed_bytes, &parsed_component);
        if (has_component) {
            return;
        }
        it->current.path += parsed_bytes;
        it->current.length -= parsed_bytes;
    }
}

static void it_trim_right_(FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    while (it->current.length > it_len_before_body_(it)) {
        FimoUSize parsed_bytes;
        FimoUTF8PathComponent parsed_component;
        bool has_component = it_parse_next_component_back_(it, &parsed_bytes, &parsed_component);
        if (has_component) {
            return;
        }
        it->current.length -= parsed_bytes;
    }
}

static FimoUTF8Path it_as_path_(const FimoUTF8PathComponentIterator *it) {
    FIMO_DEBUG_ASSERT(it)
    FimoUTF8PathComponentIterator iter = *it;
    if (iter.front == FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY) {
        it_trim_left_(&iter);
    }
    if (iter.back == FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY) {
        it_trim_right_(&iter);
    }
    return iter.current;
}

static bool it_next_component_(FimoUTF8PathComponentIterator *it, FimoUTF8PathComponent *component) {
    FIMO_DEBUG_ASSERT(it && component)
    while (!it_finished_(it)) {
        switch (it->front) {
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_PREFIX: {
                if (it->has_prefix) {
                    it->front = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR;
                    FimoUSize prefix_length = prefix_length_(it->prefix);
                    FimoUTF8Path raw = {.path = it->current.path, .length = prefix_length};
                    it->current.path += prefix_length;
                    it->current.length -= prefix_length;
                    *component = (FimoUTF8PathComponent){
                            .type = FIMO_UTF8_PATH_COMPONENT_PREFIX,
                            .data = {.prefix = {.raw = raw, .prefix = it->prefix}},
                    };
                    return true;
                }
                it->front = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR;
                break;
            }
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR: {
                it->front = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY;
                if (it->has_root_separator) {
                    FIMO_DEBUG_ASSERT(it->current.length > 0);
                    it->current.path += 1;
                    it->current.length -= 1;
                    *component = (FimoUTF8PathComponent){
                            .type = FIMO_UTF8_PATH_COMPONENT_ROOT_DIR,
                            .data = {.root_dir = 0},
                    };
                    return true;
                }
                if (it->has_prefix) {
                    if (prefix_has_implicit_root_(it->prefix) && !prefix_is_verbatim_(it->prefix)) {
                        *component = (FimoUTF8PathComponent){
                                .type = FIMO_UTF8_PATH_COMPONENT_ROOT_DIR,
                                .data = {.root_dir = 0},
                        };
                        return true;
                    }
                }
                else if (it_include_current_dir_(it)) {
                    FIMO_DEBUG_ASSERT(it->current.length > 0);
                    it->current.path += 1;
                    it->current.length -= 1;
                    *component = (FimoUTF8PathComponent){
                            .type = FIMO_UTF8_PATH_COMPONENT_CUR_DIR,
                            .data = {.cur_dir = 0},
                    };
                    return true;
                }
                break;
            }
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY: {
                if (it->current.length > 0) {
                    FimoUSize parsed_bytes = 0;
                    bool has_component = it_parse_next_component_(it, &parsed_bytes, component);
                    it->current.path += parsed_bytes;
                    it->current.length -= parsed_bytes;
                    return has_component;
                }

                it->front = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE;
                break;
            }
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE:
                FIMO_ASSERT(false)
        }
    }
    return false;
}

static bool it_next_component_back_(FimoUTF8PathComponentIterator *it, FimoUTF8PathComponent *component) {
    FIMO_DEBUG_ASSERT(it && component)
    while (!it_finished_(it)) {
        switch (it->back) {
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY: {
                if (it->current.length > it_len_before_body_(it)) {
                    FimoUSize parsed_bytes = 0;
                    bool has_component = it_parse_next_component_back_(it, &parsed_bytes, component);
                    it->current.length -= parsed_bytes;
                    if (has_component) {
                        return has_component;
                    }
                    continue;
                }
                it->back = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR;
                break;
            }
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR: {
                it->back = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_PREFIX;
                if (it->has_root_separator) {
                    it->current.path += it->current.length - 1;
                    it->current.length -= it->current.length - 1;
                    *component = (FimoUTF8PathComponent){
                            .type = FIMO_UTF8_PATH_COMPONENT_ROOT_DIR,
                            .data = {.root_dir = 0},
                    };
                    return true;
                }
                if (it->has_prefix) {
                    if (prefix_has_implicit_root_(it->prefix) && !prefix_is_verbatim_(it->prefix)) {
                        *component = (FimoUTF8PathComponent){
                                .type = FIMO_UTF8_PATH_COMPONENT_ROOT_DIR,
                                .data = {.root_dir = 0},
                        };
                        return true;
                    }
                }
                else if (it_include_current_dir_(it)) {
                    it->current.path += it->current.length - 1;
                    it->current.length -= it->current.length - 1;
                    *component = (FimoUTF8PathComponent){
                            .type = FIMO_UTF8_PATH_COMPONENT_CUR_DIR,
                            .data = {.cur_dir = 0},
                    };
                    return true;
                }
                break;
            }
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_PREFIX: {
                if (it_prefix_len_(it) > 0) {
                    it->back = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE;
                    *component = (FimoUTF8PathComponent){
                            .type = FIMO_UTF8_PATH_COMPONENT_PREFIX,
                            .data = {.prefix = {.raw = it->current, .prefix = it->prefix}},
                    };
                    return true;
                }

                it->back = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE;
                break;
            }
            case FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE:
                FIMO_ASSERT(false)
        }
    }
    return false;
}

///////////////////////////////////////////////////////////////////////
//// PathBuf implementation
///////////////////////////////////////////////////////////////////////

FIMO_MUST_USE
FimoUTF8PathBuf fimo_utf8_path_buf_new(void) { return (FimoUTF8PathBuf){.buffer = fimo_array_list_new()}; }

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_buf_with_capacity(FimoUSize capacity, FimoUTF8PathBuf *buf) {
    FIMO_DEBUG_ASSERT(buf)
    FimoArrayList buffer;
    FimoResult error = fimo_array_list_with_capacity(capacity, sizeof(char), _Alignof(char), &buffer);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    *buf = (FimoUTF8PathBuf){.buffer = buffer};
    return FIMO_EOK;
}

FIMO_EXPORT
void fimo_utf8_path_buf_free(FimoUTF8PathBuf *buf) {
    FIMO_DEBUG_ASSERT(buf)
    fimo_array_list_free(&buf->buffer, sizeof(char), _Alignof(char), NULL);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_utf8_path_buf_as_path(const FimoUTF8PathBuf *buf) {
    FIMO_DEBUG_ASSERT(buf)
    if (fimo_array_list_is_empty(&buf->buffer)) {
        return (FimoUTF8Path){.path = "", .length = 0};
    }
    return (FimoUTF8Path){.path = buf->buffer.elements, .length = buf->buffer.size};
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_buf_into_owned_path(FimoUTF8PathBuf *buf, FimoOwnedUTF8Path *owned) {
    FIMO_DEBUG_ASSERT(buf)
    FimoUSize length = buf->buffer.size;
    if (length == 0) {
        fimo_utf8_path_buf_free(buf);
        FimoResult error = fimo_owned_utf8_path_from_string("", owned);
        return error;
    }

    FimoResult error =
            fimo_array_list_set_capacity_exact(&buf->buffer, sizeof(char), _Alignof(char), length, NULL, NULL);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }
    *owned = (FimoOwnedUTF8Path){.path = buf->buffer.elements, .length = length};
    return FIMO_EOK;
}

static FimoResult push_path_str_(FimoArrayList *buffer, FimoUTF8Path path) {
    FIMO_DEBUG_ASSERT(buffer && path.path)
    if (path.length == 0) {
        return FIMO_EOK;
    }

    FimoResult error = fimo_array_list_reserve(buffer, sizeof(char), _Alignof(char), path.length, NULL);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }
    char *start = ((char *)buffer->elements) + buffer->size;
    memcpy(start, path.path, path.length);
    FIMO_RESULT_IGNORE(fimo_array_list_set_len(buffer, buffer->size + path.length));
    return FIMO_EOK;
}

FIMO_EXPORT FIMO_MUST_USE FimoResult fimo_utf8_path_buf_push_path(FimoUTF8PathBuf *buf, FimoUTF8Path path) {
    FIMO_DEBUG_ASSERT(buf && path.path)
    bool need_sep = false;
    if (!fimo_array_list_is_empty(&buf->buffer)) {
        char c = ((const char *)buf->buffer.elements)[buf->buffer.size - 1];
        need_sep = !is_separator_(c);
    }

    FimoUTF8PathComponentIterator comps = fimo_utf8_path_component_iter_new(fimo_utf8_path_buf_as_path(buf));
    if (it_prefix_len_(&comps) > 0 && it_prefix_len_(&comps) == comps.current.length &&
        prefix_is_drive_(comps.prefix)) {
        need_sep = false;
    }

    if (fimo_utf8_path_is_absolute(path) || fimo_utf8_path_component_iter_new(path).has_prefix) {
        FIMO_RESULT_IGNORE(fimo_array_list_set_len(&buf->buffer, 0));
    }
    else if (it_prefix_is_verbatim_(&comps) && path.length != 0) {
        FimoArrayList buffer = fimo_array_list_new();
        {
            FimoResult error = FIMO_EOK;
            FimoUTF8PathComponent c;
            while (fimo_utf8_path_component_iter_next(&comps, &c)) {
                error = fimo_array_list_push(&buffer, sizeof(c), _Alignof(FimoUTF8PathComponent), &c, NULL);
                if (FIMO_RESULT_IS_ERROR(error)) {
                    fimo_array_list_free(&buffer, sizeof(c), _Alignof(FimoUTF8PathComponent), NULL);
                    return error;
                }
            }
        }
        {
            FimoResult error = FIMO_EOK;
            FimoUTF8PathComponent c;
            FimoUTF8PathComponentIterator iter = fimo_utf8_path_component_iter_new(path);
            while (fimo_utf8_path_component_iter_next(&iter, &c)) {
                switch (c.type) {
                    case FIMO_UTF8_PATH_COMPONENT_ROOT_DIR:
                        if (buffer.size > 1) {
                            FIMO_RESULT_IGNORE(fimo_array_list_set_len(&buffer, 1));
                        }
                        error = fimo_array_list_push(&buffer, sizeof(c), _Alignof(FimoUTF8PathComponent), &c, NULL);
                        if (FIMO_RESULT_IS_ERROR(error)) {
                            fimo_array_list_free(&buffer, sizeof(c), _Alignof(FimoUTF8PathComponent), NULL);
                            return error;
                        }
                        break;
                    case FIMO_UTF8_PATH_COMPONENT_CUR_DIR:
                        break;
                    case FIMO_UTF8_PATH_COMPONENT_PARENT_DIR: {
                        const FimoUTF8PathComponent *last;
                        error = fimo_array_list_peek_back(&buffer, sizeof(c), (const void **)&last);
                        if (FIMO_RESULT_IS_OK(error) && last->type == FIMO_UTF8_PATH_COMPONENT_NORMAL) {
                            FimoUTF8PathComponent tmp;
                            error = fimo_array_list_pop_front(&buffer, sizeof(c), &tmp, NULL);
                            if (FIMO_RESULT_IS_ERROR(error)) {
                                fimo_array_list_free(&buffer, sizeof(c), _Alignof(FimoUTF8PathComponent), NULL);
                                return error;
                            }
                        }
                        break;
                    }
                    default:
                        error = fimo_array_list_push(&buffer, sizeof(c), _Alignof(FimoUTF8PathComponent), &c, NULL);
                        if (FIMO_RESULT_IS_ERROR(error)) {
                            fimo_array_list_free(&buffer, sizeof(c), _Alignof(FimoUTF8PathComponent), NULL);
                            return error;
                        }
                }
            }
        }

        FimoArrayList res = fimo_array_list_new();
        bool need_sep_ = false;

        for (FimoUSize i = 0; i < buffer.size; i++) {
            FimoUTF8PathComponent c = ((FimoUTF8PathComponent *)buffer.elements)[i];
            if (need_sep_ && c.type != FIMO_UTF8_PATH_COMPONENT_ROOT_DIR) {
                FimoResult error = push_path_str_(&res, (FimoUTF8Path){
                                                                .path = PATH_SEPARATOR_STR,
                                                                .length = sizeof(PATH_SEPARATOR_STR) - 1,
                                                        });
                if (FIMO_RESULT_IS_ERROR(error)) {
                    fimo_array_list_free(&res, sizeof(char), _Alignof(char), NULL);
                    fimo_array_list_free(&buffer, sizeof(FimoUTF8PathComponent), _Alignof(FimoUTF8PathComponent), NULL);
                    return error;
                }
            }

            FimoResult error = push_path_str_(&res, fimo_utf8_path_component_as_path(&c));
            if (FIMO_RESULT_IS_ERROR(error)) {
                fimo_array_list_free(&res, sizeof(char), _Alignof(char), NULL);
                fimo_array_list_free(&buffer, sizeof(FimoUTF8PathComponent), _Alignof(FimoUTF8PathComponent), NULL);
                return error;
            }

            switch (c.type) {
                case FIMO_UTF8_PATH_COMPONENT_ROOT_DIR:
                    need_sep_ = false;
                    break;
                case FIMO_UTF8_PATH_COMPONENT_PREFIX:
                    need_sep_ = !prefix_is_drive_(c.data.prefix.prefix) && prefix_length_(c.data.prefix.prefix) > 0;
                    break;
                default:
                    need_sep_ = true;
            }
        }

        fimo_array_list_free(&buffer, sizeof(FimoUTF8PathComponent), _Alignof(FimoUTF8PathComponent), NULL);
        fimo_array_list_free(&buf->buffer, sizeof(char), _Alignof(char), NULL);
        buf->buffer = res;
    }
    else if (fimo_utf8_path_has_root(path)) {
        FimoUTF8PathComponentIterator iter = fimo_utf8_path_component_iter_new(fimo_utf8_path_buf_as_path(buf));
        FimoUSize prefix_length = it_prefix_remaining_(&iter);
        if (buf->buffer.size > prefix_length) {
            FIMO_RESULT_IGNORE(fimo_array_list_set_len(&buf->buffer, prefix_length));
        }
    }
    else if (need_sep) {
        FimoResult error = push_path_str_(&buf->buffer, (FimoUTF8Path){
                                                                .path = PATH_SEPARATOR_STR,
                                                                .length = sizeof(PATH_SEPARATOR_STR) - 1,
                                                        });
        if (FIMO_RESULT_IS_ERROR(error)) {
            return error;
        }
    }

    return push_path_str_(&buf->buffer, path);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_buf_push_string(FimoUTF8PathBuf *buf, const char *path) {
    FIMO_DEBUG_ASSERT(buf)
    FimoUTF8Path p;
    FimoResult error = fimo_utf8_path_new(path, &p);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }
    return fimo_utf8_path_buf_push_path(buf, p);
}

FIMO_EXPORT
bool fimo_utf8_path_buf_pop(FimoUTF8PathBuf *buf) {
    FIMO_DEBUG_ASSERT(buf)
    FimoUTF8Path parent;
    FimoUTF8Path p = fimo_utf8_path_buf_as_path(buf);
    bool has_parent = fimo_utf8_path_parent(p, &parent);
    if (has_parent) {
        FIMO_RESULT_IGNORE(fimo_array_list_set_len(&buf->buffer, parent.length));
    }
    return has_parent;
}

///////////////////////////////////////////////////////////////////////
//// OwnedPath implementation
///////////////////////////////////////////////////////////////////////

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_utf8_path_from_string(const char *path, FimoOwnedUTF8Path *owned) {
    FimoUTF8Path p;
    FimoResult error = fimo_utf8_path_new(path, &p);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }
    return fimo_owned_utf8_path_from_path(p, owned);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_utf8_path_from_path(FimoUTF8Path path, FimoOwnedUTF8Path *owned) {
    FIMO_DEBUG_ASSERT(path.path)
    if (owned == NULL) {
        return FIMO_RESULT_FROM_STRING("path is null");
    }

    if (path.length == 0) {
        *owned = (FimoOwnedUTF8Path){.path = "", .length = 0};
        return FIMO_EOK;
    }

    FimoResult error = FIMO_EOK;
    char *dst = fimo_malloc(path.length * sizeof(char), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }
    FIMO_PRAGMA_MSVC(warning(push))
    FIMO_PRAGMA_MSVC(warning(disable : 4996))
    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.strcpy)
    // ReSharper disable once CppDeprecatedEntity
    strncpy(dst, path.path, path.length);
    FIMO_PRAGMA_MSVC(warning(pop))
    *owned = (FimoOwnedUTF8Path){.path = dst, .length = path.length};

    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_utf8_path_from_os_path(FimoOSPath path, FimoOwnedUTF8Path *owned) {
    FIMO_DEBUG_ASSERT(path.path && owned)
#if _WIN32
    if (path.length >= INT_MAX) {
        return FIMO_RESULT_FROM_STRING("path is too long");
    }

    int path_len = WideCharToMultiByte(CP_UTF8, 0, path.path, (int)path.length + 1, NULL, 0, NULL, NULL);
    if (path_len <= 0) {
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }

    FimoResult error;
    char *utf8_path_str = fimo_malloc(path.length * sizeof(char), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    int multi_byte_conv_res =
            WideCharToMultiByte(CP_UTF8, 0, path.path, (int)path.length + 1, utf8_path_str, path_len, NULL, NULL);
    if (multi_byte_conv_res <= 0) {
        fimo_free(utf8_path_str);
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }
    *owned = (FimoOwnedUTF8Path){.path = utf8_path_str, .length = path_len - 1};

    return FIMO_EOK;
#else
    FimoUTF8Path p;
    FimoResult error = fimo_utf8_path_new(path.path, &p);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }
    return fimo_owned_utf8_path_from_path(p, owned);
#endif
}

FIMO_EXPORT void fimo_owned_utf8_path_free(FimoOwnedUTF8Path path) {
    FIMO_DEBUG_ASSERT(path.path)
    if (path.length > 0) {
        fimo_free(path.path);
    }
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_owned_utf8_path_as_path(FimoOwnedUTF8Path path) {
    FIMO_DEBUG_ASSERT(path.path)
    return (FimoUTF8Path){.path = path.path, .length = path.length};
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8PathBuf fimo_owned_utf8_path_into_path_buf(FimoOwnedUTF8Path path) {
    FIMO_DEBUG_ASSERT(path.path)
    if (path.length == 0) {
        return (FimoUTF8PathBuf){.buffer = fimo_array_list_new()};
    }
    return (FimoUTF8PathBuf){.buffer = {
                                     .elements = path.path,
                                     .size = path.length,
                                     .capacity = path.length,
                             }};
}

///////////////////////////////////////////////////////////////////////
//// Path implementation
///////////////////////////////////////////////////////////////////////

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_new(const char *path_str, FimoUTF8Path *path) {
    if (path_str == NULL) {
        return FIMO_RESULT_FROM_STRING("string can not be NULL");
    }
    if (path == NULL) {
        return FIMO_RESULT_FROM_STRING("path can not be NULL");
    }

    if (!is_valid_utf8(path_str)) {
        return FIMO_RESULT_FROM_STRING("invalid UTF-8 string");
    }

    *path = (FimoUTF8Path){
            .path = path_str,
            .length = strlen(path_str),
    };
    return FIMO_EOK;
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_is_absolute(FimoUTF8Path path) {
    FIMO_DEBUG_ASSERT(path.path)
#if _WIN32
    FimoUTF8PathComponentIterator components = fimo_utf8_path_component_iter_new(path);
    return components.has_prefix;
#else
    return fimo_utf8_path_has_root(path);
#endif
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_is_relative(FimoUTF8Path path) { return !fimo_utf8_path_is_absolute(path); }

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_has_root(FimoUTF8Path path) {
    FIMO_DEBUG_ASSERT(path.path)
    FimoUTF8PathComponentIterator components = fimo_utf8_path_component_iter_new(path);
    return it_has_root_(&components);
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_parent(FimoUTF8Path path, FimoUTF8Path *parent) {
    FIMO_DEBUG_ASSERT(path.path && parent)
    FimoUTF8PathComponentIterator components = fimo_utf8_path_component_iter_new(path);
    FimoUTF8PathComponent component;
    bool has_component = fimo_utf8_path_component_iter_next_back(&components, &component);
    if (has_component) {
        switch (component.type) {
            case FIMO_UTF8_PATH_COMPONENT_NORMAL:
            case FIMO_UTF8_PATH_COMPONENT_CUR_DIR:
            case FIMO_UTF8_PATH_COMPONENT_PARENT_DIR:
                *parent = fimo_utf8_path_component_iter_as_path(&components);
                return true;
            default:
                return false;
        }
    }

    return false;
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_file_name(FimoUTF8Path path, FimoUTF8Path *file_name) {
    FIMO_DEBUG_ASSERT(path.path && file_name)
    FimoUTF8PathComponentIterator components = fimo_utf8_path_component_iter_new(path);
    FimoUTF8PathComponent component;
    bool has_component = fimo_utf8_path_component_iter_next_back(&components, &component);
    if (has_component) {
        switch (component.type) {
            case FIMO_UTF8_PATH_COMPONENT_NORMAL:
                *file_name = component.data.normal;
                return true;
            default:
                return false;
        }
    }

    return false;
}

///////////////////////////////////////////////////////////////////////
//// OwnedOSPath implementation
///////////////////////////////////////////////////////////////////////

FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_os_path_from_path(FimoUTF8Path path, FimoOwnedOSPath *os_path) {
    FIMO_DEBUG_ASSERT(path.path && os_path)
#if _WIN32
    int os_path_len = MultiByteToWideChar(CP_UTF8, 0, path.path, (int)path.length + 1, NULL, 0);
    if (os_path_len <= 0) {
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }

    FimoResult error;
    wchar_t *os_path_str = fimo_malloc(os_path_len * sizeof(wchar_t), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    int wide_conv_res = MultiByteToWideChar(CP_UTF8, 0, path.path, (int)path.length + 1, os_path_str, os_path_len);
    if (wide_conv_res <= 0) {
        fimo_free(os_path_str);
        return FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(GetLastError());
    }
    *os_path = (FimoOwnedOSPath){.path = os_path_str, .length = os_path_len - 1};

    return FIMO_EOK;
#else
    FimoResult error;
    char *clone = fimo_malloc((path.length + 1) * sizeof(char), &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return error;
    }

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.strcpy)
    // ReSharper disable once CppDeprecatedEntity
    strncpy(clone, path.path, path.length);
    clone[path.length] = '\0';

    *os_path = (FimoOwnedOSPath){.path = clone, .length = path.length};
    return FIMO_EOK;
#endif
}

FIMO_EXPORT
void fimo_owned_os_path_free(FimoOwnedOSPath path) {
    FIMO_DEBUG_ASSERT(path.path)
    fimo_free(path.path);
}

FIMO_EXPORT
FIMO_MUST_USE
FimoOSPath fimo_owned_os_path_as_os_path(FimoOwnedOSPath path) {
    FIMO_DEBUG_ASSERT(path.path)
    return (FimoOSPath){.path = path.path, .length = path.length};
}

///////////////////////////////////////////////////////////////////////
//// OSPath implementation
///////////////////////////////////////////////////////////////////////

FIMO_EXPORT
FIMO_MUST_USE
FimoOSPath fimo_os_path_new(const FimoOSPathChar *path) {
    if (path == NULL) {
#if _WIN32
        return (FimoOSPath){.path = L"", .length = 0};
#else
        return (FimoOSPath){.path = "", .length = 0};
#endif
    }
    else {
#if _WIN32
        return (FimoOSPath){.path = path, .length = wcslen(path)};
#else
        return (FimoOSPath){.path = path, .length = strlen(path)};
#endif
    }
}

///////////////////////////////////////////////////////////////////////
//// PathComponentIterator implementation
///////////////////////////////////////////////////////////////////////

FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8PathComponentIterator fimo_utf8_path_component_iter_new(FimoUTF8Path path) {
    FIMO_DEBUG_ASSERT(path.path)

    FimoUTF8PathPrefix prefix = {0};
    bool has_prefix = parse_prefix_(path, &prefix);

    return (FimoUTF8PathComponentIterator){
            .current = {.path = path.path, .length = path.length},
            .has_prefix = has_prefix,
            .prefix = prefix,
            .has_root_separator = has_root_separator_(path, has_prefix, prefix),
            .front = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_PREFIX,
            .back = FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY,
    };
}

FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_utf8_path_component_iter_as_path(const FimoUTF8PathComponentIterator *iter) {
    return it_as_path_(iter);
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_component_iter_next(FimoUTF8PathComponentIterator *iterator, FimoUTF8PathComponent *component) {
    return it_next_component_(iterator, component);
}

FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_component_iter_next_back(FimoUTF8PathComponentIterator *iterator,
                                             FimoUTF8PathComponent *component) {
    return it_next_component_back_(iterator, component);
}

///////////////////////////////////////////////////////////////////////
//// PathComponent implementation
///////////////////////////////////////////////////////////////////////

FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_utf8_path_component_as_path(const FimoUTF8PathComponent *component) {
    FIMO_DEBUG_ASSERT(component)
    switch (component->type) {
        case FIMO_UTF8_PATH_COMPONENT_PREFIX:
            return component->data.prefix.raw;
        case FIMO_UTF8_PATH_COMPONENT_ROOT_DIR:
            return (FimoUTF8Path){.path = PATH_SEPARATOR_STR, .length = sizeof(PATH_SEPARATOR_STR) - 1};
        case FIMO_UTF8_PATH_COMPONENT_CUR_DIR:
            return (FimoUTF8Path){.path = ".", .length = 1};
        case FIMO_UTF8_PATH_COMPONENT_PARENT_DIR:
            return (FimoUTF8Path){.path = "..", .length = 2};
        case FIMO_UTF8_PATH_COMPONENT_NORMAL:
            return component->data.normal;
    }

    FIMO_ASSERT(false);
}
