# HashJunkie — Plan 1: Workspace & Core Hash Library

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Cargo workspace and `hashjunkie-core` crate implementing all 13 hash algorithms with 100% test coverage, verified against known vectors.

**Architecture:** Single Rust library crate with a `Hasher` trait, per-algorithm implementations, and a `MultiHasher` that fans out bytes to all active hashers in one pass. No system library dependencies. WASM-compatible throughout.

**Tech Stack:** Rust stable, `RustCrypto/hashes`, `blake3`, `crc32fast`, `xxhash-rust`, `hex`, `cargo-llvm-cov`

---

## File Map

```
Cargo.toml                                  ← workspace root
.cargo/config.toml                          ← WASM target flags
crates/hashjunkie-core/
  Cargo.toml
  src/
    lib.rs                                  ← pub re-exports
    algorithm.rs                            ← Algorithm enum, Display, FromStr, UnknownAlgorithm
    multi.rs                                ← MultiHasher
    hashes/
      mod.rs                                ← Hasher trait
      rustcrypto.rs                         ← MD5, SHA1, SHA256, SHA512, Whirlpool
      blake3.rs
      crc32.rs
      xxhash.rs                             ← Xxh3Hasher + Xxh128Hasher
      dropbox.rs                            ← Custom: SHA256 of 4 MiB blocks
      hidrive.rs                            ← Custom: SHA1 per block + SHA1 of SHA1s
      mailru.rs                             ← Custom: chunked SHA1 variant
      quickxor.rs                           ← Custom: Microsoft QuickXOR
  tests/
    vectors.rs                              ← integration tests, known vectors, rclone fixture
    fixtures/
      small.bin                             ← 1 KiB all-zeros fixture
```

---

## Task 1: Initialize Cargo Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `.cargo/config.toml`
- Create: `crates/hashjunkie-core/Cargo.toml`
- Create: `crates/hashjunkie-core/src/lib.rs`

- [ ] **Step 1: Write failing test** — create the crate with an empty lib and a placeholder test that asserts the crate compiles

```bash
mkdir -p crates/hashjunkie-core/src
```

Create `crates/hashjunkie-core/src/lib.rs`:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
```

- [ ] **Step 2: Create workspace `Cargo.toml`**

```toml
[workspace]
members = ["crates/hashjunkie-core"]
resolver = "2"

[workspace.dependencies]
blake3      = "1"
crc32fast   = "1"
digest      = "0.10"
hex         = "0.4"
md-5        = "0.10"
sha1        = "0.10"
sha2        = "0.10"
whirlpool   = "0.10"
xxhash-rust = { version = "0.8", features = ["xxh3"] }

[profile.release]
opt-level     = "z"
lto           = true
codegen-units = 1
strip         = true
panic         = "abort"

[profile.wasm-release]
inherits  = "release"
opt-level = 3
```

- [ ] **Step 3: Create `crates/hashjunkie-core/Cargo.toml`**

```toml
[package]
name    = "hashjunkie-core"
version = "0.1.0"
edition = "2021"

[dependencies]
blake3      = { workspace = true }
crc32fast   = { workspace = true }
digest      = { workspace = true }
hex         = { workspace = true }
md-5        = { workspace = true }
sha1        = { workspace = true }
sha2        = { workspace = true }
whirlpool   = { workspace = true }
xxhash-rust = { workspace = true }
```

- [ ] **Step 4: Create `.cargo/config.toml`**

```toml
[target.wasm32-wasip1]
rustflags = ["-C", "target-feature=+simd128"]
```

- [ ] **Step 5: Run test to verify workspace compiles**

```bash
cargo test -p hashjunkie-core
```

Expected output contains: `test tests::crate_compiles ... ok`

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock .cargo/config.toml crates/
git commit -m "chore: initialize Cargo workspace with hashjunkie-core skeleton"
```

---

## Task 2: Hasher Trait + Algorithm Enum

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/mod.rs`
- Create: `crates/hashjunkie-core/src/algorithm.rs`
- Modify: `crates/hashjunkie-core/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add to `crates/hashjunkie-core/src/algorithm.rs` (create file):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn all_returns_13_algorithms() {
        assert_eq!(Algorithm::all().len(), 13);
    }

    #[test]
    fn display_roundtrips_via_from_str() {
        for alg in Algorithm::all() {
            let s = alg.to_string();
            let parsed = Algorithm::from_str(&s).unwrap();
            assert_eq!(*alg, parsed);
        }
    }

    #[test]
    fn unknown_algorithm_returns_error() {
        assert!(Algorithm::from_str("bogus").is_err());
    }

    #[test]
    fn as_str_matches_display() {
        for alg in Algorithm::all() {
            assert_eq!(alg.as_str(), alg.to_string());
        }
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p hashjunkie-core
```

Expected: `error[E0583]: file not found for module`

- [ ] **Step 3: Implement `algorithm.rs`**

```rust
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    Blake3,
    Crc32,
    Dropbox,
    Hidrive,
    Mailru,
    Md5,
    QuickXor,
    Sha1,
    Sha256,
    Sha512,
    Whirlpool,
    Xxh128,
    Xxh3,
}

