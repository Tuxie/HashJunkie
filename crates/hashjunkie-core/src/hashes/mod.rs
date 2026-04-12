pub trait Hasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize_hex(self: Box<Self>) -> String;
}

mod rustcrypto;
pub use rustcrypto::RustCryptoHasher;

mod blake3;
pub use blake3::Blake3Hasher;
