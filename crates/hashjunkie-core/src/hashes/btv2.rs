use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::hashes::Hasher;

const BLOCK_SIZE: usize = 16 * 1024;
const PARALLEL_BLOCK_BATCH_SIZE: usize = 1024;
const ZERO_HASH: [u8; 32] = [0; 32];

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

pub struct Btv2Hasher {
    leaf_hashes: Vec<[u8; 32]>,
    pending_blocks: Vec<Vec<u8>>,
    current_block: Vec<u8>,
}

impl Btv2Hasher {
    pub fn new() -> Self {
        Self {
            leaf_hashes: Vec::new(),
            pending_blocks: Vec::new(),
            current_block: Vec::with_capacity(BLOCK_SIZE),
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
        self.leaf_hashes.extend(
            pending
                .into_par_iter()
                .map(|block| sha256_block(&block))
                .collect::<Vec<_>>(),
        );
    }
}

impl Default for Btv2Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Btv2Hasher {
    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            if self.current_block.is_empty() && data.len() >= BLOCK_SIZE {
                let (block, rest) = data.split_at(BLOCK_SIZE);
                self.push_owned_block(block.to_vec());
                data = rest;
                continue;
            }

            let remaining = BLOCK_SIZE - self.current_block.len();
            let take = data.len().min(remaining);
            self.current_block.extend_from_slice(&data[..take]);
            data = &data[take..];

            if self.current_block.len() == BLOCK_SIZE {
                let block =
                    std::mem::replace(&mut self.current_block, Vec::with_capacity(BLOCK_SIZE));
                self.push_owned_block(block);
            }
        }
    }

    fn finalize_hex(mut self: Box<Self>) -> String {
        if !self.current_block.is_empty() {
            let block = std::mem::take(&mut self.current_block);
            self.pending_blocks.push(block);
        }
        self.flush_pending_blocks();

        hex::encode(merkle_root(std::mem::take(&mut self.leaf_hashes)))
    }
}

fn merkle_root(mut level: Vec<[u8; 32]>) -> [u8; 32] {
    if level.is_empty() {
        return ZERO_HASH;
    }

    let leaf_count = level.len();
    let padded_leaf_count = leaf_count.next_power_of_two();
    level.resize(padded_leaf_count, ZERO_HASH);

    while level.len() > 1 {
        level = level
            .par_chunks_exact(2)
            .map(|pair| sha256_pair(&pair[0], &pair[1]))
            .collect();
    }

    level[0]
}

fn sha256_block(block: &[u8]) -> [u8; 32] {
    Sha256::digest(block).into()
}

fn sha256_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = Btv2Hasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn empty_returns_zero_root_for_standalone_hashing() {
        assert_eq!(
            hash(b""),
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn single_block_is_sha256_of_data() {
        assert_eq!(
            hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn second_leaf_changes_root_even_when_short() {
        let mut data = vec![0x11; BLOCK_SIZE];
        data.push(0x22);

        assert_eq!(
            hash(&data),
            "00fc3eb1148fae163d7387a6327f5c177693b8e548446cd3289b7614e2c136ac"
        );
    }

    #[test]
    fn non_power_of_two_leaf_count_is_zero_padded() {
        let mut data = Vec::new();
        data.extend_from_slice(&vec![0x11; BLOCK_SIZE]);
        data.extend_from_slice(&vec![0x22; BLOCK_SIZE]);
        data.push(0x33);

        assert_eq!(
            hash(&data),
            "ed4b7706bc4eec7d8f33f4e8e623c6a57097c35764efa6ebf685b3eb6c8c9133"
        );
    }

    #[test]
    fn chunked_update_matches_single_update() {
        let data = (0..(BLOCK_SIZE * 3 + 17))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        let single = hash(&data);

        let mut h = Btv2Hasher::new();
        for chunk in data.chunks(777) {
            h.update(chunk);
        }

        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn parallel_batches_match_single_update() {
        let data = (0..(BLOCK_SIZE * (PARALLEL_BLOCK_BATCH_SIZE + 3) + 17))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        let single = hash(&data);

        reset_profile();
        let mut h = Btv2Hasher::new();
        for chunk in data.chunks(BLOCK_SIZE / 3 + 5) {
            h.update(chunk);
        }
        let chunked = Box::new(h).finalize_hex();

        assert_eq!(chunked, single);
        assert!(parallel_batches() > 0);
    }
}
