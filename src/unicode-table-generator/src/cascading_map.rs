use crate::{fmt_list, raw_emitter::RawEmitter};
use std::{collections::HashMap, fmt::Write as _, ops::Range};

impl RawEmitter {
    pub fn emit_cascading_map(&mut self, ranges: &[Range<u32>]) -> bool {
        let mut map: [u8; 256] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let points = ranges
            .iter()
            .flat_map(|r| (r.start..r.end).collect::<Vec<u32>>())
            .collect::<Vec<u32>>();

        println!("there are {} points", points.len());

        // how many distinct ranges need to be counted?
        let mut codepoints_by_high_bytes = HashMap::<usize, Vec<u32>>::new();
        for point in points {
            // assert that there is no whitespace over the 0x3000 range.
            assert!(
                point <= 0x3000,
                "the highest unicode whitespace value has changed"
            );
            let high_bytes = point as usize >> 8;
            let codepoints = codepoints_by_high_bytes.entry(high_bytes).or_default();
            codepoints.push(point);
        }

        let mut bit_for_high_byte = 1u8;
        let mut arms = Vec::<String>::new();

        let mut high_bytes: Vec<usize> = codepoints_by_high_bytes.keys().copied().collect();
        high_bytes.sort();
        for high_byte in high_bytes {
            let codepoints = codepoints_by_high_bytes.get_mut(&high_byte).unwrap();
            if codepoints.len() == 1 {
                let ch = codepoints.pop().unwrap();
                arms.push(format!("case {}:", high_byte));
                arms.push(format!("    return ((FimoU32)ch) == {:#04x};", ch));
                continue;
            }
            // more than 1 codepoint in this arm
            for codepoint in codepoints {
                map[(*codepoint & 0xff) as usize] |= bit_for_high_byte;
            }
            arms.push(format!("case {}:", high_byte));
            #[rustfmt::skip]
            arms.push(format!("    return (WHITESPACE_MAP[((size_t)ch) & ((size_t)0xff)] & {}) != 0;", bit_for_high_byte));
            bit_for_high_byte <<= 1;
        }

        writeln!(
            &mut self.src_file,
            "static const FimoU8 WHITESPACE_MAP[256] = {{{}}};",
            fmt_list(map.iter())
        )
        .unwrap();
        self.bytes_used += 256;

        #[rustfmt::skip]
        writeln!(&mut self.header_file, "bool fimo_impl_unicode_whitespace_lookup(FimoChar ch);").unwrap();

        #[rustfmt::skip]
        let x = || {
            writeln!(&mut self.src_file, "bool fimo_impl_unicode_whitespace_lookup(FimoChar ch) {{").unwrap();
            writeln!(&mut self.src_file, "    switch (((FimoU32)ch) >> 8) {{").unwrap();
            for arm in arms {
                writeln!(&mut self.src_file, "    {}", arm).unwrap();
            }
            writeln!(&mut self.src_file, "    default:").unwrap();
            writeln!(&mut self.src_file, "        return false;").unwrap();
            writeln!(&mut self.src_file, "    }}").unwrap();
            writeln!(&mut self.src_file, "}}").unwrap();
        };
        x();

        true
    }
}
