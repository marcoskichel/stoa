//! E2E quality gate: queue runs in WAL mode + `busy_timeout` respected.
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
    // NOTE: sqlite returns sync mode as int: 0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA
    assert_eq!(sync, 1, "queue must use synchronous=NORMAL");
}

#[test]
fn queue_busy_timeout_is_set() {
    let (_tmp, q) = fresh_queue();
    let ms = q.pragma_busy_timeout().unwrap();
    assert!(ms >= 1000, "busy_timeout must be at least 1000ms; got {ms}");
}

#[test]
fn checkpoint_truncates_wal_file_to_near_zero() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let path = tmp.path().join("queue.db");
    let q = stoa_queue::Queue::open(&path).unwrap();
    for n in 0..200_i32 {
        common::enqueue_session_ended(&q, &format!("sess-wal-{n}")).unwrap();
    }
    let wal_path = path.with_extension("db-wal");
    let before = std::fs::metadata(&wal_path).map_or(0, |m| m.len());
    assert!(before > 0, "WAL must have grown after 200 inserts; got {before}");
    q.checkpoint().unwrap();
    let after = std::fs::metadata(&wal_path).map_or(0, |m| m.len());
    assert!(
        after < before,
        "WAL must shrink after checkpoint(TRUNCATE): before={before} after={after}",
    );
    assert!(after <= 4096, "WAL should be near-empty after truncate; got {after}");
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
