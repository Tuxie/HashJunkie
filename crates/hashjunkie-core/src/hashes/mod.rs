pub trait Hasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize_hex(self: Box<Self>) -> String;
}

mod rustcrypto;
pub use rustcrypto::RustCryptoHasher;

mod blake3;
pub use blake3::Blake3Hasher;

mod crc32;
pub use crc32::Crc32Hasher;

mod ipfs_cid;
pub use ipfs_cid::CidHasher;

mod xxhash;
pub use xxhash::{Xxh128Hasher, Xxh3Hasher};

mod dropbox;
pub use dropbox::DropboxHasher;

mod hidrive;
pub use hidrive::HidriveHasher;

mod quickxor;
pub use quickxor::QuickXorHasher;

mod mailru;
pub use mailru::MailruHasher;
