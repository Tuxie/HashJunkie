# IPFS CID Hash Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `cid` algorithm that matches Kubo `ipfs add --nocopy` defaults for single-file byte streams.

**Architecture:** Implement CID generation in `hashjunkie-core` as a streaming hasher selected by `Algorithm::Cid`. Keep CLI, WASM, and N-API integration through the existing algorithm enum and parser.

**Tech Stack:** Rust 2021, existing `sha2` dependency, direct varint/base32/protobuf encoding, Cargo tests.

---

### Task 1: Add Failing CID Surface Tests

**Files:**
- Modify: `crates/hashjunkie-core/src/algorithm.rs`
- Modify: `crates/hashjunkie-core/src/multi.rs`
- Modify: `crates/hashjunkie-core/tests/vectors.rs`
- Modify: `crates/hashjunkie-cli/src/args.rs`
- Modify: `crates/hashjunkie-wasm/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Update count assertions from 13 to 14 and add `cid` expectations in parser/vector tests. Add a vector for `tests/fixtures/small.bin`:

```rust
("cid", "EXPECTED_KUBO_CID_FOR_SMALL_BIN")
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p hashjunkie-core -p hashjunkie-cli -p hashjunkie-wasm`

Expected: tests fail because `cid` is an unknown algorithm and the all-count assertions still see 13 algorithms.

### Task 2: Implement CID Encoding

**Files:**
- Create: `crates/hashjunkie-core/src/hashes/ipfs_cid.rs`
- Modify: `crates/hashjunkie-core/src/hashes/mod.rs`
- Modify: `crates/hashjunkie-core/src/algorithm.rs`
- Modify: `crates/hashjunkie-core/src/multi.rs`

- [ ] **Step 1: Implement minimal CID hasher**

Create `CidHasher` with constants:

```rust
const CHUNK_SIZE: usize = 262_144;
const MAX_LINKS: usize = 174;
const MULTIHASH_SHA2_256: u64 = 0x12;
const MULTICODEC_RAW: u64 = 0x55;
const MULTICODEC_DAG_PB: u64 = 0x70;
```

Implement unsigned-varint encoding, base32-lower-no-padding with multibase `b`, SHA2-256 multihash wrapping, CIDv1 bytes, DAG-PB node/link encoding, and UnixFS file data encoding.

- [ ] **Step 2: Wire the algorithm**

Add `Algorithm::Cid`, `as_str() == "cid"`, parser support, `Algorithm::all()` inclusion, and `make_hasher(Algorithm::Cid)`.

- [ ] **Step 3: Run focused tests**

Run: `cargo test -p hashjunkie-core`

Expected: core tests pass including vectors.

### Task 3: Verify Public Surfaces

**Files:**
- Modify: `crates/hashjunkie-cli/src/args.rs`
- Modify: `crates/hashjunkie-wasm/src/lib.rs`
- Modify: `npm/hashjunkie/README.md`
- Modify: `README.md`

- [ ] **Step 1: Update documentation text and counts**

Mention `cid` in user-facing algorithm lists and update any “all 13 algorithms” text to 14.

- [ ] **Step 2: Run full verification**

Run: `cargo test --workspace`

Expected: all Rust tests pass.

Run: `bun test` from `npm/hashjunkie`

Expected: JS package tests pass.
