mod algorithm;
pub mod hashes;
mod multi;

pub use algorithm::{Algorithm, UnknownAlgorithm};
pub use multi::MultiHasher;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
