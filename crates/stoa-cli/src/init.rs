//! `stoa init` — scaffold a workspace.
//!
//! Idempotent: rerunning is safe and **must not** clobber user content.
//! Repair-aware: missing sub-directories are recreated.
//!
//! Layout per ARCHITECTURE §1 — `wiki/{entities,concepts,synthesis}/`,
//! `raw/`, `sessions/`, `.stoa/`, plus the workspace `STOA.md` + `.gitignore`.

use std::fs;
use std::path::Path;

use anyhow::Context;
use chrono::{SecondsFormat, Utc};

use crate::stoa_md::DEFAULT_STOA_MD;

const DIRS: &[&str] = &[
    "wiki",
    "wiki/entities",
    "wiki/concepts",
    "wiki/synthesis",
    "raw",
    "sessions",
    ".stoa",
];

const GITIGNORE_BODY: &str = "\
# Stoa-managed paths — derived or local-only.

# Derived state (rebuildable from raw/ + wiki/ + sessions/).
.stoa/

# Captured agent sessions (can grow large; JSONL).
sessions/
";

const INDEX_MD_HEADER: &str =
    "# Wiki index\n\nAuto-generated catalog. Re-run `stoa write` to refresh.\n";

/// Run `stoa init` in the current working directory.
///
/// `no_embeddings = true` is the BM25-only fast path: the FTS5 + KG
/// `recall.db` is still created (cheap; <100 ms) but the Python venv +
/// `ChromaDB` store at `.stoa/vectors/` are not touched. Per ROADMAP M4
/// the cold-start budget is <5 s on a fresh machine.
pub(crate) fn run(no_embeddings: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().context("reading current dir")?;
    scaffold(&cwd, no_embeddings)
}

fn scaffold(root: &Path, no_embeddings: bool) -> anyhow::Result<()> {
    create_dirs(root)?;
    write_if_missing(&root.join("STOA.md"), DEFAULT_STOA_MD)?;
    write_if_missing(&root.join(".gitignore"), GITIGNORE_BODY)?;
    write_if_missing(&root.join("wiki/index.md"), INDEX_MD_HEADER)?;
    append_init_event(&root.join("wiki/log.md"))?;
    let recall_db = root.join(".stoa").join("recall.db");
    let _conn = stoa_recall_local_chroma_sqlite::ensure_schema(&recall_db)
        .map_err(|e| anyhow::anyhow!("provisioning `recall.db`: {e}"))?;
    if !no_embeddings {
        let vectors_dir = root.join(".stoa").join("vectors");
        fs::create_dir_all(&vectors_dir)
            .with_context(|| format!("creating `{}`", vectors_dir.display()))?;
    }
    Ok(())
}

fn append_init_event(log_path: &Path) -> anyhow::Result<()> {
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let line = format!("{ts}  init  workspace scaffold\n");
    let mut current = fs::read_to_string(log_path).unwrap_or_default();
    if !current.is_empty() && !current.ends_with('\n') {
        current.push('\n');
    }
    current.push_str(&line);
    fs::write(log_path, current)
        .with_context(|| format!("appending init event to `{}`", log_path.display()))
}

fn create_dirs(root: &Path) -> anyhow::Result<()> {
    for rel in DIRS {
        let path = root.join(rel);
        fs::create_dir_all(&path)
            .with_context(|| format!("creating directory `{}`", path.display()))?;
    }
    Ok(())
}

fn write_if_missing(path: &Path, contents: &str) -> anyhow::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent of `{}`", path.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("writing `{}`", path.display()))
}
