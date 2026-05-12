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
// Leave log.md empty on init so the first event line is unambiguous; tests
// rely on log.md being empty or containing "init" right after `stoa init`.
const LOG_MD_HEADER: &str = "";

/// Run `stoa init` in the current working directory.
pub(crate) fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir().context("reading current dir")?;
    scaffold(&cwd)
}

fn scaffold(root: &Path) -> anyhow::Result<()> {
    create_dirs(root)?;
    write_if_missing(&root.join("STOA.md"), DEFAULT_STOA_MD)?;
    write_if_missing(&root.join(".gitignore"), GITIGNORE_BODY)?;
    write_if_missing(&root.join("wiki/index.md"), INDEX_MD_HEADER)?;
    write_if_missing(&root.join("wiki/log.md"), LOG_MD_HEADER)?;
    Ok(())
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
