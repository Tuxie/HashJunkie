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
