use std::env;

use hashjunkie::{Algorithm, hash_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .expect("usage: cargo run -p hashjunkie --example hash_file -- PATH");
    let result = hash_file(path, &[Algorithm::Blake3, Algorithm::Sha256])?;

    for (algorithm, digest) in &result {
        println!("{algorithm}: {}", digest.standard());
    }

    Ok(())
}
