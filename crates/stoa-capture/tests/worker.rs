//! E2E quality gate: capture worker end-to-end.
//!
//! Spec source: [ARCHITECTURE.md §7 Capture pipeline (the hot path)].
//!
//! The worker drains queue rows, runs PII redaction on the source session
//! JSONL, writes redacted output to `sessions/<id>.jsonl`, appends to
//! `.stoa/audit.log`, and marks the queue row done. SIGTERM mid-capture
//! leaves the claim leased so another worker can resume (idempotent by
//! `session_id`).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use assert_fs::TempDir;
use stoa_capture::WorkerConfig;
use stoa_queue::Queue;

#[expect(
    clippy::unwrap_used,
    reason = "Test helper; structural failure is a test bug."
)]
fn workspace() -> (TempDir, WorkerConfig) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    fs::create_dir_all(root.join("sessions")).unwrap();
    fs::create_dir_all(root.join(".stoa")).unwrap();
    let cfg = WorkerConfig {
        queue_path: root.join(".stoa/queue.db"),
        sessions_dir: root.join("sessions"),
        audit_log: root.join(".stoa/audit.log"),
        workspace_root: root,
    };
    (tmp, cfg)
}

#[expect(clippy::unwrap_used, reason = "Test helper.")]
fn write_session_file(path: &Path, lines: &[&str]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, lines.join("\n") + "\n").unwrap();
}

#[expect(clippy::unwrap_used, reason = "Test helper.")]
fn enqueue_capture(cfg: &WorkerConfig, session_id: &str, session_path: &Path) {
    let q = Queue::open(&cfg.queue_path).unwrap();
    let payload = serde_json::json!({
        "session_id": session_id,
        "session_path": session_path.display().to_string(),
        "agent_id": "claude-code",
    });
    q.insert("agent.session.ended", session_id, &payload)
        .unwrap();
}

#[test]
fn worker_drains_pending_row_to_sessions_jsonl() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw-session.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"hello"}"#]);
    enqueue_capture(&cfg, "sess-001", &raw);
    stoa_capture::drain_once(&cfg).unwrap();
    let out = cfg.sessions_dir.join("sess-001.jsonl");
    assert!(out.exists(), "redacted session JSONL must be written");
}

#[test]
fn worker_runs_pii_redaction_on_source_jsonl() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"my key is AKIAIOSFODNN7EXAMPLE"}"#]);
    enqueue_capture(&cfg, "sess-002", &raw);
    stoa_capture::drain_once(&cfg).unwrap();
    let body: String = fs::read_to_string(cfg.sessions_dir.join("sess-002.jsonl")).unwrap();
    assert!(!body.contains("AKIAIOSFODNN7EXAMPLE"), "secret leaked into sessions/: {body:?}");
    assert!(body.contains("[REDACTED:"));
}

#[test]
fn worker_appends_audit_log_entry() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"hi"}"#]);
    enqueue_capture(&cfg, "sess-003", &raw);
    stoa_capture::drain_once(&cfg).unwrap();
    let audit: String = fs::read_to_string(&cfg.audit_log).unwrap();
    assert!(audit.contains("sess-003"), "audit log must mention session id: {audit:?}");
    assert!(
        audit.contains("capture") || audit.contains("transcript.captured"),
        "audit log must record the capture event: {audit:?}",
    );
}

#[test]
fn worker_marks_queue_row_done() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"hi"}"#]);
    enqueue_capture(&cfg, "sess-004", &raw);
    stoa_capture::drain_once(&cfg).unwrap();
    let q = Queue::open(&cfg.queue_path).unwrap();
    assert_eq!(q.pending_count().unwrap(), 0, "queue must drain to zero pending");
}

#[test]
fn worker_resumes_after_lease_expiry() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"hi"}"#]);
    enqueue_capture(&cfg, "sess-crashed", &raw);
    let q = Queue::open(&cfg.queue_path).unwrap();
    let claim = q
        .claim("worker-A", 1)
        .unwrap()
        .expect("first worker claims");
    assert_eq!(claim.session_id, "sess-crashed");
    std::thread::sleep(std::time::Duration::from_secs(2));
    stoa_capture::drain_once(&cfg).unwrap();
    assert!(cfg.sessions_dir.join("sess-crashed.jsonl").exists());
}

#[test]
fn worker_is_idempotent_on_session_id() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"hi"}"#]);
    enqueue_capture(&cfg, "sess-005", &raw);
    stoa_capture::drain_once(&cfg).unwrap();
    enqueue_capture(&cfg, "sess-005", &raw);
    stoa_capture::drain_once(&cfg).unwrap();
    let body = fs::read_to_string(cfg.sessions_dir.join("sess-005.jsonl")).unwrap();
    assert!(!body.is_empty(), "re-capture must still produce output");
}

