#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::{AsyncTask, Buffer};
use napi::{Env, Task};
use napi_derive::napi;

use hashjunkie::{
    Algorithm, DigestValue, HashResult, MultiHasher, PipelinedHashError, PipelinedMultiHasher,
    hash_file as hash_file_core,
};

fn parse_algorithms(names: Option<Vec<String>>) -> napi::Result<Vec<Algorithm>> {
    match names {
        None => Ok(Algorithm::all().to_vec()),
        Some(names) => {
            if names.is_empty() {
                return Err(napi::Error::from_reason(
                    "algorithms list must not be empty; omit the argument or pass null to use default algorithms",
                ));
            }
            names
                .iter()
                .map(|s| {
                    s.parse::<Algorithm>()
                        .map_err(|e| napi::Error::from_reason(e.to_string()))
                })
                .collect()
        }
    }
}

fn digest_map_from_result(result: HashResult) -> HashMap<String, String> {
    result
        .into_vec()
        .into_iter()
        .map(|(alg, digest)| (alg.as_str().to_string(), digest.standard().to_string()))
        .collect()
}

#[napi(object)]
pub struct DigestBundle {
    pub digests: HashMap<String, String>,
    pub hexdigests: HashMap<String, String>,
    pub rawdigests: HashMap<String, Buffer>,
}

fn digest_bundle(digests: HashMap<Algorithm, DigestValue>) -> DigestBundle {
    let mut standard = HashMap::new();
    let mut hex = HashMap::new();
    let mut raw = HashMap::new();

    for (alg, digest) in digests {
        let name = alg.as_str().to_string();
        standard.insert(name.clone(), digest.standard().to_string());
        hex.insert(name.clone(), digest.hex());
        raw.insert(name, Buffer::from(digest.into_raw()));
    }

    DigestBundle {
        digests: standard,
        hexdigests: hex,
        rawdigests: raw,
    }
}

fn io_error(path: &str, err: std::io::Error) -> napi::Error {
    napi::Error::from_reason(format!("failed to hash file {path}: {err}"))
}

fn pipeline_error(err: PipelinedHashError) -> napi::Error {
    napi::Error::from_reason(err.to_string())
}

fn hash_file(path: &str, algorithms: &[Algorithm]) -> napi::Result<HashMap<String, String>> {
    if algorithms == [Algorithm::Blake3] {
        let mut hasher = blake3::Hasher::new();
        hasher
            .update_mmap_rayon(path)
            .map_err(|err| io_error(path, err))?;
        return Ok(HashMap::from([(
            Algorithm::Blake3.as_str().to_string(),
            hasher.finalize().to_hex().to_string(),
        )]));
    }

    let result = hash_file_core(path, algorithms)
        .map_err(|err| napi::Error::from_reason(format!("failed to hash file {path}: {err}")))?;
    Ok(digest_map_from_result(result))
}

pub struct HashFileTask {
    path: String,
    algorithms: Vec<Algorithm>,
}

impl Task for HashFileTask {
    type Output = HashMap<String, String>;
    type JsValue = HashMap<String, String>;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        hash_file(&self.path, &self.algorithms)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(output)
    }
}

enum StreamingHasher {
    Direct(MultiHasher),
    Pipelined(PipelinedMultiHasher),
}

impl StreamingHasher {
    fn new(algorithms: &[Algorithm]) -> Self {
        if algorithms.len() <= 1 {
            Self::Direct(MultiHasher::new(algorithms))
        } else {
            Self::Pipelined(PipelinedMultiHasher::new(algorithms))
        }
    }

    fn update(&mut self, data: &[u8]) -> napi::Result<()> {
        match self {
            Self::Direct(hasher) => {
                hasher.update(data);
                Ok(())
            }
            Self::Pipelined(hasher) => hasher.update(data).map_err(pipeline_error),
        }
    }

    #[cfg(test)]
    fn finalize(self) -> napi::Result<HashMap<Algorithm, String>> {
        match self {
            Self::Direct(hasher) => Ok(hasher.finalize()),
            Self::Pipelined(hasher) => hasher.finalize().map_err(pipeline_error),
        }
    }

