use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::result::unique_algorithms;
use crate::{Algorithm, HashResult, MultiHasher, PipelinedHashError, PipelinedMultiHasher};

/// Default streaming chunk size used by HashJunkie's high-level reader helpers.
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;
const _: () = assert!(DEFAULT_CHUNK_SIZE >= 128 * 1024);

/// Error returned by high-level reader and file hashing helpers.
#[derive(Debug)]
pub enum HashError {
    Io(io::Error),
    Pipeline(PipelinedHashError),
}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashError::Io(err) => err.fmt(f),
            HashError::Pipeline(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for HashError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HashError::Io(err) => Some(err),
            HashError::Pipeline(err) => Some(err),
        }
    }
}

impl From<io::Error> for HashError {
    fn from(err: io::Error) -> Self {
        HashError::Io(err)
    }
}

impl From<PipelinedHashError> for HashError {
    fn from(err: PipelinedHashError) -> Self {
        HashError::Pipeline(err)
    }
}

/// Hashes a byte slice with the requested algorithms.
pub fn hash_bytes(data: &[u8], algorithms: &[Algorithm]) -> HashResult {
    let mut hasher = MultiHasher::new(algorithms);
    hasher.update_parallel(data);
    HashResult::from_digest_map(algorithms, hasher.finalize_digests())
}

/// Hashes a byte slice with HashJunkie's default algorithms.
pub fn hash_bytes_default(data: &[u8]) -> HashResult {
    hash_bytes(data, Algorithm::all())
}

/// Hashes all bytes read from a reader with the requested algorithms.
///
/// When more than one algorithm is requested, this uses HashJunkie's pipelined
/// multi-hash implementation so independent algorithms can run concurrently.
pub fn hash_reader<R: Read>(reader: R, algorithms: &[Algorithm]) -> Result<HashResult, HashError> {
    if unique_algorithms(algorithms).len() > 1 {
        hash_reader_pipelined(reader, algorithms)
    } else {
        hash_reader_direct(reader, algorithms)
    }
}

/// Hashes all bytes read from a reader with HashJunkie's default algorithms.
pub fn hash_reader_default<R: Read>(reader: R) -> Result<HashResult, HashError> {
    hash_reader(reader, Algorithm::all())
}

/// Hashes a file with the requested algorithms.
pub fn hash_file(
    path: impl AsRef<Path>,
    algorithms: &[Algorithm],
) -> Result<HashResult, HashError> {
    let file = File::open(path)?;
    hash_reader(file, algorithms)
}

/// Hashes a file with HashJunkie's default algorithms.
pub fn hash_file_default(path: impl AsRef<Path>) -> Result<HashResult, HashError> {
    hash_file(path, Algorithm::all())
}

fn hash_reader_direct<R: Read>(
    mut reader: R,
    algorithms: &[Algorithm],
) -> Result<HashResult, HashError> {
    let mut hasher = MultiHasher::new(algorithms);
    let mut buffer = vec![0u8; DEFAULT_CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update_parallel(&buffer[..n]);
    }
    Ok(HashResult::from_digest_map(
        algorithms,
        hasher.finalize_digests(),
    ))
}

fn hash_reader_pipelined<R: Read>(
    mut reader: R,
    algorithms: &[Algorithm],
) -> Result<HashResult, HashError> {
    let mut hasher = PipelinedMultiHasher::new(algorithms);
    let mut buffer = vec![0u8; DEFAULT_CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n])?;
    }
    Ok(HashResult::from_digest_map(
        algorithms,
        hasher.finalize_digests()?,
    ))
}
