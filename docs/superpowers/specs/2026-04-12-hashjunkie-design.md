# HashJunkie — Design Specification

**Date:** 2026-04-12  
**Status:** Approved

---

## Overview

HashJunkie is a high-performance multi-hash streaming library targeting Node.js and Bun, plus a standalone CLI binary. It computes all supported hash algorithms in a single streaming pass over the input, exposing results as a `Promise<Record<HashName, string>>` that resolves when the stream closes.

**Supported algorithms:** blake3, crc32, dropbox, hidrive, mailru, md5, quickxor, sha1, sha256, sha512, whirlpool, xxh128, xxh3

---

## Architecture

### Three Rust crates, one JS package

```
hashjunkie/
├── crates/
│   ├── hashjunkie-core/       # Pure hash logic — no JS, no napi, no WASM glue
│   ├── hashjunkie-napi/       # napi-rs wrapper → .node platform addon packages
│   └── hashjunkie-cli/        # Standalone binary (stdin + file path modes)
├── npm/
│   ├── hashjunkie/            # JS/TS package: TransformStream API + embedded WASM
│   ├── hashjunkie-linux-x64-gnu/
│   ├── hashjunkie-linux-arm64-gnu/
│   ├── hashjunkie-darwin-x64/
│   ├── hashjunkie-darwin-arm64/
│   └── hashjunkie-win32-x64-msvc/
├── wasm/                      # WASM build artifacts
└── .github/workflows/
    ├── build-native.yml       # napi-rs cross-compile matrix
    ├── build-wasm.yml         # WASM + wasm-opt
    └── build-cli.yml          # Rust CLI cross-compile
```

`hashjunkie-core` is the shared dependency for both `hashjunkie-napi` and `hashjunkie-cli`. It has no knowledge of JS, WASM targets, or napi-rs.

---

## hashjunkie-core

### Public interface

```rust
pub enum Algorithm {
    Blake3, Crc32, Dropbox, Hidrive, Mailru,
    Md5, QuickXor, Sha1, Sha256, Sha512,
    Whirlpool, Xxh128, Xxh3,
}

pub struct MultiHasher {
    hashers: Vec<Box<dyn Hasher>>,
}

impl MultiHasher {
    pub fn new(algorithms: &[Algorithm]) -> Self;
    pub fn update(&mut self, data: &[u8]);
    pub fn finalize(self) -> HashMap<Algorithm, String>; // lowercase hex strings
}
```

`update()` fans out a single chunk to all active hashers with no intermediate allocation. `finalize()` consumes the hasher and returns all digests as lowercase hex strings.

### Crate dependencies

| Algorithm(s) | Crate |
|---|---|
| MD5, SHA1, SHA256, SHA512, Whirlpool | `RustCrypto/hashes` |
| BLAKE3 | `blake3` |
| CRC32 | `crc32fast` |
| XXH3, XXH128 | `xxhash-rust` |
| Dropbox | Custom (SHA256 over 4 MiB blocks) |
| HiDrive | Custom (SHA1 per block + SHA1 of concatenated SHA1s) |
| Mail.ru | Custom (chunked SHA1 variant per Mail.ru spec) |
| QuickXor | Custom (Microsoft QuickXOR spec) |

All crates must be WASM-compatible. No system library dependencies (no OpenSSL, no zlib, no libc beyond core).

---

## hashjunkie-napi

Thin napi-rs wrapper around `hashjunkie-core`. Exposes an opaque `NativeHasher` object to JS with three methods: `update(buffer)`, `finalize() → object`. The JS layer handles `TransformStream` mechanics; the native binding handles only hashing.

**Compilation:** statically linked, size-optimised (`opt-level = "z"`, `lto`, `strip`, `panic = "abort"`).

**Distribution:** one npm package per platform, declared as `optionalDependencies` in the main `hashjunkie` package. Package manager installs only the matching platform package.

---

## hashjunkie-cli

Standalone Rust binary. Statically linked. Size-optimised. No JS runtime required.

### Modes

**File mode** (one or more path arguments):
```
hashjunkie file1.bin file2.bin
```
Outputs a JSON array matching rclone's `lsjson --hash` `Hashes` field format, one object per file.

**Stdin mode** (no arguments):
```
cat file.bin | hashjunkie
```
Reads stdin to EOF, outputs a single JSON object of digests to stdout.

### Flags

