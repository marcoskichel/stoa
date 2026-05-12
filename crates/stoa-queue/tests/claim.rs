//! E2E quality gate: queue claim-with-lease + crash recovery.
//!
//! Spec source: [ARCHITECTURE.md §7] — workers claim rows with a lease;
//! crash leaves the row claim-leased for the next worker to recover.

mod common;

use std::thread;
use std::time::Duration;

use common::{enqueue_session_ended, fresh_queue};
use stoa_queue::Queue;

#[test]
fn claim_returns_pending_row() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-001").unwrap();
    let claim = q.claim("worker-1", 30).unwrap();
    assert!(claim.is_some(), "must claim the pending row");
    let row = claim.unwrap();
    assert_eq!(row.session_id, "sess-001");
    assert_eq!(row.event, "agent.session.ended");
}

#[test]
fn claim_returns_none_when_no_pending_rows() {
    let (_tmp, q) = fresh_queue();
    let claim = q.claim("worker-1", 30).unwrap();
    assert!(claim.is_none(), "empty queue must return None");
}

#[test]
fn two_workers_cannot_claim_same_row_concurrently() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-001").unwrap();
    let first = q.claim("worker-1", 30).unwrap();
    let second = q.claim("worker-2", 30).unwrap();
    assert!(first.is_some(), "first worker must claim the row");
    assert!(second.is_none(), "second worker must see no available row");
}

#[test]
fn expired_lease_is_reclaimable() {
    let (tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-crashed").unwrap();
    let original = q.claim("worker-A", 1).unwrap();
    assert!(original.is_some());
    thread::sleep(Duration::from_secs(2));
    let q2 = Queue::open(&common::queue_path(&tmp)).unwrap();
    let resumed = q2.claim("worker-B", 30).unwrap();
    assert!(resumed.is_some(), "expired lease must be reclaimable");
    assert_eq!(resumed.unwrap().session_id, "sess-crashed");
}

#[test]
fn complete_marks_row_done_and_excludes_from_claim() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-001").unwrap();
    let claim = q.claim("worker-1", 30).unwrap().unwrap();
    q.complete(claim.id).unwrap();
    let next = q.claim("worker-2", 30).unwrap();
    assert!(next.is_none(), "completed row must not be re-claimed");
}

#[test]
fn idempotent_insert_after_complete_is_safe() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-001").unwrap();
    let claim = q.claim("worker-1", 30).unwrap().unwrap();
    q.complete(claim.id).unwrap();
    enqueue_session_ended(&q, "sess-001").unwrap();
    assert_eq!(
        q.pending_count().unwrap(),
        1,
        "re-enqueue after completion should create a fresh pending row",
    );
}
