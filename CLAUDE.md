# HashJunkie — Development Guide

## What This Project Is

HashJunkie is a high-performance multi-hash streaming library for Node.js and Bun, plus a standalone CLI tool. It wraps any `ReadableStream` and computes all supported hashes in a single streaming pass — with zero extra copies and no external system dependencies.

- **JS library**: `HashJunkie extends TransformStream` — plug it into any Web Streams pipeline
- **CLI binary**: statically linked Rust binary — no runtime required
- **Hashes**: blake3, crc32, dropbox, hidrive, mailru, md5, quickxor, sha1, sha256, sha512, whirlpool, xxh128, xxh3

---

## Repository Layout

```
hashjunkie/
├── crates/
│   ├── hashjunkie-core/    # Pure Rust hash logic — no JS, no WASM, no napi deps
│   ├── hashjunkie-napi/    # napi-rs wrapper → platform .node addon packages
│   └── hashjunkie-cli/     # Standalone CLI binary
├── npm/
│   ├── hashjunkie/         # Main JS/TS package (TransformStream API + WASM fallback)
│   └── hashjunkie-*/       # Per-platform prebuilt .node packages
├── wasm/                   # WASM build artifacts
└── .github/workflows/
```

`hashjunkie-core` is the shared heart — both `hashjunkie-napi` and `hashjunkie-cli` depend on it. Never add napi-rs or WASM-specific code to `hashjunkie-core`.

---

## Development Philosophy

### Test-Driven Development — No Exceptions

Write the test first. Then write the minimum code to make it pass. Then refactor.

This is not negotiable. Do not write implementation code without a failing test that demands it.

### Test Coverage Policy

Coverage is measured and checked in CI. Per-crate requirements:

| Crate / package | Gate |
|---|---|
| `hashjunkie-core` | **100%** — pure logic, fully unit-testable |
| `hashjunkie-cli` | ≥ 90% — I/O entry points aren't worth complicating for coverage |
| `npm/hashjunkie` | **100%** (TypeScript) |

Tools:
- Rust: `cargo llvm-cov` with `--branch` flag
- TypeScript: `bun test --coverage`

### The Golden Rule on Tests

**Never modify a test to make it pass** unless you are 100% certain the test was testing the wrong behaviour — and you must document why in a comment above the test. If implementation and test disagree, fix the implementation.

The only legitimate reasons to change a test:
1. The test was asserting incorrect expected output (document the spec reference that proves this)
2. The interface being tested changed intentionally (update the test to match the new contract, not to paper over a bug)

When in doubt: the test is right, the code is wrong.

### Regression Tests for Every Bug Fix

Every bug fix ships with a regression test that:
1. Reproduces the bug before the fix (the test must fail on the pre-fix code)
2. Passes after the fix
3. Is named to describe the bug, not the fix — e.g. `test_quickxor_misaligned_final_block`, not `test_fix_123`

No regression test = no merge.

---

## Code Style

### Rust

- Format with `rustfmt` — run `cargo fmt` before every commit
- Lint with `clippy` — run `cargo clippy -- -D warnings` (warnings are errors)
- No `unwrap()` or `expect()` in library code (`hashjunkie-core`, `hashjunkie-napi`) — use proper error propagation
- `unwrap()` is acceptable in tests and CLI where a panic produces a clear error
- Prefer explicit types over inference at function boundaries

### TypeScript

- Format and lint with **Biome** — run `biome check --write` before every commit
- `strict: true` in `tsconfig.json` — no exceptions
- No `any` types — use `unknown` and narrow explicitly
- Prefer `type` over `interface` for object shapes

### General

- Small, focused functions — if you need to scroll to read one function, split it
- A comment explains *why*, not *what* — the code already says what

---

## Commits

Use **Conventional Commits** with plain English descriptions.

```
feat: add QuickXor algorithm implementation
fix: correct HiDrive block boundary calculation for files < 4 MiB
test: add regression test for xxh128 empty input edge case
docs: document CLI --format flag options
chore: update blake3 crate to 1.6.0
perf: avoid per-chunk heap allocation in MultiHasher::update
```

- Subject line: lowercase after the `type:` prefix, no trailing period, ≤72 characters
- Body (optional): plain English, wrap at 72 characters, explain *why* not *what*
- Reference issues: `Closes #42` in the body, not the subject

---

## Build Optimisation Profiles

### Native (CLI binary + `.node` addons)

Optimised for **size**:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### WASM

Optimised for **speed**:

```toml
[profile.wasm-release]
inherits = "release"
opt-level = 3
lto = true
```

SIMD enabled via `.cargo/config.toml`:

```toml
[target.wasm32-wasip1]
rustflags = ["-C", "target-feature=+simd128"]
```

`wasm-opt -O3` is applied as a post-build step in the WASM workflow.

---

## Documentation Standards

HashJunkie documentation serves four audiences simultaneously. Every user-facing doc must work for all of them.

| Audience | What they need |
|---|---|
| CLI users | Exact commands that work, copy-paste examples, no jargon |
| Advanced developers | API contracts, performance characteristics, algorithm details |
| Junior developers | Concepts explained without assuming prior knowledge |
| Non-developer project managers | What it does and why, no code required |

### Rules

- **No-nonsense**: say what a thing does in one sentence before explaining how
- **Professional**: correct grammar, consistent terminology, no filler phrases ("simply", "just", "easy")
- **Accurate**: if the docs and the code disagree, fix the docs immediately — stale docs are bugs
- **Exemplified**: every public API method has at least one working code example
- **Up to date**: documentation is part of every PR — a feature without docs is not done

### Algorithm documentation

Each supported hash algorithm must have a brief entry covering:
- What it is and who uses it (one sentence)
- Output format (hex string length)
- Any known limitations or quirks relevant to users

---

## CI Requirements

All of the following must pass before merge:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test` (all crates)
- `cargo llvm-cov --branch` at 100%
- `biome check` (TS)
- `bun test --coverage` at 100%
- CLI output matches rclone fixture files

### Verify workflows locally with `act` before pushing

Before `git push` (and especially before tagging a release), run the GitHub Actions locally with [`act`](https://github.com/nektos/act) to catch CI failures without burning remote runner minutes or polluting history with "fix CI" commits:

```sh
act -l                              # list workflows and jobs
act -W .github/workflows/ci.yml     # run the CI workflow
act push                            # simulate a push event across all workflows
```

Only push once `act` reports every job green. If a workflow cannot be exercised under `act` (e.g. matrix runners that need macOS/Windows), note the gap explicitly and accept the risk.
