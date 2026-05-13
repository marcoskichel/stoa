//! Append one JSONL row per injection event to `<workspace>/.stoa/audit.log`.
//!
//! Best-effort: any IO failure here is logged via `tracing::warn` and
//! swallowed. The audit trail is a post-hoc observability hook and must
//! NOT fail the user-facing injection path.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};

use crate::payload::HookPayload;

/// Append one structured row.
pub(crate) fn append(
    log_path: &Path,
    payload: &HookPayload,
    query: &str,
    hits: usize,
    additional_context: &str,
) {
    let line = build_line(payload, query, hits, additional_context);
    if let Err(e) = write_line(log_path, &line) {
        tracing::warn!(?e, path = %log_path.display(), "inject: audit write failed");
    }
}

fn build_line(payload: &HookPayload, query: &str, hits: usize, additional_context: &str) -> String {
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let entry = serde_json::json!({
        "ts": ts,
        "event": "stoa.inject",
        "hook_event_name": payload.hook_event_name_str(),
        "session_id": payload.session_id_str(),
        "query": query,
        "hits": hits,
        "chars_injected": additional_context.chars().count(),
        "additional_context": additional_context,
    });
    format!("{entry}\n")
}

fn write_line(log_path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    refuse_symlink(log_path)?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    f.write_all(line.as_bytes())
}

/// Refuse the open if the audit log path is itself a symlink.
///
/// A hostile `.stoa/audit.log -> /etc/passwd` (or a TOCTOU swap mid-
/// session) would otherwise let the hook append JSONL to an arbitrary
/// file. Parent-dir symlinks stay allowed because macOS roots every
/// tmpdir at `/var/folders -> /private/var/folders` and tests would
/// otherwise fail.
fn refuse_symlink(path: &Path) -> std::io::Result<()> {
    if let Ok(meta) = std::fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("audit log `{}` is a symlink — refusing to open", path.display()),
        ));
    }
    Ok(())
}
