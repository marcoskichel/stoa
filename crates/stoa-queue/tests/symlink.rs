//! E2E quality gate: `Queue::open` refuses symlinked DB paths.
//!
//! Spec source: ARCHITECTURE.md §15 (`SQLite` open path; security policy).
//!
//! A hostile `.stoa/queue.db -> /tmp/elsewhere` would otherwise let an
//! attacker steer WAL/SHM siblings into the link target. We refuse the
//! open up front with a diagnostic mentioning "symlink".
//!
//! Parent-directory symlinks are NOT rejected — macOS roots every temp
//! dir at `/var/folders -> /private/var/folders` and the workspace
//! itself is allowed to live under such paths.

mod common;

use common::{enqueue_session_ended, fresh_queue, queue_path};

#[test]
fn baseline_open_succeeds_at_real_path() {
    let (tmp, q) = fresh_queue();
    enqueue_session_ended(&q, "sess-baseline").unwrap();
    let _path: std::path::PathBuf = queue_path(&tmp);
    assert_eq!(q.pending_count().unwrap(), 1);
}

#[cfg(unix)]
#[test]
fn open_refuses_symlinked_db_path() {
    use stoa_queue::Queue;

    let tmp = assert_fs::TempDir::new().unwrap();
    let real = tmp.path().join("real.db");
    {
        let q = Queue::open(&real).unwrap();
        enqueue_session_ended(&q, "sess-real").unwrap();
    }
    let link = tmp.path().join("queue.db");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let err = Queue::open(&link).expect_err("symlinked DB path must be rejected");
    let msg = format!("{err}");
    assert!(msg.contains("symlink"), "expected symlink rejection diagnostic, got: {msg}");
}