impl Algorithm {
    pub fn all() -> &'static [Algorithm] {
        &[
            Algorithm::Blake3,
            Algorithm::Crc32,
            Algorithm::Dropbox,
            Algorithm::Hidrive,
            Algorithm::Mailru,
            Algorithm::Md5,
            Algorithm::QuickXor,
            Algorithm::Sha1,
            Algorithm::Sha256,
            Algorithm::Sha512,
            Algorithm::Whirlpool,
            Algorithm::Xxh128,
            Algorithm::Xxh3,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Blake3    => "blake3",
            Algorithm::Crc32     => "crc32",
            Algorithm::Dropbox   => "dropbox",
            Algorithm::Hidrive   => "hidrive",
            Algorithm::Mailru    => "mailru",
            Algorithm::Md5       => "md5",
            Algorithm::QuickXor  => "quickxor",
            Algorithm::Sha1      => "sha1",
            Algorithm::Sha256    => "sha256",
            Algorithm::Sha512    => "sha512",
            Algorithm::Whirlpool => "whirlpool",
            Algorithm::Xxh128    => "xxh128",
            Algorithm::Xxh3      => "xxh3",
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug)]
pub struct UnknownAlgorithm(pub String);

impl fmt::Display for UnknownAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown algorithm: {}", self.0)
    }
}

impl std::error::Error for UnknownAlgorithm {}

impl std::str::FromStr for Algorithm {
    type Err = UnknownAlgorithm;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blake3"    => Ok(Algorithm::Blake3),
            "crc32"     => Ok(Algorithm::Crc32),
            "dropbox"   => Ok(Algorithm::Dropbox),
            "hidrive"   => Ok(Algorithm::Hidrive),
            "mailru"    => Ok(Algorithm::Mailru),
            "md5"       => Ok(Algorithm::Md5),
            "quickxor"  => Ok(Algorithm::QuickXor),
            "sha1"      => Ok(Algorithm::Sha1),
            "sha256"    => Ok(Algorithm::Sha256),
            "sha512"    => Ok(Algorithm::Sha512),
            "whirlpool" => Ok(Algorithm::Whirlpool),
            "xxh128"    => Ok(Algorithm::Xxh128),
            "xxh3"      => Ok(Algorithm::Xxh3),
            other       => Err(UnknownAlgorithm(other.to_string())),
        }
    }
}
```

- [ ] **Step 4: Implement `hashes/mod.rs`** (the `Hasher` trait)

```rust
pub trait Hasher: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize_hex(self: Box<Self>) -> String;
}
```

- [ ] **Step 5: Update `lib.rs`**

```rust
mod algorithm;
mod hashes;

pub use algorithm::{Algorithm, UnknownAlgorithm};
```

- [ ] **Step 6: Run tests to confirm they pass**

```bash
cargo test -p hashjunkie-core
```

Expected: all tests pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/
git commit -m "feat: add Algorithm enum and Hasher trait"
```

---

## Task 3: RustCrypto Algorithms (MD5, SHA1, SHA256, SHA512, Whirlpool)

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/rustcrypto.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

- [ ] **Step 1: Write failing tests**

Add to `crates/hashjunkie-core/src/hashes/rustcrypto.rs` (create file):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash_empty<H: RustCryptoHashable>() -> String {
        let mut h = RustCryptoHasher::<H>::new();
        h.update(b"");
        Box::new(h).finalize_hex()
    }

    fn hash_bytes<H: RustCryptoHashable>(data: &[u8]) -> String {
        let mut h = RustCryptoHasher::<H>::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // MD5 vectors: RFC 1321
    #[test] fn md5_empty() {
        assert_eq!(hash_empty::<md5::Md5>(), "d41d8cd98f00b204e9800998ecf8427e");
    }
    #[test] fn md5_abc() {
        assert_eq!(hash_bytes::<md5::Md5>(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    // SHA1 vectors: FIPS 180-4
    #[test] fn sha1_empty() {
        assert_eq!(hash_empty::<sha1::Sha1>(), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }
    #[test] fn sha1_abc() {
        assert_eq!(hash_bytes::<sha1::Sha1>(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    // SHA256 vectors: FIPS 180-4
    #[test] fn sha256_empty() {
        assert_eq!(hash_empty::<sha2::Sha256>(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }
    #[test] fn sha256_abc() {
        assert_eq!(hash_bytes::<sha2::Sha256>(b"abc"), "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469460b1edfd9c4d3d4e");
    }

    // SHA512 vectors: FIPS 180-4
    #[test] fn sha512_empty() {
        assert_eq!(hash_empty::<sha2::Sha512>(), "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");
    }
    #[test] fn sha512_abc() {
        assert_eq!(hash_bytes::<sha2::Sha512>(b"abc"), "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
    }

    // Whirlpool vectors: official test suite
    #[test] fn whirlpool_empty() {
        assert_eq!(hash_empty::<whirlpool::Whirlpool>(), "19fa61d75522a4669b44e39c1d2e1726c530232130d407f89afee0964997f7a73e83be698b288febcf88e3e03c4f0757ea8964e59b63d93708b138cc42a66eb3");
    }
    #[test] fn whirlpool_abc() {
        assert_eq!(hash_bytes::<whirlpool::Whirlpool>(b"abc"), "4e2448a4c6f486bb16b6562c73b4020bf3043e3a731bce721ae1b303d97e6d4a7597166f944e60ac18af7ecdaa0b59cef9e7f6a66fbe5c2ba83cd6c37b21be0f");
    }

    #[test]
    fn update_in_chunks_matches_single_update() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut h1 = RustCryptoHasher::<sha2::Sha256>::new();
        h1.update(data);
        let single = Box::new(h1).finalize_hex();

        let mut h2 = RustCryptoHasher::<sha2::Sha256>::new();
        for chunk in data.chunks(7) {
            h2.update(chunk);
        }
        let chunked = Box::new(h2).finalize_hex();

        assert_eq!(single, chunked);
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p hashjunkie-core hashes::rustcrypto
```

Expected: `error[E0433]: failed to resolve: use of undeclared crate or module`

- [ ] **Step 3: Implement `rustcrypto.rs`**

```rust
use digest::Digest;
use crate::hashes::Hasher;

pub trait RustCryptoHashable: Digest + Default + Send + 'static {}
impl<T: Digest + Default + Send + 'static> RustCryptoHashable for T {}

pub struct RustCryptoHasher<D: RustCryptoHashable> {
    inner: D,
}

impl<D: RustCryptoHashable> RustCryptoHasher<D> {
    pub fn new() -> Self {
        Self { inner: D::default() }
    }
}

impl<D: RustCryptoHashable> Hasher for RustCryptoHasher<D> {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        hex::encode(self.inner.finalize())
    }
}

