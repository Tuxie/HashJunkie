# IPFS CID Hash Design

## Goal

Add a `cid` algorithm that returns the same root CID Kubo produces for a single file imported with modern `ipfs add --nocopy` defaults.

## Scope

The algorithm hashes a byte stream and returns one CID string. It does not import directories, preserve filenames, wrap with a directory, use trickle layout, preserve mode or mtime, or expose custom IPFS importer options.

The intended import profile is:

- CIDv1 string encoded as lowercase base32.
- SHA2-256 multihash.
- Raw UnixFS leaves, because Kubo documents `--nocopy` as implying raw leaves.
- Fixed-size chunking with `size-262144`.
- Balanced UnixFS layout with max width 174.
- Raw `raw` codec CID for a single chunk.
- `dag-pb` UnixFS root CID when multiple chunks require an internal file node.

## Architecture

Add `Algorithm::Cid` in `hashjunkie-core` and implement a streaming hasher in `crates/hashjunkie-core/src/hashes/ipfs_cid.rs`. The hasher buffers up to 256 KiB chunks, emits raw leaf CIDs for full chunks, and builds balanced UnixFS parent layers at finalization.

The implementation will encode the small required subset of multiformats and protobuf directly:

- unsigned varints for CID, multicodec, multihash, and protobuf keys/integers;
- RFC 4648 base32 lowercase without padding with a leading multibase `b`;
- UnixFS `Data` protobuf messages for file nodes;
- DAG-PB `PBNode` and `PBLink` protobuf messages for internal nodes.

This avoids making HashJunkie depend on a full IPFS importer while keeping the behavior testable against Kubo vectors.

## Data Flow

`MultiHasher` forwards incoming bytes to `CidHasher`. `CidHasher` accumulates bytes into 256 KiB chunks. Full chunks are converted to raw leaf block metadata. On finalization:

- Empty input and single-chunk input return the raw leaf CID over the exact bytes.
- Multi-chunk input builds UnixFS file nodes over up to 174 children per node.
- If one parent layer still has more than 174 children, additional balanced parent layers are built until one root remains.
- The final root CID is returned as the digest string for `cid`.

## Testing

Tests compare the `cid` output against Kubo-generated vectors for:

- empty input;
- small single-chunk input;
- exactly 256 KiB;
- 256 KiB plus one byte;
- enough data to require more than one parent level.

Existing parser and surface tests should be updated so `cid` appears in `Algorithm::all()`, CLI defaults, WASM defaults, and JS-facing generated results.
