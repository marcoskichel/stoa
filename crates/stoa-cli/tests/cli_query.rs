//! E2E quality gate: `stoa query` — hybrid recall over the indexed corpus.
//!
//! Spec source: [ROADMAP.md M4] + [ARCHITECTURE.md §6.1].
//!
//! These tests pin the user-facing CLI surface for hybrid recall:
//!
//! - `stoa query <q>` returns ranked hits with `source_path` always pointing
//!   at a file on disk (ARCHITECTURE §6.1: "always resolves to a file the
//!   user can open").
//! - `--json` output carries per-stream provenance (`streams_matched`),
//!   so callers can debug which of vector / BM25 / KG contributed each hit.
//! - `--streams bm25` works without the Python sidecar (BM25-only fallback
//!   is the architecture's no-embeddings cold-start path).
//! - Outside a Stoa workspace, `stoa query` exits non-zero with a clear
//!   diagnostic (mirrors `stoa daemon` and `stoa schema --check`).

mod common;

use common::{init, stderr, stdout, stoa, workspace, write_file};

/// Minimal wiki page used to verify ingest + recall round-trips.
const PAGE_BODY: &str = "\
---
id: ent-redis
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-12
---

# Redis

In-memory data store. Used for caching session tokens.
";

#[test]
fn query_subcommand_exists_in_help() {
    let ws = workspace();
    let out = stoa(&ws, &["--help"]);
    let body = stdout(&out);
    assert!(body.contains("query"), "`stoa --help` must list the `query` subcommand: {body}");
}

#[test]
fn query_outside_workspace_exits_non_zero() {
    let ws = workspace();
    let out = stoa(&ws, &["query", "redis"]);
    assert!(
        !out.status.success(),
        "`stoa query` must require a workspace (no STOA.md): stderr={}",
        stderr(&out),
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "End-to-end test pinning the JSON shape; assertions are part of the spec."
)]
fn query_returns_ranked_hits_with_resolvable_source_paths() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", PAGE_BODY);
    let _rebuilt = stoa(&ws, &["index", "rebuild"]);
    let out = stoa(&ws, &["query", "redis", "--json"]);
    assert!(
        out.status.success(),
        "`stoa query` must succeed when the corpus contains a hit: stderr={}",
        stderr(&out),
    );
    let body = stdout(&out);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("query --json must emit valid JSON");
    let hits = parsed
        .get("hits")
        .and_then(|v| v.as_array())
        .expect("JSON output must include a top-level `hits` array per ARCHITECTURE §6.1");
    assert!(
        !hits.is_empty(),
        "non-empty corpus + matching query must return at least one hit"
    );
    for hit in hits {
        let source_path = hit
            .get("source_path")
            .and_then(|v| v.as_str())
            .expect("each hit must carry a `source_path`");
        let absolute = ws
            .path()
            .join(source_path.strip_prefix("./").unwrap_or(source_path));
        assert!(
            absolute.exists() || std::path::Path::new(source_path).exists(),
            "`source_path` `{source_path}` must resolve to a real file (ARCH §6.1)",
        );
    }
}

#[test]
fn query_json_carries_per_stream_provenance() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", PAGE_BODY);
    let _rebuilt = stoa(&ws, &["index", "rebuild"]);
    let out = stoa(&ws, &["query", "redis", "--json", "--streams", "bm25"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let hits = parsed.get("hits").and_then(|v| v.as_array()).unwrap();
    assert!(!hits.is_empty(), "BM25-only query for the indexed page must hit");
    for hit in hits {
        let streams = hit
            .get("streams_matched")
            .and_then(|v| v.as_array())
            .expect("each hit must include `streams_matched` (ARCH §6.1 RRF provenance)");
        let names: Vec<&str> = streams.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            names.contains(&"bm25"),
            "BM25-only --streams filter must list `bm25` in streams_matched: {names:?}",
        );
        assert!(
            !names.contains(&"vector") && !names.contains(&"graph"),
            "BM25-only --streams filter must NOT include other streams: {names:?}",
        );
    }
}

#[test]
fn query_with_no_results_exits_zero_emits_empty_hits() {
    let ws = workspace();
    init(&ws);
    let _rebuilt = stoa(&ws, &["index", "rebuild"]);
    let out = stoa(&ws, &["query", "zzznever-matchsomething", "--json"]);
    assert!(
        out.status.success(),
        "no-results query is not an error condition: stderr={}",
        stderr(&out),
    );
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let hits = parsed.get("hits").and_then(|v| v.as_array()).unwrap();
    assert!(hits.is_empty(), "no matches must yield empty `hits` array, not omit the field");
}

#[test]
fn query_respects_k_flag() {
    let ws = workspace();
    init(&ws);
    for n in 0..5 {
        let id = format!("ent-redis-{n}");
        let body = PAGE_BODY.replace("ent-redis", &id);
        write_file(&ws, &format!("wiki/entities/{id}.md"), &body);
    }
    let _rebuilt = stoa(&ws, &["index", "rebuild"]);
    let out = stoa(&ws, &["query", "redis", "--json", "--k", "2"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let hits = parsed.get("hits").and_then(|v| v.as_array()).unwrap();
    assert!(
        hits.len() <= 2,
        "`--k 2` must cap returned hits at 2 (got {}): {hits:?}",
        hits.len(),
    );
}
