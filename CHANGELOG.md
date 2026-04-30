# Changelog

## Unreleased

### Added

- Added CLI `--hex` to display the raw digest bytes as lowercase hex, even for
  algorithms whose standard text form is Base32, Base58, or CID text.
- Added `HashJunkie.hexdigests` and `HashJunkie.rawdigests` promises for the
  Node/Bun API while keeping `HashJunkie.digests` as the standard visual
  representation.
- Added first-class Rust library helpers: `hash_bytes()`, `hash_reader()`,
  `hash_file()`, default-algorithm variants, and the ordered `HashResult` type.

### Changed

- Renamed the reusable Rust crate from `hashjunkie-core`/`hashjunkie_core` to
  `hashjunkie`.
- Moved CLI reader hashing onto the shared Rust library API so Rust callers and
  the CLI use the same pipelined multi-hash implementation.

## 0.5.2 - 2026-04-30

HashJunkie 0.5.2 adds shell-friendly CLI output modes for pipelines and
command substitution.

### Added

- Added CLI `line` output format (`-f line`) for shell pipelines and `-1` for
  first-input hash-only command substitution.

## 0.5.1 - 2026-04-30

HashJunkie 0.5.1 fixes `cidv0` compatibility with Kubo 0.41
`ipfs add --nocopy --cid-version=0` at the single-block boundary.

### Fixed

- Fixed `cidv0` output at the 256 KiB boundary so `cidv0` matches Kubo 0.41
  `ipfs add --nocopy --cid-version=0`: raw-leaf CIDv1-style strings for
  single-block files and `Qm...` DAG-PB roots only for multiblock files.

## 0.5.0 - 2026-04-30

HashJunkie 0.5.0 expands file-sharing hash support beyond IPFS with ED2K,
AICH, Tiger Tree, and BitTorrent v2 per-file hashes.

### Added

- Added `aich` hash support for eMule/aMule AICH root hashes.
- Added `ed2k` hash support for eDonkey/eMule/MLDonkey-compatible file hashes.
- Added `tiger` Tiger Tree Hash support for Gnutella2/Direct Connect-compatible file hashes.
- Added `btv2` hash support for BitTorrent v2 per-file `pieces root` hashes.

### Changed

- The default algorithm set now computes 18 hashes and continues to omit only `whirlpool` unless requested explicitly.

## 0.4.0 - 2026-04-30

HashJunkie 0.4.0 is the IPFS CID and high-throughput hashing release.

### Added

- Added `cidv0` and `cidv1` hash algorithms compatible with modern Kubo/IPFS `ipfs add --nocopy` behavior.
- Added native file hashing for the Node/Bun package via `hashFile()`, avoiding JavaScript stream overhead for local files.
- Added parallel native hashing for multi-hash workloads, including `HashJunkie`, `hashStream()`, and file hashing paths.
- Added optimized parallel implementations for CID, Dropbox, HiDrive, Mail.ru, and BLAKE3-heavy workloads.
- Added `-f` as the short CLI flag for `--format`.
- Added automated Homebrew formula generation from release assets and SHA256 checksums.
- Added version synchronization tooling so `VERSION`, Cargo manifests, npm packages, CLI binaries, and release assets stay aligned.

### Changed

- `whirlpool` is now opt-in instead of part of the default algorithm set. Request it explicitly with `-a whirlpool` or in the API algorithm list.
- The default algorithm set now computes 14 hashes and omits Whirlpool by default.
- Rust crates now use the 2024 edition.
- Updated Rust and npm dependencies.
- Improved README and npm package documentation with best-practice guidance for `hashFile()`, `hashStream()`, and explicit algorithm selection.

### Fixed

- Fixed CIDv0 generation for large files so it matches `ipfs add --only-hash --nocopy --cid-version=0 -Q`.
- Fixed release packaging so CLI archives are produced for Linux, macOS, and Windows and uploaded to the GitHub Release.
- Fixed CI gates for version consistency, WASM regeneration, and 100% coverage expectations.
