use crate::hashes::Hasher;
use digest::Digest;

pub trait RustCryptoHashable: Digest + Default + Send + 'static {}
impl<T: Digest + Default + Send + 'static> RustCryptoHashable for T {}

pub struct RustCryptoHasher<D: RustCryptoHashable> {
    inner: D,
}

impl<D: RustCryptoHashable> RustCryptoHasher<D> {
    pub fn new() -> Self {
        Self {
            inner: D::default(),
        }
    }
}

impl<D: RustCryptoHashable> Default for RustCryptoHasher<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: RustCryptoHashable> Hasher for RustCryptoHasher<D> {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        hex::encode(self.inner.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash_empty<H: RustCryptoHashable>() -> String {
        let mut h = RustCryptoHasher::<H>::new();
        h.update(b"");
        Box::new(h).finalize_hex()
    }

    fn hash_bytes<H: RustCryptoHashable>(data: &[u8]) -> String {
        let mut h = RustCryptoHasher::<H>::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // MD5 vectors: RFC 1321
    #[test]
    fn md5_empty() {
        assert_eq!(hash_empty::<md5::Md5>(), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn md5_abc() {
        assert_eq!(
            hash_bytes::<md5::Md5>(b"abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    // SHA1 vectors: FIPS 180-4
    #[test]
    fn sha1_empty() {
        assert_eq!(
            hash_empty::<sha1::Sha1>(),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }

    #[test]
    fn sha1_abc() {
        assert_eq!(
            hash_bytes::<sha1::Sha1>(b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    // SHA256 vectors: FIPS 180-4
    #[test]
    fn sha256_empty() {
        assert_eq!(
            hash_empty::<sha2::Sha256>(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc() {
        assert_eq!(
            hash_bytes::<sha2::Sha256>(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    // SHA512 vectors: FIPS 180-4
    #[test]
    fn sha512_empty() {
        assert_eq!(
            hash_empty::<sha2::Sha512>(),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
    }

    #[test]
    fn sha512_abc() {
        assert_eq!(
            hash_bytes::<sha2::Sha512>(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    // Whirlpool vectors: official test suite
    #[test]
    fn whirlpool_empty() {
        assert_eq!(
            hash_empty::<whirlpool::Whirlpool>(),
            "19fa61d75522a4669b44e39c1d2e1726c530232130d407f89afee0964997f7a73e83be698b288febcf88e3e03c4f0757ea8964e59b63d93708b138cc42a66eb3"
        );
    }

    #[test]
    fn whirlpool_abc() {
        assert_eq!(
            hash_bytes::<whirlpool::Whirlpool>(b"abc"),
            "4e2448a4c6f486bb16b6562c73b4020bf3043e3a731bce721ae1b303d97e6d4c7181eebdb6c57e277d0e34957114cbd6c797fc9d95d8b582d225292076d4eef5"
        );
    }

    #[test]
    fn update_in_chunks_matches_single_update() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut h1 = RustCryptoHasher::<sha2::Sha256>::new();
        h1.update(data);
        let single = Box::new(h1).finalize_hex();

        let mut h2 = RustCryptoHasher::<sha2::Sha256>::new();
        for chunk in data.chunks(7) {
            h2.update(chunk);
        }
        let chunked = Box::new(h2).finalize_hex();

        assert_eq!(single, chunked);
    }
}
