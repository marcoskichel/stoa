//! End-to-end CLI tests against the source fixtures under `tests/fixtures/`.

#![expect(
    clippy::expect_used,
    reason = "test harness boilerplate; panic-on-error is the desired behavior"
)]

use std::path::PathBuf;

use assert_cmd::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_against(name: &str) -> (bool, String) {
    let out = Command::cargo_bin("stoa-doclint")
        .expect("bin present")
        .arg(fixture(name))
        .output()
        .expect("ran");
    let success = out.status.success();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (success, stdout)
}

#[test]
fn flags_bare_line_comment() {
    let (ok, stdout) = run_against("forbidden_line.rs");
    assert!(!ok, "expected failure, got success: {stdout}");
    assert!(stdout.contains("forbidden_line.rs"));
    assert!(stdout.contains("forbidden comment"));
}

#[test]
fn flags_bare_block_comment() {
    let (ok, stdout) = run_against("forbidden_block.rs");
    assert!(!ok, "expected failure, got success: {stdout}");
    assert!(stdout.contains("forbidden_block.rs"));
}

#[test]
fn flags_todo_prefix() {
    let (ok, stdout) = run_against("forbidden_todo.rs");
    assert!(!ok, "TODO must be flagged; got: {stdout}");
    assert!(stdout.contains("forbidden_todo.rs"));
}

#[test]
fn accepts_doc_comments() {
    let (ok, stdout) = run_against("ok_doc.rs");
    assert!(ok, "expected success on doc-only file, got: {stdout}");
}

#[test]
fn accepts_prefixed_comments() {
    let (ok, stdout) = run_against("ok_prefix.rs");
    assert!(ok, "expected success on prefixed comments, got: {stdout}");
}

#[test]
fn warn_only_exits_zero() {
    let out = Command::cargo_bin("stoa-doclint")
        .expect("bin present")
        .arg("--warn-only")
        .arg(fixture("forbidden_line.rs"))
        .output()
        .expect("ran");
    assert!(out.status.success());
}
