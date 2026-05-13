//! Append-only audit log writer.
//!
//! See [ARCHITECTURE.md §10 "Audit trail"]. One JSON object per line so
//! downstream tools (and `stoa` itself, in M4+) can parse the log
//! deterministically.
//!
//! Concurrent-append safety: every entry is built fully in memory and
//! handed to a single `write_all` call against an `O_APPEND` handle. POSIX
//! guarantees `O_APPEND` writes ≤ `PIPE_BUF` (4 KiB on Linux + macOS) are
//! atomic with respect to other appenders on the same file, so interleaved
//! workers never produce torn lines. We cap the JSON entry at 4 KiB and
//! truncate `session_id` / `agent_id` if the user-supplied values push us
//! over the budget.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};
use serde_json::json;

use crate::error::{Error, Result};

/// POSIX `PIPE_BUF` floor on Linux + macOS. Appends up to this many bytes
/// are atomic on an `O_APPEND` handle; longer writes can interleave.
const ATOMIC_APPEND_LIMIT: usize = 4096;

/// Hard cap on `session_id` length carried into the audit line.
const SESSION_ID_CAP: usize = 256;

/// Hard cap on `agent_id` length carried into the audit line.
const AGENT_ID_CAP: usize = 128;

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
    let line = build_line(session_id, agent_id)?;
    debug_assert!(
        line.len() <= ATOMIC_APPEND_LIMIT,
        "audit line exceeds {ATOMIC_APPEND_LIMIT}B atomic-append budget",
    );
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_log)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}

fn build_line(session_id: &str, agent_id: &str) -> Result<String> {
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let entry = json!({
        "ts": ts,
        "event": "transcript.captured",
        "session_id": truncate(session_id, SESSION_ID_CAP),
        "agent_id": truncate(agent_id, AGENT_ID_CAP),
        "operation": "capture",
    });
    Ok(format!("{}\n", serde_json::to_string(&entry)?))
}

fn truncate(s: &str, cap: usize) -> &str {
    if s.len() <= cap {
        return s;
    }
    let mut end = cap;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
