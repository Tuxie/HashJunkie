# HashJunkie

Compute multiple file hashes in a single streaming pass — no re-reading, no extra copies, no external dependencies.

HashJunkie ships as two tools that share the same Rust core:

- **`@perw/hashjunkie`** — TypeScript/JavaScript library for Bun and Node.js
- **`hashjunkie` CLI** — standalone binary for shell scripts and pipelines

Both support the same 19 algorithms and produce identical output. Whirlpool is supported but opt-in because it is much slower than the other hashes.

---

## JS library

```sh
bun add @perw/hashjunkie
```

```ts
import { HashJunkie } from "@perw/hashjunkie";

const hj = new HashJunkie(["sha256", "blake3", "md5"]);
await Bun.file("video.mp4").stream().pipeThrough(hj).pipeTo(Bun.stdout.writable);

const { sha256, blake3, md5 } = await hj.digests;
```

`HashJunkie` is a [`TransformStream`](https://developer.mozilla.org/en-US/docs/Web/API/TransformStream) — it passes every byte through unchanged while computing hashes in the background. Pipe a readable stream through it to any destination; the `digests`, `hexdigests`, and `rawdigests` promises resolve once the stream closes.

```ts
// Hash a buffer without piping anywhere
const hj = new HashJunkie(["sha256"]);
const w = hj.writable.getWriter();
await w.write(new TextEncoder().encode("hello"));
await w.close();
const { sha256 } = await hj.digests;  // lowercase hex string
const { cidv1 } = await hj.hexdigests; // raw CID bytes as lowercase hex
const raw = await hj.rawdigests;       // Uint8Array values

// No arguments = the default 18 algorithms at once; add "whirlpool" explicitly when needed
const hj2 = new HashJunkie();
```

**Full API documentation:** [npm/hashjunkie/README.md](npm/hashjunkie/README.md)

For best performance in Bun/Node, prefer `hashFile()` for local files, `HashJunkie` when bytes already need to stream onward, and explicit algorithm lists on latency-sensitive paths.

---

## CLI

Download the latest binary from [Releases](https://github.com/Tuxie/HashJunkie/releases) and put it on your `PATH`.

Release assets are published with these archive names:

- `hashjunkie-cli-{version}-linux-x64-gnu.tar.xz`
- `hashjunkie-cli-{version}-linux-arm64-gnu.tar.xz`
- `hashjunkie-cli-{version}-darwin-x64.tar.xz`
- `hashjunkie-cli-{version}-darwin-arm64.tar.xz`
- `hashjunkie-cli-{version}-win32-x64-msvc.zip`

### Hash files

```sh
# Default 18 algorithms, JSON output
hashjunkie file.bin

# Specific algorithms
hashjunkie -a sha256,md5 file.bin

# Whirlpool is opt-in
hashjunkie -a whirlpool file.bin

# Multiple files — output is a JSON array matching rclone lsjson --hash format
hashjunkie -a sha256 *.bin

# Plain text output
hashjunkie -f hex file.bin

# Force lowercase hex for algorithms whose standard form is not hex
hashjunkie --hex -a aich,cidv1 file.bin

# One line per file: hashes in requested order, then size and path
hashjunkie -f line -a blake3,sha1,md5 *.mp3

# Hashes only for the first input, useful in command substitution
echo "b3hash: $(hashjunkie -1a blake3 file.bin)"
```

**JSON output** (stdin):
```json
{"Hashes":{"sha256":"..."},"ModTime":"2026-04-30T09:30:36.146550835Z","Name":"-","Path":"-","Size":3}
```

**JSON output** (files):
```json
[
  {"Hashes":{"md5":"...","sha256":"..."},"ModTime":"...","Name":"a.bin","Path":"a.bin","Size":1024},
  {"Hashes":{"md5":"...","sha256":"..."},"ModTime":"...","Name":"b.bin","Path":"b.bin","Size":2048}
]
```

**Hex output**:
```
blake3: af1349b9f5f9a1a6a0...
md5: 900150983cd24fb0d6...
sha256: ba7816bf8f01cfea41...
```

**Line output** (`-f line`):
```
af1349b9f5f9a1a6a0... a9993e364706816aba3e25717850c26c9cd0d89d 900150983cd24fb0d6963f7d28e17f72 12345 /path/to/file.mp3
```

`line` format prints selected hashes in the order requested with `-a`, followed by file size and path. `-1` prints only the selected hashes, space-separated, for the first input.

`--hex` changes digest display to lowercase hex of the raw digest bytes for every output mode, including JSON, `-f hex`, `-f line`, and `-1`. Without `--hex`, each algorithm uses its standard visual representation.

### Hash stdin

```sh
cat file.bin | hashjunkie
cat file.bin | hashjunkie -a sha256 -f hex
```

### Use in shell scripts

```sh
sha=$(hashjunkie -a sha256 -f hex file.bin | awk '{print $2}')
echo "SHA-256: $sha"

hashjunkie -f line -a blake3,sha1,md5 *.mp3 |
  while read BLAKE3 SHA1 MD5 SIZE FILE; do
    echo "$BLAKE3 $FILE"
  done
```

---

## Supported algorithms

| Algorithm | Description | Output |
|---|---|---|
| `aich` | eMule/aMule AICH root hash — SHA-1 tree over ED2K parts and 180 KiB blocks | Base32 SHA-1 tree root |
| `blake3` | BLAKE3 | 64 hex chars |
| `btv2` | BitTorrent v2 per-file `pieces root` — BEP 52 SHA-256 Merkle root | 64 hex chars |
| `cidv0` | Kubo `ipfs add --nocopy --cid-version=0` CID | CID string; raw-leaf `bafk...` for single-block files, `Qm...` DAG-PB root for multiblock files |
| `cidv1` | IPFS CIDv1 for `ipfs add --nocopy --cid-version=1` | base32 CID string |
| `crc32` | CRC-32 | 8 hex chars |
| `dropbox` | Dropbox content hash — SHA-256 over 4 MiB blocks | 64 hex chars |
| `ed2k` | eDonkey/eMule/MLDonkey ED2K file hash — MD4 over 9500 KiB blocks | 32 hex chars |
| `hidrive` | STRATO HiDrive — SHA-1 block tree | 40 hex chars |
| `mailru` | Mail.ru hash | 40 hex chars |
| `md5` | MD5 | 32 hex chars |
| `quickxor` | Microsoft QuickXorHash (OneDrive/SharePoint) | 40 hex chars |
| `sha1` | SHA-1 | 40 hex chars |
| `sha256` | SHA-256 | 64 hex chars |
| `sha512` | SHA-512 | 128 hex chars |
| `tiger` | Tiger Tree Hash used by Gnutella2/Direct Connect | Base32 Tiger root |
| `whirlpool` | Whirlpool, opt-in | 128 hex chars |
| `xxh128` | xxHash 128-bit | 32 hex chars |
| `xxh3` | xxHash 64-bit | 16 hex chars |

Most standard digest strings are lowercase hex. `aich` returns the standard uppercase Base32 AICH root used in eD2K links as `h=...`. `btv2` returns the BEP 52 per-file `pieces root`; BEP 52 omits `pieces root` for empty files, so HashJunkie returns the zero Merkle root for standalone empty-file hashing. `cidv0` matches Kubo 0.41 `ipfs add --nocopy --cid-version=0`: single-block files return raw-leaf CIDv1-style `bafk...` strings, while multiblock files return 46-character base58btc DAG-PB roots beginning with `Qm`. `cidv1` returns lowercase base32 CIDv1 strings. `tiger` returns the standard uppercase Base32 Tiger Tree root. The JSON field names match the algorithm names above and are always sorted alphabetically. Use CLI `--hex` or JS `.hexdigests` when you need lowercase hex for the underlying digest bytes. When no algorithms are specified, HashJunkie computes the default 18 algorithms and skips `whirlpool`; pass `-a whirlpool` or include `"whirlpool"` in the API algorithm list to compute it.

The multi-block algorithms (`aich`, `btv2`, `dropbox`, `ed2k`, `hidrive`, `mailru`) produce output compatible with their standard service/client definitions; `dropbox`, `hidrive`, and `mailru` match [rclone](https://rclone.org/)'s `lsjson --hash` command.

---

## How it works

HashJunkie reads each byte of input exactly once. All active hashers run in parallel on each chunk — there is no second pass, no temporary file, and no duplication of data in memory.

The Rust core is compiled into:

- A **native `.node` addon** (via [napi-rs](https://napi.rs/)) for use in Bun and Node.js — zero-copy, statically linked, no `dlopen`
- A **WebAssembly module** embedded inline in the JS package as a base64 string — automatic fallback when no native addon is present (browsers, Deno, Node SEA, etc.)
- A **standalone CLI binary** — statically linked, no runtime required

The JS library loads the native addon if available, otherwise falls back to WASM automatically. No configuration needed.

---

## Repository layout

```
hashjunkie/
├── crates/
│   ├── hashjunkie-core/        # Rust hash logic — 19 supported algorithms
│   ├── hashjunkie-napi/        # napi-rs wrapper → platform .node addons
│   └── hashjunkie-cli/         # Standalone binary (clap, stdin + file modes)
├── npm/
│   ├── hashjunkie/             # @perw/hashjunkie — main JS/TS package
│   └── hashjunkie-*/           # Per-platform prebuilt .node packages
└── scripts/
    └── build-wasm.sh           # Builds WASM blob and embeds it in wasm_blob.ts
```

`hashjunkie-core` is the shared heart — both the CLI and the JS addon depend on it. The core has no JS, napi-rs, or WASM dependencies.

---

## Development

### Prerequisites

- Rust stable + nightly (nightly is used only for branch coverage reports)
- Bun ≥ 1.0
- For WASM builds: `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.120`

### Run all checks

```sh
# Rust
cargo fmt --all
cargo clippy --workspace --exclude hashjunkie-napi --all-targets -- -D warnings
cargo test --workspace --exclude hashjunkie-napi

# TypeScript
cd npm/hashjunkie
bun install
bun test
./node_modules/.bin/biome check .
```

### Coverage

```sh
# hashjunkie-core is held to 100% line + branch coverage
cargo +nightly llvm-cov -p hashjunkie-core --branch --fail-under-lines 100

# TypeScript
cd npm/hashjunkie && bun test --coverage
```

### Profile IPFS CID hashing

```sh
cargo run --release -p hashjunkie-core --features profile-ipfs-cid \
  --bin hashjunkie-cid-profile -- cidv0 /path/to/file
```

The profiling binary prints total runtime plus time spent in chunk buffering, raw leaf hashing, DAG-PB encoding, DAG-PB hashing, and final CID text encoding.

### Rebuild the WASM blob

Run this whenever `crates/hashjunkie-wasm/src/lib.rs` changes:

```sh
./scripts/build-wasm.sh
```

The script builds the WASM binary, runs `wasm-bindgen`, and writes the base64-encoded blob to `npm/hashjunkie/wasm_blob.ts`. Commit the generated files.

### Release

`VERSION` is the single source of truth. Before publishing, run:

```sh
node scripts/version-sync.mjs check
node scripts/release-notes.mjs "$(node scripts/version-sync.mjs print)"
```

Update `CHANGELOG.md` before every release. The section for `VERSION` is used as the GitHub Release body, so it must be written for users, not as a raw commit dump.

Pushing release-relevant changes to `main` triggers the GitHub Actions release path. The workflow publishes the platform npm packages, publishes `@perw/hashjunkie`, tags `v{VERSION}`, uploads CLI archives to the GitHub Release, updates the GitHub Release notes from `CHANGELOG.md`, and updates `Tuxie/homebrew-tap` with the release version and archive SHA256s.

Before pushing changes that add or edit GitHub Actions jobs, run the closest practical local `act` check first, for example `act -j <job-id>`. `act` will not prove cross-architecture builds, hosted macOS/Windows behavior, real uploads, publishing, release edits, or tap pushes, but it catches many workflow syntax, job wiring, shell, and missing-file mistakes before CI sees them.

### Commit style

[Conventional Commits](https://www.conventionalcommits.org/) with plain English descriptions:

```
feat: add QuickXorHash algorithm
fix: correct HiDrive block boundary for files < 128 KiB
test: add regression test for dropbox empty-file edge case
chore: update blake3 crate to 1.6.0
```

Commit subjects should be good enough that `git log --oneline <previous-tag>..HEAD` can be used as the first draft for the next release notes. Avoid vague subjects like `fix tests`, `misc`, or `release changes`; name the behavior, packaging, performance, or documentation outcome.

---

## License

MIT
