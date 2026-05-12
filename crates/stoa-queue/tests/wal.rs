//! E2E quality gate: queue runs in WAL mode + busy_timeout respected.
//!
//! Spec source: [ARCHITECTURE.md §15] + [CLAUDE.md] — `rusqlite` v0.38,
//! WAL mode, `synchronous=NORMAL`, FTS5 schema in same DB.

mod common;

use common::fresh_queue;

#[test]
fn queue_runs_in_wal_mode() {
    let (_tmp, q) = fresh_queue();
    let mode = q.pragma_journal_mode().unwrap();
    assert_eq!(mode.to_ascii_lowercase(), "wal", "queue must run in WAL mode");
}

#[test]
fn queue_uses_synchronous_normal() {
    let (_tmp, q) = fresh_queue();
    let sync = q.pragma_synchronous().unwrap();
    // sqlite returns sync mode as int: 0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA
    assert_eq!(sync, 1, "queue must use synchronous=NORMAL");
}

#[test]
fn queue_busy_timeout_is_set() {
    let (_tmp, q) = fresh_queue();
    let ms = q.pragma_busy_timeout().unwrap();
    assert!(ms >= 1000, "busy_timeout must be at least 1000ms; got {ms}");
}

#[test]
fn second_connection_sees_first_connections_writes() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let path = tmp.path().join("queue.db");
    let q1 = stoa_queue::Queue::open(&path).unwrap();
    common::enqueue_session_ended(&q1, "sess-001").unwrap();
    let q2 = stoa_queue::Queue::open(&path).unwrap();
    assert_eq!(
        q2.pending_count().unwrap(),
        1,
        "WAL mode must let a second connection see committed writes",
    );
}
