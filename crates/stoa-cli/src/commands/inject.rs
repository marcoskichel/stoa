//! `stoa inject log` — tail the injection audit JSONL.
//!
//! Reads `.stoa/audit.log` from the nearest STOA.md workspace, returns
//! the last N rows (newest first), optionally filtered to one session.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::cli::InjectLogArgs;

/// Run `stoa inject log`.
pub(crate) fn log(args: &InjectLogArgs) -> Result<()> {
    let root = resolve_workspace_root()?;
    let log_path = root.join(".stoa").join("audit.log");
    if !log_path.is_file() {
        println("No audit log yet (no injections recorded).");
        return Ok(());
    }
    let raw =
        fs::read_to_string(&log_path).with_context(|| format!("reading {}", log_path.display()))?;
    let filter = args.session.as_deref();
    let limit = args.limit.max(1);
    let mut rows: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if let Some(sid) = filter {
        rows.retain(|l| line_session_matches(l, sid));
    }
    let n = rows.len();
    let start = n.saturating_sub(limit);
    let tail = &rows[start..];
    for row in tail.iter().rev() {
        println(row);
    }
    Ok(())
}

fn line_session_matches(line: &str, sid: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|v| {
            v.get("session_id")
                .and_then(|s| s.as_str())
                .map(str::to_owned)
        })
        .is_some_and(|s| s == sid)
}

fn resolve_workspace_root() -> Result<PathBuf> {
    let here = std::env::current_dir().context("getting current dir")?;
    let mut cursor: Option<&Path> = Some(&here);
    while let Some(d) = cursor {
        if d.join("STOA.md").is_file() {
            return Ok(d.to_path_buf());
        }
        cursor = d.parent();
    }
    bail!("no STOA.md found from `{}` up to /", here.display());
}

#[expect(clippy::print_stdout, reason = "User-facing CLI output.")]
fn println(msg: &str) {
    println!("{msg}");
}
