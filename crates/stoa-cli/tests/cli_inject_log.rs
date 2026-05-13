//! E2E quality gate: `stoa inject log` — view `SessionStart` injection
//! history from `.stoa/audit.log`.
//!
//! Spec source: ROADMAP.md M5 + ARCHITECTURE.md §6.2.
//!
//! Mirrors `stoa query --json`: prints one JSON line per injection event
//! when `--json` is passed, ordered most-recent-first. `--limit N` caps
//! output; `--session <id>` filters to a single session.

#![allow(
    clippy::unwrap_used,
    reason = "Test helpers fast-fail on tmp-dir / IO setup errors."
)]

mod common;

use std::fs::OpenOptions;
use std::io::Write;

use common::{init, stderr, stdout, stoa, workspace};

fn write_audit_line(ws: &assert_fs::TempDir, line: &str) {
    let dir = ws.path().join(".stoa");
    std::fs::create_dir_all(&dir).unwrap();
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("audit.log"))
        .unwrap();
    writeln!(f, "{line}").unwrap();
}

fn make_event(session_id: &str, ctx: &str) -> String {
    serde_json::json!({
        "ts": "2026-05-13T12:00:00.000Z",
        "event": "stoa.inject",
        "hook_event_name": "SessionStart",
        "session_id": session_id,
        "query": "redis cache",
        "hits": 3,
        "chars_injected": ctx.len(),
        "additional_context": ctx,
    })
    .to_string()
}

#[test]
fn inject_log_subcommand_exists_in_help() {
    let ws = workspace();
    let out = stoa(&ws, &["--help"]);
    let body = stdout(&out);
    assert!(
        body.contains("inject"),
        "`stoa --help` must list the `inject` subcommand: {body}"
    );
}

#[test]
fn inject_log_outside_workspace_exits_non_zero() {
    let ws = workspace();
    let out = stoa(&ws, &["inject", "log"]);
    assert!(
        !out.status.success(),
        "`stoa inject log` must require a workspace: stderr={}",
        stderr(&out),
    );
}

#[test]
fn inject_log_emits_recorded_events() {
    let ws = workspace();
    init(&ws);
    write_audit_line(
        &ws,
        &make_event("01JINJECTLOGSESSION00000001", "<stoa-memory>page A</stoa-memory>"),
    );
    write_audit_line(
        &ws,
        &make_event("01JINJECTLOGSESSION00000002", "<stoa-memory>page B</stoa-memory>"),
    );
    let out = stoa(&ws, &["inject", "log"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(
        body.contains("01JINJECTLOGSESSION00000001"),
        "log output must include the first session id: {body}",
    );
    assert!(
        body.contains("01JINJECTLOGSESSION00000002"),
        "log output must include the second session id: {body}",
    );
    assert!(
        body.contains("page A") || body.contains("<stoa-memory>"),
        "log output must include the wrapped injection text: {body}",
    );
}

#[test]
fn inject_log_filter_by_session() {
    let ws = workspace();
    init(&ws);
    write_audit_line(
        &ws,
        &make_event("01JINJECTLOGSESSION00000001", "<stoa-memory>page A</stoa-memory>"),
    );
    write_audit_line(
        &ws,
        &make_event("01JINJECTLOGSESSION00000002", "<stoa-memory>page B</stoa-memory>"),
    );
    let out = stoa(&ws, &["inject", "log", "--session", "01JINJECTLOGSESSION00000002"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(
        body.contains("01JINJECTLOGSESSION00000002"),
        "filter target session must appear: {body}",
    );
    assert!(
        !body.contains("01JINJECTLOGSESSION00000001"),
        "filter must exclude other sessions: {body}",
    );
}

#[test]
fn inject_log_limit_caps_output() {
    let ws = workspace();
    init(&ws);
    for i in 0..5 {
        let sid = format!("01JINJECTLOGSESSION0000000{i}");
        write_audit_line(&ws, &make_event(&sid, "<stoa-memory>x</stoa-memory>"));
    }
    let out = stoa(&ws, &["inject", "log", "--limit", "2"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let body = stdout(&out);
    let occurrences = body.matches("stoa.inject").count();
    assert!(
        occurrences <= 2,
        "`--limit 2` must emit at most 2 events (saw {occurrences}): {body}",
    );
}
