//! `stoa-inject-hook` library — SessionStart handler.
//!
//! Reads a Claude-Code SessionStart hook payload from stdin, runs a recall
//! query against the workspace's `.stoa/recall.db`, and emits a wrapped
//! `<stoa-memory>` block as `additionalContext` per ARCHITECTURE.md §6.2.
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
//! M5 skeleton — concrete implementation lands once the failing E2E gates
//! pin the contract. The skeleton emits an empty `additionalContext` so
//! the gates fail explicitly rather than panicking.

use std::io::{Read, Write};

use anyhow::Result;

/// Parse stdin (Claude-Code SessionStart JSON), run the recall query, and
/// write the response JSON to `out`. Returns the same error as the inner
/// handler so the binary can map it to an exit code.
pub fn run<R: Read, W: Write>(stdin: R, mut out: W) -> Result<()> {
    let _payload = read_payload(stdin)?;
    let response = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": "",
        },
    });
    serde_json::to_writer(&mut out, &response)?;
    Ok(())
}

fn read_payload<R: Read>(mut stdin: R) -> Result<serde_json::Value> {
    let mut buf = String::new();
    stdin.read_to_string(&mut buf)?;
    if buf.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    Ok(serde_json::from_str(&buf)?)
}
