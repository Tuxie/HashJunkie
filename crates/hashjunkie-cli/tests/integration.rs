use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hashjunkie"))
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