#[cfg(test)]
mod tests { /* see above */ }
```

- [ ] **Step 4: Re-export from `hashes/mod.rs`**

Add to `crates/hashjunkie-core/src/hashes/mod.rs`:
```rust
mod rustcrypto;
pub use rustcrypto::RustCryptoHasher;
```

- [ ] **Step 5: Run tests to confirm they pass**

```bash
cargo test -p hashjunkie-core hashes::rustcrypto
```

Expected: 11 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement RustCrypto hashers (MD5, SHA1, SHA256, SHA512, Whirlpool)"
```

---

## Task 4: BLAKE3

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/blake3.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/hashjunkie-core/src/hashes/blake3.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = Blake3Hasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // Official BLAKE3 test vectors from github.com/BLAKE3-team/BLAKE3
    #[test] fn blake3_empty() {
        assert_eq!(hash(b""), "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");
    }
    #[test] fn blake3_abc() {
        assert_eq!(hash(b"abc"), "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85");
    }
    #[test]
    fn chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let single = hash(data);
        let mut h = Blake3Hasher::new();
        for chunk in data.chunks(5) { h.update(chunk); }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p hashjunkie-core hashes::blake3
```

Expected: `error[E0433]: failed to resolve`

- [ ] **Step 3: Implement**

```rust
use crate::hashes::Hasher;

pub struct Blake3Hasher {
    inner: blake3::Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self { inner: blake3::Hasher::new() }
    }
}

impl Hasher for Blake3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        hex::encode(self.inner.finalize().as_bytes())
    }
}
```

- [ ] **Step 4: Re-export from `hashes/mod.rs`**

```rust
mod blake3;
pub use blake3::Blake3Hasher;
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p hashjunkie-core hashes::blake3
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement BLAKE3 hasher"
```

---

## Task 5: CRC32

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/crc32.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/hashjunkie-core/src/hashes/crc32.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = Crc32Hasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // IEEE 802.3 CRC32 vectors
    #[test] fn crc32_empty()    { assert_eq!(hash(b""),    "00000000"); }
    #[test] fn crc32_abc()      { assert_eq!(hash(b"abc"), "352441c2"); }
    #[test] fn crc32_123456789() {
        assert_eq!(hash(b"123456789"), "cbf43926");
    }
    #[test]
    fn chunked_matches_single() {
        let data = b"the quick brown fox";
        let single = hash(data);
        let mut h = Crc32Hasher::new();
        for chunk in data.chunks(3) { h.update(chunk); }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p hashjunkie-core hashes::crc32
```

- [ ] **Step 3: Implement**

```rust
use crate::hashes::Hasher;

pub struct Crc32Hasher {
    inner: crc32fast::Hasher,
}

impl Crc32Hasher {
    pub fn new() -> Self {
        Self { inner: crc32fast::Hasher::new() }
    }
}

impl Hasher for Crc32Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize_hex(self: Box<Self>) -> String {
        format!("{:08x}", self.inner.finalize())
    }
}
```

- [ ] **Step 4: Re-export**

```rust
mod crc32;
pub use crc32::Crc32Hasher;
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p hashjunkie-core hashes::crc32
```

Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement CRC32 hasher"
```

---

## Task 6: XXH3 and XXH128

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/xxhash.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/hashjunkie-core/src/hashes/xxhash.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    // Official xxHash test vectors from github.com/Cyan4973/xxHash
    #[test] fn xxh3_empty() {
        let mut h = Xxh3Hasher::new();
        h.update(b"");
        assert_eq!(Box::new(h).finalize_hex(), "2d06800538d394c2");
    }
    #[test] fn xxh128_empty() {
        let mut h = Xxh128Hasher::new();
        h.update(b"");
        assert_eq!(Box::new(h).finalize_hex(), "99aa06d3014798d86001c324468d497f");
    }
    #[test]
    fn xxh3_chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut h1 = Xxh3Hasher::new();
        h1.update(data);
        let single = Box::new(h1).finalize_hex();

        let mut h2 = Xxh3Hasher::new();
        for chunk in data.chunks(9) { h2.update(chunk); }
        assert_eq!(Box::new(h2).finalize_hex(), single);
    }
    #[test]
    fn xxh128_chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut h1 = Xxh128Hasher::new();
        h1.update(data);
        let single = Box::new(h1).finalize_hex();

        let mut h2 = Xxh128Hasher::new();
        for chunk in data.chunks(9) { h2.update(chunk); }
        assert_eq!(Box::new(h2).finalize_hex(), single);
    }
    #[test]
    fn xxh3_output_is_16_hex_chars() {
        let mut h = Xxh3Hasher::new();
        h.update(b"test");
        assert_eq!(Box::new(h).finalize_hex().len(), 16);
    }
    #[test]
    fn xxh128_output_is_32_hex_chars() {
        let mut h = Xxh128Hasher::new();
        h.update(b"test");
        assert_eq!(Box::new(h).finalize_hex().len(), 32);
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p hashjunkie-core hashes::xxhash
```

