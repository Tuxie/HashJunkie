# hashjunkie-napi + WASM Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `hashjunkie-napi` (napi-rs 2 native binding) and `hashjunkie-wasm` (wasm-bindgen binding), plus scaffold per-platform npm package stubs, so the JS package (Plan 4) has both a native fast path and a WASM fallback.

**Architecture:** `hashjunkie-napi` is a thin napi-rs 2.x wrapper around `hashjunkie-core::MultiHasher`, compiled to a per-platform `.node` native addon that Node.js/Bun load at runtime. `hashjunkie-wasm` is a wasm-bindgen wrapper compiled to a `.wasm` file plus JS glue that runs in any JS environment without a native addon. Both expose the same logical API: `new(algorithms?)`, `update(data)`, `finalize() → object`. Per-platform npm packages are scaffolded as empty stubs — CI (Plan 5) builds and publishes the actual `.node` binaries via cross-compilation matrix.

**Tech Stack:** Rust, napi-rs 2.x (`napi` + `napi-derive` + `napi-build`), wasm-bindgen 0.2, js-sys 0.3, hashjunkie-core (workspace path dep)

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `Cargo.toml` | Modify | Add workspace members, napi/wasm-bindgen workspace deps, fix wasm-release lto |
| `.cargo/config.toml` | Modify | Add wasm32-unknown-unknown SIMD flags |
| `crates/hashjunkie-napi/Cargo.toml` | Create | napi crate manifest (cdylib, napi 2.x) |
| `crates/hashjunkie-napi/build.rs` | Create | napi-build setup (required for `.node` export registration) |
| `crates/hashjunkie-napi/src/lib.rs` | Create | NativeHasher napi binding |
| `crates/hashjunkie-wasm/Cargo.toml` | Create | wasm crate manifest (cdylib + rlib, wasm-bindgen) |
| `crates/hashjunkie-wasm/src/lib.rs` | Create | WasmHasher wasm-bindgen binding |
| `npm/hashjunkie-linux-x64-gnu/package.json` | Create | Linux x64 platform package stub |
| `npm/hashjunkie-linux-arm64-gnu/package.json` | Create | Linux arm64 platform package stub |
| `npm/hashjunkie-darwin-x64/package.json` | Create | macOS x64 platform package stub |
| `npm/hashjunkie-darwin-arm64/package.json` | Create | macOS arm64 platform package stub |
| `npm/hashjunkie-win32-x64-msvc/package.json` | Create | Windows x64 platform package stub |

---

### Task 1: Workspace setup + hashjunkie-napi skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/hashjunkie-napi/Cargo.toml`
- Create: `crates/hashjunkie-napi/build.rs`
- Create: `crates/hashjunkie-napi/src/lib.rs`

**Context:** napi-rs 2.x requires a `build.rs` that calls `napi_build::setup()`. This generates the `napi_register_module_v1` symbol that Node.js looks for when loading a `.node` file. The crate type is `cdylib` (a C-compatible shared library). napi-rs does NOT require Node.js to be installed at compile time — it links napi symbols at runtime when Node.js loads the addon. This means `cargo build -p hashjunkie-napi` works without Node.js.

**Testing note:** `cargo test -p hashjunkie-napi` does not work for napi-rs cdylib crates — `napi_build::setup()` emits `cargo:rustc-link-lib=node` which requires `libnode` at link time. A test executable linked this way would need Node.js installed as a system library. Correctness is instead verified by (1) `cargo build` (compilation) and (2) `cargo clippy` (lint). JS-level integration tests are added in Plan 4. All workspace `cargo test` invocations in this plan use `--exclude hashjunkie-napi` to skip this crate.

- [ ] **Step 1: Update workspace Cargo.toml**

Full replacement of `Cargo.toml` (add `hashjunkie-napi` member, add `napi`/`napi-derive` workspace deps, add explicit `lto = true` to `wasm-release` profile):

