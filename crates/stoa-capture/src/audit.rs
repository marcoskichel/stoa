//! Append-only audit log writer.
//!
//! See [ARCHITECTURE.md §10 "Audit trail"]. One JSON object per line so
//! downstream tools (and `stoa` itself, in M4+) can parse the log
//! deterministically.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};
use serde_json::json;

use crate::error::{Error, Result};

/// Append one `transcript.captured` event to the audit log.
///
/// Refuses to follow symlinks on the log path: an attacker pre-seeding
/// `.stoa/audit.log -> /etc/something` must not redirect our append. The
/// check is `symlink_metadata` + `is_symlink()` before opening — there is
/// still a TOCTOU window, but the realistic attacker shape here is "user
/// laid down a malicious symlink", not a concurrent racer.
pub(crate) fn append_capture(audit_log: &Path, session_id: &str, agent_id: &str) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(audit_log)
        && meta.file_type().is_symlink()
    {
        return Err(Error::PayloadRejected("audit log is a symlink"));
    }
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
