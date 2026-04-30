use std::io::Write;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hashjunkie"))
}

fn run_with_stdin(args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = bin()
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn help_exits_zero_and_mentions_binary_name() {
    let output = bin().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hashjunkie"));
}

#[test]
fn version_exits_zero_and_contains_version() {
    let output = bin().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_algorithm_exits_one_and_stderr_contains_message() {
    let output = bin().args(["-a", "bogus"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("unknown algorithm: bogus"),
        "unexpected stderr: {stderr}"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn stdin_json_default_contains_sha256_and_md5_for_abc() {
    let output = run_with_stdin(&[], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(
        r#""sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad""#
    ));
    assert!(stdout.contains(r#""md5":"900150983cd24fb0d6963f7d28e17f72""#));
    assert!(!stdout.contains("whirlpool"));
}

#[test]
fn stdin_hex_format_contains_sha256_line() {
    let output = run_with_stdin(&["--format", "hex"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("sha256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
}

#[test]
fn stdin_short_format_hex_contains_sha256_line() {
    let output = run_with_stdin(&["-f", "hex"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("sha256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
}

#[test]
fn stdin_with_single_algorithm_outputs_only_that_algorithm() {
    let output = run_with_stdin(&["-a", "sha256"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("sha256"));
    assert!(!stdout.contains("md5"));
}

#[test]
fn stdin_hex_flag_outputs_raw_digest_bytes_as_lowercase_hex() {
    let output = run_with_stdin(&["--hex", "-a", "cidv1"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        parsed["Hashes"]["cidv1"],
        "01551220ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn hashes_only_hex_flag_uses_requested_order() {
    let output = run_with_stdin(&["-1", "--hex", "-a", "aich,cidv1"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim_end(),
        "a9993e364706816aba3e25717850c26c9cd0d89d 01551220ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn stdin_with_two_algorithms_json_output_is_sorted() {
    let output = run_with_stdin(&["-a", "sha256,md5"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["Path"], "-");
    assert_eq!(parsed["Name"], "-");
    assert_eq!(parsed["Size"], 3);
    let mod_time = parsed["ModTime"].as_str().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(mod_time).is_ok());
    assert_eq!(parsed["Hashes"]["md5"], "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        parsed["Hashes"]["sha256"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/hashjunkie-core/tests/fixtures/small.bin"
);

fn write_temp_file(name: &str, contents: &[u8]) -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("hashjunkie-{name}-{unique}"));
    std::fs::write(&path, contents).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn file_mode_default_18_hashes_correct_for_fixture() {
    let output = bin().arg(FIXTURE).output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let hashes = &parsed[0]["Hashes"];
    assert_eq!(hashes["aich"], "LMAGNHCIBVOP7PP2RPN2TFLBCYHS2G3X");
    assert_eq!(
        hashes["blake3"],
        "882179b8dbccd285cda241d968cfcccb3156c5edac2fa3761bb6eda7ff8cb172"
    );
    assert_eq!(
        hashes["btv2"],
        "785b0751fc2c53dc14a4ce3d800e69ef9ce1009eb327ccf458afe09c242c26c9"
    );
    assert_eq!(
        hashes["cidv0"],
        "bafkreidylmdvd7bmkpobjjgohwaa42ppttqqbhvte7gpiwfp4cocilbgze"
    );
    assert_eq!(
        hashes["cidv1"],
        "bafkreidylmdvd7bmkpobjjgohwaa42ppttqqbhvte7gpiwfp4cocilbgze"
    );
    assert_eq!(hashes["crc32"], "b70b4c26");
    assert_eq!(
        hashes["dropbox"],
        "05fe36f555179feb8712eadb2a1cadac8c3c7378859f8dbeaa8a6ea224ea3658"
    );
    assert_eq!(hashes["ed2k"], "5ae257c47e9be1243ee32aabe408fb6b");
    assert_eq!(
        hashes["hidrive"],
        "5b00669c480d5cffbdfa8bdba99561160f2d1b77"
    );
    assert_eq!(hashes["mailru"], "2b4639914e8e0e8f99d2a90a23801c7a87a089c1");
    assert_eq!(hashes["md5"], "b2ea9f7fcea831a4a63b213f41a8855b");
    assert_eq!(
        hashes["quickxor"],
        "87b86bd9d6c26b264241847d28ac65c03b93e142"
    );
    assert_eq!(hashes["sha1"], "5b00669c480d5cffbdfa8bdba99561160f2d1b77");
    assert_eq!(
        hashes["sha256"],
        "785b0751fc2c53dc14a4ce3d800e69ef9ce1009eb327ccf458afe09c242c26c9"
    );
    assert_eq!(
        hashes["sha512"],
        "37f652be867f28ed033269cbba201af2112c2b3fd334a89fd2f757938ddee815787cc61d6e24a8a33340d0f7e86ffc058816b88530766ba6e231620a130b566c"
    );
    assert_eq!(hashes["tiger"], "4OQY25UN2XHIDQPV5U6BXAZ47INUCYGIBK7LFNI");
    assert!(hashes["whirlpool"].is_null());
    assert_eq!(hashes["xxh128"], "83885e853bb6640ca870f92984398d22");
    assert_eq!(hashes["xxh3"], "a870f92984398d22");
}

#[test]
fn file_mode_explicit_whirlpool_hashes_fixture() {
    let output = bin().args(["-a", "whirlpool", FIXTURE]).output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let hashes = &parsed[0]["Hashes"];
    assert_eq!(
        hashes["whirlpool"],
        "d606b7f44bd288759f8869d880d9d4a2f159d739005e72d00f93b814e8c04e657f40c838e4d6f9030a8c9e0308a4e3b450246250243b2f09e09fa5a24761e26b"
    );
    assert_eq!(hashes.as_object().unwrap().len(), 1);
}

#[test]
fn file_mode_two_identical_args_produce_array_of_length_2() {
    let output = bin().arg(FIXTURE).arg(FIXTURE).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    for entry in arr {
        assert_eq!(
            entry["Hashes"]["sha256"],
            "785b0751fc2c53dc14a4ce3d800e69ef9ce1009eb327ccf458afe09c242c26c9"
        );
        assert_eq!(entry["Size"], std::fs::metadata(FIXTURE).unwrap().len());
        let mod_time = entry["ModTime"].as_str().unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(mod_time).is_ok());
    }
}

#[test]
fn file_mode_hex_format_contains_path_and_sha256() {
    let output = bin().args(["--format", "hex", FIXTURE]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(FIXTURE));
    assert!(
        stdout.contains("sha256: 785b0751fc2c53dc14a4ce3d800e69ef9ce1009eb327ccf458afe09c242c26c9")
    );
}

#[test]
fn file_mode_line_format_outputs_requested_hashes_size_and_path() {
    let output = bin()
        .args(["-f", "line", "-a", "blake3,sha1,md5", FIXTURE])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim_end(),
        format!(
            "{} {} {} {} {}",
            "882179b8dbccd285cda241d968cfcccb3156c5edac2fa3761bb6eda7ff8cb172",
            "5b00669c480d5cffbdfa8bdba99561160f2d1b77",
            "b2ea9f7fcea831a4a63b213f41a8855b",
            std::fs::metadata(FIXTURE).unwrap().len(),
            FIXTURE
        )
    );
}

#[test]
fn hashes_only_short_flag_outputs_requested_hashes_for_first_file_only() {
    let first = write_temp_file("first", b"abc");
    let second = write_temp_file("second", b"def");
    let output = bin()
        .args(["-1a", "blake3,sha1", &first, &second])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim_end(),
        "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85 a9993e364706816aba3e25717850c26c9cd0d89d"
    );
}

#[test]
fn file_mode_nonexistent_file_exits_one_stderr_has_path_stdout_empty() {
    let output = bin()
        .arg("/tmp/does_not_exist_hashjunkie_test.bin")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("No such file") || stderr.contains("cannot find"),
        "unexpected stderr: {stderr}"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn file_mode_one_good_one_bad_exits_one_stdout_has_good_result() {
    let output = bin()
        .arg(FIXTURE)
        .arg("/tmp/does_not_exist_hashjunkie_test.bin")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("No such file") || stderr.contains("cannot find"),
        "unexpected stderr: {stderr}"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(
        parsed[0]["Hashes"]["sha256"],
        "785b0751fc2c53dc14a4ce3d800e69ef9ce1009eb327ccf458afe09c242c26c9"
    );
}

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
