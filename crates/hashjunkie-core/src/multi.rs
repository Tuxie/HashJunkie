use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use rayon::prelude::*;

use crate::Algorithm;
use crate::hashes::{self, Hasher};

const PARALLEL_UPDATE_MIN: usize = 128 * 1024;
const PIPELINE_QUEUE_DEPTH: usize = 2;

pub struct MultiHasher {
    pairs: Vec<(Algorithm, Box<dyn Hasher>)>,
}

#[derive(Debug)]
pub enum PipelinedHashError {
    WorkerStopped,
    WorkerPanicked,
}

impl fmt::Display for PipelinedHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelinedHashError::WorkerStopped => f.write_str("hash worker stopped unexpectedly"),
            PipelinedHashError::WorkerPanicked => f.write_str("hash worker panicked"),
        }
    }
}

impl std::error::Error for PipelinedHashError {}

pub struct PipelinedMultiHasher {
    senders: Option<Vec<mpsc::SyncSender<Arc<[u8]>>>>,
    workers: Vec<thread::JoinHandle<WorkerResult>>,
}

struct WorkerResult {
    algorithm: Algorithm,
    digest: String,
    elapsed: Duration,
}

impl PipelinedMultiHasher {
    pub fn new(algorithms: &[Algorithm]) -> Self {
        let mut seen = HashSet::new();
        let algorithms = algorithms.iter().copied().filter(|alg| seen.insert(*alg));

        let mut senders = Vec::new();
        let mut workers = Vec::new();

        for algorithm in algorithms {
            let (sender, receiver) = mpsc::sync_channel(PIPELINE_QUEUE_DEPTH);
            senders.push(sender);
            workers.push(thread::spawn(move || {
                hash_one_algorithm(algorithm, receiver)
            }));
        }

        Self {
            senders: Some(senders),
            workers,
        }
    }

    pub fn update(&mut self, data: &[u8]) -> Result<(), PipelinedHashError> {
        let chunk: Arc<[u8]> = Arc::from(data.to_vec().into_boxed_slice());
        let senders = self
            .senders
            .as_ref()
            .ok_or(PipelinedHashError::WorkerStopped)?;

        for sender in senders {
            sender
                .send(Arc::clone(&chunk))
                .map_err(|_| PipelinedHashError::WorkerStopped)?;
        }

        Ok(())
    }

    pub fn finalize(mut self) -> Result<HashMap<Algorithm, String>, PipelinedHashError> {
        self.senders.take();

        let mut digests = HashMap::new();
        for worker in self.workers {
            let result = worker
                .join()
                .map_err(|_| PipelinedHashError::WorkerPanicked)?;
            if std::env::var_os("HASHJUNKIE_PROFILE_PIPELINE").is_some() {
                eprintln!(
                    "hashjunkie pipeline {:>9}: {:.3}s",
                    result.algorithm,
                    result.elapsed.as_secs_f64()
                );
            }
            digests.insert(result.algorithm, result.digest);
        }

        Ok(digests)
    }
}

