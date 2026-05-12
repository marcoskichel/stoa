//! Shared helpers for `stoa-queue` integration tests.

#![expect(
    dead_code,
    reason = "Helpers shared across test files; not all are used in every test."
)]
#![expect(
    unreachable_pub,
    reason = "`tests/common/` is included via `mod common;` per integration-test binary; `pub` is needed for cross-file sharing."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test helpers panic on setup failure — fast-fail is intended."
)]

use std::path::PathBuf;

use assert_fs::TempDir;
use stoa_queue::Queue;

/// Build a fresh tmp-dir + open a queue at `<tmp>/queue.db`.
pub fn fresh_queue() -> (TempDir, Queue) {
    let tmp = TempDir::new().unwrap();
    let q = Queue::open(&queue_path(&tmp)).unwrap();
    (tmp, q)
}

/// Path to the queue DB under a workspace tmp dir.
pub fn queue_path(tmp: &TempDir) -> PathBuf {
    tmp.path().join("queue.db")
}

/// Enqueue a simple `agent.session.ended` event for `session_id`.
pub fn enqueue_session_ended(q: &Queue, session_id: &str) -> stoa_queue::Result<()> {
    q.insert(
        "agent.session.ended",
        session_id,
        &serde_json::json!({"session_id": session_id, "agent_id": "test"}),
    )?;
    Ok(())
}
