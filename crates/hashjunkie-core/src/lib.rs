mod algorithm;
mod digest;
pub mod hashes;
mod multi;

pub use algorithm::{Algorithm, UnknownAlgorithm};
pub use digest::{
    DigestValue, base32_lower_no_padding_multibase, base32_upper_no_padding, bytes_to_lower_hex,
};
pub use multi::{MultiHasher, PipelinedHashError, PipelinedMultiHasher};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
