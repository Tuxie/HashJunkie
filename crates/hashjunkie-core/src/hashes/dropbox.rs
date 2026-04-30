use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::hashes::Hasher;

pub const BLOCK_SIZE: usize = 4 * 1024 * 1024; // 4 MiB
const PARALLEL_BLOCK_BATCH_SIZE: usize = 8;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
static PARALLEL_BATCHES: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub fn reset_profile() {
    PARALLEL_BATCHES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
pub fn parallel_batches() -> usize {
    PARALLEL_BATCHES.load(Ordering::Relaxed)
}

pub struct DropboxHasher {
    block_hashes: Vec<[u8; 32]>,
    pending_blocks: Vec<Vec<u8>>,
    current_block: Vec<u8>,
}

impl DropboxHasher {
    pub fn new() -> Self {
        Self {
            block_hashes: Vec::new(),
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
        self.block_hashes.extend(
            pending
                .into_par_iter()
                .map(|block| sha256_block(&block))
                .collect::<Vec<_>>(),
        );
    }
}

impl Default for DropboxHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for DropboxHasher {
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
        if !self.current_block.is_empty()
            || (self.block_hashes.is_empty() && self.pending_blocks.is_empty())
        {
            let block = std::mem::take(&mut self.current_block);
            self.pending_blocks.push(block);
        }
        self.flush_pending_blocks();

        let Self {
            block_hashes,
            pending_blocks: _,
            current_block: _,
        } = *self;

        let mut outer = Sha256::new();
        for h in &block_hashes {
            outer.update(h);
        }
        hex::encode(outer.finalize())
    }
}

fn sha256_block(block: &[u8]) -> [u8; 32] {
    Sha256::digest(block).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = DropboxHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // Empty: outer SHA256 of single empty block's SHA256
    // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    // SHA256(raw bytes of above) = 5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456
    #[test]
    fn dropbox_empty() {
        assert_eq!(
            hash(b""),
            "5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456"
        );
    }

    // Single block (< 4 MiB): SHA256(SHA256(content)) where inner hash is raw bytes
    #[test]
    fn dropbox_abc() {
        use sha2::{Digest, Sha256};
        let inner = Sha256::digest(b"abc");
        let expected = hex::encode(Sha256::digest(inner));
        assert_eq!(hash(b"abc"), expected);
    }

    #[test]
    fn chunked_update_matches_single() {
        let data = vec![0xABu8; 1024];
        let single = hash(&data);
        let mut h = DropboxHasher::new();
        for chunk in data.chunks(100) {
            h.update(chunk);
        }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn default_equals_new() {
        let mut h = DropboxHasher::default();
        h.update(b"abc");
        assert_eq!(Box::new(h).finalize_hex(), hash(b"abc"));
    }

    // When data ends exactly on a 4 MiB boundary, current_block_len is 0 after
    // update() and block_hashes is non-empty, so finalize_hex must skip the
    // extra push (the else branch of `if current_block_len > 0 || block_hashes.is_empty()`).
    #[test]
    fn exact_block_boundary_skips_empty_partial_block() {
        use sha2::{Digest, Sha256};
        // Build two complete 4 MiB blocks — after update, current_block_len == 0
        // and block_hashes has 2 entries.
        let block1 = vec![0x11u8; BLOCK_SIZE];
        let block2 = vec![0x22u8; BLOCK_SIZE];
        let mut data = block1.clone();
        data.extend_from_slice(&block2);

        let result = hash(&data);

        // Manual derivation: outer = SHA256(SHA256(block1) || SHA256(block2))
        let h1: [u8; 32] = Sha256::digest(&block1).into();
        let h2: [u8; 32] = Sha256::digest(&block2).into();
        let mut outer = Sha256::new();
        outer.update(h1);
        outer.update(h2);
        let expected = hex::encode(outer.finalize());

        assert_eq!(result, expected);
    }

    #[test]
    fn block_boundary_produces_two_blocks() {
        // Exactly BLOCK_SIZE bytes in first block, then 1 more byte = two blocks
        let full_block = vec![0u8; BLOCK_SIZE];
        let mut two_block_data = full_block.clone();
        two_block_data.push(1u8);

        let two_block_hash = hash(&two_block_data);

        // Single block of same data would give SHA256(SHA256(combined))
        use sha2::{Digest, Sha256};
        let inner = Sha256::digest(&two_block_data);
        let single_block_hash = hex::encode(Sha256::digest(inner));

        // Two-block and single-block hashes must differ
        assert_ne!(two_block_hash, single_block_hash);
    }

    #[test]
    fn parallel_batches_match_single_update() {
        let data = (0..(BLOCK_SIZE * (PARALLEL_BLOCK_BATCH_SIZE + 3) + 17))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        let single = hash(&data);

        reset_profile();
        let mut h = DropboxHasher::new();
        for chunk in data.chunks(BLOCK_SIZE / 3 + 5) {
            h.update(chunk);
        }
        let chunked = Box::new(h).finalize_hex();

        assert_eq!(chunked, single);
        assert!(parallel_batches() > 0);
    }
}
