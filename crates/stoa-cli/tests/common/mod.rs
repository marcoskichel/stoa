//! Shared helpers for stoa-cli end-to-end tests.
//!
//! All helpers operate against a real `stoa` binary built by Cargo and a
//! per-test temporary workspace from `assert_fs::TempDir`.

#![expect(
    dead_code,
    reason = "Helpers shared across test files; not all are used in every test."
)]
#![expect(
    unreachable_pub,
    reason = "`tests/common/` is included via `mod common;` per integration-test binary; `pub` is needed for cross-file sharing inside each test binary."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test helpers panic on setup failure — fast-fail on tmp-dir / IO errors is intended."
)]

use std::process::Output;

use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use snapbox::cmd::Command;

/// Build a fresh tmp workspace for one test.
pub fn workspace() -> TempDir {
    TempDir::new().unwrap()
}

/// Spawn `stoa <args...>` with the workspace as `cwd`.
pub fn stoa(workspace: &TempDir, args: &[&str]) -> Output {
    Command::new(snapbox::cmd::cargo_bin!("stoa"))
        .current_dir(workspace.path())
        .args(args)
        .output()
        .unwrap()
}

/// Convenience: run `stoa init` and assert success.
pub fn init(workspace: &TempDir) {
    let out = stoa(workspace, &["init"]);
    assert!(
        out.status.success(),
        "`stoa init` failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Write a file under the workspace, ensuring parent dirs exist.
pub fn write_file(workspace: &TempDir, rel: &str, contents: &str) {
    let path = workspace.child(rel);
    if let Some(parent) = path.path().parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path.path(), contents).unwrap();
}

/// Read a file under the workspace as a UTF-8 string.
pub fn read_file(workspace: &TempDir, rel: &str) -> String {
    let path = workspace.child(rel);
    std::fs::read_to_string(path.path()).unwrap()
}

/// Assert that every path exists under the workspace.
pub fn assert_paths_exist(workspace: &TempDir, rels: &[&str]) {
    for rel in rels {
        let p = workspace.child(*rel);
        assert!(
            p.path().exists(),
            "expected path `{rel}` to exist under workspace {}",
            workspace.path().display(),
        );
    }
}

/// Decode stdout/stderr as UTF-8 (test-only, panics on invalid UTF-8).
pub fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

/// Decode stderr as UTF-8 (test-only).
pub fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

/// Returns true if `haystack` contains `needle` as a substring.
pub fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// Path-existence helper that swallows symlink quirks across platforms.
pub fn exists(workspace: &TempDir, rel: &str) -> bool {
    workspace.child(rel).path().exists()
}
