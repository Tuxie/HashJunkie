/// HiDrive hash — combines SHA-1 block hashes hierarchically.
///
/// Algorithm spec:
///   <https://static.hidrive.com/dev/0001>
///
/// Key properties
/// - Block size: 4 096 bytes
/// - A block consisting entirely of null bytes contributes a zero-sum (20 null
///   bytes) instead of its SHA-1.
/// - An empty file returns the zero-sum directly.
/// - Up to 256 block sums are aggregated per level; full levels roll up into
///   the next level using a positional-embedding scheme.
use sha1::{Digest, Sha1};

use crate::hashes::Hasher;

/// Number of data bytes per leaf block.
pub const BLOCK_SIZE: usize = 4096;
/// Number of block-level checksums per aggregation level.
const SUMS_PER_LEVEL: usize = 256;
/// Size of a SHA-1 digest in bytes.
const SHA1_SIZE: usize = 20;

type Sum = [u8; SHA1_SIZE];

const ZERO_SUM: Sum = [0u8; SHA1_SIZE];

// ---------------------------------------------------------------------------
// Level aggregation
// ---------------------------------------------------------------------------

/// One aggregation level: collects up to [`SUMS_PER_LEVEL`] checksums.
///
/// Each incoming checksum is position-embedded (the current index byte is
/// appended to the running SHA-1 state) before being XOR-accumulated into
/// the level's running total.
#[derive(Clone)]
struct Level {
    /// Accumulated (arithmetic-add-with-overflow) checksum of this level.
    checksum: Sum,
    /// Number of child sums written so far.
    sum_count: usize,
    /// SHA-1 hasher for the current child.
    hasher: Sha1,
    /// Bytes written into `hasher` so far.
    bytes_in_hasher: usize,
    /// Whether `hasher` has seen only null bytes.
    only_null_bytes: bool,
}

impl Level {
    fn new() -> Self {
        Self {
            checksum: ZERO_SUM,
            sum_count: 0,
            hasher: Sha1::new(),
            bytes_in_hasher: 0,
            only_null_bytes: true,
        }
    }

    fn is_full(&self) -> bool {
        self.sum_count >= SUMS_PER_LEVEL
    }

    /// Arithmetic-add `sha1sum` into `self.checksum` (byte-by-byte, with carry).
    fn accumulate(&mut self, sha1sum: &Sum) {
        let mut carry = false;
        for i in (0..SHA1_SIZE).rev() {
            let tmp = u16::from(sha1sum[i]) + u16::from(self.checksum[i]) + u16::from(carry);
            carry = tmp > 255;
            self.checksum[i] = tmp as u8;
        }
    }

    /// Write `sha1sum` as a child checksum at the current position.
    ///
    /// The child's positional index byte is appended to the SHA-1 running
    /// state before the digest is taken, matching the reference implementation.
    fn write_child(&mut self, sha1sum: &Sum) {
        debug_assert!(!self.is_full());
        // Feed the child sum into the per-child hasher.
        self.hasher.update(sha1sum);
        self.bytes_in_hasher += SHA1_SIZE;
        // Check if the child sum is all zeros (null).
        let only_null = *sha1sum == ZERO_SUM;
        self.only_null_bytes = self.only_null_bytes && only_null;

        if !self.only_null_bytes {
            // Append position byte (matches hidrivehash.go's `l.hasher.Write([]byte{byte(l.sumCount)})`).
            let pos_byte = [self.sum_count as u8];
            self.hasher.update(pos_byte);
            let digest: Sum = self.hasher.finalize_reset().into();
            self.accumulate(&digest);
        }

        self.sum_count += 1;
        self.only_null_bytes = true; // reset for next child
    }

    /// Return the accumulated level checksum.
    fn sum(&self) -> Sum {
        self.checksum
    }

    fn reset(&mut self) {
        self.checksum = ZERO_SUM;
        self.sum_count = 0;
        self.hasher = Sha1::new();
        self.bytes_in_hasher = 0;
        self.only_null_bytes = true;
    }
}

// ---------------------------------------------------------------------------
// Public hasher
// ---------------------------------------------------------------------------

pub struct HidriveHasher {
    /// Aggregation levels (index 0 = level-1 in the spec).
    levels: Vec<Level>,
    /// The last checksum written to any level (used for single-child final level).
    last_sum_written: Sum,
    /// SHA-1 hasher for the current 4 096-byte block.
    block_hash: Sha1,
    /// Bytes written into `block_hash` so far.
    bytes_in_block: usize,
    /// Whether `block_hash` has seen only null bytes.
    only_null_in_block: bool,
}

