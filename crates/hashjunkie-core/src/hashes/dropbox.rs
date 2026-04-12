use sha2::{Digest, Sha256};

use crate::hashes::Hasher;

pub const BLOCK_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

pub struct DropboxHasher {
    block_hashes: Vec<[u8; 32]>,
    current_block: Sha256,
    current_block_len: usize,
}

impl DropboxHasher {
    pub fn new() -> Self {
        Self {
            block_hashes: Vec::new(),
            current_block: Sha256::new(),
            current_block_len: 0,
        }
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
            let remaining = BLOCK_SIZE - self.current_block_len;
            let take = data.len().min(remaining);
            self.current_block.update(&data[..take]);
            self.current_block_len += take;
            data = &data[take..];

            if self.current_block_len == BLOCK_SIZE {
                let finished = std::mem::replace(&mut self.current_block, Sha256::new());
                self.block_hashes.push(finished.finalize().into());
                self.current_block_len = 0;
            }
        }
    }

    fn finalize_hex(self: Box<Self>) -> String {
        let Self {
            mut block_hashes,
            current_block,
            current_block_len,
        } = *self;

        // Always finalize the last block — even if empty (handles empty file case)
        // But skip if block is empty AND we already have complete blocks
        if current_block_len > 0 || block_hashes.is_empty() {
            block_hashes.push(current_block.finalize().into());
        }

        let mut outer = Sha256::new();
        for h in &block_hashes {
            outer.update(h);
        }
        hex::encode(outer.finalize())
    }
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
}
