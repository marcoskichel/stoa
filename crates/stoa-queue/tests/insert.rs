//! E2E quality gate: queue insert + idempotency.
//!
//! Spec source: [ROADMAP.md M3] + [ARCHITECTURE.md §7 Capture pipeline].

mod common;

use common::{enqueue_session_ended, fresh_queue};

#[test]
fn insert_creates_pending_row() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-001").unwrap();
    assert_eq!(q.pending_count().unwrap(), 1);
}

#[test]
fn insert_is_idempotent_on_session_id() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-001").unwrap();
    enqueue_session_ended(&q, "sess-001").unwrap();
    enqueue_session_ended(&q, "sess-001").unwrap();
    assert_eq!(
        q.pending_count().unwrap(),
        1,
        "re-firing the same session_id must NOT create duplicate rows",
    );
}

#[test]
fn insert_distinct_session_ids_creates_distinct_rows() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-A").unwrap();
    enqueue_session_ended(&q, "sess-B").unwrap();
    enqueue_session_ended(&q, "sess-C").unwrap();
    assert_eq!(q.pending_count().unwrap(), 3);
}

#[test]
fn insert_persists_payload_json() {
    let (_tmp, q) = fresh_queue();
    let payload = serde_json::json!({"k": "v", "n": 42});
    q.insert("custom.event", "sess-X", &payload).unwrap();
    let row = q.peek_first_pending().unwrap().expect("row should exist");
    assert_eq!(row.event, "custom.event");
    assert_eq!(row.session_id, "sess-X");
    assert_eq!(serde_json::from_str::<serde_json::Value>(&row.payload).unwrap(), payload,);
}

#[test]
fn reopening_db_preserves_pending_rows() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let path = tmp.path().join("queue.db");
    {
        let q = stoa_queue::Queue::open(&path).unwrap();
        enqueue_session_ended(&q, "sess-001").unwrap();
    }
    let q2 = stoa_queue::Queue::open(&path).unwrap();
    assert_eq!(q2.pending_count().unwrap(), 1);
}

#[test]
fn open_fast_path_works_after_init() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let path = tmp.path().join("queue.db");
    {
        let q = stoa_queue::Queue::init(&path).unwrap();
        enqueue_session_ended(&q, "sess-fp-001").unwrap();
    }
    let q2 = stoa_queue::Queue::open(&path).unwrap();
    assert_eq!(q2.pending_count().unwrap(), 1, "fast-path open must see prior inserts");
    enqueue_session_ended(&q2, "sess-fp-002").unwrap();
    assert_eq!(q2.pending_count().unwrap(), 2, "fast-path open must support inserts");
}

#[test]
fn open_creates_db_first_run_when_no_file_exists() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let path = tmp.path().join("queue.db");
    let q = stoa_queue::Queue::open(&path).unwrap();
    enqueue_session_ended(&q, "sess-firstrun").unwrap();
    assert_eq!(q.pending_count().unwrap(), 1);
}

#[test]
fn pending_excludes_completed_rows() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-001").unwrap();
    let claim = q.claim("worker-1", 30).unwrap().expect("must claim");
    q.complete(claim.id).unwrap();
    assert_eq!(q.pending_count().unwrap(), 0);
}
