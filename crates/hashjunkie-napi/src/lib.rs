#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

use hashjunkie_core::{Algorithm, MultiHasher};

fn parse_algorithms(names: Option<Vec<String>>) -> napi::Result<Vec<Algorithm>> {
    match names {
        None => Ok(Algorithm::all().to_vec()),
        Some(names) => {
            if names.is_empty() {
                return Err(napi::Error::from_reason(
                    "algorithms list must not be empty; omit the argument or pass null to use all algorithms",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_algorithms_none_returns_all_13() {
        let algs = parse_algorithms(None).unwrap();
        assert_eq!(algs.len(), 13);
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
        assert!(result
            .unwrap_err()
            .reason
            .contains("algorithms list must not be empty"));
    }
}

/// A streaming multi-algorithm hasher exposed as a Node.js native class.
///
/// ```js
/// const h = new NativeHasher(['sha256', 'blake3']);
/// h.update(Buffer.from('hello'));
/// const digests = h.finalize(); // { sha256: '...', blake3: '...' }
/// ```
#[napi]
pub struct NativeHasher {
    inner: Option<MultiHasher>,
}

#[napi]
impl NativeHasher {
    /// Create a new hasher. Pass an array of algorithm name strings (e.g.
    /// `['sha256', 'blake3']`) or omit / pass `null` / `undefined` to hash
    /// with all 13 algorithms. Throws if any name is unrecognised.
    #[napi(constructor)]
    pub fn new(algorithms: Option<Vec<String>>) -> napi::Result<Self> {
        let algs = parse_algorithms(algorithms)?;
        Ok(Self {
            inner: Some(MultiHasher::new(&algs)),
        })
    }

    /// Feed a chunk of data into all active hashers.
    /// Throws if called after `finalize()`.
    #[napi]
    pub fn update(&mut self, data: Buffer) -> napi::Result<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("hasher already finalized"))?
            .update(&data);
        Ok(())
    }

    /// Finalize all hashers and return a plain JS object mapping algorithm
    /// name to lowercase hex digest string. After this call, `update()` and
    /// `finalize()` will throw if called again.
    #[napi]
    pub fn finalize(&mut self) -> napi::Result<HashMap<String, String>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| napi::Error::from_reason("hasher already finalized"))?;
        Ok(inner
            .finalize()
            .into_iter()
            .map(|(alg, digest)| (alg.as_str().to_string(), digest))
            .collect())
    }
}
