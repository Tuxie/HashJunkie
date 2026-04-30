# Changelog

## Unreleased

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
