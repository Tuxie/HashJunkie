use std::collections::HashMap;
use std::slice;

use crate::{Algorithm, DigestValue};

/// Completed digest values for one hashing run.
///
/// Results keep the caller's requested algorithm order, with duplicates removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashResult {
    digests: Vec<(Algorithm, DigestValue)>,
}

impl HashResult {
    pub(crate) fn from_digest_map(
        algorithms: &[Algorithm],
        mut digests: HashMap<Algorithm, DigestValue>,
    ) -> Self {
        let mut ordered = Vec::with_capacity(digests.len());
        for algorithm in unique_algorithms(algorithms) {
            if let Some(digest) = digests.remove(&algorithm) {
                ordered.push((algorithm, digest));
            }
        }
        Self { digests: ordered }
    }

    /// Returns the number of digest values in this result.
    pub fn len(&self) -> usize {
        self.digests.len()
    }

    /// Returns true when this result contains no digest values.
    pub fn is_empty(&self) -> bool {
        self.digests.is_empty()
    }

    /// Returns a digest by algorithm.
    pub fn get(&self, algorithm: Algorithm) -> Option<&DigestValue> {
        self.digests
            .iter()
            .find_map(|(candidate, digest)| (*candidate == algorithm).then_some(digest))
    }

    /// Returns the algorithm's standard display form.
    ///
    /// For most hashes this is lowercase hex. For algorithms with a standard
    /// non-hex representation, such as CIDv1 or AICH, this returns that form.
    pub fn standard(&self, algorithm: Algorithm) -> Option<&str> {
        self.get(algorithm).map(DigestValue::standard)
    }

    /// Returns the algorithm's lowercase hexadecimal digest.
    pub fn hex(&self, algorithm: Algorithm) -> Option<String> {
        self.get(algorithm).map(DigestValue::hex)
    }

    /// Returns the algorithm's raw digest bytes.
    pub fn raw(&self, algorithm: Algorithm) -> Option<&[u8]> {
        self.get(algorithm).map(DigestValue::raw)
    }

    /// Iterates over digest values in the caller's requested algorithm order.
    pub fn iter(&self) -> impl Iterator<Item = (Algorithm, &DigestValue)> {
        self.digests
            .iter()
            .map(|(algorithm, digest)| (*algorithm, digest))
    }

    /// Returns ordered digest values as a slice.
    pub fn as_slice(&self) -> &[(Algorithm, DigestValue)] {
        &self.digests
    }

    /// Converts this result into ordered digest values.
    pub fn into_vec(self) -> Vec<(Algorithm, DigestValue)> {
        self.digests
    }
}

impl IntoIterator for HashResult {
    type Item = (Algorithm, DigestValue);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.digests.into_iter()
    }
}

impl<'a> IntoIterator for &'a HashResult {
    type Item = (Algorithm, &'a DigestValue);
    type IntoIter = HashResultIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        HashResultIter {
            inner: self.digests.iter(),
        }
    }
}

pub struct HashResultIter<'a> {
    inner: slice::Iter<'a, (Algorithm, DigestValue)>,
}

impl<'a> Iterator for HashResultIter<'a> {
    type Item = (Algorithm, &'a DigestValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(algorithm, digest)| (*algorithm, digest))
    }
}

pub(crate) fn unique_algorithms(algorithms: &[Algorithm]) -> Vec<Algorithm> {
    let mut unique = Vec::with_capacity(algorithms.len());
    for algorithm in algorithms {
        if !unique.contains(algorithm) {
            unique.push(*algorithm);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn sample_result() -> HashResult {
        HashResult::from_digest_map(
            &[Algorithm::Sha256, Algorithm::Md5, Algorithm::Sha256],
            HashMap::from([
                (
                    Algorithm::Md5,
                    DigestValue::from_hex("900150983cd24fb0d6963f7d28e17f72").unwrap(),
                ),
                (
                    Algorithm::Sha256,
                    DigestValue::from_hex(
                        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                    )
                    .unwrap(),
                ),
            ]),
        )
    }

    #[test]
    fn result_accessors_report_ordered_digests() {
        let result = sample_result();

        assert_eq!(result.len(), 2);
        assert!(!result.is_empty());
        assert_eq!(result.as_slice()[0].0, Algorithm::Sha256);
        assert_eq!(
            result.standard(Algorithm::Md5),
            Some("900150983cd24fb0d6963f7d28e17f72")
        );
        assert_eq!(
            result.hex(Algorithm::Sha256).as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        assert!(result.get(Algorithm::Whirlpool).is_none());
    }

    #[test]
    fn result_iterators_return_requested_order() {
        let result = sample_result();

        let borrowed = (&result)
            .into_iter()
            .map(|(algorithm, _)| algorithm)
            .collect::<Vec<_>>();
        assert_eq!(borrowed, vec![Algorithm::Sha256, Algorithm::Md5]);

        let iterated = result
            .iter()
            .map(|(algorithm, _)| algorithm)
            .collect::<Vec<_>>();
        assert_eq!(iterated, vec![Algorithm::Sha256, Algorithm::Md5]);

        let owned = result
            .into_iter()
            .map(|(algorithm, _)| algorithm)
            .collect::<Vec<_>>();
        assert_eq!(owned, vec![Algorithm::Sha256, Algorithm::Md5]);
    }

    #[test]
    fn empty_result_is_empty() {
        let result = HashResult::from_digest_map(&[], HashMap::new());

        assert_eq!(result.len(), 0);
        assert!(result.is_empty());
        assert_eq!(result.as_slice(), &[]);
        assert_eq!(result.into_vec(), Vec::new());
    }
}
