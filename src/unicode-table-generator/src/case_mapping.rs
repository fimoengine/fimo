use crate::{fmt_list, UnicodeData};
use std::{
    char,
    collections::BTreeMap,
    fmt::{self, Write},
};

const INDEX_MASK: u32 = 1 << 22;

pub(crate) fn generate_case_mapping(data: &UnicodeData) -> (String, String) {
    let mut src_file = String::new();
    let mut header_file = String::new();

    header_file.push('\n');
    header_file.push_str(HEADER.trim_start());

    src_file.push_str(SRC_HEADER.trim_start());
    src_file.push('\n');

    write!(
        src_file,
        "static const FimoU32 CONVERSIONS_INDEX_MASK = 0x{:x};",
        INDEX_MASK
    )
    .unwrap();
    src_file.push_str("\n\n");
    src_file.push_str(&generate_tables("LOWER", &data.to_lower));
    src_file.push_str("\n\n");
    src_file.push_str(&generate_tables("UPPER", &data.to_upper));
    src_file.push('\n');
    src_file.push_str(SRC_FOOTER.trim_start());
    (src_file, header_file)
}

fn generate_tables(case: &str, data: &BTreeMap<u32, (u32, u32, u32)>) -> String {
    let mut mappings = Vec::with_capacity(data.len());
    let mut multis = Vec::new();

    for (&key, &(a, b, c)) in data.iter() {
        let key = char::from_u32(key).unwrap();

        if key.is_ascii() {
            continue;
        }

        let value = if b == 0 && c == 0 {
            a
        } else {
            multis.push(BraceFormat([
                CharHex(char::from_u32(a).unwrap()),
                CharHex(char::from_u32(b).unwrap()),
                CharHex(char::from_u32(c).unwrap()),
            ]));

            INDEX_MASK | (u32::try_from(multis.len()).unwrap() - 1)
        };

        mappings.push(BraceFormat((CharHex(key), value)));
    }

    let mut tables = String::new();

    write!(
        tables,
        "static const struct CharValuePair_ CONVERSIONS_{}CASE_TABLE[] = {{{}}};",
        case,
        fmt_list(mappings)
    )
    .unwrap();

    tables.push_str("\n\n");

    write!(
        tables,
        "static const FimoChar CONVERSIONS_{}CASE_TABLE_MULTI[][3] = {{{}}};",
        case,
        fmt_list(multis)
    )
    .unwrap();

    tables
}

struct CharEscape(char);

impl fmt::Debug for CharEscape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}'", self.0.escape_default())
    }
}

struct BraceFormat<T>(T);

impl<T: fmt::Debug> fmt::Debug for BraceFormat<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        write!(&mut s, "{:?}", self.0).unwrap();

        s.pop();
        s.remove(0);
        write!(f, "{{{}}}", s)
    }
}

struct CharHex(char);

impl fmt::Debug for CharHex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(FimoChar)0x{:x}", self.0 as u32)
    }
}

static HEADER: &str = r"
struct FimoUnicodeCharTriple {
    FimoChar ch[3];
};

struct FimoUnicodeCharTriple fimo_internal_unicode_to_lower(FimoChar ch);
struct FimoUnicodeCharTriple fimo_internal_unicode_to_upper(FimoChar ch);
";

static SRC_HEADER: &str = r"
struct CharValuePair_ {
    FimoChar key;
    FimoU32 val;
};
";

static SRC_FOOTER: &str = r"
static inline bool binary_search_conversion_table_(FimoChar el, 
    const struct CharValuePair_ conversion_table[], size_t len, size_t* idx) 
{
    size_t left = 0;
    size_t right = len;
    size_t size = len;
    while (left < right) {
        size_t mid = left + size / (size_t)2;
        FimoU32 x = conversion_table[mid].key;

        if (x < el) {
            left = mid + 1;
        } else if (x > el) {
            right = mid;
        } else {
            *idx = mid;
            return true;
        }

        size = right - left;
    }

    *idx = left;
    return false;
}

struct FimoUnicodeCharTriple fimo_internal_unicode_to_lower(FimoChar ch)
{
    if (fimo_char_is_ascii(ch)) {
        return (struct FimoUnicodeCharTriple) {
            .ch = { fimo_char_to_ascii_lowercase(ch), '\0', '\0' },
        };
    } else {
        size_t idx;
        bool found = binary_search_conversion_table_(
            ch,
            CONVERSIONS_LOWERCASE_TABLE,
            sizeof(CONVERSIONS_LOWERCASE_TABLE)/sizeof(CONVERSIONS_LOWERCASE_TABLE[0]),
            &idx
        );
        if (!found) {
            return (struct FimoUnicodeCharTriple) {
                .ch = { ch, '\0', '\0' },
            };
        }

        FimoChar c;
        FimoU32 u = CONVERSIONS_LOWERCASE_TABLE[idx].val;
        FimoError error = fimo_char_from_u32(u, &c);
        if (FIMO_IS_ERROR(error)) {
            size_t i = (size_t)(u & (CONVERSIONS_INDEX_MASK - 1));
            const FimoChar* table_entry = &CONVERSIONS_LOWERCASE_TABLE_MULTI[i][0];
            return (struct FimoUnicodeCharTriple) {
                .ch = { table_entry[0], table_entry[1], table_entry[2] },
            };
        } else {
            return (struct FimoUnicodeCharTriple) {
                .ch = { c, '\0', '\0' },
            };
        }
    }
}

struct FimoUnicodeCharTriple fimo_internal_unicode_to_upper(FimoChar ch)
{
    if (fimo_char_is_ascii(ch)) {
        return (struct FimoUnicodeCharTriple) {
            .ch = { fimo_char_to_ascii_lowercase(ch), '\0', '\0' },
        };
    } else {
        size_t idx;
        bool found = binary_search_conversion_table_(
            ch,
            CONVERSIONS_UPPERCASE_TABLE,
            sizeof(CONVERSIONS_UPPERCASE_TABLE)/sizeof(CONVERSIONS_UPPERCASE_TABLE[0]),
            &idx
        );
        if (!found) {
            return (struct FimoUnicodeCharTriple) {
                .ch = { ch, '\0', '\0' },
            };
        }

        FimoChar c;
        FimoU32 u = CONVERSIONS_UPPERCASE_TABLE[idx].val;
        FimoError error = fimo_char_from_u32(u, &c);
        if (FIMO_IS_ERROR(error)) {
            size_t i = (size_t)(u & (CONVERSIONS_INDEX_MASK - 1));
            const FimoChar* table_entry = &CONVERSIONS_UPPERCASE_TABLE_MULTI[i][0];
            return (struct FimoUnicodeCharTriple) {
                .ch = { table_entry[0], table_entry[1], table_entry[2] },
            };
        } else {
            return (struct FimoUnicodeCharTriple) {
                .ch = { c, '\0', '\0' },
            };
        }
    }
}
";
