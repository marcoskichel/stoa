//! Shared helpers for `stoa-hook` binary tests.

#![expect(
    dead_code,
    reason = "Helpers shared across test files; not all are used in every test."
)]
#![expect(
    unreachable_pub,
    reason = "`tests/common/` is included via `mod common;` per integration-test binary; `pub` is needed for cross-file sharing."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test helpers panic on setup failure — fast-fail is intended."
)]

use std::path::PathBuf;
use std::process::Output;

use assert_fs::TempDir;
use snapbox::cmd::Command;

/// Build a fresh tmp dir + return the `<tmp>/.stoa/queue.db` path.
pub fn fresh_queue_path() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join(".stoa/queue.db");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    (tmp, path)
}

/// Spawn `stoa-hook` with the canonical capture args.
pub fn run_hook(queue: &PathBuf, session_id: &str, session_path: &str) -> Output {
    Command::new(snapbox::cmd::cargo_bin!("stoa-hook"))
        .args([
            "--queue",
            queue.to_str().unwrap(),
            "--session-id",
            session_id,
            "--session-path",
            session_path,
            "--agent-id",
            "test",
        ])
        .output()
        .unwrap()
}
