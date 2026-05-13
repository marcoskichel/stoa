//! Shared helpers for `stoa-inject-hooks` integration tests.
//!
//! Each test gets its own `assert_fs::TempDir` workspace; we drive the
//! `stoa-inject-hook` binary via `snapbox::cmd::cargo_bin!`. Workspace
//! scaffolding (init + write + index rebuild) is delegated to the `stoa`
//! CLI so we exercise the same plumbing operators will hit.

#![allow(
    dead_code,
    reason = "Helpers shared across test files; not all are used in every binary."
)]
#![allow(
    unreachable_pub,
    reason = "`tests/common.rs` is included via `mod common;` per binary; `pub` is needed for cross-file sharing."
)]
#![allow(
    clippy::unwrap_used,
    reason = "Test helpers panic on setup failure — fast-fail on tmp-dir / IO errors is intended."
)]

use std::process::Output;

use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use snapbox::cmd::Command;

/// Build a fresh tmp workspace.
pub fn workspace() -> TempDir {
    TempDir::new().unwrap()
}

/// Run `stoa <args...>` from inside `ws`. Panics if the binary is missing
/// (i.e. dev-deps wired wrong).
pub fn stoa(ws: &TempDir, args: &[&str]) -> Output {
    Command::new(stoa_bin())
        .current_dir(ws.path())
        .args(args)
        .output()
        .unwrap()
}

/// Run `stoa-inject-hook` from inside `ws` with `stdin_payload` piped in.
pub fn inject_hook(ws: &TempDir, stdin_payload: &str) -> Output {
    Command::new(inject_hook_bin())
        .current_dir(ws.path())
        .stdin(stdin_payload.as_bytes())
        .output()
        .unwrap()
}

/// Initialize a workspace via `stoa init --no-embeddings` (BM25-only;
/// avoids the Python sidecar in test runs).
pub fn init(ws: &TempDir) {
    let out = stoa(ws, &["init", "--no-embeddings"]);
    assert!(out.status.success(), "stoa init failed: {}", stderr(&out));
}

/// Write a file under the workspace, creating parent dirs.
pub fn write_file(ws: &TempDir, rel: &str, contents: &str) {
    let path = ws.child(rel);
    if let Some(parent) = path.path().parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path.path(), contents).unwrap();
}

/// Rebuild the recall index after wiki writes (FTS5 + KG).
pub fn rebuild(ws: &TempDir) {
    let out = stoa(ws, &["index", "rebuild"]);
    assert!(out.status.success(), "stoa index rebuild failed: {}", stderr(&out));
}

/// Decode stdout as UTF-8.
pub fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

/// Decode stderr as UTF-8.
pub fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

fn inject_hook_bin() -> String {
    std::env::var("CARGO_BIN_EXE_stoa-inject-hook").unwrap_or_else(|_| {
        panic!("CARGO_BIN_EXE_stoa-inject-hook not set — broken cargo wiring")
    })
}

/// Cargo only sets `CARGO_BIN_EXE_<name>` for binaries in the *same*
/// package. We need `stoa` (in `stoa-cli`) to scaffold workspaces, so
/// we derive its path from the inject-hook binary which Cargo does
/// expose to us — both land in the same `target/<profile>/` directory.
fn stoa_bin() -> std::path::PathBuf {
    let inject = std::path::PathBuf::from(inject_hook_bin());
    let dir = inject.parent().expect("inject hook bin must have a parent dir");
    let exe_suffix = std::env::consts::EXE_SUFFIX;
    dir.join(format!("stoa{exe_suffix}"))
}
