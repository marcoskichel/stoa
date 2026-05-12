//! E2E quality gate: `stoa-hook` binary behavior.
//!
//! Spec source: [ARCHITECTURE.md §7] — the hook is a single short executable
//! with no LLM calls. It opens `.stoa/queue.db`, inserts one row, exits.
//! Idempotent on `session_id`.

mod common;

use common::{fresh_queue_path, run_hook};

#[test]
fn hook_succeeds_and_creates_queue_db() {
    let (_tmp, queue) = fresh_queue_path();
    let out = run_hook(&queue, "sess-001", "/tmp/raw-session.jsonl");
    assert!(
        out.status.success(),
        "hook must exit 0: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(queue.exists(), "queue DB must be created at the path");
}

#[test]
fn hook_inserts_one_pending_row() {
    let (_tmp, queue) = fresh_queue_path();
    let _ = run_hook(&queue, "sess-001", "/tmp/raw.jsonl");
    let q = stoa_queue::Queue::open(&queue).unwrap();
    assert_eq!(q.pending_count().unwrap(), 1);
}

#[test]
fn hook_is_idempotent_on_session_id() {
    let (_tmp, queue) = fresh_queue_path();
    let _ = run_hook(&queue, "sess-A", "/tmp/raw.jsonl");
    let _ = run_hook(&queue, "sess-A", "/tmp/raw.jsonl");
    let _ = run_hook(&queue, "sess-A", "/tmp/raw.jsonl");
    let q = stoa_queue::Queue::open(&queue).unwrap();
    assert_eq!(
        q.pending_count().unwrap(),
        1,
        "re-firing the same session_id MUST NOT create duplicate rows",
    );
}

#[test]
fn hook_persists_session_path_in_payload() {
    let (_tmp, queue) = fresh_queue_path();
    let _ = run_hook(&queue, "sess-002", "/tmp/specific-session.jsonl");
    let q = stoa_queue::Queue::open(&queue).unwrap();
    let row = q.peek_first_pending().unwrap().expect("must have row");
    assert!(
        row.payload.contains("/tmp/specific-session.jsonl"),
        "payload must carry the session path: {:?}",
        row.payload,
    );
}

#[test]
fn hook_exits_non_zero_when_queue_path_unwritable() {
    // NOTE: /proc is virtual + read-only; opening a SQLite file there must fail.
    let out = snapbox::cmd::Command::new(snapbox::cmd::cargo_bin!("stoa-hook"))
        .args([
            "--queue",
            "/proc/cannot-write.db",
            "--session-id",
            "sess-X",
            "--session-path",
            "/tmp/whatever.jsonl",
            "--agent-id",
            "test",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "hook must surface IO failure");
}
