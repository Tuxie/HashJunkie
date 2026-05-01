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

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io::{self, Cursor, Read};

    use super::*;

    struct ErrorReader;

    impl Read for ErrorReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("injected read error"))
        }
    }

    #[test]
    fn default_helpers_hash_with_default_algorithm_set() {
        let bytes = hash_bytes_default(b"abc");
        assert_eq!(bytes.len(), Algorithm::all().len());

        let reader = hash_reader_default(Cursor::new(b"abc")).unwrap();
        assert_eq!(reader.len(), Algorithm::all().len());

        let file = hash_file_default("tests/fixtures/small.bin").unwrap();
        assert_eq!(file.len(), Algorithm::all().len());
    }

    #[test]
    fn single_algorithm_reader_uses_direct_path() {
        let result = hash_reader(Cursor::new(b"abc"), &[Algorithm::Sha256]).unwrap();

        assert_eq!(
            result.standard(Algorithm::Sha256),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
    }

    #[test]
    fn duplicate_algorithm_reader_uses_direct_path_once() {
        let result =
            hash_reader(Cursor::new(b"abc"), &[Algorithm::Sha256, Algorithm::Sha256]).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result.standard(Algorithm::Sha256),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
    }

    #[test]
    fn multi_algorithm_reader_uses_pipelined_path() {
        let result =
            hash_reader(Cursor::new(b"abc"), &[Algorithm::Sha256, Algorithm::Md5]).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(
            result.standard(Algorithm::Md5),
            Some("900150983cd24fb0d6963f7d28e17f72")
        );
    }

    #[test]
    fn file_hashing_reports_io_errors() {
        let err = hash_file(
            "/definitely/not/a/hashjunkie/test/file",
            &[Algorithm::Sha256],
        )
        .unwrap_err();

        assert!(matches!(err, HashError::Io(_)));
        assert!(err.source().is_some());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn reader_hashing_reports_io_errors() {
        let err = hash_reader(ErrorReader, &[Algorithm::Sha256]).unwrap_err();

        assert!(matches!(err, HashError::Io(_)));
        assert!(err.source().is_some());
        assert_eq!(err.to_string(), "injected read error");
    }

    #[test]
    fn pipeline_errors_report_sources() {
        let err = HashError::from(PipelinedHashError::WorkerStopped);

        assert!(matches!(err, HashError::Pipeline(_)));
        assert_eq!(err.to_string(), "hash worker stopped unexpectedly");
        assert!(err.source().is_some());
    }
}
