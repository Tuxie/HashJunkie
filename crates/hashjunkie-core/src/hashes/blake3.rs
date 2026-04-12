use crate::hashes::Hasher;

pub struct Blake3Hasher {
    inner: blake3::Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self {
            inner: blake3::Hasher::new(),
        }
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Blake3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        hex::encode(self.inner.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = Blake3Hasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // Official BLAKE3 test vectors from github.com/BLAKE3-team/BLAKE3
    #[test]
    fn blake3_empty() {
        assert_eq!(
            hash(b""),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn blake3_abc() {
        assert_eq!(
            hash(b"abc"),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn default_equals_new() {
        let mut h = Blake3Hasher::default();
        h.update(b"abc");
        assert_eq!(Box::new(h).finalize_hex(), hash(b"abc"));
    }

    #[test]
    fn chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let single = hash(data);
        let mut h = Blake3Hasher::new();
        for chunk in data.chunks(5) {
            h.update(chunk);
        }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }
}
