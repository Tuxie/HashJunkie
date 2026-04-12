// Mail.ru hash (mrhash) — a modified SHA1 used by rclone for the Mail.ru cloud backend.
//
// Algorithm (from rclone's backend/mailru/mrhash/mrhash.go):
// 1. The SHA1 internal state is seeded with the prefix "mrCloud".
// 2. If total data written is <= SHA1 digest size (20 bytes), the hash is the
//    raw data right-padded with zero bytes to exactly 20 bytes — SHA1 is NOT used.
// 3. Otherwise the hash is SHA1("mrCloud" + data + decimal_string(total_length)).
// Output: 40 hex characters (20 bytes).

use sha1::{Digest, Sha1};

use crate::hashes::Hasher;

/// Size of the Mail.ru hash digest in bytes (same as SHA1).
pub const SIZE: usize = 20;

/// The prefix written into the SHA1 state at initialisation.
const START_STRING: &[u8] = b"mrCloud";

pub struct MailruHasher {
    /// Underlying SHA1 with "mrCloud" already written in.
    sha: Sha1,
    /// Accumulated total byte count.
    total: usize,
    /// Buffer for small inputs (only kept while total <= SIZE).
    small: Vec<u8>,
}

impl MailruHasher {
    pub fn new() -> Self {
        let mut sha = Sha1::new();
        sha.update(START_STRING);
        Self {
            sha,
            total: 0,
            small: Vec::new(),
        }
    }
}

impl Default for MailruHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for MailruHasher {
    fn update(&mut self, data: &[u8]) {
        self.sha.update(data);
        self.total += data.len();
        if self.total <= SIZE {
            self.small.extend_from_slice(data);
        }
    }

    fn finalize_hex(self: Box<Self>) -> String {
        let Self { sha, total, small } = *self;

        if total <= SIZE {
            // Return the raw data padded with zeros to exactly SIZE bytes.
            let mut padded = [0u8; SIZE];
            padded[..total].copy_from_slice(&small);
            hex::encode(padded)
        } else {
            // SHA1("mrCloud" + data + decimal_length)
            let mut sha = sha;
            sha.update(total.to_string().as_bytes());
            hex::encode(sha.finalize())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = MailruHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // --- Small-data path (total <= 20 bytes): raw data zero-padded to 20 bytes ---

    /// Empty input: 20 zero bytes.
    #[test]
    fn mailru_empty() {
        let result = hash(b"");
        assert_eq!(result.len(), 40, "must be 40 hex chars");
        assert_eq!(result, "0000000000000000000000000000000000000000");
    }

    /// "abc" (3 bytes): data followed by 17 zero bytes.
    #[test]
    fn mailru_abc() {
        assert_eq!(hash(b"abc"), "6162630000000000000000000000000000000000");
    }

    /// "hello world" (11 bytes): data followed by 9 zero bytes.
    #[test]
    fn mailru_hello_world() {
        assert_eq!(
            hash(b"hello world"),
            "68656c6c6f20776f726c64000000000000000000"
        );
    }

    /// Exactly 19 bytes: data followed by 1 zero byte.
    #[test]
    fn mailru_19_bytes() {
        let data = [b'B'; 19];
        assert_eq!(hash(&data), "4242424242424242424242424242424242424200");
    }

    /// Exactly 20 bytes (boundary): raw data, no padding needed.
    #[test]
    fn mailru_exactly_20_bytes() {
        let data = [b'A'; 20];
        assert_eq!(hash(&data), "4141414141414141414141414141414141414141");
    }

    // --- Large-data path (total > 20 bytes): SHA1("mrCloud" + data + length_str) ---

    /// 21 bytes crosses into SHA1 path.
    #[test]
    fn mailru_21_bytes() {
        let data = [b'C'; 21];
        assert_eq!(hash(&data), "de6f4121a7e0230684bc6fa002cb5052ab73020c");
    }

    /// 100 bytes of 'x'.
    #[test]
    fn mailru_100_bytes() {
        let data = [b'x'; 100];
        assert_eq!(hash(&data), "195475d61466510189551d1d84ab1bb35eaebf17");
    }

    // --- Streaming consistency ---

    /// Chunked updates must produce the same result as a single update (small data).
    #[test]
    fn chunked_small_matches_single() {
        let data = b"hello world";
        let single = hash(data);
        let mut h = MailruHasher::new();
        for chunk in data.chunks(3) {
            h.update(chunk);
        }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    /// Chunked updates must produce the same result as a single update (large data).
    #[test]
    fn chunked_large_matches_single() {
        let data = vec![0xABu8; 1024];
        let single = hash(&data);
        let mut h = MailruHasher::new();
        for chunk in data.chunks(100) {
            h.update(chunk);
        }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    /// The small-data and large-data paths must produce different results for
    /// overlapping content to confirm the two branches behave differently.
    #[test]
    fn small_and_large_paths_differ() {
        // 20 bytes -> small path (zero-padded)
        let result_small = hash(&[b'Z'; 20]);
        // 21 bytes -> large path (SHA1 with prefix + length)
        let result_large = hash(&[b'Z'; 21]);
        assert_ne!(result_small, result_large);
    }
}