| Flag | Description |
|---|---|
| `-a, --algorithms <list>` | Comma-separated subset of algorithms (default: all) |
| `--format <json\|hex>` | Output format: JSON (default) or `algo: digest` lines |
| `--help` | Usage |
| `--version` | Version string |

---

## JS/TS Package (npm/hashjunkie)

### Public API

```ts
type Algorithm =
  | 'blake3' | 'crc32' | 'dropbox' | 'hidrive' | 'mailru'
  | 'md5' | 'quickxor' | 'sha1' | 'sha256' | 'sha512'
  | 'whirlpool' | 'xxh128' | 'xxh3';

type Digests = Record<Algorithm, string>;

class HashJunkie extends TransformStream<Uint8Array, Uint8Array> {
  constructor(algorithms?: Algorithm[]);  // omit for all algorithms
  readonly digests: Promise<Digests>;    // resolves on stream close, rejects on stream error
}
```

### Usage

```ts
import { HashJunkie } from 'hashjunkie';

const hj = new HashJunkie(['sha256', 'blake3', 'xxh3']);

await Bun.file('input.bin').stream()
  .pipeThrough(hj)
  .pipeTo(Bun.file('output.bin').writer());

const digests = await hj.digests;
// { sha256: "0c80cb...", blake3: "543e4e...", xxh3: "9697df..." }
```

### Native/WASM Loader

The loader uses static `require()` calls per platform (no dynamic string construction) so `bun build --compile` can embed the `.node` file at bundle time. On load, it attempts the native addon for the current platform; on any failure it falls back to the inlined WASM blob. The WASM blob is base64-encoded and decoded at module init — no filesystem access required.

```ts
function loadNative() {
  if (process.platform === 'linux' && process.arch === 'x64')
    return require('./hashjunkie.linux-x64-gnu.node');
  if (process.platform === 'linux' && process.arch === 'arm64')
    return require('./hashjunkie.linux-arm64-gnu.node');
  if (process.platform === 'darwin' && process.arch === 'x64')
    return require('./hashjunkie.darwin-x64.node');
  if (process.platform === 'darwin' && process.arch === 'arm64')
    return require('./hashjunkie.darwin-arm64.node');
  if (process.platform === 'win32' && process.arch === 'x64')
    return require('./hashjunkie.win32-x64-msvc.node');
  return null;
}
```

### Deployment matrix

| Deployment | Runtime |
|---|---|
| `bun run` / `node` | Native addon |
| `bun build --compile` | Native addon (embedded) |
| `esbuild` bundle | Native addon (external file) |
| Node SEA | WASM fallback |
| Browser / Deno | WASM fallback |

---

## Build Optimisation

### Native (CLI + .node addons)

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### WASM

```toml
[profile.wasm-release]
inherits = "release"
opt-level = 3
lto = true
```

SIMD via `.cargo/config.toml`:

```toml
[target.wasm32-wasip1]
rustflags = ["-C", "target-feature=+simd128"]
```

`wasm-opt -O3` applied post-build.

---

## Error Handling

- Invalid algorithm name at construction → synchronous `TypeError` (fast fail before any IO)
- Source stream abort → `hj.digests` rejects with the upstream error; error propagates through the `TransformStream` chain normally
- Native addon load failure → silent fallback to WASM; if WASM also fails → `Error` thrown at construction with a clear message
- CLI IO error → exit code 1, message to stderr, nothing to stdout

---

## Testing

- **`hashjunkie-core`:** unit tests per algorithm against known vectors (NIST, rclone fixtures, Dropbox/QuickXor spec vectors). `cargo test`.
- **Cross-binding parity:** integration tests feeding identical byte sequences through both the native binding and WASM binding, asserting identical digests.
- **JS package:** `bun test` — `HashJunkie` TransformStream end-to-end: correct digests, passthrough correctness (output bytes === input bytes), `digests` rejection on stream abort.
- **CLI:** shell-level tests comparing output against `rclone lsjson --hash` on fixture files.
- **Coverage:** `cargo llvm-cov --branch` at 100% for Rust; `bun test --coverage` at 100% for TS.
- **Regression:** every bug fix ships a named regression test.

---

## CI

All must pass before merge:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test` (all crates)
- `cargo llvm-cov --branch` → 100%
- `biome check`
- `bun test --coverage` → 100%
- CLI fixture comparison against rclone
- Native builds: `ubuntu-latest`, `macos-latest`, `windows-latest`
- WASM build: `ubuntu-latest`