- [ ] **Step 3: Implement**

```rust
use crate::hashes::Hasher;
use xxhash_rust::xxh3::Xxh3;

pub struct Xxh3Hasher { inner: Xxh3 }
pub struct Xxh128Hasher { inner: Xxh3 }

impl Xxh3Hasher   { pub fn new() -> Self { Self { inner: Xxh3::new() } } }
impl Xxh128Hasher { pub fn new() -> Self { Self { inner: Xxh3::new() } } }

impl Hasher for Xxh3Hasher {
    fn update(&mut self, data: &[u8]) { self.inner.update(data); }
    fn finalize_hex(self: Box<Self>) -> String {
        format!("{:016x}", self.inner.digest())
    }
}

impl Hasher for Xxh128Hasher {
    fn update(&mut self, data: &[u8]) { self.inner.update(data); }
    fn finalize_hex(self: Box<Self>) -> String {
        format!("{:032x}", self.inner.digest128())
    }
}
```

- [ ] **Step 4: Re-export**

```rust
mod xxhash;
pub use xxhash::{Xxh3Hasher, Xxh128Hasher};
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p hashjunkie-core hashes::xxhash
```

Expected: 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement XXH3 and XXH128 hashers"
```

---

## Task 7: Dropbox Hash

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/dropbox.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

The Dropbox content hash splits a file into 4 MiB blocks, SHA256s each block, then SHA256s the concatenation of those block hashes. Spec: https://www.dropbox.com/developers/reference/content-hash

- [ ] **Step 1: Write failing tests**

Create `crates/hashjunkie-core/src/hashes/dropbox.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = DropboxHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // Empty file: SHA256(SHA256(""))
    // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    // SHA256(above_as_raw_bytes) = 5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456
    #[test] fn dropbox_empty() {
        assert_eq!(hash(b""), "5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456");
    }

    // Single-block file (< 4 MiB): SHA256(SHA256(content))
    #[test] fn dropbox_abc() {
        // SHA256("abc") = ba7816bf...
        // SHA256(raw_bytes_of_that) = pre-computed below
        use sha2::{Sha256, Digest};
        let inner = Sha256::digest(b"abc");
        let expected = hex::encode(Sha256::digest(&inner));
        assert_eq!(hash(b"abc"), expected);
    }

    #[test]
    fn chunked_update_matches_single() {
        let data = vec![0xABu8; 1024];
        let single = hash(&data);
        let mut h = DropboxHasher::new();
        for chunk in data.chunks(100) { h.update(chunk); }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn block_boundary_produces_two_blocks() {
        // Feed exactly BLOCK_SIZE bytes, then 1 more byte
        // Result must differ from a single-block hash of the same data
        let full_block = vec![0u8; BLOCK_SIZE];
        let extra_byte = vec![1u8; 1];

        let mut h = DropboxHasher::new();
        h.update(&full_block);
        h.update(&extra_byte);
        let two_blocks = Box::new(h).finalize_hex();

        let mut combined = full_block.clone();
        combined.extend_from_slice(&extra_byte);
        // single-block hash of the same bytes would be SHA256(SHA256(combined))
        use sha2::{Sha256, Digest};
        let inner = Sha256::digest(&combined);
        let single_block = hex::encode(Sha256::digest(&inner));

        assert_ne!(two_blocks, single_block, "two-block hash must differ from single-block hash");
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test -p hashjunkie-core hashes::dropbox
```

- [ ] **Step 3: Implement**

```rust
use sha2::{Sha256, Digest};
use crate::hashes::Hasher;

pub const BLOCK_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

pub struct DropboxHasher {
    block_hashes: Vec<[u8; 32]>,
    current_block: Sha256,
    current_block_len: usize,
}

impl DropboxHasher {
    pub fn new() -> Self {
        Self {
            block_hashes: Vec::new(),
            current_block: Sha256::new(),
            current_block_len: 0,
        }
    }
}

impl Hasher for DropboxHasher {
    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let remaining = BLOCK_SIZE - self.current_block_len;
            let take = data.len().min(remaining);
            self.current_block.update(&data[..take]);
            self.current_block_len += take;
            data = &data[take..];

            if self.current_block_len == BLOCK_SIZE {
                let finished = std::mem::replace(&mut self.current_block, Sha256::new());
                self.block_hashes.push(finished.finalize().into());
                self.current_block_len = 0;
            }
        }
    }

    fn finalize_hex(self: Box<Self>) -> String {
        let Self { mut block_hashes, current_block, current_block_len } = *self;

        if current_block_len > 0 || block_hashes.is_empty() {
            block_hashes.push(current_block.finalize().into());
        }

        let mut outer = Sha256::new();
        for h in &block_hashes {
            outer.update(h);
        }
        hex::encode(outer.finalize())
    }
}
```

- [ ] **Step 4: Re-export**

```rust
mod dropbox;
pub use dropbox::DropboxHasher;
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p hashjunkie-core hashes::dropbox
```

Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement Dropbox content hash"
```

