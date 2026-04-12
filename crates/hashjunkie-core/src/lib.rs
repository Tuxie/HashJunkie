mod algorithm;
pub mod hashes;

pub use algorithm::{Algorithm, UnknownAlgorithm};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
