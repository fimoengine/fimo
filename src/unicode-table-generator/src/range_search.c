FimoU64 rotl_(FimoU64 value, int shift)
{
    return (value << shift) | (value >> (sizeof(value) * 8 - shift));
}

static inline bool bitset_search_(FimoU32 needle, const FimoU8 chunk_idx_map[],
    size_t chunk_idx_map_len, const FimoU8 bitset_chunk_idx[],
    size_t bitset_chunk_idx_columns, const FimoU64 bitset_canonical[],
    size_t bitset_canonical_len, const FimoU8 bitset_canonicalized[][2])
{
    size_t bucket_idx = (size_t)(needle / 64);
    size_t chunk_map_idx = bucket_idx / bitset_chunk_idx_columns;
    size_t chunk_piece = bucket_idx % bitset_chunk_idx_columns;

    FimoU8 chunk_idx;
    if (chunk_map_idx < chunk_idx_map_len) {
        chunk_idx = chunk_idx_map[chunk_map_idx];
    } else {
        return false;
    }

    size_t flat_chunk_idx = chunk_piece + (((size_t)chunk_idx) * bitset_chunk_idx_columns);
    size_t idx = (size_t)(bitset_chunk_idx[flat_chunk_idx]);

    FimoU64 word;
    if (idx < bitset_canonical_len) {
        word = bitset_canonical[idx];
    } else {
        FimoU8 real_idx = bitset_canonicalized[idx - bitset_canonical_len][0];
        FimoU8 mapping = bitset_canonicalized[idx - bitset_canonical_len][1];
        word = bitset_canonical[(size_t)real_idx];
        bool should_invert = (mapping & (1 << 6)) != 0;
        if (should_invert) {
            word = ~word;
        }

        FimoU8 quantity = mapping & ((1 << 6) - 1);
        if ((mapping & (1 << 7)) != 0) {
            word >>= (FimoU64)quantity;
        } else {
            word = rotl_(word, (int)quantity);
        }
    }
    return (word & ((FimoU64)1 << (FimoU64)(needle % 64))) != 0;
}

static inline bool binary_seach_short_offset_runs_(FimoU32 el, const FimoU32* arr,
    size_t len, size_t* idx)
{
    size_t left = 0;
    size_t right = len;
    size_t size = len;
    while (left < right) {
        size_t mid = left + size / (size_t)2;
        FimoU32 x = arr[mid] << 11;

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

static inline FimoU32 decode_prefix_sum_(FimoU32 short_offset_run_header)
{
    return short_offset_run_header & (((FimoU32)1 << 21) - (FimoU32)1);
}

static inline size_t decode_length_(FimoU32 short_offset_run_header)
{
    return (size_t)(short_offset_run_header >> 21);
}

static inline bool skip_search_(FimoU32 needle, const FimoU32* short_offset_runs,
    size_t short_offset_runs_len, const FimoU8* offsets, size_t offsets_len)
{
    size_t last_idx = 0;
    if (binary_seach_short_offset_runs_(needle << 11, short_offset_runs,
            short_offset_runs_len, &last_idx)) {
        last_idx++;
    }

    size_t offset_idx = decode_length_(short_offset_runs[last_idx]);
    size_t length;
    if (last_idx + 1 < short_offset_runs_len) {
        length = decode_length_(short_offset_runs[last_idx + 1]) - offset_idx;
    } else {
        length = offsets_len - offset_idx;
    }

    FimoU32 prev = 0;
    if (last_idx > 0) {
        prev = decode_prefix_sum_(short_offset_runs[last_idx - 1]);
    }

    FimoU32 total = needle - prev;
    FimoU32 prefix_sum = 0;
    for (size_t i = 0; i < (length - 1); i++) {
        FimoU8 offset = offsets[offset_idx];
        prefix_sum += (FimoU32)offset;
        if (prefix_sum > total) {
            break;
        }
        offset_idx += 1;
    }
    return (offset_idx % 2) == 1;
}