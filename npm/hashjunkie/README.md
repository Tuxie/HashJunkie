# HashJunkie

Multi-hash streaming library for Bun and Node.js. Computes any combination of 13 hash algorithms in a single pass — zero extra copies, no external system dependencies.

```ts
import { HashJunkie } from "hashjunkie";

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
bun add hashjunkie
```

## Usage

`HashJunkie` is a [`TransformStream`](https://developer.mozilla.org/en-US/docs/Web/API/TransformStream) — pipe data through it, then await `digests`.

```ts
import { HashJunkie } from "hashjunkie";

// Hash a file while streaming it somewhere
const hj = new HashJunkie(["sha256", "md5"]);
await source.pipeThrough(hj).pipeTo(dest);
const { sha256, md5 } = await hj.digests;

// Hash a buffer directly
const hj2 = new HashJunkie();            // all 13 algorithms
const writer = hj2.writable.getWriter();
await writer.write(new TextEncoder().encode("hello world"));
await writer.close();
const digests = await hj2.digests;       // Record<Algorithm, string> — hex strings

// Read the typed algorithm list
import { ALGORITHMS } from "hashjunkie";
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
type Algorithm =
  | "blake3" | "crc32" | "dropbox" | "hidrive" | "mailru"
  | "md5" | "quickxor" | "sha1" | "sha256" | "sha512"
  | "whirlpool" | "xxh128" | "xxh3";

type Digests = Record<Algorithm, string>;

class HashJunkie extends TransformStream<Uint8Array, Uint8Array> {
  constructor(algorithms?: Algorithm[]);
  readonly digests: Promise<Digests>;
}
```

## License

MIT
