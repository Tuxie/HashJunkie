// Functions are pub API wired into main in a later task; suppress dead_code until then.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

/// Formats a digest map as a compact JSON object with sorted keys.
/// Example: `{"md5":"900150...","sha256":"ba7816..."}`
pub fn format_as_json_object(digests: &BTreeMap<String, String>) -> String {
    let obj: serde_json::Map<String, serde_json::Value> = digests
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    serde_json::to_string(&serde_json::Value::Object(obj))
        .expect("BTreeMap<String, String> always serializes")
}

/// Formats a digest map as `algo: hex\n` lines, sorted by algorithm name.
/// Example: `"md5: 900150...\nsha256: ba7816...\n"`
pub fn format_as_hex_lines(digests: &BTreeMap<String, String>) -> String {
    digests.iter().map(|(k, v)| format!("{k}: {v}\n")).collect()
}

/// Formats multiple (path, digests) pairs as a JSON array.
/// Each element: `{"Hashes":{...},"Name":"filename","Path":"path/as/given"}`.
pub fn format_as_file_json(files: &[(&str, &BTreeMap<String, String>)]) -> String {
    let array: Vec<serde_json::Value> = files
        .iter()
        .map(|(path, digests)| {
            let name = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            let hashes: serde_json::Map<String, serde_json::Value> = digests
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            // Fields in alphabetical order: Hashes, Name, Path
            let mut obj = serde_json::Map::new();
            obj.insert("Hashes".to_string(), serde_json::Value::Object(hashes));
            obj.insert(
                "Name".to_string(),
                serde_json::Value::String(name.to_string()),
            );
            obj.insert(
                "Path".to_string(),
                serde_json::Value::String((*path).to_string()),
            );
            serde_json::Value::Object(obj)
        })
        .collect();
    serde_json::to_string(&serde_json::Value::Array(array)).expect("file entries always serialize")
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
        let json = format_as_file_json(&[("dir/file1.bin", &d)]);
        assert_eq!(
            json,
            r#"[{"Hashes":{"md5":"900150983cd24fb0d6963f7d28e17f72","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"},"Name":"file1.bin","Path":"dir/file1.bin"}]"#
        );
    }

    #[test]
    fn file_json_two_files_produces_array_of_length_2() {
        let d = sample();
        let json = format_as_file_json(&[("a.bin", &d), ("b.bin", &d)]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[1]["Path"], "b.bin");
    }

    #[test]
    fn file_json_name_is_last_path_segment() {
        let d = sample();
        let json = format_as_file_json(&[("/deep/nested/thing.dat", &d)]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["Name"], "thing.dat");
        assert_eq!(parsed[0]["Path"], "/deep/nested/thing.dat");
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
