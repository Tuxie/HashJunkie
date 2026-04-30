use rayon::prelude::*;
use sha1::{Digest, Sha1};

use crate::hashes::Hasher;

const PART_SIZE: u64 = 9_728_000;
const BLOCK_SIZE: u64 = 180 * 1024;
const BLOCK_SIZE_USIZE: usize = BLOCK_SIZE as usize;
const PARALLEL_BLOCK_BATCH_SIZE: usize = 64;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
static PARALLEL_BATCHES: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn reset_profile() {
    PARALLEL_BATCHES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
fn parallel_batches() -> usize {
    PARALLEL_BATCHES.load(Ordering::Relaxed)
}

pub struct AichHasher {
    block_hashes: Vec<[u8; 20]>,
    pending_blocks: Vec<Vec<u8>>,
    current_block: Vec<u8>,
    total_size: u64,
}

impl AichHasher {
    pub fn new() -> Self {
        Self {
            block_hashes: Vec::new(),
            pending_blocks: Vec::new(),
            current_block: Vec::with_capacity(BLOCK_SIZE_USIZE),
            total_size: 0,
        }
    }

    fn push_owned_block(&mut self, block: Vec<u8>) {
        self.pending_blocks.push(block);
        if self.pending_blocks.len() >= PARALLEL_BLOCK_BATCH_SIZE {
            self.flush_pending_blocks();
        }
    }

    fn flush_pending_blocks(&mut self) {
        let pending = std::mem::take(&mut self.pending_blocks);
        #[cfg(test)]
        if pending.len() > 1 {
            PARALLEL_BATCHES.fetch_add(1, Ordering::Relaxed);
        }
        self.block_hashes.extend(
            pending
                .into_par_iter()
                .map(|block| sha1_block(&block))
                .collect::<Vec<_>>(),
        );
    }
}

impl Default for AichHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for AichHasher {
    fn update(&mut self, mut data: &[u8]) {
        self.total_size += data.len() as u64;

        while !data.is_empty() {
            if self.current_block.is_empty() && data.len() >= BLOCK_SIZE_USIZE {
                let (block, rest) = data.split_at(BLOCK_SIZE_USIZE);
                self.push_owned_block(block.to_vec());
                data = rest;
                continue;
            }

            let remaining = BLOCK_SIZE_USIZE - self.current_block.len();
            let take = data.len().min(remaining);
            self.current_block.extend_from_slice(&data[..take]);
            data = &data[take..];

            if self.current_block.len() == BLOCK_SIZE_USIZE {
                let block = std::mem::replace(
                    &mut self.current_block,
                    Vec::with_capacity(BLOCK_SIZE_USIZE),
                );
                self.push_owned_block(block);
            }
        }
    }

    fn finalize_hex(mut self: Box<Self>) -> String {
        if !self.current_block.is_empty()
            || (self.total_size == 0
                && self.block_hashes.is_empty()
                && self.pending_blocks.is_empty())
        {
            let block = std::mem::take(&mut self.current_block);
            self.pending_blocks.push(block);
        }
        self.flush_pending_blocks();

        base32_no_padding(&aich_root(&self.block_hashes, self.total_size))
    }
}

fn aich_root(block_hashes: &[[u8; 20]], total_size: u64) -> [u8; 20] {
    let base_size = if total_size <= PART_SIZE {
        BLOCK_SIZE
    } else {
        PART_SIZE
    };
    node_hash(block_hashes, 0, total_size, true, base_size)
}

fn node_hash(
    block_hashes: &[[u8; 20]],
    start: u64,
    size: u64,
    is_left_branch: bool,
    base_size: u64,
) -> [u8; 20] {
    if size <= base_size {
        let index = (start / BLOCK_SIZE) as usize;
        return block_hashes[index];
    }

    let (left_size, right_size) = split_node(size, is_left_branch, base_size);
    let left_base_size = child_base_size(left_size);
    let right_base_size = child_base_size(right_size);

    let (left, right) = if size > PART_SIZE * 4 {
        rayon::join(
            || node_hash(block_hashes, start, left_size, true, left_base_size),
            || {
                node_hash(
                    block_hashes,
                    start + left_size,
                    right_size,
                    false,
                    right_base_size,
                )
            },
        )
    } else {
        (
            node_hash(block_hashes, start, left_size, true, left_base_size),
            node_hash(
                block_hashes,
                start + left_size,
                right_size,
                false,
                right_base_size,
            ),
        )
    };
    sha1_pair(&left, &right)
}

fn split_node(size: u64, is_left_branch: bool, base_size: u64) -> (u64, u64) {
    let blocks = size.div_ceil(base_size);
    let left_blocks = if is_left_branch {
        blocks.div_ceil(2)
    } else {
        blocks / 2
    };
    let left_size = left_blocks * base_size;
    (left_size, size - left_size)
}

fn child_base_size(size: u64) -> u64 {
    if size <= PART_SIZE {
        BLOCK_SIZE
    } else {
        PART_SIZE
    }
}

fn sha1_block(block: &[u8]) -> [u8; 20] {
    Sha1::digest(block).into()
}

fn sha1_pair(left: &[u8; 20], right: &[u8; 20]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

fn base32_no_padding(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::with_capacity((bytes.len() * 8).div_ceil(5));
    let mut buffer = 0u16;
    let mut bits = 0u8;

    for byte in bytes {
        buffer = (buffer << 8) | u16::from(*byte);
        bits += 8;

        while bits >= 5 {
            let shift = bits - 5;
            let index = ((buffer >> shift) & 0x1f) as usize;
            out.push(ALPHABET[index] as char);
            bits -= 5;
            buffer &= (1 << bits) - 1;
        }
    }

    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(ALPHABET[index] as char);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = AichHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn empty_is_base32_sha1_empty() {
        assert_eq!(hash(b""), "3I42H3S6NNFQ2MSVX7XZKYAYSCX5QBYJ");
    }

    #[test]
    fn default_equals_new() {
        let mut default_hasher = AichHasher::default();
        default_hasher.update(b"abc");

        let mut new_hasher = AichHasher::new();
        new_hasher.update(b"abc");

        assert_eq!(
            Box::new(default_hasher).finalize_hex(),
            Box::new(new_hasher).finalize_hex()
        );
    }

    #[test]
    fn single_block_is_base32_sha1_of_data() {
        assert_eq!(hash(b"abc"), "VGMT4NSHA2AWVOR6EVYXQUGCNSONBWE5");
    }

    #[test]
    fn one_byte_over_block_boundary_uses_verifying_hash() {
        let mut data = vec![0x11; BLOCK_SIZE_USIZE];
        data.push(0x22);
        assert_eq!(hash(&data), "J573AFG7KZF7FWRT4FS56AVF5EFGSV7B");
    }

    #[test]
    fn exact_part_boundary_uses_block_tree() {
        let data = vec![0x33; PART_SIZE as usize];
        assert_eq!(hash(&data), "C35EVTQPNLQ23UVDH46DMRKNNBGOLVMH");
    }

    #[test]
    fn part_boundary_enters_top_part_tree() {
        let mut data = vec![0x33; PART_SIZE as usize];
        data.push(0x44);
        assert_eq!(hash(&data), "X3Z2D23I35AOHQQBF3RDINSZN5V26HYS");
    }

    #[test]
    fn odd_top_level_split_follows_branch_direction_rule() {
        let mut data = vec![0x11; PART_SIZE as usize];
        data.extend_from_slice(&vec![0x22; PART_SIZE as usize]);
        data.push(0x33);
        assert_eq!(hash(&data), "WWRGDBDJNEJQDTK6ABCCHV74T5LBGY4T");
    }

    #[test]
    fn chunked_update_matches_single_update() {
        let data = (0..(BLOCK_SIZE_USIZE * 3 + 17))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        let single = hash(&data);

        let mut h = AichHasher::new();
        for chunk in data.chunks(3333) {
            h.update(chunk);
        }

        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn parallel_batches_match_single_update() {
        let data = (0..(BLOCK_SIZE_USIZE * (PARALLEL_BLOCK_BATCH_SIZE + 3) + 17))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        let single = hash(&data);

        reset_profile();
        let mut h = AichHasher::new();
        for chunk in data.chunks(BLOCK_SIZE_USIZE / 3 + 5) {
            h.update(chunk);
        }
        let chunked = Box::new(h).finalize_hex();

        assert_eq!(chunked, single);
        assert!(parallel_batches() > 0);
    }

    #[test]
    fn large_tree_uses_parallel_recursive_join() {
        let data = (0..(PART_SIZE as usize * 5 + 1))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        assert_eq!(hash(&data), "3Q7HWWLCHUD4TWSBFMNSL6H3Z4MOVLSG");
    }

    #[test]
    fn base32_encodes_remaining_bits() {
        assert_eq!(base32_no_padding(&[0xff]), "74");
    }
}
