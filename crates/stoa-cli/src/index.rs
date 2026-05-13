//! `stoa index rebuild` — drop + rebuild the BM25 index from
//! `wiki/` + `sessions/`.
//!
//! Idempotent: every page id is upserted, every session JSONL is
//! re-tokenized. `ChromaDB` / KG ingestion are deferred to the Python
//! sidecar (M4 indexes the BM25 stream in Rust; the vector stream is
//! lazily populated via the daemon when running with embeddings).

use std::fs;
use std::path::Path;

use anyhow::{Context, anyhow};
use stoa_recall_local_chroma_sqlite::Bm25Backend;

use crate::page::split_page;
use crate::workspace::Workspace;

/// Run `stoa index rebuild`.
pub(crate) fn rebuild() -> anyhow::Result<()> {
    let ws = Workspace::current().context("locating Stoa workspace")?;
    let bm25 = open_bm25(&ws)?;
    reindex_wiki(&ws, &bm25)?;
    reindex_sessions(&ws, &bm25)?;
    Ok(())
}

/// Re-index the entire workspace from disk. Daemon helper used when the
/// recall queue payload references a missing path (deletion or rename).
pub(crate) fn reindex_via_full_rebuild(workspace_root: &Path) -> anyhow::Result<()> {
    let ws = Workspace::find_from(workspace_root).context("locating Stoa workspace")?;
    let bm25 = open_bm25(&ws)?;
    reindex_wiki(&ws, &bm25)?;
    reindex_sessions(&ws, &bm25)
}

/// Re-index one wiki page. Daemon helper used by the watcher when a
/// `wiki/**/*.md` change fires.
pub(crate) fn reindex_one_wiki_page(
    abs_path: &Path,
    bm25: &Bm25Backend,
    rel_path: &str,
) -> anyhow::Result<()> {
    let page_id = abs_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("page id missing for `{}`", abs_path.display()))?
        .to_owned();
    let raw = fs::read_to_string(abs_path)
        .with_context(|| format!("reading `{}`", abs_path.display()))?;
    let body = extract_body(&raw, &page_id);
    bm25.upsert(&page_id, "page", rel_path, &body)
        .map_err(|e| anyhow!("upsert `{page_id}`: {e}"))
}

fn open_bm25(ws: &Workspace) -> anyhow::Result<Bm25Backend> {
    let db = ws
        .root
        .join(".stoa")
        .join(stoa_recall_local_chroma_sqlite::RECALL_DB_FILE);
    Bm25Backend::open(&db).with_context(|| format!("opening `{}`", db.display()))
}

fn reindex_wiki(ws: &Workspace, bm25: &Bm25Backend) -> anyhow::Result<()> {
    for dir in stoa_core::PageDir::all() {
        let sub = ws.wiki_subdir(dir);
        if !sub.is_dir() {
            continue;
        }
        index_dir(&sub, bm25, dir.as_subdir())?;
    }
    Ok(())
}

fn index_dir(dir: &Path, bm25: &Bm25Backend, subdir: &str) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading `{}`", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        index_single_page(&path, bm25, subdir)?;
    }
    Ok(())
}

fn index_single_page(path: &Path, bm25: &Bm25Backend, subdir: &str) -> anyhow::Result<()> {
    let page_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("page id missing for `{}`", path.display()))?
        .to_owned();
    let raw = fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    let body = extract_body(&raw, &page_id);
    let source = format!("wiki/{subdir}/{page_id}.md");
    bm25.upsert(&page_id, "page", &source, &body)
        .map_err(|e| anyhow!("upsert `{page_id}`: {e}"))
}

fn extract_body(raw: &str, page_id: &str) -> String {
    match split_page(raw, page_id) {
        Ok(parsed) => parsed.body,
        Err(_) => raw.to_owned(),
    }
}

fn reindex_sessions(ws: &Workspace, bm25: &Bm25Backend) -> anyhow::Result<()> {
    let dir = ws.root.join("sessions");
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(&dir).with_context(|| format!("reading `{}`", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "jsonl") {
            continue;
        }
        index_session_file(&path, bm25)?;
    }
    Ok(())
}

fn index_session_file(path: &Path, bm25: &Bm25Backend) -> anyhow::Result<()> {
    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("session id missing for `{}`", path.display()))?
        .to_owned();
    let raw = fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    let body = flatten_jsonl(&raw);
    let source = format!("sessions/{session_id}.jsonl");
    let doc_id = format!("session/{session_id}");
    bm25.upsert(&doc_id, "session", &source, &body)
        .map_err(|e| anyhow!("upsert session `{session_id}`: {e}"))
}

fn flatten_jsonl(raw: &str) -> String {
    let mut out = String::new();
    for line in raw.lines() {
        let Some(text) = extract_text(line) else {
            continue;
        };
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&text);
    }
    out
}

fn extract_text(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}
