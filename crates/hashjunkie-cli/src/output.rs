use std::collections::BTreeMap;

pub struct FileJsonEntry<'a> {
    pub path: &'a str,
    pub name: &'a str,
    pub size: u64,
    pub mod_time: String,
    pub digests: &'a BTreeMap<String, String>,
}

/// Formats a digest map as a compact JSON object with sorted keys.
/// Example: `{"md5":"900150...","sha256":"ba7816..."}`
pub fn format_as_json_object(digests: &BTreeMap<String, String>) -> String {
    // Serialize BTreeMap directly so sorted key order is structurally guaranteed
    // by the BTreeMap type, not by an implicit serde_json feature flag.
    serde_json::to_string(digests).expect("BTreeMap<String, String> always serializes")
}

/// Formats a digest map as `algo: hex\n` lines, sorted by algorithm name.
/// Example: `"md5: 900150...\nsha256: ba7816...\n"`
pub fn format_as_hex_lines(digests: &BTreeMap<String, String>) -> String {
    digests.iter().map(|(k, v)| format!("{k}: {v}\n")).collect()
}

fn file_entry_value(entry: &FileJsonEntry<'_>) -> serde_json::Value {
    let hashes = serde_json::to_value(entry.digests).expect("BTreeMap<String, String> always serializes");
    let mut obj = serde_json::Map::new();
    obj.insert("Hashes".to_string(), hashes);
    obj.insert(
        "ModTime".to_string(),
        serde_json::Value::String(entry.mod_time.clone()),
    );
    obj.insert(
        "Name".to_string(),
        serde_json::Value::String(entry.name.to_string()),
    );
    obj.insert(
        "Path".to_string(),
        serde_json::Value::String(entry.path.to_string()),
    );
    obj.insert("Size".to_string(), serde_json::Value::Number(entry.size.into()));
    serde_json::Value::Object(obj)
}

/// Formats multiple file entries as a JSON array.
/// Each element matches the `rclone lsjson --hash` object shape.
pub fn format_as_file_json(files: &[FileJsonEntry<'_>]) -> String {
    let array: Vec<serde_json::Value> = files
        .iter()
        .map(file_entry_value)
        .collect();
    serde_json::to_string(&serde_json::Value::Array(array)).expect("file entries always serialize")
}

/// Formats stdin JSON output using the same metadata-bearing shape as file mode.
pub fn format_as_stdin_json(entry: &FileJsonEntry<'_>) -> String {
    serde_json::to_string(&file_entry_value(entry)).expect("stdin entry always serializes")
}

/// Formats multiple (path, digests) pairs as grouped text blocks.
/// Each block: file path, then `algo: hex\n` lines. Blocks separated by blank line.
pub fn format_as_file_hex(files: &[(&str, &BTreeMap<String, String>)]) -> String {
    files
        .iter()
        .map(|(path, digests)| format!("{path}\n{}", format_as_hex_lines(digests)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert(
            "md5".to_string(),
            "900150983cd24fb0d6963f7d28e17f72".to_string(),
        );
        m.insert(
            "sha256".to_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_string(),
        );
        m
    }

    #[test]
    fn json_object_is_compact_and_sorted() {
        let json = format_as_json_object(&sample());
        assert_eq!(
            json,
            r#"{"md5":"900150983cd24fb0d6963f7d28e17f72","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}"#
        );
    }

    #[test]
    fn hex_lines_are_sorted_with_newline_terminator() {
        let hex = format_as_hex_lines(&sample());
        assert_eq!(
            hex,
            "md5: 900150983cd24fb0d6963f7d28e17f72\nsha256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n"
        );
    }

    #[test]
    fn file_json_has_hashes_name_path_fields() {
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
    fn file_json_two_files_produces_array_of_length_2() {
        let d = sample();
        let json = format_as_file_json(&[
            FileJsonEntry {
                path: "a.bin",
                name: "a.bin",
                size: 1,
                mod_time: "2026-04-13T08:26:13Z".to_string(),
                digests: &d,
            },
            FileJsonEntry {
                path: "b.bin",
                name: "b.bin",
                size: 2,
                mod_time: "2026-04-13T08:26:14Z".to_string(),
                digests: &d,
            },
        ]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[1]["Path"], "b.bin");
    }

    #[test]
    fn file_json_name_is_last_path_segment() {
        let d = sample();
        let json = format_as_file_json(&[FileJsonEntry {
            path: "/deep/nested/thing.dat",
            name: "thing.dat",
            size: 9,
            mod_time: "2026-04-13T08:26:13Z".to_string(),
            digests: &d,
        }]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["Name"], "thing.dat");
        assert_eq!(parsed[0]["Path"], "/deep/nested/thing.dat");
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

    #[test]
    fn file_hex_has_path_header_and_sorted_lines() {
        let d = sample();
        let hex = format_as_file_hex(&[("dir/file1.bin", &d)]);
        assert_eq!(
            hex,
            "dir/file1.bin\nmd5: 900150983cd24fb0d6963f7d28e17f72\nsha256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n"
        );
    }

    #[test]
    fn file_hex_two_files_separated_by_blank_line() {
        let d = sample();
        let hex = format_as_file_hex(&[("a.bin", &d), ("b.bin", &d)]);
        assert!(hex.contains("\n\n"));
        assert!(hex.starts_with("a.bin\n"));
        assert!(hex.contains("\nb.bin\n"));
    }
}
