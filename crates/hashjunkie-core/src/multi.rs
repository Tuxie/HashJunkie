use std::collections::{HashMap, HashSet};

use crate::Algorithm;
use crate::hashes::{self, Hasher};

pub struct MultiHasher {
    pairs: Vec<(Algorithm, Box<dyn Hasher>)>,
}

impl MultiHasher {
    pub fn new(algorithms: &[Algorithm]) -> Self {
        let mut seen = HashSet::new();
        let pairs = algorithms
            .iter()
            .filter(|&&alg| seen.insert(alg))
            .map(|&alg| (alg, make_hasher(alg)))
            .collect();
        Self { pairs }
    }

    pub fn all() -> Self {
        Self::new(Algorithm::all())
    }

    pub fn update(&mut self, data: &[u8]) {
        for (_, hasher) in &mut self.pairs {
            hasher.update(data);
        }
    }

    pub fn finalize(self) -> HashMap<Algorithm, String> {
        self.pairs
            .into_iter()
            .map(|(alg, hasher)| (alg, hasher.finalize_hex()))
            .collect()
    }
}

fn make_hasher(alg: Algorithm) -> Box<dyn Hasher> {
    use hashes::*;
    match alg {
        Algorithm::Blake3 => Box::new(Blake3Hasher::new()),
        Algorithm::CidV0 => Box::new(CidHasher::v0()),
        Algorithm::CidV1 => Box::new(CidHasher::v1()),
        Algorithm::Crc32 => Box::new(Crc32Hasher::new()),
        Algorithm::Dropbox => Box::new(DropboxHasher::new()),
        Algorithm::Hidrive => Box::new(HidriveHasher::new()),
        Algorithm::Mailru => Box::new(MailruHasher::new()),
        Algorithm::Md5 => Box::new(RustCryptoHasher::<md5::Md5>::new()),
        Algorithm::QuickXor => Box::new(QuickXorHasher::new()),
        Algorithm::Sha1 => Box::new(RustCryptoHasher::<sha1::Sha1>::new()),
        Algorithm::Sha256 => Box::new(RustCryptoHasher::<sha2::Sha256>::new()),
        Algorithm::Sha512 => Box::new(RustCryptoHasher::<sha2::Sha512>::new()),
        Algorithm::Whirlpool => Box::new(RustCryptoHasher::<whirlpool::Whirlpool>::new()),
        Algorithm::Xxh128 => Box::new(Xxh128Hasher::new()),
        Algorithm::Xxh3 => Box::new(Xxh3Hasher::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_subset_produces_only_requested_algorithms() {
        let algs = &[Algorithm::Md5, Algorithm::Sha256];
        let mut h = MultiHasher::new(algs);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(digests.len(), 2);
        assert!(digests.contains_key(&Algorithm::Md5));
        assert!(digests.contains_key(&Algorithm::Sha256));
    }

    #[test]
    fn all_produces_all_15_algorithms() {
        let mut h = MultiHasher::all();
        h.update(b"");
        let digests = h.finalize();
        assert_eq!(digests.len(), 15);
    }

    #[test]
    fn md5_result_matches_standalone_hasher() {
        let mut h = MultiHasher::new(&[Algorithm::Md5]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(digests[&Algorithm::Md5], "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn sha256_result_matches_standalone_hasher() {
        let mut h = MultiHasher::new(&[Algorithm::Sha256]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(
            digests[&Algorithm::Sha256],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn multi_update_in_chunks_matches_single_update() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let algs = &[Algorithm::Blake3, Algorithm::Sha256, Algorithm::Xxh3];

        let mut h1 = MultiHasher::new(algs);
        h1.update(data);
        let single = h1.finalize();

        let mut h2 = MultiHasher::new(algs);
        for chunk in data.chunks(11) {
            h2.update(chunk);
        }
        let chunked = h2.finalize();

        assert_eq!(single, chunked);
    }

    #[test]
    fn empty_algorithms_slice_produces_empty_result() {
        let mut h = MultiHasher::new(&[]);
        h.update(b"data");
        assert_eq!(h.finalize().len(), 0);
    }

    #[test]
    fn duplicate_algorithms_are_deduplicated() {
        let algs = &[Algorithm::Md5, Algorithm::Md5, Algorithm::Md5];
        let mut h = MultiHasher::new(algs);
        h.update(b"abc");
        assert_eq!(h.finalize().len(), 1);
    }
}
