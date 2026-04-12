use std::collections::HashMap;

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

use hashjunkie_core::{Algorithm, MultiHasher};

pub(crate) fn parse_algorithm_names(names: Option<Vec<String>>) -> Result<Vec<Algorithm>, String> {
    match names {
        None => Ok(Algorithm::all().to_vec()),
        Some(names) => {
            if names.is_empty() {
                return Err(
                    "algorithms list must not be empty; omit the argument or pass null \
                     to use all algorithms"
                        .to_string(),
                );
            }
            names
                .iter()
                .map(|s| s.parse::<Algorithm>().map_err(|e| e.to_string()))
                .collect()
        }
    }
}

// Natively testable hasher core — no JsValue involved.
// Holds the MultiHasher behind an Option so finalize() can take ownership
// while keeping &mut self semantics required by wasm-bindgen.
struct HasherCore {
    inner: Option<MultiHasher>,
}

impl HasherCore {
    fn new(algs: &[Algorithm]) -> Self {
        Self {
            inner: Some(MultiHasher::new(algs)),
        }
    }

    fn update(&mut self, data: &[u8]) -> Result<(), &'static str> {
        self.inner
            .as_mut()
            .ok_or("hasher already finalized")?
            .update(data);
        Ok(())
    }

    fn finalize(&mut self) -> Result<HashMap<Algorithm, String>, &'static str> {
        self.inner
            .take()
            .ok_or("hasher already finalized")
            .map(MultiHasher::finalize)
    }
}

/// A streaming multi-algorithm hasher exposed to JavaScript via WebAssembly.
///
/// ```js
/// const h = new WasmHasher(['sha256', 'blake3']);
/// h.update(new Uint8Array([104, 101, 108, 108, 111]));
/// const digests = h.finalize(); // { sha256: '...', blake3: '...' }
/// ```
#[wasm_bindgen]
pub struct WasmHasher(HasherCore);

#[wasm_bindgen]
impl WasmHasher {
    /// Create a new hasher. Pass an array of algorithm name strings (e.g.
    /// `['sha256', 'blake3']`) or omit / pass `null` / `undefined` to hash
    /// with all 13 algorithms. Throws if any name is unrecognised or if an
    /// empty array is passed.
    #[wasm_bindgen(constructor)]
    pub fn new(algorithms: JsValue) -> Result<WasmHasher, JsValue> {
        let names: Option<Vec<String>> = if algorithms.is_null() || algorithms.is_undefined() {
            None
        } else {
            let arr = js_sys::Array::from(&algorithms);
            let mut names = Vec::with_capacity(arr.length() as usize);
            for val in arr.iter() {
                let s = val
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("algorithm name must be a string"))?;
                names.push(s);
            }
            Some(names)
        };
        let algs = parse_algorithm_names(names).map_err(|e| JsValue::from_str(&e))?;
        Ok(WasmHasher(HasherCore::new(&algs)))
    }

    /// Feed a chunk of data into all active hashers. Throws if called after `finalize()`.
    pub fn update(&mut self, data: &[u8]) -> Result<(), JsValue> {
        self.0.update(data).map_err(JsValue::from_str)
    }

    /// Finalize all hashers and return a plain JS object mapping algorithm
    /// name to lowercase hex digest string. Throws if called again after the
    /// first `finalize()`.
    pub fn finalize(&mut self) -> Result<JsValue, JsValue> {
        let digests = self.0.finalize().map_err(JsValue::from_str)?;
        let obj = Object::new();
        for (alg, digest) in &digests {
            Reflect::set(
                &obj,
                &JsValue::from_str(alg.as_str()),
                &JsValue::from_str(digest),
            )?;
        }
        Ok(obj.into())
    }
}

#[cfg(test)]
mod tests {
    use hashjunkie_core::Algorithm;

    use super::{parse_algorithm_names, HasherCore};

    #[test]
    fn parse_none_returns_all_algorithms() {
        let algs = parse_algorithm_names(None).unwrap();
        assert_eq!(algs.len(), Algorithm::all().len());
    }

    #[test]
    fn parse_two_known_names() {
        let names = vec!["sha256".to_string(), "blake3".to_string()];
        let algs = parse_algorithm_names(Some(names)).unwrap();
        assert_eq!(algs.len(), 2);
        assert!(algs.contains(&Algorithm::Sha256));
        assert!(algs.contains(&Algorithm::Blake3));
    }

    #[test]
    fn parse_unknown_name_returns_error() {
        let names = vec!["bogus".to_string()];
        assert!(parse_algorithm_names(Some(names)).is_err());
    }

    #[test]
    fn parse_empty_vec_returns_error() {
        assert!(parse_algorithm_names(Some(vec![])).is_err());
    }

    #[test]
    fn hasher_core_update_after_finalize_returns_error() {
        let mut core = HasherCore::new(&[Algorithm::Sha256]);
        let _ = core.finalize();
        assert!(core.update(b"oops").is_err());
    }

    #[test]
    fn hasher_core_double_finalize_returns_error() {
        let mut core = HasherCore::new(&[Algorithm::Sha256]);
        let _ = core.finalize();
        assert!(core.finalize().is_err());
    }

    #[test]
    fn hasher_core_sha256_matches_known_vector() {
        let mut core = HasherCore::new(&[Algorithm::Sha256]);
        core.update(b"abc").unwrap();
        let digests = core.finalize().unwrap();
        assert_eq!(
            digests[&Algorithm::Sha256],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
