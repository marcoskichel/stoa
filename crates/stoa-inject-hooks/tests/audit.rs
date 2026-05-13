//! E2E quality gate: every injection appends a JSONL row to
//! `.stoa/audit.log`.
//!
//! Spec source: ARCHITECTURE.md §6.2 + §10 — audit trail must be
//! append-only and machine-readable. Each row records the hook event,
//! query, returned hit count, char count of the wrapped block, and the
//! session id from the inbound payload.

mod common;

use common::{init, inject_hook, rebuild, stderr, workspace, write_file};

const PAGE_BODY: &str = "\
---
id: ent-redis
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-12
---

# Redis

Caching layer for session tokens and rate limits.
";

fn payload(workspace_path: &std::path::Path, session_id: &str) -> String {
    serde_json::json!({
        "hook_event_name": "SessionStart",
        "session_id": session_id,
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": workspace_path.display().to_string(),
        "model": "claude-opus-4-7",
        "source": "startup",
    })
    .to_string()
}

fn read_audit_log(ws: &assert_fs::TempDir) -> String {
    let p = ws.path().join(".stoa").join("audit.log");
    std::fs::read_to_string(&p)
        .unwrap_or_else(|_| panic!("audit log missing at {}", p.display()))
}

#[test]
fn injection_appends_jsonl_audit_line() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", PAGE_BODY);
    rebuild(&ws);
    let out = inject_hook(&ws, &payload(ws.path(), "01JAUDITSESS00000000000001"));
    assert!(out.status.success(), "{}", stderr(&out));
    let log = read_audit_log(&ws);
    let last = log.lines().last().expect("audit log must contain at least one line");
    let parsed: serde_json::Value = serde_json::from_str(last)
        .unwrap_or_else(|e| panic!("audit row must be valid JSON ({e}): {last}"));
    assert_eq!(
        parsed.get("event").and_then(|v| v.as_str()),
        Some("stoa.inject"),
        "audit row event must be `stoa.inject`: {last}",
    );
    assert_eq!(
        parsed.get("hook_event_name").and_then(|v| v.as_str()),
        Some("SessionStart"),
    );
    assert_eq!(
        parsed.get("session_id").and_then(|v| v.as_str()),
        Some("01JAUDITSESS00000000000001"),
    );
    assert!(
        parsed.get("hits").and_then(serde_json::Value::as_u64).is_some(),
        "audit row must record a numeric `hits` count: {last}",
    );
    assert!(
        parsed.get("chars_injected").and_then(serde_json::Value::as_u64).is_some(),
        "audit row must record `chars_injected` for utilization tracking: {last}",
    );
}

#[test]
fn audit_log_is_append_only() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", PAGE_BODY);
    rebuild(&ws);
    let _first = inject_hook(&ws, &payload(ws.path(), "01JAUDITSESS00000000000001"));
    let _second = inject_hook(&ws, &payload(ws.path(), "01JAUDITSESS00000000000002"));
    let log = read_audit_log(&ws);
    let inject_lines: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("\"stoa.inject\""))
        .collect();
    assert!(
        inject_lines.len() >= 2,
        "two invocations must yield two appended audit rows (got {}): {log}",
        inject_lines.len(),
    );
}
