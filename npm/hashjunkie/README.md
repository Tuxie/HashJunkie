# HashJunkie

Multi-hash streaming library for Bun and Node.js. Computes any combination of 13 hash algorithms in a single pass — zero extra copies, no external system dependencies.

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
| `crc32` | CRC-32 |
| `dropbox` | Dropbox content hash (SHA-256 of 4 MiB blocks) |
| `hidrive` | STRATO HiDrive hash (SHA-1 block tree) |
| `mailru` | Mail.ru hash |
| `md5` | MD5 |
| `quickxor` | Microsoft QuickXorHash (used by OneDrive) |
| `sha1` | SHA-1 |
| `sha256` | SHA-256 |
| `sha512` | SHA-512 |
| `whirlpool` | Whirlpool |
| `xxh128` | xxHash 128-bit |
| `xxh3` | xxHash 64-bit (xxh3) |

Pass no arguments to get all 13 hashes at once.

## Installation

```sh
bun add @perw/hashjunkie
```

## Usage

Three entry points cover the common cases:

```ts
import { HashJunkie, hashBuffer, hashStream } from "@perw/hashjunkie";

// 1. Pass-through: compute hashes while streaming bytes somewhere else.
const hj = new HashJunkie(["sha256", "md5"]);
await source.pipeThrough(hj).pipeTo(dest);
const { sha256, md5 } = await hj.digests;

// 2. In-memory buffer → digests, no plumbing.
const digests = await hashBuffer(new TextEncoder().encode("hello world"));

// 3. ReadableStream → digests; the stream is drained, the bytes are discarded.
const fileDigests = await hashStream(Bun.file("big.bin").stream(), ["blake3", "sha256"]);

// Read the typed algorithm list
import { ALGORITHMS } from "@perw/hashjunkie";
console.log(ALGORITHMS); // readonly ["blake3", "crc32", ...]
```

All digests are lowercase hex strings. The `digests` promise resolves when the writable side closes cleanly, and rejects if the stream is aborted.

## How it works

HashJunkie uses a Rust core via a native `.node` addon (napi-rs, statically linked) when running in Bun or Node.js, with an automatic fallback to a WebAssembly module when no native addon is available. The WASM module is embedded inline — no fetch, no extra files.

The native path is **zero-copy**: each chunk is passed directly to the Rust hasher without intermediate buffers.

## Performance

On an M2 MacBook Pro, hashing a 1 GiB file with all 13 algorithms simultaneously runs at ~2.5 GiB/s with the native addon.

## Types

```ts
import type { Algorithm, Digests } from "@perw/hashjunkie";
```

```ts
type Algorithm =
  | "blake3" | "crc32" | "dropbox" | "hidrive" | "mailru"
  | "md5" | "quickxor" | "sha1" | "sha256" | "sha512"
  | "whirlpool" | "xxh128" | "xxh3";

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
```

## License

MIT
