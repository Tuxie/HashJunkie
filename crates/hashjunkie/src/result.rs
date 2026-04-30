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
