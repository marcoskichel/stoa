//! E2E quality gate: `Queue::take_response_for` demuxes by `session_id`.
//!
//! Spec source: ARCHITECTURE.md §6.1 (recall.request / recall.response IPC).
//!
//! Pins the contract that two concurrent IPC callers (different
//! `request_id`s) never block each other when responses arrive
//! out-of-order on the response lane.

mod common;

use common::{enqueue_session_ended, fresh_queue};

const LANE: &str = "recall.response";

#[test]
fn take_response_ignores_default_lane_rows() {
    let (_tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-default-lane").unwrap();
    let row = q.take_response_for(LANE, "sess-default-lane").unwrap();
    assert!(row.is_none(), "default-lane row must NOT match a response-lane lookup");
}

#[test]
fn take_response_returns_none_when_lane_empty() {
    let (_tmp, q) = fresh_queue();
    let row = q.take_response_for(LANE, "missing-id").unwrap();
    assert!(row.is_none(), "empty lane must yield None");
}

#[test]
fn take_response_skips_rows_with_other_session_id() {
    let (_tmp, q) = fresh_queue();
    q.insert_lane(LANE, "recall.search", "request-A", &serde_json::json!({"ok": true}))
        .unwrap();
    let row = q.take_response_for(LANE, "request-B").unwrap();
    assert!(row.is_none(), "non-matching session_id must NOT consume the head row");
    let still_there = q
        .take_response_for(LANE, "request-A")
        .unwrap()
        .expect("matching session_id must consume the row");
    assert!(still_there.1.contains("\"ok\":true"), "payload preserved: {}", still_there.1);
}

#[test]
fn take_response_marks_row_done_so_concurrent_callers_unblock() {
    let (_tmp, q) = fresh_queue();
    q.insert_lane(LANE, "recall.search", "req-A", &serde_json::json!({"v": 1}))
        .unwrap();
    q.insert_lane(LANE, "recall.search", "req-B", &serde_json::json!({"v": 2}))
        .unwrap();
    let b = q
        .take_response_for(LANE, "req-B")
        .unwrap()
        .expect("B must be reachable even though A is at head");
    assert!(b.1.contains("\"v\":2"));
    let a = q
        .take_response_for(LANE, "req-A")
        .unwrap()
        .expect("A must still be reachable after B drained");
    assert!(a.1.contains("\"v\":1"));
}

#[test]
fn take_response_is_idempotent_after_consume() {
    let (_tmp, q) = fresh_queue();
    q.insert_lane(LANE, "recall.search", "req-A", &serde_json::json!({"x": 9}))
        .unwrap();
    let _consumed = q.take_response_for(LANE, "req-A").unwrap().unwrap();
    let again = q.take_response_for(LANE, "req-A").unwrap();
    assert!(again.is_none(), "second consume must be a no-op");
}
