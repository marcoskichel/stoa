//! E2E quality gate: `stoa hook install --platform claude-code --inject session-start`.
//!
//! Spec source: ROADMAP.md M5 — registers the SessionStart injection
//! hook by emitting a paste-ready settings snippet that pins the
//! `stoa-inject-hook` binary to the `startup` matcher.

mod common;

use common::{stderr, stdout, stoa, workspace};

#[test]
fn install_inject_session_start_emits_session_start_block() {
    let ws = workspace();
    let out = stoa(
        &ws,
        &["hook", "install", "--platform", "claude-code", "--inject", "session-start"],
    );
    assert!(out.status.success(), "command failed: {}", stderr(&out));
    let body = stdout(&out);
    assert!(body.contains("SessionStart"), "snippet must register the SessionStart hook: {body}");
    assert!(
        body.contains("stoa-inject-hook"),
        "snippet must invoke the `stoa-inject-hook` binary: {body}",
    );
    assert!(body.contains("startup"), "snippet must pin the `startup` matcher: {body}");
}

#[test]
fn install_inject_unknown_kind_errors() {
    let ws = workspace();
    let out = stoa(
        &ws,
        &["hook", "install", "--platform", "claude-code", "--inject", "user-prompt-submit"],
    );
    assert!(
        !out.status.success(),
        "v0.1 only supports `--inject session-start`; other kinds must error",
    );
    let err = stderr(&out);
    assert!(err.contains("session-start"), "diagnostic must name the supported kind: {err}");
}

#[test]
fn install_without_inject_emits_capture_snippet() {
    let ws = workspace();
    let out = stoa(&ws, &["hook", "install", "--platform", "claude-code"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(
        body.contains("Stop") || body.contains("SessionEnd"),
        "default install must still emit the capture (Stop/SessionEnd) snippet: {body}",
    );
    assert!(
        !body.contains("stoa-inject-hook"),
        "capture snippet must NOT reference the inject binary: {body}",
    );
}
