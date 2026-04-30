# lsjson Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `Size` and `ModTime` to `hashjunkie` JSON output for both file mode and stdin mode, matching the `rclone lsjson --hash` object shape while keeping hex output unchanged.

**Architecture:** Introduce a small CLI-only JSON entry model that carries metadata plus digests, then route file mode and stdin mode through that model only when `--format json` is selected. File mode reads filesystem metadata, while stdin mode counts bytes read and stamps the current time after EOF.

**Tech Stack:** Rust, `clap`, `serde_json`, `std::fs::Metadata`, `std::time::SystemTime`, `chrono`

---

## File Structure

- Modify: `crates/hashjunkie-cli/Cargo.toml`
- Modify: `crates/hashjunkie-cli/src/output.rs`
- Modify: `crates/hashjunkie-cli/src/main.rs`
- Modify: `crates/hashjunkie-cli/tests/integration.rs`

### Task 1: Add failing formatter tests for metadata-bearing JSON entries

**Files:**
- Modify: `crates/hashjunkie-cli/src/output.rs`
- Test: `crates/hashjunkie-cli/src/output.rs`

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn file_json_includes_size_and_mod_time_fields() {
        let d = sample();
        let json = format_as_file_json(&[FileJsonEntry {
            path: "dir/file1.bin",
            name: "file1.bin",
            size: 3,
            mod_time: "2026-04-13T08:26:13.274435233Z".to_string(),
            digests: &d,
        }]);
        assert_eq!(
            json,
            r#"[{"Hashes":{"md5":"900150983cd24fb0d6963f7d28e17f72","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"},"ModTime":"2026-04-13T08:26:13.274435233Z","Name":"file1.bin","Path":"dir/file1.bin","Size":3}]"#
        );
    }

    #[test]
    fn stdin_json_includes_metadata_fields() {
        let d = sample();
        let json = format_as_stdin_json(&FileJsonEntry {
            path: "-",
            name: "-",
            size: 3,
            mod_time: "2026-04-14T00:00:00Z".to_string(),
            digests: &d,
        });
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["Path"], "-");
        assert_eq!(parsed["Name"], "-");
        assert_eq!(parsed["Size"], 3);
        assert_eq!(parsed["ModTime"], "2026-04-14T00:00:00Z");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hashjunkie-cli output::tests::file_json_includes_size_and_mod_time_fields output::tests::stdin_json_includes_metadata_fields`
Expected: FAIL with missing `FileJsonEntry` and/or missing metadata fields in JSON output.

- [ ] **Step 3: Write minimal implementation**

```rust
pub struct FileJsonEntry<'a> {
    pub path: &'a str,
    pub name: &'a str,
    pub size: u64,
    pub mod_time: String,
    pub digests: &'a BTreeMap<String, String>,
}

fn file_entry_value(entry: &FileJsonEntry<'_>) -> serde_json::Value {
    let hashes = serde_json::to_value(entry.digests)
        .expect("BTreeMap<String, String> always serializes");
    let mut obj = serde_json::Map::new();
    obj.insert("Hashes".to_string(), hashes);
    obj.insert(
        "ModTime".to_string(),
        serde_json::Value::String(entry.mod_time.clone()),
    );
    obj.insert("Name".to_string(), serde_json::Value::String(entry.name.to_string()));
    obj.insert("Path".to_string(), serde_json::Value::String(entry.path.to_string()));
    obj.insert("Size".to_string(), serde_json::Value::Number(entry.size.into()));
    serde_json::Value::Object(obj)
}

