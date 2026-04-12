use crate::hashes::Hasher;
use xxhash_rust::xxh3::Xxh3;

pub struct Xxh3Hasher {
    inner: Xxh3,
}

pub struct Xxh128Hasher {
    inner: Xxh3,
}

impl Xxh3Hasher {
    pub fn new() -> Self {
        Self { inner: Xxh3::new() }
    }
}

impl Xxh128Hasher {
    pub fn new() -> Self {
        Self { inner: Xxh3::new() }
    }
}

impl Default for Xxh3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Xxh128Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Xxh3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        format!("{:016x}", self.inner.digest())
    }
}

impl Hasher for Xxh128Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        format!("{:032x}", self.inner.digest128())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Official xxHash test vectors from github.com/Cyan4973/xxHash
    #[test]
    fn xxh3_empty() {
        let mut h = Xxh3Hasher::new();
        h.update(b"");
        assert_eq!(Box::new(h).finalize_hex(), "2d06800538d394c2");
    }

    #[test]
    fn xxh128_empty() {
        let mut h = Xxh128Hasher::new();
        h.update(b"");
        assert_eq!(
            Box::new(h).finalize_hex(),
            "99aa06d3014798d86001c324468d497f"
        );
    }

    #[test]
    fn xxh3_chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut h1 = Xxh3Hasher::new();
        h1.update(data);
        let single = Box::new(h1).finalize_hex();
        let mut h2 = Xxh3Hasher::new();
        for chunk in data.chunks(9) {
            h2.update(chunk);
        }
        assert_eq!(Box::new(h2).finalize_hex(), single);
    }

    #[test]
    fn xxh128_chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut h1 = Xxh128Hasher::new();
        h1.update(data);
        let single = Box::new(h1).finalize_hex();
        let mut h2 = Xxh128Hasher::new();
        for chunk in data.chunks(9) {
            h2.update(chunk);
        }
        assert_eq!(Box::new(h2).finalize_hex(), single);
    }

    #[test]
    fn xxh3_default_equals_new() {
        let mut h = Xxh3Hasher::default();
        h.update(b"abc");
        let mut expected = Xxh3Hasher::new();
        expected.update(b"abc");
        assert_eq!(
            Box::new(h).finalize_hex(),
            Box::new(expected).finalize_hex()
        );
    }

    #[test]
    fn xxh128_default_equals_new() {
        let mut h = Xxh128Hasher::default();
        h.update(b"abc");
        let mut expected = Xxh128Hasher::new();
        expected.update(b"abc");
        assert_eq!(
            Box::new(h).finalize_hex(),
            Box::new(expected).finalize_hex()
        );
    }

    #[test]
    fn xxh3_output_is_16_hex_chars() {
        let mut h = Xxh3Hasher::new();
        h.update(b"test");
        assert_eq!(Box::new(h).finalize_hex().len(), 16);
    }

    #[test]
    fn xxh128_output_is_32_hex_chars() {
        let mut h = Xxh128Hasher::new();
        h.update(b"test");
        assert_eq!(Box::new(h).finalize_hex().len(), 32);
    }
}
