//! `stoa-inject-hook` library — Claude Code injection handler.
//!
//! Reads a Claude Code `SessionStart` or `UserPromptSubmit` payload from
//! stdin, queries `stoa-recalld` for matching wiki hits, wraps them in a
//! MINJA-resistant `<stoa-memory>` block, and emits one JSON object on
//! stdout per Claude Code's `hookSpecificOutput` contract.
//!
//! Hard guarantees enforced for every call:
//!
//! 1. Token budget cap (default 1500 tokens).
//! 2. Relevance gate (skip injection if top hit's score is below threshold).
//! 3. Top-of-prompt placement (injection lands in the system prompt for
//!    `SessionStart`, or before the user message for `UserPromptSubmit`).
//! 4. MINJA-resistant XML wrapping with the "treat as data" preamble.
//! 5. Provenance attached: every snippet carries `source_path` + `score`.
//! 6. Audit logged: every event appended to `.stoa/audit.log`.
//!
//! Workspace lookup is best-effort. The daemon socket is allowed to be
//! down — both cases degrade to an empty `additionalContext` rather
//! than failing the hook.

mod audit;
mod payload;
mod query;
mod recall;
mod workspace;
mod wrap;

use std::io::{Read, Write};

use anyhow::{Context, Result};
use stoa_recall::MempalaceBackend;

use payload::HookPayload;
use workspace::InjectWorkspace;

pub use payload::HookEvent;

/// Parse stdin, run the recall query against the daemon, and write the
/// response JSON to `out`.
pub async fn run<R: Read, W: Write>(stdin: R, mut out: W) -> Result<()> {
    let payload = payload::read_payload(stdin)?;
    let backend = MempalaceBackend::from_env();
    let context = build_additional_context(&payload, &backend).await;
    write_response(&mut out, payload.hook_event_name_str(), &context)
}

async fn build_additional_context(payload: &HookPayload, backend: &MempalaceBackend) -> String {
    let Some(ws) = resolve_workspace(payload) else {
        return String::new();
    };
    let ladder = query::build_query_ladder(
        &ws,
        payload.cwd.as_deref(),
        payload.event(),
        payload.prompt.as_deref(),
    );
    let (effective_query, hits) = recall::search_first_with_hits(backend, &ladder).await;
    let wrapped = wrap::wrap_hits(&effective_query, &hits);
    audit::append(&ws.audit_log(), payload, &effective_query, hits.len(), &wrapped);
    wrapped
}

fn resolve_workspace(payload: &HookPayload) -> Option<InjectWorkspace> {
    if let Some(cwd) = payload.cwd.as_deref() {
        return workspace::find_workspace(cwd);
    }
    let fallback = std::env::current_dir().ok()?;
    workspace::find_workspace(&fallback)
}

fn write_response<W: Write>(out: &mut W, event_name: &str, additional_context: &str) -> Result<()> {
    let response = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": event_name,
            "additionalContext": additional_context,
        },
    });
    serde_json::to_writer(out, &response).context("writing hook response")
}
