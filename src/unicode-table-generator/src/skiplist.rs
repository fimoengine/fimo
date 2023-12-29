use crate::{fmt_list, raw_emitter::RawEmitter};
use std::{fmt::Write as _, ops::Range};

/// This will get packed into a single u32 before inserting into the data set.
#[derive(Debug, PartialEq)]
struct ShortOffsetRunHeader {
    /// Note, we only allow for 21 bits here.
    prefix_sum: u32,

    /// Note, we actually only allow for 11 bits here. This should be enough --
    /// our largest sets are around ~1400 offsets long.
    start_idx: u16,
}

impl ShortOffsetRunHeader {
    fn pack(&self) -> u32 {
        assert!(self.start_idx < (1 << 11));
        assert!(self.prefix_sum < (1 << 21));

        (self.start_idx as u32) << 21 | self.prefix_sum
    }
}

impl RawEmitter {
    pub fn emit_skiplist(&mut self, ranges: &[Range<u32>], property: &str) {
        let mut offsets = Vec::<u32>::new();
        let points = ranges
            .iter()
            .flat_map(|r| vec![r.start, r.end])
            .collect::<Vec<u32>>();
        let mut offset = 0;
        for pt in points {
            let delta = pt - offset;
            offsets.push(delta);
            offset = pt;
        }
        // Guaranteed to terminate, as it's impossible to subtract a value this
        // large from a valid char.
        offsets.push(std::char::MAX as u32 + 1);
        let mut coded_offsets: Vec<u8> = Vec::new();
        let mut short_offset_runs: Vec<ShortOffsetRunHeader> = vec![];
        let mut iter = offsets.iter().cloned();
        let mut prefix_sum = 0;
        loop {
            let mut any_elements = false;
            let mut inserted = false;
            let start = coded_offsets.len();
            for offset in iter.by_ref() {
                any_elements = true;
                prefix_sum += offset;
                if let Ok(offset) = offset.try_into() {
                    coded_offsets.push(offset);
                } else {
                    short_offset_runs.push(ShortOffsetRunHeader {
                        start_idx: start.try_into().unwrap(),
                        prefix_sum,
                    });
                    // This is just needed to maintain indices even/odd
                    // correctly.
                    coded_offsets.push(0);
                    inserted = true;
                    break;
                }
            }
            if !any_elements {
                break;
            }
            // We always append the huge char::MAX offset to the end which
            // should never be able to fit into the u8 offsets.
            assert!(inserted);
        }

        let property_upper = property.to_uppercase();
        let property_lower = property.to_lowercase();
        writeln!(
            &mut self.src_file,
            "static const FimoU32 {}_SHORT_OFFSET_RUNS[{}] = {{{}}};",
            property_upper,
            short_offset_runs.len(),
            fmt_list(short_offset_runs.iter().map(|v| v.pack()))
        )
        .unwrap();
        self.bytes_used += 4 * short_offset_runs.len();
        writeln!(
            &mut self.src_file,
            "static const FimoU8 {}_OFFSETS[{}] = {{{}}};",
            property_upper,
            coded_offsets.len(),
            fmt_list(&coded_offsets)
        )
        .unwrap();
        self.bytes_used += coded_offsets.len();

        #[rustfmt::skip]
        writeln!(&mut self.header_file, "bool fimo_internal_unicode_{}_lookup(FimoChar ch);", property_lower).unwrap();

        #[rustfmt::skip]
        let mut x = || {
            writeln!(&mut self.src_file, "bool fimo_internal_unicode_{}_lookup(FimoChar ch)", property_lower).unwrap();
            writeln!(&mut self.src_file, "{{").unwrap();
            writeln!(&mut self.src_file, "    return skip_search_(",).unwrap();
            writeln!(&mut self.src_file, "        (FimoU32)ch,").unwrap();
            writeln!(&mut self.src_file, "        {}_SHORT_OFFSET_RUNS,", property_upper).unwrap();
            writeln!(&mut self.src_file, "        sizeof({0}_SHORT_OFFSET_RUNS)/sizeof({0}_SHORT_OFFSET_RUNS[0]),", property_upper).unwrap();
            writeln!(&mut self.src_file, "        {}_OFFSETS,", property_upper).unwrap();
            writeln!(&mut self.src_file, "        sizeof({0}_OFFSETS)/sizeof({0}_OFFSETS[0])", property_upper).unwrap();
            writeln!(&mut self.src_file, "    );").unwrap();
            writeln!(&mut self.src_file, "}}").unwrap();
        };
        x();
    }
}
