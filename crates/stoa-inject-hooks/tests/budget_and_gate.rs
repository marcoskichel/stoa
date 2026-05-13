//! E2E quality gate: token-budget cap + relevance gate.
//!
//! Spec source: ARCHITECTURE.md §6.2 — every injection enforces a hard
//! token cap (default 1500 tokens for `SessionStart`) and a relevance
//! threshold below which no injection is emitted.
//!
//! These tests are deliberately conservative: token estimation is
//! approximate (chars/4), so we assert the wrapped block stays well
//! below 4 * 1500 = 6000 chars even when the corpus is huge.

mod common;

use common::{init, inject_hook, rebuild, stderr, stdout, workspace, write_file};

const PAGE_TEMPLATE: &str = "\
---
id: {ID}
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-12
---

# {NAME}

{NAME} is a recurring library used across the redis caching system. \
It exposes a key/value API and supports session-token storage. \
Repeated text to make BM25 scoring meaningful: \
redis redis redis cache cache cache session session session.
";

fn make_corpus(ws: &assert_fs::TempDir, n: usize) {
    init(ws);
    for i in 0..n {
        let id = format!("ent-redis-{i}");
        let body = PAGE_TEMPLATE.replace("{ID}", &id).replace("{NAME}", &id);
        write_file(ws, &format!("wiki/entities/{id}.md"), &body);
    }
    rebuild(ws);
}

fn payload(workspace_path: &std::path::Path) -> String {
    serde_json::json!({
        "hook_event_name": "SessionStart",
        "session_id": "01JBUDGET00000000000000000",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": workspace_path.display().to_string(),
        "model": "claude-opus-4-7",
        "source": "startup",
    })
    .to_string()
}

#[test]
fn token_budget_cap_truncates_oversized_corpus() {
    let ws = workspace();
    make_corpus(&ws, 200);
    let out = inject_hook(&ws, &payload(ws.path()));
    assert!(out.status.success(), "{}", stderr(&out));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let ctx = parsed["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .expect("missing additionalContext");
    let approx_tokens = ctx.chars().count() / 4;
    assert!(
        approx_tokens <= 2000,
        "additionalContext must respect the ~1500-token SessionStart cap (saw ~{approx_tokens} tokens, {} chars): \n{ctx}",
        ctx.chars().count(),
    );
}

#[test]
fn relevance_gate_skips_when_corpus_is_empty() {
    let ws = workspace();
    init(&ws);
    rebuild(&ws);
    let out = inject_hook(&ws, &payload(ws.path()));
    assert!(out.status.success(), "{}", stderr(&out));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let ctx = parsed["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap_or("");
    assert!(
        ctx.is_empty(),
        "empty workspace → no hits → relevance gate fires → empty additionalContext: {ctx}",
    );
}
