//! `stoa inject log` — print `SessionStart` injection history from
//! `.stoa/audit.log`.
//!
//! Spec: ARCHITECTURE.md §6.2 — every injection event is appended as a
//! single JSON line. This command tails the file (optionally filtered
//! by `--session` and capped by `--limit`), printing each row in
//! most-recent-first order.

use std::fs;
use std::path::Path;

use anyhow::Context;

use crate::workspace::Workspace;

/// Dispatched from `Cli::dispatch`.
pub(crate) fn log(session: Option<&str>, limit: Option<usize>) -> anyhow::Result<()> {
    let ws = Workspace::current().context("locating Stoa workspace")?;
    let path = ws.root.join(".stoa").join("audit.log");
    let events = collect_events(&path, session, limit)?;
    for ev in &events {
        emit_event(ev);
    }
    Ok(())
}

#[derive(Debug)]
struct InjectEvent {
    ts: String,
    session_id: String,
    hits: u64,
    chars_injected: u64,
    additional_context: String,
    raw: String,
}

fn collect_events(
    path: &Path,
    session: Option<&str>,
    limit: Option<usize>,
) -> anyhow::Result<Vec<InjectEvent>> {
    let body = read_audit_body(path)?;
    let mut filtered: Vec<InjectEvent> = body
        .lines()
        .filter_map(parse_inject_line)
        .filter(|ev| session_matches(ev, session))
        .collect();
    filtered.reverse();
    if let Some(n) = limit {
        filtered.truncate(n);
    }
    Ok(filtered)
}

fn read_audit_body(path: &Path) -> anyhow::Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))
}

fn parse_inject_line(line: &str) -> Option<InjectEvent> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("event").and_then(|v| v.as_str()) != Some("stoa.inject") {
        return None;
    }
    Some(InjectEvent {
        ts: take_str(&value, "ts"),
        session_id: take_str(&value, "session_id"),
        hits: take_u64(&value, "hits"),
        chars_injected: take_u64(&value, "chars_injected"),
        additional_context: take_str(&value, "additional_context"),
        raw: line.to_owned(),
    })
}

fn take_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned()
}

fn take_u64(value: &serde_json::Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

fn session_matches(ev: &InjectEvent, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(needle) => ev.session_id.contains(needle),
    }
}

#[expect(
    clippy::print_stdout,
    reason = "CLI subcommand emits one event per line to stdout by design."
)]
fn emit_event(ev: &InjectEvent) {
    println!(
        "{ts}  session={sid}  hits={hits}  chars={chars}",
        ts = ev.ts,
        sid = ev.session_id,
        hits = ev.hits,
        chars = ev.chars_injected,
    );
    if !ev.additional_context.is_empty() {
        println!("{}", ev.additional_context);
    }
    println!("{}", ev.raw);
    println!();
}
