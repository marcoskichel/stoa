//! Catalog rendering — refresh `wiki/index.md` from the current wiki tree.
//!
//! The index is grouped by page kind (ARCHITECTURE §2 "index.md"). Each
//! entry is a line of the form `- [<id>](<rel>): <title>` where the title
//! is pulled from the page's frontmatter (best-effort).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::Context;
use serde_yaml::Value;
use stoa_core::PageDir;

use crate::page::split_page;
use crate::workspace::Workspace;

/// Re-render `wiki/index.md` to reflect what's currently on disk.
pub(crate) fn refresh_index(ws: &Workspace) -> anyhow::Result<()> {
    let mut out = String::new();
    out.push_str("# Wiki index\n\n");
    out.push_str("Auto-generated catalog. Re-run `stoa write` to refresh.\n\n");
    for dir in PageDir::all() {
        append_section(ws, dir, &mut out)?;
    }
    fs::write(ws.index_md(), out).with_context(|| format!("writing `{}`", ws.index_md().display()))
}

fn append_section(ws: &Workspace, dir: PageDir, out: &mut String) -> anyhow::Result<()> {
    let path = ws.wiki_subdir(dir);
    let _ = writeln!(out, "## {}\n", dir.as_subdir());
    if !path.is_dir() {
        out.push_str("_(empty)_\n\n");
        return Ok(());
    }
    let mut entries = collect_entries(&path)?;
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    if entries.is_empty() {
        out.push_str("_(empty)_\n\n");
        return Ok(());
    }
    for entry in entries {
        let _ = writeln!(
            out,
            "- [{id}]({sub}/{id}.md): {title}",
            id = entry.id,
            sub = dir.as_subdir(),
            title = entry.title,
        );
    }
    out.push('\n');
    Ok(())
}

struct Entry {
    id: String,
    title: String,
}

fn collect_entries(dir: &Path) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for raw in fs::read_dir(dir).with_context(|| format!("reading `{}`", dir.display()))? {
        let raw = raw?;
        let path = raw.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_owned();
        let title = read_title(&path).unwrap_or_else(|| id.clone());
        entries.push(Entry { id, title });
    }
    Ok(entries)
}

fn read_title(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let parsed = split_page(&text, "").ok()?;
    let value: Value = serde_yaml::from_str(&parsed.frontmatter_yaml).ok()?;
    value.get("title")?.as_str().map(str::to_owned)
}
