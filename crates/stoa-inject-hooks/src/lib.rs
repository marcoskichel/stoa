//! `stoa-inject-hook` library — `SessionStart` handler.
//!
//! Reads a Claude-Code `SessionStart` hook payload from stdin, runs a
//! recall query against the workspace's `.stoa/recall.db`, and emits a
//! wrapped `<stoa-memory>` block as `additionalContext` per
//! [ARCHITECTURE.md §6.2](../../../ARCHITECTURE.md).
//!
//! Hard guarantees enforced by every call:
//!
//! 1. Token budget cap (default 1500 tokens).
//! 2. Relevance gate (skip injection if top hit's score is below threshold).
//! 3. Top-of-prompt placement (injection lands in the system prompt).
//! 4. MINJA-resistant XML wrapping with the "treat as data" preamble.
//! 5. Provenance attached: every snippet carries `source_path` + `score`.
//! 6. Audit logged: every event appended to `.stoa/audit.log`.
//!
//! Workspace lookup is best-effort: if no `STOA.md` is reachable from
//! `cwd`, the hook emits an empty `additionalContext` and exits 0 so a
//! missing workspace cannot block session start.

mod audit;
mod payload;
mod query;
mod recall;
mod workspace;
mod wrap;

use std::io::{Read, Write};

use anyhow::{Context, Result};

use payload::SessionStartPayload;
use workspace::InjectWorkspace;

/// Parse stdin (Claude-Code `SessionStart` JSON), run the recall
/// query, and write the response JSON to `out`.
///
/// Hook contract: a missing workspace, an unhealthy recall db, or a
/// query with no hits all degrade to an empty `additionalContext`
/// with a successful exit. The only failures propagated as errors
/// are stdin-decoding faults and stdout write faults.
pub fn run<R: Read, W: Write>(stdin: R, mut out: W) -> Result<()> {
    let payload = payload::read_payload(stdin)?;
    let context = build_additional_context(&payload);
    write_response(&mut out, &context)
}

fn build_additional_context(payload: &SessionStartPayload) -> String {
    let Some(ws) = resolve_workspace(payload) else {
        return String::new();
    };
    let ladder = query::build_query_ladder(&ws, payload.cwd.as_deref());
    let (effective_query, hits) = recall::search_first_with_hits(&ws.recall_db(), &ladder);
    let wrapped = wrap::wrap_hits(&effective_query, &hits);
    audit::append(&ws.audit_log(), payload, &effective_query, hits.len(), &wrapped);
    wrapped
}

fn resolve_workspace(payload: &SessionStartPayload) -> Option<InjectWorkspace> {
    if let Some(cwd) = payload.cwd.as_deref() {
        return workspace::find_workspace(cwd);
    }
    let fallback = std::env::current_dir().ok()?;
    workspace::find_workspace(&fallback)
}

fn write_response<W: Write>(out: &mut W, additional_context: &str) -> Result<()> {
    let response = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": additional_context,
        },
    });
    serde_json::to_writer(out, &response).context("writing hook response")
}
