//! End-to-end CLI tests against the source fixtures under `tests/fixtures/`.

use std::path::PathBuf;

use assert_cmd::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn flags_trivial_const_env() {
    let mut cmd = Command::cargo_bin("stoa-doclint").expect("bin present");
    let out = cmd.arg(fixture("trivial_const_env.rs")).assert().failure();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).into_owned();
    assert!(stdout.contains("trivial_const_env.rs"));
    assert!(stdout.contains("`VERSION`"));
}

#[test]
fn flags_trivial_struct() {
    let mut cmd = Command::cargo_bin("stoa-doclint").expect("bin present");
    let out = cmd.arg(fixture("trivial_struct.rs")).assert().failure();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).into_owned();
    assert!(stdout.contains("`UserSession`"));
}

#[test]
fn does_not_flag_genuine_docs() {
    let mut cmd = Command::cargo_bin("stoa-doclint").expect("bin present");
    cmd.arg(fixture("ok_genuine.rs")).assert().success();
}

#[test]
fn warn_only_exits_zero() {
    let mut cmd = Command::cargo_bin("stoa-doclint").expect("bin present");
    cmd.arg("--warn-only")
        .arg(fixture("trivial_const_env.rs"))
        .assert()
        .success();
}