pub fn format_as_file_json(files: &[FileJsonEntry<'_>]) -> String {
    let array: Vec<serde_json::Value> = files.iter().map(file_entry_value).collect();
    serde_json::to_string(&serde_json::Value::Array(array)).expect("file entries always serialize")
}

pub fn format_as_stdin_json(entry: &FileJsonEntry<'_>) -> String {
    serde_json::to_string(&file_entry_value(entry)).expect("stdin entry always serializes")
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hashjunkie-cli output::tests::file_json_includes_size_and_mod_time_fields output::tests::stdin_json_includes_metadata_fields`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/hashjunkie-cli/src/output.rs
git commit -m "refactor: add metadata-aware json formatter"
```

### Task 2: Add failing CLI integration tests for file mode and stdin mode metadata

**Files:**
- Modify: `crates/hashjunkie-cli/tests/integration.rs`
- Test: `crates/hashjunkie-cli/tests/integration.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn stdin_json_includes_size_name_path_and_mod_time() {
    let output = run_with_stdin(&["-a", "sha256"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["Path"], "-");
    assert_eq!(parsed["Name"], "-");
    assert_eq!(parsed["Size"], 3);
    let mod_time = parsed["ModTime"].as_str().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(mod_time).is_ok());
    assert_eq!(
        parsed["Hashes"]["sha256"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn file_mode_json_includes_fixture_size_and_parseable_mod_time() {
    let output = bin().arg(FIXTURE).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed[0]["Size"], std::fs::metadata(FIXTURE).unwrap().len());
    let mod_time = parsed[0]["ModTime"].as_str().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(mod_time).is_ok());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hashjunkie-cli --test integration stdin_json_includes_size_name_path_and_mod_time file_mode_json_includes_fixture_size_and_parseable_mod_time`
Expected: FAIL because current JSON output does not include `Size` or `ModTime`.

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Debug)]
struct CountingReader<R> {
    inner: R,
    bytes_read: u64,
}

impl<R: Read> CountingReader<R> {
    fn new(inner: R) -> Self {
        Self { inner, bytes_read: 0 }
    }
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.bytes_read += n as u64;
        Ok(n)
    }
}

fn now_rfc3339_utc() -> String {
    chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now())
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hashjunkie-cli --test integration stdin_json_includes_size_name_path_and_mod_time file_mode_json_includes_fixture_size_and_parseable_mod_time`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/hashjunkie-cli/tests/integration.rs crates/hashjunkie-cli/src/main.rs crates/hashjunkie-cli/Cargo.toml
git commit -m "feat: add size and modtime to json output"
```

### Task 3: Wire metadata-aware JSON output into CLI code paths without changing hex output

**Files:**
- Modify: `crates/hashjunkie-cli/src/main.rs`
- Modify: `crates/hashjunkie-cli/src/output.rs`
- Modify: `crates/hashjunkie-cli/Cargo.toml`
- Test: `crates/hashjunkie-cli/tests/integration.rs`

- [ ] **Step 1: Write the failing behavior-focused assertions**

```rust
#[test]
fn stdin_hex_format_does_not_include_metadata_labels() {
    let output = run_with_stdin(&["--format", "hex", "-a", "sha256"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("ModTime"));
    assert!(!stdout.contains("Size"));
}

#[test]
fn file_mode_hex_format_does_not_include_metadata_labels() {
    let output = bin().args(["--format", "hex", FIXTURE]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("ModTime"));
    assert!(!stdout.contains("Size"));
}
```

- [ ] **Step 2: Run test to verify it fails only if hex output regresses**

Run: `cargo test -p hashjunkie-cli --test integration stdin_hex_format_does_not_include_metadata_labels file_mode_hex_format_does_not_include_metadata_labels`
Expected: PASS now and after implementation. This guards the non-goal while Task 3 is completed.

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/hashjunkie-cli/Cargo.toml
[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }

// crates/hashjunkie-cli/src/main.rs
fn file_name_for_path(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

fn system_time_to_rfc3339(value: std::time::SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(value)
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

// In run_stdin JSON branch:
let mut counting = CountingReader::new(io::stdin());
let digests = hash_reader(&mut counting, algorithms)?;
let entry = output::FileJsonEntry {
    path: "-",
    name: "-",
    size: counting.bytes_read,
    mod_time: now_rfc3339_utc(),
    digests: &digests,
};

// In run_files JSON branch:
let metadata = std::fs::metadata(path)?;
let entry = output::FileJsonEntry {
    path,
    name: file_name_for_path(path),
    size: metadata.len(),
    mod_time: system_time_to_rfc3339(metadata.modified()?),
    digests: &digests,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p hashjunkie-cli`
Expected: PASS all unit and integration tests in `hashjunkie-cli`.

- [ ] **Step 5: Commit**

```bash
git add crates/hashjunkie-cli/Cargo.toml crates/hashjunkie-cli/src/main.rs crates/hashjunkie-cli/src/output.rs crates/hashjunkie-cli/tests/integration.rs
git commit -m "feat: include size and modtime in json output"
```

## Self-Review

- Spec coverage: file-mode metadata, stdin metadata, unchanged hex output, and RFC 3339 UTC parsing are all covered by Tasks 1 through 3.
- Placeholder scan: no `TODO`, `TBD`, or unresolved references remain.
- Type consistency: the plan consistently uses `FileJsonEntry`, `CountingReader`, `format_as_file_json`, and `format_as_stdin_json`.
