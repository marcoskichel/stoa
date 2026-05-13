//! E2E quality gate: `stoa init --no-embeddings` cold-start path.
//!
//! Spec source: [ROADMAP.md M4 exit criteria].
//!
//! "Cold start: `stoa init --no-embeddings` produces a working BM25-only
//! workspace in <5s on fresh machine."
//!
//! Without `--no-embeddings`, init bootstraps the Python sidecar via `uv
//! sync` (model download, venv setup) and can take >60s on a clean machine.
//! The flag flips that off — the CLI must still scaffold a usable workspace
//! that can answer BM25-only `stoa query` calls.

mod common;

use std::time::{Duration, Instant};

use common::{exists, init, stderr, stoa, workspace, write_file};

#[test]
fn init_accepts_no_embeddings_flag() {
    let ws = workspace();
    let out = stoa(&ws, &["init", "--no-embeddings"]);
    assert!(
        out.status.success(),
        "`stoa init --no-embeddings` must succeed: {}",
        stderr(&out)
    );
}

#[test]
fn init_no_embeddings_completes_under_5s() {
    let ws = workspace();
    let start = Instant::now();
    let out = stoa(&ws, &["init", "--no-embeddings"]);
    let elapsed = start.elapsed();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        elapsed < Duration::from_secs(5),
        "ROADMAP M4 exit criterion: `stoa init --no-embeddings` must complete in <5s on \
         fresh machine; got {elapsed:?}",
    );
}

#[test]
fn init_no_embeddings_skips_python_sidecar_bootstrap() {
    let ws = workspace();
    let out = stoa(&ws, &["init", "--no-embeddings"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        !exists(&ws, ".stoa/python.venv") && !exists(&ws, ".stoa/vectors"),
        "`--no-embeddings` must NOT create the Python venv or ChromaDB store; \
         those are deferred until `stoa index rebuild --embeddings`",
    );
}

#[test]
fn init_no_embeddings_supports_bm25_query_round_trip() {
    let ws = workspace();
    let out = stoa(&ws, &["init", "--no-embeddings"]);
    assert!(out.status.success(), "{}", stderr(&out));
    write_file(
        &ws,
        "wiki/entities/ent-redis.md",
        "---\nid: ent-redis\nkind: entity\ntype: library\ncreated: 2026-05-12\nupdated: 2026-05-12\n---\n\nRedis is an in-memory cache.\n",
    );
    let rebuilt = stoa(&ws, &["index", "rebuild"]);
    assert!(
        rebuilt.status.success(),
        "BM25-only `stoa index rebuild` must work without ChromaDB: {}",
        stderr(&rebuilt),
    );
    let q = stoa(&ws, &["query", "redis", "--json", "--streams", "bm25"]);
    assert!(
        q.status.success(),
        "BM25-only query must succeed without the Python sidecar: {}",
        stderr(&q),
    );
    let parsed: serde_json::Value = serde_json::from_str(&common::stdout(&q)).unwrap();
    let hits = parsed.get("hits").and_then(|v| v.as_array()).unwrap();
    assert!(!hits.is_empty(), "indexed page must be reachable via BM25 stream");
}

#[test]
fn init_default_workspace_still_supports_query_subcommand() {
    let ws = workspace();
    init(&ws);
    let out = stoa(&ws, &["query", "--help"]);
    let body = common::stdout(&out);
    assert!(
        body.contains("query") || !body.is_empty(),
        "default `stoa init` (no flag) must still expose the `query` subcommand",
    );
}
