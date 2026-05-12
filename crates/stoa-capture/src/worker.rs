//! Capture worker — drains one `agent.session.ended` queue row.
//!
//! See [ARCHITECTURE.md §7 "Capture pipeline (the hot path)"]:
//! 1. Claim a row with a lease.
//! 2. Read the source `JSONL` referenced by the payload.
//! 3. Run the [`crate::Redactor`] line-by-line.
//! 4. Write redacted output to `sessions/<session_id>.jsonl`.
//! 5. Append the capture event to `.stoa/audit.log`.
//! 6. Mark the row done.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use stoa_core::SessionId;
use stoa_queue::{ClaimedRow, Queue};

use crate::audit;
use crate::error::{Error, Result};
use crate::redactor::Redactor;

/// Default lease used by `drain_once` (60s — well above typical capture).
const DEFAULT_LEASE_SECS: i64 = 60;

/// Worker identifier prefix; per-process id is appended at runtime.
const WORKER_PREFIX: &str = "stoa-capture";

/// Max processing attempts before a row is dead-lettered (`status='failed'`).
///
/// On every `process()` error the worker increments the row's `attempts`
/// column and releases it back to `pending`; once the count hits this
/// ceiling the row is moved to `failed` with `error_kind` set so the next
/// claim cycle skips it instead of looping forever.
const MAX_ATTEMPTS: i64 = 5;

/// Paths the worker needs to do its job.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// `.stoa/queue.db` path.
    pub queue_path: PathBuf,
    /// `sessions/` directory.
    pub sessions_dir: PathBuf,
    /// `.stoa/audit.log` path.
    pub audit_log: PathBuf,
    /// Workspace root (for resolving relative paths in payloads).
    pub workspace_root: PathBuf,
}

/// Side-effect record returned by [`drain_once`] on success.
#[derive(Debug, Clone)]
pub struct DrainResult {
    /// `session_id` of the drained row.
    pub session_id: String,
    /// Path of the written `sessions/<id>.jsonl`.
    pub output_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Payload {
    session_id: String,
    session_path: String,
    #[serde(default)]
    agent_id: String,
}

/// Drain one queue row. Returns `None` on an empty queue.
///
/// On `process()` error the row's `attempts` column is incremented and the
/// row is released back to `pending`. Once `attempts` reaches
/// [`MAX_ATTEMPTS`] the row is dead-lettered (`status='failed'`,
/// `error_kind` recorded) so the worker stops looping on poison payloads.
pub fn drain_once(cfg: &WorkerConfig) -> Result<Option<DrainResult>> {
    let q = Queue::open(&cfg.queue_path)?;
    let Some(claim) = q.claim(&worker_id(), DEFAULT_LEASE_SECS)? else {
        return Ok(None);
    };
    match process(cfg, &claim) {
        Ok(result) => {
            q.complete(claim.id)?;
            Ok(Some(result))
        },
        Err(e) => {
            handle_failure(&q, &claim, &e)?;
            Err(e)
        },
    }
}

fn handle_failure(q: &Queue, claim: &ClaimedRow, err: &Error) -> Result<()> {
    let kind = err.classify();
    let outcome = q.record_failure(claim.id, kind, MAX_ATTEMPTS)?;
    if outcome.dead_lettered {
        tracing::error!(
            row_id = claim.id,
            session_id = %claim.session_id,
            attempts = outcome.attempts,
            error_kind = kind,
            error = %err,
            "capture worker dead-lettered poison row",
        );
    } else {
        tracing::error!(
            row_id = claim.id,
            session_id = %claim.session_id,
            attempts = outcome.attempts,
            max_attempts = MAX_ATTEMPTS,
            error_kind = kind,
            error = %err,
            "capture worker released row for retry",
        );
    }
    Ok(())
}

fn process(cfg: &WorkerConfig, claim: &ClaimedRow) -> Result<DrainResult> {
    let payload = parse_payload(&claim.payload)?;
    let sid = SessionId::parse(&payload.session_id)
        .ok_or(Error::PayloadRejected("session_id failed grammar check"))?;
    let source = resolve_source(cfg, &payload.session_path)?;
    let output = cfg.sessions_dir.join(format!("{sid}.jsonl"));
    redact_to_disk(&source, &output)?;
    audit::append_capture(&cfg.audit_log, &sid.raw, &payload.agent_id)?;
    Ok(DrainResult {
        session_id: sid.raw,
        output_path: output,
    })
}

fn parse_payload(raw: &str) -> Result<Payload> {
    let p: Payload = serde_json::from_str(raw)?;
    if p.session_id.is_empty() {
        return Err(Error::PayloadField("session_id"));
    }
    if p.session_path.is_empty() {
        return Err(Error::PayloadField("session_path"));
    }
    Ok(p)
}

/// Resolve `raw` against the workspace root and harden against traversal.
///
/// Canonicalizes via `fs::canonicalize` so symlinks + `..` segments are
/// collapsed; the resulting path must (a) live under
/// [`WorkerConfig::workspace_root`] and (b) refer to a regular file with
/// no `is_symlink()` ancestor at the leaf. Rejection is mapped to
/// [`Error::PayloadRejected`] so the row dead-letters instead of looping.
fn resolve_source(cfg: &WorkerConfig, raw: &str) -> Result<PathBuf> {
    let candidate = build_candidate(cfg, raw);
    if fs::symlink_metadata(&candidate)
        .map_err(Error::Io)?
        .file_type()
        .is_symlink()
    {
        return Err(Error::PayloadRejected("session_path is a symlink"));
    }
    let canonical = fs::canonicalize(&candidate).map_err(Error::Io)?;
    let root = fs::canonicalize(&cfg.workspace_root).map_err(Error::Io)?;
    if !canonical.starts_with(&root) {
        return Err(Error::PayloadRejected("session_path escapes workspace root"));
    }
    Ok(canonical)
}

fn build_candidate(cfg: &WorkerConfig, raw: &str) -> PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cfg.workspace_root.join(p)
    }
}

fn redact_to_disk(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let redactor = Redactor::with_defaults();
    let input = File::open(source)?;
    let reader = BufReader::new(input);
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(dest)?;
    let mut writer = BufWriter::new(output);
    for line in reader.lines() {
        let line = line?;
        let redacted = redactor.redact_line(&line);
        writer.write_all(redacted.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn worker_id() -> String {
    format!("{WORKER_PREFIX}-{}", std::process::id())
}
