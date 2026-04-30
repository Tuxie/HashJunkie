use crate::hashes::Hasher;

pub struct Crc32Hasher {
    inner: crc32fast::Hasher,
}

impl Crc32Hasher {
    pub fn new() -> Self {
        Self {
            inner: crc32fast::Hasher::new(),
        }
    }
}

impl Default for Crc32Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Crc32Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        format!("{:08x}", self.inner.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = Crc32Hasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // IEEE 802.3 CRC32 vectors
    #[test]
    fn crc32_empty() {
        assert_eq!(hash(b""), "00000000");
    }

    #[test]
    fn crc32_abc() {
        assert_eq!(hash(b"abc"), "352441c2");
    }

    #[test]
    fn crc32_123456789() {
        assert_eq!(hash(b"123456789"), "cbf43926");
    }

    #[test]
    fn default_equals_new() {
        let mut h = Crc32Hasher::default();
        h.update(b"abc");
        assert_eq!(Box::new(h).finalize_hex(), hash(b"abc"));
    }

    #[test]
    fn chunked_matches_single() {
        let data = b"the quick brown fox";
        let single = hash(data);
        let mut h = Crc32Hasher::new();
        for chunk in data.chunks(3) {
            h.update(chunk);
        }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }
}
