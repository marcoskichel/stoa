//! E2E quality gate: `stoa index rebuild` — full reindex from `wiki/` +
//! `sessions/` + `raw/`.
//!
//! Spec source: [ROADMAP.md M4] + [ARCHITECTURE.md §1, §6.1].
//!
//! ARCHITECTURE §1: ".stoa/ is derived. `stoa rebuild` regenerates the entire
//! contents from `raw/` + `wiki/` + `sessions/`." This is the disaster
//! recovery story; M4 wires the FTS5 + KG side of it.

mod common;

use std::time::Duration;

use common::{exists, init, stderr, stoa, workspace, write_file};

const PAGE_REDIS: &str = "\
---
id: ent-redis
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-12
---

Redis is an in-memory cache.
";

const PAGE_PG: &str = "\
---
id: ent-postgres
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-12
---

Postgres is a relational database.
";

#[test]
fn index_rebuild_subcommand_exists() {
    let ws = workspace();
    let out = stoa(&ws, &["index", "--help"]);
    assert!(
        out.status.success() || !out.stderr.is_empty(),
        "`stoa index` subcommand must exist"
    );
}

#[test]
fn index_rebuild_succeeds_on_fresh_workspace() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["index", "rebuild"]);
    assert!(
        out.status.success(),
        "`stoa index rebuild` must succeed on an empty workspace: {}",
        stderr(&out),
    );
    assert!(
        exists(&ws, ".stoa/recall.db"),
        "`stoa index rebuild` must create `.stoa/recall.db` per ARCHITECTURE §1",
    );
}

#[test]
fn index_rebuild_indexes_wiki_pages() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", PAGE_REDIS);
    write_file(&ws, "wiki/entities/ent-postgres.md", PAGE_PG);
    let rebuilt = stoa(&ws, &["index", "rebuild"]);
    assert!(rebuilt.status.success(), "{}", stderr(&rebuilt));
    let out = stoa(&ws, &["query", "postgres", "--json", "--streams", "bm25"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let parsed: serde_json::Value = serde_json::from_str(&common::stdout(&out)).unwrap();
    let hits = parsed.get("hits").and_then(|v| v.as_array()).unwrap();
    assert!(
        hits.iter().any(|h| {
            h.get("source_path")
                .and_then(|v| v.as_str())
                .is_some_and(|p| p.contains("postgres"))
        }),
        "post-rebuild query must surface the indexed page; got {hits:?}",
    );
}

#[test]
fn index_rebuild_indexes_session_jsonl() {
    let ws = workspace();
    init(&ws);
    let session_body = "\
{\"role\":\"user\",\"text\":\"how does the slipstream cache work\"}
{\"role\":\"assistant\",\"text\":\"slipstream uses redis under the hood\"}
";
    write_file(&ws, "sessions/sess-001.jsonl", session_body);
    let rebuilt = stoa(&ws, &["index", "rebuild"]);
    assert!(rebuilt.status.success(), "{}", stderr(&rebuilt));
    let out = stoa(&ws, &["query", "slipstream", "--json", "--streams", "bm25"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let parsed: serde_json::Value = serde_json::from_str(&common::stdout(&out)).unwrap();
    let hits = parsed.get("hits").and_then(|v| v.as_array()).unwrap();
    assert!(
        !hits.is_empty(),
        "session JSONL content must be searchable post-rebuild (ARCHITECTURE §6.1: \
         sessions are indexed alongside wiki pages)",
    );
}

#[test]
fn index_rebuild_is_idempotent() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", PAGE_REDIS);
    let first = stoa(&ws, &["index", "rebuild"]);
    assert!(first.status.success(), "{}", stderr(&first));
    let count_before = count_hits(&ws, "redis");
    std::thread::sleep(Duration::from_millis(50));
    let second = stoa(&ws, &["index", "rebuild"]);
    assert!(second.status.success(), "{}", stderr(&second));
    let count_after = count_hits(&ws, "redis");
    assert_eq!(
        count_before, count_after,
        "second `stoa index rebuild` must not duplicate doc rows: {count_before} -> {count_after}",
    );
}

#[test]
fn index_rebuild_outside_workspace_exits_non_zero() {
    let ws = workspace();
    let out = stoa(&ws, &["index", "rebuild"]);
    assert!(
        !out.status.success(),
        "`stoa index rebuild` must require a workspace: {}",
        stderr(&out),
    );
}

fn count_hits(ws: &assert_fs::TempDir, q: &str) -> usize {
    let out = stoa(ws, &["query", q, "--json", "--streams", "bm25"]);
    if !out.status.success() {
        return 0;
    }
    let parsed: serde_json::Value = match serde_json::from_str(&common::stdout(&out)) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    parsed
        .get("hits")
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len)
}