```toml
[workspace]
members = ["crates/hashjunkie-core", "crates/hashjunkie-cli", "crates/hashjunkie-napi"]
resolver = "2"

[workspace.dependencies]
blake3      = "1"
clap        = { version = "4", features = ["derive"] }
crc32fast   = "1"
digest      = "0.10"
hex         = "0.4"
md-5        = "0.10"
napi        = { version = "2", features = ["napi4"] }
napi-derive = "2"
serde_json  = "1"
sha1        = "0.10"
sha2        = "0.10"
whirlpool   = "0.10"
xxhash-rust = { version = "0.8", features = ["xxh3"] }

[profile.release]
opt-level     = "z"
lto           = true
codegen-units = 1
strip         = true
panic         = "abort"

[profile.wasm-release]
inherits  = "release"
opt-level = 3
lto       = true
```

- [ ] **Step 2: Create crates/hashjunkie-napi/Cargo.toml**

```toml
[package]
name = "hashjunkie-napi"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
hashjunkie-core = { path = "../hashjunkie-core" }
napi            = { workspace = true }
napi-derive     = { workspace = true }

[build-dependencies]
napi-build = "2"
```

- [ ] **Step 3: Create crates/hashjunkie-napi/build.rs**

```rust
extern crate napi_build;

fn main() {
    napi_build::setup();
}
```

- [ ] **Step 4: Create crates/hashjunkie-napi/src/lib.rs (compiling stub)**

```rust
#![deny(clippy::all)]

use napi_derive::napi;

#[napi]
pub struct NativeHasher;
```

- [ ] **Step 5: Verify compilation**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo build -p hashjunkie-napi
```

Expected: compiles without errors. Produces `target/debug/libhashjunkie_napi.so` (Linux) or `.dylib` (macOS).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/hashjunkie-napi/
git commit -m "chore: add hashjunkie-napi crate skeleton with napi-rs wiring"
```

---

### Task 2: NativeHasher implementation

**Files:**
- Modify: `crates/hashjunkie-napi/src/lib.rs`

**Context:** The `#[napi]` attribute on a struct exposes it as a JavaScript class. `#[napi(constructor)]` on a method makes it the class constructor, callable as `new NativeHasher(...)` in JS. napi-rs automatically converts `HashMap<String, String>` return values to plain JS objects. We store `Option<MultiHasher>` (not `MultiHasher`) because `MultiHasher::finalize(self)` consumes its receiver — `Option::take()` lets us move the inner hasher out while keeping `&mut self` signature required by napi. Calling `finalize()` twice returns a napi error instead of panicking.

The `parse_algorithms` helper is a plain Rust function (no napi types) so it can be tested in isolation if a future test harness supports it.

- [ ] **Step 1: Write the full NativeHasher implementation**

Full contents of `crates/hashjunkie-napi/src/lib.rs`:

```rust
#![deny(clippy::all)]

use std::collections::HashMap;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

use hashjunkie_core::{Algorithm, MultiHasher};

fn parse_algorithms(names: Option<Vec<String>>) -> napi::Result<Vec<Algorithm>> {
    match names {
        None => Ok(Algorithm::all().to_vec()),
        Some(names) => names
            .iter()
            .map(|s| {
                s.parse::<Algorithm>()
                    .map_err(|e| napi::Error::from_reason(e.to_string()))
            })
            .collect(),
    }
}

/// A streaming multi-algorithm hasher exposed as a Node.js native class.
///
/// ```js
/// const h = new NativeHasher(['sha256', 'blake3']);
/// h.update(Buffer.from('hello'));
/// const digests = h.finalize(); // { sha256: '...', blake3: '...' }
/// ```
#[napi]
pub struct NativeHasher {
    inner: Option<MultiHasher>,
}

#[napi]
impl NativeHasher {
    /// Create a new hasher. Pass an array of algorithm name strings (e.g.
    /// `['sha256', 'blake3']`) or omit / pass `null` / `undefined` to hash
    /// with all 13 algorithms. Throws if any name is unrecognised.
    #[napi(constructor)]
    pub fn new(algorithms: Option<Vec<String>>) -> napi::Result<Self> {
        let algs = parse_algorithms(algorithms)?;
        Ok(Self {
            inner: Some(MultiHasher::new(&algs)),
        })
    }