#[test]
fn worker_returns_none_on_empty_queue() {
    let (_tmp, cfg) = workspace();
    let result = stoa_capture::drain_once(&cfg).unwrap();
    assert!(result.is_none(), "draining an empty queue must return None");
}

#[expect(clippy::unwrap_used, reason = "Test helper.")]
fn enqueue_raw_payload(cfg: &WorkerConfig, session_id: &str, session_path: &str) {
    let q = Queue::open(&cfg.queue_path).unwrap();
    let payload = serde_json::json!({
        "session_id": session_id,
        "session_path": session_path,
        "agent_id": "claude-code",
    });
    q.insert("agent.session.ended", session_id, &payload)
        .unwrap();
}

#[test]
fn worker_rejects_absolute_path_outside_workspace_root() {
    let (_tmp, cfg) = workspace();
    enqueue_raw_payload(&cfg, "sess-escape-abs", "/etc/shadow");
    for _ in 0..6 {
        let outcome = stoa_capture::drain_once(&cfg);
        if outcome.is_ok() {
            break;
        }
    }
    let q = Queue::open(&cfg.queue_path).unwrap();
    assert_eq!(q.failed_count().unwrap(), 1, "outside-root path must dead-letter");
}

#[test]
fn worker_rejects_relative_traversal_payload() {
    let (_tmp, cfg) = workspace();
    enqueue_raw_payload(&cfg, "sess-escape-rel", "../../../etc/shadow");
    for _ in 0..6 {
        let outcome = stoa_capture::drain_once(&cfg);
        if outcome.is_ok() {
            break;
        }
    }
    let q = Queue::open(&cfg.queue_path).unwrap();
    assert_eq!(q.failed_count().unwrap(), 1, "traversal path must dead-letter");
}

#[test]
fn worker_dead_letters_poison_payload_after_max_attempts() {
    let (tmp, cfg) = workspace();
    let missing = tmp.path().join("does-not-exist.jsonl");
    enqueue_capture(&cfg, "sess-poison", &missing);
    for _ in 0..6 {
        let outcome = stoa_capture::drain_once(&cfg);
        if outcome.is_ok() {
            break;
        }
    }
    let q = Queue::open(&cfg.queue_path).unwrap();
    assert_eq!(q.pending_count().unwrap(), 0, "poison row must not stay claimed");
    assert_eq!(q.failed_count().unwrap(), 1, "poison row must be dead-lettered");
}

#[cfg(unix)]
#[test]
fn worker_refuses_to_follow_audit_log_symlink() {
    let (tmp, cfg) = workspace();
    let target = tmp.path().join("not-audit.log");
    fs::write(&target, b"").unwrap();
    std::os::unix::fs::symlink(&target, &cfg.audit_log).unwrap();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"hi"}"#]);
    enqueue_capture(&cfg, "sess-auditlink", &raw);
    for _ in 0..6 {
        let _ignored = stoa_capture::drain_once(&cfg);
    }
    let body = fs::read_to_string(&target).unwrap();
    assert!(
        body.is_empty(),
        "symlink target must remain untouched: {body:?}",
    );
    let q = Queue::open(&cfg.queue_path).unwrap();
    assert_eq!(q.failed_count().unwrap(), 1, "symlinked audit log must dead-letter");
}

#[cfg(unix)]
#[test]
fn worker_refuses_to_write_through_session_output_symlink() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(&raw, &[r#"{"role":"user","text":"hi"}"#]);
    let evil = tmp.path().join("evil-target.jsonl");
    fs::write(&evil, b"").unwrap();
    let out = cfg.sessions_dir.join("sess-outlink.jsonl");
    std::os::unix::fs::symlink(&evil, &out).unwrap();
    enqueue_capture(&cfg, "sess-outlink", &raw);
    for _ in 0..6 {
        let _ignored = stoa_capture::drain_once(&cfg);
    }
    let body = fs::read_to_string(&evil).unwrap();
    assert!(body.is_empty(), "symlink target must not be overwritten: {body:?}");
}

#[test]
fn worker_outputs_one_jsonl_line_per_input_line() {
    let (tmp, cfg) = workspace();
    let raw = tmp.path().join("raw.jsonl");
    write_session_file(
        &raw,
        &[
            r#"{"role":"user","text":"first"}"#,
            r#"{"role":"assistant","text":"second"}"#,
            r#"{"role":"user","text":"third"}"#,
        ],
    );
    enqueue_capture(&cfg, "sess-006", &raw);
    stoa_capture::drain_once(&cfg).unwrap();
    let body: PathBuf = cfg.sessions_dir.join("sess-006.jsonl");
    let lines = fs::read_to_string(&body).unwrap();
    let count = lines.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(count, 3, "redacted JSONL must preserve line count: {lines:?}");
}
