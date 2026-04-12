use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

use hashjunkie_core::{Algorithm, MultiHasher};

fn parse_algorithm_names(names: Option<Vec<String>>) -> Result<Vec<Algorithm>, String> {
    match names {
        None => Ok(Algorithm::all().to_vec()),
        Some(names) => {
            if names.is_empty() {
                return Err(
                    "algorithms list must not be empty; omit the argument or pass null to use all algorithms"
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

/// A streaming multi-algorithm hasher exposed to JavaScript via WebAssembly.
///
/// ```js
/// const h = new WasmHasher(['sha256', 'blake3']);
/// h.update(new Uint8Array([104, 101, 108, 108, 111]));
/// const digests = h.finalize(); // { sha256: '...', blake3: '...' }
/// ```
#[wasm_bindgen]
pub struct WasmHasher {
    inner: Option<MultiHasher>,
}

#[wasm_bindgen]
impl WasmHasher {
    /// Create a new hasher. Pass an array of algorithm name strings (e.g.
    /// `['sha256', 'blake3']`) or omit / pass `null` / `undefined` to hash
    /// with all 13 algorithms. Throws a `TypeError` if any name is unrecognised
    /// or if an empty array is passed.
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
        Ok(WasmHasher {
            inner: Some(MultiHasher::new(&algs)),
        })
    }

    /// Feed a chunk of data into all active hashers.
    /// Throws if called after `finalize()`.
    pub fn update(&mut self, data: &[u8]) -> Result<(), JsValue> {
        self.inner
            .as_mut()
            .ok_or_else(|| JsValue::from_str("hasher already finalized"))?
            .update(data);
        Ok(())
    }

    /// Finalize all hashers and return a plain JS object mapping algorithm
    /// name to lowercase hex digest string. After this call, `update()` and
    /// `finalize()` will throw if called again.
    pub fn finalize(&mut self) -> Result<JsValue, JsValue> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| JsValue::from_str("hasher already finalized"))?;

        let digests = inner.finalize();
        let obj = Object::new();
        for (alg, digest) in digests {
            Reflect::set(
                &obj,
                &JsValue::from_str(alg.as_str()),
                &JsValue::from_str(&digest),
            )?;
        }
        Ok(obj.into())
    }
}

#[cfg(test)]
mod tests {
    use hashjunkie_core::Algorithm;

    #[test]
    fn parse_none_returns_all_13_algorithms() {
        let algs = super::parse_algorithm_names(None).unwrap();
        assert_eq!(algs.len(), 13);
    }

    #[test]
    fn parse_two_known_names() {
        let names = vec!["sha256".to_string(), "blake3".to_string()];
        let algs = super::parse_algorithm_names(Some(names)).unwrap();
        assert_eq!(algs.len(), 2);
        assert!(algs.contains(&Algorithm::Sha256));
        assert!(algs.contains(&Algorithm::Blake3));
    }

    #[test]
    fn parse_unknown_name_returns_error() {
        let names = vec!["bogus".to_string()];
        assert!(super::parse_algorithm_names(Some(names)).is_err());
    }

    #[test]
    fn parse_empty_vec_returns_error() {
        assert!(super::parse_algorithm_names(Some(vec![])).is_err());
    }

    #[test]
    fn sha256_of_abc_matches_known_vector() {
        use hashjunkie_core::MultiHasher;
        let mut h = MultiHasher::new(&[Algorithm::Sha256]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(
            digests[&Algorithm::Sha256],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
