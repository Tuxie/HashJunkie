use digest::Digest;
use rayon::prelude::*;

use crate::hashes::Hasher;

pub const BLOCK_SIZE: usize = 9_728_000;
const PARALLEL_BLOCK_BATCH_SIZE: usize = 4;

pub struct Ed2kHasher {
    block_hashes: Vec<[u8; 16]>,
    pending_blocks: Vec<Vec<u8>>,
    current_block: Vec<u8>,
}

impl Ed2kHasher {
    pub fn new() -> Self {
        Self {
            block_hashes: Vec::new(),
            pending_blocks: Vec::new(),
            current_block: Vec::with_capacity(BLOCK_SIZE),
        }
    }

    fn push_current_block(&mut self) {
        let block = std::mem::replace(&mut self.current_block, Vec::with_capacity(BLOCK_SIZE));
        self.pending_blocks.push(block);
        if self.pending_blocks.len() >= PARALLEL_BLOCK_BATCH_SIZE {
            self.flush_pending_blocks();
        }
    }

    fn flush_pending_blocks(&mut self) {
        let pending = std::mem::take(&mut self.pending_blocks);
        self.block_hashes.extend(
            pending
                .into_par_iter()
                .map(|block| md4_block(&block))
                .collect::<Vec<_>>(),
        );
    }
}

impl Default for Ed2kHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Ed2kHasher {
    fn update(&mut self, mut data: &[u8]) {
        if self.current_block.len() == BLOCK_SIZE && !data.is_empty() {
            self.push_current_block();
        }

        while !data.is_empty() {
            let remaining = BLOCK_SIZE - self.current_block.len();
            let take = data.len().min(remaining);
            self.current_block.extend_from_slice(&data[..take]);
            data = &data[take..];

            if self.current_block.len() == BLOCK_SIZE && !data.is_empty() {
                self.push_current_block();
            }
        }
    }

    fn finalize_hex(mut self: Box<Self>) -> String {
        if self.block_hashes.is_empty() && self.pending_blocks.is_empty() {
            return hex::encode(md4_block(&self.current_block));
        }

        if !self.current_block.is_empty() {
            self.push_current_block();
        }
        self.flush_pending_blocks();

        let mut root = md4::Md4::new();
        for block_hash in &self.block_hashes {
            root.update(block_hash);
        }
        hex::encode(root.finalize())
    }
}

fn md4_block(block: &[u8]) -> [u8; 16] {
    md4::Md4::digest(block).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = Ed2kHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn empty_matches_md4_empty() {
        assert_eq!(hash(b""), "31d6cfe0d16ae931b73c59d7e0c089c0");
    }

    #[test]
    fn default_equals_new() {
        let mut default_hasher = Ed2kHasher::default();
        default_hasher.update(b"abc");

        let mut new_hasher = Ed2kHasher::new();
        new_hasher.update(b"abc");

        assert_eq!(
            Box::new(default_hasher).finalize_hex(),
            Box::new(new_hasher).finalize_hex()
        );
    }

    #[test]
    fn small_file_is_plain_md4() {
        assert_eq!(hash(b"abc"), "a448017aaf21d8525fc10ae87aa6729d");
    }

    #[test]
    fn chunked_update_matches_single_update() {
        let data = (0..(BLOCK_SIZE + 1234))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        let single = hash(&data);

        let mut h = Ed2kHasher::new();
        for chunk in data.chunks(123_457) {
            h.update(chunk);
        }

        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn exact_block_boundary_is_single_block_hash() {
        let data = vec![0xA5; BLOCK_SIZE];
        assert_eq!(hash(&data), hex::encode(md4_block(&data)));
    }

    #[test]
    fn one_byte_over_block_boundary_uses_root_hash() {
        let mut data = vec![0xA5; BLOCK_SIZE];
        data.push(0x5A);

        let h1 = md4_block(&data[..BLOCK_SIZE]);
        let h2 = md4_block(&data[BLOCK_SIZE..]);
        let mut root = md4::Md4::new();
        root.update(h1);
        root.update(h2);

        assert_eq!(hash(&data), hex::encode(root.finalize()));
    }

    #[test]
    fn new_update_after_exact_block_flushes_deferred_block() {
        let mut h = Ed2kHasher::new();
        h.update(&vec![0xA5; BLOCK_SIZE]);
        h.update(&[0x5A]);

        let mut data = vec![0xA5; BLOCK_SIZE];
        data.push(0x5A);

        assert_eq!(Box::new(h).finalize_hex(), hash(&data));
    }

    #[test]
    fn pending_blocks_flush_at_parallel_batch_threshold() {
        let data = (0..(BLOCK_SIZE * PARALLEL_BLOCK_BATCH_SIZE + 1))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();

        assert_eq!(hash(&data), "dab73f86a7763a9268d72761b7a4ae2a");
    }
}
