//! E2E quality gate: `Queue::open` refuses symlinked DB paths.
//!
//! Spec source: ARCHITECTURE.md §15 (`SQLite` open path; security policy).
//!
//! A hostile `.stoa/queue.db -> /tmp/elsewhere` would otherwise let an
//! attacker steer WAL/SHM siblings into the link target on builds that
//! ignore `SQLITE_OPEN_NOFOLLOW`. We refuse the open up front with a
//! diagnostic mentioning "symlink".

mod common;

use common::{enqueue_session_ended, fresh_queue, queue_path};
use stoa_queue::Queue;

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

#[cfg(unix)]
#[test]
fn open_refuses_db_under_symlinked_parent() {
    let tmp = assert_fs::TempDir::new().unwrap();
    let real_dir = tmp.path().join("real-dir");
    std::fs::create_dir_all(&real_dir).unwrap();
    let link_dir = tmp.path().join("link-dir");
    std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();
    let candidate = link_dir.join("queue.db");
    let err =
        Queue::open(&candidate).expect_err("symlinked parent dir must be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("symlink"),
        "expected symlink rejection diagnostic on parent, got: {msg}",
    );
}
