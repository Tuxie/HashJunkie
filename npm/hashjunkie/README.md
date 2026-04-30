# HashJunkie

Multi-hash streaming library for Bun and Node.js. Computes any combination of 18 hash algorithms in a single pass — zero extra copies, no external system dependencies. Whirlpool is supported but opt-in because it is much slower than the other hashes.

```ts
import { HashJunkie } from "@perw/hashjunkie";

const hj = new HashJunkie(["sha256", "blake3", "md5"]);
await Bun.file("large-file.bin").stream().pipeThrough(hj).pipeTo(Bun.stdout.writable);

const { sha256, blake3, md5 } = await hj.digests;
```

## Supported algorithms

| Name | Description |
|---|---|
| `blake3` | BLAKE3 (256-bit) |
| `btv2` | BitTorrent v2 per-file `pieces root` (BEP 52 SHA-256 Merkle root) |
| `cidv0` | IPFS CID matching stock Kubo `ipfs add --nocopy` defaults |
| `cidv1` | IPFS CIDv1 for `ipfs add --nocopy --cid-version=1` |
| `crc32` | CRC-32 |
| `dropbox` | Dropbox content hash (SHA-256 of 4 MiB blocks) |
| `ed2k` | eDonkey/eMule/MLDonkey ED2K file hash (MD4 over 9500 KiB blocks) |
| `hidrive` | STRATO HiDrive hash (SHA-1 block tree) |
| `mailru` | Mail.ru hash |
| `md5` | MD5 |
| `quickxor` | Microsoft QuickXorHash (used by OneDrive) |
| `sha1` | SHA-1 |
| `sha256` | SHA-256 |
| `sha512` | SHA-512 |
| `tiger` | Tiger Tree Hash used by Gnutella2/Direct Connect |
| `whirlpool` | Whirlpool, opt-in |
| `xxh128` | xxHash 128-bit |
| `xxh3` | xxHash 64-bit (xxh3) |

Pass no arguments to get the default 17 hashes at once. Include `whirlpool` explicitly when you need a 1Fichier-compatible Whirlpool hash.

## Installation

```sh
bun add @perw/hashjunkie
```

## Usage

Four entry points cover the common cases:

```ts
import { HashJunkie, hashBuffer, hashFile, hashStream } from "@perw/hashjunkie";

// 1. Pass-through: compute hashes while streaming bytes somewhere else.
const hj = new HashJunkie(["sha256", "md5"]);
await source.pipeThrough(hj).pipeTo(dest);
const { sha256, md5 } = await hj.digests;

// 2. In-memory buffer → digests, no plumbing.
const digests = await hashBuffer(new TextEncoder().encode("hello world"));

// 3. ReadableStream → digests; the stream is drained, the bytes are discarded.
const fileDigests = await hashStream(Bun.file("big.bin").stream(), ["blake3", "sha256"]);

// 4. Local file path → digests; native builds do file IO in Rust.
const localFileDigests = await hashFile("big.bin", ["blake3", "sha256"]);

// Read the typed algorithm list
import { ALGORITHMS, DEFAULT_ALGORITHMS } from "@perw/hashjunkie";
console.log(ALGORITHMS); // readonly ["blake3", "crc32", ...]
console.log(DEFAULT_ALGORITHMS); // same list without "whirlpool"
```

Most digests are lowercase hex strings. `btv2` returns the BEP 52 per-file `pieces root` as lowercase hex; BEP 52 omits `pieces root` for empty files, so HashJunkie returns the zero Merkle root for standalone empty-file hashing. `cidv0` returns Kubo-compatible CIDv0 roots for multi-block DAG-PB files and CIDv1 raw-leaf strings for single-block files. `cidv1` returns lowercase base32 CIDv1 strings. `tiger` returns the standard uppercase Base32 Tiger Tree root. The `digests` promise resolves when the writable side closes cleanly, and rejects if the stream is aborted.

## Best practices

Use `hashFile()` for local files when you only need digests. Native builds do file IO in Rust with large reads and avoid JavaScript stream overhead:

```ts
import { hashFile } from "@perw/hashjunkie";

const digests = await hashFile("/media/card/DCIM/IMG_0001.CR3", [
  "blake3",
  "sha256",
  "cidv1",
]);
```

Use `HashJunkie` when bytes already need to flow somewhere else. It is a pass-through transform, so this computes hashes while preserving your existing pipeline:

```ts
import { HashJunkie } from "@perw/hashjunkie";

const hasher = new HashJunkie(["blake3", "sha256"]);
await Bun.file("clip.mov").stream().pipeThrough(hasher).pipeTo(uploadBody);
const digests = await hasher.digests;
```

Use `hashStream()` when you have a stream and only want the digests. When you control the producer, feed multi-MiB `Uint8Array` chunks so the native backend can keep worker threads busy:

```ts
import { hashStream } from "@perw/hashjunkie";

const digests = await hashStream(myReadableStream, ["blake3", "dropbox", "cidv1"]);
```

Specify the algorithms you actually need for latency-sensitive paths. The default set is convenient, but explicit lists avoid work you will not use. `whirlpool` is always opt-in and should only be requested for services that require it.

## How it works

HashJunkie uses a Rust core via a native `.node` addon (napi-rs, statically linked) when running in Bun or Node.js, with an automatic fallback to a WebAssembly module when no native addon is available. The WASM module is embedded inline — no fetch, no extra files.

`hashFile()` is the fastest local-file API. Native builds do file IO in Rust with large reads; the BLAKE3-only path uses BLAKE3's mmap+rayon whole-file implementation. WASM builds fall back to `Bun.file(path).stream()`.

For `HashJunkie` and `hashStream()`, native builds pipeline multiple active hashers across worker threads while preserving byte order for each algorithm. Feed multi-MiB chunks when you control the stream source.

## Performance

On an M2 MacBook Pro, hashing a 1 GiB file with all pre-CID algorithms simultaneously runs at ~2.5 GiB/s with the native addon.

## Types

```ts
import type { Algorithm, Digests } from "@perw/hashjunkie";
```

```ts
type Algorithm =
  | "blake3" | "btv2" | "cidv0" | "cidv1" | "crc32" | "dropbox" | "ed2k"
  | "hidrive" | "mailru" | "md5" | "quickxor" | "sha1" | "sha256" | "sha512"
  | "tiger" | "whirlpool" | "xxh128" | "xxh3";

type Digests = Record<Algorithm, string>;

class HashJunkie extends TransformStream<Uint8Array, Uint8Array> {
  constructor(algorithms?: Algorithm[]);
  readonly digests: Promise<Digests>;
}

function hashBuffer(data: Uint8Array, algorithms?: Algorithm[]): Promise<Digests>;
function hashStream(
  stream: ReadableStream<Uint8Array>,
  algorithms?: Algorithm[],
): Promise<Digests>;
function hashFile(path: string, algorithms?: Algorithm[]): Promise<Digests>;
```

## License

MIT
