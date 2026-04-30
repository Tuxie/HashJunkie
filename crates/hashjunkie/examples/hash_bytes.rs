use hashjunkie::{Algorithm, hash_bytes};

fn main() {
    let result = hash_bytes(b"hello", &[Algorithm::Blake3, Algorithm::Sha256]);

    for (algorithm, digest) in &result {
        println!("{algorithm}: {}", digest.standard());
    }
}
