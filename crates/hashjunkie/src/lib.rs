//! Fast multi-algorithm hashing for files, streams, and byte slices.
//!
//! HashJunkie computes many standard, cloud, and file-sharing hashes in one
//! streaming pass. The high-level helpers are usually the right API for Rust
//! applications:
//!
//! ```
//! use hashjunkie::{Algorithm, hash_bytes};
//!
//! let result = hash_bytes(b"hello", &[Algorithm::Blake3, Algorithm::Sha256]);
//! assert_eq!(
//!     result.standard(Algorithm::Sha256),
//!     Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
//! );
//! ```
//!
//! For files and arbitrary [`std::io::Read`] sources, use [`hash_file`] or
//! [`hash_reader`]. These helpers use the same pipelined multi-hash engine as
//! the CLI when several algorithms are requested.
//!
//! ```
//! # use std::io::Cursor;
//! use hashjunkie::{Algorithm, hash_reader};
//!
//! let reader = Cursor::new(b"hello");
//! let result = hash_reader(reader, &[Algorithm::CidV1, Algorithm::Blake3])?;
//! assert!(result.standard(Algorithm::CidV1).unwrap().starts_with("bafk"));
//! # Ok::<(), hashjunkie::HashError>(())
//! ```

mod algorithm;
mod digest;
mod hashes;
mod io;
mod multi;
mod result;

pub use algorithm::{Algorithm, UnknownAlgorithm};
pub use digest::{
    DigestValue, base32_lower_no_padding_multibase, base32_upper_no_padding, bytes_to_lower_hex,
};
#[cfg(feature = "profile-ipfs-cid")]
pub use hashes::{CidProfile, reset_profile, take_profile};
pub use io::{
    DEFAULT_CHUNK_SIZE, HashError, hash_bytes, hash_bytes_default, hash_file, hash_file_default,
    hash_reader, hash_reader_default,
};
pub use multi::{MultiHasher, PipelinedHashError, PipelinedMultiHasher};
pub use result::HashResult;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
