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
use stoa_queue::{ClaimedRow, Queue};

use crate::audit;
use crate::error::{Error, Result};
use crate::redactor::Redactor;

/// Default lease used by `drain_once` (60s — well above typical capture).
const DEFAULT_LEASE_SECS: i64 = 60;

/// Worker identifier prefix; per-process id is appended at runtime.
const WORKER_PREFIX: &str = "stoa-capture";

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
pub fn drain_once(cfg: &WorkerConfig) -> Result<Option<DrainResult>> {
    let q = Queue::open(&cfg.queue_path)?;
    let Some(claim) = q.claim(&worker_id(), DEFAULT_LEASE_SECS)? else {
        return Ok(None);
    };
    let outcome = process(cfg, &claim);
    match outcome {
        Ok(result) => {
            q.complete(claim.id)?;
            Ok(Some(result))
        },
        Err(e) => Err(e),
    }
}

fn process(cfg: &WorkerConfig, claim: &ClaimedRow) -> Result<DrainResult> {
    let payload = parse_payload(&claim.payload)?;
    let source = resolve_source(cfg, &payload.session_path);
    let output = cfg
        .sessions_dir
        .join(format!("{}.jsonl", payload.session_id));
    redact_to_disk(&source, &output)?;
    audit::append_capture(&cfg.audit_log, &payload.session_id, &payload.agent_id)?;
    Ok(DrainResult {
        session_id: payload.session_id,
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

fn resolve_source(cfg: &WorkerConfig, raw: &str) -> PathBuf {
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
