//! Append-only audit log writer.
//!
//! See [ARCHITECTURE.md §10 "Audit trail"]. One JSON object per line so
//! downstream tools (and `stoa` itself, in M4+) can parse the log
//! deterministically.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};
use serde_json::json;

use crate::error::Result;

/// Append one `transcript.captured` event to the audit log.
pub(crate) fn append_capture(audit_log: &Path, session_id: &str, agent_id: &str) -> Result<()> {
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let entry = json!({
        "ts": ts,
        "event": "transcript.captured",
        "session_id": session_id,
        "agent_id": agent_id,
        "operation": "capture",
    });
    let line = format!("{}\n", serde_json::to_string(&entry)?);
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_log)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}