fn hash_one_algorithm(algorithm: Algorithm, chunks: mpsc::Receiver<Arc<[u8]>>) -> WorkerResult {
    let mut elapsed = Duration::ZERO;
    let mut hasher = MultiHasher::new(&[algorithm]);
    for chunk in chunks {
        let started = std::time::Instant::now();
        hasher.update(&chunk);
        elapsed += started.elapsed();
    }

    let mut digests = hasher.finalize();
    let digest = digests
        .remove(&algorithm)
        .expect("single-algorithm hasher returns its digest");
    WorkerResult {
        algorithm,
        digest,
        elapsed,
    }
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

    pub fn update_parallel(&mut self, data: &[u8]) {
        if self.pairs.len() < 2 || data.len() < PARALLEL_UPDATE_MIN {
            self.update(data);
            return;
        }

        self.pairs
            .par_iter_mut()
            .for_each(|(_, hasher)| hasher.update(data));
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
        Algorithm::Btv2 => Box::new(Btv2Hasher::new()),
        Algorithm::CidV0 => Box::new(CidHasher::v0()),
        Algorithm::CidV1 => Box::new(CidHasher::v1()),
        Algorithm::Crc32 => Box::new(Crc32Hasher::new()),
        Algorithm::Dropbox => Box::new(DropboxHasher::new()),
        Algorithm::Ed2k => Box::new(Ed2kHasher::new()),
        Algorithm::Hidrive => Box::new(HidriveHasher::new()),
        Algorithm::Mailru => Box::new(MailruHasher::new()),
        Algorithm::Md5 => Box::new(RustCryptoHasher::<md5::Md5>::new()),
        Algorithm::QuickXor => Box::new(QuickXorHasher::new()),
        Algorithm::Sha1 => Box::new(RustCryptoHasher::<sha1::Sha1>::new()),
        Algorithm::Sha256 => Box::new(RustCryptoHasher::<sha2::Sha256>::new()),
        Algorithm::Sha512 => Box::new(RustCryptoHasher::<sha2::Sha512>::new()),
        Algorithm::Tiger => Box::new(TigerTreeHasher::new()),
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
    fn all_produces_default_algorithms_without_whirlpool() {
        let mut h = MultiHasher::all();
        h.update(b"");
        let digests = h.finalize();
        assert_eq!(digests.len(), 17);
        assert!(digests.contains_key(&Algorithm::Ed2k));
        assert!(digests.contains_key(&Algorithm::Tiger));
        assert!(!digests.contains_key(&Algorithm::Whirlpool));
    }

    #[test]
    fn explicit_whirlpool_is_still_supported() {
        let mut h = MultiHasher::new(&[Algorithm::Whirlpool]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(
            digests[&Algorithm::Whirlpool],
            "4e2448a4c6f486bb16b6562c73b4020bf3043e3a731bce721ae1b303d97e6d4c7181eebdb6c57e277d0e34957114cbd6c797fc9d95d8b582d225292076d4eef5"
        );
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
    fn parallel_update_matches_single_update() {
        let data = vec![7; PARALLEL_UPDATE_MIN * 2 + 13];
        let algs = &[
            Algorithm::Blake3,
            Algorithm::Btv2,
            Algorithm::Sha256,
            Algorithm::Md5,
            Algorithm::Xxh3,
            Algorithm::Dropbox,
        ];

        let mut sequential = MultiHasher::new(algs);
        sequential.update(&data);

        let mut parallel = MultiHasher::new(algs);
        parallel.update_parallel(&data);

        assert_eq!(parallel.finalize(), sequential.finalize());
    }

    #[test]
    fn parallel_update_falls_back_for_small_chunks_and_single_algorithm() {
        let small = vec![5; PARALLEL_UPDATE_MIN - 1];
        let algs = &[Algorithm::Blake3, Algorithm::Sha256];

        let mut sequential = MultiHasher::new(algs);
        sequential.update(&small);

        let mut fallback_small = MultiHasher::new(algs);
        fallback_small.update_parallel(&small);

        assert_eq!(fallback_small.finalize(), sequential.finalize());

        let large = vec![9; PARALLEL_UPDATE_MIN + 1];
        let mut sequential_single = MultiHasher::new(&[Algorithm::Sha256]);
        sequential_single.update(&large);

        let mut fallback_single = MultiHasher::new(&[Algorithm::Sha256]);
        fallback_single.update_parallel(&large);

        assert_eq!(fallback_single.finalize(), sequential_single.finalize());
    }

    #[test]
    fn pipelined_multi_hasher_matches_sequential_across_chunks() {
        let data = vec![19; 1024 * 1024 + 13];
        let algs = &[
            Algorithm::Blake3,
            Algorithm::Sha256,
            Algorithm::Sha512,
            Algorithm::CidV0,
            Algorithm::CidV1,
            Algorithm::Dropbox,
        ];

        let mut sequential = MultiHasher::new(algs);
        for chunk in data.chunks(123_457) {
            sequential.update(chunk);
        }

        let mut pipelined = PipelinedMultiHasher::new(algs);
        for chunk in data.chunks(123_457) {
            pipelined.update(chunk).unwrap();
        }

        assert_eq!(pipelined.finalize().unwrap(), sequential.finalize());
    }

    #[test]
    fn pipelined_multi_hasher_profile_branch_still_finalizes() {
        // SAFETY: this test restores the process-wide environment variable before
        // returning; the variable is read only after worker threads have joined.
        unsafe {
            std::env::set_var("HASHJUNKIE_PROFILE_PIPELINE", "1");
        }

        let mut pipelined = PipelinedMultiHasher::new(&[Algorithm::Sha256]);
        pipelined.update(b"abc").unwrap();
        let result = pipelined.finalize();

        // SAFETY: remove the test-only environment variable before returning.
        unsafe {
            std::env::remove_var("HASHJUNKIE_PROFILE_PIPELINE");
        }

        assert_eq!(
            result.unwrap()[&Algorithm::Sha256],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn pipelined_error_display_messages_are_stable() {
        assert_eq!(
            PipelinedHashError::WorkerStopped.to_string(),
            "hash worker stopped unexpectedly"
        );
        assert_eq!(
            PipelinedHashError::WorkerPanicked.to_string(),
            "hash worker panicked"
        );
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
