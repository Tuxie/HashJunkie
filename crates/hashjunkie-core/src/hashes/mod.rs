use crate::DigestValue;

pub trait Hasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize_hex(self: Box<Self>) -> String;

    fn finalize_digest(self: Box<Self>) -> DigestValue {
        DigestValue::from_hex(self.finalize_hex()).expect("hex hasher returned valid lowercase hex")
    }
}

mod rustcrypto;
pub use rustcrypto::RustCryptoHasher;

mod aich;
pub use aich::AichHasher;

mod blake3;
pub use blake3::Blake3Hasher;

mod btv2;
pub use btv2::Btv2Hasher;

mod crc32;
pub use crc32::Crc32Hasher;

mod ipfs_cid;
pub use ipfs_cid::CidHasher;
#[cfg(feature = "profile-ipfs-cid")]
pub use ipfs_cid::{CidProfile, reset_profile, take_profile};

mod xxhash;
pub use xxhash::{Xxh3Hasher, Xxh128Hasher};

mod dropbox;
pub use dropbox::DropboxHasher;

mod ed2k;
pub use ed2k::Ed2kHasher;

mod hidrive;
pub use hidrive::HidriveHasher;

mod quickxor;
pub use quickxor::QuickXorHasher;

mod mailru;
pub use mailru::MailruHasher;

mod tiger_tree;
pub use tiger_tree::TigerTreeHasher;