---

## Task 8: HiDrive Hash

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/hidrive.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

The HiDrive hash splits the file into 128 KiB blocks, SHA1s each block, then SHA1s the concatenation of those block SHA1s. Source: rclone `backend/hidrive/hidrive.go` — verify `hidriveBlockSize` constant before running tests.

- [ ] **Step 1: Read rclone source to confirm block size**

```bash
curl -s https://raw.githubusercontent.com/rclone/rclone/master/backend/hidrive/hidrive.go | grep -i "block\|chunk\|size" | head -20
```

Confirm `hidriveBlockSize` or equivalent. Update `BLOCK_SIZE` in the implementation if it differs from 131072.

- [ ] **Step 2: Write failing tests**

Create `crates/hashjunkie-core/src/hashes/hidrive.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = HidriveHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    // Empty: SHA1(SHA1(""))
    // SHA1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
    // SHA1(raw bytes of above) = pre-computed
    #[test] fn hidrive_empty() {
        use sha1::{Sha1, Digest};
        let inner = Sha1::digest(b"");
        let expected = hex::encode(Sha1::digest(&inner));
        assert_eq!(hash(b""), expected);
    }

    #[test]
    fn chunked_matches_single() {
        let data = vec![0x42u8; 1024];
        let single = hash(&data);
        let mut h = HidriveHasher::new();
        for chunk in data.chunks(100) { h.update(chunk); }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn block_boundary_differs_from_single_block() {
        let full_block = vec![0u8; BLOCK_SIZE];
        let mut h = HidriveHasher::new();
        h.update(&full_block);
        h.update(&[1u8]);
        let two_block = Box::new(h).finalize_hex();

        let mut combined = full_block;
        combined.push(1u8);
        assert_ne!(hash(&combined), two_block == hash(&combined) || true,
            "boundary test: two-block must differ from same data as one block");
        // Simpler: just verify the two paths produce different results
        let one_block = hash(&combined);
        assert_ne!(two_block, one_block);
    }
}
```

- [ ] **Step 3: Run to confirm failure**

```bash
cargo test -p hashjunkie-core hashes::hidrive
```

- [ ] **Step 4: Implement**

```rust
use sha1::{Sha1, Digest};
use crate::hashes::Hasher;

pub const BLOCK_SIZE: usize = 128 * 1024; // 128 KiB — verify against rclone source

pub struct HidriveHasher {
    block_hashes: Vec<[u8; 20]>,
    current_block: Sha1,
    current_block_len: usize,
}

impl HidriveHasher {
    pub fn new() -> Self {
        Self {
            block_hashes: Vec::new(),
            current_block: Sha1::new(),
            current_block_len: 0,
        }
    }
}

impl Hasher for HidriveHasher {
    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let remaining = BLOCK_SIZE - self.current_block_len;
            let take = data.len().min(remaining);
            self.current_block.update(&data[..take]);
            self.current_block_len += take;
            data = &data[take..];

            if self.current_block_len == BLOCK_SIZE {
                let finished = std::mem::replace(&mut self.current_block, Sha1::new());
                self.block_hashes.push(finished.finalize().into());
                self.current_block_len = 0;
            }
        }
    }

    fn finalize_hex(self: Box<Self>) -> String {
        let Self { mut block_hashes, current_block, current_block_len } = *self;

        if current_block_len > 0 || block_hashes.is_empty() {
            block_hashes.push(current_block.finalize().into());
        }

        let mut outer = Sha1::new();
        for h in &block_hashes {
            outer.update(h);
        }
        hex::encode(outer.finalize())
    }
}
```

- [ ] **Step 5: Re-export**

```rust
mod hidrive;
pub use hidrive::HidriveHasher;
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p hashjunkie-core hashes::hidrive
```

- [ ] **Step 7: Commit**

```bash
git add crates/
git commit -m "feat: implement HiDrive hash"
```

---

## Task 9: Mail.ru Hash

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/mailru.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

Mail.ru uses a SHA1-based chunked hash. Read the rclone source to confirm the exact algorithm and block size before writing tests.

- [ ] **Step 1: Read rclone source to confirm the algorithm**

```bash
curl -s https://raw.githubusercontent.com/rclone/rclone/master/backend/mailru/features.go | grep -A 30 -i "hash\|chunk\|block"
```

Also check:
```bash
curl -s https://raw.githubusercontent.com/rclone/rclone/master/backend/mailru/mailru.go | grep -A 20 "hash\|Hash"
```

Note the block size and whether it differs by file size threshold. Update the implementation accordingly.

- [ ] **Step 2: Write failing tests** (adjust expected values after rclone source research)

