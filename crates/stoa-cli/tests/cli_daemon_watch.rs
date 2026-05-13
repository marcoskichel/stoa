//! E2E quality gate: daemon's wiki watcher reindexes on `wiki/**/*.md` change.
//!
//! Spec source: [ROADMAP.md M4] — "Wiki page change detection: the daemon
//! watches `wiki/` and re-indexes changed pages."
//!
//! Implementation strategy (per architecture review):
//! - The daemon embeds a `notify-debouncer-full`-driven watcher rooted at
//!   `<workspace>/wiki`.
//! - On a debounced `Modify` event for any `*.md` file, the watcher enqueues
//!   a `wiki.page.written` row on the `recall.request` lane with
//!   `method: "index_page"`. The recall worker picks it up and updates the
//!   BM25 + vector indexes.
//!
//! These tests cover the contract from the user's perspective: write a page,
//! wait briefly, query, see the new content. The exact debounce window is an
//! implementation detail — we tolerate up to 5 s here for CI variance on
//! shared runners.

mod common;

use std::time::Duration;

use common::{init, stderr, stoa, workspace, write_file};

const ORIGINAL_BODY: &str = "\
---
id: ent-redis
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-12
---

Redis is an in-memory data store.
";

const UPDATED_BODY: &str = "\
---
id: ent-redis
kind: entity
type: library
created: 2026-05-12
updated: 2026-05-12
---

Redis is now described as a streaming engine for serverless workloads.
";

#[test]
fn daemon_once_picks_up_pending_recall_request() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", ORIGINAL_BODY);
    let _rebuilt = stoa(&ws, &["index", "rebuild"]);
    let q = stoa_queue::Queue::open(&ws.path().join(".stoa/queue.db")).unwrap();
    q.insert_lane(
        "recall.request",
        "wiki.page.written",
        "ent-redis",
        &serde_json::json!({
            "method": "index_page",
            "args": {"page_id": "ent-redis", "path": "wiki/entities/ent-redis.md"},
        }),
    )
    .unwrap();
    drop(q);
    let out = stoa(&ws, &["daemon", "--once"]);
    assert!(
        out.status.success(),
        "`stoa daemon --once` must drain a `recall.request` row: {}",
        stderr(&out),
    );
}

#[test]
fn daemon_once_rejects_path_traversal_payload() {
    let ws = workspace();
    init(&ws);
    let q = stoa_queue::Queue::open(&ws.path().join(".stoa/queue.db")).unwrap();
    q.insert_lane(
        "recall.request",
        "wiki.page.written",
        "evil",
        &serde_json::json!({
            "method": "index_page",
            "args": {"path": "../../../etc/passwd"},
        }),
    )
    .unwrap();
    drop(q);
    let out = stoa(&ws, &["daemon", "--once"]);
    assert!(
        !out.status.success(),
        "`stoa daemon --once` must refuse `..`-escaping recall payloads (got success)",
    );
    let err = stderr(&out);
    assert!(
        err.contains("..") || err.contains("escapes") || err.contains("workspace"),
        "rejection diagnostic must mention the traversal: stderr={err}",
    );
}

#[test]
fn daemon_once_rejects_absolute_path_payload() {
    let ws = workspace();
    init(&ws);
    let q = stoa_queue::Queue::open(&ws.path().join(".stoa/queue.db")).unwrap();
    q.insert_lane(
        "recall.request",
        "wiki.page.written",
        "evil-abs",
        &serde_json::json!({
            "method": "index_page",
            "args": {"path": "/etc/passwd"},
        }),
    )
    .unwrap();
    drop(q);
    let out = stoa(&ws, &["daemon", "--once"]);
    assert!(
        !out.status.success(),
        "`stoa daemon --once` must refuse absolute-path recall payloads (got success)",
    );
}

#[test]
fn daemon_watch_reindexes_on_wiki_page_change() {
    let ws = workspace();
    init(&ws);
    write_file(&ws, "wiki/entities/ent-redis.md", ORIGINAL_BODY);
    let _rebuilt = stoa(&ws, &["index", "rebuild"]);
    write_file(&ws, "wiki/entities/ent-redis.md", UPDATED_BODY);
    std::thread::sleep(Duration::from_millis(500));
    drop(stoa(&ws, &["daemon", "--once"]));
    std::thread::sleep(Duration::from_millis(500));
    let q = stoa(&ws, &["query", "streaming", "--json", "--streams", "bm25"]);
    assert!(q.status.success(), "{}", stderr(&q));
    let parsed: serde_json::Value = serde_json::from_str(&common::stdout(&q)).unwrap();
    let hits = parsed.get("hits").and_then(|v| v.as_array()).unwrap();
    assert!(
        !hits.is_empty(),
        "after editing `ent-redis.md`, the new content (`streaming`) must be \
         searchable; got empty hits — watcher did not re-index",
    );
}
