# Hashjunkie lsjson Metadata Design

## Goal

Make `hashjunkie` JSON output include `Size` and `ModTime` in the same shape as `rclone lsjson --hash`, while leaving hex output unchanged.

## Scope

This change applies to JSON output only.

- File mode (`hashjunkie <file>`) must emit `Size` from filesystem metadata and `ModTime` from the file modification timestamp.
- Stdin mode (`hashjunkie` reading from standard input) must emit `Size` as the number of bytes consumed and `ModTime` as the current timestamp captured after EOF.
- Existing `Hashes`, `Name`, and `Path` fields remain.
- Hex output format does not change.

## Output Shape

File mode continues to emit a JSON array with one object per input path. Each object contains:

- `Path`: input path string as provided to the CLI
- `Name`: last path segment
- `Size`: integer byte count
- `ModTime`: UTC timestamp encoded as RFC 3339 with fractional seconds and `Z`
- `Hashes`: sorted object of algorithm name to lowercase hex digest

Stdin mode currently emits only the hashes object. After this change, stdin JSON output becomes a single object with the same metadata fields:

- `Path`: `"-"`
- `Name`: `"-"`
- `Size`: total bytes read from stdin
- `ModTime`: timestamp captured immediately after EOF
- `Hashes`: sorted object of algorithm name to lowercase hex digest

Using `"-"` for stdin keeps the shape explicit without pretending stdin has a filesystem path.

## Implementation

Introduce a small metadata-bearing result type in the CLI layer instead of passing only `(path, digests)` pairs into JSON formatting.

- File mode:
  - collect `std::fs::Metadata` before hashing
  - use `metadata.len()` for `Size`
  - use `metadata.modified()` for `ModTime`
- Stdin mode:
  - wrap the input reader in a counting adapter
  - after `hash_reader` finishes, capture `SystemTime::now()`
  - format the counted byte total and timestamp into the JSON output object
- Timestamp formatting:
  - convert `SystemTime` to `chrono::DateTime<Utc>`
  - serialize with RFC 3339 configured to use `Z` and fractional seconds

## Error Handling

- File mode keeps current partial-success behavior: successful file entries still print if some files fail.
- If file metadata lookup fails for a path, treat that file as failed and report the error on stderr.
- Stdin mode keeps current read-error handling and exits non-zero on read failure.

## Testing

Write tests first.

- Unit tests for JSON formatting to assert `Size` and `ModTime` fields are present in the expected structure.
- Integration test for file mode to assert:
  - `Size` matches `std::fs::metadata(FIXTURE).len()`
  - `ModTime` parses as RFC 3339 UTC
- Integration test for stdin mode to assert:
  - `Size` equals input byte length
  - `Path` and `Name` equal `"-"`
  - `ModTime` parses as RFC 3339 UTC
- Existing hex-format tests remain unchanged.

## Non-Goals

- No change to hex output.
- No attempt to guarantee byte-for-byte identical timestamp precision to every `rclone` build or platform. The contract is RFC 3339 UTC with fractional seconds and `Z`.
