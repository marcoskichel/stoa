//! E2E quality gate: `stoa daemon` subcommand.
//!
//! Spec source: [ROADMAP.md M3] + [ARCHITECTURE.md §7].
//!
//! Full daemon-loop testing (long-running poll, SIGTERM graceful drain) is
//! covered in `stoa-capture` worker tests + unit tests inside `stoa-cli`.
//! These E2E tests cover the CLI surface: subcommand exists, `--once` drains
//! a single cycle and exits cleanly, even on an empty queue.

mod common;

use common::{init, stderr, stoa, workspace, write_file};

#[test]
fn daemon_once_succeeds_on_fresh_workspace() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["daemon", "--once"]);
    assert!(
        out.status.success(),
        "`stoa daemon --once` must succeed on a fresh workspace: {}",
        stderr(&out),
    );
}

#[test]
fn daemon_once_drains_one_queued_event() {
    let ws = workspace();
    init(&ws);
    let raw = ws.path().join("raw.jsonl");
    write_file(&ws, "raw.jsonl", "{\"role\":\"user\",\"text\":\"hi\"}\n");
    // NOTE: enqueue directly via stoa-queue; the daemon subcommand opens
    // `.stoa/queue.db` under the workspace root.
    let q = stoa_queue::Queue::open(&ws.path().join(".stoa/queue.db")).unwrap();
    q.insert(
        "agent.session.ended",
        "sess-001",
        &serde_json::json!({
            "session_id": "sess-001",
            "session_path": raw.display().to_string(),
            "agent_id": "claude-code",
        }),
    )
    .unwrap();
    drop(q);
    let out = stoa(&ws, &["daemon", "--once"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        ws.path().join("sessions/sess-001.jsonl").exists(),
        "daemon must produce a session JSONL after one drain cycle",
    );
}

#[test]
fn daemon_outside_workspace_exits_non_zero() {
    let ws = workspace();
    let out = stoa(&ws, &["daemon", "--once"]);
    assert!(!out.status.success(), "daemon must require a workspace (no STOA.md)",);
}