    fn finalize_digests(self) -> napi::Result<HashMap<Algorithm, DigestValue>> {
        match self {
            Self::Direct(hasher) => Ok(hasher.finalize_digests()),
            Self::Pipelined(hasher) => hasher.finalize_digests().map_err(pipeline_error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_algorithms_none_returns_default_18_without_whirlpool() {
        let algs = parse_algorithms(None).unwrap();
        assert_eq!(algs.len(), 18);
        assert!(algs.contains(&Algorithm::Aich));
        assert!(algs.contains(&Algorithm::Btv2));
        assert!(algs.contains(&Algorithm::Ed2k));
        assert!(algs.contains(&Algorithm::Tiger));
        assert!(!algs.contains(&Algorithm::Whirlpool));
    }

    #[test]
    fn parse_algorithms_subset_returns_correct_variants() {
        let algs =
            parse_algorithms(Some(vec!["sha256".to_string(), "blake3".to_string()])).unwrap();
        assert_eq!(algs.len(), 2);
        assert!(algs.contains(&Algorithm::Sha256));
        assert!(algs.contains(&Algorithm::Blake3));
    }

    #[test]
    fn parse_algorithms_unknown_name_returns_err() {
        let result = parse_algorithms(Some(vec!["bogus".to_string()]));
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("unknown algorithm"));
    }

    // Regression: passing an empty Vec was silently producing a hasher that
    // returns an empty digest map — almost certainly a caller mistake.
    #[test]
    fn parse_algorithms_empty_vec_returns_err() {
        let result = parse_algorithms(Some(vec![]));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .reason
                .contains("algorithms list must not be empty")
        );
    }

    #[test]
    fn hash_file_blake3_matches_known_value() {
        let path =
            std::env::temp_dir().join(format!("hashjunkie-napi-test-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        let result = hash_file(path.to_str().unwrap(), &[Algorithm::Blake3]).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(
            result[Algorithm::Blake3.as_str()],
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn hash_file_multi_hash_matches_known_values() {
        let path =
            std::env::temp_dir().join(format!("hashjunkie-napi-test-multi-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        let result =
            hash_file(path.to_str().unwrap(), &[Algorithm::Sha256, Algorithm::Md5]).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(
            result[Algorithm::Sha256.as_str()],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            result[Algorithm::Md5.as_str()],
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn streaming_hasher_uses_pipeline_for_multiple_algorithms() {
        let hasher = StreamingHasher::new(&[Algorithm::Sha256, Algorithm::Sha512]);
        assert!(matches!(hasher, StreamingHasher::Pipelined(_)));
    }

    #[test]
    fn streaming_hasher_matches_direct_multi_hasher_across_chunks() {
        let data = vec![23; 1024 * 1024 + 17];
        let algorithms = [
            Algorithm::Blake3,
            Algorithm::Sha256,
            Algorithm::Sha512,
            Algorithm::CidV0,
            Algorithm::CidV1,
            Algorithm::Dropbox,
        ];

        let mut streaming = StreamingHasher::new(&algorithms);
        for chunk in data.chunks(123_457) {
            streaming.update(chunk).unwrap();
        }

        let mut direct = MultiHasher::new(&algorithms);
        for chunk in data.chunks(123_457) {
            direct.update(chunk);
        }

        assert_eq!(streaming.finalize().unwrap(), direct.finalize());
    }

    #[test]
    fn streaming_hasher_exposes_standard_hex_and_raw_digests() {
        let mut streaming = StreamingHasher::new(&[Algorithm::CidV1, Algorithm::Aich]);
        streaming.update(b"abc").unwrap();
        let bundle = digest_bundle(streaming.finalize_digests().unwrap());

        assert_eq!(
            bundle.digests[Algorithm::CidV1.as_str()],
            "bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu"
        );
        assert_eq!(
            bundle.hexdigests[Algorithm::CidV1.as_str()],
            "01551220ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            bundle.hexdigests[Algorithm::Aich.as_str()],
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(bundle.rawdigests[Algorithm::Aich.as_str()].len(), 20);
    }
}

/// A streaming multi-algorithm hasher exposed as a Node.js native class.
///
/// ```js
/// const h = new NativeHasher(['sha256', 'blake3']);
/// h.update(Buffer.from('hello'));
/// const bundle = h.finalize(); // { digests, hexdigests, rawdigests }
/// ```
#[napi]
pub struct NativeHasher {
    inner: Option<StreamingHasher>,
}

#[napi]
impl NativeHasher {
    /// Create a new hasher. Pass an array of algorithm name strings (e.g.
    /// `['sha256', 'blake3']`) or omit / pass `null` / `undefined` to hash
    /// with the default algorithms. Whirlpool is supported but opt-in because
    /// it is much slower than the other hashes. Throws if any name is unrecognised.
    #[napi(constructor)]
    pub fn new(algorithms: Option<Vec<String>>) -> napi::Result<Self> {
        let algs = parse_algorithms(algorithms)?;
        Ok(Self {
            inner: Some(StreamingHasher::new(&algs)),
        })
    }

    /// Feed a chunk of data into all active hashers.
    /// Throws if called after `finalize()`.
    #[napi]
    pub fn update(&mut self, data: Buffer) -> napi::Result<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("hasher already finalized"))?
            .update(&data)?;
        Ok(())
    }

    /// Finalize all hashers and return standard, hex, and raw digest maps.
    /// After this call, `update()` and
    /// `finalize()` will throw if called again.
    #[napi]
    pub fn finalize(&mut self) -> napi::Result<DigestBundle> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| napi::Error::from_reason("hasher already finalized"))?;
        Ok(digest_bundle(inner.finalize_digests()?))
    }
}

/// Hash a local file on libuv's worker pool. The BLAKE3-only path uses
/// BLAKE3's mmap+rayon whole-file implementation; mixed algorithms use one
/// large-buffer file read feeding every requested hasher.
#[napi(js_name = "hashFile")]
pub fn hash_file_async(
    path: String,
    algorithms: Option<Vec<String>>,
) -> napi::Result<AsyncTask<HashFileTask>> {
    let algorithms = parse_algorithms(algorithms)?;
    Ok(AsyncTask::new(HashFileTask { path, algorithms }))
}
