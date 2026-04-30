use digest::Digest;

use crate::{DigestValue, base32_upper_no_padding, hashes::Hasher};

const LEAF_SIZE: usize = 1024;
const TIGER_DIGEST_SIZE: usize = 24;

pub struct TigerTreeHasher {
    leaves: Vec<[u8; TIGER_DIGEST_SIZE]>,
    current_leaf: Vec<u8>,
}

impl TigerTreeHasher {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            current_leaf: Vec::with_capacity(LEAF_SIZE),
        }
    }

    fn push_current_leaf(&mut self) {
        let leaf = std::mem::replace(&mut self.current_leaf, Vec::with_capacity(LEAF_SIZE));
        self.leaves.push(tiger_leaf(&leaf));
    }
}

impl Default for TigerTreeHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for TigerTreeHasher {
    fn update(&mut self, mut data: &[u8]) {
        if self.current_leaf.len() == LEAF_SIZE && !data.is_empty() {
            self.push_current_leaf();
        }

        while !data.is_empty() {
            let remaining = LEAF_SIZE - self.current_leaf.len();
            let take = data.len().min(remaining);
            self.current_leaf.extend_from_slice(&data[..take]);
            data = &data[take..];

            if self.current_leaf.len() == LEAF_SIZE && !data.is_empty() {
                self.push_current_leaf();
            }
        }
    }

    fn finalize_hex(self: Box<Self>) -> String {
        self.finalize_digest().standard().to_string()
    }

    fn finalize_digest(mut self: Box<Self>) -> DigestValue {
        if self.leaves.is_empty() || !self.current_leaf.is_empty() {
            self.push_current_leaf();
        }

        let mut level = self.leaves;
        while level.len() > 1 {
            level = level
                .chunks(2)
                .map(|chunk| {
                    if chunk.len() == 1 {
                        chunk[0]
                    } else {
                        tiger_node(&chunk[0], &chunk[1])
                    }
                })
                .collect();
        }

        DigestValue::from_raw_standard(level[0], base32_upper_no_padding(&level[0]))
    }
}

fn tiger_leaf(data: &[u8]) -> [u8; TIGER_DIGEST_SIZE] {
    let mut tiger = tiger::Tiger::new();
    tiger.update([0x00]);
    tiger.update(data);
    tiger.finalize().into()
}

fn tiger_node(left: &[u8; TIGER_DIGEST_SIZE], right: &[u8; TIGER_DIGEST_SIZE]) -> [u8; 24] {
    let mut tiger = tiger::Tiger::new();
    tiger.update([0x01]);
    tiger.update(left);
    tiger.update(right);
    tiger.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = TigerTreeHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn empty_matches_known_gnutella_tiger_tree_hash() {
        assert_eq!(hash(b""), "LWPNACQDBZRYXW3VHJVCJ64QBZNGHOHHHZWCLNQ");
    }

    #[test]
    fn default_equals_new() {
        let mut default_hasher = TigerTreeHasher::default();
        default_hasher.update(b"abc");

        let mut new_hasher = TigerTreeHasher::new();
        new_hasher.update(b"abc");

        assert_eq!(
            Box::new(default_hasher).finalize_hex(),
            Box::new(new_hasher).finalize_hex()
        );
    }

    #[test]
    fn single_leaf_hash_is_base32_tiger_of_prefixed_data() {
        assert_eq!(hash(b"abc"), base32_upper_no_padding(&tiger_leaf(b"abc")));
    }

    #[test]
    fn chunked_update_matches_single_update() {
        let data = (0..(LEAF_SIZE * 3 + 17))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        let single = hash(&data);

        let mut h = TigerTreeHasher::new();
        for chunk in data.chunks(333) {
            h.update(chunk);
        }

        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn one_byte_over_leaf_boundary_uses_internal_node() {
        let mut data = vec![0xA5; LEAF_SIZE];
        data.push(0x5A);

        let left = tiger_leaf(&data[..LEAF_SIZE]);
        let right = tiger_leaf(&data[LEAF_SIZE..]);

        assert_eq!(
            hash(&data),
            base32_upper_no_padding(&tiger_node(&left, &right))
        );
    }

    #[test]
    fn new_update_after_exact_leaf_flushes_deferred_leaf() {
        let mut h = TigerTreeHasher::new();
        h.update(&vec![0xA5; LEAF_SIZE]);
        h.update(&[0x5A]);

        let mut data = vec![0xA5; LEAF_SIZE];
        data.push(0x5A);

        assert_eq!(Box::new(h).finalize_hex(), hash(&data));
    }
}
