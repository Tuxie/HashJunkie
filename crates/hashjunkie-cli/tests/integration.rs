use std::io::Write;
use std::process::Command;

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
    assert!(stdout.contains("0.1.0"));
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
fn stdin_with_single_algorithm_outputs_only_that_algorithm() {
    let output = run_with_stdin(&["-a", "sha256"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("sha256"));
    assert!(!stdout.contains("md5"));
}

#[test]
fn stdin_with_two_algorithms_json_output_is_sorted() {
    let output = run_with_stdin(&["-a", "sha256,md5"], b"abc");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        r#"{"md5":"900150983cd24fb0d6963f7d28e17f72","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}"#
    );
}