    /// Feed a chunk of data into all active hashers.
    /// Throws if called after `finalize()`.
    #[napi]
    pub fn update(&mut self, data: Buffer) -> napi::Result<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| napi::Error::from_reason("hasher already finalized"))?
            .update(&data);
        Ok(())
    }

    /// Finalize all hashers and return a plain JS object mapping algorithm
    /// name to lowercase hex digest string. After this call, `update()` and
    /// `finalize()` will throw if called again.
    #[napi]
    pub fn finalize(&mut self) -> napi::Result<HashMap<String, String>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| napi::Error::from_reason("hasher already finalized"))?;
        Ok(inner
            .finalize()
            .into_iter()
            .map(|(alg, digest)| (alg.as_str().to_string(), digest))
            .collect())
    }
}
```

- [ ] **Step 2: Verify compilation and clippy**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo build -p hashjunkie-napi && \
  cargo clippy --workspace -- -D warnings
```

Expected: 0 errors, 0 warnings across the entire workspace.

- [ ] **Step 3: Commit**

```bash
git add crates/hashjunkie-napi/src/lib.rs
git commit -m "feat: NativeHasher napi-rs binding with update and finalize"
```

---

### Task 3: Workspace setup + hashjunkie-wasm skeleton

**Files:**
- Modify: `Cargo.toml`
- Modify: `.cargo/config.toml`
- Create: `crates/hashjunkie-wasm/Cargo.toml`
- Create: `crates/hashjunkie-wasm/src/lib.rs`