impl HidriveHasher {
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
            last_sum_written: ZERO_SUM,
            block_hash: Sha1::new(),
            bytes_in_block: 0,
            only_null_in_block: true,
        }
    }

    /// Push a completed block checksum into the level hierarchy.
    fn push_block_sum(&mut self, sum: Sum) {
        self.last_sum_written = sum;
        let mut current_sum = sum;
        let mut level_idx = 0;
        loop {
            if level_idx >= self.levels.len() {
                self.levels.push(Level::new());
            }
            self.levels[level_idx].write_child(&current_sum);
            if !self.levels[level_idx].is_full() {
                break;
            }
            // Roll this level up.
            current_sum = self.levels[level_idx].sum();
            self.levels[level_idx].reset();
            level_idx += 1;
        }
    }
}

impl Default for HidriveHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for HidriveHasher {
    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let remaining = BLOCK_SIZE - self.bytes_in_block;
            let take = data.len().min(remaining);

            // Track whether this slice is all nulls.
            let slice_is_null = data[..take].iter().all(|&b| b == 0);
            self.only_null_in_block = self.only_null_in_block && slice_is_null;

            self.block_hash.update(&data[..take]);
            self.bytes_in_block += take;
            data = &data[take..];

            if self.bytes_in_block == BLOCK_SIZE {
                let sum: Sum = if self.only_null_in_block {
                    ZERO_SUM
                } else {
                    self.block_hash.finalize_reset().into()
                };
                self.block_hash = Sha1::new();
                self.bytes_in_block = 0;
                self.only_null_in_block = true;
                self.push_block_sum(sum);
            }
        }
    }

    fn finalize_hex(mut self: Box<Self>) -> String {
        // Empty file → zero-sum.
        if self.bytes_in_block == 0 && self.levels.is_empty() {
            return hex::encode(ZERO_SUM);
        }

        // Flush the partial block (if any).
        if self.bytes_in_block > 0 {
            let sum: Sum = if self.only_null_in_block {
                ZERO_SUM
            } else {
                self.block_hash.finalize_reset().into()
            };
            self.push_block_sum(sum);
        }

        // Aggregate non-final levels upward.
        let num_levels = self.levels.len();
        for i in 0..num_levels.saturating_sub(1) {
            if self.levels[i].sum_count >= 1 {
                let s = self.levels[i].sum();
                // Push the level's own accumulated sum into the next level.
                let next = i + 1;
                if next >= self.levels.len() {
                    self.levels.push(Level::new());
                }
                // We need to record `s` as the last sum written and push it.
                self.last_sum_written = s;
                self.levels[next].write_child(&s);
                self.levels[i].reset();
            }
        }

        let final_level = self.levels.last().expect("at least one level after flush");
        let checksum = if final_level.sum_count > 1 {
            final_level.sum()
        } else {
            self.last_sum_written
        };

        hex::encode(checksum)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = HidriveHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // Empty file → zero-sum (20 null bytes), per the HiDrive spec.
    #[test]
    fn hidrive_empty() {
        assert_eq!(hash(b""), "0000000000000000000000000000000000000000");
    }

    // Small non-null file (< 4 096 bytes, one block) → SHA1 of the data.
    // With a single block there is one sum in the final level (sumCount == 1),
    // so the result is lastSumWritten = SHA1(data) directly.
    #[test]
    fn hidrive_small_non_null() {
        let data = b"hello world";
        let expected = hex::encode(Sha1::digest(data));
        assert_eq!(hash(data), expected);
    }

    // A block of all zeros → zero-sum (null-block shortcut).
    #[test]
    fn hidrive_all_zeros_one_block() {
        let data = vec![0u8; 512];
        assert_eq!(hash(&data), "0000000000000000000000000000000000000000");
    }

    // Chunked updates must produce the same result as a single update.
    #[test]
    fn chunked_matches_single() {
        let data = vec![0x42u8; 4096];
        let single = hash(&data);
        let mut h = HidriveHasher::new();
        for chunk in data.chunks(100) {
            h.update(chunk);
        }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    // Two blocks must hash differently from a single block of the same bytes,
    // because the aggregation is order- and count-sensitive.
    #[test]
    fn block_boundary_differs_from_single_block() {
        // First BLOCK_SIZE bytes are non-null, one extra byte → two blocks.
        let mut two_block_data = vec![0x01u8; BLOCK_SIZE];
        two_block_data.push(0x01u8);

        let two_block_hash = hash(&two_block_data);

        // A single-block hash of the same data = SHA1(two_block_data).
        let single_block_hash = hex::encode(Sha1::digest(&two_block_data));

        assert_ne!(two_block_hash, single_block_hash);
    }

    // The zero-sum null-block shortcut must not apply to a block that has
    // even one non-null byte.
    #[test]
    fn mostly_null_block_is_not_zero_sum() {
        let mut data = vec![0u8; BLOCK_SIZE - 1];
        data.push(1u8);
        let result = hash(&data);
        assert_ne!(result, "0000000000000000000000000000000000000000");
    }

    // Exact block size (4 096 bytes) of non-null data → SHA1 of block.
    #[test]
    fn exactly_one_block_non_null() {
        let data = vec![0xFFu8; BLOCK_SIZE];
        let expected = hex::encode(Sha1::digest(&data));
        assert_eq!(hash(&data), expected);
    }
}