Create `crates/hashjunkie-core/src/hashes/mailru.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = MailruHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn mailru_empty() {
        // Update expected value after verifying against rclone:
        // echo -n "" | rclone hashsum mailru /dev/stdin
        // Or use: hashjunkie-cli (once built) on empty input
        let result = hash(b"");
        assert_eq!(result.len(), 40, "Mail.ru hash must be 40 hex chars (SHA1 length)");
        // TODO: replace with exact expected value from rclone verification
    }

    #[test]
    fn chunked_matches_single() {
        let data = vec![0xBBu8; 4096];
        let single = hash(&data);
        let mut h = MailruHasher::new();
        for chunk in data.chunks(100) { h.update(chunk); }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }
}
```

- [ ] **Step 3: Implement** (based on rclone source findings from Step 1)

```rust
use sha1::{Sha1, Digest};
use crate::hashes::Hasher;

// Verify this block size against rclone source before committing.
// rclone mailru uses 1 MiB blocks for the chunked hash.
pub const BLOCK_SIZE: usize = 1024 * 1024; // 1 MiB — verify

pub struct MailruHasher {
    block_hashes: Vec<[u8; 20]>,
    current_block: Sha1,
    current_block_len: usize,
}

impl MailruHasher {
    pub fn new() -> Self {
        Self {
            block_hashes: Vec::new(),
            current_block: Sha1::new(),
            current_block_len: 0,
        }
    }
}

impl Hasher for MailruHasher {
    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let remaining = BLOCK_SIZE - self.current_block_len;
            let take = data.len().min(remaining);
            self.current_block.update(&data[..take]);
            self.current_block_len += take;
            data = &data[take..];

            if self.current_block_len == BLOCK_SIZE {
                let finished = std::mem::replace(&mut self.current_block, Sha1::new());
                self.block_hashes.push(finished.finalize().into());
                self.current_block_len = 0;
            }
        }
    }

    fn finalize_hex(self: Box<Self>) -> String {
        let Self { mut block_hashes, current_block, current_block_len } = *self;

        if current_block_len > 0 || block_hashes.is_empty() {
            block_hashes.push(current_block.finalize().into());
        }

        let mut outer = Sha1::new();
        for h in &block_hashes {
            outer.update(h);
        }
        hex::encode(outer.finalize())
    }
}
```

- [ ] **Step 4: Verify exact expected values against rclone**

```bash
# On any system with rclone installed:
echo -n "" | rclone hashsum mailru -
printf "abc" | rclone hashsum mailru -
```

Update the `mailru_empty` test with the exact expected string.

- [ ] **Step 5: Re-export + run tests**

Add to `hashes/mod.rs`:
```rust
mod mailru;
pub use mailru::MailruHasher;
```

```bash
cargo test -p hashjunkie-core hashes::mailru
```

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement Mail.ru hash"
```

---

## Task 10: QuickXor Hash

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/quickxor.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`

Microsoft's QuickXOR algorithm. Spec: MS-FSSHTTP §2.3.1. Width = 160 bits, shift = 11 bits per byte. Output: 20-byte XOR state with length XORed in at the end.

- [ ] **Step 1: Write failing tests**

Create `crates/hashjunkie-core/src/hashes/quickxor.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = QuickXorHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn quickxor_empty() {
        // Empty input: state stays all-zeros, length=0 XORed in → still all-zeros
        assert_eq!(hash(b""), "0000000000000000000000000000000000000000");
    }

    #[test]
    fn quickxor_output_is_40_hex_chars() {
        assert_eq!(hash(b"test").len(), 40);
    }

    #[test]
    fn chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let single = hash(data);
        let mut h = QuickXorHasher::new();
        for chunk in data.chunks(7) { h.update(chunk); }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    // Verify this vector against rclone or OneDrive SDK before committing:
    // printf "abc" | rclone hashsum quickxor -
    #[test]
    fn quickxor_abc() {
        // Replace with verified expected value
        let result = hash(b"abc");
        assert_eq!(result.len(), 40, "output must be 40 hex chars");
        // TODO: assert_eq!(result, "<verified value>");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test -p hashjunkie-core hashes::quickxor
```

- [ ] **Step 3: Implement**

```rust
use crate::hashes::Hasher;

const WIDTH_BITS: u64 = 160;
const SHIFT: u64 = 11;

pub struct QuickXorHasher {
    state: [u8; 20],
    length: u64,
    bit_offset: u64,
}

impl QuickXorHasher {
    pub fn new() -> Self {
        Self { state: [0u8; 20], length: 0, bit_offset: 0 }
    }
}

impl Hasher for QuickXorHasher {
    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            let start_bit  = self.bit_offset % WIDTH_BITS;
            let byte_index = (start_bit / 8) as usize;
            let bit_in_byte = (start_bit % 8) as u8;

            self.state[byte_index % 20] ^= byte << bit_in_byte;
            if bit_in_byte > 0 {
                self.state[(byte_index + 1) % 20] ^= byte >> (8 - bit_in_byte);
            }

            self.bit_offset += SHIFT;
        }
        self.length += data.len() as u64;
    }

    fn finalize_hex(mut self: Box<Self>) -> String {
        // XOR the 8-byte little-endian length into the state starting at byte 0
        for (i, b) in self.length.to_le_bytes().iter().enumerate() {
            self.state[i] ^= b;
        }
        hex::encode(self.state)
    }
}
```

- [ ] **Step 4: Verify `quickxor_abc` expected value against rclone**