**Context:** `wasm-bindgen` generates JavaScript glue so the WASM module is callable from JS without manual memory management. We target `wasm32-unknown-unknown` (the correct target for JS-callable WASM — `wasm32-wasip1` is for WASI-runtime use cases and isn't compatible with wasm-bindgen). The `simd128` rustflag enables the WebAssembly SIMD extension, which is supported in all modern browsers and in Node.js / Bun. We use `crate-type = ["cdylib", "rlib"]`: `cdylib` produces the `.wasm` file; `rlib` lets `cargo test` compile test binaries for the pure-Rust unit tests.

- [ ] **Step 1: Add hashjunkie-wasm to workspace and workspace deps**

Modify `Cargo.toml` — replace `[workspace]` members list and add `js-sys` + `wasm-bindgen` to `[workspace.dependencies]`:

```toml
[workspace]
members = [
    "crates/hashjunkie-core",
    "crates/hashjunkie-cli",
    "crates/hashjunkie-napi",
    "crates/hashjunkie-wasm",
]
resolver = "2"

[workspace.dependencies]
blake3       = "1"
clap         = { version = "4", features = ["derive"] }
crc32fast    = "1"
digest       = "0.10"
hex          = "0.4"
js-sys       = "0.3"
md-5         = "0.10"
napi         = { version = "2", features = ["napi4"] }
napi-derive  = "2"
serde_json   = "1"
sha1         = "0.10"
sha2         = "0.10"
wasm-bindgen = "0.2"
whirlpool    = "0.10"
xxhash-rust  = { version = "0.8", features = ["xxh3"] }

[profile.release]
opt-level     = "z"
lto           = true
codegen-units = 1
strip         = true
panic         = "abort"

[profile.wasm-release]
inherits  = "release"
opt-level = 3
lto       = true
```

- [ ] **Step 2: Add wasm32-unknown-unknown SIMD flags to .cargo/config.toml**

Full `.cargo/config.toml`:

```toml
[target.wasm32-wasip1]
rustflags = ["-C", "target-feature=+simd128"]

[target.wasm32-unknown-unknown]
rustflags = ["-C", "target-feature=+simd128"]
```

- [ ] **Step 3: Create crates/hashjunkie-wasm/Cargo.toml**

```toml
[package]
name = "hashjunkie-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
hashjunkie-core = { path = "../hashjunkie-core" }
js-sys          = { workspace = true }
wasm-bindgen    = { workspace = true }
```

- [ ] **Step 4: Create crates/hashjunkie-wasm/src/lib.rs (compiling stub)**

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmHasher;
```

- [ ] **Step 5: Install the wasm32-unknown-unknown target**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  rustup target add wasm32-unknown-unknown
```

Expected: "info: component 'rust-std' for target 'wasm32-unknown-unknown' is up to date" (or installs it)

- [ ] **Step 6: Verify WASM compilation**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo build -p hashjunkie-wasm --target wasm32-unknown-unknown
```

Expected: compiles without errors. Produces `target/wasm32-unknown-unknown/debug/hashjunkie_wasm.wasm`.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock .cargo/config.toml crates/hashjunkie-wasm/
git commit -m "chore: add hashjunkie-wasm crate skeleton with wasm-bindgen wiring"
```

---

### Task 4: WasmHasher implementation

**Files:**
- Modify: `crates/hashjunkie-wasm/src/lib.rs`

**Context:** `wasm-bindgen` handles JS↔WASM marshalling. `update(&mut self, data: &[u8])` receives a `Uint8Array` view from JS — wasm-bindgen automatically passes a slice into WASM memory. For `finalize()` returning a JS object, we construct a `js_sys::Object` and use `js_sys::Reflect::set()` to set each key — this avoids adding a serde dependency. The constructor takes `JsValue` (not `Option<Vec<String>>`) to cleanly handle `null`, `undefined`, or a JS Array from the JS side.

`cargo test -p hashjunkie-wasm` **does** work on the native target because `crate-type = ["cdylib", "rlib"]` and the tests only call `parse_algorithm_names()`, which is a plain Rust function with no wasm-bindgen types. The wasm-bindgen types (`JsValue`, `Object`, `Reflect`) are not called from test code.

- [ ] **Step 1: Write failing unit tests**

Add the test module to `crates/hashjunkie-wasm/src/lib.rs` (append after the existing stub):

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmHasher;

#[cfg(test)]
mod tests {
    use hashjunkie_core::Algorithm;

    #[test]
    fn parse_none_returns_all_13_algorithms() {
        let algs = super::parse_algorithm_names(None).unwrap();
        assert_eq!(algs.len(), 13);
    }

    #[test]
    fn parse_two_known_names() {
        let names = vec!["sha256".to_string(), "blake3".to_string()];
        let algs = super::parse_algorithm_names(Some(names)).unwrap();
        assert_eq!(algs.len(), 2);
        assert!(algs.contains(&Algorithm::Sha256));
        assert!(algs.contains(&Algorithm::Blake3));
    }

    #[test]
    fn parse_unknown_name_returns_error() {
        let names = vec!["bogus".to_string()];
        assert!(super::parse_algorithm_names(Some(names)).is_err());
    }

    #[test]
    fn sha256_of_abc_matches_known_vector() {
        use hashjunkie_core::MultiHasher;
        let mut h = MultiHasher::new(&[Algorithm::Sha256]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(
            digests[&Algorithm::Sha256],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo test -p hashjunkie-wasm
```

Expected: FAIL — `error[E0425]: cannot find function 'parse_algorithm_names' in module 'super'`

- [ ] **Step 3: Implement WasmHasher**

Full replacement of `crates/hashjunkie-wasm/src/lib.rs`:

```rust
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

use hashjunkie_core::{Algorithm, MultiHasher};

fn parse_algorithm_names(names: Option<Vec<String>>) -> Result<Vec<Algorithm>, String> {
    match names {
        None => Ok(Algorithm::all().to_vec()),
        Some(names) => names
            .iter()
            .map(|s| s.parse::<Algorithm>().map_err(|e| e.to_string()))
            .collect(),
    }
}

/// A streaming multi-algorithm hasher exposed to JavaScript via WebAssembly.
///
/// ```js
/// const h = new WasmHasher(['sha256', 'blake3']);
/// h.update(new Uint8Array([104, 101, 108, 108, 111]));
/// const digests = h.finalize(); // { sha256: '...', blake3: '...' }
/// ```
#[wasm_bindgen]
pub struct WasmHasher {
    inner: Option<MultiHasher>,
}

#[wasm_bindgen]
impl WasmHasher {
    /// Create a new hasher. Pass an array of algorithm name strings (e.g.
    /// `['sha256', 'blake3']`) or omit / pass `null` / `undefined` to hash
    /// with all 13 algorithms. Throws a `TypeError` if any name is unrecognised.
    #[wasm_bindgen(constructor)]
    pub fn new(algorithms: JsValue) -> Result<WasmHasher, JsValue> {
        let names: Option<Vec<String>> = if algorithms.is_null() || algorithms.is_undefined() {
            None
        } else {
            let arr = js_sys::Array::from(&algorithms);
            let mut names = Vec::with_capacity(arr.length() as usize);
            for val in arr.iter() {
                let s = val
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("algorithm name must be a string"))?;
                names.push(s);
            }
            Some(names)
        };

        let algs = parse_algorithm_names(names).map_err(|e| JsValue::from_str(&e))?;
        Ok(WasmHasher {
            inner: Some(MultiHasher::new(&algs)),
        })
    }

    /// Feed a chunk of data into all active hashers.
    /// Throws if called after `finalize()`.
    pub fn update(&mut self, data: &[u8]) -> Result<(), JsValue> {
        self.inner
            .as_mut()
            .ok_or_else(|| JsValue::from_str("hasher already finalized"))?
            .update(data);
        Ok(())
    }

    /// Finalize all hashers and return a plain JS object mapping algorithm
    /// name to lowercase hex digest string. After this call, `update()` and
    /// `finalize()` will throw if called again.
    pub fn finalize(&mut self) -> Result<JsValue, JsValue> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| JsValue::from_str("hasher already finalized"))?;

        let digests = inner.finalize();
        let obj = Object::new();
        for (alg, digest) in digests {
            Reflect::set(
                &obj,
                &JsValue::from_str(alg.as_str()),
                &JsValue::from_str(&digest),
            )
            .map_err(|e| e)?;
        }
        Ok(obj.into())
    }
}

#[cfg(test)]
mod tests {
    use hashjunkie_core::Algorithm;

    #[test]
    fn parse_none_returns_all_13_algorithms() {
        let algs = super::parse_algorithm_names(None).unwrap();
        assert_eq!(algs.len(), 13);
    }

    #[test]
    fn parse_two_known_names() {
        let names = vec!["sha256".to_string(), "blake3".to_string()];
        let algs = super::parse_algorithm_names(Some(names)).unwrap();
        assert_eq!(algs.len(), 2);
        assert!(algs.contains(&Algorithm::Sha256));
        assert!(algs.contains(&Algorithm::Blake3));
    }

    #[test]
    fn parse_unknown_name_returns_error() {
        let names = vec!["bogus".to_string()];
        assert!(super::parse_algorithm_names(Some(names)).is_err());
    }

    #[test]
    fn sha256_of_abc_matches_known_vector() {
        use hashjunkie_core::MultiHasher;
        let mut h = MultiHasher::new(&[Algorithm::Sha256]);
        h.update(b"abc");
        let digests = h.finalize();
        assert_eq!(
            digests[&Algorithm::Sha256],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo test -p hashjunkie-wasm
```

Expected: 4 tests pass

- [ ] **Step 5: Verify WASM release build**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo build -p hashjunkie-wasm --target wasm32-unknown-unknown --profile wasm-release
```

Expected: produces `target/wasm32-unknown-unknown/wasm-release/hashjunkie_wasm.wasm`

- [ ] **Step 6: Run clippy across the full workspace**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo clippy --workspace -- -D warnings
```

Expected: 0 warnings

- [ ] **Step 7: Commit**

```bash
git add crates/hashjunkie-wasm/src/lib.rs
git commit -m "feat: WasmHasher wasm-bindgen binding with update and finalize"
```

---

### Task 5: Per-platform npm package scaffolding

**Files:**
- Create: `npm/hashjunkie-linux-x64-gnu/package.json`
- Create: `npm/hashjunkie-linux-arm64-gnu/package.json`
- Create: `npm/hashjunkie-darwin-x64/package.json`
- Create: `npm/hashjunkie-darwin-arm64/package.json`
- Create: `npm/hashjunkie-win32-x64-msvc/package.json`

**Context:** These five packages are declared as `optionalDependencies` in the main `hashjunkie` npm package (Plan 4). Package managers use the `os` and `cpu` fields to install only the correct platform package. The `main` field points to the `.node` file that will be placed here by the CI release workflow — the binary does not exist in source control. We use the `@hashjunkie` npm scope for all platform packages.

The naming convention (`linux-x64-gnu`, `darwin-arm64`, etc.) matches napi-rs's default target triple naming, which CI uses when placing built artifacts.

- [ ] **Step 1: Create npm/hashjunkie-linux-x64-gnu/package.json**

```json
{
  "name": "@hashjunkie/linux-x64-gnu",
  "version": "0.1.0",
  "description": "Prebuilt hashjunkie native addon for Linux x64 (glibc)",
  "os": ["linux"],
  "cpu": ["x64"],
  "main": "hashjunkie.linux-x64-gnu.node",
  "files": ["hashjunkie.linux-x64-gnu.node"],
  "license": "MIT"
}
```

- [ ] **Step 2: Create npm/hashjunkie-linux-arm64-gnu/package.json**

```json
{
  "name": "@hashjunkie/linux-arm64-gnu",
  "version": "0.1.0",
  "description": "Prebuilt hashjunkie native addon for Linux arm64 (glibc)",
  "os": ["linux"],
  "cpu": ["arm64"],
  "main": "hashjunkie.linux-arm64-gnu.node",
  "files": ["hashjunkie.linux-arm64-gnu.node"],
  "license": "MIT"
}
```

- [ ] **Step 3: Create npm/hashjunkie-darwin-x64/package.json**

```json
{
  "name": "@hashjunkie/darwin-x64",
  "version": "0.1.0",
  "description": "Prebuilt hashjunkie native addon for macOS x64",
  "os": ["darwin"],
  "cpu": ["x64"],
  "main": "hashjunkie.darwin-x64.node",
  "files": ["hashjunkie.darwin-x64.node"],
  "license": "MIT"
}
```

- [ ] **Step 4: Create npm/hashjunkie-darwin-arm64/package.json**

```json
{
  "name": "@hashjunkie/darwin-arm64",
  "version": "0.1.0",
  "description": "Prebuilt hashjunkie native addon for macOS arm64 (Apple Silicon)",
  "os": ["darwin"],
  "cpu": ["arm64"],
  "main": "hashjunkie.darwin-arm64.node",
  "files": ["hashjunkie.darwin-arm64.node"],
  "license": "MIT"
}
```

- [ ] **Step 5: Create npm/hashjunkie-win32-x64-msvc/package.json**

```json
{
  "name": "@hashjunkie/win32-x64-msvc",
  "version": "0.1.0",
  "description": "Prebuilt hashjunkie native addon for Windows x64 (MSVC)",
  "os": ["win32"],
  "cpu": ["x64"],
  "main": "hashjunkie.win32-x64-msvc.node",
  "files": ["hashjunkie.win32-x64-msvc.node"],
  "license": "MIT"
}
```

- [ ] **Step 6: Run full workspace test suite (regression check)**

```bash
CARGO_HOME=/home/per/.cargo PATH=/home/per/.cargo/bin:$PATH \
  cargo test --workspace --exclude hashjunkie-napi
```

Expected: all 105 tests pass (101 existing + 4 new wasm unit tests from Task 4), 0 failures.
Note: `hashjunkie-napi` is excluded because its cdylib link args require `libnode` at link time, which is only available when Node.js loads the addon at runtime. The napi binding is tested via JS integration tests in Plan 4.

- [ ] **Step 7: Commit**

```bash
git add npm/
git commit -m "chore: scaffold per-platform npm package stubs for native addon distribution"
```
