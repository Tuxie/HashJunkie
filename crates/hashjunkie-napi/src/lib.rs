#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

use hashjunkie_core::{Algorithm, MultiHasher};

fn parse_algorithms(names: Option<Vec<String>>) -> napi::Result<Vec<Algorithm>> {
    match names {
        None => Ok(Algorithm::all().to_vec()),
        Some(names) => names
            .iter()
            .map(|s| {
                s.parse::<Algorithm>()
                    .map_err(|e| napi::Error::from_reason(e.to_string()))
            })
            .collect(),
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