```bash
printf "abc" | rclone hashsum quickxor -
```

Update the `quickxor_abc` test with the exact expected string and remove the TODO comment.

- [ ] **Step 5: Re-export + run tests**

```rust
mod quickxor;
pub use quickxor::QuickXorHasher;
```

```bash
cargo test -p hashjunkie-core hashes::quickxor
```

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement QuickXOR hash (Microsoft MS-FSSHTTP spec)"
```

---

## Task 11: MultiHasher

**Files:**
- Create: `crates/hashjunkie-core/src/multi.rs`
- Modify: `crates/hashjunkie-core/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/hashjunkie-core/src/multi.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Algorithm;
    use std::collections::HashMap;

    #[test]
    fn new_with_subset_produces_only_requested_algorithms() {
        let algs = &[Algorithm::Md5, Algorithm::Sha256];
        let mut h = MultiHasher::new(algs);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(digests.len(), 2);
        assert!(digests.contains_key(&Algorithm::Md5));
        assert!(digests.contains_key(&Algorithm::Sha256));
    }

    #[test]
    fn all_produces_all_13_algorithms() {
        let mut h = MultiHasher::all();
        h.update(b"");
        let digests = h.finalize();
        assert_eq!(digests.len(), 13);
    }

    #[test]
    fn md5_result_matches_standalone_hasher() {
        let mut h = MultiHasher::new(&[Algorithm::Md5]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(digests[&Algorithm::Md5], "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn sha256_result_matches_standalone_hasher() {
        let mut h = MultiHasher::new(&[Algorithm::Sha256]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(digests[&Algorithm::Sha256], "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469460b1edfd9c4d3d4e");
    }

    #[test]
    fn multi_update_in_chunks_matches_single_update() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let algs = &[Algorithm::Blake3, Algorithm::Sha256, Algorithm::Xxh3];

        let mut h1 = MultiHasher::new(algs);
        h1.update(data);
        let single = h1.finalize();

        let mut h2 = MultiHasher::new(algs);
        for chunk in data.chunks(11) { h2.update(chunk); }
        let chunked = h2.finalize();

        assert_eq!(single, chunked);
    }

    #[test]
    fn empty_algorithms_slice_produces_empty_result() {
        let mut h = MultiHasher::new(&[]);
        h.update(b"data");
        assert_eq!(h.finalize().len(), 0);
    }

    #[test]
    fn duplicate_algorithms_are_deduplicated() {
        let algs = &[Algorithm::Md5, Algorithm::Md5, Algorithm::Md5];
        let mut h = MultiHasher::new(algs);
        h.update(b"abc");
        // Dedup: only one MD5 result
        assert_eq!(h.finalize().len(), 1);
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test -p hashjunkie-core multi
```

- [ ] **Step 3: Implement**

```rust
use std::collections::HashMap;
use crate::Algorithm;
use crate::hashes::{self, Hasher};

pub struct MultiHasher {
    pairs: Vec<(Algorithm, Box<dyn Hasher>)>,
}

impl MultiHasher {
    pub fn new(algorithms: &[Algorithm]) -> Self {
        // Deduplicate while preserving first-seen order
        let mut seen = std::collections::HashSet::new();
        let pairs = algorithms
            .iter()
            .filter(|&&alg| seen.insert(alg))
            .map(|&alg| (alg, make_hasher(alg)))
            .collect();
        Self { pairs }
    }

    pub fn all() -> Self {
        Self::new(Algorithm::all())
    }

    pub fn update(&mut self, data: &[u8]) {
        for (_, hasher) in &mut self.pairs {
            hasher.update(data);
        }
    }

    pub fn finalize(self) -> HashMap<Algorithm, String> {
        self.pairs
            .into_iter()
            .map(|(alg, hasher)| (alg, hasher.finalize_hex()))
            .collect()
    }
}

fn make_hasher(alg: Algorithm) -> Box<dyn Hasher> {
    use hashes::*;
    match alg {
        Algorithm::Blake3    => Box::new(Blake3Hasher::new()),
        Algorithm::Crc32     => Box::new(Crc32Hasher::new()),
        Algorithm::Dropbox   => Box::new(DropboxHasher::new()),
        Algorithm::Hidrive   => Box::new(HidriveHasher::new()),
        Algorithm::Mailru    => Box::new(MailruHasher::new()),
        Algorithm::Md5       => Box::new(RustCryptoHasher::<md5::Md5>::new()),
        Algorithm::QuickXor  => Box::new(QuickXorHasher::new()),
        Algorithm::Sha1      => Box::new(RustCryptoHasher::<sha1::Sha1>::new()),
        Algorithm::Sha256    => Box::new(RustCryptoHasher::<sha2::Sha256>::new()),
        Algorithm::Sha512    => Box::new(RustCryptoHasher::<sha2::Sha512>::new()),
        Algorithm::Whirlpool => Box::new(RustCryptoHasher::<whirlpool::Whirlpool>::new()),
        Algorithm::Xxh128    => Box::new(Xxh128Hasher::new()),
        Algorithm::Xxh3      => Box::new(Xxh3Hasher::new()),
    }
}
```

- [ ] **Step 4: Update `lib.rs`**

```rust
mod algorithm;
mod hashes;
mod multi;

pub use algorithm::{Algorithm, UnknownAlgorithm};
pub use multi::MultiHasher;
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p hashjunkie-core multi
```

Expected: 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/
git commit -m "feat: implement MultiHasher with deduplication and fan-out"
```

---

## Task 12: Integration Test + Rclone Fixture Verification

**Files:**
- Create: `crates/hashjunkie-core/tests/vectors.rs`
- Create: `crates/hashjunkie-core/tests/fixtures/small.bin`

This task cross-verifies all 13 algorithms against rclone output on a known fixture file.

- [ ] **Step 1: Create the fixture file**

```bash
mkdir -p crates/hashjunkie-core/tests/fixtures
# 1 KiB of incrementing bytes (0x00..0xFF repeated)
python3 -c "import sys; sys.stdout.buffer.write(bytes(i % 256 for i in range(1024)))" \
  > crates/hashjunkie-core/tests/fixtures/small.bin
```

- [ ] **Step 2: Get rclone ground-truth for the fixture**

```bash
rclone hashsum all crates/hashjunkie-core/tests/fixtures/small.bin
# Or if rclone lsjson is preferred:
rclone lsjson crates/hashjunkie-core/tests/fixtures/small.bin --hash | jq '.[0].Hashes'
```

Record every hash value from the output — you will paste them into the test below.

- [ ] **Step 3: Write failing integration test**

Create `crates/hashjunkie-core/tests/vectors.rs`:
```rust
use hashjunkie_core::{Algorithm, MultiHasher};
use std::collections::HashMap;

fn hash_fixture(path: &str) -> HashMap<Algorithm, String> {
    let data = std::fs::read(path).expect("fixture file must exist");
    let mut h = MultiHasher::all();
    h.update(&data);
    h.finalize()
}

/// Ground truth produced by: rclone lsjson tests/fixtures/small.bin --hash
/// Replace each value below with the actual rclone output.
const EXPECTED: &[(&str, &str)] = &[
    ("blake3",    "REPLACE_WITH_RCLONE_OUTPUT"),
    ("crc32",     "REPLACE_WITH_RCLONE_OUTPUT"),
    ("dropbox",   "REPLACE_WITH_RCLONE_OUTPUT"),
    ("hidrive",   "REPLACE_WITH_RCLONE_OUTPUT"),
    ("mailru",    "REPLACE_WITH_RCLONE_OUTPUT"),
    ("md5",       "REPLACE_WITH_RCLONE_OUTPUT"),
    ("quickxor",  "REPLACE_WITH_RCLONE_OUTPUT"),
    ("sha1",      "REPLACE_WITH_RCLONE_OUTPUT"),
    ("sha256",    "REPLACE_WITH_RCLONE_OUTPUT"),
    ("sha512",    "REPLACE_WITH_RCLONE_OUTPUT"),
    ("whirlpool", "REPLACE_WITH_RCLONE_OUTPUT"),
    ("xxh128",    "REPLACE_WITH_RCLONE_OUTPUT"),
    ("xxh3",      "REPLACE_WITH_RCLONE_OUTPUT"),
];

#[test]
fn all_algorithms_match_rclone_on_fixture() {
    let digests = hash_fixture("tests/fixtures/small.bin");
    for (name, expected) in EXPECTED {
        let alg: Algorithm = name.parse().unwrap();
        let got = digests.get(&alg)
            .unwrap_or_else(|| panic!("missing algorithm: {name}"));
        assert_eq!(got, expected, "mismatch for {name}");
    }
}
```

- [ ] **Step 4: Replace REPLACE_WITH_RCLONE_OUTPUT with actual values from Step 2**

- [ ] **Step 5: Run integration test**

```bash
cargo test -p hashjunkie-core --test vectors
```

Expected: 1 test passes. If any algorithm fails, the mismatch message names which one — debug that algorithm's implementation.

- [ ] **Step 6: Commit fixture and test**

```bash
git add crates/hashjunkie-core/tests/
git commit -m "test: add rclone-verified integration test for all 13 algorithms"
```

---

## Task 13: Coverage Gate

- [ ] **Step 1: Install `cargo-llvm-cov` if not present**

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

- [ ] **Step 2: Run coverage**

```bash
cargo llvm-cov --package hashjunkie-core --branch --summary-only
```

- [ ] **Step 3: Confirm 100% line and branch coverage**

Expected output ends with a line like:
```
TOTAL   100.00%  100.00%
```

If any line or branch is uncovered, write a test that exercises it. Common gaps: error paths in `Algorithm::from_str`, the `block_boundary` branches in Dropbox/HiDrive/Mailru, the `bit_in_byte == 0` branch in QuickXor.

- [ ] **Step 4: Final commit**

```bash
git add .
git commit -m "test: confirm 100% line and branch coverage for hashjunkie-core"
```

---

## Self-Review Notes

- **Spec coverage:** All 13 algorithms implemented. MultiHasher with fan-out. rclone fixture verification. ✓
- **Placeholder check:** Tasks 9 and 10 have TODOs for exact expected values — these are resolved by running rclone in Step 4 of each task, not deferred. ✓
- **Type consistency:** `Hasher::finalize_hex(self: Box<Self>)` used consistently throughout. `MultiHasher::finalize()` returns `HashMap<Algorithm, String>`. ✓
- **Dedup test:** Task 11 includes a test for duplicate algorithm deduplication. ✓
